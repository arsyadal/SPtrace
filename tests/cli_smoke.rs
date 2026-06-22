use std::{
    env, fs,
    path::PathBuf,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_sptrace"))
}

fn example(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join("procedures")
        .join(name)
}

fn unique_temp_dir(prefix: &str) -> PathBuf {
    let mut dir = env::temp_dir();
    let pid = std::process::id();
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    dir.push(format!("sptrace-{prefix}-{pid}-{ts}"));
    dir
}

#[test]
fn scan_json_smoke() {
    let output = Command::new(bin())
        .args([
            "scan",
            example("duplicate_aggregation.sql").to_str().unwrap(),
            "--json",
        ])
        .output()
        .expect("run sptrace");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("multi_join_aggregation"));
    assert!(stdout.contains("SP_GENERATE_GR_SAMPLE"));
}

#[test]
fn scan_out_writes_markdown() {
    let dir = unique_temp_dir("markdown");
    fs::create_dir_all(&dir).unwrap();
    let report = dir.join("report.md");

    let output = Command::new(bin())
        .args([
            "scan",
            example("duplicate_aggregation.sql").to_str().unwrap(),
            "--out",
            report.to_str().unwrap(),
        ])
        .output()
        .expect("run sptrace");

    assert!(output.status.success());
    let md = fs::read_to_string(&report).expect("report exists");
    assert!(md.contains("# SPTrace Report"));
    assert!(md.contains("## 5. Dependency Diagram"));
}

#[test]
fn scan_context_json_smoke() {
    let output = Command::new(bin())
        .args([
            "context",
            example("duplicate_aggregation.sql").to_str().unwrap(),
            "--json",
        ])
        .output()
        .expect("run sptrace");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\"summary\""));
    assert!(stdout.contains("\"suggested_queries\""));
}
