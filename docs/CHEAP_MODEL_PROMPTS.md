# SPTrace — Copy/Paste Prompts for Cheap Model Execution

Gunakan prompt ini satu per satu. Jangan kirim semua sekaligus ke model murah.

## Global Instruction to Reuse

Tambahkan instruksi ini di awal setiap prompt jika model murah sering ngaco:

```txt
You are implementing SPTrace MVP v0.1. Follow docs/IMPLEMENTATION_SPEC.md exactly. Do not add AI, database connection, network calls, web UI, desktop UI, or advanced SQL parser. Keep changes minimal and scoped to the requested task. After editing, run the specified cargo command and report the result.
```

## Prompt 0.1 — Initialize Rust Project

```txt
You are implementing SPTrace MVP v0.1. Follow docs/IMPLEMENTATION_SPEC.md exactly.

Task: Initialize the Rust project.

Requirements:
1. Create Cargo.toml if it does not exist.
2. Add dependencies:
   - anyhow = "1"
   - clap = { version = "4", features = ["derive"] }
   - colored = "2"
   - regex = "1"
   - serde = { version = "1", features = ["derive"] }
   - serde_json = "1"
   - walkdir = "2"
3. Create src/main.rs with a minimal main function if missing.
4. Run cargo check.

Do not implement parser/rules/report yet.
```

## Prompt 1.1 — Implement CLI

```txt
You are implementing SPTrace MVP v0.1. Follow docs/IMPLEMENTATION_SPEC.md exactly.

Task: Implement CLI structs only.

Files to modify:
- src/cli.rs
- src/main.rs

Requirements:
1. Add clap-based CLI with command:
   sptrace scan <path> [--out <file>] [--json]
2. In main.rs, parse CLI and for now print the parsed values for the Scan command.
3. Do not implement analyzer yet.
4. Run:
   cargo run -- scan examples/procedures/duplicate_aggregation.sql --json

Acceptance:
- Command is accepted.
- Program does not panic.
```

## Prompt 2.1 — Implement Data Models

```txt
You are implementing SPTrace MVP v0.1. Follow docs/IMPLEMENTATION_SPEC.md exactly.

Task: Implement src/model.rs.

Requirements:
1. Add these structs/enums with serde::Serialize:
   - ProcedureTrace
   - TraceMetrics
   - Parameter
   - Dependency
   - Operation
   - RiskFinding
   - Severity
   - StatementSummary
   - StatementKind
2. Match the fields defined in docs/IMPLEMENTATION_SPEC.md.
3. Derive Debug, Serialize, Clone where appropriate.
4. Operation must derive PartialEq, Eq, Hash.
5. Add mod model in main.rs.
6. Run cargo check.

Do not implement parser/rules/report yet.
```

## Prompt 3.1 — Implement Normalizer

```txt
You are implementing SPTrace MVP v0.1. Follow docs/IMPLEMENTATION_SPEC.md exactly.

Task: Implement SQL normalizer.

Files to modify:
- src/normalizer.rs
- src/main.rs if module registration is needed

Functions:
- strip_comments(sql: &str) -> String
- normalize_whitespace(sql: &str) -> String
- normalize_identifier(identifier: &str) -> String
- split_statements(sql: &str) -> Vec<String>

Requirements:
1. Remove -- line comments.
2. Remove /* block comments */.
3. Normalize repeated whitespace to one space.
4. normalize_identifier removes [ and ], trims commas/semicolons.
5. split_statements can split by semicolon and GO lines approximately.
6. Add unit tests for each function.
7. Run cargo test.

Do not implement parser/rules/report yet.
```

## Prompt 4.1 — Extract Procedure Name

```txt
You are implementing SPTrace MVP v0.1. Follow docs/IMPLEMENTATION_SPEC.md exactly.

Task: Implement procedure name extraction.

Files to modify:
- src/parser.rs
- src/main.rs if module registration is needed

Requirements:
1. Implement extract_procedure_name(sql: &str) -> Option<String>.
2. Support CREATE PROCEDURE, ALTER PROCEDURE, CREATE PROC, ALTER PROC.
3. Support bracket identifiers like [dbo].[SP_TEST], normalized to dbo.SP_TEST.
4. Add unit tests.
5. Run cargo test.

Do not implement dependency extraction yet.
```

## Prompt 4.2 — Extract Parameters

```txt
You are implementing SPTrace MVP v0.1. Follow docs/IMPLEMENTATION_SPEC.md exactly.

Task: Implement parameter extraction.

Files to modify:
- src/parser.rs

Requirements:
1. Implement extract_parameters(sql: &str) -> Vec<Parameter>.
2. Extract parameters between procedure declaration and AS keyword.
3. Support types like DATE, INT, VARCHAR(20), NVARCHAR(MAX), DECIMAL(18,2).
4. Support default values like = NULL and = 0.
5. Add unit tests using this SQL:

CREATE PROCEDURE dbo.SP_TEST
  @ORDER_NO VARCHAR(20),
  @PICKUP_DT DATE = NULL,
  @FLAG INT = 0
AS
BEGIN
END

Expected:
- @ORDER_NO VARCHAR(20) default None
- @PICKUP_DT DATE default NULL
- @FLAG INT default 0

Run cargo test.
```

## Prompt 4.3 — Extract Dependencies

```txt
You are implementing SPTrace MVP v0.1. Follow docs/IMPLEMENTATION_SPEC.md exactly.

Task: Implement dependency extraction.

Files to modify:
- src/parser.rs

Requirements:
1. Implement extract_dependencies(sql: &str) -> Vec<Dependency>.
2. Detect READ from FROM and JOIN.
3. Detect WRITE from INSERT INTO, UPDATE, DELETE FROM, MERGE INTO.
4. Detect EXECUTE from EXEC and EXECUTE, except EXEC(@SQL).
5. Deduplicate by object + operation + source.
6. Normalize bracket identifiers.
7. Ignore dependencies starting with @.
8. Add unit tests.
9. Run cargo test.

Expected example:
SQL:
INSERT INTO TB_OUT
SELECT * FROM TB_A a
JOIN TB_B b ON a.ID = b.ID
UPDATE TB_C SET FLAG = 1 WHERE ID = 1
EXEC SP_NEXT

Dependencies:
- TB_OUT WRITE INSERT INTO
- TB_A READ FROM
- TB_B READ JOIN
- TB_C WRITE UPDATE
- SP_NEXT EXECUTE EXEC
```

## Prompt 4.4 — Extract Temp Tables

```txt
You are implementing SPTrace MVP v0.1. Follow docs/IMPLEMENTATION_SPEC.md exactly.

Task: Implement temp table extraction.

Files to modify:
- src/parser.rs

Requirements:
1. Implement extract_temp_tables(sql: &str) -> Vec<String>.
2. Detect names like #TMP_A and #TMP_ORDER_PART.
3. Deduplicate and sort alphabetically.
4. Add unit tests.
5. Run cargo test.
```

## Prompt 4.5 — Statement Summary

```txt
You are implementing SPTrace MVP v0.1. Follow docs/IMPLEMENTATION_SPEC.md exactly.

Task: Implement statement summaries.

Files to modify:
- src/parser.rs

Requirements:
1. Implement summarize_statements(sql: &str) -> Vec<StatementSummary>.
2. Use normalizer::split_statements.
3. Classify Select, Insert, Update, Delete, Merge, Execute, Transaction, Unknown.
4. Add basic tests.
5. Run cargo test.
```

## Prompt 5.1 — Risk Rule Framework

```txt
You are implementing SPTrace MVP v0.1. Follow docs/IMPLEMENTATION_SPEC.md exactly.

Task: Create risk rule framework.

Files to modify:
- src/rules.rs
- src/main.rs if module registration is needed

Requirements:
1. Implement public function:
   detect_risks(sql: &str, trace: &ProcedureTrace) -> Vec<RiskFinding>
2. For now, it can call internal rule functions if already created, or return empty Vec.
3. Add sorting helper so future findings are sorted High, Medium, Low.
4. Add a unit test that empty SQL returns no findings.
5. Run cargo test.
```

## Prompt 5.2 — Simple Risk Rules

```txt
You are implementing SPTrace MVP v0.1. Follow docs/IMPLEMENTATION_SPEC.md exactly.

Task: Implement simple pattern risk rules.

Files to modify:
- src/rules.rs

Rules to implement:
1. select_star - Low
2. nolock_used - Medium
3. dynamic_sql - Medium
4. linked_server - Medium
5. cursor_used - Medium
6. hardcoded_date - Low
7. status_magic_number - Low

Use exact messages and suggestions from docs/IMPLEMENTATION_SPEC.md.
Add unit tests for each rule.
Run cargo test.
```

## Prompt 5.3 — Statement-Based Risk Rules

```txt
You are implementing SPTrace MVP v0.1. Follow docs/IMPLEMENTATION_SPEC.md exactly.

Task: Implement statement-based risk rules.

Files to modify:
- src/rules.rs

Rules:
1. update_without_where - High
2. delete_without_where - High
3. insert_select_no_distinct - Medium

Requirements:
1. Use normalizer::split_statements.
2. UPDATE statement with SET but no WHERE triggers.
3. DELETE statement with no WHERE triggers.
4. INSERT INTO ... SELECT without DISTINCT or GROUP BY triggers.
5. Add unit tests for positive and negative cases.
6. Run cargo test.
```

## Prompt 5.4 — Aggregation and Transaction Risk Rules

```txt
You are implementing SPTrace MVP v0.1. Follow docs/IMPLEMENTATION_SPEC.md exactly.

Task: Implement aggregation, transaction, and temp table rules.

Files to modify:
- src/rules.rs

Rules:
1. multi_join_aggregation - High
2. transaction_without_trycatch - Medium
3. trycatch_without_rollback - High
4. temp_table_chain - Low

Requirements:
1. SUM or COUNT + JOIN + GROUP BY triggers multi_join_aggregation.
2. BEGIN TRAN without BEGIN TRY and BEGIN CATCH triggers transaction_without_trycatch.
3. BEGIN TRY and BEGIN CATCH without ROLLBACK triggers trycatch_without_rollback.
4. trace.temp_tables.len() >= 3 triggers temp_table_chain.
5. Add unit tests.
6. Run cargo test.
```

## Prompt 6.1 — Analyzer Orchestration

```txt
You are implementing SPTrace MVP v0.1. Follow docs/IMPLEMENTATION_SPEC.md exactly.

Task: Implement analyzer orchestration.

Files to modify:
- src/analyzer.rs
- src/main.rs if module registration is needed

Requirements:
1. Implement analyze_sql(sql: &str) -> anyhow::Result<ProcedureTrace>.
2. Use normalizer, parser, and rules modules.
3. Compute metrics:
   - statement_count
   - read_count
   - write_count
   - risk_level
4. Add test using duplicate aggregation SQL.
5. Run cargo test.

Do not implement report rendering in this task.
```

## Prompt 7.1 — Terminal Report

```txt
You are implementing SPTrace MVP v0.1. Follow docs/IMPLEMENTATION_SPEC.md exactly.

Task: Implement terminal report and wire default scan.

Files to modify:
- src/report.rs
- src/main.rs

Requirements:
1. Implement render_terminal(trace: &ProcedureTrace) -> String.
2. Group dependencies into Tables Read, Tables Written, Procedures Executed.
3. Print Procedure, Parameters, Tables Read, Tables Written, Temp Tables, Risks.
4. Print '- None' for empty sections.
5. In main.rs, read the input file, call analyze_sql, print terminal report.
6. Handle missing file with clear error via anyhow.
7. Run:
   cargo run -- scan examples/procedures/duplicate_aggregation.sql

Do not implement Markdown or JSON yet.
```

## Prompt 7.2 — Markdown Report and Mermaid

```txt
You are implementing SPTrace MVP v0.1. Follow docs/IMPLEMENTATION_SPEC.md exactly.

Task: Implement Markdown report and Mermaid rendering.

Files to modify:
- src/report.rs
- src/main.rs

Requirements:
1. Implement render_markdown(trace: &ProcedureTrace) -> String.
2. Implement render_mermaid(trace: &ProcedureTrace) -> String.
3. Markdown must contain sections:
   - Overview
   - Parameters
   - Dependencies
   - Temp Tables
   - Dependency Diagram
   - Execution Flow
   - Risk Findings
   - Questions to Verify
   - Suggested Next Queries
4. Wire --out so it writes Markdown to the provided path.
5. Create parent directories if needed.
6. Run:
   cargo run -- scan examples/procedures/duplicate_aggregation.sql --out sptrace-output/report.md

Acceptance:
- File is created.
- File contains '# SPTrace Report'.
- File contains '```mermaid'.
```

## Prompt 7.3 — JSON Output

```txt
You are implementing SPTrace MVP v0.1. Follow docs/IMPLEMENTATION_SPEC.md exactly.

Task: Implement JSON output.

Files to modify:
- src/main.rs

Requirements:
1. If --json is provided, print serde_json::to_string_pretty(&trace) to stdout.
2. If --json is not provided, print terminal report.
3. If --out is provided, still write Markdown report.
4. Run:
   cargo run -- scan examples/procedures/duplicate_aggregation.sql --json

Acceptance:
- Output is valid pretty JSON.
- No terminal report appears when --json is used.
```

## Prompt 8.1 — Add Example SQL Files

```txt
You are implementing SPTrace MVP v0.1. Follow docs/IMPLEMENTATION_SPEC.md exactly.

Task: Add safe example SQL files.

Files to create:
- examples/procedures/duplicate_aggregation.sql
- examples/procedures/linked_server.sql
- examples/procedures/update_without_where.sql
- examples/procedures/dynamic_sql.sql
- examples/procedures/select_star_nolock.sql

Use the exact SQL examples from docs/IMPLEMENTATION_SPEC.md section 7.

After creating files, run:
- cargo run -- scan examples/procedures/duplicate_aggregation.sql
- cargo run -- scan examples/procedures/linked_server.sql
- cargo run -- scan examples/procedures/update_without_where.sql
- cargo run -- scan examples/procedures/dynamic_sql.sql
- cargo run -- scan examples/procedures/select_star_nolock.sql
```

## Prompt 8.2 — Add README

```txt
You are implementing SPTrace MVP v0.1. Follow docs/IMPLEMENTATION_SPEC.md and PRD.md.

Task: Create README.md.

README sections:
1. # SPTrace
2. Tagline: Understand legacy Stored Procedures without reading 1,000 lines of SQL.
3. What is SPTrace?
4. Why?
5. Install from source.
6. Usage examples.
7. Example input/output.
8. Risk rules table.
9. Security and privacy:
   - No database connection
   - No credentials
   - No production access
   - No AI required
   - Offline static analysis only
10. Limitations.
11. Roadmap.

Keep README concise but GitHub-ready.
```

## Prompt 9.1 — Final Validation

```txt
You are implementing SPTrace MVP v0.1. Follow docs/IMPLEMENTATION_SPEC.md exactly.

Task: Final validation and bug fixing only.

Run:
1. cargo fmt
2. cargo check
3. cargo test
4. cargo run -- scan examples/procedures/duplicate_aggregation.sql
5. cargo run -- scan examples/procedures/linked_server.sql
6. cargo run -- scan examples/procedures/update_without_where.sql
7. cargo run -- scan examples/procedures/dynamic_sql.sql
8. cargo run -- scan examples/procedures/select_star_nolock.sql
9. cargo run -- scan examples/procedures/duplicate_aggregation.sql --out sptrace-output/report.md
10. cargo run -- scan examples/procedures/duplicate_aggregation.sql --json

If a command fails, make the smallest fix needed. Do not add new features.

Report final status with:
- commands passed
- files changed
- known limitations
```

## Prompt for Code Review by Cheap Model

```txt
Review the current SPTrace MVP implementation against docs/IMPLEMENTATION_SPEC.md.

Focus only on correctness issues:
1. Does CLI match spec?
2. Does parser extract expected metadata?
3. Do risk rules match spec?
4. Does report contain required sections?
5. Are there any network/database/API/AI calls? These are forbidden.
6. Are there panics on empty or invalid SQL?

Output concise findings with file path, issue, and suggested fix. Do not rewrite code unless asked.
```

## Prompt for Fixing One Bug

```txt
Fix only this bug:

<describe bug here>

Constraints:
- Follow docs/IMPLEMENTATION_SPEC.md.
- Keep change minimal.
- Do not refactor unrelated code.
- Add or update a test that fails before the fix and passes after.
- Run cargo test.
```
