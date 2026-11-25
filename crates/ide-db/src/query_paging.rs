//! Result paging: LIMIT/OFFSET wrapper for SELECT queries.

use crate::DbDriver;

/// Default auto-LIMIT when the user omits LIMIT (config override via `select_limit`).
pub const RESULT_PAGE_SIZE: usize = 50;

/// Hard cap when reading rows from the driver (manual LIMIT can be larger than page size).
pub const MAX_FETCH_ROWS: usize = 5_000;

/// Presets cycled by Alt-L in Studio.
pub const SELECT_LIMIT_PRESETS: &[usize] = &[50, 100, 200, 500, 0];

/// Append LIMIT/OFFSET when the statement looks like a plain SELECT/WITH (no existing LIMIT).
/// `page_size == 0` disables auto-paging (run as written).
/// Trino requires `OFFSET n LIMIT m` (#155).
pub fn paginate_select_sql(sql: &str, page: usize, page_size: usize) -> String {
    paginate_select_sql_for(DbDriver::Postgres, sql, page, page_size)
}

pub fn paginate_select_sql_for(
    driver: DbDriver,
    sql: &str,
    page: usize,
    page_size: usize,
) -> String {
    let trimmed = sql.trim().trim_end_matches(';').trim();
    if trimmed.is_empty() {
        return sql.to_string();
    }
    if page_size == 0 || !is_paged_query(trimmed) || has_limit_clause(trimmed) {
        return sql.to_string();
    }
    let offset = page.saturating_mul(page_size);
    match driver {
        DbDriver::Trino => format!("{trimmed} OFFSET {offset} LIMIT {page_size}"),
        DbDriver::Mssql => {
            if offset == 0 && !has_order_clause(trimmed) {
                // Page 0 without ORDER BY: use SELECT TOP.
                mssql_inject_top(trimmed, page_size)
            } else {
                // OFFSET/FETCH requires ORDER BY; add a deterministic one if missing.
                let ordered = if has_order_clause(trimmed) {
                    trimmed.to_string()
                } else {
                    format!("{trimmed} ORDER BY 1")
                };
                format!("{ordered} OFFSET {offset} ROWS FETCH NEXT {page_size} ROWS ONLY")
            }
        }
        _ => format!("{trimmed} LIMIT {page_size} OFFSET {offset}"),
    }
}

pub fn is_paged_query(sql: &str) -> bool {
    let stripped = crate::strip_sql_comments(sql);
    let t = stripped.trim_start().to_ascii_lowercase();
    t.starts_with("select") || t.starts_with("with")
}

fn has_order_clause(sql: &str) -> bool {
    let lower = sql.to_ascii_lowercase();
    lower.contains(" order ")
        || lower.contains("\norder ")
        || lower.contains("\torder ")
}

/// Inject `TOP (n)` after the first SELECT keyword for MSSQL page-0 queries.
fn mssql_inject_top(sql: &str, n: usize) -> String {
    let lower = sql.to_ascii_lowercase();
    if let Some(idx) = lower.find("select") {
        let after = idx + "select".len();
        // Preserve DISTINCT if present.
        let rest = &sql[after..];
        let rest_lower = rest.trim_start().to_ascii_lowercase();
        if rest_lower.starts_with("distinct") {
            let ws_len = rest.len() - rest.trim_start().len();
            let after_distinct = after + ws_len + "distinct".len();
            format!(
                "{}SELECT DISTINCT TOP ({n}){}",
                &sql[..idx],
                &sql[after_distinct..]
            )
        } else {
            format!("{}SELECT TOP ({n}){}", &sql[..idx], &sql[after..])
        }
    } else {
        format!("{sql} OFFSET 0 ROWS FETCH NEXT {n} ROWS ONLY")
    }
}

pub fn has_limit_clause(sql: &str) -> bool {
    let lower = sql.to_ascii_lowercase();
    // User-authored LIMIT disables auto-paging (word boundary-ish).
    lower.contains(" limit ")
        || lower.contains("\nlimit ")
        || lower.contains("\tlimit ")
        || lower.ends_with(" limit")
        || lower.contains(" limit\n")
        || lower.contains(" limit\t")
}

/// Next preset after `current` (wraps). `0` means unlimited / no auto LIMIT.
pub fn next_select_limit_preset(current: usize) -> usize {
    if let Some(i) = SELECT_LIMIT_PRESETS.iter().position(|&p| p == current) {
        return SELECT_LIMIT_PRESETS[(i + 1) % SELECT_LIMIT_PRESETS.len()];
    }
    // Snap unknown values onto the cycle starting at 50.
    50
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_page_size_is_50() {
        assert_eq!(RESULT_PAGE_SIZE, 50);
        assert_eq!(SELECT_LIMIT_PRESETS, &[50, 100, 200, 500, 0]);
    }

    #[test]
    fn auto_limit_on_bare_select() {
        let out = paginate_select_sql("SELECT * FROM brain.session_notes", 0, 50);
        assert_eq!(out, "SELECT * FROM brain.session_notes LIMIT 50 OFFSET 0");
    }

    #[test]
    fn page_two_uses_offset() {
        let out = paginate_select_sql("select 1", 2, 50);
        assert_eq!(out, "select 1 LIMIT 50 OFFSET 100");
    }

    #[test]
    fn trino_offset_before_limit() {
        let out = paginate_select_sql_for(DbDriver::Trino, "SELECT 1", 1, 50);
        assert_eq!(out, "SELECT 1 OFFSET 50 LIMIT 50");
    }

    #[test]
    fn explicit_limit_untouched() {
        let sql = "SELECT * FROM t LIMIT 10";
        assert_eq!(paginate_select_sql(sql, 0, 50), sql);
        assert!(has_limit_clause(sql));
    }

    #[test]
    fn page_size_zero_disables() {
        let sql = "SELECT * FROM t";
        assert_eq!(paginate_select_sql(sql, 0, 0), sql);
    }

    #[test]
    fn leading_comments_do_not_hide_select() {
        assert!(is_paged_query("-- comment\nSELECT 1"));
        assert!(is_paged_query("/* c */\nWITH x AS (SELECT 1) SELECT * FROM x"));
        assert!(!is_paged_query("-- comment\nDELETE FROM t"));
    }

    #[test]
    fn with_cte_is_paged() {
        let out = paginate_select_sql("WITH x AS (SELECT 1) SELECT * FROM x", 0, 50);
        assert!(out.ends_with("LIMIT 50 OFFSET 0"));
    }

    #[test]
    fn non_select_untouched() {
        let sql = "UPDATE t SET a = 1";
        assert_eq!(paginate_select_sql(sql, 0, 50), sql);
    }

    #[test]
    fn cycle_presets_wrap() {
        assert_eq!(next_select_limit_preset(50), 100);
        assert_eq!(next_select_limit_preset(100), 200);
        assert_eq!(next_select_limit_preset(200), 500);
        assert_eq!(next_select_limit_preset(500), 0);
        assert_eq!(next_select_limit_preset(0), 50);
        assert_eq!(next_select_limit_preset(77), 50);
    }

    #[test]
    fn mssql_page_zero_uses_top() {
        let out = paginate_select_sql_for(DbDriver::Mssql, "SELECT * FROM t", 0, 50);
        assert_eq!(out, "SELECT TOP (50) * FROM t");
    }

    #[test]
    fn mssql_page_one_uses_offset_fetch() {
        let out = paginate_select_sql_for(DbDriver::Mssql, "SELECT * FROM t", 1, 50);
        assert_eq!(
            out,
            "SELECT * FROM t ORDER BY 1 OFFSET 50 ROWS FETCH NEXT 50 ROWS ONLY"
        );
    }

    #[test]
    fn mssql_with_order_by_keeps_it() {
        let out = paginate_select_sql_for(
            DbDriver::Mssql,
            "SELECT * FROM t ORDER BY id",
            1,
            50,
        );
        assert_eq!(
            out,
            "SELECT * FROM t ORDER BY id OFFSET 50 ROWS FETCH NEXT 50 ROWS ONLY"
        );
    }

    #[test]
    fn mssql_page_zero_with_order_uses_offset_fetch() {
        let out = paginate_select_sql_for(
            DbDriver::Mssql,
            "SELECT * FROM t ORDER BY id",
            0,
            50,
        );
        assert_eq!(
            out,
            "SELECT * FROM t ORDER BY id OFFSET 0 ROWS FETCH NEXT 50 ROWS ONLY"
        );
    }
}
