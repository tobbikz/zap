//! Shared chrome helpers (modal scrim, fills).

use crate::theme;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

/// Paint jackin L0 — full-bleed app background before chrome.
pub fn paint_l0(frame: &mut Frame, area: Rect) {
    frame.render_widget(
        Block::default().style(Style::default().bg(theme::bg())),
        area,
    );
}

/// Frosted-glass scrim: dim dither wash so Studio chrome still peeks (#286).
/// Avoids a hard black Clear wipe that erases the stage.
pub fn paint_scrim(frame: &mut Frame, screen: Rect) {
    if screen.width == 0 || screen.height == 0 {
        return;
    }
    let mut lines: Vec<Line> = Vec::with_capacity(screen.height as usize);
    for y in 0..screen.height {
        let mut spans: Vec<Span> = Vec::with_capacity(screen.width as usize);
        for x in 0..screen.width {
            // Checkerboard dim: ░ on scrim_dither + space on scrim — reads as blur, not a plate.
            let dither = (x + y) % 2 == 0;
            if dither {
                spans.push(Span::styled(
                    "░",
                    Style::default()
                        .fg(theme::scrim_dither())
                        .bg(theme::scrim()),
                ));
            } else {
                spans.push(Span::styled(
                    " ",
                    Style::default().fg(theme::scrim()).bg(theme::scrim()),
                ));
            }
        }
        lines.push(Line::from(spans));
    }
    frame.render_widget(Paragraph::new(lines), screen);
}

/// Opaque modal stack: scrim + plate fill + focused border (caller may re-title).
pub fn modal_chrome(frame: &mut Frame, screen: Rect, area: Rect) {
    paint_scrim(frame, screen);
    fill_rect(
        frame,
        area,
        Style::default().bg(theme::modal()).fg(theme::modal()),
    );
    frame.render_widget(
        Block::default()
            .borders(Borders::ALL)
            .border_type(theme::border_type())
            .border_style(Style::default().fg(theme::border_focus()))
            .style(Style::default().bg(theme::modal())),
        area,
    );
}

fn fill_rect(frame: &mut Frame, area: Rect, style: Style) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let row = " ".repeat(area.width as usize);
    let lines: Vec<Line> = (0..area.height).map(|_| Line::from(row.as_str())).collect();
    frame.render_widget(Paragraph::new(lines).style(style), area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    #[test]
    fn modal_chrome_covers_screen() {
        let backend = TestBackend::new(40, 12);
        let mut term = Terminal::new(backend).unwrap();
        term.draw(|f| {
            let screen = f.area();
            let area = Rect {
                x: 5,
                y: 2,
                width: 30,
                height: 8,
            };
            modal_chrome(f, screen, area);
            paint_l0(f, screen);
        })
        .unwrap();
    }
}
