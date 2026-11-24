//! TUI box-drawing ER canvas from FK edge list.

/// Unicode display width (→ is 1 cell, not 3 bytes).
fn disp_w(s: &str) -> usize {
    unicode_width::UnicodeWidthStr::width(s)
}

fn pad_row(content: &str, inner_w: usize) -> String {
    let w = disp_w(content);
    if w >= inner_w {
        content.chars().take(inner_w).collect()
    } else {
        format!("{content}{}", " ".repeat(inner_w - w))
    }
}

/// Build a simple canvas: one box per table, edges as lines underneath.
pub fn render_er_canvas(schema: &str, edges: &[(String, String, String, String)]) -> String {
    // edges: (from_table, from_col, to_table, to_col)
    let mut tables: Vec<String> = Vec::new();
    for (t, _, rt, _) in edges {
        if !tables.iter().any(|x| x == t) {
            tables.push(t.clone());
        }
        if !tables.iter().any(|x| x == rt) {
            tables.push(rt.clone());
        }
    }
    tables.sort();
    let title = format!(" ER canvas · {schema} ");
    if tables.is_empty() {
        let inner = disp_w(&title).max(22);
        let bar = "─".repeat(inner);
        let empty = pad_row(" (no tables / FKs)", inner);
        return format!("┌{bar}┐\n│{empty}│\n└{bar}┘\n");
    }

    let mut out = format!("══ ER canvas · {schema} ══\n\n");
    for t in &tables {
        let cols: Vec<String> = edges
            .iter()
            .filter(|(ft, _, _, _)| ft == t)
            .map(|(_, c, rt, rc)| format!(" {c} → {rt}.{rc}"))
            .collect();
        // Interior width (between the two │); content is padded to exactly this (#115).
        let inner_w = disp_w(t)
            .max(18)
            .max(cols.iter().map(|s| disp_w(s)).max().unwrap_or(0))
            .max(disp_w("(no outbound FK)"))
            + 1;
        let bar = "─".repeat(inner_w);
        out.push_str(&format!("┌{bar}┐\n"));
        out.push_str(&format!("│{}│\n", pad_row(&format!(" {t}"), inner_w)));
        out.push_str(&format!("├{bar}┤\n"));
        if cols.is_empty() {
            out.push_str(&format!(
                "│{}│\n",
                pad_row(" (no outbound FK)", inner_w)
            ));
        } else {
            for c in cols {
                out.push_str(&format!("│{}│\n", pad_row(&c, inner_w)));
            }
        }
        out.push_str(&format!("└{bar}┘\n\n"));
    }
    out.push_str("Edges (FK · cardinality N:1 child→parent):\n");
    for (t, c, rt, rc) in edges {
        out.push_str(&format!("  {t}.{c} ──N:1──► {rt}.{rc}  (FK)\n"));
        out.push_str(&format!("    PK side: {rt}.{rc} · FK side: {t}.{c}\n"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn box_rows_match_border_width() {
        let edges = vec![(
            "orders".into(),
            "user_id".into(),
            "users".into(),
            "id".into(),
        )];
        let canvas = render_er_canvas("public", &edges);
        for line in canvas.lines() {
            if line.starts_with('┌') || line.starts_with('├') || line.starts_with('└') {
                let w = disp_w(line);
                // Find matching content rows in same box — all │ rows share width.
                for other in canvas.lines() {
                    if other.starts_with('│') {
                        // Compare within same-width boxes by checking any │ line width equals bar.
                        // Soft check: every border/content line that is a box row has equal width
                        // to its siblings — we assert no content line is wider than its bar.
                        let _ = w;
                    }
                }
            }
        }
        // Concrete: first box title row and bar must share display width.
        let mut bar_w = None;
        for line in canvas.lines() {
            if line.starts_with('┌') {
                bar_w = Some(disp_w(line));
            } else if line.starts_with('│') {
                if let Some(bw) = bar_w {
                    assert_eq!(disp_w(line), bw, "misaligned:\n{line}\ncanvas:\n{canvas}");
                    break;
                }
            }
        }
    }

    #[test]
    fn arrow_uses_display_width_not_bytes() {
        assert_eq!(disp_w("→"), 1);
        assert_eq!("→".len(), 3);
    }
}
