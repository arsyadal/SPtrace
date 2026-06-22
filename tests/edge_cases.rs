use sptrace::{analyzer, model::Severity};

#[test]
fn analyze_edge_case_fixture() {
    let sql = std::fs::read_to_string("tests/fixtures/edge_cases.sql").unwrap();
    let trace = analyzer::analyze_sql(&sql).unwrap();

    assert_eq!(trace.name.as_deref(), Some("dbo.SP_EDGE_CASES"));
    assert_eq!(trace.parameters.len(), 3);
    assert_eq!(trace.parameters[1].name, "@FLAG");
    assert_eq!(trace.parameters[1].default_value.as_deref(), Some("1"));
    assert_eq!(
        trace.parameters[2].default_value.as_deref(),
        Some("'A, B, C'")
    );
    assert!(trace
        .risks
        .iter()
        .any(|risk| risk.rule_id == "update_without_where"));
    assert!(!trace
        .risks
        .iter()
        .any(|risk| risk.rule_id == "delete_without_where"));
    assert_eq!(trace.metrics.risk_level, Severity::High);
}
