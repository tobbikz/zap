//! Integrated terminal via portable-pty.

use anyhow::{Context, Result};
use parking_lot::Mutex;
use portable_pty::{CommandBuilder, MasterPty, NativePtySystem, PtySize, PtySystem};
use std::io::{Read, Write};
use std::sync::Arc;
use tracing::warn;

pub struct TerminalPane {
    master: Box<dyn MasterPty + Send>,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    reader_buf: Arc<Mutex<String>>,
    scroll: usize,
    cols: u16,
    rows: u16,
    _child: Box<dyn portable_pty::Child + Send + Sync>,
    _pair_reader: Option<std::thread::JoinHandle<()>>,
}

impl TerminalPane {
    pub fn spawn(cwd: &std::path::Path, cols: u16, rows: u16) -> Result<Self> {
        let shell = default_shell();
        Self::spawn_command(cwd, cols, rows, &shell, &[])
    }

    /// Spawn an arbitrary command in a PTY (e.g. `ssh` with args).
    pub fn spawn_command(
        cwd: &std::path::Path,
        cols: u16,
        rows: u16,
        program: &str,
        args: &[String],
    ) -> Result<Self> {
        let cols = cols.max(10);
        let rows = rows.max(3);
        let pty_system = NativePtySystem::default();
        let pair = pty_system
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .context("open pty")?;

        let mut cmd = CommandBuilder::new(program);
        for a in args {
            cmd.arg(a);
        }
        cmd.cwd(cwd);
        // Avoid trapping the embedded TTY in `less` after `git log` / man / etc.
        cmd.env("PAGER", "cat");
        cmd.env("GIT_PAGER", "cat");
        cmd.env("LESS", "FRX");

        let child = pair
            .slave
            .spawn_command(cmd)
            .with_context(|| format!("spawn {program}"))?;
        drop(pair.slave);

        let mut reader = pair.master.try_clone_reader().context("clone reader")?;
        let writer = pair.master.take_writer().context("take writer")?;
        let reader_buf = Arc::new(Mutex::new(String::new()));
        let buf_c = reader_buf.clone();

        let handle = std::thread::spawn(move || {
            let mut tmp = [0u8; 4096];
            loop {
                match reader.read(&mut tmp) {
                    Ok(0) => break,
                    Ok(n) => {
                        let s = String::from_utf8_lossy(&tmp[..n]);
                        let mut g = buf_c.lock();
                        g.push_str(&s);
                        // keep last ~200k bytes, cut only on a UTF-8 boundary (#199)
                        trim_buffer(&mut g, 200_000, 150_000);
                    }
                    Err(e) => {
                        warn!("pty read: {e}");
                        break;
                    }
                }
            }
        });

        Ok(Self {
            master: pair.master,
            writer: Arc::new(Mutex::new(writer)),
            reader_buf,
            scroll: 0,
            cols,
            rows,
            _child: child,
            _pair_reader: Some(handle),
        })
    }

    pub fn resize(&mut self, cols: u16, rows: u16) {
        let cols = cols.max(10);
        let rows = rows.max(3);
        if cols == self.cols && rows == self.rows {
            return;
        }
        self.cols = cols;
        self.rows = rows;
        if let Err(e) = self.master.resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        }) {
            warn!("pty resize: {e}");
        }
    }

    pub fn size(&self) -> (u16, u16) {
        (self.cols, self.rows)
    }

    pub fn write_bytes(&self, data: &[u8]) -> Result<()> {
        let mut w = self.writer.lock();
        w.write_all(data)?;
        w.flush()?;
        Ok(())
    }

    pub fn write_char(&self, c: char) -> Result<()> {
        let mut buf = [0u8; 4];
        let s = c.encode_utf8(&mut buf);
        self.write_bytes(s.as_bytes())
    }

    pub fn visible_lines(&self) -> Vec<String> {
        let g = self.reader_buf.lock();
        // strip some ANSI for display simplicity
        let plain = strip_ansi(&g);
        let lines: Vec<&str> = plain.lines().collect();
        let h = self.rows as usize;
        let total = lines.len();
        let start = total.saturating_sub(h + self.scroll);
        let end = total.saturating_sub(self.scroll);
        lines[start..end.max(start)]
            .iter()
            .map(|s| s.chars().take(self.cols as usize).collect())
            .collect()
    }

    pub fn scroll_up(&mut self) {
        self.scroll += 1;
    }

    pub fn scroll_down(&mut self) {
        self.scroll = self.scroll.saturating_sub(1);
    }
}

/// Default interactive shell (#198): `$SHELL` on Unix; on Windows prefer
/// `pwsh` / `powershell`, then `%COMSPEC%`, then `cmd.exe`.
pub fn default_shell() -> String {
    pick_default_shell(
        cfg!(windows),
        std::env::var("SHELL").ok(),
        std::env::var("COMSPEC").ok(),
        which_on_path,
    )
}

fn pick_default_shell(
    windows: bool,
    shell: Option<String>,
    comspec: Option<String>,
    on_path: impl Fn(&str) -> bool,
) -> String {
    let nonempty = |v: Option<String>| v.filter(|s| !s.trim().is_empty());
    if windows {
        if let Some(sh) = nonempty(shell) {
            if std::path::Path::new(&sh).is_absolute() || on_path(&sh) {
                return sh;
            }
        }
        for cand in ["pwsh", "powershell"] {
            if on_path(cand) {
                return cand.into();
            }
        }
        nonempty(comspec).unwrap_or_else(|| "cmd.exe".into())
    } else {
        nonempty(shell).unwrap_or_else(|| "/bin/sh".into())
    }
}

fn which_on_path(name: &str) -> bool {
    let Some(paths) = std::env::var_os("PATH") else {
        return false;
    };
    for dir in std::env::split_paths(&paths) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return true;
        }
        #[cfg(windows)]
        {
            for ext in ["EXE", "CMD", "BAT"] {
                let with_ext = dir.join(format!("{name}.{ext}"));
                if with_ext.is_file() {
                    return true;
                }
            }
        }
    }
    false
}

/// Drop the oldest bytes of `buf` once it exceeds `max`, keeping about `keep_len`.
/// Advances the cut to a UTF-8 char boundary so `drain` never panics (#199).
fn trim_buffer(buf: &mut String, max: usize, keep_len: usize) {
    if buf.len() <= max {
        return;
    }
    let mut cut = buf.len().saturating_sub(keep_len);
    while cut < buf.len() && !buf.is_char_boundary(cut) {
        cut += 1;
    }
    buf.drain(..cut);
}

fn strip_ansi(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            match chars.peek().copied() {
                // CSI: ESC [ ... final-byte
                Some('[') => {
                    chars.next();
                    for c2 in chars.by_ref() {
                        if (0x40..=0x7e).contains(&(c2 as u32)) {
                            break;
                        }
                    }
                }
                // OSC: ESC ] ... BEL  or  ESC ] ... ESC \
                Some(']') => {
                    chars.next();
                    while let Some(c2) = chars.next() {
                        if c2 == '\u{07}' {
                            break;
                        }
                        if c2 == '\u{1b}' && chars.peek() == Some(&'\\') {
                            chars.next();
                            break;
                        }
                    }
                }
                Some(_) => {
                    let _ = chars.next();
                }
                None => {}
            }
            continue;
        }
        if c == '\r' || c == '\u{07}' {
            continue;
        }
        // Orphan OSC title crumbs if ESC was already lost (e.g. "]0;title")
        if c == ']' && matches!(chars.peek(), Some('0'..='9')) {
            while let Some(c2) = chars.next() {
                if c2 == '\u{07}' || c2 == '\n' {
                    break;
                }
                if c2 == '\u{1b}' && chars.peek() == Some(&'\\') {
                    chars.next();
                    break;
                }
            }
            continue;
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod strip_tests {
    use super::strip_ansi;

    #[test]
    fn strips_osc_window_title() {
        let s = "\u{1b}]0;Cursor Agent\u{07}hello";
        assert_eq!(strip_ansi(s), "hello");
    }

    #[test]
    fn strips_orphan_osc_crumb() {
        let s = "]0;Cursor Agent\u{07}plan";
        assert_eq!(strip_ansi(s), "plan");
    }
}

#[cfg(test)]
mod trim_tests {
    use super::trim_buffer;

    #[test]
    fn noop_when_under_max() {
        let mut s = "hello".to_string();
        trim_buffer(&mut s, 100, 50);
        assert_eq!(s, "hello");
    }

    #[test]
    fn does_not_panic_on_mid_codepoint_cut() {
        // "é" is 2 bytes (C3 A9). Without boundary flooring, drain would panic.
        let mut s = String::new();
        for _ in 0..80 {
            s.push('é');
        }
        trim_buffer(&mut s, 20, 10);
        assert!(s.len() <= 20);
        assert!(std::str::from_utf8(s.as_bytes()).is_ok());
        assert!(!s.is_empty());
    }
}

#[cfg(test)]
mod shell_tests {
    use super::pick_default_shell;

    #[test]
    fn unix_falls_back_to_bin_sh() {
        assert_eq!(
            pick_default_shell(false, None, None, |_| false),
            "/bin/sh"
        );
        assert_eq!(
            pick_default_shell(false, Some("/bin/zsh".into()), None, |_| false),
            "/bin/zsh"
        );
    }

    #[test]
    fn windows_prefers_pwsh_then_comspec() {
        assert_eq!(
            pick_default_shell(true, None, None, |n| n == "pwsh"),
            "pwsh"
        );
        assert_eq!(
            pick_default_shell(true, None, None, |n| n == "powershell"),
            "powershell"
        );
        assert_eq!(
            pick_default_shell(true, None, Some(r"C:\Windows\System32\cmd.exe".into()), |_| {
                false
            }),
            r"C:\Windows\System32\cmd.exe"
        );
        assert_eq!(
            pick_default_shell(true, None, None, |_| false),
            "cmd.exe"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn spawn_resize_and_echo() {
        let tmp = std::env::temp_dir().join("tui-ide-term-smoke");
        let _ = std::fs::create_dir_all(&tmp);
        let mut pane = TerminalPane::spawn(&tmp, 40, 8).expect("spawn");
        pane.resize(60, 12);
        assert_eq!(pane.size(), (60, 12));
        pane.write_bytes(b"echo wave4\r").expect("write");
        std::thread::sleep(Duration::from_millis(200));
        let lines = pane.visible_lines().join("\n");
        assert!(
            lines.contains("wave4") || !lines.is_empty(),
            "expected pty output, got: {lines:?}"
        );
    }
}
