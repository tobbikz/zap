//! Detect `:name` / `@name` / `$n` placeholders and substitute SQL-literal values for one run.

use anyhow::{Context, Result, bail};
use std::collections::{BTreeSet, HashMap};

/// Escape a cell value as a SQL literal. Numbers / bool / NULL stay bare (#68).
pub fn sql_string_literal(v: &str) -> String {
    let t = v.trim();
    if t.eq_ignore_ascii_case("NULL") {
        return "NULL".into();
    }
    if t.eq_ignore_ascii_case("true") || t.eq_ignore_ascii_case("false") {
        return t.to_ascii_lowercase();
    }
    if is_bare_number(t) {
        return t.to_string();
    }
    format!("'{}'", t.replace('\'', "''"))
}

fn is_bare_number(t: &str) -> bool {
    if t.is_empty() {
        return false;
    }
    // Integers / decimals (optional leading sign). Reject empty after sign.
    let mut chars = t.chars().peekable();
    if matches!(chars.peek(), Some('+' | '-')) {
        chars.next();
    }
    let mut saw_digit = false;
    let mut saw_dot = false;
    for c in chars {
        if c.is_ascii_digit() {
            saw_digit = true;
        } else if c == '.' && !saw_dot {
            saw_dot = true;
        } else {
            return false;
        }
    }
    saw_digit
}

/// Redis / key-value commands must not treat `foo:bar` as SQL params (#68).
fn looks_like_kv_command(sql: &str) -> bool {
    let head = sql
        .trim_start()
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_ascii_uppercase();
    matches!(
        head.as_str(),
        "GET"
            | "SET"
            | "DEL"
            | "EXISTS"
            | "EXPIRE"
            | "TTL"
            | "PTTL"
            | "TYPE"
            | "HGET"
            | "HGETALL"
            | "HSET"
            | "HMGET"
            | "LRANGE"
            | "LPUSH"
            | "RPUSH"
            | "SMEMBERS"
            | "SADD"
            | "ZRANGE"
            | "ZADD"
            | "KEYS"
            | "SCAN"
            | "PING"
            | "INFO"
            | "DBSIZE"
    )
}

/// Return sorted unique placeholder keys: `:name`/`@name` without sigil, and `"1"`, `"2"` for `$n`.
pub fn detect_query_params(sql: &str) -> Vec<String> {
    if looks_like_kv_command(sql) {
        return Vec::new();
    }
    let mut named = BTreeSet::new();
    let mut positional = BTreeSet::new();
    let bytes = sql.as_bytes();
    let mut i = 0;
    let mut in_single = false;
    let mut in_double = false;
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
        if c == '\'' && !in_double {
            if in_single && bytes.get(i + 1) == Some(&b'\'') {
                i += 2;
                continue;
            }
            in_single = !in_single;
            i += 1;
            continue;
        }
        if c == '"' && !in_single {
            in_double = !in_double;
            i += 1;
            continue;
        }
        if in_single || in_double {
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
        if (c == ':' || c == '@') && i + 1 < bytes.len() {
            let next = bytes[i + 1] as char;
            // Skip `::` (PG cast) and `@@` (MSSQL globals).
            if next == c {
                i += 2;
                continue;
            }
            if next.is_ascii_alphabetic() || next == '_' {
                let start = i + 1;
                i += 2;
                while i < bytes.len() {
                    let ch = bytes[i] as char;
                    if ch.is_ascii_alphanumeric() || ch == '_' {
                        i += 1;
                    } else {
                        break;
                    }
                }
                let name = &sql[start..i];
                named.insert(name.to_string());
                continue;
            }
        }
        if c == '$' && i + 1 < bytes.len() && bytes[i + 1].is_ascii_digit() {
            let start = i + 1;
            i += 1;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
            let n = &sql[start..i];
            positional.insert(n.parse::<u32>().unwrap_or(0));
            continue;
        }
        i += 1;
    }
    let mut out: Vec<String> = named.into_iter().collect();
    for n in positional {
        if n > 0 {
            out.push(n.to_string());
        }
    }
    out.sort_by(|a, b| {
        let a_pos = a.parse::<u32>().ok();
        let b_pos = b.parse::<u32>().ok();
        match (a_pos, b_pos) {
            (Some(x), Some(y)) => x.cmp(&y),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => a.cmp(b),
        }
    });
    out
}

pub fn sql_has_query_params(sql: &str) -> bool {
    !detect_query_params(sql).is_empty()
}

/// Seed text for the query-params prompt (`name=,1=` style).
pub fn query_param_prompt_seed(sql: &str) -> String {
    let params = detect_query_params(sql);
    params
        .iter()
        .map(|p| format!("{p}="))
        .collect::<Vec<_>>()
        .join(",")
}

/// Parse `key=value` pairs. Values may contain commas; next key starts at `,name=` (#68).
fn parse_param_map(input: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let mut rest = input.trim();
    while !rest.is_empty() {
        let Some(eq) = rest.find('=') else {
            break;
        };
        let key = rest[..eq]
            .trim()
            .trim_start_matches([',', ';'])
            .trim()
            .to_string();
        rest = rest[eq + 1..].trim_start();
        let mut end = rest.len();
        let bytes = rest.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b',' || bytes[i] == b';' {
                let after = rest[i + 1..].trim_start();
                if let Some(eq2) = after.find('=') {
                    let candidate = after[..eq2].trim();
                    if !candidate.is_empty()
                        && candidate
                            .chars()
                            .all(|c| c.is_ascii_alphanumeric() || c == '_')
                    {
                        end = i;
                        break;
                    }
                }
            }
            i += 1;
        }
        let val = rest[..end].trim().to_string();
        if !key.is_empty() {
            map.insert(key, val);
        }
        rest = rest[end..].trim_start_matches([',', ';']).trim_start();
    }
    map
}

/// Replace `needle` only outside SQL strings / line / block comments (#68).
fn replace_outside_strings(haystack: &str, needle: &str, replacement: &str) -> String {
    if needle.is_empty() || !haystack.contains(needle) {
        return haystack.to_string();
    }
    let bytes = haystack.as_bytes();
    let needle_b = needle.as_bytes();
    let mut out = String::with_capacity(haystack.len());
    let mut i = 0;
    let mut in_single = false;
    let mut in_double = false;
    let mut in_line = false;
    let mut in_block = false;
    while i < bytes.len() {
        let c = bytes[i] as char;
        if in_line {
            out.push(c);
            if c == '\n' {
                in_line = false;
            }
            i += 1;
            continue;
        }
        if in_block {
            out.push(c);
            if c == '*' && bytes.get(i + 1) == Some(&b'/') {
                out.push('/');
                in_block = false;
                i += 2;
            } else {
                i += 1;
            }
            continue;
        }
        if !in_single && !in_double {
            if c == '-' && bytes.get(i + 1) == Some(&b'-') {
                out.push('-');
                out.push('-');
                in_line = true;
                i += 2;
                continue;
            }
            if c == '/' && bytes.get(i + 1) == Some(&b'*') {
                out.push('/');
                out.push('*');
                in_block = true;
                i += 2;
                continue;
            }
            if i + needle_b.len() <= bytes.len() && &bytes[i..i + needle_b.len()] == needle_b {
                // Don't match mid-identifier (e.g. `$1` in `$10` already handled by order).
                let after = bytes.get(i + needle_b.len()).copied();
                let boundary_ok = match needle.chars().next() {
                    Some('$') => after.map(|b| !b.is_ascii_digit()).unwrap_or(true),
                    Some(':' | '@') => after
                        .map(|b| !b.is_ascii_alphanumeric() && b != b'_')
                        .unwrap_or(true),
                    _ => true,
                };
                if boundary_ok {
                    out.push_str(replacement);
                    i += needle_b.len();
                    continue;
                }
            }
        }
        if c == '\'' && !in_double {
            if in_single && bytes.get(i + 1) == Some(&b'\'') {
                out.push('\'');
                out.push('\'');
                i += 2;
                continue;
            }
            in_single = !in_single;
            out.push(c);
            i += 1;
            continue;
        }
        if c == '"' && !in_single {
            in_double = !in_double;
            out.push(c);
            i += 1;
            continue;
        }
        out.push(c);
        i += 1;
    }
    out
}

/// Substitute placeholders using `name=alice,1=42` (values SQL-escaped; use NULL for null).
pub fn substitute_query_params(sql: &str, param_input: &str) -> Result<String> {
    let required = detect_query_params(sql);
    if required.is_empty() {
        return Ok(sql.to_string());
    }
    let values = parse_param_map(param_input);
    for key in &required {
        if !values.contains_key(key) {
            bail!("missing value for query param `{key}` — use `{key}=...` in the prompt");
        }
    }

    let mut out = sql.to_string();
    // Positional $n — highest first so $10 is not broken by $1.
    let mut pos: Vec<u32> = required
        .iter()
        .filter_map(|k| k.parse().ok())
        .collect();
    pos.sort_by(|a, b| b.cmp(a));
    for n in pos {
        let key = n.to_string();
        let lit = sql_string_literal(values.get(&key).context("param")?);
        let needle = format!("${n}");
        if !out.contains(&needle) {
            bail!("SQL no longer contains placeholder ${n}");
        }
        out = replace_outside_strings(&out, &needle, &lit);
    }
    // Named :params / @params — longest names first to avoid prefix clashes.
    let mut names: Vec<String> = required
        .iter()
        .filter(|k| k.parse::<u32>().is_err())
        .cloned()
        .collect();
    names.sort_by_key(|n| std::cmp::Reverse(n.len()));
    for name in names {
        let lit = sql_string_literal(values.get(&name).context("param")?);
        let colon = format!(":{name}");
        let at = format!("@{name}");
        if out.contains(&colon) {
            out = replace_outside_strings(&out, &colon, &lit);
        } else if out.contains(&at) {
            out = replace_outside_strings(&out, &at, &lit);
        } else {
            bail!("SQL no longer contains placeholder :{name} or @{name}");
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_mixed_params() {
        let sql = "SELECT * FROM t WHERE id = $1 AND name = :name AND x = :other AND y = @y;";
        let p = detect_query_params(sql);
        assert!(p.contains(&"1".to_string()));
        assert!(p.contains(&"name".to_string()));
        assert!(p.contains(&"other".to_string()));
        assert!(p.contains(&"y".to_string()));
    }

    #[test]
    fn substitutes_literals() {
        let sql = "SELECT * FROM u WHERE id = $1 AND slug = :slug AND n = @n;";
        let out = substitute_query_params(sql, "1=42,slug=O'Brien,n=hi").unwrap();
        assert!(out.contains("id = 42"), "{out}");
        assert!(out.contains("slug = 'O''Brien'"), "{out}");
        assert!(out.contains("n = 'hi'"), "{out}");
        assert!(!out.contains('$'));
        assert!(!out.contains(":slug"));
        assert!(!out.contains("@n"));
    }

    #[test]
    fn skips_redis_commands() {
        assert!(detect_query_params("GET user:profile").is_empty());
        assert!(detect_query_params("SET foo:bar baz").is_empty());
    }

    #[test]
    fn skips_params_inside_strings_and_comments() {
        let sql = "SELECT ':name' -- :skip\nFROM t WHERE id = :id";
        let p = detect_query_params(sql);
        assert_eq!(p, vec!["id".to_string()]);
    }

    #[test]
    fn param_values_may_contain_commas() {
        let sql = "SELECT :note";
        let out = substitute_query_params(sql, "note=hello, world").unwrap();
        assert!(out.contains("'hello, world'"), "{out}");
    }
}
