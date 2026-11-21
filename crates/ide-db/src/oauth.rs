//! Browser OAuth helper — open authorize URL, paste token, persist for Studio.

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;

use crate::DbDriver;

#[derive(Debug, Default, Serialize, Deserialize)]
struct OauthFile {
    #[serde(default)]
    tokens: HashMap<String, String>,
}

fn oauth_path() -> Option<PathBuf> {
    ide_core::config_dir().map(|p| p.join("oauth.toml"))
}

fn load_file() -> OauthFile {
    let Some(path) = oauth_path() else {
        return OauthFile::default();
    };
    if !path.exists() {
        return OauthFile::default();
    }
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|t| toml::from_str(&t).ok())
        .unwrap_or_default()
}

fn save_file(file: &OauthFile) -> Result<()> {
    let path = oauth_path().context("no config dir for oauth")?;
    ide_core::write_atomic(&path, toml::to_string_pretty(file)?)
        .with_context(|| format!("write {}", path.display()))?;
    Ok(())
}

fn env_key(driver: DbDriver) -> &'static str {
    match driver {
        DbDriver::Bigquery => "BIGQUERY_ACCESS_TOKEN",
        DbDriver::Snowflake => "SNOWFLAKE_PASSWORD",
        _ => "TUI_IDE_OAUTH_TOKEN",
    }
}

fn pct_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// Build the Google installed-app authorize URL. Requires a real client id;
/// without one Google rejects the request, so callers get `None` (#63).
pub fn bigquery_authorize_url(client_id: &str, redirect_uri: &str) -> Option<String> {
    let client_id = client_id.trim();
    if client_id.is_empty() {
        return None;
    }
    let redirect_uri = if redirect_uri.trim().is_empty() {
        "http://localhost"
    } else {
        redirect_uri.trim()
    };
    Some(format!(
        "https://accounts.google.com/o/oauth2/v2/auth?response_type=token&client_id={}&redirect_uri={}&scope={}&prompt=consent",
        pct_encode(client_id),
        pct_encode(redirect_uri),
        pct_encode("https://www.googleapis.com/auth/bigquery"),
    ))
}

/// Authorize URLs for browser OAuth (paste token after consent).
/// BigQuery needs `ZAP_GOOGLE_CLIENT_ID` / `GOOGLE_CLIENT_ID` or returns `None`.
pub fn authorize_url(driver: DbDriver) -> Option<String> {
    match driver {
        DbDriver::Bigquery => {
            let client_id = std::env::var("ZAP_GOOGLE_CLIENT_ID")
                .or_else(|_| std::env::var("GOOGLE_CLIENT_ID"))
                .unwrap_or_default();
            let redirect = std::env::var("ZAP_OAUTH_REDIRECT_URI").unwrap_or_default();
            bigquery_authorize_url(&client_id, &redirect)
        }
        DbDriver::Snowflake => Some("https://app.snowflake.com/".to_string()),
        _ => None,
    }
}

/// Open URL in the desktop browser (xdg-open / open / cmd start).
pub fn open_browser(url: &str) -> Result<()> {
    let url = url.trim();
    if url.is_empty() {
        bail!("empty OAuth URL");
    }
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(url)
            .spawn()
            .context("open browser")?;
        return Ok(());
    }
    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .args(["/C", "start", "", url])
            .spawn()
            .context("open browser")?;
        return Ok(());
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        for bin in ["xdg-open", "gio", "sensible-browser"] {
            if Command::new(bin).arg(url).spawn().is_ok() {
                return Ok(());
            }
        }
        bail!("no browser opener (tried xdg-open/gio/sensible-browser)");
    }
}

/// Persist a pasted bearer/access token (process env + oauth.toml).
pub fn apply_pasted_token(driver: DbDriver, token: &str) -> Result<String> {
    let t = token.trim();
    if t.is_empty() {
        bail!("empty token");
    }
    let key = env_key(driver);
    std::env::set_var(key, t);
    let mut file = load_file();
    file.tokens.insert(key.to_string(), t.to_string());
    save_file(&file)?;
    Ok(format!("{key} set (process + ~/.config/zap/oauth.toml)"))
}

/// Load persisted tokens into the process env (no overwrite if already set).
pub fn load_persisted_tokens() {
    let file = load_file();
    for (key, val) in file.tokens {
        if std::env::var(&key).ok().filter(|s| !s.is_empty()).is_none() {
            std::env::set_var(key, val);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bigquery_url_requires_client_id() {
        assert!(bigquery_authorize_url("", "http://localhost").is_none());
        assert!(bigquery_authorize_url("   ", "").is_none());
    }

    #[test]
    fn bigquery_url_includes_client_and_redirect() {
        let url = bigquery_authorize_url("my-client.apps.googleusercontent.com", "").unwrap();
        assert!(url.contains("client_id=my-client.apps.googleusercontent.com"), "{url}");
        assert!(url.contains("redirect_uri=http%3A%2F%2Flocalhost"), "{url}");
        assert!(url.contains("response_type=token"), "{url}");
        assert!(url.contains("bigquery"), "{url}");
        assert!(!url.contains("client_id=&"), "{url}");
    }
}
