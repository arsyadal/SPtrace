pub mod analyzer;
pub mod cli;
pub mod model;
pub mod normalizer;
pub mod parser;
pub mod report;
pub mod rules;

use anyhow::{bail, Context, Result};
use cli::{Cli, Commands};
use model::{Operation, ScanIndexEntry};
use std::{
    fs,
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

pub fn execute(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Scan {
            path,
            out,
            json,
            diagram,
        } => execute_scan(&path, out.as_deref(), json, diagram.as_deref()),
        Commands::Context { path, out, json } => execute_context(&path, out.as_deref(), json),
        Commands::Diff {
            before,
            after,
            out,
            json,
        } => execute_diff(&before, &after, out.as_deref(), json),
    }
}

pub fn execute_scan(
    path: &Path,
    out: Option<&Path>,
    json: bool,
    diagram: Option<&str>,
) -> Result<()> {
    if let Some(format) = diagram {
        if !format.eq_ignore_ascii_case("mermaid") {
            bail!(
                "Unsupported diagram format: {}. Only mermaid is supported in v0.1.",
                format
            );
        }
    }

    if path.is_dir() {
        return execute_directory_scan(path, out, json, diagram);
    }
    execute_file_scan(path, out, json, diagram)
}

pub fn execute_context(path: &Path, out: Option<&Path>, json: bool) -> Result<()> {
    let sql = read_sql(path)?;
    let trace = analyzer::analyze_sql(&sql)?;
    let output = if json {
        report::render_context_json(&trace)
    } else {
        report::render_context(&trace)
    };
    emit_text(&output, out)
}

pub fn execute_diff(before: &Path, after: &Path, out: Option<&Path>, json: bool) -> Result<()> {
    let before_sql = read_sql(before)?;
    let after_sql = read_sql(after)?;
    let before_trace = analyzer::analyze_sql(&before_sql)?;
    let after_trace = analyzer::analyze_sql(&after_sql)?;
    let output = if json {
        report::render_diff_json(
            &before_trace,
            &after_trace,
            &before.display().to_string(),
            &after.display().to_string(),
        )
    } else {
        report::render_diff(
            &before_trace,
            &after_trace,
            &before.display().to_string(),
            &after.display().to_string(),
        )
    };
    emit_text(&output, out)
}

fn execute_file_scan(
    path: &Path,
    out: Option<&Path>,
    json: bool,
    diagram: Option<&str>,
) -> Result<()> {
    let sql = read_sql(path)?;
    let trace = analyzer::analyze_sql(&sql)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&trace)?);
    } else {
        println!("{}", report::render_terminal(&trace));
    }

    if let Some(out_path) = out {
        let markdown = report::render_markdown(&trace);
        write_text(out_path, &markdown)?;
        if diagram.is_some() {
            let diagram_path = out_path.with_extension("mmd");
            write_text(&diagram_path, &report::render_mermaid(&trace))?;
        }
    } else if diagram.is_some() {
        let diagram_path = default_diagram_path(path, trace.name.as_deref());
        write_text(&diagram_path, &report::render_mermaid(&trace))?;
    }

    Ok(())
}

fn execute_directory_scan(
    dir: &Path,
    out: Option<&Path>,
    json: bool,
    diagram: Option<&str>,
) -> Result<()> {
    if json {
        bail!("JSON output is only supported for single-file scan in v0.1.");
    }

    let out_dir = out
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("sptrace-output"));
    fs::create_dir_all(&out_dir)
        .with_context(|| format!("Failed to create output directory: {}", out_dir.display()))?;

    let mut entries: Vec<ScanIndexEntry> = Vec::new();
    let mut all_traces: Vec<crate::model::ProcedureTrace> = Vec::new();
    let mut scanned = 0usize;

    for entry in WalkDir::new(dir).into_iter().filter_map(Result::ok) {
        let path = entry.path();
        if !entry.file_type().is_file() {
            continue;
        }
        if path.starts_with(&out_dir) {
            continue;
        }
        if path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("sql"))
            .unwrap_or(false)
        {
            let sql = read_sql(path)?;
            let trace = analyzer::analyze_sql(&sql)?;
            let procedure = trace.name.clone().unwrap_or_else(|| fallback_name(path));
            let report_file_name = format!("{}.md", sanitize_filename(&procedure));
            let report_path = out_dir.join(&report_file_name);
            let markdown = report::render_markdown(&trace);
            fs::write(&report_path, markdown)
                .with_context(|| format!("Failed to write report: {}", report_path.display()))?;

            if diagram.is_some() {
                let diagram_path = report_path.with_extension("mmd");
                fs::write(&diagram_path, report::render_mermaid(&trace)).with_context(|| {
                    format!("Failed to write diagram: {}", diagram_path.display())
                })?;
            }

            entries.push(ScanIndexEntry {
                source_file: path.display().to_string(),
                report_file: report_path.display().to_string(),
                procedure,
                read_count: trace
                    .dependencies
                    .iter()
                    .filter(|dep| dep.operation == Operation::Read)
                    .count(),
                write_count: trace
                    .dependencies
                    .iter()
                    .filter(|dep| dep.operation == Operation::Write)
                    .count(),
                risk_level: trace.metrics.risk_level.clone(),
                risk_rules: trace
                    .risks
                    .iter()
                    .map(|risk| risk.rule_id.clone())
                    .collect(),
            });
            scanned += 1;
            all_traces.push(trace);
        }
    }

    entries.sort_by(|a, b| {
        a.procedure
            .cmp(&b.procedure)
            .then_with(|| a.source_file.cmp(&b.source_file))
    });
    let index = report::render_dependency_index(&entries);
    let index_path = out_dir.join("dependency-index.md");
    fs::write(&index_path, index)
        .with_context(|| format!("Failed to write dependency index: {}", index_path.display()))?;

    if diagram.is_some() {
        let master_diagram = report::render_folder_mermaid(&all_traces);
        let master_diagram_path = out_dir.join("folder-dependency.mmd");
        fs::write(&master_diagram_path, master_diagram).with_context(|| {
            format!("Failed to write folder diagram: {}", master_diagram_path.display())
        })?;
    }

    println!("Scanned {} SQL file(s)", scanned);
    println!("Generated dependency index: {}", index_path.display());
    for entry in entries {
        println!("- {} -> {}", entry.source_file, entry.report_file);
    }

    Ok(())
}

fn read_sql(path: &Path) -> Result<String> {
    if path.is_dir() {
        bail!("Expected a .sql file, got directory: {}", path.display());
    }
    fs::read_to_string(path).with_context(|| format!("Failed to read SQL file: {}", path.display()))
}

fn emit_text(text: &str, out: Option<&Path>) -> Result<()> {
    if let Some(out_path) = out {
        write_text(out_path, text)?;
    } else {
        println!("{}", text);
    }
    Ok(())
}

fn write_text(path: &Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).with_context(|| {
                format!("Failed to create output directory: {}", parent.display())
            })?;
        }
    }
    fs::write(path, content).with_context(|| format!("Failed to write file: {}", path.display()))
}

fn default_diagram_path(source: &Path, procedure: Option<&str>) -> PathBuf {
    let stem = procedure
        .map(sanitize_filename)
        .or_else(|| {
            source
                .file_stem()
                .and_then(|s| s.to_str())
                .map(sanitize_filename)
        })
        .unwrap_or_else(|| "diagram".to_string());
    PathBuf::from("sptrace-output").join(format!("{}.mmd", stem))
}

fn fallback_name(path: &Path) -> String {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("unknown_procedure")
        .to_string()
}

fn sanitize_filename(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' || ch == '.' {
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push('_');
        }
    }
    while out.contains("__") {
        out = out.replace("__", "_");
    }
    out.trim_matches('_').to_string()
}
