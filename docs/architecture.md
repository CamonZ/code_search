# Architecture

This is a Rust CLI tool for querying call graph data stored in a SurrealDB database (RocksDB storage). Uses Rust 2024 edition with clap derive macros for CLI parsing.

## Code Organization

### Database crate (`db/src/`)

- `lib.rs` - Public API surface, re-exports
- `db.rs` - Database connection and query utilities
- `queries/<name>.rs` - SurrealQL queries and result parsing (31 query modules)
- `query_builders.rs` - SQL condition builders (`ConditionBuilder`, `OptionalConditionBuilder`)
- `types/` - Shared types (`ModuleGroupResult`, `ModuleGroup`, `Call`, `FunctionRef`, etc.)
- `fixtures/` - Test data (feature-gated)
- `test_utils.rs` - Test helpers (feature-gated)

### CLI crate (`cli/src/`)

- `main.rs` - Entry point, module declarations
- `cli.rs` - Top-level CLI structure with global `--db` and `--format` flags
- `commands/mod.rs` - `Command` enum, `Execute` trait, `CommonArgs`, dispatch via enum_dispatch
- `commands/<name>/` - Individual command modules (27 commands, directory structure)
- `output.rs` - `OutputFormat` enum, `Outputable` and `TableFormatter` traits
- `dedup.rs` - Deduplication utilities (`sort_and_deduplicate`, `deduplicate_retain`)
- `utils.rs` - Presentation helpers (`group_by_module`, `convert_to_module_groups`, `format_type_definition`)
- `test_macros.rs` - Declarative test macros for CLI, execute, and output tests

## Command Module Structure

Each command is a directory module with these files:

- `mod.rs` - Command struct with clap attributes, re-exports
- `execute.rs` - `Execute` trait implementation, result types, tests
- `output.rs` - `Outputable` implementation for the command's result type
- `models.rs` - (optional) Data models for deserialization

## Execute Trait

```rust
// Defined in cli/src/commands/mod.rs
pub trait Execute {
    type Output: Outputable;
    fn execute(self, db: &dyn db::backend::Database) -> Result<Self::Output, Box<dyn Error>>;
}
```

## Dispatch Flow

```
main.rs -> Args::parse() -> Command::run(db_path, format) -> cmd.execute() -> result.format()
```

## Key Traits

- `Execute` - Core execution trait: `execute(self, db) -> Result<Self::Output>`
- `CommandRunner` - Auto-generated via `enum_dispatch` macro for all Command variants
- `Outputable` - Output formatting: `to_table()` + automatic `format(OutputFormat)`
- `TableFormatter` - Customizable table layout for module-grouped results

## CommonArgs Pattern

Commands share common arguments via `#[command(flatten)]`:

```rust
pub struct MyCmd {
    pub module: String,
    #[command(flatten)]
    pub common: CommonArgs,  // Adds --regex, --limit
}
```

## Adding New Commands

See [NEW_COMMANDS.md](./NEW_COMMANDS.md) for a step-by-step recipe. For module-grouped commands specifically, see the "Adding Module-Grouped Commands" section in [NEW_COMMANDS.md](./NEW_COMMANDS.md).
