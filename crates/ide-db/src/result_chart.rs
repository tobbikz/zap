//! ASCII / Unicode charts from numeric query columns.

use crate::QueryResult;

pub const CHART_KINDS: &[&str] = &["bar", "line", "pie"];

pub fn chart_result(result: &QueryResult, col_idx: Option<usize>) -> Option<String> {
    chart_result_n(result, col_idx, 12)
}

pub fn chart_result_n(result: &QueryResult, col_idx: Option<usize>, limit: usize) -> Option<String> {
    if result.rows.is_empty() || result.columns.is_empty() {
        return None;
    }
    let ci = col_idx.unwrap_or_else(|| first_numeric_column(result));
    let col_name = result.columns.get(ci)?;
    let values = parse_numeric_column(result, ci)?;
    if values.is_empty() {
        return None;
    }
    let limit = limit.clamp(4, 40);
    let min = values.iter().copied().fold(f64::INFINITY, f64::min);
    let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let span = (max - min).abs().max(f64::EPSILON);
    let sparkline: String = values
        .iter()
        .map(|v| {
            let t = ((v - min) / span).clamp(0.0, 1.0);
            crate::draw_glyphs::spark_at(t)
        })
        .collect();
    let bar_w = 24usize;
    let mut bars = String::new();
    for (i, v) in values.iter().take(limit).enumerate() {
        let t = ((v - min) / span).clamp(0.0, 1.0);
        let n = (t * bar_w as f64).round() as usize;
        let bar: String = std::iter::repeat(crate::draw_glyphs::bar()).take(n).collect();
        bars.push_str(&format!("#{i} {:<width$}  {v:.4}\n", bar, width = bar_w));
    }
    let more = if values.len() > limit {
        format!("… +{} more  [ ] rows\n", values.len() - limit)
    } else {
        String::new()
    };
    Some(format!(
        "{col_name}  n={}  {min:.4}…{max:.4}\n{sparkline}\n{bars}{more}",
        values.len(),
    ))
}

fn first_numeric_column(result: &QueryResult) -> usize {
    for ci in 0..result.columns.len() {
        if parse_numeric_column(result, ci)
            .map(|v| !v.is_empty())
            .unwrap_or(false)
        {
            return ci;
        }
    }
    0
}

fn parse_numeric_column(result: &QueryResult, ci: usize) -> Option<Vec<f64>> {
    let mut out = Vec::new();
    for row in &result.rows {
        let cell = row.get(ci)?;
        if cell == "NULL" {
            continue;
        }
        if let Ok(v) = cell.parse::<f64>() {
            out.push(v);
        }
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

/// Sparkline from latency history (ms).
pub fn latency_sparkline(ms: &[u128], width: usize) -> String {
    if ms.is_empty() {
        return crate::draw_glyphs::spark_floor()
            .to_string()
            .repeat(width.max(1));
    }
    let slice = if ms.len() > width {
        &ms[ms.len() - width..]
    } else {
        ms
    };
    let min = *slice.iter().min().unwrap_or(&0) as f64;
    let max = *slice.iter().max().unwrap_or(&1) as f64;
    let span = (max - min).abs().max(1.0);
    slice
        .iter()
        .map(|v| {
            let t = ((*v as f64 - min) / span).clamp(0.0, 1.0);
            crate::draw_glyphs::spark_at(t)
        })
        .collect()
}

/// Selection stats for numeric cells in the current column (or whole grid if col None).
pub fn selection_stats(result: &QueryResult, col: Option<usize>) -> String {
    let cols: Vec<usize> = match col {
        Some(c) if c < result.columns.len() => vec![c],
        _ => (0..result.columns.len()).collect(),
    };
    let mut nums = Vec::new();
    for ci in cols {
        if let Some(vs) = parse_numeric_column(result, ci) {
            nums.extend(vs);
        }
    }
    if nums.is_empty() {
        return format!("n={}", result.rows.len());
    }
    let sum: f64 = nums.iter().sum();
    let avg = sum / nums.len() as f64;
    let min = nums.iter().copied().fold(f64::INFINITY, f64::min);
    let max = nums.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    format!(
        "n={} sum={sum:.4} avg={avg:.4} min={min:.4} max={max:.4}",
        nums.len()
    )
}

pub fn chart_kind(result: &QueryResult, kind: &str, col_idx: Option<usize>) -> Option<String> {
    chart_kind_n(result, kind, col_idx, 12)
}

pub fn chart_kind_n(
    result: &QueryResult,
    kind: &str,
    col_idx: Option<usize>,
    limit: usize,
) -> Option<String> {
    let ci = col_idx.unwrap_or_else(|| first_numeric_column(result));
    let col_name = result.columns.get(ci)?;
    let values = parse_numeric_column(result, ci)?;
    if values.is_empty() {
        return None;
    }
    let limit = limit.clamp(4, 40);
    let min = values.iter().copied().fold(f64::INFINITY, f64::min);
    let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let span = (max - min).abs().max(f64::EPSILON);
    match kind {
        "line" => {
            let spark: String = values
                .iter()
                .map(|v| {
                    let t = ((v - min) / span).clamp(0.0, 1.0);
                    crate::draw_glyphs::spark_at(t)
                })
                .collect();
            Some(format!(
                "{col_name}  n={}  {min:.4}…{max:.4}\n{spark}\n",
                values.len()
            ))
        }
        "pie" => {
            // Abs-normalize so mixed signs cannot make sum≈0 and OOM on ● (#103).
            let abs: Vec<f64> = values.iter().map(|v| v.abs()).collect();
            let sum: f64 = abs.iter().sum();
            if sum < f64::EPSILON {
                return Some(format!(
                    "{col_name}  n={}\n (pie: |sum|≈0 — cannot chart)\n",
                    values.len()
                ));
            }
            let mut out = format!("{col_name}  n={}\n", values.len());
            for (i, v) in abs.iter().take(limit).enumerate() {
                let pct = (v / sum) * 100.0;
                let n = ((pct / 5.0).round() as usize).clamp(0, 20);
                let wedge: String = std::iter::repeat('●').take(n).collect();
                out.push_str(&format!("#{i} {pct:4.0}% {wedge}\n"));
            }
            if values.len() > limit {
                out.push_str(&format!("… +{} more  [ ] rows\n", values.len() - limit));
            }
            Some(out)
        }
        _ => chart_result_n(result, Some(ci), limit),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::QueryResult;

    #[test]
    fn pie_mixed_signs_near_zero_does_not_explode() {
        let qr = QueryResult {
            columns: vec!["n".into()],
            rows: vec![vec!["100".into()], vec!["-100".into()]],
            ..QueryResult::empty()
        };
        let out = chart_kind_n(&qr, "pie", Some(0), 12).expect("pie");
        assert!(
            out.contains("cannot chart") || out.contains('%'),
            "{out}"
        );
        // Wedge width is clamped — never a multi-megabyte string.
        assert!(out.len() < 4_000, "len={}", out.len());
    }
}
