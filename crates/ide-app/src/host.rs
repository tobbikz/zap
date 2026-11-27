//! Host terminal capabilities: Windows console code page, font, and icon tier.
//!
//! Windows PowerShell (conhost) defaults to an OEM code page and Consolas. Nerd Font
//! private-use glyphs and block carets (`▌ ▎ █`) are not in that font, so the console
//! falls back to a double-width replacement and the whole TUI shifts. Rounded corners
//! (`╭╮`) are missing too. We switch the console to UTF-8, avoid those glyphs unless
//! the face is a Nerd Font, and use square box-drawing on Consolas.

use ide_config::IconPref;
use ide_ui::theme::{self, FontClass, GlyphSet};

pub struct ConsoleGuard {
    #[cfg(windows)]
    inner: WinGuard,
}

pub fn install_console() -> ConsoleGuard {
    install_panic_hook();
    #[cfg(windows)]
    {
        ConsoleGuard {
            inner: WinGuard::install(),
        }
    }
    #[cfg(not(windows))]
    {
        ConsoleGuard {}
    }
}

/// Install a panic hook that restores the terminal before printing the panic.
/// On Windows this also leaves the alternate screen so the user gets a usable
/// shell back instead of garbled input / wrong code page (#75).
fn install_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        // Best-effort: leave raw mode so the panic message is readable.
        let _ = crossterm::terminal::disable_raw_mode();
        let _ = crossterm::execute!(
            std::io::stderr(),
            crossterm::terminal::LeaveAlternateScreen
        );
        default_hook(info);
    }));
}

pub fn apply_icons(pref: IconPref) {
    let face = console_face();
    let class = theme::classify_font(&face);
    let glyphs = resolve_glyphs(pref, class);
    // ConPTY under Windows Terminal often reports Consolas via GetCurrentConsoleFontEx
    // even when WT is rendering Cascadia / a Nerd Font — trust WT_SESSION for rich glyphs.
    let rich = matches!(class, FontClass::Nerd | FontClass::Modern)
        || windows_terminal()
        || force_rich_glyphs()
        || (matches!(class, FontClass::Unknown) && !cfg!(windows));
    theme::set_glyph_set(glyphs);
    theme::set_rounded_borders(rich);
    theme::set_block_glyphs(rich);
    ide_db::set_rich_blocks(rich);
    theme::set_truecolor(detect_truecolor());
}

/// Windows Terminal sets `WT_SESSION`. ConPTY font queries are unreliable there.
pub fn windows_terminal() -> bool {
    #[cfg(windows)]
    {
        std::env::var_os("WT_SESSION").is_some()
    }
    #[cfg(not(windows))]
    {
        false
    }
}

fn force_rich_glyphs() -> bool {
    match std::env::var("ZAP_RICH_GLYPHS") {
        Ok(v) => {
            let v = v.trim().to_ascii_lowercase();
            matches!(v.as_str(), "1" | "true" | "on" | "yes")
        }
        Err(_) => false,
    }
}

/// Detect whether the terminal supports 24-bit truecolor (#74).
/// Returns `true` on most modern terminals; `false` forces 16-color fallback.
pub fn detect_truecolor() -> bool {
    // COLORTERM=truecolor / 24bit is the standard signal.
    if let Ok(ct) = std::env::var("COLORTERM") {
        let ct = ct.to_ascii_lowercase();
        if ct.contains("truecolor") || ct.contains("24bit") {
            return true;
        }
    }
    // Windows Terminal sets WT_SESSION and supports 24-bit natively.
    if windows_terminal() {
        return true;
    }
    // ConPTY / conhost without WT — legacy; fall back to 16-color only
    // when we're actually on Windows with no other signal.
    #[cfg(windows)]
    {
        return false;
    }
    // Linux / macOS: virtually all modern terminals support truecolor.
    #[cfg(not(windows))]
    {
        true
    }
}

/// Kitty keyboard protocol. Conhost prints or ignores the enable sequence and then
/// mis-reads keys; Windows Terminal (`WT_SESSION`) understands it.
pub fn keyboard_protocol() -> bool {
    #[cfg(windows)]
    {
        windows_terminal()
    }
    #[cfg(not(windows))]
    {
        true
    }
}

fn resolve_glyphs(pref: IconPref, class: FontClass) -> GlyphSet {
    if let Ok(raw) = std::env::var("ZAP_ICONS") {
        if let Some(set) = parse_env(&raw) {
            return set;
        }
    }
    match pref {
        IconPref::Nerd => GlyphSet::Nerd,
        IconPref::Safe => GlyphSet::Safe,
        IconPref::Ascii => GlyphSet::Ascii,
        IconPref::Auto => {
            // WT + Cascadia Mono NF can draw Nerd PUA; ConPTY may still say "Consolas".
            if windows_terminal() || class == FontClass::Nerd {
                GlyphSet::Nerd
            } else if cfg!(windows) {
                GlyphSet::Safe
            } else {
                GlyphSet::Nerd
            }
        }
    }
}

fn parse_env(raw: &str) -> Option<GlyphSet> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "nerd" | "nf" | "on" | "true" | "1" => Some(GlyphSet::Nerd),
        "safe" | "windows" | "consolas" => Some(GlyphSet::Safe),
        "ascii" | "off" | "false" | "0" | "none" => Some(GlyphSet::Ascii),
        _ => None,
    }
}

fn console_face() -> String {
    #[cfg(windows)]
    {
        win_face()
    }
    #[cfg(not(windows))]
    {
        String::new()
    }
}

#[cfg(windows)]
struct WinGuard {
    out_cp: u32,
    in_cp: u32,
    font: Option<ConsoleFontInfoEx>,
}

#[cfg(windows)]
impl WinGuard {
    fn install() -> Self {
        unsafe {
            let handle = GetStdHandle(STD_OUTPUT_HANDLE);
            let out_cp = GetConsoleOutputCP();
            let in_cp = GetConsoleCP();
            if out_cp != 0 && out_cp != CP_UTF8 {
                SetConsoleOutputCP(CP_UTF8);
            }
            if in_cp != 0 && in_cp != CP_UTF8 {
                SetConsoleCP(CP_UTF8);
            }
            let mut previous = None;
            if is_console(handle) {
                let mut info = ConsoleFontInfoEx::empty();
                if GetCurrentConsoleFontEx(handle, 0, &mut info) != 0 {
                    let face = face_string(&info);
                    if poor_tui_font(&face) {
                        let mut next = info;
                        set_face(&mut next, "Consolas");
                        next.font_family = FF_MODERN;
                        next.font_weight = 400;
                        if next.font_size.y < 16 {
                            next.font_size = Coord { x: 0, y: 16 };
                        }
                        if SetCurrentConsoleFontEx(handle, 0, &next) != 0 {
                            previous = Some(info);
                        }
                    }
                }
            }
            Self {
                out_cp,
                in_cp,
                font: previous,
            }
        }
    }
}

#[cfg(windows)]
impl Drop for WinGuard {
    fn drop(&mut self) {
        unsafe {
            if self.out_cp != 0 && self.out_cp != CP_UTF8 {
                SetConsoleOutputCP(self.out_cp);
            }
            if self.in_cp != 0 && self.in_cp != CP_UTF8 {
                SetConsoleCP(self.in_cp);
            }
            if let Some(font) = self.font {
                let handle = GetStdHandle(STD_OUTPUT_HANDLE);
                if is_console(handle) {
                    SetCurrentConsoleFontEx(handle, 0, &font);
                }
            }
        }
    }
}

#[cfg(windows)]
fn win_face() -> String {
    unsafe {
        let handle = GetStdHandle(STD_OUTPUT_HANDLE);
        if !is_console(handle) {
            return String::new();
        }
        let mut info = ConsoleFontInfoEx::empty();
        if GetCurrentConsoleFontEx(handle, 0, &mut info) == 0 {
            return String::new();
        }
        face_string(&info)
    }
}

#[cfg(windows)]
fn is_console(handle: *mut core::ffi::c_void) -> bool {
    !handle.is_null() && handle != (-1isize as *mut core::ffi::c_void)
}

#[cfg(windows)]
fn poor_tui_font(face: &str) -> bool {
    let f = face.trim();
    f.is_empty()
        || f.eq_ignore_ascii_case("Terminal")
        || f.eq_ignore_ascii_case("Fixedsys")
        || f.eq_ignore_ascii_case("Lucida Console")
        || f.eq_ignore_ascii_case("Courier")
        || f.eq_ignore_ascii_case("Courier New")
        || f.eq_ignore_ascii_case("SimSun")
        || f.eq_ignore_ascii_case("NSimSun")
        || f.eq_ignore_ascii_case("MS Gothic")
}

#[cfg(windows)]
fn face_string(info: &ConsoleFontInfoEx) -> String {
    let len = info
        .face_name
        .iter()
        .position(|c| *c == 0)
        .unwrap_or(info.face_name.len());
    String::from_utf16_lossy(&info.face_name[..len])
}

#[cfg(windows)]
fn set_face(info: &mut ConsoleFontInfoEx, name: &str) {
    info.face_name = [0; 32];
    for (i, c) in name.encode_utf16().take(31).enumerate() {
        info.face_name[i] = c;
    }
}

#[cfg(windows)]
const CP_UTF8: u32 = 65001;
#[cfg(windows)]
const STD_OUTPUT_HANDLE: u32 = 0xFFFF_FFF5;
#[cfg(windows)]
const FF_MODERN: u32 = 0x30;

#[cfg(windows)]
#[repr(C)]
#[derive(Clone, Copy)]
struct Coord {
    x: i16,
    y: i16,
}

#[cfg(windows)]
#[repr(C)]
#[derive(Clone, Copy)]
struct ConsoleFontInfoEx {
    cb_size: u32,
    n_font: u32,
    font_size: Coord,
    font_family: u32,
    font_weight: u32,
    face_name: [u16; 32],
}

#[cfg(windows)]
impl ConsoleFontInfoEx {
    fn empty() -> Self {
        Self {
            cb_size: std::mem::size_of::<Self>() as u32,
            n_font: 0,
            font_size: Coord { x: 0, y: 0 },
            font_family: 0,
            font_weight: 0,
            face_name: [0; 32],
        }
    }
}

#[cfg(windows)]
#[link(name = "kernel32")]
extern "system" {
    fn GetStdHandle(nstd: u32) -> *mut core::ffi::c_void;
    fn GetConsoleOutputCP() -> u32;
    fn GetConsoleCP() -> u32;
    fn SetConsoleOutputCP(code_page: u32) -> i32;
    fn SetConsoleCP(code_page: u32) -> i32;
    fn GetCurrentConsoleFontEx(
        handle: *mut core::ffi::c_void,
        maximum_window: i32,
        info: *mut ConsoleFontInfoEx,
    ) -> i32;
    fn SetCurrentConsoleFontEx(
        handle: *mut core::ffi::c_void,
        maximum_window: i32,
        info: *const ConsoleFontInfoEx,
    ) -> i32;
}
