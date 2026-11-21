//! Import JetBrains DataGrip / Database Tools `dataSources.xml` into Zap.

use crate::{DataSource, DbDriver, SourceRole};
use anyhow::{bail, Context, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// True when at least one JetBrains `dataSources.xml` is on disk.
pub fn datagrip_detected() -> bool {
    !discover_datagrip_xmls().is_empty()
}

/// Locate project-level `dataSources.xml` under JetBrains config dirs.
pub fn discover_datagrip_xmls() -> Vec<PathBuf> {
    let mut out = Vec::new();
    for root in jetbrains_roots() {
        if !root.is_dir() {
            continue;
        }
        // DataGrip2024.3/projects/<name>/.idea/dataSources.xml
        // Also IntelliJIdea*/PyCharm*/Rider* with Database tools.
        let Ok(entries) = std::fs::read_dir(&root) else {
            continue;
        };
        for ent in entries.flatten() {
            let name = ent.file_name().to_string_lossy().to_string();
            if !is_jetbrains_product_dir(&name) {
                continue;
            }
            let projects = ent.path().join("projects");
            if !projects.is_dir() {
                continue;
            }
            let Ok(projs) = std::fs::read_dir(&projects) else {
                continue;
            };
            for proj in projs.flatten() {
                let xml = proj.path().join(".idea").join("dataSources.xml");
                if xml.is_file() {
                    out.push(xml);
                }
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

fn is_jetbrains_product_dir(name: &str) -> bool {
    let prefixes = [
        "DataGrip",
        "IntelliJIdea",
        "IdeaIC",
        "PyCharm",
        "Rider",
        "WebStorm",
        "PhpStorm",
        "GoLand",
        "RubyMine",
        "CLion",
    ];
    prefixes.iter().any(|p| name.starts_with(p))
}

fn jetbrains_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(cfg) = dirs::config_dir() {
        roots.push(cfg.join("JetBrains"));
    }
    #[cfg(target_os = "macos")]
    if let Some(home) = dirs::home_dir() {
        roots.push(
            home.join("Library")
                .join("Application Support")
                .join("JetBrains"),
        );
    }
    #[cfg(windows)]
    if let Some(roaming) = dirs::config_dir() {
        // dirs::config_dir on Windows is already %APPDATA%
        let _ = roaming;
    }
    roots
}

/// Parse one `dataSources.xml` (+ sibling `dataSources.local.xml` for user names).
pub fn parse_datagrip_xml(path: &Path) -> Result<Vec<DataSource>> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("read {}", path.display()))?;
    let local_users = load_local_users(path);
    let mut out = Vec::new();
    for block in data_source_blocks(&text) {
        if let Some(src) = parse_one_block(&block, &local_users) {
            out.push(src);
        }
    }
    if out.is_empty() {
        bail!("no supported data sources in {}", path.display());
    }
    Ok(out)
}

fn load_local_users(shared: &Path) -> HashMap<String, String> {
    let local = shared.with_file_name("dataSources.local.xml");
    let Ok(text) = std::fs::read_to_string(local) else {
        return HashMap::new();
    };
    let mut map = HashMap::new();
    for block in data_source_blocks(&text) {
        let uuid = attr_in_open(&block, "uuid").unwrap_or_default();
        let name = attr_in_open(&block, "name").unwrap_or_default();
        let user = tag_text(&block, "user-name").unwrap_or_default();
        if user.is_empty() {
            continue;
        }
        if !uuid.is_empty() {
            map.insert(uuid, user.clone());
        }
        if !name.is_empty() {
            map.insert(format!("name:{name}"), user);
        }
    }
    map
}

fn data_source_blocks(xml: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = xml;
    while let Some(start) = rest.find("<data-source") {
        let after = &rest[start..];
        let Some(end_rel) = after.find("</data-source>") else {
            break;
        };
        let end = end_rel + "</data-source>".len();
        out.push(after[..end].to_string());
        rest = &after[end..];
    }
    out
}

fn parse_one_block(block: &str, local_users: &HashMap<String, String>) -> Option<DataSource> {
    let name = attr_in_open(block, "name").unwrap_or_else(|| "DataGrip".into());
    let uuid = attr_in_open(block, "uuid").unwrap_or_default();
    let driver_ref = tag_text(block, "driver-ref").unwrap_or_default();
    let jdbc_url = tag_text(block, "jdbc-url").unwrap_or_default();
    let jdbc_driver = tag_text(block, "jdbc-driver").unwrap_or_default();
    let read_only = tag_text(block, "read-only")
        .map(|s| s.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    let driver = map_driver(&driver_ref, &jdbc_driver, &jdbc_url)?;
    let mut src = DataSource {
        id: if uuid.is_empty() {
            uuid::Uuid::new_v4().to_string()
        } else {
            // Keep JetBrains uuid so re-import can dedupe by id.
            uuid.clone()
        },
        name: name.clone(),
        driver,
        host: String::new(),
        port: default_port_for(driver),
        user: String::new(),
        password: String::new(),
        database: String::new(),
        file: String::new(),
        ssh_host: String::new(),
        ssh_user: String::new(),
        ssh_port: 22,
        ssh_key: String::new(),
        ssl_mode: "prefer".into(),
        ssl_trust_cert: true,
        ssl_ca: String::new(),
        read_only,
        role: if read_only {
            SourceRole::Viewer
        } else {
            SourceRole::Admin
        },
        group: "DataGrip".into(),
        accent: String::new(),
        pinned: false,
    };

    if let Some(u) = local_users.get(&uuid).cloned().or_else(|| {
        local_users
            .get(&format!("name:{name}"))
            .cloned()
    }) {
        src.user = u;
    } else if let Some(u) = tag_text(block, "user-name") {
        src.user = u;
    }

    fill_from_jdbc(&mut src, &jdbc_url);
    // Property overrides (common in MSSQL / Snowflake).
    if let Some(db) = property(block, "database") {
        src.database = db;
    }
    if let Some(h) = property(block, "host") {
        src.host = h;
    }
    if let Some(p) = property(block, "port") {
        if let Ok(n) = p.parse() {
            src.port = n;
        }
    }
    Some(src)
}

fn map_driver(driver_ref: &str, jdbc_driver: &str, jdbc_url: &str) -> Option<DbDriver> {
    let blob = format!(
        "{} {} {}",
        driver_ref.to_ascii_lowercase(),
        jdbc_driver.to_ascii_lowercase(),
        jdbc_url.to_ascii_lowercase()
    );
    if blob.contains("postgres") {
        Some(DbDriver::Postgres)
    } else if blob.contains("mariadb") || blob.contains("mysql") {
        Some(DbDriver::Mysql)
    } else if blob.contains("sqlserver") || blob.contains("mssql") || blob.contains("jtds") {
        Some(DbDriver::Mssql)
    } else if blob.contains("sqlite") {
        Some(DbDriver::Sqlite)
    } else if blob.contains("clickhouse") {
        Some(DbDriver::Clickhouse)
    } else if blob.contains("duckdb") {
        Some(DbDriver::Duckdb)
    } else if blob.contains("redis") {
        Some(DbDriver::Redis)
    } else if blob.contains("mongo") {
        Some(DbDriver::Mongo)
    } else if blob.contains("snowflake") {
        Some(DbDriver::Snowflake)
    } else if blob.contains("bigquery") {
        Some(DbDriver::Bigquery)
    } else if blob.contains("elastic") || blob.contains("opensearch") {
        Some(DbDriver::Elasticsearch)
    } else if blob.contains("trino") || blob.contains("presto") {
        Some(DbDriver::Trino)
    } else {
        None
    }
}

fn default_port_for(d: DbDriver) -> u16 {
    match d {
        DbDriver::Postgres => 5432,
        DbDriver::Mysql => 3306,
        DbDriver::Mssql => 1433,
        DbDriver::Clickhouse => 8123,
        DbDriver::Redis => 6379,
        DbDriver::Mongo => 27017,
        DbDriver::Elasticsearch => 9200,
        DbDriver::Trino => 8080,
        DbDriver::Snowflake => 443,
        DbDriver::Bigquery => 443,
        DbDriver::Sqlite | DbDriver::Duckdb => 0,
    }
}

fn fill_from_jdbc(src: &mut DataSource, url: &str) {
    let u = url.trim();
    if u.is_empty() {
        return;
    }
    let lower = u.to_ascii_lowercase();
    if lower.starts_with("jdbc:sqlite:") {
        let path = u
            .trim_start_matches("jdbc:sqlite:")
            .trim_start_matches("//")
            .trim()
            .to_string();
        src.file = path;
        src.host.clear();
        return;
    }
    if lower.starts_with("jdbc:duckdb:") {
        let path = u
            .trim_start_matches("jdbc:duckdb:")
            .trim_start_matches("//")
            .trim()
            .to_string();
        src.file = path;
        src.host.clear();
        return;
    }
    // jdbc:postgresql://host:port/db?params
    // jdbc:mysql://host:port/db
    // jdbc:clickhouse://host:port/db
    if let Some(rest) = strip_jdbc_scheme(u) {
        let rest = rest.trim_start_matches('/');
        // MSSQL puts `;key=value` on the authority — cut those off first.
        let rest_no_semi = rest.split(';').next().unwrap_or(rest);
        let (authority, path_q) = rest_no_semi
            .split_once('/')
            .map(|(a, p)| (a, p))
            .unwrap_or((rest_no_semi, ""));
        let authority = authority.split('?').next().unwrap_or(authority);
        // host:port or host
        if let Some((h, p)) = authority.rsplit_once(':') {
            if !h.is_empty() && p.chars().all(|c| c.is_ascii_digit()) {
                src.host = h.to_string();
                if let Ok(n) = p.parse() {
                    src.port = n;
                }
            } else {
                src.host = authority.to_string();
            }
        } else if !authority.is_empty() {
            src.host = authority.to_string();
        }
        let db = path_q.split('?').next().unwrap_or(path_q).trim();
        if !db.is_empty() && !db.contains(';') {
            src.database = db.to_string();
        }
        // MSSQL: jdbc:sqlserver://host:1433;databaseName=foo
        if lower.contains("sqlserver") {
            if let Some(dbn) = param_semi(u, "databaseName").or_else(|| param_semi(u, "database"))
            {
                src.database = dbn;
            }
            if let Some(h) = param_semi(u, "serverName") {
                src.host = h;
            }
        }
        if let Some(mode) = query_param(u, "sslmode").or_else(|| query_param(u, "ssl-mode")) {
            src.ssl_mode = mode;
        }
        if query_param(u, "encrypt")
            .or_else(|| query_param(u, "ssl"))
            .is_some_and(|v| v == "true" || v == "1" || v.eq_ignore_ascii_case("require"))
        {
            if src.ssl_mode == "prefer" {
                src.ssl_mode = "require".into();
            }
        }
        if query_param(u, "trustServerCertificate")
            .or_else(|| param_semi(u, "trustServerCertificate"))
            .is_some_and(|v| v.eq_ignore_ascii_case("true"))
        {
            src.ssl_trust_cert = true;
        }
    }
}

fn strip_jdbc_scheme(url: &str) -> Option<&str> {
    let lower = url.to_ascii_lowercase();
    for prefix in [
        "jdbc:postgresql://",
        "jdbc:postgres://",
        "jdbc:mysql://",
        "jdbc:mariadb://",
        "jdbc:clickhouse://",
        "jdbc:ch://",
        "jdbc:redis://",
        "jdbc:mongodb://",
        "jdbc:mongo://",
        "jdbc:trino://",
        "jdbc:presto://",
        "jdbc:snowflake://",
        "jdbc:bigquery://",
        "jdbc:elasticsearch://",
        "jdbc:sqlserver://",
    ] {
        if lower.starts_with(prefix) {
            return Some(&url[prefix.len()..]);
        }
    }
    // Generic jdbc:xxx://
    if let Some(idx) = lower.find("://") {
        return Some(&url[idx + 3..]);
    }
    None
}

fn query_param(url: &str, key: &str) -> Option<String> {
    let q = url.split_once('?')?.1;
    for part in q.split('&') {
        if let Some((k, v)) = part.split_once('=') {
            if k.eq_ignore_ascii_case(key) {
                return Some(v.to_string());
            }
        }
    }
    None
}

fn param_semi(url: &str, key: &str) -> Option<String> {
    for part in url.split(';') {
        if let Some((k, v)) = part.split_once('=') {
            if k.trim().eq_ignore_ascii_case(key) {
                return Some(v.trim().to_string());
            }
        }
    }
    None
}

fn property(block: &str, name: &str) -> Option<String> {
    // <property name="host" value="..." />  or nested
    let needle = format!("name=\"{name}\"");
    let idx = block.find(&needle)?;
    let after = &block[idx..];
    if let Some(v) = attr_in_open(after, "value") {
        return Some(v);
    }
    None
}

fn tag_text(xml: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = xml.find(&open)? + open.len();
    let end = xml[start..].find(&close)? + start;
    let raw = xml[start..end].trim();
    if raw.is_empty() {
        return None;
    }
    Some(unescape_xml(raw))
}

fn attr_in_open(block: &str, attr: &str) -> Option<String> {
    let open_end = block.find('>')?;
    let head = &block[..open_end];
    let key = format!("{attr}=\"");
    let i = head.find(&key)? + key.len();
    let rest = &head[i..];
    let j = rest.find('"')?;
    Some(unescape_xml(&rest[..j]))
}

fn unescape_xml(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_postgres_jdbc() {
        let xml = r#"
<data-source source="LOCAL" name="pg-local" uuid="abc-123">
  <driver-ref>postgresql</driver-ref>
  <jdbc-driver>org.postgresql.Driver</jdbc-driver>
  <jdbc-url>jdbc:postgresql://127.0.0.1:5432/brain?sslmode=require</jdbc-url>
  <user-name>iks</user-name>
</data-source>
"#;
        let src = parse_one_block(xml, &HashMap::new()).unwrap();
        assert_eq!(src.driver, DbDriver::Postgres);
        assert_eq!(src.host, "127.0.0.1");
        assert_eq!(src.port, 5432);
        assert_eq!(src.database, "brain");
        assert_eq!(src.user, "iks");
        assert_eq!(src.ssl_mode, "require");
        assert_eq!(src.group, "DataGrip");
    }

    #[test]
    fn parses_sqlite_file() {
        let xml = r#"
<data-source name="mem" uuid="x">
  <driver-ref>sqlite.xerial</driver-ref>
  <jdbc-url>jdbc:sqlite:/tmp/app.db</jdbc-url>
</data-source>
"#;
        let src = parse_one_block(xml, &HashMap::new()).unwrap();
        assert_eq!(src.driver, DbDriver::Sqlite);
        assert_eq!(src.file, "/tmp/app.db");
    }

    #[test]
    fn parses_mssql_semicolons() {
        let xml = r#"
<data-source name="mssql" uuid="m1">
  <driver-ref>sqlserver.ms</driver-ref>
  <jdbc-url>jdbc:sqlserver://db.local:1433;databaseName=AppDb;encrypt=true;trustServerCertificate=true</jdbc-url>
</data-source>
"#;
        let src = parse_one_block(xml, &HashMap::new()).unwrap();
        assert_eq!(src.driver, DbDriver::Mssql);
        assert_eq!(src.host, "db.local");
        assert_eq!(src.port, 1433);
        assert_eq!(src.database, "AppDb");
        assert!(src.ssl_trust_cert);
    }

    #[test]
    fn skips_unknown_oracle() {
        let xml = r#"
<data-source name="ora" uuid="o1">
  <driver-ref>oracle</driver-ref>
  <jdbc-url>jdbc:oracle:thin:@//host:1521/ORCL</jdbc-url>
</data-source>
"#;
        assert!(parse_one_block(xml, &HashMap::new()).is_none());
    }
}
