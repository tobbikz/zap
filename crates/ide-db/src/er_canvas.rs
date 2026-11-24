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
///
/// `tables` lists schema tables (so entities without FKs still appear — #106).
/// Extra tables referenced only by edges are merged in.
pub fn render_er_canvas(
    schema: &str,
    tables: &[String],
    edges: &[(String, String, String, String)],
) -> String {
    let mut all: Vec<String> = tables.to_vec();
    for (t, _, rt, _) in edges {
        if !all.iter().any(|x| x == t) {
            all.push(t.clone());
        }
        if !all.iter().any(|x| x == rt) {
            all.push(rt.clone());
        }
    }
    all.sort();
    all.dedup();
    let title = format!(" ER canvas · {schema} ");
    if all.is_empty() {
        let inner = disp_w(&title).max(22);
        let bar = "─".repeat(inner);
        let empty = pad_row(" (no tables)", inner);
        return format!("┌{bar}┐\n│{empty}│\n└{bar}┘\n");
    }

    let mut out = format!("══ ER canvas · {schema} ══\n\n");
    if edges.is_empty() {
        out.push_str("(no foreign keys — showing tables)\n\n");
    }
    for t in &all {
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
    if !edges.is_empty() {
        out.push_str("Edges (FK · cardinality N:1 child→parent):\n");
        for (t, c, rt, rc) in edges {
            out.push_str(&format!("  {t}.{c} ──N:1──► {rt}.{rc}  (FK)\n"));
            out.push_str(&format!("    PK side: {rt}.{rc} · FK side: {t}.{c}\n"));
        }
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
        let tables = vec!["orders".into(), "users".into()];
        let canvas = render_er_canvas("public", &tables, &edges);
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
    fn tables_without_fks_still_draw_boxes() {
        let tables = vec!["lonely".into(), "orphan".into()];
        let canvas = render_er_canvas("public", &tables, &[]);
        assert!(canvas.contains("lonely"), "{canvas}");
        assert!(canvas.contains("orphan"), "{canvas}");
        assert!(canvas.contains("no foreign keys"), "{canvas}");
        assert!(!canvas.contains("(no tables)"), "{canvas}");
    }

    #[test]
    fn arrow_uses_display_width_not_bytes() {
        assert_eq!(disp_w("→"), 1);
        assert_eq!("→".len(), 3);
    }
}
