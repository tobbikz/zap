# Install & update — Zap

Requires a [Rust toolchain](https://rustup.rs/) (1.75+) and a terminal with truecolor + a Nerd Font recommended.

## Linux

```bash
git clone https://github.com/tobbikz/zap.git && cd zap
./scripts/install.sh
# binary → ~/.local/bin/zap
```

Ensure `~/.local/bin` is on `PATH`:

```bash
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc   # or ~/.zshrc
```

Update later:

```bash
cd zap && ./scripts/update.sh
```

## Windows

In PowerShell (Developer / user):

```powershell
git clone https://github.com/tobbikz/zap.git
cd zap
.\scripts\install.ps1
# binary → %LOCALAPPDATA%\zap\zap.exe
```

Reopen the terminal after first install so `PATH` picks up the folder.

Update:

```powershell
cd zap
.\scripts\update.ps1
```

## From source (any OS)

```bash
cargo build -p ide-app --release
./target/release/zap          # Unix
.\target\release\zap.exe      # Windows
```

## Config

User config and connections: `~/.config/zap/` (Linux/macOS) or `%APPDATA%\zap\` (Windows via `dirs` crate).

Themes: `…/zap/themes/*.toml` (legacy `tui-ide` still works) — see [THEMES.md](./THEMES.md).
