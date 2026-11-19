//! Chart glyphs. Block elements (`█ ▁ ▌`) are missing from Consolas, so Windows
//! PowerShell uses WGL4 stand-ins that stay one cell wide.

use std::sync::atomic::{AtomicBool, Ordering};

static RICH: AtomicBool = AtomicBool::new(true);

pub fn set_rich_blocks(on: bool) {
    RICH.store(on, Ordering::Relaxed);
}

pub fn rich_blocks() -> bool {
    RICH.load(Ordering::Relaxed)
}

pub fn bar() -> char {
    if rich_blocks() { '█' } else { '■' }
}

pub fn shade() -> char {
    if rich_blocks() { '▓' } else { '■' }
}

pub fn spark() -> [char; 8] {
    if rich_blocks() {
        ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█']
    } else {
        ['.', ':', '-', '=', '+', '*', '#', '#']
    }
}

pub fn spark_at(t: f64) -> char {
    let s = spark();
    let idx = (t.clamp(0.0, 1.0) * (s.len() - 1) as f64).round() as usize;
    s[idx.min(s.len() - 1)]
}

pub fn spark_floor() -> char {
    spark()[0]
}
