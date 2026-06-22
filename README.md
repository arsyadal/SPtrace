# SPTrace

Understand legacy Stored Procedures without reading 1,000 lines of SQL.

SPTrace is an offline CLI static analyzer for SQL Stored Procedures. It reads `.sql` files, extracts dependencies, detects risky patterns, and generates readable reports for debugging, onboarding, and RCA.

## Why

In many enterprise systems, business logic lives inside Stored Procedures. Documentation is often missing, and production debugging usually means reading long SQL scripts manually.

SPTrace helps you answer:

- What tables does this procedure read?
- What tables does it write?
- Does it use temp tables or linked servers?
- Is there risky aggregation logic?
- Are there UPDATE/DELETE statements without WHERE?

## Features

- Offline static analysis
- Procedure name extraction
- Parameter extraction
- Read/write dependency detection
- Temp table detection
- Mermaid dependency diagram
- Markdown report generation
- JSON output
- Context mode
- Diff mode
- Folder scan + dependency index
- Risk detection rules

## Safety

- No database connection
- No credentials
- No production access
- No AI required
- No network calls

## Install

```bash
cargo build
```

## Usage

```bash
cargo run -- scan examples/procedures/duplicate_aggregation.sql
cargo run -- scan examples/procedures/duplicate_aggregation.sql --diagram
cargo run -- scan examples/procedures --out sptrace-output
cargo run -- scan examples/procedures/duplicate_aggregation.sql --out sptrace-output/report.md
cargo run -- scan examples/procedures/duplicate_aggregation.sql --json
cargo run -- context examples/procedures/duplicate_aggregation.sql
cargo run -- context examples/procedures/duplicate_aggregation.sql --json
cargo run -- diff examples/procedures/duplicate_aggregation.sql examples/procedures/update_without_where.sql
cargo run -- diff examples/procedures/duplicate_aggregation.sql examples/procedures/update_without_where.sql --json
```

## Example

Input:

```sql
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
```

Output includes:

- `TB_R_DAILY_ORDER_PART` read
- `TB_R_DELIVERY_CTL_H` read
- `TB_R_DELIVERY_CTL_D` read
- `TB_T_GR_HUB` write
- `multi_join_aggregation` high risk

## Risk Rules

Implemented in v0.1:

- `select_star`
- `nolock_used`
- `dynamic_sql`
- `linked_server`
- `update_without_where`
- `delete_without_where`
- `insert_select_no_distinct`
- `multi_join_aggregation`
- `cursor_used`
- `transaction_without_trycatch`
- `trycatch_without_rollback`
- `hardcoded_date`
- `status_magic_number`
- `temp_table_chain`

## Limitations

- Regex/token-based analysis only
- Dynamic SQL dependency detection is incomplete
- T-SQL edge cases may produce false positives
- No database validation
- Optimized for stored procedures first

## Roadmap

- Folder scan with dependency index
- AI context mode
- Diff mode
- VS Code extension
