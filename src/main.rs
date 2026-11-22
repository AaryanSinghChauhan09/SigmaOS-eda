//! sigmaos-eda CLI — summarize a CSV file.

use std::env;
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = env::args().nth(1).ok_or("usage: sigmaos-eda <file.csv>")?;
    let raw = fs::read_to_string(path)?;
    let table = sigmaos_eda::parse_csv(&raw)?;
    println!("{} rows x {} cols", table.rows.len(), table.columns.len());
    for s in sigmaos_eda::all_stats(&table) {
        println!(
            "{}: n={} missing={} min={:?} max={:?} mean={:?}",
            s.name, s.numeric_count, s.missing, s.min, s.max, s.mean
        );
    }
    Ok(())
}
