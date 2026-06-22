use serde_json::Value;
use std::{
    env, fs,
    path::PathBuf,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

fn run_scan_json(path: &str) -> Value {
    let output = Command::new(env!("CARGO_BIN_EXE_sptrace"))
        .args(["scan", path, "--json"])
        .output()
        .expect("failed to run sptrace");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("valid json")
}

fn temp_sql_path(name: &str) -> PathBuf {
    let mut path = env::temp_dir();
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    path.push(format!("sptrace-{name}-{unique}.sql"));
    path
}

#[test]
fn duplicate_aggregation_fixture_has_expected_high_risk() {
    let json = run_scan_json("examples/procedures/duplicate_aggregation.sql");

    assert_eq!(json["name"], "SP_GENERATE_GR_SAMPLE");
    assert_eq!(json["metrics"]["risk_level"], "High");
    assert!(json["risks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|risk| risk["rule_id"] == "multi_join_aggregation"));
    assert!(json["dependencies"]
        .as_array()
        .unwrap()
        .iter()
        .any(|dep| dep["object"] == "TB_T_GR_HUB" && dep["operation"] == "Write"));
}

#[test]
fn linked_server_fixture_detects_linked_server_risk() {
    let json = run_scan_json("examples/procedures/linked_server.sql");

    assert!(json["risks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|risk| risk["rule_id"] == "linked_server"));
}

#[test]
fn dynamic_sql_fixture_detects_dynamic_sql_risk() {
    let json = run_scan_json("examples/procedures/dynamic_sql.sql");

    assert!(json["risks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|risk| risk["rule_id"] == "dynamic_sql"));
}

#[test]
fn update_without_where_fixture_detects_high_risk() {
    let json = run_scan_json("examples/procedures/update_without_where.sql");

    assert!(json["risks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|risk| risk["rule_id"] == "update_without_where"));
    assert_eq!(json["metrics"]["risk_level"], "High");
}

#[test]
fn select_star_nolock_fixture_detects_both_rules() {
    let json = run_scan_json("examples/procedures/select_star_nolock.sql");
    let rule_ids: Vec<_> = json["risks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|risk| risk["rule_id"].as_str().unwrap().to_string())
        .collect();

    assert!(rule_ids.contains(&"select_star".to_string()));
    assert!(rule_ids.contains(&"nolock_used".to_string()));
}

#[test]
fn delete_join_does_not_count_target_as_read() {
    let path = temp_sql_path("delete-join");
    fs::write(
        &path,
        r#"
CREATE PROCEDURE dbo.SP_DELETE_JOIN
AS
BEGIN
    DELETE t
    FROM TB_TARGET t
    JOIN TB_SRC s ON s.ID = t.ID
    WHERE s.FLAG = 1
END
"#,
    )
    .expect("write temp sql");

    let json = run_scan_json(path.to_str().unwrap());
    let deps = json["dependencies"].as_array().unwrap();

    assert!(deps
        .iter()
        .any(|dep| dep["object"] == "TB_TARGET" && dep["operation"] == "Write"));
    assert!(deps
        .iter()
        .any(|dep| dep["object"] == "TB_SRC" && dep["operation"] == "Read"));
    assert!(!deps
        .iter()
        .any(|dep| dep["object"] == "TB_TARGET" && dep["operation"] == "Read"));

    let _ = fs::remove_file(path);
}
