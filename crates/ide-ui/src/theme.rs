//! Visual tokens — DataRig-inspired built-in themes (truecolor) + custom TOML.

#[path = "theme_custom.rs"]
mod theme_custom;

pub use theme_custom::{create_custom_theme, customs, reload_customs, themes_dir, OwnedPalette};

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use theme_custom::{active, find_custom, set_active};

/// Which icon repertoire the host font can draw.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum GlyphSet {
    /// Nerd Font private-use glyphs (JetBrainsMono Nerd Font, etc.).
    Nerd = 0,
    /// Symbols that ship in Consolas / Cascadia (no private-use area).
    Safe = 1,
    /// Plain ASCII.
    Ascii = 2,
}

/// What the active console font can actually render.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontClass {
    /// Face name looks like a Nerd Font.
    Nerd,
    /// Modern monospace with box-drawing, blocks, and rounded corners (Cascadia, JetBrains, …).
    Modern,
    /// Consolas, Lucida Console, raster — WGL4 only.
    Legacy,
    /// No face reported (not a Windows console, or the query failed).
    Unknown,
}

static GLYPHS: AtomicU8 = AtomicU8::new(GlyphSet::Nerd as u8);
static ROUNDED: AtomicBool = AtomicBool::new(true);
static BLOCKS: AtomicBool = AtomicBool::new(true);
static TRUECOLOR: AtomicBool = AtomicBool::new(true);

pub fn set_glyph_set(set: GlyphSet) {
    GLYPHS.store(set as u8, Ordering::Relaxed);
}

pub fn glyph_set() -> GlyphSet {
    match GLYPHS.load(Ordering::Relaxed) {
        x if x == GlyphSet::Safe as u8 => GlyphSet::Safe,
        x if x == GlyphSet::Ascii as u8 => GlyphSet::Ascii,
        _ => GlyphSet::Nerd,
    }
}

/// `true` keeps Nerd Font glyphs; `false` forces ASCII.
pub fn set_icons(on: bool) {
    set_glyph_set(if on {
        GlyphSet::Nerd
    } else {
        GlyphSet::Ascii
    });
}

pub fn icons_on() -> bool {
    glyph_set() != GlyphSet::Ascii
}

pub fn set_rounded_borders(on: bool) {
    ROUNDED.store(on, Ordering::Relaxed);
}

pub fn set_block_glyphs(on: bool) {
    BLOCKS.store(on, Ordering::Relaxed);
}

pub fn block_glyphs() -> bool {
    BLOCKS.load(Ordering::Relaxed)
}

/// Set whether the host terminal supports 24-bit truecolor (#74).
/// When `false`, themes should degrade to 16-color ANSI.
pub fn set_truecolor(on: bool) {
    TRUECOLOR.store(on, Ordering::Relaxed);
}

/// `true` when the host was detected as truecolor-capable.
pub fn truecolor() -> bool {
    TRUECOLOR.load(Ordering::Relaxed)
}

/// Rounded corners need Cascadia / a Nerd Font. Consolas only has square box-drawing.
pub fn border_type() -> ratatui::widgets::BorderType {
    if ROUNDED.load(Ordering::Relaxed) {
        ratatui::widgets::BorderType::Rounded
    } else {
        ratatui::widgets::BorderType::Plain
    }
}

pub fn caret() -> &'static str {
    if block_glyphs() { "▌" } else { "|" }
}

pub fn caret_char() -> char {
    if block_glyphs() { '▌' } else { '|' }
}

/// Explorer / gutter selection rail glyph (legacy). Prefer `select_rail_span` —
/// a space with accent *background* fills the whole cell (no black gap beside `▎`).
pub fn select_mark() -> &'static str {
    " "
}

/// Solid 1-cell accent rail for selected rows (Explorer, gutters, inspectors).
pub fn select_rail_span(selected: bool) -> ratatui::text::Span<'static> {
    use ratatui::style::Style;
    use ratatui::text::Span;
    if selected && !is_mono() {
        Span::styled(" ", Style::default().bg(accent()))
    } else {
        Span::styled(" ", Style::default())
    }
}

pub fn swatch() -> &'static str {
    if block_glyphs() { "██" } else { "■■" }
}

/// Classify a Windows console face name. Empty means we could not read a face.
pub fn classify_font(face: &str) -> FontClass {
    let f = face.trim().to_lowercase();
    if f.is_empty() {
        return FontClass::Unknown;
    }
    // Nerd Font / NF / "CaskaydiaCove Nerd Font" / "CascadiaMono NF" (#3).
    if f.contains("nerd")
        || f.contains(" nf")
        || f.ends_with("nf")
        || f.contains("nerd font")
        || f.contains("symbols nerd")
        || f.contains("caskaydia")
    {
        return FontClass::Nerd;
    }
    const MODERN: &[&str] = &[
        "cascadia",
        "jetbrains",
        "fira",
        "iosevka",
        "hack",
        "dejavu",
        "source code",
        "meslo",
        "victor",
        "intel one",
        "recursive",
        "monaspace",
        "commit mono",
        "geist mono",
        "ibm plex mono",
        "ubuntu mono",
        "noto sans mono",
        "sarasa",
    ];
    if MODERN.iter().any(|name| f.contains(name)) {
        FontClass::Modern
    } else {
        FontClass::Legacy
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Palette {
    pub id: &'static str,
    pub label: &'static str,
    pub light: bool,
    pub bg: Color,
    pub surface: Color,
    pub surface_2: Color,
    pub surface_3: Color,
    pub modal: Color,
    pub border_idle: Color,
    pub border_focus: Color,
    pub text: Color,
    pub muted: Color,
    pub faint: Color,
    pub accent: Color,
    pub accent_warm: Color,
    pub warn: Color,
    pub danger: Color,
    pub success: Color,
    pub info: Color,
    pub syn_keyword: Color,
    pub syn_string: Color,
    pub syn_number: Color,
    pub syn_comment: Color,
    pub syn_function: Color,
    pub syn_ident: Color,
}

const fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::Rgb(r, g, b)
}

/// Built-ins (DataRig-inspired names + AMOLED mono).
pub static PALETTES: &[Palette] = &[
    // 0 — AMOLED mono: pure black canvas; no grey panel wash (DataRig surfaces stay on `dark`+)
    Palette {
        id: "black",
        label: "AMOLED mono",
        light: false,
        bg: rgb(0, 0, 0),
        surface: rgb(0, 0, 0),
        surface_2: rgb(0, 0, 0),
        surface_3: rgb(0, 0, 0), // selection uses invert — not a grey step
        modal: rgb(0, 0, 0),
        border_idle: rgb(42, 42, 42),
        border_focus: rgb(220, 220, 220),
        text: rgb(230, 230, 230),
        muted: rgb(140, 140, 140),
        faint: rgb(90, 90, 90),
        accent: rgb(230, 230, 230),
        accent_warm: rgb(200, 200, 200),
        warn: rgb(180, 180, 180),
        danger: rgb(239, 68, 68),
        success: rgb(34, 197, 94),
        info: rgb(140, 140, 140),
        syn_keyword: rgb(230, 230, 230),
        syn_string: rgb(160, 160, 160),
        syn_number: rgb(210, 210, 210),
        syn_comment: rgb(90, 90, 90),
        syn_function: rgb(200, 200, 200),
        syn_ident: rgb(180, 180, 180),
    },
    // 1 — AMOLED light mono (pure white / black)
    Palette {
        id: "white",
        label: "AMOLED light mono",
        light: true,
        bg: rgb(255, 255, 255),
        surface: rgb(255, 255, 255),
        surface_2: rgb(255, 255, 255),
        surface_3: rgb(255, 255, 255),
        modal: rgb(255, 255, 255),
        border_idle: rgb(210, 210, 210),
        border_focus: rgb(40, 40, 40),
        text: rgb(16, 16, 16),
        muted: rgb(100, 100, 100),
        faint: rgb(150, 150, 150),
        accent: rgb(16, 16, 16),
        accent_warm: rgb(60, 60, 60),
        warn: rgb(80, 80, 80),
        danger: rgb(220, 38, 38),
        success: rgb(22, 163, 74),
        info: rgb(100, 100, 100),
        syn_keyword: rgb(16, 16, 16),
        syn_string: rgb(80, 80, 80),
        syn_number: rgb(40, 40, 40),
        syn_comment: rgb(150, 150, 150),
        syn_function: rgb(40, 40, 40),
        syn_ident: rgb(30, 30, 30),
    },
    // 2 — Zap default: charcoal gray + arc-blue accent
    Palette {
        id: "zap",
        label: "Zap",
        light: false,
        bg: rgb(0x0a, 0x0a, 0x0a),
        surface: rgb(0x1a, 0x1a, 0x1a),
        surface_2: rgb(0x21, 0x21, 0x21),
        surface_3: rgb(0x30, 0x30, 0x30), // selection
        modal: rgb(0x1a, 0x1a, 0x1a),
        border_idle: rgb(0x40, 0x40, 0x40),
        border_focus: rgb(0x60, 0xa5, 0xfa), // arc blue
        text: rgb(0xee, 0xee, 0xee),
        muted: rgb(0xa3, 0xa3, 0xa3),
        faint: rgb(0x73, 0x73, 0x73),
        accent: rgb(0x60, 0xa5, 0xfa),
        accent_warm: rgb(0x93, 0xc5, 0xfd),
        warn: rgb(0xe5, 0xc0, 0x7b),
        danger: rgb(0xe0, 0x6c, 0x75),
        success: rgb(0x7f, 0xd8, 0x8f),
        info: rgb(0x56, 0xb6, 0xc2),
        syn_keyword: rgb(0x9d, 0x7c, 0xd8),
        syn_string: rgb(0x7f, 0xd8, 0x8f),
        syn_number: rgb(0x93, 0xc5, 0xfd),
        syn_comment: rgb(0x6a, 0x6a, 0x6a),
        syn_function: rgb(0x60, 0xa5, 0xfa),
        syn_ident: rgb(0xe0, 0xe0, 0xe0),
    },
    // 3 — DataRig dark (teal accent)
    Palette {
        id: "dark",
        label: "Dark",
        light: false,
        bg: rgb(0x14, 0x16, 0x1B),
        surface: rgb(0x1B, 0x1E, 0x25),
        surface_2: rgb(0x20, 0x24, 0x2C),
        surface_3: rgb(0x2A, 0x3B, 0x4D), // selection / grid cursor cell
        modal: rgb(0x1B, 0x1E, 0x25),
        border_idle: rgb(0x2E, 0x34, 0x40),
        border_focus: rgb(0x4F, 0xB3, 0xA9),
        text: rgb(0xD8, 0xDE, 0xE9),
        muted: rgb(0x8A, 0x93, 0xA6),
        faint: rgb(0x5C, 0x65, 0x77),
        accent: rgb(0x4F, 0xB3, 0xA9),
        accent_warm: rgb(0xE0, 0xA9, 0x6D),
        warn: rgb(0xE6, 0xC3, 0x5C),
        danger: rgb(0xE0, 0x6C, 0x75),
        success: rgb(0x8F, 0xC7, 0x7A),
        info: rgb(0x61, 0xAF, 0xEF),
        syn_keyword: rgb(0xC7, 0x92, 0xEA),
        syn_string: rgb(0x9C, 0xCB, 0x85),
        syn_number: rgb(0xE0, 0xA9, 0x6D),
        syn_comment: rgb(0x5C, 0x65, 0x77),
        syn_function: rgb(0x6F, 0xA8, 0xDC),
        syn_ident: rgb(0xD8, 0xDE, 0xE9),
    },
    // 4 — Nord
    Palette {
        id: "nord",
        label: "Nord",
        light: false,
        bg: rgb(46, 52, 64),
        surface: rgb(59, 66, 82),
        surface_2: rgb(67, 76, 94),
        surface_3: rgb(76, 86, 106),
        modal: rgb(59, 66, 82),
        border_idle: rgb(76, 86, 106),
        border_focus: rgb(136, 192, 208),
        text: rgb(236, 239, 244),
        muted: rgb(216, 222, 233),
        faint: rgb(129, 161, 193),
        accent: rgb(136, 192, 208),
        accent_warm: rgb(235, 203, 139),
        warn: rgb(235, 203, 139),
        danger: rgb(191, 97, 106),
        success: rgb(163, 190, 140),
        info: rgb(129, 161, 193),
        syn_keyword: rgb(129, 161, 193),
        syn_string: rgb(163, 190, 140),
        syn_number: rgb(180, 142, 173),
        syn_comment: rgb(97, 110, 136),
        syn_function: rgb(136, 192, 208),
        syn_ident: rgb(216, 222, 233),
    },
    // 3 — Catppuccin Mocha
    Palette {
        id: "catppuccin-mocha",
        label: "Catppuccin Mocha",
        light: false,
        bg: rgb(30, 30, 46),
        surface: rgb(49, 50, 68),
        surface_2: rgb(69, 71, 90),
        surface_3: rgb(88, 91, 112),
        modal: rgb(49, 50, 68),
        border_idle: rgb(69, 71, 90),
        border_focus: rgb(137, 180, 250),
        text: rgb(205, 214, 244),
        muted: rgb(186, 194, 222),
        faint: rgb(108, 112, 134),
        accent: rgb(137, 180, 250),
        accent_warm: rgb(250, 179, 135),
        warn: rgb(249, 226, 175),
        danger: rgb(243, 139, 168),
        success: rgb(166, 227, 161),
        info: rgb(148, 226, 213),
        syn_keyword: rgb(203, 166, 247),
        syn_string: rgb(166, 227, 161),
        syn_number: rgb(250, 179, 135),
        syn_comment: rgb(108, 112, 134),
        syn_function: rgb(137, 180, 250),
        syn_ident: rgb(205, 214, 244),
    },
    // 4 — Tokyo Night
    Palette {
        id: "tokyo-night",
        label: "Tokyo Night",
        light: false,
        bg: rgb(26, 27, 38),
        surface: rgb(36, 40, 59),
        surface_2: rgb(41, 46, 66),
        surface_3: rgb(59, 66, 97),
        modal: rgb(36, 40, 59),
        border_idle: rgb(59, 66, 97),
        border_focus: rgb(122, 162, 247),
        text: rgb(192, 202, 245),
        muted: rgb(169, 177, 214),
        faint: rgb(86, 95, 137),
        accent: rgb(122, 162, 247),
        accent_warm: rgb(224, 175, 104),
        warn: rgb(224, 175, 104),
        danger: rgb(247, 118, 142),
        success: rgb(158, 206, 106),
        info: rgb(125, 207, 255),
        syn_keyword: rgb(187, 154, 247),
        syn_string: rgb(158, 206, 106),
        syn_number: rgb(255, 158, 100),
        syn_comment: rgb(86, 95, 137),
        syn_function: rgb(122, 162, 247),
        syn_ident: rgb(192, 202, 245),
    },
    // 5 — Dracula
    Palette {
        id: "dracula",
        label: "Dracula",
        light: false,
        bg: rgb(40, 42, 54),
        surface: rgb(68, 71, 90),
        surface_2: rgb(98, 114, 164),
        surface_3: rgb(68, 71, 90),
        modal: rgb(68, 71, 90),
        border_idle: rgb(98, 114, 164),
        border_focus: rgb(189, 147, 249),
        text: rgb(248, 248, 242),
        muted: rgb(189, 147, 249),
        faint: rgb(98, 114, 164),
        accent: rgb(139, 233, 253),
        accent_warm: rgb(241, 250, 140),
        warn: rgb(255, 184, 108),
        danger: rgb(255, 85, 85),
        success: rgb(80, 250, 123),
        info: rgb(139, 233, 253),
        syn_keyword: rgb(255, 121, 198),
        syn_string: rgb(241, 250, 140),
        syn_number: rgb(189, 147, 249),
        syn_comment: rgb(98, 114, 164),
        syn_function: rgb(80, 250, 123),
        syn_ident: rgb(248, 248, 242),
    },
    // 6 — Light / bone
    Palette {
        id: "light",
        label: "Light",
        light: true,
        bg: rgb(250, 250, 250),
        surface: rgb(245, 245, 245),
        surface_2: rgb(235, 235, 235),
        surface_3: rgb(225, 225, 225),
        modal: rgb(255, 255, 255),
        border_idle: rgb(210, 210, 210),
        border_focus: rgb(30, 102, 245),
        text: rgb(24, 24, 27),
        muted: rgb(82, 82, 91),
        faint: rgb(161, 161, 170),
        accent: rgb(30, 102, 245),
        accent_warm: rgb(180, 83, 9),
        warn: rgb(161, 98, 7),
        danger: rgb(220, 38, 38),
        success: rgb(22, 163, 74),
        info: rgb(37, 99, 235),
        syn_keyword: rgb(109, 40, 217),
        syn_string: rgb(21, 128, 61),
        syn_number: rgb(194, 65, 12),
        syn_comment: rgb(161, 161, 170),
        syn_function: rgb(37, 99, 235),
        syn_ident: rgb(39, 39, 42),
    },
    // 7 — High contrast
    Palette {
        id: "high-contrast",
        label: "High contrast",
        light: false,
        bg: rgb(0, 0, 0),
        surface: rgb(0, 0, 0),
        surface_2: rgb(20, 20, 20),
        surface_3: rgb(40, 40, 40),
        modal: rgb(0, 0, 0),
        border_idle: rgb(120, 120, 120),
        border_focus: rgb(255, 255, 255),
        text: rgb(255, 255, 255),
        muted: rgb(200, 200, 200),
        faint: rgb(140, 140, 140),
        accent: rgb(0, 255, 255),
        accent_warm: rgb(255, 255, 0),
        warn: rgb(255, 255, 0),
        danger: rgb(255, 0, 0),
        success: rgb(0, 255, 0),
        info: rgb(0, 200, 255),
        syn_keyword: rgb(255, 255, 255),
        syn_string: rgb(0, 255, 0),
        syn_number: rgb(255, 255, 0),
        syn_comment: rgb(140, 140, 140),
        syn_function: rgb(0, 255, 255),
        syn_ident: rgb(255, 255, 255),
    },
];

/// Legacy alias kept for call sites.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeMode {
    Black,
    White,
}

pub fn set_mode(mode: ThemeMode) {
    set_mode_name(match mode {
        ThemeMode::Black => "black",
        ThemeMode::White => "white",
    });
}

pub fn builtin_by_id(id: &str) -> Option<&'static Palette> {
    PALETTES.iter().find(|p| p.id == id)
}

pub fn resolve_theme_id(name: &str) -> String {
    let n = name.trim().to_ascii_lowercase();
    match n.as_str() {
        "amoled-light" | "amoled_light" | "light-mono" | "light_mono" => "white".into(),
        "bone" => "light".into(),
        "mocha" | "catppuccin" => "catppuccin-mocha".into(),
        "tokyo" | "tokyonight" => "tokyo-night".into(),
        "oc" | "open-code" | "open_code" | "opencode" | "gray" | "grey" | "default" => {
            "zap".into()
        }
        "hc" | "contrast" => "high-contrast".into(),
        other => other.to_string(),
    }
}

pub fn set_mode_name(name: &str) {
    let idx = resolve_theme_id(name);
    if let Some(p) = builtin_by_id(&idx) {
        set_active(OwnedPalette::from_builtin(p));
        return;
    }
    if let Some(c) = find_custom(&idx) {
        set_active(c);
        return;
    }
    // Fallback: product default
    if let Some(p) = builtin_by_id("zap") {
        set_active(OwnedPalette::from_builtin(p));
    }
}

pub fn current_id() -> String {
    active().id
}

pub fn current_label() -> String {
    active().label
}

/// Built-ins + custom themes for palette / docs.
pub fn list_themes() -> Vec<(String, String)> {
    let mut v: Vec<(String, String)> = PALETTES
        .iter()
        .map(|p| (p.id.to_string(), p.label.to_string()))
        .collect();
    for c in customs() {
        v.push((c.id, c.label));
    }
    v
}

pub fn cycle_theme(forward: bool) -> String {
    let all = list_themes();
    if all.is_empty() {
        return "zap".into();
    }
    let cur = current_id();
    let i = all.iter().position(|(id, _)| id == &cur).unwrap_or(0);
    let next = if forward {
        (i + 1) % all.len()
    } else {
        i.checked_sub(1).unwrap_or(all.len() - 1)
    };
    let id = all[next].0.clone();
    set_mode_name(&id);
    id
}

fn light() -> bool {
    active().light
}

/// AMOLED black/white — pure canvas, invert selection; no DataRig grey hierarchy.
pub fn is_mono() -> bool {
    matches!(active().id.as_str(), "black" | "white")
}

/// Pane / toolbar fill: pure `bg` on mono, L1 `surface` on chromatic themes.
pub fn pane_fill() -> Color {
    if is_mono() {
        bg()
    } else {
        surface()
    }
}

/// Soft wash behind chips/tabs — skipped on mono so the bar stays true black/white.
pub fn chrome_wash() -> Color {
    if is_mono() {
        bg()
    } else {
        surface_2()
    }
}

/// Classic AMOLED selection: invert body ink onto the canvas (never a grey slab).
fn style_mono_invert() -> Style {
    Style::default()
        .fg(bg())
        .bg(text())
        .add_modifier(Modifier::BOLD)
}

pub fn bg() -> Color {
    active().bg
}
pub fn surface() -> Color {
    active().surface
}
pub fn surface_2() -> Color {
    active().surface_2
}
pub fn surface_3() -> Color {
    degrade_rgb(active().surface_3)
}
pub fn modal() -> Color {
    active().modal
}
/// Opaque modal wash — dark but not pure black so Studio chrome can peek (#286).
pub fn scrim() -> Color {
    if light() {
        rgb(200, 200, 205)
    } else if truecolor() {
        rgb(6, 8, 12)
    } else {
        Color::Black
    }
}
/// Dither fg for frosted scrim (half-shade cells).
pub fn scrim_dither() -> Color {
    if light() {
        rgb(160, 160, 168)
    } else if truecolor() {
        rgb(28, 32, 40)
    } else {
        Color::DarkGray
    }
}

/// Soft list/explorer selection fill — never reverse-video white on chromatic themes (#21).
pub fn selection_bg() -> Color {
    if is_mono() {
        text()
    } else if truecolor() {
        surface_3()
    } else {
        // Legacy 16-color hosts: DarkGray slab, not White invert.
        Color::DarkGray
    }
}

/// Soften RGB tokens when the host cannot do 24-bit (#21 / #74).
fn degrade_rgb(c: Color) -> Color {
    if truecolor() {
        return c;
    }
    match c {
        Color::Rgb(r, g, b) => {
            let lum = (u16::from(r) + u16::from(g) + u16::from(b)) / 3;
            if lum < 48 {
                Color::Black
            } else if lum < 120 {
                Color::DarkGray
            } else if lum < 200 {
                Color::Gray
            } else {
                Color::White
            }
        }
        other => other,
    }
}
pub fn border_idle() -> Color {
    active().border_idle
}
pub fn border_focus() -> Color {
    active().border_focus
}
pub fn text() -> Color {
    active().text
}
pub fn muted() -> Color {
    active().muted
}
pub fn faint() -> Color {
    active().faint
}
pub fn accent() -> Color {
    active().accent
}
pub fn accent_warm() -> Color {
    active().accent_warm
}
pub fn warn() -> Color {
    active().warn
}
pub fn error_c() -> Color {
    active().danger
}
pub fn info() -> Color {
    active().info
}
pub fn success() -> Color {
    active().success
}
pub fn danger() -> Color {
    active().danger
}

pub fn style_ok() -> Style {
    Style::default()
        .fg(success())
        .add_modifier(Modifier::BOLD)
}
pub fn style_err() -> Style {
    Style::default()
        .fg(danger())
        .add_modifier(Modifier::BOLD)
}
pub fn style_ok_badge() -> Style {
    Style::default()
        .fg(contrast_on(success()))
        .bg(success())
        .add_modifier(Modifier::BOLD)
}
pub fn style_err_badge() -> Style {
    Style::default()
        .fg(contrast_on(danger()))
        .bg(danger())
        .add_modifier(Modifier::BOLD)
}
pub fn code_pill() -> Color {
    accent()
}
pub fn db_pill() -> Color {
    muted()
}

pub fn style_text() -> Style {
    Style::default().fg(text())
}
pub fn style_muted() -> Style {
    Style::default().fg(muted())
}
pub fn style_faint() -> Style {
    Style::default().fg(faint())
}
pub fn style_accent() -> Style {
    Style::default().fg(accent())
}
pub fn style_warn() -> Style {
    Style::default().fg(warn()).add_modifier(Modifier::BOLD)
}
pub fn style_title(focused: bool) -> Style {
    if focused {
        Style::default()
            .fg(accent())
            .add_modifier(Modifier::BOLD)
    } else if is_mono() {
        Style::default().fg(muted())
    } else {
        // DataRig chromatic: quieter idle titles.
        Style::default().fg(faint())
    }
}
pub fn style_selected() -> Style {
    if is_mono() {
        style_mono_invert()
    } else {
        // Soft row selection (Explorer / lists) — never a full reverse-video wash (#21).
        Style::default()
            .fg(text())
            .bg(selection_bg())
            .add_modifier(Modifier::BOLD)
    }
}

/// Results grid: whole cursor row — selection tint so the row is obvious.
pub fn style_grid_cursor_row() -> Style {
    if is_mono() {
        style_mono_invert()
    } else {
        Style::default().bg(selection_bg()).fg(text())
    }
}

/// Results grid: focused cell — solid “cuadrado” (accent fill or AMOLED invert).
pub fn style_grid_cursor_cell() -> Style {
    if accent_fill_safe() {
        Style::default()
            .fg(contrast_on(accent()))
            .bg(accent())
            .add_modifier(Modifier::BOLD)
    } else {
        style_mono_invert()
    }
}

/// Results header cell for the cursor column (DataRig `grid_cursor_header`).
pub fn style_grid_cursor_header() -> Style {
    if is_mono() {
        // Bold only — the shared rule under the header is enough (no double underline).
        Style::default()
            .fg(text())
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(accent())
            .bg(surface_2())
            .add_modifier(Modifier::BOLD)
    }
}

/// Active console / results tab fill (DataRig `tab_active` — accent blended into bg).
pub fn style_tab_active() -> Style {
    if is_mono() {
        style_mono_invert()
    } else {
        Style::default()
            .fg(text())
            .bg(selection_bg())
            .add_modifier(Modifier::BOLD)
    }
}

/// Quiet toolbar chip (db / schema / lim).
pub fn style_chip() -> Style {
    Style::default().fg(muted()).bg(chrome_wash())
}

/// Run chip when Console is focused — filled accent when safe; mono uses inverted chip.
pub fn style_chip_run() -> Style {
    if accent_fill_safe() {
        Style::default()
            .fg(contrast_on(accent()))
            .bg(accent())
            .add_modifier(Modifier::BOLD)
    } else {
        style_mono_invert()
    }
}

/// Run chip when Console is idle — accent ink on surface (quiet “off” state).
pub fn style_chip_run_idle() -> Style {
    Style::default()
        .fg(if accent_fill_safe() { accent() } else { muted() })
        .bg(chrome_wash())
        .add_modifier(Modifier::BOLD)
}

/// Strong inverted chip (badges / running) — not for list rows.
pub fn style_selected_strong() -> Style {
    Style::default()
        .fg(contrast_on(accent()))
        .bg(accent())
        .add_modifier(Modifier::BOLD)
}
pub fn style_pill_on(bg_c: Color) -> Style {
    Style::default()
        .fg(contrast_on(bg_c))
        .bg(bg_c)
        .add_modifier(Modifier::BOLD)
}
pub fn style_pill_off() -> Style {
    Style::default().fg(muted()).bg(surface_2())
}

/// Studio pane block — L1 surface on chromatic themes; pure `bg` on AMOLED mono.
pub fn pane_block(title: impl Into<String>, focused: bool) -> ratatui::widgets::Block<'static> {
    use ratatui::widgets::{Block, Borders};
    let raw: String = title.into();
    // Breath around the title label (DataRig-style air).
    let title = if raw.starts_with(' ') || raw.is_empty() {
        raw
    } else {
        format!(" {raw} ")
    };
    Block::default()
        .borders(Borders::ALL)
        .border_type(border_type())
        .title(ratatui::text::Span::styled(title, style_title(focused)))
        .border_style(Style::default().fg(border(focused)))
        .style(Style::default().bg(pane_fill()))
}
pub fn border(focused: bool) -> Color {
    if focused {
        border_focus()
    } else {
        border_idle()
    }
}

/// True when `accent` is chromatic enough to use as a solid fill (not near-grey mono).
fn accent_fill_safe() -> bool {
    match accent() {
        Color::Rgb(r, g, b) => {
            let max = r.max(g).max(b);
            let min = r.min(g).min(b);
            // Chroma: teal/blue/etc. have spread; AMOLED mono accents are near-grey.
            max.saturating_sub(min) >= 28
        }
        _ => true,
    }
}

fn luminance(c: Color) -> u16 {
    match c {
        Color::Rgb(r, g, b) => {
            (u32::from(r) * 299 + u32::from(g) * 587 + u32::from(b) * 114) / 1000
        }
        _ => {
            if light() {
                240
            } else {
                20
            }
        }
    }
    .min(255) as u16
}

/// Pick readable fg for a filled chip (light/dark heuristic).
fn contrast_on(bg_c: Color) -> Color {
    if luminance(bg_c) > 140 {
        Color::Rgb(16, 16, 16)
    } else {
        Color::Rgb(245, 245, 245)
    }
}

pub fn syn_keyword() -> Color {
    active().syn_keyword
}
pub fn syn_string() -> Color {
    active().syn_string
}
pub fn syn_number() -> Color {
    active().syn_number
}
pub fn syn_comment() -> Color {
    active().syn_comment
}
pub fn syn_function() -> Color {
    active().syn_function
}
pub fn syn_ident() -> Color {
    active().syn_ident
}

/// tuile-style semantic roles mapped onto the active palette.
/// Zap keeps multi-theme DataRig palettes; roles stay theme-relative (unlike Hop's locked mono).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Text,
    Muted,
    Faint,
    Panel,
    Boost,
    Border,
    BorderFocus,
    Selection,
    Accent,
    FooterKey,
    Success,
    Danger,
    Warning,
}

impl Role {
    pub fn style(self) -> Style {
        match self {
            Role::Text => style_text(),
            Role::Muted => style_muted(),
            Role::Faint => style_faint(),
            Role::Panel => Style::default().bg(surface()),
            Role::Boost => Style::default().bg(surface_2()),
            Role::Border => Style::default().fg(border_idle()),
            Role::BorderFocus => Style::default().fg(border_focus()),
            Role::Selection => style_selected(),
            Role::Accent => style_accent().add_modifier(Modifier::BOLD),
            Role::FooterKey => Style::default().fg(accent()).add_modifier(Modifier::BOLD),
            Role::Success => style_ok(),
            Role::Danger => style_err(),
            Role::Warning => style_warn(),
        }
    }
}

/// Charm/tuile footer chip: accent key + faint label.
pub fn footer_key(key: &str, label: &str) -> Vec<Span<'static>> {
    vec![
        Span::styled(format!(" {key} "), Role::FooterKey.style()),
        Span::styled(format!("{label} "), Role::Faint.style()),
    ]
}

/// Append several footer chips into `out`.
pub fn extend_footer_keys(out: &mut Vec<Span<'static>>, keys: &[(&str, &str)]) {
    for (k, l) in keys {
        out.extend(footer_key(k, l));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn font_class_matches_windows_faces() {
        assert_eq!(classify_font(""), FontClass::Unknown);
        assert_eq!(classify_font("Consolas"), FontClass::Legacy);
        assert_eq!(classify_font("Lucida Console"), FontClass::Legacy);
        assert_eq!(classify_font("Cascadia Mono"), FontClass::Modern);
        assert_eq!(classify_font("JetBrainsMono Nerd Font"), FontClass::Nerd);
        assert_eq!(classify_font("FiraCode NF"), FontClass::Nerd);
        assert_eq!(classify_font("CaskaydiaCove Nerd Font"), FontClass::Nerd);
        assert_eq!(classify_font("CascadiaMono NF"), FontClass::Nerd);
    }

    #[test]
    fn footer_key_has_key_and_label() {
        let spans = footer_key("Ctrl-P", "palette");
        assert_eq!(spans.len(), 2);
        assert!(spans[0].content.contains("Ctrl-P"));
        assert!(spans[1].content.contains("palette"));
    }

    #[test]
    fn role_selection_is_soft_not_reverse_default() {
        // Soft selection uses selection_bg + bold text (DataRig), not Hop reverse wash.
        set_mode_name("dark");
        set_truecolor(true);
        let s = Role::Selection.style();
        assert_eq!(s.bg, Some(selection_bg()));
        assert_ne!(s.bg, Some(text()), "chromatic must not invert to white wash");
    }

    #[test]
    fn selection_stays_soft_when_glyphs_are_safe() {
        set_mode_name("zap");
        set_truecolor(true);
        set_glyph_set(GlyphSet::Safe);
        let s = style_selected();
        assert_eq!(s.bg, Some(selection_bg()));
        assert_ne!(s.bg, Some(Color::White));
        assert!(!s.add_modifier.contains(Modifier::REVERSED));
        set_glyph_set(GlyphSet::Nerd);
    }

    #[test]
    fn selection_bg_is_darkgray_without_truecolor() {
        set_mode_name("zap");
        set_truecolor(false);
        assert_eq!(selection_bg(), Color::DarkGray);
        set_truecolor(true);
    }

    #[test]
    fn amoled_black_is_pure_canvas() {
        set_mode_name("black");
        assert!(is_mono());
        assert_eq!(bg(), surface());
        assert_eq!(pane_fill(), bg());
        assert_eq!(chrome_wash(), bg());
        // Selection inverts — no grey slab.
        let s = style_selected();
        assert_eq!(s.bg, Some(text()));
        assert_eq!(s.fg, Some(bg()));
    }

    #[test]
    fn grid_cursor_cell_readable_on_amoled_black() {
        set_mode_name("black");
        let s = style_grid_cursor_cell();
        // Invert: dark ink on light bar (accent==text on mono — that's fine).
        assert_eq!(s.bg, Some(text()));
        assert_eq!(s.fg, Some(bg()));
        assert_ne!(s.fg, s.bg);
    }

    #[test]
    fn grid_cursor_cell_accent_fill_on_datarig_dark() {
        set_mode_name("dark");
        assert!(!is_mono());
        let s = style_grid_cursor_cell();
        assert_eq!(s.bg, Some(accent()));
    }

    #[test]
    fn theme_ids_normalize_case() {
        assert_eq!(resolve_theme_id("Solarized"), "solarized");
        assert!(theme_custom::valid_theme_name("MyTheme"));
        assert!(!theme_custom::valid_theme_name("dark")); // built-in
        assert!(!theme_custom::valid_theme_name("zap"));
        assert_eq!(resolve_theme_id("gray"), "zap");
        assert_eq!(resolve_theme_id("opencode"), "zap");
    }

    #[test]
    fn zap_default_palette_exists() {
        set_mode_name("zap");
        assert!(!is_mono());
        assert_eq!(current_id(), "zap");
        assert_ne!(bg(), surface());
        assert_eq!(accent(), rgb(0x60, 0xa5, 0xfa));
    }
}
