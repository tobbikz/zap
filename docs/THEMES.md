# Themes — Zap

<p align="center">
  <img src="assets/theme-light-preview.png" alt="Live theme preview in palette" width="720" />
</p>

Built-ins (palette: `Ctrl-P` → type `theme`):

| Id | Label |
|----|--------|
| `black` | AMOLED mono |
| `white` | AMOLED light mono |
| `dark` | Dark (default — DataRig teal accent) |
| `nord` | Nord |
| `catppuccin-mocha` | Catppuccin Mocha |
| `tokyo-night` | Tokyo Night |
| `dracula` | Dracula |
| `light` | Light (accent blue) |
| `high-contrast` | High contrast |

- **↑↓** in the palette live-previews a theme  
- **Enter** keeps it (writes `theme.name` in config)  
- **Esc** restores the previous theme  
- **Shift-T** cycles built-ins + customs  

## Custom themes

1. Palette → **Theme: new custom…**  
   or create `~/.config/zap/themes/mytheme.toml` (Windows: `%APPDATA%\zap\themes\`).

2. Edit colours (hex `#rrggbb`). Example:

```toml
extends = "dark"
label = "Ocean"

[colors]
accent = "#89b4fa"
bg = "#0b1220"
fg = "#cdd6f4"
surface = "#111827"
border_focus = "#89b4fa"
syn_keyword = "#cba6f7"
syn_string = "#a6e3a1"
```

3. Palette → **Theme: reload customs**, then pick **Theme: Ocean (custom)**.

`extends` must be a built-in id. Unknown colour keys are ignored; bad files log to stderr and fall back.

Config:

```toml
[theme]
name = "mytheme"
# auto | nerd | safe | ascii   (true/false still work)
icons = "auto"
```

## Icons

`auto` picks a repertoire the host font can draw:

| Host | Icons | Borders | Caret |
|------|--------|---------|--------|
| Linux / macOS | Nerd Font | rounded | block `▌` |
| Windows + Nerd Font face (`… NF`, CaskaydiaCove, …) | Nerd Font | rounded | block |
| Windows Terminal (`WT_SESSION`) | Nerd Font (ConPTY often misreports Consolas) | rounded | block |
| Windows PowerShell / conhost (Consolas) | Consolas-safe (`P` `●` `▪`) | square `┌┐` | vertical bar |

Override with `[theme] icons = "nerd"` (always wins) or `ZAP_ICONS=nerd|safe|ascii`. Press `?` in Studio to see the active set (`icons: nerd|safe|ascii`).

Motion (busy spinner / pulse) can be disabled with `ZAP_ANIMATIONS=0`.

Design language (surfaces, footer chips, L0, modals): **[TUI-DESIGN.md](./TUI-DESIGN.md)**.
