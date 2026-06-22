pub fn strip_comments(sql: &str) -> String {
    let mut out = String::with_capacity(sql.len());
    let chars: Vec<char> = sql.chars().collect();
    let mut i = 0;
    let mut in_single = false;
    let mut in_double = false;
    let mut in_bracket = false;

    while i < chars.len() {
        let c = chars[i];

        if in_single {
            out.push(c);
            if c == '\'' {
                if i + 1 < chars.len() && chars[i + 1] == '\'' {
                    out.push(chars[i + 1]);
                    i += 2;
                    continue;
                }
                in_single = false;
            }
            i += 1;
            continue;
        }

        if in_double {
            out.push(c);
            if c == '"' {
                if i + 1 < chars.len() && chars[i + 1] == '"' {
                    out.push(chars[i + 1]);
                    i += 2;
                    continue;
                }
                in_double = false;
            }
            i += 1;
            continue;
        }

        if in_bracket {
            out.push(c);
            if c == ']' {
                if i + 1 < chars.len() && chars[i + 1] == ']' {
                    out.push(chars[i + 1]);
                    i += 2;
                    continue;
                }
                in_bracket = false;
            }
            i += 1;
            continue;
        }

        if c == '\'' {
            in_single = true;
            out.push(c);
            i += 1;
            continue;
        }

        if c == '"' {
            in_double = true;
            out.push(c);
            i += 1;
            continue;
        }

        if c == '[' {
            in_bracket = true;
            out.push(c);
            i += 1;
            continue;
        }

        if c == '-' && i + 1 < chars.len() && chars[i + 1] == '-' {
            i += 2;
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            if i < chars.len() && chars[i] == '\n' {
                out.push('\n');
                i += 1;
            }
            continue;
        }

        if c == '/' && i + 1 < chars.len() && chars[i + 1] == '*' {
            i += 2;
            while i + 1 < chars.len() {
                if chars[i] == '*' && chars[i + 1] == '/' {
                    i += 2;
                    break;
                }
                if chars[i] == '\n' {
                    out.push('\n');
                }
                i += 1;
            }
            continue;
        }

        out.push(c);
        i += 1;
    }

    out
}

pub fn normalize_whitespace(sql: &str) -> String {
    sql.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn normalize_identifier(identifier: &str) -> String {
    identifier
        .trim()
        .trim_end_matches(|c| matches!(c, ';' | ','))
        .split('.')
        .map(|part| {
            part.trim()
                .trim_matches(|c| matches!(c, '[' | ']' | '"' | '\'' | '`'))
                .trim_end_matches(|c| matches!(c, ';' | ',' | ')' | '('))
                .to_string()
        })
        .collect::<Vec<_>>()
        .join(".")
}

pub fn split_statements(sql: &str) -> Vec<String> {
    let stripped = strip_comments(sql);
    let mut statements = Vec::new();
    let mut batch = String::new();
    let mut in_single = false;

    for line in stripped.lines() {
        let trimmed = line.trim();
        if !in_single && trimmed.eq_ignore_ascii_case("GO") {
            statements.extend(split_by_semicolon(&batch));
            batch.clear();
            continue;
        }

        batch.push_str(line);
        batch.push('\n');
        in_single = update_single_quote_state(in_single, line);
    }

    statements.extend(split_by_semicolon(&batch));
    statements
        .into_iter()
        .map(|stmt| stmt.trim().to_string())
        .filter(|stmt| !stmt.is_empty())
        .collect()
}

fn update_single_quote_state(mut in_single: bool, line: &str) -> bool {
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\'' {
            if in_single {
                if matches!(chars.peek(), Some('\'')) {
                    chars.next();
                } else {
                    in_single = false;
                }
            } else {
                in_single = true;
            }
        }
    }
    in_single
}

fn split_by_semicolon(batch: &str) -> Vec<String> {
    let mut statements = Vec::new();
    let mut current = String::new();
    let mut in_single = false;
    let mut chars = batch.chars().peekable();

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

        if c == ';' && !in_single {
            if !current.trim().is_empty() {
                statements.push(current.trim().to_string());
            }
            current.clear();
            continue;
        }

        current.push(c);
    }

    if !current.trim().is_empty() {
        statements.push(current.trim().to_string());
    }

    statements
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_line_and_block_comments() {
        let sql = r#"
            SELECT 1 -- comment
            /* block
            comment */
            FROM [dbo].[Table]
        "#;

        let stripped = strip_comments(sql);
        assert!(!stripped.contains("comment"));
        assert!(stripped.contains("SELECT 1"));
        assert!(stripped.contains("FROM [dbo].[Table]"));
    }

    #[test]
    fn normalizes_identifier_brackets() {
        assert_eq!(normalize_identifier("[dbo].[Table]"), "dbo.Table");
        assert_eq!(normalize_identifier("[TB_T_GR_HUB]"), "TB_T_GR_HUB");
    }

    #[test]
    fn splits_statements_on_semicolon_and_go() {
        let sql = "SELECT 1;\nGO\nSELECT 2;";
        let statements = split_statements(sql);
        assert_eq!(statements, vec!["SELECT 1", "SELECT 2"]);
    }
}
