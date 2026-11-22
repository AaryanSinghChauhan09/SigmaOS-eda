//! SigmaOS EDA component — exploratory data analysis primitives.
//!
//! CSV parsing, column statistics, correlation, and missing-value counts.
//! Pure Rust, no unsafe code, deterministic output.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::fmt;

/// Errors produced during analysis.
#[derive(Debug)]
pub enum EdaError {
    /// Input could not be parsed as CSV text.
    Parse(String),
    /// A requested column does not exist.
    MissingColumn(String),
}

impl fmt::Display for EdaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse(m) => write!(f, "parse error: {m}"),
            Self::MissingColumn(c) => write!(f, "missing column: {c}"),
        }
    }
}

impl std::error::Error for EdaError {}

/// A parsed CSV table.
#[derive(Debug, Clone)]
pub struct Table {
    /// Column names in order.
    pub columns: Vec<String>,
    /// Rows; each row has the same length as `columns`.
    pub rows: Vec<Vec<String>>,
}

/// Summary statistics for a numeric column.
#[derive(Debug, Clone, PartialEq)]
pub struct ColumnStats {
    /// Column name.
    pub name: String,
    /// Count of non-empty, parseable numeric cells.
    pub numeric_count: usize,
    /// Count of empty cells.
    pub missing: usize,
    /// Minimum numeric value.
    pub min: Option<f64>,
    /// Maximum numeric value.
    pub max: Option<f64>,
    /// Arithmetic mean.
    pub mean: Option<f64>,
}

/// Parse CSV text (RFC 4180-ish: quoted fields, escaped quotes, CRLF/LF).
pub fn parse_csv(input: &str) -> Result<Table, EdaError> {
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut field = String::new();
    let mut row: Vec<String> = Vec::new();
    let mut chars = input.chars().peekable();
    let mut in_quotes = false;

    while let Some(c) = chars.next() {
        match c {
            '"' if in_quotes && chars.peek() == Some(&'"') => {
                chars.next();
                field.push('"');
            }
            '"' => in_quotes = !in_quotes,
            ',' if !in_quotes => {
                row.push(std::mem::take(&mut field));
            }
            '\n' | '\r' if !in_quotes => {
                if c == '\r' && chars.peek() == Some(&'\n') {
                    chars.next();
                }
                row.push(std::mem::take(&mut field));
                if row.iter().any(|f| !f.is_empty()) || rows.is_empty() {
                    rows.push(std::mem::take(&mut row));
                } else {
                    row.clear();
                }
            }
            _ => field.push(c),
        }
    }
    if !field.is_empty() || !row.is_empty() {
        row.push(field);
        rows.push(row);
    }
    if rows.is_empty() {
        return Err(EdaError::Parse("empty input".into()));
    }
    let columns = rows.remove(0);
    if columns.iter().any(|c| c.trim().is_empty()) {
        return Err(EdaError::Parse("empty column name".into()));
    }
    Ok(Table { columns, rows })
}

/// Compute statistics for one column by index.
pub fn column_stats(table: &Table, index: usize) -> Result<ColumnStats, EdaError> {
    let name = table
        .columns
        .get(index)
        .cloned()
        .ok_or_else(|| EdaError::MissingColumn(format!("#{index}")))?;
    let mut count = 0usize;
    let mut missing = 0usize;
    let mut min = f64::INFINITY;
    let mut max = f64::NEG_INFINITY;
    let mut sum = 0.0f64;
    for row in &table.rows {
        let cell = row.get(index).map(String::as_str).unwrap_or("");
        if cell.trim().is_empty() {
            missing += 1;
            continue;
        }
        match cell.trim().parse::<f64>() {
            Ok(v) => {
                count += 1;
                sum += v;
                if v < min { min = v; }
                if v > max { max = v; }
            }
            Err(_) => missing += 1,
        }
    }
    Ok(ColumnStats {
        name,
        numeric_count: count,
        missing,
        min: (count > 0).then_some(min),
        max: (count > 0).then_some(max),
        mean: (count > 0).then_some(sum / count as f64),
    })
}

/// Statistics for every column.
pub fn all_stats(table: &Table) -> Vec<ColumnStats> {
    (0..table.columns.len())
        .filter_map(|i| column_stats(table, i).ok())
        .collect()
}

/// Pearson correlation between two numeric columns.
pub fn correlation(table: &Table, a: usize, b: usize) -> Result<f64, EdaError> {
    let mut xs = Vec::new();
    let mut ys = Vec::new();
    for row in &table.rows {
        let xa = row.get(a).and_then(|s| s.trim().parse::<f64>().ok());
        let yb = row.get(b).and_then(|s| s.trim().parse::<f64>().ok());
        if let (Some(x), Some(y)) = (xa, yb) {
            xs.push(x);
            ys.push(y);
        }
    }
    let n = xs.len();
    if n < 2 {
        return Err(EdaError::Parse("need >=2 paired numeric rows".into()));
    }
    let mx = xs.iter().sum::<f64>() / n as f64;
    let my = ys.iter().sum::<f64>() / n as f64;
    let mut sxy = 0.0;
    let mut sxx = 0.0;
    let mut syy = 0.0;
    for (x, y) in xs.iter().zip(ys.iter()) {
        let dx = x - mx;
        let dy = y - my;
        sxy += dx * dy;
        sxx += dx * dx;
        syy += dy * dy;
    }
    if sxx == 0.0 || syy == 0.0 {
        return Err(EdaError::Parse("zero variance".into()));
    }
    Ok(sxy / (sxx * syy).sqrt())
}

/// Frequency map of values in a column (top-N by count).
pub fn value_counts(table: &Table, index: usize, top: usize) -> Result<BTreeMap<String, usize>, EdaError> {
    let mut counts = BTreeMap::new();
    for row in &table.rows {
        if let Some(cell) = row.get(index) {
            *counts.entry(cell.clone()).or_insert(0) += 1;
        }
    }
    let mut v: Vec<_> = counts.into_iter().collect();
    v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    v.truncate(top);
    Ok(v.into_iter().collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_stats() {
        let t = parse_csv("name,score\nalice,10\nbob,20\ncarol,\n").unwrap();
        let s = column_stats(&t, 1).unwrap();
        assert_eq!(s.numeric_count, 2);
        assert_eq!(s.missing, 1);
        assert_eq!(s.min, Some(10.0));
        assert_eq!(s.max, Some(20.0));
        assert_eq!(s.mean, Some(15.0));
    }

    #[test]
    fn correlation_perfect() {
        let t = parse_csv("x,y\n1,2\n2,4\n3,6\n").unwrap();
        let r = correlation(&t, 0, 1).unwrap();
        assert!((r - 1.0).abs() < 1e-9);
    }

    #[test]
    fn quoted_fields() {
        let t = parse_csv("a,b\n\"x,y\",\"q\"\"z\"\n").unwrap();
        assert_eq!(t.rows[0][0], "x,y");
        assert_eq!(t.rows[0][1], "q\"z");
    }
}
