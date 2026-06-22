use crate::model::{ProcedureTrace, RiskFinding, Severity};
use crate::normalizer::split_statements;
use crate::parser::extract_body_sql;
use regex::Regex;
use std::collections::HashSet;

pub fn detect_risks(sql: &str, trace: &ProcedureTrace) -> Vec<RiskFinding> {
    detect_risks_with_config(sql, trace, &crate::model::Config::default())
}

pub fn detect_risks_with_config(sql: &str, trace: &ProcedureTrace, config: &crate::model::Config) -> Vec<RiskFinding> {
    let body = extract_body_sql(sql);
    let mut findings = Vec::new();

    if let Some(finding) = rule_select_star(&body) {
        findings.push(finding);
    }
    if let Some(finding) = rule_nolock_used(&body) {
        findings.push(finding);
    }
    if let Some(finding) = rule_dynamic_sql(&body) {
        findings.push(finding);
    }
    if let Some(finding) = rule_linked_server(&body) {
        findings.push(finding);
    }

    findings.extend(rule_update_without_where(&body));
    findings.extend(rule_delete_without_where(&body));

    if let Some(finding) = rule_insert_select_no_distinct(&body) {
        findings.push(finding);
    }
    if let Some(finding) = rule_multi_join_aggregation(&body) {
        findings.push(finding);
    }
    if let Some(finding) = rule_cursor_used(&body) {
        findings.push(finding);
    }
    if let Some(finding) = rule_transaction_without_trycatch(&body) {
        findings.push(finding);
    }
    if let Some(finding) = rule_trycatch_without_rollback(&body) {
        findings.push(finding);
    }
    if let Some(finding) = rule_hardcoded_date(&body) {
        findings.push(finding);
    }
    if let Some(finding) = rule_status_magic_number(&body) {
        findings.push(finding);
    }
    if let Some(finding) = rule_temp_table_chain(trace) {
        findings.push(finding);
    }

    let mut processed_findings = Vec::new();
    for mut finding in findings {
        if let Some(rule_cfg) = config.rules.get(&finding.rule_id) {
            match rule_cfg {
                crate::model::RuleConfig::Bool(enabled) => {
                    if *enabled {
                        processed_findings.push(finding);
                    }
                }
                crate::model::RuleConfig::Severity(sev) => {
                    finding.severity = sev.clone();
                    processed_findings.push(finding);
                }
            }
        } else {
            processed_findings.push(finding);
        }
    }

    dedupe_and_sort(processed_findings)
}

fn dedupe_and_sort(findings: Vec<RiskFinding>) -> Vec<RiskFinding> {
    let mut seen = HashSet::new();
    let mut unique = Vec::new();

    for finding in findings {
        let key = (
            finding.rule_id.clone(),
            finding.severity.clone(),
            finding.message.clone(),
            finding.suggestion.clone(),
        );
        if seen.insert(key) {
            unique.push(finding);
        }
    }

    unique.sort_by(|a, b| {
        b.severity
            .rank()
            .cmp(&a.severity.rank())
            .then_with(|| a.rule_id.cmp(&b.rule_id))
    });

    unique
}

fn rule_select_star(sql: &str) -> Option<RiskFinding> {
    let re = Regex::new(r"(?is)\bSELECT\s+\*").unwrap();
    re.find(sql).map(|_| {
        risk(
            "select_star",
            Severity::Low,
            "SELECT * found. This may make the procedure fragile when table schema changes.",
            "List required columns explicitly.",
        )
    })
}

fn rule_nolock_used(sql: &str) -> Option<RiskFinding> {
    let re = Regex::new(r"(?is)WITH\s*\(\s*NOLOCK\s*\)").unwrap();
    re.find(sql).map(|_| {
        risk(
            "nolock_used",
            Severity::Medium,
            "NOLOCK found. Query may read dirty or uncommitted data.",
            "Verify whether dirty reads are acceptable for this procedure.",
        )
    })
}

fn rule_dynamic_sql(sql: &str) -> Option<RiskFinding> {
    let exec_dynamic = Regex::new(r"(?is)\bEXEC(?:UTE)?\s*\(\s*@").unwrap();
    let sp_executesql = Regex::new(r"(?is)\bsp_executesql\b").unwrap();

    if exec_dynamic.is_match(sql) || sp_executesql.is_match(sql) {
        Some(risk(
            "dynamic_sql",
            Severity::Medium,
            "Dynamic SQL found. Static dependency detection may be incomplete.",
            "Review generated SQL string and validate runtime dependencies manually.",
        ))
    } else {
        None
    }
}

fn rule_linked_server(sql: &str) -> Option<RiskFinding> {
    let re = Regex::new(
        r"(?is)\b[A-Za-z0-9_\[\]]+\.[A-Za-z0-9_\[\]]+\.[A-Za-z0-9_\[\]]+\.[A-Za-z0-9_\[\]]+\b",
    )
    .unwrap();
    re.find(sql).map(|_| risk(
        "linked_server",
        Severity::Medium,
        "Linked server or four-part object reference found. External DB dependency should be verified.",
        "Verify linked server availability, permissions, and source data freshness.",
    ))
}

fn rule_update_without_where(sql: &str) -> Vec<RiskFinding> {
    split_statements(sql)
        .into_iter()
        .filter_map(|stmt| clean_statement_for_rule(&stmt))
        .filter_map(|stmt| {
            let upper = stmt.trim_start().to_ascii_uppercase();
            if upper.starts_with("UPDATE") && upper.contains(" SET ") && !upper.contains(" WHERE ")
            {
                Some(risk(
                    "update_without_where",
                    Severity::High,
                    "UPDATE statement may be missing WHERE condition.",
                    "Confirm this update is intended to affect all rows, or add a WHERE clause.",
                ))
            } else {
                None
            }
        })
        .collect()
}

fn rule_delete_without_where(sql: &str) -> Vec<RiskFinding> {
    split_statements(sql)
        .into_iter()
        .filter_map(|stmt| clean_statement_for_rule(&stmt))
        .filter_map(|stmt| {
            let upper = stmt.trim_start().to_ascii_uppercase();
            if upper.starts_with("DELETE") && !upper.contains(" WHERE ") {
                Some(risk(
                    "delete_without_where",
                    Severity::High,
                    "DELETE statement may be missing WHERE condition.",
                    "Confirm this delete is intended to affect all rows, or add a WHERE clause.",
                ))
            } else {
                None
            }
        })
        .collect()
}

fn rule_insert_select_no_distinct(sql: &str) -> Option<RiskFinding> {
    for stmt in split_statements(sql) {
        let Some(stmt) = clean_statement_for_rule(&stmt) else {
            continue;
        };
        let upper = stmt.trim_start().to_ascii_uppercase();
        if upper.starts_with("INSERT INTO") && upper.contains(" SELECT ") {
            if !upper.contains(" DISTINCT ") && !upper.contains(" GROUP BY ") {
                return Some(risk(
                    "insert_select_no_distinct",
                    Severity::Medium,
                    "INSERT INTO ... SELECT without DISTINCT or GROUP BY found. Duplicate rows may be inserted if source rows are duplicated.",
                    "Verify source row uniqueness or add explicit duplicate prevention.",
                ));
            }
        }
    }
    None
}

fn rule_multi_join_aggregation(sql: &str) -> Option<RiskFinding> {
    let upper = sql.to_ascii_uppercase();
    let has_aggregation = upper.contains("SUM(") || upper.contains("COUNT(");
    let join_count = Regex::new(r"(?is)\bJOIN\b").unwrap().find_iter(sql).count();
    let has_group_by = Regex::new(r"(?is)\bGROUP\s+BY\b").unwrap().is_match(sql);

    if has_aggregation && join_count >= 1 && has_group_by {
        Some(risk(
            "multi_join_aggregation",
            Severity::High,
            "SUM/COUNT used with JOIN and GROUP BY. Aggregated values may be duplicated if join keys are not unique.",
            "Check join key uniqueness, pre-aggregate detail rows before joins, and verify whether additional keys such as MANIFEST_NO, ORDER_NO, or PART_NO are required.",
        ))
    } else {
        None
    }
}

fn rule_cursor_used(sql: &str) -> Option<RiskFinding> {
    let re = Regex::new(r"(?is)\bCURSOR\b").unwrap();
    re.find(sql).map(|_| {
        risk(
            "cursor_used",
            Severity::Medium,
            "CURSOR usage found. This may indicate row-by-row processing and performance risk.",
            "Check whether cursor logic can be replaced with set-based operations.",
        )
    })
}

fn rule_transaction_without_trycatch(sql: &str) -> Option<RiskFinding> {
    let upper = sql.to_ascii_uppercase();
    let has_transaction = upper.contains("BEGIN TRAN") || upper.contains("BEGIN TRANSACTION");
    let has_try = upper.contains("BEGIN TRY");
    let has_catch = upper.contains("BEGIN CATCH");

    if has_transaction && !(has_try && has_catch) {
        Some(risk(
            "transaction_without_trycatch",
            Severity::Medium,
            "Transaction found without TRY/CATCH error handling.",
            "Consider adding TRY/CATCH with COMMIT/ROLLBACK handling.",
        ))
    } else {
        None
    }
}

fn rule_trycatch_without_rollback(sql: &str) -> Option<RiskFinding> {
    let upper = sql.to_ascii_uppercase();
    let has_try = upper.contains("BEGIN TRY");
    let has_catch = upper.contains("BEGIN CATCH");
    let has_rollback = upper.contains("ROLLBACK");

    if has_try && has_catch && !has_rollback {
        Some(risk(
            "trycatch_without_rollback",
            Severity::High,
            "TRY/CATCH found without ROLLBACK. Failed transactions may remain open or partially applied.",
            "Verify transaction handling and add ROLLBACK in CATCH when appropriate.",
        ))
    } else {
        None
    }
}

fn rule_hardcoded_date(sql: &str) -> Option<RiskFinding> {
    let re = Regex::new(r"'\d{4}-\d{2}-\d{2}'").unwrap();
    re.find(sql).map(|_| {
        risk(
            "hardcoded_date",
            Severity::Low,
            "Hardcoded date literal found.",
            "Verify whether date should be parameterized.",
        )
    })
}

fn rule_status_magic_number(sql: &str) -> Option<RiskFinding> {
    let re = Regex::new(r"(?is)\bSTATUS\s*=\s*\d+").unwrap();
    re.find(sql).map(|_| {
        risk(
        "status_magic_number",
        Severity::Low,
        "Status magic number found.",
        "Consider documenting status meaning or replacing with named constants/reference table.",
    )
    })
}

fn rule_temp_table_chain(trace: &ProcedureTrace) -> Option<RiskFinding> {
    if trace.temp_tables.len() >= 3 {
        Some(risk(
            "temp_table_chain",
            Severity::Low,
            "Multiple temp tables found. Procedure flow may be complex and order-dependent.",
            "Review temp table lifecycle and ensure intermediate data is deduplicated where needed.",
        ))
    } else {
        None
    }
}

fn clean_statement_for_rule(stmt: &str) -> Option<String> {
    let mut tokens: Vec<&str> = stmt.trim().split_whitespace().collect();
    if tokens.is_empty() {
        return None;
    }

    loop {
        if tokens.is_empty() {
            return None;
        }

        let first = tokens[0];
        if first.eq_ignore_ascii_case("BEGIN") {
            if let Some(next) = tokens.get(1) {
                let next_upper = next.to_ascii_uppercase();
                if matches!(
                    next_upper.as_str(),
                    "TRY" | "CATCH" | "TRAN" | "TRANSACTION"
                ) {
                    tokens.drain(0..2);
                } else {
                    tokens.drain(0..1);
                }
                continue;
            }
            return None;
        }

        if first.eq_ignore_ascii_case("END") {
            if let Some(next) = tokens.get(1) {
                let next_upper = next.to_ascii_uppercase();
                if matches!(next_upper.as_str(), "TRY" | "CATCH") {
                    tokens.drain(0..2);
                } else {
                    tokens.drain(0..1);
                }
                continue;
            }
            return None;
        }

        break;
    }

    if tokens.is_empty() {
        None
    } else {
        Some(tokens.join(" "))
    }
}

fn risk(rule_id: &str, severity: Severity, message: &str, suggestion: &str) -> RiskFinding {
    RiskFinding {
        rule_id: rule_id.to_string(),
        severity,
        message: message.to_string(),
        suggestion: suggestion.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ProcedureTrace, TraceMetrics};

    fn empty_trace() -> ProcedureTrace {
        ProcedureTrace {
            name: Some("dbo.SP_TEST".to_string()),
            parameters: Vec::new(),
            dependencies: Vec::new(),
            temp_tables: Vec::new(),
            risks: Vec::new(),
            statements: Vec::new(),
            metrics: TraceMetrics::default(),
        }
    }

    #[test]
    fn detects_select_star() {
        let sql = "SELECT * FROM TB_A";
        let risks = detect_risks(sql, &empty_trace());
        assert!(risks.iter().any(|r| r.rule_id == "select_star"));
    }

    #[test]
    fn detects_nolock() {
        let sql = "SELECT * FROM TB_A WITH (NOLOCK)";
        let risks = detect_risks(sql, &empty_trace());
        assert!(risks.iter().any(|r| r.rule_id == "nolock_used"));
    }

    #[test]
    fn detects_dynamic_sql() {
        let sql = "EXEC(@SQL)";
        let risks = detect_risks(sql, &empty_trace());
        assert!(risks.iter().any(|r| r.rule_id == "dynamic_sql"));
    }

    #[test]
    fn detects_linked_server() {
        let sql = "SELECT * FROM IPPCS_PROD.MAIN_DB.dbo.TB_A";
        let risks = detect_risks(sql, &empty_trace());
        assert!(risks.iter().any(|r| r.rule_id == "linked_server"));
    }

    #[test]
    fn detects_update_without_where() {
        let sql = "UPDATE TB_A SET FLAG = 1;";
        let risks = detect_risks(sql, &empty_trace());
        assert!(risks.iter().any(|r| r.rule_id == "update_without_where"));
    }

    #[test]
    fn detects_delete_without_where() {
        let sql = "DELETE FROM TB_A;";
        let risks = detect_risks(sql, &empty_trace());
        assert!(risks.iter().any(|r| r.rule_id == "delete_without_where"));
    }

    #[test]
    fn detects_insert_select_no_distinct() {
        let sql = "INSERT INTO TB_OUT SELECT COL FROM TB_A;";
        let risks = detect_risks(sql, &empty_trace());
        assert!(risks
            .iter()
            .any(|r| r.rule_id == "insert_select_no_distinct"));
    }

    #[test]
    fn detects_multi_join_aggregation() {
        let sql = r#"
            SELECT A.ID, SUM(B.QTY)
            FROM TB_A A
            JOIN TB_B B ON A.ID = B.ID
            GROUP BY A.ID
        "#;
        let risks = detect_risks(sql, &empty_trace());
        assert!(risks.iter().any(|r| r.rule_id == "multi_join_aggregation"));
    }

    #[test]
    fn detects_transaction_without_trycatch() {
        let sql = "BEGIN TRAN; UPDATE TB_A SET FLAG = 1; COMMIT;";
        let risks = detect_risks(sql, &empty_trace());
        assert!(risks
            .iter()
            .any(|r| r.rule_id == "transaction_without_trycatch"));
    }

    #[test]
    fn detects_trycatch_without_rollback() {
        let sql = r#"
            BEGIN TRY
                UPDATE TB_A SET FLAG = 1;
            END TRY
            BEGIN CATCH
                SELECT ERROR_MESSAGE();
            END CATCH
        "#;
        let risks = detect_risks(sql, &empty_trace());
        assert!(risks
            .iter()
            .any(|r| r.rule_id == "trycatch_without_rollback"));
    }

    #[test]
    fn detects_temp_table_chain() {
        let mut trace = empty_trace();
        trace.temp_tables = vec!["#A".to_string(), "#B".to_string(), "#C".to_string()];
        let risks = detect_risks("SELECT 1", &trace);
        assert!(risks.iter().any(|r| r.rule_id == "temp_table_chain"));
    }

    #[test]
    fn sorts_high_before_medium_before_low() {
        let sql = r#"
            SELECT * FROM TB_A WITH (NOLOCK);
            UPDATE TB_B SET FLAG = 1;
        "#;
        let risks = detect_risks(sql, &empty_trace());
        assert!(risks.first().unwrap().severity >= risks.last().unwrap().severity);
    }

    #[test]
    fn test_config_overrides() {
        let sql = r#"
            SELECT * FROM TB_A WITH (NOLOCK);
        "#;
        let trace_default = crate::analyzer::analyze_sql(sql).unwrap();
        assert!(trace_default.risks.iter().any(|r| r.rule_id == "select_star" && r.severity == Severity::Low));
        assert!(trace_default.risks.iter().any(|r| r.rule_id == "nolock_used" && r.severity == Severity::Medium));

        let mut config = crate::model::Config::default();
        config.rules.insert("select_star".to_string(), crate::model::RuleConfig::Bool(false));
        config.rules.insert("nolock_used".to_string(), crate::model::RuleConfig::Severity(Severity::High));

        let trace_custom = crate::analyzer::analyze_sql_with_config(sql, &config).unwrap();
        assert!(!trace_custom.risks.iter().any(|r| r.rule_id == "select_star"));
        assert!(trace_custom.risks.iter().any(|r| r.rule_id == "nolock_used" && r.severity == Severity::High));
    }
}
