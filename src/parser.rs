use crate::model::{Dependency, Operation, Parameter, StatementKind, StatementSummary};
use crate::normalizer::{normalize_identifier, split_statements, strip_comments};
use regex::Regex;
use std::collections::HashSet;

pub fn extract_procedure_name(sql: &str) -> Option<String> {
    let stripped = strip_comments(sql);
    let re = Regex::new(
        r"(?is)\b(?:CREATE|ALTER)\s+(?:OR\s+ALTER\s+)?(?:PROCEDURE|PROC)\s+([A-Za-z0-9_\[\]\.]+)",
    )
    .ok()?;

    re.captures(&stripped)
        .and_then(|caps| caps.get(1))
        .map(|m| normalize_identifier(m.as_str()))
}

pub fn extract_body_sql(sql: &str) -> String {
    let stripped = strip_comments(sql);
    let Some(decl_end) = procedure_decl_end(&stripped) else {
        return stripped.trim().to_string();
    };

    if let Some(body_start) = find_body_start(&stripped, decl_end) {
        stripped[body_start..].trim().to_string()
    } else {
        stripped.trim().to_string()
    }
}

pub fn extract_parameters(sql: &str) -> Vec<Parameter> {
    let stripped = strip_comments(sql);
    let Some(decl_end) = procedure_decl_end(&stripped) else {
        return Vec::new();
    };

    let body_start = find_body_start(&stripped, decl_end).unwrap_or(stripped.len());
    let mut block = stripped[decl_end..body_start].trim();
    block = strip_outer_parens(block);

    split_parameter_items(block)
        .into_iter()
        .filter_map(|item| parse_parameter_item(&item))
        .collect()
}

pub fn extract_dependencies(sql: &str) -> Vec<Dependency> {
    let body = extract_body_sql(sql);

    let mut candidates: Vec<(usize, Dependency)> = Vec::new();

    for stmt in split_statements(&body) {
        let stmt = stmt.trim();
        if stmt.is_empty() {
            continue;
        }

        let write_deps = collect_write_dependencies(stmt);
        let write_targets: HashSet<String> = write_deps
            .iter()
            .map(|(_, dep)| dep.object.clone())
            .collect();
        candidates.extend(write_deps);

        let read_patterns = [
            (r"(?is)\bFROM\s+([A-Za-z0-9_#@\[\]\.]+)", "FROM"),
            (r"(?is)\bJOIN\s+([A-Za-z0-9_#@\[\]\.]+)", "JOIN"),
            (r"(?is)\bUSING\s+([A-Za-z0-9_#@\[\]\.]+)", "USING"),
        ];

        for (pattern, source) in read_patterns {
            candidates.extend(collect_dependencies(
                stmt,
                pattern,
                Operation::Read,
                source,
                Some(&write_targets),
            ));
        }

        candidates.extend(collect_exec_dependencies(stmt));
    }

    candidates.sort_by_key(|(pos, _)| *pos);

    let mut seen = HashSet::new();
    let mut dependencies = Vec::new();

    for (_, dep) in candidates {
        let key = (
            dep.object.clone(),
            dep.operation.clone(),
            dep.source.clone(),
        );
        if seen.insert(key) {
            dependencies.push(dep);
        }
    }

    dependencies
}

pub fn extract_temp_tables(sql: &str) -> Vec<String> {
    let body = extract_body_sql(sql);
    let re = Regex::new(r"(?i)#{1,2}[A-Za-z0-9_]+").unwrap();
    let mut tables: Vec<String> = re
        .find_iter(&body)
        .map(|m| normalize_identifier(m.as_str()))
        .filter(|name| !name.is_empty())
        .collect();
    tables.sort();
    tables.dedup();
    tables
}

pub fn summarize_statements(sql: &str) -> Vec<StatementSummary> {
    let body = extract_body_sql(sql);
    let statements = split_statements(&body);

    statements
        .into_iter()
        .map(|stmt| clean_statement_for_summary(&stmt))
        .filter(|stmt| !stmt.is_empty())
        .enumerate()
        .map(|(index, stmt)| {
            let kind = classify_statement(&stmt);
            let deps = extract_dependencies(&stmt);
            let sources = deps
                .iter()
                .filter(|dep| dep.operation == Operation::Read)
                .map(|dep| dep.object.clone())
                .collect::<Vec<_>>();
            let target = deps
                .iter()
                .find(|dep| {
                    dep.operation == Operation::Write || dep.operation == Operation::Execute
                })
                .map(|dep| dep.object.clone());

            StatementSummary {
                index: index + 1,
                kind,
                target,
                sources,
            }
        })
        .collect()
}

fn procedure_decl_end(stripped: &str) -> Option<usize> {
    let re = Regex::new(
        r"(?is)\b(?:CREATE|ALTER)\s+(?:OR\s+ALTER\s+)?(?:PROCEDURE|PROC)\s+([A-Za-z0-9_\[\]\.]+)",
    )
    .unwrap();

    re.captures(stripped)
        .and_then(|caps| caps.get(1))
        .map(|m| m.end())
}

fn find_body_start(stripped: &str, search_from: usize) -> Option<usize> {
    let remainder = &stripped[search_from..];
    let as_re = Regex::new(r"(?is)\bAS\b").unwrap();

    for m in as_re.find_iter(remainder) {
        let before = &remainder[..m.start()];
        let prev_word = before.split_whitespace().last().unwrap_or("");
        if !prev_word.eq_ignore_ascii_case("EXECUTE") {
            let mut idx = search_from + m.end();
            while idx < stripped.len() {
                let next = stripped[idx..].chars().next().unwrap();
                if next.is_whitespace() {
                    idx += next.len_utf8();
                } else {
                    break;
                }
            }
            return Some(idx);
        }
    }

    None
}

fn split_parameter_items(block: &str) -> Vec<String> {
    let mut items = Vec::new();
    let mut current = String::new();
    let mut depth = 0usize;
    let mut in_single = false;
    let mut chars = block.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\'' {
            current.push(c);
            if in_single {
                if matches!(chars.peek(), Some('\'')) {
                    current.push(chars.next().unwrap());
                } else {
                    in_single = false;
                }
            } else {
                in_single = true;
            }
            continue;
        }

        if !in_single {
            match c {
                '(' => {
                    depth += 1;
                    current.push(c);
                    continue;
                }
                ')' => {
                    if depth > 0 {
                        depth -= 1;
                    }
                    current.push(c);
                    continue;
                }
                ',' if depth == 0 => {
                    if !current.trim().is_empty() {
                        items.push(current.trim().to_string());
                    }
                    current.clear();
                    continue;
                }
                _ => {}
            }
        }

        current.push(c);
    }

    if !current.trim().is_empty() {
        items.push(current.trim().to_string());
    }

    items
}

fn parse_parameter_item(item: &str) -> Option<Parameter> {
    let mut decl = item
        .trim()
        .trim_end_matches(',')
        .trim()
        .trim_end_matches(';')
        .trim();
    if decl.is_empty() || !decl.starts_with('@') {
        return None;
    }

    if let Some(stripped) = strip_suffix_ci(decl, " OUTPUT") {
        decl = stripped.trim_end();
    }

    let decl = strip_trailing_keywords(decl, &["AS", "BEGIN", "END"]);

    let mut parts = decl.split_whitespace();
    let name = parts.next()?.to_string();
    let rest = parts.collect::<Vec<_>>().join(" ");
    if rest.is_empty() {
        return Some(Parameter {
            name,
            data_type: String::new(),
            default_value: None,
        });
    }

    let (data_type, default_value) = split_first_assignment(&rest);
    let default_value = default_value.and_then(|value| {
        let cleaned = value.trim().trim_end_matches(',').trim();
        if cleaned.is_empty() {
            None
        } else {
            Some(cleaned.to_string())
        }
    });

    Some(Parameter {
        name,
        data_type: data_type.trim().to_string(),
        default_value,
    })
}

fn strip_outer_parens(block: &str) -> &str {
    let trimmed = block.trim();
    if trimmed.starts_with('(') && trimmed.ends_with(')') {
        trimmed[1..trimmed.len() - 1].trim()
    } else {
        trimmed
    }
}

fn split_first_assignment(input: &str) -> (String, Option<String>) {
    let mut depth = 0isize;
    let mut in_single = false;
    let mut chars = input.char_indices().peekable();

    while let Some((idx, c)) = chars.next() {
        if c == '\'' {
            if in_single {
                if matches!(chars.peek(), Some((_, '\''))) {
                    chars.next();
                } else {
                    in_single = false;
                }
            } else {
                in_single = true;
            }
            continue;
        }

        if in_single {
            continue;
        }

        match c {
            '(' => depth += 1,
            ')' => {
                if depth > 0 {
                    depth -= 1;
                }
            }
            '=' if depth == 0 => {
                let left = input[..idx].trim().to_string();
                let right = input[idx + c.len_utf8()..].trim().to_string();
                return (left, Some(right));
            }
            _ => {}
        }
    }

    (input.trim().to_string(), None)
}

fn strip_suffix_ci<'a>(input: &'a str, suffix: &str) -> Option<&'a str> {
    if input.len() >= suffix.len()
        && input[input.len() - suffix.len()..].eq_ignore_ascii_case(suffix)
    {
        Some(&input[..input.len() - suffix.len()])
    } else {
        None
    }
}

fn strip_trailing_keywords(input: &str, keywords: &[&str]) -> String {
    let mut tokens: Vec<&str> = input.split_whitespace().collect();
    while let Some(last) = tokens.last() {
        if keywords
            .iter()
            .any(|keyword| last.eq_ignore_ascii_case(keyword))
        {
            tokens.pop();
        } else {
            break;
        }
    }
    tokens.join(" ")
}

fn clean_statement_for_summary(stmt: &str) -> String {
    let trimmed = stmt.trim();
    if trimmed.eq_ignore_ascii_case("END") {
        return String::new();
    }

    let tokens: Vec<&str> = trimmed.split_whitespace().collect();
    if tokens
        .first()
        .map(|token| token.eq_ignore_ascii_case("BEGIN"))
        .unwrap_or(false)
    {
        if let Some(next) = tokens.get(1) {
            let next_upper = next.to_ascii_uppercase();
            if !matches!(
                next_upper.as_str(),
                "TRY" | "CATCH" | "TRAN" | "TRANSACTION"
            ) {
                return tokens[1..].join(" ");
            }
        }
    }

    trimmed.to_string()
}

fn collect_dependencies(
    body: &str,
    pattern: &str,
    operation: Operation,
    source: &str,
    skip_objects: Option<&HashSet<String>>,
) -> Vec<(usize, Dependency)> {
    let re = Regex::new(pattern).unwrap();
    re.captures_iter(body)
        .filter_map(|caps| {
            let m = caps.get(1)?;
            let raw = m.as_str().trim();
            if raw.is_empty() || raw.starts_with('@') {
                return None;
            }
            if matches!(operation, Operation::Execute) && is_builtin_exec_target(raw) {
                return None;
            }
            let object = normalize_identifier(raw);
            if object.is_empty() {
                return None;
            }
            if skip_objects
                .map(|objects| matches!(source, "FROM") && objects.contains(&object))
                .unwrap_or(false)
            {
                return None;
            }
            Some((
                m.start(),
                Dependency {
                    object,
                    operation: operation.clone(),
                    source: source.to_string(),
                },
            ))
        })
        .collect()
}

fn collect_write_dependencies(body: &str) -> Vec<(usize, Dependency)> {
    let write_patterns = [
        (
            r"(?is)\bSELECT\s+INTO\s+([A-Za-z0-9_#@\[\]\.]+)",
            "SELECT INTO",
        ),
        (
            r"(?is)\bINSERT\s+INTO\s+([A-Za-z0-9_#@\[\]\.]+)",
            "INSERT INTO",
        ),
        (r"(?is)\bUPDATE\s+([A-Za-z0-9_#@\[\]\.]+)", "UPDATE"),
        (
            r"(?is)\bDELETE(?:\s+[A-Za-z0-9_#@\[\]\.]+)?\s+FROM\s+([A-Za-z0-9_#@\[\]\.]+)",
            "DELETE FROM",
        ),
        (
            r"(?is)\bMERGE\s+INTO\s+([A-Za-z0-9_#@\[\]\.]+)",
            "MERGE INTO",
        ),
    ];

    let mut candidates = Vec::new();
    for (pattern, source) in write_patterns {
        candidates.extend(collect_dependencies(
            body,
            pattern,
            Operation::Write,
            source,
            None,
        ));
    }
    candidates
}

fn collect_exec_dependencies(body: &str) -> Vec<(usize, Dependency)> {
    let re = Regex::new(r"(?is)\bEXEC(?:UTE)?\s+([A-Za-z0-9_#@\[\]\.]+)").unwrap();
    re.captures_iter(body)
        .filter_map(|caps| {
            let m = caps.get(1)?;
            let raw = m.as_str().trim();
            if raw.is_empty() || raw.starts_with('@') || is_builtin_exec_target(raw) {
                return None;
            }
            let object = normalize_identifier(raw);
            if object.is_empty() {
                return None;
            }
            Some((
                m.start(),
                Dependency {
                    object,
                    operation: Operation::Execute,
                    source: "EXEC".to_string(),
                },
            ))
        })
        .collect()
}

fn is_builtin_exec_target(raw: &str) -> bool {
    matches!(
        raw.to_ascii_lowercase().as_str(),
        "as" | "caller" | "owner" | "self" | "sp_executesql" | "sp_prepare" | "sp_execute"
    )
}

fn classify_statement(stmt: &str) -> StatementKind {
    let upper = stmt.trim_start().to_ascii_uppercase();

    if upper.starts_with("SELECT INTO") {
        StatementKind::Insert
    } else if upper.starts_with("SELECT") {
        StatementKind::Select
    } else if upper.starts_with("INSERT") {
        StatementKind::Insert
    } else if upper.starts_with("UPDATE") {
        StatementKind::Update
    } else if upper.starts_with("DELETE") {
        StatementKind::Delete
    } else if upper.starts_with("MERGE") {
        StatementKind::Merge
    } else if upper.starts_with("EXECUTE") || upper.starts_with("EXEC ") || upper == "EXEC" {
        StatementKind::Execute
    } else if upper.contains("BEGIN TRAN")
        || upper.contains("BEGIN TRANSACTION")
        || upper.contains("COMMIT")
        || upper.contains("ROLLBACK")
    {
        StatementKind::Transaction
    } else {
        StatementKind::Unknown
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_procedure_name_from_create_and_alter() {
        let sql = r#"
            CREATE PROCEDURE dbo.SP_TEST
            AS
            BEGIN
                SELECT 1
            END
        "#;
        assert_eq!(extract_procedure_name(sql), Some("dbo.SP_TEST".to_string()));

        let sql = r#"
            ALTER PROC [dbo].[SP_TEST]
            AS SELECT 1
        "#;
        assert_eq!(extract_procedure_name(sql), Some("dbo.SP_TEST".to_string()));
    }

    #[test]
    fn extracts_procedure_name_from_create_or_alter_multiline() {
        let sql = r#"
            CREATE OR ALTER PROC [dbo].[SP_TEST_EDGE]
            AS
            SELECT 1
        "#;
        assert_eq!(
            extract_procedure_name(sql),
            Some("dbo.SP_TEST_EDGE".to_string())
        );
    }

    #[test]
    fn extracts_parameters_basic() {
        let sql = r#"
            CREATE PROCEDURE dbo.SP_TEST
                @ORDER_NO VARCHAR(20),
                @PICKUP_DT DATE = NULL,
                @FLAG INT = 0
            AS
            BEGIN
                SELECT 1
            END
        "#;

        let params = extract_parameters(sql);
        assert_eq!(params.len(), 3);
        assert_eq!(params[0].name, "@ORDER_NO");
        assert_eq!(params[0].data_type, "VARCHAR(20)");
        assert_eq!(params[0].default_value, None);
        assert_eq!(params[1].name, "@PICKUP_DT");
        assert_eq!(params[1].data_type, "DATE");
        assert_eq!(params[1].default_value.as_deref(), Some("NULL"));
        assert_eq!(params[2].name, "@FLAG");
        assert_eq!(params[2].data_type, "INT");
        assert_eq!(params[2].default_value.as_deref(), Some("0"));
    }

    #[test]
    fn extracts_parameters_multiline_and_output() {
        let sql = r#"
            CREATE OR ALTER PROCEDURE dbo.SP_TEST
                @ORDER_NO VARCHAR(20),
                @FLAG INT = 1 OUTPUT,
                @NAME NVARCHAR(50) = 'A, B, C'
            AS
            SELECT 1
        "#;

        let params = extract_parameters(sql);
        assert_eq!(params.len(), 3);
        assert_eq!(params[1].name, "@FLAG");
        assert_eq!(params[1].data_type, "INT");
        assert_eq!(params[1].default_value.as_deref(), Some("1"));
        assert_eq!(params[2].name, "@NAME");
        assert_eq!(params[2].default_value.as_deref(), Some("'A, B, C'"));
    }

    #[test]
    fn extracts_parameters_without_begin_block() {
        let sql = r#"
            ALTER PROC [dbo].[SP_TEST]
                @A INT,
                @B BIT = 0
            AS
            SELECT 1
        "#;

        let params = extract_parameters(sql);
        assert_eq!(params.len(), 2);
        assert_eq!(params[0].name, "@A");
        assert_eq!(params[1].default_value.as_deref(), Some("0"));
    }

    #[test]
    fn extracts_dependencies_reads_and_writes() {
        let sql = r#"
            CREATE PROCEDURE dbo.SP_TEST
                @ORDER_NO VARCHAR(20)
            WITH EXECUTE AS OWNER
            AS
            BEGIN
                SELECT *
                FROM TB_R_A a
                JOIN TB_R_B b ON a.ID = b.ID
                SELECT INTO #TMP_A
                FROM TB_R_C c
                INSERT INTO TB_OUT
                SELECT * FROM TB_R_D
                UPDATE TB_R_E SET FLAG = 1 WHERE ID = 1
                DELETE FROM TB_R_F WHERE ID = 1
                EXEC dbo.SP_NEXT
            END
        "#;

        let deps = extract_dependencies(sql);
        let objects: Vec<_> = deps
            .iter()
            .map(|d| {
                (
                    d.object.as_str(),
                    d.operation.to_string(),
                    d.source.as_str(),
                )
            })
            .collect();

        assert!(objects.contains(&("TB_R_A", "READ".to_string(), "FROM")));
        assert!(objects.contains(&("TB_R_B", "READ".to_string(), "JOIN")));
        assert!(objects.contains(&("TB_R_C", "READ".to_string(), "FROM")));
        assert!(objects.contains(&("#TMP_A", "WRITE".to_string(), "SELECT INTO")));
        assert!(objects.contains(&("TB_OUT", "WRITE".to_string(), "INSERT INTO")));
        assert!(objects.contains(&("TB_R_E", "WRITE".to_string(), "UPDATE")));
        assert!(objects.contains(&("TB_R_F", "WRITE".to_string(), "DELETE FROM")));
        assert!(objects.contains(&("dbo.SP_NEXT", "EXECUTE".to_string(), "EXEC")));
        assert!(!objects.iter().any(|(_, _, source)| *source == "AS"));
    }

    #[test]
    fn does_not_count_delete_target_as_read() {
        let sql = r#"
            DELETE FROM TB_B WHERE NOTE = 'WHERE';
        "#;

        let deps = extract_dependencies(sql);
        assert!(deps
            .iter()
            .any(|dep| dep.object == "TB_B" && dep.operation == Operation::Write));
        assert!(!deps
            .iter()
            .any(|dep| dep.object == "TB_B" && dep.operation == Operation::Read));
    }

    #[test]
    fn detects_delete_join_target_and_sources() {
        let sql = r#"
            DELETE t
            FROM TB_TARGET t
            JOIN TB_SRC s ON s.ID = t.ID
            WHERE s.FLAG = 1
        "#;

        let deps = extract_dependencies(sql);
        assert!(deps
            .iter()
            .any(|dep| dep.object == "TB_TARGET" && dep.operation == Operation::Write));
        assert!(deps
            .iter()
            .any(|dep| dep.object == "TB_SRC" && dep.operation == Operation::Read));
        assert!(!deps
            .iter()
            .any(|dep| dep.object == "TB_TARGET" && dep.operation == Operation::Read));
    }

    #[test]
    fn extracts_temp_tables() {
        let sql = r#"
            CREATE PROCEDURE dbo.SP_TEST
            AS
            BEGIN
                SELECT * INTO #TMP_A FROM TB_A;
                INSERT INTO #TMP_B SELECT * FROM #TMP_A;
            END
        "#;

        let temps = extract_temp_tables(sql);
        assert_eq!(temps, vec!["#TMP_A".to_string(), "#TMP_B".to_string()]);
    }

    #[test]
    fn summarizes_statements_basic() {
        let sql = r#"
            CREATE PROCEDURE dbo.SP_TEST
            AS
            BEGIN
                INSERT INTO TB_OUT SELECT * FROM TB_A;
                UPDATE TB_B SET FLAG = 1 WHERE ID = 1;
                EXEC dbo.SP_NEXT;
            END
        "#;

        let statements = summarize_statements(sql);
        assert_eq!(statements.len(), 3);
        assert!(matches!(statements[0].kind, StatementKind::Insert));
        assert!(matches!(statements[1].kind, StatementKind::Update));
        assert!(matches!(statements[2].kind, StatementKind::Execute));
        assert_eq!(statements[0].target.as_deref(), Some("TB_OUT"));
        assert!(statements[0].sources.contains(&"TB_A".to_string()));
    }
}
