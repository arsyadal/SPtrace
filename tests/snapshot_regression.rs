use std::fs;

use sptrace::{analyzer, report};

fn fixture(name: &str) -> String {
    let path = format!(
        "{}/examples/procedures/{}",
        env!("CARGO_MANIFEST_DIR"),
        name
    );
    fs::read_to_string(path).expect("read fixture")
}

fn assert_snapshot(actual: String, expected: &str) {
    assert_eq!(actual.trim_end(), expected.trim_end());
}

#[test]
fn duplicate_aggregation_markdown_snapshot() {
    let sql = fixture("duplicate_aggregation.sql");
    let trace = analyzer::analyze_sql(&sql).expect("analyze sql");
    assert_snapshot(
        report::render_markdown(&trace),
        include_str!("snapshots/duplicate_aggregation.md.snap"),
    );
}

#[test]
fn duplicate_aggregation_json_snapshot() {
    let sql = fixture("duplicate_aggregation.sql");
    let trace = analyzer::analyze_sql(&sql).expect("analyze sql");
    let actual = serde_json::to_string_pretty(&trace).expect("json");
    assert_snapshot(
        actual,
        include_str!("snapshots/duplicate_aggregation.json.snap"),
    );
}
