//! Extra engines beyond sqlx: MSSQL (tiberius) and ClickHouse (HTTP JSONCompact).

use anyhow::{Context, Result, bail};
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;
use tiberius::{AuthMethod, Client, Config, EncryptionLevel};
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio_util::compat::{Compat, TokioAsyncWriteCompatExt};

use crate::{ColumnInfo, DataSource, DbNodeKind, DbTreeNode, QueryResult};

pub type MssqlClient = Client<Compat<TcpStream>>;

/// Connect timeout for ClickHouse / ES / Trino HTTP (#197).
pub(crate) const HTTP_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
/// Overall request timeout so a hung CH does not freeze the Studio forever (#197).
pub(crate) const HTTP_REQUEST_TIMEOUT: Duration = Duration::from_secs(300);

/// Shared reqwest client for HTTP engines: timeouts, `ssl_ca`, and `ssl_trust_cert` (#197/#205).
pub(crate) fn build_http_client(src: &DataSource) -> Result<reqwest::Client> {
    let mut builder = reqwest::Client::builder()
        .connect_timeout(HTTP_CONNECT_TIMEOUT)
        .timeout(HTTP_REQUEST_TIMEOUT);
    if !src.ssl_ca.trim().is_empty() {
        let pem = std::fs::read(src.ssl_ca.trim())
            .with_context(|| format!("read ssl CA {}", src.ssl_ca))?;
        builder = builder.add_root_certificate(
            reqwest::Certificate::from_pem(&pem).context("parse ssl CA PEM")?,
        );
    }
    if src.accepts_invalid_certs() {
        builder = builder.danger_accept_invalid_certs(true);
    }
    builder.build().context("http client")
}

#[derive(Clone)]
pub struct ClickhouseHttp {
    http: reqwest::Client,
    base: String,
    user: String,
    password: String,
    database: String,
}

impl ClickhouseHttp {
    pub fn from_source(src: &DataSource, local_port: Option<u16>) -> Result<Self> {
        let (host, port) = match local_port {
            Some(lp) => ("127.0.0.1".to_string(), lp),
            None => (
                if src.host.is_empty() {
                    "127.0.0.1".into()
                } else {
                    src.host.clone()
                },
                if src.port == 0 { 8123 } else { src.port },
            ),
        };
        let scheme = if src.tls_required() {
            "https"
        } else {
            "http"
        };
        let base = format!("{scheme}://{host}:{port}/");
        let http = build_http_client(src)?;
        Ok(Self {
            http,
            base,
            user: if src.user.is_empty() {
                "default".into()
            } else {
                src.user.clone()
            },
            password: src.password.clone(),
            database: if src.database.is_empty() {
                "default".into()
            } else {
                src.database.clone()
            },
        })
    }

    pub fn with_database(&self, database: &str) -> Self {
        let mut c = self.clone();
        c.database = database.to_string();
        c
    }
}

pub async fn connect_mssql(src: &DataSource, local_port: Option<u16>) -> Result<Arc<Mutex<MssqlClient>>> {
    let host = match local_port {
        Some(_) => "127.0.0.1".to_string(),
        None => {
            if src.host.is_empty() {
                "127.0.0.1".into()
            } else {
                src.host.clone()
            }
        }
    };
    let port = local_port.unwrap_or(if src.port == 0 { 1433 } else { src.port });

    let mut config = Config::new();
    config.host(&host);
    config.port(port);
    config.authentication(AuthMethod::sql_server(
        if src.user.is_empty() { "sa" } else { &src.user },
        &src.password,
    ));
    if !src.database.is_empty() {
        config.database(&src.database);
    }
    let ssl_mode = src.ssl_mode.trim();
    if ssl_mode == "disable" {
        config.encryption(EncryptionLevel::NotSupported);
    } else {
        config.encryption(EncryptionLevel::Required);
    }
    if src.ssl_trust_cert {
        config.trust_cert();
    }

    let tcp = TcpStream::connect(config.get_addr())
        .await
        .with_context(|| format!("tcp connect mssql {host}:{port}"))?;
    tcp.set_nodelay(true)?;
    let client = Client::connect(config, tcp.compat_write())
        .await
        .with_context(|| format!("mssql login {host}:{port}"))?;
    Ok(Arc::new(Mutex::new(client)))
}

/// Session id (`@@SPID`) for the shared MSSQL client — used by Esc cancel (#41).
pub async fn mssql_backend_spid(client: &Arc<Mutex<MssqlClient>>) -> Result<i64> {
    let mut c = client.lock().await;
    let stream = c
        .simple_query("SELECT CONVERT(BIGINT, @@SPID)")
        .await
        .context("mssql @@SPID")?;
    let rows = stream.into_first_result().await.context("mssql @@SPID rows")?;
    let spid = rows
        .first()
        .and_then(|r| r.try_get::<i64, _>(0).ok().flatten())
        .unwrap_or(0);
    Ok(spid)
}

pub async fn exec_mssql(client: &Arc<Mutex<MssqlClient>>, sql: &str) -> Result<QueryResult> {
    let mut c = client.lock().await;
    let sql = sql.trim();
    if sql.is_empty() {
        bail!("empty SQL");
    }
    if is_query_like(sql) {
        let stream = c.simple_query(sql).await.context("mssql query")?;
        let rows = stream.into_first_result().await.context("mssql rows")?;
        Ok(rows_from_mssql(&rows))
    } else {
        let result = c.execute(sql, &[]).await.context("mssql execute")?;
        let affected = result.total();
        Ok(QueryResult {
            columns: vec!["affected".into()],
            rows: vec![vec![affected.to_string()]],
            affected: Some(affected),
            elapsed_ms: 0,
            truncated: false,
            source_schema: None,
            source_table: None,
            primary_keys: Vec::new(),
            original_rows: Vec::new(),
            cursor_row: 0,
            cursor_col: 0,
            explain: None,
            chart: None,
            chart_value_col: None,
            chart_category_col: None,
            chart_kind_name: None,
            page: 0,
            has_more: false,
        })
    }
}

/// SHOWPLAN on a dedicated path: always clear the session flag (#57 #52).
pub async fn exec_mssql_showplan(
    client: &Arc<Mutex<MssqlClient>>,
    stmt: &str,
) -> Result<QueryResult> {
    let stmt = stmt.trim();
    if stmt.is_empty() {
        bail!("empty SQL");
    }
    let mut c = client.lock().await;
    c.execute("SET SHOWPLAN_ALL ON", &[])
        .await
        .context("mssql SHOWPLAN_ALL ON")?;
    let plan = async {
        let stream = c
            .simple_query(stmt)
            .await
            .context("mssql SHOWPLAN query")?;
        let rows = stream
            .into_first_result()
            .await
            .context("mssql SHOWPLAN rows")?;
        Ok::<_, anyhow::Error>(rows_from_mssql(&rows))
    }
    .await;
    let off = c
        .execute("SET SHOWPLAN_ALL OFF", &[])
        .await
        .context("mssql SHOWPLAN_ALL OFF");
    match (plan, off) {
        (Ok(qr), Ok(_)) => Ok(qr),
        (Ok(_), Err(e)) => Err(e).context("plan ok but failed to clear SHOWPLAN_ALL"),
        (Err(e), Ok(_)) => Err(e),
        (Err(e), Err(off_err)) => Err(e).context(format!(
            "also failed to clear SHOWPLAN_ALL: {off_err:#}"
        )),
    }
}

fn rows_from_mssql(rows: &[tiberius::Row]) -> QueryResult {
    if rows.is_empty() {
        return QueryResult::empty();
    }
    let cols: Vec<String> = rows[0]
        .columns()
        .iter()
        .map(|c| c.name().to_string())
        .collect();
    let mut out_rows = Vec::with_capacity(rows.len());
    for r in rows {
        let mut line = Vec::with_capacity(cols.len());
        for i in 0..cols.len() {
            line.push(mssql_cell(r, i));
        }
        out_rows.push(line);
    }
    let mut q = QueryResult::empty();
    q.columns = cols;
    q.rows = out_rows;
    q.original_rows = q.rows.clone();
    q
}

fn mssql_cell(row: &tiberius::Row, i: usize) -> String {
    // Broaden Tiberius coverage — uniqueidentifier / decimal / dates / bytes (#56).
    if let Ok(v) = row.try_get::<&str, _>(i) {
        return v.unwrap_or("NULL").to_string();
    }
    if let Ok(v) = row.try_get::<i64, _>(i) {
        return v.map(|n| n.to_string()).unwrap_or_else(|| "NULL".into());
    }
    if let Ok(v) = row.try_get::<i32, _>(i) {
        return v.map(|n| n.to_string()).unwrap_or_else(|| "NULL".into());
    }
    if let Ok(v) = row.try_get::<i16, _>(i) {
        return v.map(|n| n.to_string()).unwrap_or_else(|| "NULL".into());
    }
    if let Ok(v) = row.try_get::<u8, _>(i) {
        return v.map(|n| n.to_string()).unwrap_or_else(|| "NULL".into());
    }
    if let Ok(v) = row.try_get::<f64, _>(i) {
        return v.map(|n| n.to_string()).unwrap_or_else(|| "NULL".into());
    }
    if let Ok(v) = row.try_get::<f32, _>(i) {
        return v.map(|n| n.to_string()).unwrap_or_else(|| "NULL".into());
    }
    if let Ok(v) = row.try_get::<bool, _>(i) {
        return v.map(|n| n.to_string()).unwrap_or_else(|| "NULL".into());
    }
    if let Ok(v) = row.try_get::<uuid::Uuid, _>(i) {
        return v
            .map(|u| u.to_string())
            .unwrap_or_else(|| "NULL".into());
    }
    if let Ok(v) = row.try_get::<tiberius::numeric::Numeric, _>(i) {
        return v
            .map(|n| n.to_string())
            .unwrap_or_else(|| "NULL".into());
    }
    if let Ok(v) = row.try_get::<chrono::NaiveDateTime, _>(i) {
        return v
            .map(|n| n.to_string())
            .unwrap_or_else(|| "NULL".into());
    }
    if let Ok(v) = row.try_get::<chrono::NaiveDate, _>(i) {
        return v
            .map(|n| n.to_string())
            .unwrap_or_else(|| "NULL".into());
    }
    if let Ok(v) = row.try_get::<chrono::DateTime<chrono::Utc>, _>(i) {
        return v
            .map(|n| n.to_rfc3339())
            .unwrap_or_else(|| "NULL".into());
    }
    if let Ok(v) = row.try_get::<&[u8], _>(i) {
        return match v {
            None => "NULL".into(),
            Some(b) if b.is_empty() => String::new(),
            Some(b) => {
                let mut s = String::with_capacity(2 + b.len() * 2);
                s.push_str("0x");
                for byte in b {
                    s.push_str(&format!("{byte:02x}"));
                }
                s
            }
        };
    }
    // Last resort: column type name so grids are not silently empty.
    row.columns()
        .get(i)
        .map(|c| format!("<{:?}>", c.column_type()))
        .unwrap_or_else(|| "NULL".into())
}

pub async fn exec_clickhouse(
    ch: &ClickhouseHttp,
    sql: &str,
    query_id: Option<&str>,
) -> Result<QueryResult> {
    let sql = sql.trim();
    if sql.is_empty() {
        bail!("empty SQL");
    }
    let body = if is_query_like(sql) {
        clickhouse_query_body(sql)
    } else {
        sql.to_string()
    };
    let mut req = ch.http.post(&ch.base).query(&[("database", ch.database.as_str())]);
    if let Some(qid) = query_id.filter(|s| !s.is_empty()) {
        req = req.query(&[("query_id", qid)]);
    }
    let mut req = req.body(body).header("X-ClickHouse-User", &ch.user);
    if !ch.password.is_empty() {
        req = req.header("X-ClickHouse-Key", &ch.password);
    }
    let resp = req.send().await.context("clickhouse http")?;
    let status = resp.status();
    let text = resp.text().await.context("clickhouse body")?;
    if !status.is_success() {
        bail!("clickhouse {}: {}", status.as_u16(), text.trim());
    }
    if !is_query_like(sql) {
        return Ok(QueryResult {
            columns: vec!["status".into()],
            rows: vec![vec!["ok".into()]],
            affected: None,
            elapsed_ms: 0,
            truncated: false,
            source_schema: None,
            source_table: None,
            primary_keys: Vec::new(),
            original_rows: Vec::new(),
            cursor_row: 0,
            cursor_col: 0,
            explain: None,
            chart: None,
            chart_value_col: None,
            chart_category_col: None,
            chart_kind_name: None,
            page: 0,
            has_more: false,
        });
    }
    parse_json_compact(&text)
}

fn parse_json_compact(text: &str) -> Result<QueryResult> {
    let v: Value = serde_json::from_str(text).context("parse ClickHouse JSONCompact")?;
    let meta = v
        .get("meta")
        .and_then(|m| m.as_array())
        .cloned()
        .unwrap_or_default();
    let columns: Vec<String> = meta
        .iter()
        .filter_map(|c| c.get("name").and_then(|n| n.as_str()).map(|s| s.to_string()))
        .collect();
    let data = v
        .get("data")
        .and_then(|d| d.as_array())
        .cloned()
        .unwrap_or_default();
    let mut rows = Vec::with_capacity(data.len());
    for row in data {
        let Some(arr) = row.as_array() else {
            continue;
        };
        rows.push(
            arr.iter()
                .map(|cell| match cell {
                    Value::Null => "NULL".into(),
                    Value::String(s) => s.clone(),
                    other => other.to_string(),
                })
                .collect(),
        );
    }
    let mut q = QueryResult::empty();
    q.columns = columns;
    q.rows = rows;
    q.original_rows = q.rows.clone();
    Ok(q)
}

/// Prepare a ClickHouse SELECT-like body: strip one trailing `;`, skip appending
/// FORMAT when the user already specified one (#66).
fn clickhouse_query_body(sql: &str) -> String {
    let trimmed = sql.trim().trim_end_matches(';').trim();
    if sql_has_explicit_format(trimmed) {
        trimmed.to_string()
    } else {
        format!("{trimmed}\nFORMAT JSONCompact")
    }
}

fn sql_has_explicit_format(sql: &str) -> bool {
    // Walk outside strings/comments for a bare FORMAT keyword.
    let bytes = sql.as_bytes();
    let mut i = 0;
    let mut in_single = false;
    let mut in_line = false;
    let mut in_block = false;
    while i < bytes.len() {
        let c = bytes[i] as char;
        if in_line {
            if c == '\n' {
                in_line = false;
            }
            i += 1;
            continue;
        }
        if in_block {
            if c == '*' && bytes.get(i + 1) == Some(&b'/') {
                in_block = false;
                i += 2;
            } else {
                i += 1;
            }
            continue;
        }
        if c == '\'' && !in_single {
            in_single = true;
            i += 1;
            continue;
        }
        if c == '\'' && in_single {
            if bytes.get(i + 1) == Some(&b'\'') {
                i += 2;
                continue;
            }
            in_single = false;
            i += 1;
            continue;
        }
        if in_single {
            i += 1;
            continue;
        }
        if c == '-' && bytes.get(i + 1) == Some(&b'-') {
            in_line = true;
            i += 2;
            continue;
        }
        if c == '/' && bytes.get(i + 1) == Some(&b'*') {
            in_block = true;
            i += 2;
            continue;
        }
        if c.eq_ignore_ascii_case(&'f') && i + 6 <= bytes.len() {
            let word = &sql[i..i + 6];
            if word.eq_ignore_ascii_case("format") {
                let before_ok = i == 0 || {
                    let b = bytes[i - 1];
                    !b.is_ascii_alphanumeric() && b != b'_'
                };
                let after_ok = i + 6 >= bytes.len() || {
                    let b = bytes[i + 6];
                    !b.is_ascii_alphanumeric() && b != b'_'
                };
                if before_ok && after_ok {
                    return true;
                }
            }
        }
        i += 1;
    }
    false
}

fn is_query_like(sql: &str) -> bool {
    let stripped = crate::strip_sql_comments(sql);
    let s = stripped.trim_start();
    let head = s
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    if matches!(
        head.as_str(),
        "select"
            | "with"
            | "show"
            | "describe"
            | "desc"
            | "explain"
            | "pragma"
            | "values"
            | "table"
            | "exec"
            | "execute"
            | "sp_help"
    ) {
        return true;
    }
    let lower = s.to_ascii_lowercase();
    matches!(head.as_str(), "insert" | "update" | "delete")
        && (lower.contains(" returning ") || lower.contains(" output "))
}

pub async fn mssql_databases(client: &Arc<Mutex<MssqlClient>>, source_id: &str) -> Result<Vec<DbTreeNode>> {
    let qr = exec_mssql(
        client,
        "SELECT name FROM sys.databases WHERE database_id > 4 ORDER BY name",
    )
    .await?;
    Ok(qr
        .rows
        .into_iter()
        .filter_map(|r| r.first().cloned())
        .map(|name| DbTreeNode {
            id: format!("{source_id}::db::{name}"),
            label: name.clone(),
            kind: DbNodeKind::Database,
            source_id: source_id.into(),
            database: Some(name),
            schema: None,
            table: None,
            query_sql: None,
            children: Vec::new(),
            expanded: false,
        })
        .collect())
}

pub async fn mssql_schemas(
    client: &Arc<Mutex<MssqlClient>>,
    source_id: &str,
    database: Option<&str>,
) -> Result<Vec<DbTreeNode>> {
    if let Some(db) = database {
        let _ = exec_mssql(client, &format!("USE [{}]", db.replace(']', "]]"))).await;
    }
    let qr = exec_mssql(
        client,
        "SELECT name FROM sys.schemas WHERE schema_id < 16384 ORDER BY name",
    )
    .await?;
    let db_owned = database.map(|s| s.to_string());
    Ok(qr
        .rows
        .into_iter()
        .filter_map(|r| r.first().cloned())
        .map(|name| {
            let id_db = db_owned.clone().unwrap_or_default();
            DbTreeNode {
                id: format!("{source_id}::schema::{id_db}::{name}"),
                label: name.clone(),
                kind: DbNodeKind::Schema,
                source_id: source_id.into(),
                database: db_owned.clone(),
                schema: Some(name),
                table: None,
                query_sql: None,
                children: Vec::new(),
                expanded: false,
            }
        })
        .collect())
}

pub async fn mssql_tables(
    client: &Arc<Mutex<MssqlClient>>,
    source_id: &str,
    database: Option<&str>,
    schema: &str,
) -> Result<Vec<DbTreeNode>> {
    if let Some(db) = database {
        let _ = exec_mssql(client, &format!("USE [{}]", db.replace(']', "]]"))).await;
    }
    let qr = exec_mssql(
        client,
        &format!(
            "SELECT TABLE_NAME, TABLE_TYPE FROM INFORMATION_SCHEMA.TABLES \
             WHERE TABLE_SCHEMA = N'{}' ORDER BY TABLE_TYPE, TABLE_NAME",
            schema.replace('\'', "''")
        ),
    )
    .await?;
    let db_owned = database.map(|s| s.to_string());
    Ok(qr
        .rows
        .into_iter()
        .filter_map(|r| {
            let name = r.first()?.clone();
            let ty = r.get(1).cloned().unwrap_or_default();
            let kind = if ty.to_ascii_uppercase().contains("VIEW") {
                DbNodeKind::View
            } else {
                DbNodeKind::Table
            };
            let id_db = db_owned.clone().unwrap_or_default();
            Some(DbTreeNode {
                id: format!("{source_id}::table::{id_db}::{schema}::{name}"),
                label: name.clone(),
                kind,
                source_id: source_id.into(),
                database: db_owned.clone(),
                schema: Some(schema.into()),
                table: Some(name),
                query_sql: None,
                children: Vec::new(),
                expanded: false,
            })
        })
        .collect())
}

pub async fn mssql_columns(
    client: &Arc<Mutex<MssqlClient>>,
    database: Option<&str>,
    schema: &str,
    table: &str,
) -> Result<Vec<ColumnInfo>> {
    if let Some(db) = database {
        let _ = exec_mssql(client, &format!("USE [{}]", db.replace(']', "]]"))).await;
    }
    let qr = exec_mssql(
        client,
        &format!(
            "SELECT COLUMN_NAME, DATA_TYPE, IS_NULLABLE FROM INFORMATION_SCHEMA.COLUMNS \
             WHERE TABLE_SCHEMA = N'{}' AND TABLE_NAME = N'{}' ORDER BY ORDINAL_POSITION",
            schema.replace('\'', "''"),
            table.replace('\'', "''")
        ),
    )
    .await?;
    Ok(qr
        .rows
        .into_iter()
        .filter_map(|r| {
            Some(ColumnInfo {
                name: r.first()?.clone(),
                data_type: r.get(1).cloned().unwrap_or_default(),
                nullable: r
                    .get(2)
                    .map(|s| s.eq_ignore_ascii_case("YES"))
                    .unwrap_or(true),
            })
        })
        .collect())
}

pub async fn ch_databases(ch: &ClickhouseHttp, source_id: &str) -> Result<Vec<DbTreeNode>> {
    let qr = exec_clickhouse(ch, "SHOW DATABASES", None).await?;
    Ok(qr
        .rows
        .into_iter()
        .filter_map(|r| r.first().cloned())
        .filter(|n| n != "system" && n != "INFORMATION_SCHEMA" && n != "information_schema")
        .map(|name| DbTreeNode {
            id: format!("{source_id}::db::{name}"),
            label: name.clone(),
            kind: DbNodeKind::Database,
            source_id: source_id.into(),
            database: Some(name),
            schema: None,
            table: None,
            query_sql: None,
            children: Vec::new(),
            expanded: false,
        })
        .collect())
}

pub async fn ch_tables(
    ch: &ClickhouseHttp,
    source_id: &str,
    database: &str,
) -> Result<Vec<DbTreeNode>> {
    let qr = exec_clickhouse(
        &ch.with_database(database),
        &format!(
            "SELECT name, engine FROM system.tables WHERE database = '{}' ORDER BY name",
            database.replace('\'', "''")
        ),
        None,
    )
    .await?;
    Ok(qr
        .rows
        .into_iter()
        .filter_map(|r| {
            let name = r.first()?.clone();
            let engine = r.get(1).cloned().unwrap_or_default();
            let kind = if engine.to_ascii_lowercase().contains("view") {
                DbNodeKind::View
            } else {
                DbNodeKind::Table
            };
            Some(DbTreeNode {
                id: format!("{source_id}::table::{database}::{name}"),
                label: name.clone(),
                kind,
                source_id: source_id.into(),
                database: Some(database.into()),
                schema: Some(database.into()),
                table: Some(name),
                query_sql: None,
                children: Vec::new(),
                expanded: false,
            })
        })
        .collect())
}

/// Run a query and return the raw HTTP body (appends `FORMAT <name>`).
pub async fn ch_query_format(ch: &ClickhouseHttp, sql: &str, format: &str) -> Result<String> {
    let sql = sql.trim();
    if sql.is_empty() {
        bail!("empty SQL");
    }
    let body = format!("{sql}\nFORMAT {format}");
    let mut req = ch
        .http
        .post(&ch.base)
        .query(&[("database", ch.database.as_str())])
        .body(body)
        .header("X-ClickHouse-User", &ch.user);
    if !ch.password.is_empty() {
        req = req.header("X-ClickHouse-Key", &ch.password);
    }
    let resp = req.send().await.context("clickhouse http")?;
    let status = resp.status();
    let text = resp.text().await.context("clickhouse body")?;
    if !status.is_success() {
        bail!("clickhouse {}: {}", status.as_u16(), text.trim());
    }
    Ok(text)
}

/// POST arbitrary SQL (DDL/DML) without forcing JSONCompact on SELECTs.
pub async fn ch_exec_script(ch: &ClickhouseHttp, sql: &str) -> Result<()> {
    let sql = sql.trim();
    if sql.is_empty() {
        return Ok(());
    }
    let mut req = ch
        .http
        .post(&ch.base)
        .query(&[("database", ch.database.as_str())])
        .body(sql.to_string())
        .header("X-ClickHouse-User", &ch.user);
    if !ch.password.is_empty() {
        req = req.header("X-ClickHouse-Key", &ch.password);
    }
    let resp = req.send().await.context("clickhouse http")?;
    let status = resp.status();
    let text = resp.text().await.context("clickhouse body")?;
    if !status.is_success() {
        bail!("clickhouse {}: {}", status.as_u16(), text.trim());
    }
    Ok(())
}

pub async fn ch_show_create(ch: &ClickhouseHttp, database: &str, table: &str) -> Result<String> {
    let qr = exec_clickhouse(
        &ch.with_database(database),
        &format!(
            "SHOW CREATE TABLE `{}`.`{}`",
            database.replace('`', "``"),
            table.replace('`', "``")
        ),
        None,
    )
    .await?;
    Ok(qr
        .rows
        .first()
        .and_then(|r| r.get(1).cloned().or_else(|| r.first().cloned()))
        .unwrap_or_else(|| "-- unavailable".into()))
}

pub async fn mssql_show_create(
    client: &Arc<Mutex<MssqlClient>>,
    database: Option<&str>,
    schema: &str,
    table: &str,
) -> Result<String> {
    if let Some(db) = database {
        let _ = exec_mssql(client, &format!("USE [{}]", db.replace(']', "]]"))).await;
    }
    let qr = exec_mssql(
        client,
        &format!(
            "SELECT m.definition \
             FROM sys.sql_modules m \
             JOIN sys.objects o ON m.object_id = o.object_id \
             JOIN sys.schemas s ON o.schema_id = s.schema_id \
             WHERE s.name = N'{}' AND o.name = N'{}' AND o.type IN ('V','P','FN','IF','TF')",
            schema.replace('\'', "''"),
            table.replace('\'', "''")
        ),
    )
    .await?;
    if let Some(def) = qr.rows.first().and_then(|r| r.first()).filter(|s| !s.is_empty()) {
        return Ok(def.clone());
    }
    Ok(String::new())
}

pub async fn mssql_primary_keys(
    client: &Arc<Mutex<MssqlClient>>,
    database: Option<&str>,
    schema: &str,
    table: &str,
) -> Result<Vec<String>> {
    if let Some(db) = database {
        let _ = exec_mssql(client, &format!("USE [{}]", db.replace(']', "]]"))).await;
    }
    let qr = exec_mssql(
        client,
        &format!(
            "SELECT c.name FROM sys.indexes i \
             JOIN sys.index_columns ic ON i.object_id = ic.object_id AND i.index_id = ic.index_id \
             JOIN sys.columns c ON ic.object_id = c.object_id AND ic.column_id = c.column_id \
             JOIN sys.tables t ON i.object_id = t.object_id \
             JOIN sys.schemas s ON t.schema_id = s.schema_id \
             WHERE i.is_primary_key = 1 AND s.name = N'{}' AND t.name = N'{}' \
             ORDER BY ic.key_ordinal",
            schema.replace('\'', "''"),
            table.replace('\'', "''")
        ),
    )
    .await?;
    Ok(qr
        .rows
        .into_iter()
        .filter_map(|r| r.first().cloned())
        .collect())
}

pub fn mssql_qual(schema: &str, table: &str) -> String {
    format!(
        "[{}].[{}]",
        schema.replace(']', "]]"),
        table.replace(']', "]]")
    )
}

pub fn build_mssql_create_table(schema: &str, table: &str, cols: &[ColumnInfo]) -> String {
    let qual = mssql_qual(schema, table);
    let body: Vec<String> = cols
        .iter()
        .map(|c| {
            let null = if c.nullable { " NULL" } else { " NOT NULL" };
            format!("  [{}] {}{null}", c.name.replace(']', "]]"), c.data_type)
        })
        .collect();
    format!("CREATE TABLE {qual} (\n{}\n);", body.join(",\n"))
}

pub async fn ch_columns(
    ch: &ClickhouseHttp,
    database: &str,
    table: &str,
) -> Result<Vec<ColumnInfo>> {
    let qr = exec_clickhouse(
        ch,
        &format!(
            "SELECT name, type FROM system.columns WHERE database = '{}' AND table = '{}' ORDER BY position",
            database.replace('\'', "''"),
            table.replace('\'', "''")
        ),
        None,
    )
    .await?;
    Ok(qr
        .rows
        .into_iter()
        .filter_map(|r| {
            Some(ColumnInfo {
                name: r.first()?.clone(),
                data_type: r.get(1).cloned().unwrap_or_default(),
                nullable: true,
            })
        })
        .collect())
}

#[cfg(test)]
mod http_client_tests {
    use super::{build_http_client, HTTP_CONNECT_TIMEOUT, HTTP_REQUEST_TIMEOUT};
    use crate::DataSource;

    #[test]
    fn build_http_client_applies_timeouts_and_trust() {
        assert_eq!(HTTP_CONNECT_TIMEOUT.as_secs(), 10);
        assert_eq!(HTTP_REQUEST_TIMEOUT.as_secs(), 300);
        let mut s = DataSource::postgres_local("ch", "default");
        s.ssl_mode = "require".into();
        s.ssl_trust_cert = true;
        assert!(build_http_client(&s).is_ok());
    }
}

#[cfg(test)]
mod query_like_tests {
    use super::is_query_like;

    #[test]
    fn comments_and_returning_are_queries() {
        assert!(is_query_like("-- starter\nSELECT 1 AS ok"));
        assert!(is_query_like("/* recipe */\nSELECT 1"));
        assert!(is_query_like("INSERT INTO t VALUES (1) RETURNING id"));
        assert!(is_query_like("UPDATE t SET x=1 OUTPUT inserted.id"));
        assert!(!is_query_like("-- comment\nDELETE FROM t"));
    }
}

#[cfg(test)]
mod clickhouse_format_tests {
    use super::{clickhouse_query_body, sql_has_explicit_format};

    #[test]
    fn strips_trailing_semicolon_and_appends_format() {
        let body = clickhouse_query_body("SELECT 1;");
        assert!(body.ends_with("FORMAT JSONCompact"), "{body}");
        assert!(!body.contains("1;"), "{body}");
    }

    #[test]
    fn respects_explicit_format() {
        assert!(sql_has_explicit_format("SELECT 1 FORMAT Pretty"));
        let body = clickhouse_query_body("SELECT 1 FORMAT Pretty;");
        assert_eq!(body, "SELECT 1 FORMAT Pretty");
        assert!(!body.contains("JSONCompact"));
    }

    #[test]
    fn format_inside_string_is_ignored() {
        assert!(!sql_has_explicit_format("SELECT 'FORMAT' AS x"));
        let body = clickhouse_query_body("SELECT 'FORMAT' AS x");
        assert!(body.contains("FORMAT JSONCompact"), "{body}");
    }
}
