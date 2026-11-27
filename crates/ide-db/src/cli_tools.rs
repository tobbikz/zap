//! Discover native dump/restore CLIs across PATH and common install dirs (#293 / #84).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Resolve an absolute path for `tool` (e.g. `pg_dump`), preferring persisted overrides.
pub fn resolve_tool(
    tool: &str,
    overrides: &HashMap<String, String>,
) -> Option<PathBuf> {
    if let Some(p) = overrides.get(tool) {
        let path = PathBuf::from(p);
        if path.is_file() {
            return Some(path);
        }
    }
    if let Some(p) = which_on_path(tool) {
        return Some(p);
    }
    for cand in candidate_paths(tool) {
        if cand.is_file() {
            return Some(cand);
        }
    }
    None
}

/// Cross-platform PATH lookup (`where` on Windows, `which` elsewhere) (#84).
pub fn which_on_path(tool: &str) -> Option<PathBuf> {
    #[cfg(windows)]
    {
        let output = Command::new("where")
            .arg(tool)
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let text = String::from_utf8_lossy(&output.stdout);
        let first = text.lines().next()?.trim();
        if first.is_empty() {
            return None;
        }
        let p = PathBuf::from(first);
        p.is_file().then_some(p)
    }
    #[cfg(not(windows))]
    {
        let output = Command::new("which").arg(tool).output().ok()?;
        if !output.status.success() {
            return None;
        }
        let first = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if first.is_empty() {
            return None;
        }
        let p = PathBuf::from(first);
        p.is_file().then_some(p)
    }
}

fn candidate_paths(tool: &str) -> Vec<PathBuf> {
    let mut out = Vec::new();
    #[cfg(windows)]
    {
        let exe = if tool.ends_with(".exe") {
            tool.to_string()
        } else {
            format!("{tool}.exe")
        };
        let pf = std::env::var_os("ProgramFiles").map(PathBuf::from);
        let pf86 = std::env::var_os("ProgramFiles(x86)").map(PathBuf::from);
        let local = std::env::var_os("LOCALAPPDATA").map(PathBuf::from);
        if let Some(pf) = pf {
            // PostgreSQL installs: C:\Program Files\PostgreSQL\16\bin\pg_dump.exe
            if let Ok(entries) = std::fs::read_dir(pf.join("PostgreSQL")) {
                for e in entries.flatten() {
                    out.push(e.path().join("bin").join(&exe));
                }
            }
            out.push(pf.join("MySQL").join("MySQL Server 8.0").join("bin").join(&exe));
            out.push(pf.join("MySQL").join("MySQL Server 8.4").join("bin").join(&exe));
            out.push(pf.join("MariaDB 11.4").join("bin").join(&exe));
            out.push(pf.join("MariaDB 10.11").join("bin").join(&exe));
        }
        if let Some(pf86) = pf86 {
            if let Ok(entries) = std::fs::read_dir(pf86.join("PostgreSQL")) {
                for e in entries.flatten() {
                    out.push(e.path().join("bin").join(&exe));
                }
            }
        }
        if let Some(local) = local {
            out.push(local.join("Programs").join("PostgreSQL").join("bin").join(&exe));
            // scoop shims / apps
            out.push(local.join("scoop").join("shims").join(&exe));
            out.push(local.join("scoop").join("apps").join("postgresql").join("current").join("bin").join(&exe));
            out.push(local.join("scoop").join("apps").join("mysql").join("current").join("bin").join(&exe));
        }
        if let Some(home) = dirs::home_dir() {
            out.push(home.join("chocolatey").join("bin").join(&exe));
        }
    }
    #[cfg(not(windows))]
    {
        out.push(PathBuf::from(format!("/usr/bin/{tool}")));
        out.push(PathBuf::from(format!("/usr/local/bin/{tool}")));
        out.push(PathBuf::from(format!("/opt/homebrew/bin/{tool}")));
        // /usr/lib/postgresql/*/bin
        if let Ok(entries) = std::fs::read_dir("/usr/lib/postgresql") {
            for e in entries.flatten() {
                out.push(e.path().join("bin").join(tool));
            }
        }
        if let Ok(entries) = std::fs::read_dir("/usr/pgsql-16/bin") {
            let _ = entries;
            out.push(PathBuf::from(format!("/usr/pgsql-16/bin/{tool}")));
        }
        out.push(PathBuf::from(format!("/usr/pgsql-15/bin/{tool}")));
        out.push(PathBuf::from(format!("/usr/pgsql-14/bin/{tool}")));
        let _ = Path::new("/"); // silence unused import on unix-only Path use
    }
    out
}

/// Hint when a tool is missing.
pub fn missing_tool_hint(tool: &str) -> String {
    #[cfg(windows)]
    {
        format!(
            "{tool} not found — install client tools (PostgreSQL/MySQL bin) or set \
             database.tool_paths.{tool} in config.toml"
        )
    }
    #[cfg(not(windows))]
    {
        format!(
            "{tool} not found — install postgresql-client / mysql-client, or set \
             database.tool_paths.{tool} in ~/.config/zap/config.toml"
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_hint_mentions_tool() {
        assert!(missing_tool_hint("pg_dump").contains("pg_dump"));
    }
}
