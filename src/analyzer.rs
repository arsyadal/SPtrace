use anyhow::Result;

use crate::model::{Operation, ProcedureTrace, Severity, TraceMetrics};
use crate::parser::{
    extract_dependencies, extract_parameters, extract_procedure_name, extract_temp_tables,
    summarize_statements,
};

pub fn analyze_sql(sql: &str) -> Result<ProcedureTrace> {
    analyze_sql_with_config(sql, &crate::model::Config::default())
}

pub fn analyze_sql_with_config(sql: &str, config: &crate::model::Config) -> Result<ProcedureTrace> {
    let mut trace = ProcedureTrace::default();
    trace.name = extract_procedure_name(sql);
    trace.parameters = extract_parameters(sql);
    trace.dependencies = extract_dependencies(sql);
    trace.temp_tables = extract_temp_tables(sql);
    trace.statements = summarize_statements(sql);
    trace.risks = crate::rules::detect_risks_with_config(sql, &trace, config);
    trace.metrics = compute_metrics(&trace);
    Ok(trace)
}

fn compute_metrics(trace: &ProcedureTrace) -> TraceMetrics {
    let read_count = trace
        .dependencies
        .iter()
        .filter(|dep| dep.operation == Operation::Read)
        .count();
    let write_count = trace
        .dependencies
        .iter()
        .filter(|dep| dep.operation == Operation::Write)
        .count();
    let risk_level = highest_severity(&trace.risks);

    TraceMetrics {
        statement_count: trace.statements.len(),
        read_count,
        write_count,
        risk_level,
    }
}

fn highest_severity(risks: &[crate::model::RiskFinding]) -> Severity {
    risks
        .iter()
        .map(|risk| risk.severity.clone())
        .max()
        .unwrap_or(Severity::Low)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analyzes_duplicate_aggregation_sql() {
        let sql = r#"
            CREATE PROCEDURE SP_GENERATE_GR_SAMPLE
                @ORDER_NO VARCHAR(20)
            AS
            BEGIN
                INSERT INTO TB_T_GR_HUB
                SELECT
                    h.ORDER_NO,
                    d.PART_NO,
                    SUM(d.QTY) AS TOTAL_QTY,
                    SUM(d.AMOUNT) AS TOTAL_AMOUNT
                FROM TB_R_DAILY_ORDER_PART d
                JOIN TB_R_DELIVERY_CTL_H h
                    ON d.ORDER_NO = h.ORDER_NO
                JOIN TB_R_DELIVERY_CTL_D x
                    ON h.DELIVERY_NO = x.DELIVERY_NO
                WHERE h.ORDER_NO = @ORDER_NO
                GROUP BY h.ORDER_NO, d.PART_NO
            END
        "#;

        let trace = analyze_sql(sql).unwrap();
        assert_eq!(trace.name.as_deref(), Some("SP_GENERATE_GR_SAMPLE"));
        assert_eq!(trace.parameters.len(), 1);
        assert!(trace
            .dependencies
            .iter()
            .any(|dep| dep.object == "TB_R_DAILY_ORDER_PART"));
        assert!(trace
            .dependencies
            .iter()
            .any(|dep| dep.object == "TB_T_GR_HUB"));
        assert!(trace
            .risks
            .iter()
            .any(|risk| risk.rule_id == "multi_join_aggregation"));
        assert_eq!(trace.metrics.risk_level, Severity::High);
        assert_eq!(trace.metrics.write_count, 1);
        assert!(trace.metrics.read_count >= 3);
    }
}
