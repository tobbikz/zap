//! User config: languages, theme, keymaps.

use anyhow::{bail, Context, Result};
use serde::de::Deserializer;
use serde::{Deserialize, Serialize, Serializer};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdeConfig {
    #[serde(default)]
    pub theme: ThemeConfig,
    #[serde(default)]
    pub editor: EditorConfig,
    #[serde(default)]
    pub code: CodeConfig,
    #[serde(default)]
    pub languages: HashMap<String, LanguageServerConfig>,
    /// Optional keymap overlay: action name -> key description (informational + future remap).
    #[serde(default)]
    pub keymaps: HashMap<String, String>,
    /// Recently opened workspace folders (absolute paths, newest first).
    #[serde(default)]
    pub recent_projects: Vec<String>,
    /// Recently opened files (absolute paths, newest first).
    #[serde(default)]
    pub recent_files: Vec<String>,
    #[serde(default)]
    pub database: DatabaseConfig,
    /// True when `config.toml` failed to parse — refuse `save` so defaults cannot clobber it (#180).
    #[serde(skip)]
    pub load_corrupt: bool,
    /// Parse error message when `load_corrupt` (for UI toast).
    #[serde(skip)]
    pub load_error: Option<String>,
}

impl Default for IdeConfig {
    fn default() -> Self {
        Self {
            theme: ThemeConfig::default(),
            editor: EditorConfig::default(),
            code: CodeConfig::default(),
            languages: default_languages(),
            keymaps: default_keymaps(),
            recent_projects: Vec::new(),
            recent_files: Vec::new(),
            database: DatabaseConfig::default(),
            load_corrupt: false,
            load_error: None,
        }
    }
}

/// Code / Neovim embed prefs (Wave 8).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeConfig {
    /// Override Neovim binary (else `TUI_IDE_NVIM` / PATH `nvim`).
    #[serde(default)]
    pub nvim_binary: Option<String>,
    /// `NVIM_APPNAME` override (e.g. `astronvim`).
    #[serde(default)]
    pub nvim_appname: Option<String>,
}

impl Default for CodeConfig {
    fn default() -> Self {
        Self {
            nvim_binary: None,
            nvim_appname: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// Soft SQL timeout in seconds (0 = disabled). Cancel still works via Esc.
    #[serde(default = "default_query_timeout")]
    pub query_timeout_secs: u64,
    /// Studio Explorer pane width (cols). Ctrl+← / Ctrl+→ in Studio.
    #[serde(default = "default_explorer_width")]
    pub explorer_width: u16,
    /// Explorer + Structure stack width (cols). Grows with Explorer as needed.
    #[serde(default = "default_studio_left_width")]
    pub studio_left_width: u16,
    /// Results pane height percent (25–70). Ctrl+↑ / Ctrl+↓ in Studio.
    #[serde(default = "default_results_pct")]
    pub results_pct: u16,
    /// Auto LIMIT for SELECT/WITH without an explicit LIMIT (0 = off). Alt-L cycles.
    #[serde(default = "default_select_limit")]
    pub select_limit: usize,
}

fn default_query_timeout() -> u64 {
    120
}
fn default_explorer_width() -> u16 {
    28
}
fn default_studio_left_width() -> u16 {
    // Explorer (≤28) + Structure (≥24) needs ~52+; prefer readable Structure empty state.
    56
}
fn default_results_pct() -> u16 {
    40
}
fn default_select_limit() -> usize {
    50
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            query_timeout_secs: default_query_timeout(),
            explorer_width: default_explorer_width(),
            studio_left_width: default_studio_left_width(),
            results_pct: default_results_pct(),
            select_limit: default_select_limit(),
        }
    }
}

/// Icon repertoire. `auto` follows the host font (Consolas-safe on Windows).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IconPref {
    #[default]
    Auto,
    Nerd,
    Safe,
    Ascii,
}

impl IconPref {
    fn parse(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "nerd" | "nf" | "on" | "true" => Self::Nerd,
            "safe" | "windows" | "unicode" => Self::Safe,
            "ascii" | "off" | "false" | "none" => Self::Ascii,
            _ => Self::Auto,
        }
    }
}

impl Serialize for IconPref {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(match self {
            Self::Auto => "auto",
            Self::Nerd => "nerd",
            Self::Safe => "safe",
            Self::Ascii => "ascii",
        })
    }
}

impl<'de> Deserialize<'de> for IconPref {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Raw {
            Bool(bool),
            Str(String),
        }
        match Raw::deserialize(deserializer)? {
            Raw::Bool(true) => Ok(Self::Nerd),
            Raw::Bool(false) => Ok(Self::Ascii),
            Raw::Str(s) => Ok(Self::parse(&s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeConfig {
    #[serde(default = "default_theme_name")]
    pub name: String,
    #[serde(default)]
    pub status_bg: String,
    #[serde(default)]
    pub accent: String,
    /// `auto` (default), `nerd`, `safe`, or `ascii`. `true`/`false` still work.
    /// On Windows, `auto` uses Consolas-safe glyphs unless the console font is a Nerd Font.
    #[serde(default)]
    pub icons: IconPref,
}

fn default_theme_name() -> String {
    // Zap default: charcoal canvas + arc-blue accent (AMOLED mono still `black`).
    "zap".into()
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            name: default_theme_name(),
            status_bg: "#000000".into(),
            accent: "mono".into(),
            icons: IconPref::Auto,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorConfig {
    #[serde(default = "default_tab_width")]
    pub tab_width: usize,
    #[serde(default = "default_true")]
    pub inlay_hints: bool,
    #[serde(default = "default_true")]
    pub format_on_save: bool,
    #[serde(default = "default_true")]
    pub soft_wrap: bool,
}

fn default_tab_width() -> usize {
    4
}
fn default_true() -> bool {
    true
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self {
            tab_width: 4,
            inlay_hints: true,
            format_on_save: true,
            soft_wrap: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageServerConfig {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    /// Language ids this server handles (LSP languageId strings).
    pub languages: Vec<String>,
    #[serde(default)]
    pub file_extensions: Vec<String>,
}

fn default_languages() -> HashMap<String, LanguageServerConfig> {
    let mut m = HashMap::new();
    m.insert(
        "rust".into(),
        LanguageServerConfig {
            command: "rust-analyzer".into(),
            args: vec![],
            languages: vec!["rust".into()],
            file_extensions: vec!["rs".into()],
        },
    );
    m.insert(
        "python".into(),
        LanguageServerConfig {
            command: "pyright-langserver".into(),
            args: vec!["--stdio".into()],
            languages: vec!["python".into()],
            file_extensions: vec!["py".into()],
        },
    );
    m.insert(
        "typescript".into(),
        LanguageServerConfig {
            command: "typescript-language-server".into(),
            args: vec!["--stdio".into()],
            languages: vec!["typescript".into(), "javascript".into()],
            file_extensions: vec![
                "ts".into(),
                "tsx".into(),
                "js".into(),
                "jsx".into(),
                "mjs".into(),
            ],
        },
    );
    m.insert(
        "go".into(),
        LanguageServerConfig {
            command: "gopls".into(),
            args: vec![],
            languages: vec!["go".into()],
            file_extensions: vec!["go".into()],
        },
    );
    m.insert(
        "clangd".into(),
        LanguageServerConfig {
            command: "clangd".into(),
            args: vec![],
            languages: vec!["c".into(), "cpp".into()],
            file_extensions: vec!["c".into(), "h".into(), "cpp".into(), "cc".into(), "hpp".into()],
        },
    );
    m.insert(
        "lua".into(),
        LanguageServerConfig {
            command: "lua-language-server".into(),
            args: vec![],
            languages: vec!["lua".into()],
            file_extensions: vec!["lua".into()],
        },
    );
    m
}

fn default_keymaps() -> HashMap<String, String> {
    HashMap::from([
        ("save".into(), "Ctrl-S".into()),
        ("palette".into(), "Space".into()),
        ("open_file".into(), "Ctrl-P".into()),
        ("search".into(), "Ctrl-Shift-F".into()),
        ("format".into(), "Ctrl-Shift-I".into()),
        ("rename".into(), "F2".into()),
        ("code_action".into(), "Ctrl-.".into()),
        ("goto_definition".into(), "g".into()),
        ("hover".into(), "K".into()),
        ("toggle_terminal".into(), "Ctrl-`".into()),
        ("git".into(), "Ctrl-Shift-G".into()),
        ("split_vertical".into(), "Ctrl-\\".into()),
        ("debug_toggle_breakpoint".into(), "F9".into()),
        ("debug_continue".into(), "F5".into()),
    ])
}

impl IdeConfig {
    pub fn config_path() -> Option<PathBuf> {
        Some(ide_core::config_dir()?.join("config.toml"))
    }

    pub fn load() -> Self {
        let Some(path) = Self::config_path() else {
            return Self::default();
        };
        match Self::load_from(&path) {
            Ok(cfg) => cfg,
            Err(e) => {
                // Do not overwrite the broken file — keep defaults in memory only (#180).
                let mut cfg = Self::default();
                cfg.load_corrupt = true;
                cfg.load_error = Some(format!("{e:#}"));
                cfg
            }
        }
    }

    pub fn load_from(path: &Path) -> Result<Self> {
        if !path.exists() {
            let cfg = Self::default();
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let text = toml::to_string_pretty(&cfg)?;
            ide_core::write_atomic(path, text)?;
            return Ok(cfg);
        }
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("read config {}", path.display()))?;
        let mut cfg: IdeConfig = toml::from_str(&text)
            .with_context(|| format!("parse config {}", path.display()))?;
        // Merge defaults for missing language servers
        for (k, v) in default_languages() {
            cfg.languages.entry(k).or_insert(v);
        }
        cfg.load_corrupt = false;
        cfg.load_error = None;
        Ok(cfg)
    }

    /// Parse TOML text without writing disk (Settings draft validation).
    pub fn parse_toml(text: &str) -> Result<Self> {
        let mut cfg: IdeConfig = toml::from_str(text).context("parse config.toml draft")?;
        for (k, v) in default_languages() {
            cfg.languages.entry(k).or_insert(v);
        }
        Ok(cfg)
    }

    pub fn server_for_language(&self, language_id: &str) -> Option<&LanguageServerConfig> {
        self.languages.values().find(|c| {
            c.languages.iter().any(|l| l == language_id)
                || c.file_extensions.iter().any(|e| {
                    language_id.contains(e.as_str()) // weak fallback
                })
        })
    }

    pub fn server_for_extension(&self, ext: &str) -> Option<(&str, &LanguageServerConfig)> {
        self.languages.iter().find_map(|(name, cfg)| {
            if cfg.file_extensions.iter().any(|e| e == ext) {
                Some((name.as_str(), cfg))
            } else {
                None
            }
        })
    }

    pub fn remember_project(&mut self, path: &Path) {
        if self.load_corrupt {
            return;
        }
        let Ok(canon) = path.canonicalize() else {
            return;
        };
        let s = canon.display().to_string();
        self.recent_projects.retain(|p| p != &s);
        self.recent_projects.insert(0, s);
        self.recent_projects.truncate(20);
        let _ = self.save();
    }

    /// Drop a missing/moved path from recent projects (and matching string forms).
    pub fn forget_project(&mut self, path: &Path) {
        if self.load_corrupt {
            return;
        }
        let display = path.display().to_string();
        let canon = path.canonicalize().ok().map(|p| p.display().to_string());
        let before = self.recent_projects.len();
        self.recent_projects.retain(|p| {
            if p == &display {
                return false;
            }
            if canon.as_ref().is_some_and(|c| c == p) {
                return false;
            }
            Path::new(p) != path
        });
        if self.recent_projects.len() != before && !cfg!(test) {
            let _ = self.save();
        }
    }

    pub fn remember_file(&mut self, path: &Path) {
        if self.load_corrupt {
            return;
        }
        let Ok(canon) = path.canonicalize() else {
            return;
        };
        if !canon.is_file() {
            return;
        }
        let s = canon.display().to_string();
        self.recent_files.retain(|p| p != &s);
        self.recent_files.insert(0, s);
        self.recent_files.truncate(30);
        let _ = self.save();
    }

    pub fn save(&self) -> Result<()> {
        if self.load_corrupt {
            bail!(
                "config.toml is corrupt — fix or restore {}.bak before saving",
                Self::config_path()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| "config.toml".into())
            );
        }
        let Some(path) = Self::config_path() else {
            return Ok(());
        };
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        if path.exists() {
            let bak = path.with_extension("toml.bak");
            let _ = std::fs::copy(&path, &bak);
        }
        let text = toml::to_string_pretty(self)?;
        // Atomic replace: a crash mid-save must not truncate the user's settings (#166).
        ide_core::write_atomic(&path, text)
            .with_context(|| format!("write config {}", path.display()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_toml_rejects_garbage() {
        assert!(IdeConfig::parse_toml("not = [valid").is_err());
        assert!(IdeConfig::parse_toml("theme = {}\n").is_ok());
    }

    #[test]
    fn forget_project_drops_stale_entry() {
        let mut cfg = IdeConfig::default();
        cfg.recent_projects = vec![
            "/gone/project".into(),
            "/keep/project".into(),
        ];
        cfg.forget_project(Path::new("/gone/project"));
        assert_eq!(cfg.recent_projects, vec!["/keep/project".to_string()]);
    }

    #[test]
    fn corrupt_config_refuses_save() {
        let mut cfg = IdeConfig::default();
        cfg.load_corrupt = true;
        assert!(cfg.save().is_err());
    }

    #[test]
    fn icon_pref_accepts_bool_and_string() {
        #[derive(Deserialize)]
        struct Wrap {
            #[serde(default)]
            icons: IconPref,
        }
        let off: Wrap = toml::from_str("icons = false").unwrap();
        assert_eq!(off.icons, IconPref::Ascii);
        let on: Wrap = toml::from_str("icons = true").unwrap();
        assert_eq!(on.icons, IconPref::Nerd);
        let safe: Wrap = toml::from_str("icons = \"safe\"").unwrap();
        assert_eq!(safe.icons, IconPref::Safe);
        let missing: Wrap = toml::from_str("").unwrap();
        assert_eq!(missing.icons, IconPref::Auto);
    }
}
