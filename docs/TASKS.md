# SPTrace MVP v0.1 — Task Breakdown

Gunakan file ini sebagai checklist eksekusi. Idealnya satu task = satu prompt ke model murah.

## Status Legend

- [ ] Not started
- [~] In progress
- [x] Done

## Phase 0 — Project Setup

### Task 0.1 — Initialize Rust Project

**Goal:** buat struktur project Rust yang bisa compile.

Steps:

1. Jika belum ada `Cargo.toml`, jalankan/init project Rust.
2. Buat folder `src/`.
3. Buat `src/main.rs` minimal.
4. Tambahkan dependencies di `Cargo.toml`:
   - anyhow
   - clap with derive
   - colored
   - regex
   - serde with derive
   - serde_json
   - walkdir
5. Jalankan `cargo check`.

Acceptance:

- `cargo check` success.
- `cargo run -- --help` bisa jalan setelah CLI task selesai.

Files:

- `Cargo.toml`
- `src/main.rs`

Checklist:

- [ ] Cargo project created
- [ ] Dependencies added
- [ ] `cargo check` passes

## Phase 1 — CLI Skeleton

### Task 1.1 — Implement CLI Structs

**Goal:** command `sptrace scan <path> [--out <file>] [--json]` tersedia.

Steps:

1. Buat `src/cli.rs`.
2. Implement `Cli` dan `Commands` sesuai `docs/IMPLEMENTATION_SPEC.md`.
3. Update `main.rs` untuk parse CLI.
4. Untuk sementara command `scan` boleh print path/out/json.

Acceptance:

```bash
cargo run -- scan examples/procedures/duplicate_aggregation.sql
cargo run -- scan examples/procedures/duplicate_aggregation.sql --out report.md
cargo run -- scan examples/procedures/duplicate_aggregation.sql --json
```

Semua command tidak panic.

Files:

- `src/cli.rs`
- `src/main.rs`

Checklist:

- [ ] CLI structs implemented
- [ ] `scan` command accepted
- [ ] `--out` accepted
- [ ] `--json` accepted

## Phase 2 — Data Model

### Task 2.1 — Implement Models

**Goal:** semua struct/enums untuk trace tersedia dan serializable.

Steps:

1. Buat `src/model.rs`.
2. Tambahkan semua struct/enums dari implementation spec.
3. Add helper if needed:
   - `Severity::rank()` atau sorting helper.

Acceptance:

- `cargo check` success.
- `ProcedureTrace` bisa di-serialize oleh `serde_json`.

Files:

- `src/model.rs`
- `src/main.rs` add `mod model;`

Checklist:

- [ ] ProcedureTrace added
- [ ] Parameter added
- [ ] Dependency added
- [ ] RiskFinding added
- [ ] Severity added
- [ ] StatementSummary added
- [ ] `cargo check` passes

## Phase 3 — Normalizer

### Task 3.1 — Implement SQL Normalizer

**Goal:** basic cleanup SQL sebelum parsing.

Steps:

1. Buat `src/normalizer.rs`.
2. Implement:
   - `strip_comments`
   - `normalize_whitespace`
   - `normalize_identifier`
   - `split_statements`
3. Add unit tests.

Acceptance tests:

- `-- comment` removed.
- `/* block */` removed.
- `[dbo].[Table]` becomes `dbo.Table`.
- `SELECT  1\nFROM X` normalized to one-space style.

Files:

- `src/normalizer.rs`

Checklist:

- [ ] Line comments stripped
- [ ] Block comments stripped
- [ ] Whitespace normalized
- [ ] Brackets removed from identifiers
- [ ] Statements split approximately
- [ ] Unit tests added

## Phase 4 — Parser

### Task 4.1 — Procedure Name Extraction

**Goal:** detect procedure name from CREATE/ALTER PROC/PROCEDURE.

Steps:

1. Buat `src/parser.rs`.
2. Implement `extract_procedure_name`.
3. Use regex from implementation spec.
4. Normalize identifier.
5. Add tests.

Acceptance:

Input:

```sql
CREATE PROCEDURE dbo.SP_TEST AS SELECT 1
```

Output:

```txt
dbo.SP_TEST
```

Also support:

```sql
ALTER PROC [dbo].[SP_TEST]
```

Output:

```txt
dbo.SP_TEST
```

Checklist:

- [ ] CREATE PROCEDURE supported
- [ ] ALTER PROCEDURE supported
- [ ] CREATE PROC supported
- [ ] ALTER PROC supported
- [ ] Bracket identifier normalized

### Task 4.2 — Parameter Extraction

**Goal:** extract parameters between procedure name and AS.

Steps:

1. Implement `extract_parameters` in `parser.rs`.
2. Use regex from spec.
3. Handle default value.
4. Add tests.

Acceptance:

Input:

```sql
CREATE PROCEDURE dbo.SP_TEST
  @ORDER_NO VARCHAR(20),
  @PICKUP_DT DATE = NULL,
  @FLAG INT = 0
AS
BEGIN
END
```

Output:

- `@ORDER_NO`, `VARCHAR(20)`, default None
- `@PICKUP_DT`, `DATE`, default `NULL`
- `@FLAG`, `INT`, default `0`

Checklist:

- [ ] Basic parameters detected
- [ ] Type with parentheses detected
- [ ] Default NULL detected
- [ ] Default number detected
- [ ] No panic if no params

### Task 4.3 — Dependency Extraction

**Goal:** detect READ/WRITE/EXECUTE dependencies.

Steps:

1. Implement `extract_dependencies`.
2. Add read regex for FROM/JOIN.
3. Add write regex for INSERT/UPDATE/DELETE/MERGE.
4. Add execute regex.
5. Deduplicate dependencies.
6. Ignore table variables starting with `@`.
7. Add tests.

Acceptance:

Input:

```sql
INSERT INTO TB_OUT
SELECT *
FROM TB_A a
JOIN TB_B b ON a.ID = b.ID
UPDATE TB_C SET FLAG = 1 WHERE ID = 1
EXEC SP_NEXT
```

Output dependencies:

- `TB_A` READ FROM
- `TB_B` READ JOIN
- `TB_OUT` WRITE INSERT INTO
- `TB_C` WRITE UPDATE
- `SP_NEXT` EXECUTE EXEC

Checklist:

- [ ] FROM read detected
- [ ] JOIN read detected
- [ ] INSERT write detected
- [ ] UPDATE write detected
- [ ] DELETE write detected
- [ ] MERGE write detected
- [ ] EXEC execute detected
- [ ] Dependencies deduped

### Task 4.4 — Temp Table Extraction

**Goal:** detect temp tables.

Steps:

1. Implement `extract_temp_tables`.
2. Regex: `#[a-zA-Z0-9_]+`.
3. Deduplicate and sort.
4. Add tests.

Acceptance:

Input:

```sql
SELECT * INTO #TMP_A FROM TB_A
INSERT INTO #TMP_A SELECT * FROM #TMP_B
```

Output:

- `#TMP_A`
- `#TMP_B`

Checklist:

- [ ] Temp tables detected
- [ ] Deduped
- [ ] Sorted

### Task 4.5 — Statement Summary

**Goal:** rough statement summaries for metrics and future report.

Steps:

1. Implement `summarize_statements`.
2. Use `split_statements`.
3. Classify by first keyword.
4. Add tests.

Acceptance:

- INSERT statement classified Insert.
- UPDATE statement classified Update.
- EXEC statement classified Execute.

Checklist:

- [ ] Select classified
- [ ] Insert classified
- [ ] Update classified
- [ ] Delete classified
- [ ] Merge classified
- [ ] Execute classified
- [ ] Unknown fallback works

## Phase 5 — Risk Rules

### Task 5.1 — Implement Risk Rule Framework

**Goal:** `detect_risks(sql, trace)` returns sorted findings.

Steps:

1. Buat `src/rules.rs`.
2. Implement public `detect_risks`.
3. Add helper for sorting severity.
4. Initially return empty vec.
5. Add tests for empty SQL.

Acceptance:

- Compiles.
- Empty SQL produces empty risk list.

Checklist:

- [ ] rules module created
- [ ] detect_risks public function exists
- [ ] sorting helper exists

### Task 5.2 — Implement Simple Pattern Rules

**Goal:** detect simple global rules.

Rules:

1. `select_star`
2. `nolock_used`
3. `dynamic_sql`
4. `linked_server`
5. `cursor_used`
6. `hardcoded_date`
7. `status_magic_number`

Steps:

1. Implement each rule as function.
2. Add to `detect_risks`.
3. Add unit tests.

Acceptance:

Each fixture triggers expected rule.

Checklist:

- [ ] select_star
- [ ] nolock_used
- [ ] dynamic_sql
- [ ] linked_server
- [ ] cursor_used
- [ ] hardcoded_date
- [ ] status_magic_number

### Task 5.3 — Implement Statement-Based Rules

**Goal:** detect risky UPDATE/DELETE/INSERT SELECT patterns.

Rules:

1. `update_without_where`
2. `delete_without_where`
3. `insert_select_no_distinct`

Steps:

1. Split SQL into statements.
2. For each statement, detect patterns.
3. Add tests.

Acceptance:

- `UPDATE TB SET X=1` triggers high risk.
- `UPDATE TB SET X=1 WHERE ID=1` does not trigger.
- `DELETE FROM TB` triggers high risk.
- `DELETE FROM TB WHERE ID=1` does not trigger.
- `INSERT INTO A SELECT COL FROM B` triggers medium risk if no DISTINCT/GROUP BY.

Checklist:

- [ ] update_without_where
- [ ] delete_without_where
- [ ] insert_select_no_distinct

### Task 5.4 — Implement Aggregation/Transaction/Temp Rules

**Goal:** detect more SP-specific risks.

Rules:

1. `multi_join_aggregation`
2. `transaction_without_trycatch`
3. `trycatch_without_rollback`
4. `temp_table_chain`

Acceptance:

- SUM + JOIN + GROUP BY triggers High.
- BEGIN TRAN without TRY/CATCH triggers Medium.
- TRY/CATCH without ROLLBACK triggers High.
- 3+ temp tables triggers Low.

Checklist:

- [ ] multi_join_aggregation
- [ ] transaction_without_trycatch
- [ ] trycatch_without_rollback
- [ ] temp_table_chain

## Phase 6 — Analyzer

### Task 6.1 — Implement Analyzer Orchestration

**Goal:** one function analyzes SQL and returns full trace.

Steps:

1. Buat `src/analyzer.rs`.
2. Implement `analyze_sql`.
3. Wire normalizer/parser/rules.
4. Compute metrics.
5. Add test with duplicate aggregation fixture.

Acceptance:

For duplicate aggregation SQL:

- name detected;
- parameter detected;
- 3 reads detected;
- 1 write detected;
- high risk detected.

Checklist:

- [ ] analyzer module created
- [ ] analyze_sql implemented
- [ ] metrics computed
- [ ] risks attached

## Phase 7 — Report Output

### Task 7.1 — Terminal Renderer

**Goal:** readable terminal output.

Steps:

1. Buat `src/report.rs`.
2. Implement `render_terminal`.
3. Group dependencies by operation.
4. Print None for empty sections.
5. Wire into `main.rs` for default scan.

Acceptance:

```bash
cargo run -- scan examples/procedures/duplicate_aggregation.sql
```

Prints Procedure, Parameters, Tables Read, Tables Written, Temp Tables, Risks.

Checklist:

- [ ] Terminal report implemented
- [ ] Dependencies grouped
- [ ] Risks printed with severity
- [ ] Default scan uses terminal report

### Task 7.2 — Markdown Renderer

**Goal:** `--out report.md` writes full report.

Steps:

1. Implement `render_markdown`.
2. Include required sections from spec.
3. Implement `render_mermaid`.
4. Wire `--out` in `main.rs`.
5. Create parent directory if needed.

Acceptance:

```bash
cargo run -- scan examples/procedures/duplicate_aggregation.sql --out sptrace-output/report.md
```

- file created;
- contains `# SPTrace Report`;
- contains Mermaid block;
- contains risk finding.

Checklist:

- [ ] Markdown renderer implemented
- [ ] Mermaid renderer implemented
- [ ] `--out` writes file
- [ ] Parent dir created

### Task 7.3 — JSON Output

**Goal:** `--json` prints JSON trace.

Steps:

1. In `main.rs`, if `json` true, print `serde_json::to_string_pretty(&trace)`.
2. If `--out` and `--json` both provided, write JSON to file or choose behavior.
3. For v0.1, define behavior: `--json` controls stdout format; `--out` still writes Markdown unless future changed.

Recommended v0.1 behavior:

- `--json` without `--out`: print JSON to stdout.
- `--json` with `--out`: write Markdown to `--out`, print JSON to stdout.

Acceptance:

```bash
cargo run -- scan examples/procedures/duplicate_aggregation.sql --json
```

Valid JSON printed.

Checklist:

- [ ] JSON output implemented
- [ ] Pretty JSON
- [ ] No terminal report when `--json` true

## Phase 8 — Examples and README

### Task 8.1 — Add Example SQL Files

**Goal:** demo files exist and are safe.

Create:

- `examples/procedures/duplicate_aggregation.sql`
- `examples/procedures/linked_server.sql`
- `examples/procedures/update_without_where.sql`
- `examples/procedures/dynamic_sql.sql`
- `examples/procedures/select_star_nolock.sql`

Use SQL from implementation spec.

Acceptance:

Each file can be scanned without error.

Checklist:

- [ ] duplicate_aggregation.sql
- [ ] linked_server.sql
- [ ] update_without_where.sql
- [ ] dynamic_sql.sql
- [ ] select_star_nolock.sql

### Task 8.2 — Add README

**Goal:** GitHub-ready README.

Sections:

1. Title and tagline.
2. What is SPTrace?
3. Why?
4. Install from source.
5. Usage.
6. Example input/output.
7. Risk rules.
8. Security/privacy.
9. Limitations.
10. Roadmap.

Acceptance:

README explains:

- offline;
- no DB connection;
- no credentials;
- no AI required;
- CLI examples.

Checklist:

- [ ] README created
- [ ] Install section
- [ ] Usage section
- [ ] Demo section
- [ ] Limitations section

## Phase 9 — Final Validation

### Task 9.1 — Run Full Validation

Commands:

```bash
cargo fmt
cargo check
cargo test
cargo run -- scan examples/procedures/duplicate_aggregation.sql
cargo run -- scan examples/procedures/linked_server.sql
cargo run -- scan examples/procedures/update_without_where.sql
cargo run -- scan examples/procedures/dynamic_sql.sql
cargo run -- scan examples/procedures/select_star_nolock.sql
cargo run -- scan examples/procedures/duplicate_aggregation.sql --out sptrace-output/report.md
cargo run -- scan examples/procedures/duplicate_aggregation.sql --json
```

Acceptance:

- all commands pass;
- output is readable;
- risks match fixtures;
- no network/database/API feature.

Checklist:

- [ ] cargo fmt
- [ ] cargo check
- [ ] cargo test
- [ ] all examples scan
- [ ] markdown generated
- [ ] json generated

## Phase 10 — Optional Polish After MVP Works

Do only after v0.1 works.

- [ ] Add colored severity in terminal.
- [x] Add `--diagram mermaid` to write `.mmd` separately.
- [x] Add folder scan.
- [x] Add `sptrace context`.
- [ ] Add config file.
- [ ] Add GitHub Actions CI.
- [ ] Add release binaries.

## Notes for Cheap Model Execution

Bad prompt:

> Build SPTrace fully.

Good prompt:

> Implement Task 4.2 only. Read `docs/IMPLEMENTATION_SPEC.md`. Modify only `src/parser.rs`. Add tests for parameter extraction. Run `cargo test`. Do not change CLI or report output.

This prevents the model from overbuilding or breaking unrelated files.
