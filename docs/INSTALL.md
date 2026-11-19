# Install & update — Zap

Prebuilt binaries are attached to each [GitHub release](https://github.com/tobbikz/zap/releases). A Rust toolchain is only needed to build from source.

Windows PowerShell is supported: Zap switches the console to UTF-8 and draws Consolas-safe glyphs (square borders, no private-use icons). Windows Terminal with Cascadia Mono looks closest to the Linux UI. A [Nerd Font](https://www.nerdfonts.com/) whose face name contains "Nerd" turns the full icon set back on, or set `[theme] icons = "nerd"`.

## Linux

```bash
mkdir -p ~/.local/bin
curl -fL -o ~/.local/bin/zap https://github.com/tobbikz/zap/releases/latest/download/zap-linux-x86_64
chmod +x ~/.local/bin/zap
```

Ensure `~/.local/bin` is on `PATH`:

```bash
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc   # or ~/.zshrc
```

From a git checkout (downloads the release, or builds if the asset is missing):

```bash
git clone https://github.com/tobbikz/zap.git && cd zap
./scripts/install.sh
```

Update later:

```bash
cd zap && ./scripts/update.sh
```

## Windows

In PowerShell:

```powershell
irm https://raw.githubusercontent.com/tobbikz/zap/main/scripts/install.ps1 | iex
# binary → %LOCALAPPDATA%\zap\zap.exe
```

Or, from a git checkout:

```powershell
git clone https://github.com/tobbikz/zap.git
cd zap
.\scripts\install.ps1
```

Reopen the terminal after first install so `PATH` picks up the folder.

Update:

```powershell
cd zap
.\scripts\update.ps1
```

Direct download:

```powershell
$dir = Join-Path $env:LOCALAPPDATA "zap"
New-Item -ItemType Directory -Force -Path $dir | Out-Null
Invoke-WebRequest https://github.com/tobbikz/zap/releases/latest/download/zap-windows-x86_64.exe -OutFile (Join-Path $dir "zap.exe")
```

## From source (any OS)

Requires [Rust](https://rustup.rs/) 1.75+.

```bash
cargo build -p ide-app --release
./target/release/zap          # Unix
.\target\release\zap.exe      # Windows
```

## Config

User config and connections: `~/.config/zap/` (Linux/macOS) or `%APPDATA%\zap\` (Windows via `dirs` crate).

Themes: `…/zap/themes/*.toml` (legacy `tui-ide` still works) — see [THEMES.md](./THEMES.md).

Icons:

```toml
[theme]
icons = "auto"   # nerd | safe | ascii
```

`ZAP_ICONS=safe` overrides that for one launch.
