use std::fs;
use std::path::Path;

use crate::record;
use crate::run::{capped, cargo, shown, status};
use crate::tasks::limit;

pub fn run(root: &Path, dir: &Path) -> Result<(), String> {
    // geiger reads the dependencies' sources, which a build without vendor/ has fetched
    // only for the host's targets.
    status(cargo(root, &["fetch", "--locked"]))?;
    let path = dir.join("geiger.txt");
    let file = fs::File::create(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    // geiger cleans before it builds; in target/debug that would take everyone's build.
    let mut scan = cargo(root, &["geiger", "--locked", "--offline", "--output-format", "Ascii"]);
    scan.env("CARGO_TARGET_DIR", root.join("target/geiger"));
    let mut cmd = capped(scan, &limit("geiger"));
    record::say(&format!("$ {} > geiger.txt", shown(&cmd)));
    let ended = record::exec(&mut cmd, Some(file)).map_err(|e| format!("cargo geiger: {e}"))?;
    let table = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    // geiger exits non-zero when files it was shown are not Rust (READMEs, included
    // text), so the table itself tells whether it ran; a signal says it did not finish.
    if ended.code().is_none() {
        return Err(format!("cargo geiger did not finish: {ended}"));
    }
    if !complete(&table) {
        return Err(format!("cargo geiger wrote no whole table: {ended}"));
    }
    if !ended.success() {
        record::say(&format!("cargo geiger: {ended}, with a whole table"));
    }
    Ok(())
}

/// Whether `table` has its legend and ends in the row of totals.
fn complete(table: &str) -> bool {
    let last = table.lines().rfind(|line| !line.trim().is_empty());
    table.contains("Metric output format")
        && last.is_some_and(|line| line.starts_with(|c: char| c.is_ascii_digit()))
}

#[cfg(test)]
mod tests {
    use super::complete;

    #[test]
    fn a_table_is_whole_with_its_legend_and_totals() {
        assert!(complete("Metric output format: x/y\n\n0/0  0/0 :) a 1.0\n\n1/2  3/4\n\n"));
        assert!(!complete("Metric output format: x/y\n\nFunctions  Expressions\n"));
        assert!(!complete("1/2  3/4\n"));
        assert!(!complete(""));
    }
}
