# Zap TUI design system

Synthesizes the same four references Hop uses (`product-hop` → uses in Brain), adapted for a multi-theme Database IDE:

| Source | What Zap takes |
|--------|----------------|
| [Monospace Design TUI](https://github.com/coreyt/monospace-design-tui) | Master–detail Studio, footer command bar, one focus owner |
| [Charm](https://github.com/charmbracelet) | Restrained chrome, quiet brand masthead, faint help |
| [tuile](https://github.com/Osyna/tuile) | Semantic roles (`text`, `muted`, `panel`, `boost`, `border`, `selection`, `footer_key`) |
| [jackin / Rivora](https://jackin.tailrocks.com/reference/tui/visual-design/) | Surface levels L0–L3, accent focus rail, contextual footer |

**Unlike Hop:** Zap does **not** lock to a single AMOLED mono palette. Built-in themes (dark, nord, mocha, …) and custom TOML stay first-class. Roles map onto the *active* palette.

**DataRig dark (default):** palette values follow [DokaDev/datarig](https://github.com/DokaDev/datarig) `DARK` — teal accent `#4FB3A9`, quiet borders `#2E3440`, selection `#2A3B4D`. Results cursor: whole row on `surface_3`, focused cell accent-filled. Column rule under the header uses border ink with an accent segment on the active column. Console Run chip is accent-filled when Console is focused, accent-ink on `surface_2` when idle; tabs use a tinted active fill.

**AMOLED mono (`black` / `white`):** pure canvas (`bg` = `surface` = wash), invert selection (body ink on canvas), no grey panel hierarchy. DataRig L1/L2 surfaces apply only to chromatic themes.

## Surfaces

| Level | Zap use |
|-------|---------|
| L0 | Full-bleed app background (`theme::bg`) painted before chrome |
| L1 | Studio panes (`pane_block` → `surface` fill) sit above L0 |
| L2 | Soft selection / Results cursor row (`surface_3` + accent rail `▎` / `│` on Consolas) |
| L3 | Modals / palette / discover (`modal` + opaque scrim) |

## Focus

- Exactly one interaction owner.
- Focused pane: accent border + quiet accent title (no heavy bold wash).
- Unfocused: idle border, faint title.
- List selection: left rail (never a full reverse-video wash on DataRig themes).

## Footer

Home and Studio footers use tuile-style chips via `theme::footer_key` / `extend_footer_keys`:

```text
 Enter  open  A  add  W  workspaces  Space  keys  Ctrl-P  palette
```

Busy work (`connecting…` / `RUNNING…`) replaces chips with a braille (or ASCII) spinner.

## Motion

| Effect | Where | Notes |
|--------|-------|-------|
| Braille spinner `⠋⠙…` | header / footer while `busy_label` or query running | 80 ms; ASCII `\|/-\\` on Consolas |
| Live pulse | reserved for live status dots | slow (~900 ms) |

Disable with `ZAP_ANIMATIONS=0` (or `false` / `off` / `no`).

## Modals

`widgets::paint_scrim` / `modal_chrome` erase the stage with a real cell fill (Block background alone can leave ghosts). Palette, connect form, cell inspector, admin, and monitor use the opaque scrim.

## Windows

See [INSTALL.md](./INSTALL.md) and [THEMES.md](./THEMES.md) for Consolas-safe glyphs, UTF-8 console, and `icons = auto|nerd|safe|ascii`.

## Related

- Themes: [THEMES.md](./THEMES.md)
- Hop reference implementation: `~/Projects/hop/docs/TUI-DESIGN.md`
- Brain ticket: `ZAP-TUI-DESIGN-PORT`
