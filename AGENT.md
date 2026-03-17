# Lodge

A structured data tool for AI agents. CLI-driven, schema-defined, SQLite-backed.

## Build & Test

```bash
cargo build              # dev build
cargo build --release    # release build
cargo test               # run all tests
cargo clippy             # lint
cargo fmt --check        # format check
```

## Architecture

Single-binary Rust CLI. No server, no HTTP. The agent creates collections via the CLI, and Lodge dynamically generates subcommands from the SQLite schema at runtime.

### Source layout

- `src/main.rs` — entrypoint, top-level dispatch across static and dynamic subcommands
- `src/cli.rs` — runtime `clap::Command` building with dynamic collection subcommands (leaks strings for `'static` lifetime)
- `src/db.rs` — database init/open, `.lodge/` directory discovery (walks up from cwd)
- `src/collection.rs` — create/alter collections (DDL operations)
- `src/record.rs` — add/query/update/delete rows + raw SQL execution
- `src/schema.rs` — load collection metadata from `_lodge_meta` table
- `src/types.rs` — `FieldType` enum, field parsing, validation
- `src/output.rs` — JSON/table/CSV formatting (`--format` flag)
- `src/log.rs` — mutation log queries (`_lodge_log` table)
- `src/error.rs` — `LodgeError` enum with `thiserror`

### Key design decisions

- **`_lodge_meta` table** stores collection schemas (collection, field_name, field_type, field_order). This is the source of truth — the CLI is generated from it at runtime.
- **Auto-managed columns**: every collection gets `id` (autoincrement PK), `created_at`, `updated_at`. Users cannot define fields with these names.
- **Reserved names**: `init`, `create`, `alter`, `sql`, `help`, `log` cannot be used as collection names.
- **Type system**: text, int, real, bool, date, datetime. Bool stores as INTEGER (0/1). Date/datetime store as TEXT in ISO format.
- **`_lodge_log` table** records all mutations (add/update/delete) with before/after data, success/failure status, and error messages. Logging happens inside `record.rs` to guarantee coverage for all call sites.
- **Field validation** happens at the type layer before insert — dates are parsed with chrono, ints/reals are parsed, bools normalize to 0/1.

### Data flow

1. `lodge init` creates `.lodge/lodge.db` with the `_lodge_meta` table
2. `lodge create <name> --fields "..."` parses the spec, creates the table + meta rows
3. Runtime: load collections from `_lodge_meta`, build clap subcommands dynamically
4. `lodge <collection> add/query/update/delete` operates on the collection table
5. `lodge alter <name> --add-fields "..."` adds columns to existing collections
6. `lodge sql "<query>"` executes raw SQL directly
7. `--format json|table|csv` on query/sql commands controls output format (default: json)
8. `--where`, `--sort`, `--limit` on query commands filter/order results (raw SQL passthrough)

## Dependencies

- `clap` 4 — CLI framework
- `rusqlite` 0.31 (bundled) — SQLite
- `chrono` 0.4 — date/time parsing
- `thiserror` 1 — error derive
- `serde_json` 1 — JSON output

Dev: `tempfile`, `assert_cmd`, `predicates`

## Tests

32 integration tests across 8 test files in `tests/`. All tests use `assert_cmd` to run the compiled binary as a subprocess with a real SQLite DB in a temp directory (no mocks).

- `test_init.rs` (3) — init creates DB, double-init errors, meta table exists
- `test_create.rs` (6) — create succeeds, correct columns, duplicates, invalid types, reserved names, all field types
- `test_add.rs` (6) — returns JSON with id, validates int/date, optional fields, incrementing ids
- `test_query.rs` (7) — all rows, `--where`, `--sort`, `--limit`, empty result, table/csv formats
- `test_update.rs` (3) — changes field, nonexistent id, validates types
- `test_delete.rs` (2) — removes record, nonexistent id
- `test_alter.rs` (4) — adds field, existing data gets null, add+query with new field, nonexistent collection
- `test_sql.rs` (3) — raw SELECT, bad SQL errors, format flag
- `test_help.rs` (4) — static commands, collections appear in help, subcommands, field flags

## Conventions

- Keep the CLI output optimized for LLM consumption (structured, parseable, no decoration).
- `lodge help` must be dynamically generated from the schema — never hardcode collection info.
- Prefer clear error messages that tell the agent what to do next (e.g., "Run `lodge init` first").
- No unwrap in library code. Use the `Result<T>` alias from `error.rs`.
- Field names and collection names must be valid identifiers (alphanumeric + underscore, no leading digit).
