//! Lightweight command plugins loaded from TOML (no WASM marketplace — local scripts/commands).

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;
use tracing::warn;

const PLUGIN_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub commands: Vec<PluginCommand>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginCommand {
    pub id: String,
    pub title: String,
    /// Shell command; `{file}`, `{root}`, `{line}` placeholders supported.
    pub run: String,
}

#[derive(Debug, Clone)]
pub struct PluginHost {
    pub plugins: Vec<PluginManifest>,
}

impl Default for PluginHost {
    fn default() -> Self {
        Self::load()
    }
}

impl PluginHost {
    pub fn load() -> Self {
        let mut plugins = Vec::new();
        if let Some(dir) = plugins_dir() {
            let _ = std::fs::create_dir_all(&dir);
            // Seed example plugin once
            let example = dir.join("example-echo.toml");
            if !example.exists() {
                let sample = PluginManifest {
                    name: "example".into(),
                    description: "Sample local plugin".into(),
                    commands: vec![PluginCommand {
                        id: "example.echo".into(),
                        title: "Echo current file path".into(),
                        run: "echo \"file={file} root={root} line={line}\"".into(),
                    }],
                };
                let _ = std::fs::write(
                    &example,
                    toml::to_string_pretty(&sample).unwrap_or_default(),
                );
            }
            if let Ok(rd) = std::fs::read_dir(&dir) {
                for entry in rd.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|e| e.to_str()) != Some("toml") {
                        continue;
                    }
                    match std::fs::read_to_string(&path)
                        .ok()
                        .and_then(|t| toml::from_str::<PluginManifest>(&t).ok())
                    {
                        Some(m) => plugins.push(m),
                        None => warn!("failed to load plugin {}", path.display()),
                    }
                }
            }
        }
        Self { plugins }
    }

    pub fn all_commands(&self) -> Vec<(String, String, String)> {
        // (plugin, id, title)
        let mut out = Vec::new();
        for p in &self.plugins {
            for c in &p.commands {
                out.push((p.name.clone(), c.id.clone(), c.title.clone()));
            }
        }
        out
    }

    /// Run a plugin command asynchronously (does not block the TUI runtime thread).
    pub async fn run(
        &self,
        command_id: &str,
        root: &Path,
        file: Option<&Path>,
        line: usize,
    ) -> Result<String> {
        for p in &self.plugins {
            if let Some(cmd) = p.commands.iter().find(|c| c.id == command_id) {
                let mut script = cmd.run.clone();
                script = script.replace("{root}", &shell_quote(&root.display().to_string()));
                script = script.replace(
                    "{file}",
                    &file
                        .map(|f| shell_quote(&f.display().to_string()))
                        .unwrap_or_else(|| shell_quote("")),
                );
                script = script.replace("{line}", &line.to_string());

                let mut child = shell_command(&script);
                child.current_dir(root).kill_on_drop(true);
                let out = timeout(PLUGIN_TIMEOUT, child.output())
                    .await
                    .with_context(|| {
                        format!(
                            "plugin {} timed out after {}s",
                            cmd.id,
                            PLUGIN_TIMEOUT.as_secs()
                        )
                    })?
                    .with_context(|| format!("run plugin {}", cmd.id))?;
                let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
                if !out.stderr.is_empty() {
                    if !text.is_empty() && !text.ends_with('\n') {
                        text.push('\n');
                    }
                    text.push_str(&String::from_utf8_lossy(&out.stderr));
                }
                if !out.status.success() {
                    bail!("plugin exited {:?}: {text}", out.status.code());
                }
                return Ok(text);
            }
        }
        bail!("unknown plugin command {command_id}")
    }
}

fn shell_quote(s: &str) -> String {
    if cfg!(windows) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        format!("'{}'", s.replace('\'', "'\\''"))
    }
}

fn shell_command(script: &str) -> Command {
    if cfg!(windows) {
        let mut c = Command::new("cmd");
        c.arg("/C").arg(script);
        c
    } else {
        let mut c = Command::new("sh");
        c.arg("-c").arg(script);
        c
    }
}

fn plugins_dir() -> Option<PathBuf> {
    Some(ide_core::config_dir()?.join("plugins"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_quote_wraps_spaces() {
        let q = shell_quote("/tmp/my project");
        assert!(q.contains("my project") || q.contains("my project"));
        assert!(q.starts_with('\'') || q.starts_with('"'));
    }

    #[tokio::test]
    async fn run_echo_plugin_async() {
        let host = PluginHost {
            plugins: vec![PluginManifest {
                name: "t".into(),
                description: String::new(),
                commands: vec![PluginCommand {
                    id: "t.echo".into(),
                    title: "echo".into(),
                    run: if cfg!(windows) {
                        "echo hello-plugin".into()
                    } else {
                        "echo hello-plugin".into()
                    },
                }],
            }],
        };
        let root = std::env::temp_dir();
        let out = host.run("t.echo", &root, None, 1).await.expect("run");
        assert!(out.contains("hello-plugin"), "{out}");
    }
}
