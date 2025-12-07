# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Test Commands

```bash
cargo build              # Build the project
cargo test               # Run all tests
cargo test <test_name>   # Run a single test by name
cargo run -- --help      # Show CLI help
```

## Architecture

This is a Rust CLI tool for querying call graph data stored in a CozoDB SQLite database. Uses Rust 2024 edition with clap derive macros for CLI parsing.

**Code organization:**
- `src/main.rs` - Entry point, module declarations
- `src/cli.rs` - Top-level CLI structure with global `--db` and `--format` flags
- `src/commands/mod.rs` - `Command` enum, `Execute` trait, dispatch via `run()` method
- `src/commands/<name>/` - Individual command modules (directory structure)
- `src/db.rs` - Database connection and query utilities
- `src/output.rs` - `OutputFormat` enum, `Outputable` trait for formatting results

**Command module structure:**
Each command is a directory module with these files:
- `mod.rs` - Command struct with clap attributes, re-exports
- `execute.rs` - `Execute` trait implementation, result types, tests
- `output.rs` - `Outputable` implementation for the command's result type
- `models.rs` - (optional) Data models for deserialization

**Execute trait:**
```rust
pub trait Execute {
    type Output: Outputable;
    fn execute(self, db_path: &Path) -> Result<Self::Output, Box<dyn Error>>;
}
```

**Output formatting:**
- Commands return a result type implementing `Outputable`
- Four formats available: `Table` (default), `Json`, `Terse`, `Toon`
- `Outputable` requires `Serialize` + `to_table()` + `to_terse()` methods
- JSON and Toon formats are derived automatically from `Serialize`

**Dispatch flow:**
```
main.rs → Args::parse() → Command::run(db_path, format) → cmd.execute() → result.format()
```

**Testing pattern:**
- Uses `rstest` with `#[fixture]` for shared test data
- Uses `tempfile` for filesystem-based tests
- Tests live alongside implementation in each module
- Output tests use expected string constants for clarity
- Run with `cargo test` or `cargo nextest run`

**Adding new commands:**
See `docs/NEW_COMMANDS.md` for a step-by-step recipe.
