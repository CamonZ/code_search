# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Test Commands

```bash
cargo build                      # Build entire workspace
cargo build -p cli               # Build CLI binary only
cargo build -p db                # Build database library only
cargo test                       # Run all tests in workspace
cargo test -p db                 # Test database layer only
cargo test -p code_search        # Test CLI layer only
cargo test <test_name>           # Run a single test by name
cargo nextest run                # Alternative test runner (faster)
cargo run -p code_search -- --help      # Show CLI help
cargo run -p code_search -- describe    # Show detailed command documentation
```

## Workspace Structure

This is a Cargo workspace with two crates:

- **`db/`** - Database library crate
  - CozoDB query layer (all `queries/` modules)
  - Database utilities (`db.rs`)
  - Shared types (`types/`)
  - Query builders (`query_builders.rs`)
  - Test utilities and fixtures (behind `test-utils` feature flag)

- **`cli/`** - CLI binary crate (package name: `code_search`)
  - Command-line interface (`cli.rs`, `main.rs`)
  - All command modules (`commands/`)
  - Output formatting (`output.rs`)
  - Presentation utilities (`utils.rs`, `dedup.rs`)
  - Test macros (`test_macros.rs`)

**Dependency flow:** `cli` depends on `db` via `db = { path = "../db" }`. The database layer is completely independent of the CLI.

**Test utilities:** Database test helpers and fixtures are available via the `test-utils` feature. CLI tests use: `db = { path = "../db", features = ["test-utils"] }`

## Architecture

This is a Rust CLI tool for querying call graph data stored in a CozoDB SQLite database. Uses Rust 2024 edition with clap derive macros for CLI parsing.

**Code organization:**

*Database crate (`db/src/`):*
- `lib.rs` - Public API surface, re-exports
- `db.rs` - Database connection and query utilities
- `queries/<name>.rs` - CozoScript queries and result parsing (31 query modules)
- `query_builders.rs` - SQL condition builders (`ConditionBuilder`, `OptionalConditionBuilder`)
- `types/` - Shared types (`ModuleGroupResult`, `ModuleGroup`, `Call`, `FunctionRef`, etc.)
- `fixtures/` - Test data (feature-gated)
- `test_utils.rs` - Test helpers (feature-gated)

*CLI crate (`cli/src/`):*
- `main.rs` - Entry point, module declarations
- `cli.rs` - Top-level CLI structure with global `--db` and `--format` flags
- `commands/mod.rs` - `Command` enum, `Execute` trait, `CommonArgs`, dispatch via enum_dispatch
- `commands/<name>/` - Individual command modules (27 commands, directory structure)
- `output.rs` - `OutputFormat` enum, `Outputable` and `TableFormatter` traits
- `dedup.rs` - Deduplication utilities (`sort_and_deduplicate`, `deduplicate_retain`)
- `utils.rs` - Presentation helpers (`group_by_module`, `convert_to_module_groups`, `format_type_definition`)
- `test_macros.rs` - Declarative test macros for CLI, execute, and output tests

**Command module structure:**
Each command is a directory module with these files:
- `mod.rs` - Command struct with clap attributes, re-exports
- `execute.rs` - `Execute` trait implementation, result types, tests
- `output.rs` - `Outputable` implementation for the command's result type
- `models.rs` - (optional) Data models for deserialization

**Execute trait:**
```rust
// Defined in cli/src/commands/mod.rs
pub trait Execute {
    type Output: Outputable;
    fn execute(self, db: &db::DbInstance) -> Result<Self::Output, Box<dyn Error>>;
}
```

**Output formatting:**
- Commands return a result type implementing `Outputable`
- Three formats available: `Table` (default), `Json`, `Toon`
- `Outputable` requires `Serialize` + `to_table()` method
- JSON and Toon formats are derived automatically from `Serialize`

**Output format details:**

*Table format* - Human-readable, optimized for terminal display. Hand-crafted in each command's `to_table()` method to show the most relevant information clearly.

*JSON format* - Standard JSON via `serde_json::to_string_pretty()`. Uses the struct's `#[derive(Serialize)]` implementation. Nested structures serialize as nested objects/arrays. Use `#[serde(skip_serializing_if = "...")]` to omit empty collections.

*Toon format* - Token-efficient serialization via the `toon` crate. Automatically derived from the same Serialize implementation as JSON. Key design principles:
- Designed for LLM consumption (minimal tokens while preserving structure)
- Arrays show count in brackets: `callers[2]:` means 2 items follow
- Objects omit braces, use indentation for nesting
- Inline notation for simple objects: `targets[1]{arity,function,line}: 2,get,15`
- Empty collections still show: `modules[0]:` indicates empty array
- Whitespace-sensitive (indentation conveys hierarchy)

When refactoring output, ensure all three formats remain consistent:
1. The struct hierarchy should make sense for both JSON and toon
2. Test fixtures exist in `db/src/fixtures/output/<command>/` for JSON and toon
3. Output tests verify round-trip consistency between formats

**Dispatch flow:**
```
main.rs → Args::parse() → Command::run(db_path, format) → cmd.execute() → result.format()
```

**Key traits:**
- `Execute` - Core execution trait: `execute(self, db) -> Result<Self::Output>`
- `CommandRunner` - Auto-generated via `enum_dispatch` macro for all Command variants
- `Outputable` - Output formatting: `to_table()` + automatic `format(OutputFormat)`
- `TableFormatter` - Customizable table layout for module-grouped results

**CommonArgs pattern:**
Commands share common arguments via `#[command(flatten)]`:
```rust
pub struct MyCmd {
    pub module: String,
    #[command(flatten)]
    pub common: CommonArgs,  // Adds --project, --regex, --limit
}
```

**Testing pattern:**
- Uses `rstest` with `#[fixture]` for shared test data
- Uses `tempfile` for filesystem-based tests
- Tests live alongside implementation in each module
- Output tests use expected string constants for clarity
- Test macros in `cli/src/test_macros.rs` reduce boilerplate (see `docs/TESTING_STRATEGY.md`)
- Shared fixtures in `db/src/fixtures/` for database and output tests
- Database test utilities available via `db` crate with `test-utils` feature
- Run with `cargo test` or `cargo nextest run`

**Adding new commands:**
See `docs/NEW_COMMANDS.md` for a step-by-step recipe. For module-grouped commands specifically, see the "Adding Module-Grouped Commands" section in `docs/NEW_COMMANDS.md`.

## Architectural Patterns & Refactoring Rules

### Query-Level vs Output-Level Concerns

**Pattern:** Distinguish between query filters and output metadata.

**Query filters** (applied during database query):
- `project`, `module_pattern`, `function_pattern`
- `regex`, `limit`, `depth`
- These are parameters, not data

**Output metadata** (included in result struct):
- `total_items`, `entries`, `items`
- `file`, `kind`, `start_line`
- These describe the results

**Rule:** Never include query-level filters in output result structs.

**Benefits:**
- Cleaner result types that only carry data
- No confusion about what's data vs. configuration
- Easier to cache/serialize results

### Avoid Custom Outputable Implementations

**Anti-pattern:**
```rust
impl Outputable for CustomResult {
    fn to_table(&self) -> String {
        // 40+ lines of boilerplate layout logic
    }
}
```

**Preferred pattern:**
```rust
impl TableFormatter for ModuleGroupResult<Entry> {
    type Entry = Entry;
    // 15-20 lines of domain-specific formatting
    fn format_header(&self) -> String { ... }
    fn format_entry(&self, entry, module, file) -> String { ... }
    // TableFormatter default impl handles all layout
}
```

This rule applies to all module-grouped output commands.

**See `docs/NEW_COMMANDS.md`** for detailed implementation patterns and checklist for adding new module-grouped commands.
