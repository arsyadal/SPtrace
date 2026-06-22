use crate::model::{Dependency, Operation, ProcedureTrace, ScanIndexEntry};
use serde_json::json;
use std::collections::HashSet;

pub fn render_terminal(trace: &ProcedureTrace) -> String {
    let name = trace.name.as_deref().unwrap_or("Unknown");
    let groups = group_dependencies(&trace.dependencies);
    let mut lines = Vec::new();

    lines.push(format!("Procedure: {name}"));
    lines.push(String::new());
    lines.push("Parameters:".to_string());
    if trace.parameters.is_empty() {
        lines.push("- None".to_string());
    } else {
        for param in &trace.parameters {
            let default = param.default_value.as_deref().unwrap_or("Required");
            lines.push(format!(
                "- {} {} ({})",
                param.name, param.data_type, default
            ));
        }
    }
    lines.push(String::new());
    lines.push("Tables Read:".to_string());
    push_list(&mut lines, &groups.reads);
    lines.push(String::new());
    lines.push("Tables Written:".to_string());
    push_list(&mut lines, &groups.writes);
    lines.push(String::new());
    lines.push("Procedures Executed:".to_string());
    push_list(&mut lines, &groups.executes);
    lines.push(String::new());
    lines.push("Temp Tables:".to_string());
    push_list(&mut lines, &trace.temp_tables);
    lines.push(String::new());
    lines.push("Risks:".to_string());
    if trace.risks.is_empty() {
        lines.push("- None".to_string());
    } else {
        for risk in &trace.risks {
            lines.push(format!(
                "[{}] {} - {}",
                risk.severity.to_string().to_uppercase(),
                risk.rule_id,
                risk.message
            ));
        }
    }

    lines.join("\n")
}

pub fn render_markdown(trace: &ProcedureTrace) -> String {
    let name = trace.name.as_deref().unwrap_or("Unknown");
    let groups = group_dependencies(&trace.dependencies);
    let diagram = render_mermaid(trace);
    let flow = render_flow(trace, &groups);
    let questions = render_questions(trace);
    let queries = render_next_queries(trace, &groups);

    let mut out = String::new();

    out.push_str(&format!("# SPTrace Report: {}\n\n", name));
    out.push_str("## 1. Overview\n");
    out.push_str(&format!("- Procedure: {}\n", name));
    out.push_str("- Dialect: T-SQL\n");
    out.push_str(&format!(
        "- Statements: {}\n",
        trace.metrics.statement_count
    ));
    out.push_str(&format!("- Risk Level: {}\n\n", trace.metrics.risk_level));

    out.push_str("## 2. Parameters\n");
    out.push_str("| Parameter | Type | Default |\n|---|---|---|\n");
    if trace.parameters.is_empty() {
        out.push_str("| None | - | - |\n");
    } else {
        for param in &trace.parameters {
            let default = param.default_value.as_deref().unwrap_or("Required");
            out.push_str(&format!(
                "| {} | {} | {} |\n",
                param.name, param.data_type, default
            ));
        }
    }
    out.push_str("\n");

    out.push_str("## 3. Dependencies\n\n");
    out.push_str("### Tables Read\n");
    push_markdown_list(&mut out, &groups.reads);
    out.push_str("\n### Tables Written\n");
    push_markdown_list(&mut out, &groups.writes);
    out.push_str("\n### Procedures Executed\n");
    push_markdown_list(&mut out, &groups.executes);
    out.push_str("\n### External Dependencies\n");
    push_markdown_list(&mut out, &groups.external);
    out.push_str("\n");

    out.push_str("## 4. Temp Tables\n");
    push_markdown_list(&mut out, &trace.temp_tables);
    out.push_str("\n");

    out.push_str("## 5. Dependency Diagram\n\n");
    out.push_str("```mermaid\n");
    out.push_str(&diagram);
    out.push_str("\n```\n\n");

    out.push_str("## 6. Execution Flow\n");
    out.push_str(&flow);
    out.push_str("\n\n");

    out.push_str("## 7. Risk Findings\n");
    if trace.risks.is_empty() {
        out.push_str("- None detected.\n");
    } else {
        for risk in &trace.risks {
            out.push_str(&format!("\n### {}: {}\n", risk.severity, risk.rule_id));
            out.push_str(&format!("{}\n\n", risk.message));
            out.push_str(&format!("Suggestion: {}\n", risk.suggestion));
        }
    }
    out.push_str("\n");

    out.push_str("## 8. Questions to Verify\n");
    push_markdown_list(&mut out, &questions);
    out.push_str("\n");

    out.push_str("## 9. Suggested Next Queries\n");
    if queries.is_empty() {
        out.push_str("- No specific query suggested.\n");
    } else {
        for query in queries {
            out.push_str("```sql\n");
            out.push_str(&query);
            out.push_str("\n```\n");
        }
    }

    out
}

pub fn render_context(trace: &ProcedureTrace) -> String {
    let name = trace.name.as_deref().unwrap_or("Unknown");
    let groups = group_dependencies(&trace.dependencies);
    let mut out = String::new();

    out.push_str("# AI Context for Stored Procedure Analysis\n\n");
    out.push_str("## Procedure\n");
    out.push_str(&format!("{}\n\n", name));
    out.push_str("## Summary\n");
    out.push_str(&format!(
        "This procedure reads {} {}, writes {} {}, uses {} temp {}, and has risk level {}.\n\n",
        groups.reads.len(),
        pluralize(groups.reads.len(), "table", "tables"),
        groups.writes.len(),
        pluralize(groups.writes.len(), "table", "tables"),
        trace.temp_tables.len(),
        pluralize(trace.temp_tables.len(), "table", "tables"),
        trace.metrics.risk_level
    ));

    out.push_str("## Extracted Dependencies\n");
    out.push_str("### Tables Read\n");
    push_markdown_list(&mut out, &groups.reads);
    out.push_str("\n### Tables Written\n");
    push_markdown_list(&mut out, &groups.writes);
    out.push_str("\n### Procedures Executed\n");
    push_markdown_list(&mut out, &groups.executes);
    out.push_str("\n### Temp Tables\n");
    push_markdown_list(&mut out, &trace.temp_tables);
    out.push_str("\n");

    out.push_str("## Suspicious Logic\n");
    if trace.risks.is_empty() {
        out.push_str("- None detected.\n");
    } else {
        for risk in &trace.risks {
            out.push_str(&format!(
                "- [{}] {}: {}\n",
                risk.severity, risk.rule_id, risk.message
            ));
        }
    }
    out.push_str("\n");

    out.push_str("## Ask\n");
    for question in render_questions(trace) {
        out.push_str(&format!("- {}\n", question));
    }
    out.push_str("\n");

    out.push_str("## Suggested Verification Queries\n");
    let queries = render_next_queries(trace, &groups);
    if queries.is_empty() {
        out.push_str("- No specific query suggested.\n");
    } else {
        for query in queries {
            out.push_str("```sql\n");
            out.push_str(&query);
            out.push_str("\n```\n");
        }
    }

    out
}

pub fn render_context_json(trace: &ProcedureTrace) -> String {
    let groups = group_dependencies(&trace.dependencies);
    let value = json!({
        "procedure": trace.name,
        "summary": {
            "read_count": groups.reads.len(),
            "write_count": groups.writes.len(),
            "temp_table_count": trace.temp_tables.len(),
            "risk_level": trace.metrics.risk_level,
            "statement_count": trace.metrics.statement_count,
        },
        "dependencies": {
            "reads": groups.reads,
            "writes": groups.writes,
            "executes": groups.executes,
            "external": groups.external,
        },
        "temp_tables": trace.temp_tables,
        "risks": trace.risks,
        "questions": render_questions(trace),
        "suggested_queries": render_next_queries(trace, &groups),
    });
    serde_json::to_string_pretty(&value).expect("context json")
}

pub fn render_dependency_index(entries: &[ScanIndexEntry]) -> String {
    let mut out = String::new();
    out.push_str("# SPTrace Dependency Index\n\n");
    out.push_str("| File | Procedure | Reads | Writes | Risk Level | Risk Rules |\n");
    out.push_str("|---|---|---:|---:|---|---|\n");

    if entries.is_empty() {
        out.push_str("| None | - | 0 | 0 | Low | - |\n");
        return out;
    }

    for entry in entries {
        let rules = if entry.risk_rules.is_empty() {
            "-".to_string()
        } else {
            entry.risk_rules.join(", ")
        };
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} |\n",
            entry.source_file,
            entry.procedure,
            entry.read_count,
            entry.write_count,
            entry.risk_level,
            rules
        ));
    }

    out
}

pub fn render_mermaid(trace: &ProcedureTrace) -> String {
    let proc_name = trace.name.as_deref().unwrap_or("UNKNOWN_PROC");
    let proc_id = node_id(proc_name);
    let proc_label = escape_label(proc_name);

    let groups = group_dependencies(&trace.dependencies);
    let mut lines = Vec::new();
    let mut emitted_nodes = HashSet::new();

    lines.push("flowchart LR".to_string());
    lines.push(format!("    {}[\"{}\"]", proc_id, proc_label));
    emitted_nodes.insert(proc_id.clone());

    for dep in &trace.dependencies {
        let dep_id = node_id(&dep.object);
        if emitted_nodes.insert(dep_id.clone()) {
            lines.push(format!("    {}[\"{}\"]", dep_id, escape_label(&dep.object)));
        }

        match dep.operation {
            Operation::Read => {
                lines.push(format!("    {} --> {}", dep_id, proc_id));
            }
            Operation::Write => {
                lines.push(format!("    {} --> {}", proc_id, dep_id));
            }
            Operation::Execute => {
                lines.push(format!("    {} -. executes .-> {}", proc_id, dep_id));
            }
            Operation::Unknown => {}
        }
    }

    if trace.dependencies.is_empty() {
        if groups.external.is_empty() {
            lines.push("    %% No dependencies detected".to_string());
        }
    }

    lines.join("\n")
}

struct DependencyGroups {
    reads: Vec<String>,
    writes: Vec<String>,
    executes: Vec<String>,
    external: Vec<String>,
}

fn group_dependencies(deps: &[Dependency]) -> DependencyGroups {
    DependencyGroups {
        reads: ordered_unique_objects(deps, |dep| dep.operation == Operation::Read),
        writes: ordered_unique_objects(deps, |dep| dep.operation == Operation::Write),
        executes: ordered_unique_objects(deps, |dep| dep.operation == Operation::Execute),
        external: ordered_unique_objects(deps, |dep| is_external_dependency(dep)),
    }
}

fn ordered_unique_objects<F>(deps: &[Dependency], predicate: F) -> Vec<String>
where
    F: Fn(&Dependency) -> bool,
{
    let mut seen = HashSet::new();
    let mut items = Vec::new();

    for dep in deps {
        if predicate(dep) && seen.insert(dep.object.clone()) {
            items.push(dep.object.clone());
        }
    }

    items
}

fn is_external_dependency(dep: &Dependency) -> bool {
    dep.object
        .split('.')
        .filter(|part| !part.is_empty())
        .count()
        >= 4
}

fn push_list(lines: &mut Vec<String>, items: &[String]) {
    if items.is_empty() {
        lines.push("- None".to_string());
    } else {
        for item in items {
            lines.push(format!("- {}", item));
        }
    }
}

fn push_markdown_list(out: &mut String, items: &[String]) {
    if items.is_empty() {
        out.push_str("- None\n");
    } else {
        for item in items {
            out.push_str(&format!("- {}\n", item));
        }
    }
}

fn render_flow(trace: &ProcedureTrace, groups: &DependencyGroups) -> String {
    let mut steps = Vec::new();

    if !groups.reads.is_empty() {
        steps.push(format!("1. Reads from `{}`", groups.reads.join("`, `")));
    }
    if !trace.temp_tables.is_empty() {
        let step_no = steps.len() + 1;
        steps.push(format!(
            "{}. Uses temp tables `{}`",
            step_no,
            trace.temp_tables.join("`, `")
        ));
    }
    if !groups.writes.is_empty() {
        let step_no = steps.len() + 1;
        steps.push(format!(
            "{}. Writes to `{}`",
            step_no,
            groups.writes.join("`, `")
        ));
    }
    if !groups.executes.is_empty() {
        let step_no = steps.len() + 1;
        steps.push(format!(
            "{}. Calls `{}`",
            step_no,
            groups.executes.join("`, `")
        ));
    }

    if steps.is_empty() {
        "1. No explicit flow detected.".to_string()
    } else {
        steps.join("\n")
    }
}

fn render_questions(trace: &ProcedureTrace) -> Vec<String> {
    let mut questions = Vec::new();

    if contains_risk(trace, "multi_join_aggregation") {
        questions.push("Is PART_NO unique per ORDER_NO?".to_string());
        questions.push("Should MANIFEST_NO be part of the join key?".to_string());
        questions.push("Should quantity be pre-aggregated before joining master data?".to_string());
    }
    if contains_risk(trace, "update_without_where") || contains_risk(trace, "delete_without_where")
    {
        questions.push("Is the full-table update/delete intentional?".to_string());
        questions.push("Should a WHERE clause be added to narrow the affected rows?".to_string());
    }
    if contains_risk(trace, "dynamic_sql") {
        questions.push("What runtime object names are injected into the dynamic SQL?".to_string());
        questions.push("Can the dynamic SQL be replaced with a static query?".to_string());
    }
    if contains_risk(trace, "linked_server") {
        questions
            .push("Is the linked server available and trusted in all environments?".to_string());
        questions.push("Does this external dependency affect freshness or latency?".to_string());
    }
    if contains_risk(trace, "transaction_without_trycatch")
        || contains_risk(trace, "trycatch_without_rollback")
    {
        questions.push("Is transaction rollback handled correctly on error?".to_string());
    }
    if contains_risk(trace, "select_star") {
        questions.push("Can the query list explicit columns instead of SELECT *?".to_string());
    }

    if questions.is_empty() {
        questions.push(
            "Does the current dependency graph match the intended business flow?".to_string(),
        );
    }

    questions.sort();
    questions.dedup();
    questions
}

pub fn render_diff(
    before: &ProcedureTrace,
    after: &ProcedureTrace,
    before_label: &str,
    after_label: &str,
) -> String {
    let before_groups = group_dependencies(&before.dependencies);
    let after_groups = group_dependencies(&after.dependencies);

    let mut out = String::new();
    out.push_str("# SPTrace Diff Report\n\n");
    out.push_str("## Overview\n");
    out.push_str(&format!("- Before: {}\n", before_label));
    out.push_str(&format!("- After: {}\n", after_label));
    out.push_str(&format!(
        "- Before Procedure: {}\n",
        before.name.as_deref().unwrap_or("Unknown")
    ));
    out.push_str(&format!(
        "- After Procedure: {}\n",
        after.name.as_deref().unwrap_or("Unknown")
    ));
    out.push_str(&format!(
        "- Before Risk Level: {}\n",
        before.metrics.risk_level
    ));
    out.push_str(&format!(
        "- After Risk Level: {}\n\n",
        after.metrics.risk_level
    ));

    out.push_str("## Parameter Changes\n");
    let before_params = parameter_set(before);
    let after_params = parameter_set(after);
    write_string_section(&mut out, "Added", &set_diff(&after_params, &before_params));
    write_string_section(
        &mut out,
        "Removed",
        &set_diff(&before_params, &after_params),
    );
    out.push_str("\n");

    out.push_str("## Dependency Changes\n");
    write_string_section(
        &mut out,
        "New Tables Read",
        &set_diff(&before_groups.reads, &after_groups.reads),
    );
    write_string_section(
        &mut out,
        "Removed Tables Read",
        &set_diff(&after_groups.reads, &before_groups.reads),
    );
    write_string_section(
        &mut out,
        "New Tables Written",
        &set_diff(&before_groups.writes, &after_groups.writes),
    );
    write_string_section(
        &mut out,
        "Removed Tables Written",
        &set_diff(&after_groups.writes, &before_groups.writes),
    );
    write_string_section(
        &mut out,
        "New Procedures Executed",
        &set_diff(&before_groups.executes, &after_groups.executes),
    );
    write_string_section(
        &mut out,
        "Removed Procedures Executed",
        &set_diff(&after_groups.executes, &before_groups.executes),
    );
    out.push_str("\n");

    out.push_str("## Temp Table Changes\n");
    write_string_section(
        &mut out,
        "New Temp Tables",
        &set_diff(&before.temp_tables, &after.temp_tables),
    );
    write_string_section(
        &mut out,
        "Removed Temp Tables",
        &set_diff(&after.temp_tables, &before.temp_tables),
    );
    out.push_str("\n");

    out.push_str("## Risk Changes\n");
    let before_risks = risk_rule_set(before);
    let after_risks = risk_rule_set(after);
    write_string_section(
        &mut out,
        "Added Risks",
        &set_diff(&after_risks, &before_risks),
    );
    write_string_section(
        &mut out,
        "Removed Risks",
        &set_diff(&before_risks, &after_risks),
    );
    out.push_str(&format!(
        "- Before Count: {}\n- After Count: {}\n\n",
        before.risks.len(),
        after.risks.len()
    ));

    out
}

pub fn render_diff_json(
    before: &ProcedureTrace,
    after: &ProcedureTrace,
    before_label: &str,
    after_label: &str,
) -> String {
    let before_groups = group_dependencies(&before.dependencies);
    let after_groups = group_dependencies(&after.dependencies);

    let value = json!({
        "overview": {
            "before_label": before_label,
            "after_label": after_label,
            "before_procedure": before.name,
            "after_procedure": after.name,
            "before_risk_level": before.metrics.risk_level,
            "after_risk_level": after.metrics.risk_level,
        },
        "parameter_changes": {
            "added": set_diff(&parameter_set(after), &parameter_set(before)),
            "removed": set_diff(&parameter_set(before), &parameter_set(after)),
        },
        "dependency_changes": {
            "new_tables_read": set_diff(&before_groups.reads, &after_groups.reads),
            "removed_tables_read": set_diff(&after_groups.reads, &before_groups.reads),
            "new_tables_written": set_diff(&before_groups.writes, &after_groups.writes),
            "removed_tables_written": set_diff(&after_groups.writes, &before_groups.writes),
            "new_procedures_executed": set_diff(&before_groups.executes, &after_groups.executes),
            "removed_procedures_executed": set_diff(&after_groups.executes, &before_groups.executes),
        },
        "temp_table_changes": {
            "added": set_diff(&before.temp_tables, &after.temp_tables),
            "removed": set_diff(&after.temp_tables, &before.temp_tables),
        },
        "risk_changes": {
            "added": set_diff(&risk_rule_set(after), &risk_rule_set(before)),
            "removed": set_diff(&risk_rule_set(before), &risk_rule_set(after)),
            "before_count": before.risks.len(),
            "after_count": after.risks.len(),
        },
    });

    serde_json::to_string_pretty(&value).expect("diff json")
}

fn parameter_set(trace: &ProcedureTrace) -> Vec<String> {
    let mut items: Vec<String> = trace
        .parameters
        .iter()
        .map(|param| match &param.default_value {
            Some(default) => format!("{} {} = {}", param.name, param.data_type, default),
            None => format!("{} {}", param.name, param.data_type),
        })
        .collect();
    items.sort();
    items.dedup();
    items
}

fn risk_rule_set(trace: &ProcedureTrace) -> Vec<String> {
    let mut items: Vec<String> = trace
        .risks
        .iter()
        .map(|risk| format!("{}: {}", risk.rule_id, risk.message))
        .collect();
    items.sort();
    items.dedup();
    items
}

fn set_diff(a: &[String], b: &[String]) -> Vec<String> {
    let bset: HashSet<_> = b.iter().cloned().collect();
    let mut diff: Vec<String> = a
        .iter()
        .cloned()
        .filter(|item| !bset.contains(item))
        .collect();
    diff.sort();
    diff.dedup();
    diff
}

fn write_string_section(out: &mut String, title: &str, items: &[String]) {
    out.push_str(&format!("### {}\n", title));
    if items.is_empty() {
        out.push_str("- None\n");
    } else {
        for item in items {
            out.push_str(&format!("- {}\n", item));
        }
    }
}

fn pluralize<'a>(count: usize, singular: &'a str, plural: &'a str) -> &'a str {
    if count == 1 {
        singular
    } else {
        plural
    }
}

fn render_next_queries(trace: &ProcedureTrace, groups: &DependencyGroups) -> Vec<String> {
    let mut queries = Vec::new();

    if contains_risk(trace, "multi_join_aggregation") {
        let source = groups
            .reads
            .first()
            .cloned()
            .unwrap_or_else(|| "<SOURCE_TABLE>".to_string());
        queries.push(format!(
            "SELECT ORDER_NO, PART_NO, COUNT(*) AS CNT\nFROM {}\nGROUP BY ORDER_NO, PART_NO\nHAVING COUNT(*) > 1;",
            source
        ));
    }

    if contains_risk(trace, "update_without_where") || contains_risk(trace, "delete_without_where")
    {
        let target = groups
            .writes
            .first()
            .cloned()
            .unwrap_or_else(|| "<TARGET_TABLE>".to_string());
        queries.push(format!("SELECT TOP 100 *\nFROM {};", target));
    }

    if contains_risk(trace, "dynamic_sql") {
        queries.push(
            "-- Inspect the generated SQL string before execution\n-- PRINT @SQL;".to_string(),
        );
    }

    queries
}

fn contains_risk(trace: &ProcedureTrace, rule_id: &str) -> bool {
    trace.risks.iter().any(|risk| risk.rule_id == rule_id)
}

fn node_id(label: &str) -> String {
    let mut out = String::from("N_");
    for ch in label.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    while out.ends_with('_') {
        out.pop();
    }
    if out == "N_" {
        out.push_str("UNKNOWN");
    }
    out
}

fn escape_label(label: &str) -> String {
    label.replace('"', "'")
}

pub fn render_folder_mermaid(traces: &[ProcedureTrace]) -> String {
    let mut lines = Vec::new();
    let mut emitted_nodes = HashSet::new();
    let mut edges = HashSet::new();

    lines.push("flowchart LR".to_string());

    for trace in traces {
        let proc_name = trace.name.as_deref().unwrap_or("UNKNOWN_PROC");
        let proc_id = node_id(proc_name);
        let proc_label = escape_label(proc_name);

        if emitted_nodes.insert(proc_id.clone()) {
            lines.push(format!("    {}[\"{}\"]", proc_id, proc_label));
        }

        for dep in &trace.dependencies {
            let dep_id = node_id(&dep.object);
            if emitted_nodes.insert(dep_id.clone()) {
                lines.push(format!("    {}[\"{}\"]", dep_id, escape_label(&dep.object)));
            }

            let edge = match dep.operation {
                Operation::Read => format!("    {} --> {}", dep_id, proc_id),
                Operation::Write => format!("    {} --> {}", proc_id, dep_id),
                Operation::Execute => format!("    {} -. executes .-> {}", proc_id, dep_id),
                Operation::Unknown => continue,
            };

            if edges.insert(edge.clone()) {
                lines.push(edge);
            }
        }
    }

    if lines.len() == 1 {
        lines.push("    %% No dependencies detected".to_string());
    }

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Parameter, RiskFinding, Severity, TraceMetrics};

    fn sample_trace() -> ProcedureTrace {
        ProcedureTrace {
            name: Some("SP_TEST".to_string()),
            parameters: vec![Parameter {
                name: "@ORDER_NO".to_string(),
                data_type: "VARCHAR(20)".to_string(),
                default_value: None,
            }],
            dependencies: vec![
                Dependency {
                    object: "TB_A".to_string(),
                    operation: Operation::Read,
                    source: "FROM".to_string(),
                },
                Dependency {
                    object: "TB_OUT".to_string(),
                    operation: Operation::Write,
                    source: "INSERT INTO".to_string(),
                },
            ],
            temp_tables: vec!["#TMP_A".to_string()],
            risks: vec![RiskFinding {
                rule_id: "select_star".to_string(),
                severity: Severity::Low,
                message: "SELECT * found.".to_string(),
                suggestion: "List columns explicitly.".to_string(),
            }],
            statements: vec![],
            metrics: TraceMetrics {
                statement_count: 2,
                read_count: 1,
                write_count: 1,
                risk_level: Severity::Low,
            },
        }
    }

    #[test]
    fn terminal_report_contains_core_sections() {
        let report = render_terminal(&sample_trace());
        assert!(report.contains("Procedure: SP_TEST"));
        assert!(report.contains("Tables Read:"));
        assert!(report.contains("Risks:"));
    }

    #[test]
    fn markdown_contains_core_sections() {
        let report = render_markdown(&sample_trace());
        assert!(report.contains("# SPTrace Report: SP_TEST"));
        assert!(report.contains("## 1. Overview"));
        assert!(report.contains("## 7. Risk Findings"));
        assert!(report.contains("```mermaid"));
    }

    #[test]
    fn mermaid_contains_dependency_edges() {
        let diagram = render_mermaid(&sample_trace());
        assert!(diagram.contains("flowchart LR"));
        assert!(diagram.contains("-->"));
    }

    #[test]
    fn context_contains_ai_sections() {
        let context = render_context(&sample_trace());
        assert!(context.contains("# AI Context for Stored Procedure Analysis"));
        assert!(context.contains("## Suspicious Logic"));
    }

    #[test]
    fn context_json_contains_sections() {
        let context = render_context_json(&sample_trace());
        assert!(context.contains("\"procedure\""));
        assert!(context.contains("\"suggested_queries\""));
    }

    #[test]
    fn dependency_index_contains_rows() {
        let index = render_dependency_index(&[ScanIndexEntry {
            source_file: "a.sql".to_string(),
            report_file: "a.md".to_string(),
            procedure: "SP_TEST".to_string(),
            read_count: 1,
            write_count: 1,
            risk_level: Severity::Low,
            risk_rules: vec!["select_star".to_string()],
        }]);
        assert!(index.contains("# SPTrace Dependency Index"));
        assert!(index.contains("a.sql"));
        assert!(index.contains("select_star"));
    }

    #[test]
    fn diff_contains_changes() {
        let before = sample_trace();
        let mut after = sample_trace();
        after.parameters.push(Parameter {
            name: "@X".to_string(),
            data_type: "INT".to_string(),
            default_value: None,
        });
        after.dependencies.push(Dependency {
            object: "TB_NEW".to_string(),
            operation: Operation::Read,
            source: "JOIN".to_string(),
        });
        after.risks.push(RiskFinding {
            rule_id: "dynamic_sql".to_string(),
            severity: Severity::Medium,
            message: "Dynamic SQL found.".to_string(),
            suggestion: "Review generated SQL string.".to_string(),
        });

        let diff = render_diff(&before, &after, "before.sql", "after.sql");
        assert!(diff.contains("# SPTrace Diff Report"));
        assert!(diff.contains("Added"));
        assert!(diff.contains("TB_NEW"));
        assert!(diff.contains("dynamic_sql"));
    }

    #[test]
    fn diff_json_contains_sections() {
        let before = sample_trace();
        let mut after = sample_trace();
        after.parameters.push(Parameter {
            name: "@X".to_string(),
            data_type: "INT".to_string(),
            default_value: None,
        });
        let json = render_diff_json(&before, &after, "before.sql", "after.sql");
        assert!(json.contains("\"parameter_changes\""));
        assert!(json.contains("\"dependency_changes\""));
    }
}
