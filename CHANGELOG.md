# Changelog

## Unreleased

### Added

- TUI design language port from Hop: tuile `Role` tokens, footer key chips, jackin L0 fill, opaque modal scrim, restrained busy spinner (`ZAP_ANIMATIONS=0` to disable). Multi-theme DataRig palettes stay unlocked. See [docs/TUI-DESIGN.md](docs/TUI-DESIGN.md).

## [0.6.1] - 2026-11-18

### Added

- GitHub Release binaries: `zap-linux-x86_64` and `zap-windows-x86_64.exe`. Install scripts download them and only compile from source if the asset is missing.

### Fixed

- Windows PowerShell: the console is switched to UTF-8, raster fonts are upgraded to Consolas for the session, and the UI uses Consolas glyphs (square borders, `|` caret, no Nerd Font private-use icons) so the layout stays aligned. Windows Terminal with Cascadia keeps rounded borders and block carets. A Nerd Font face, `[theme] icons = "nerd"`, or `ZAP_ICONS=nerd` restores the full icon set.

## [0.6.0] - 2026-11-15

### Changed

- **Rebrand to Zap** — display name **Zap** (capital Z), binary `zap`, config `~/.config/zap` (falls back to existing `~/.config/tui-ide`), GitHub `tobbikz/zap`.
