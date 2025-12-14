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
- `src/output.rs` - `OutputFormat` enum, `Outputable` and `TableFormatter` traits
- `src/dedup.rs` - Deduplication utilities (`sort_and_deduplicate`, `DeduplicationFilter`)
- `src/utils.rs` - Module grouping helpers (`group_by_module`, `convert_to_module_groups`)
- `src/types/` - Shared types (`ModuleGroupResult`, `ModuleGroup`, `Call`, etc.)

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
    fn execute(self, db: &DbInstance) -> Result<Self::Output, Box<dyn Error>>;
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
2. Test fixtures exist in `src/fixtures/output/<command>/` for JSON and toon
3. Output tests verify round-trip consistency between formats

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
See `docs/NEW_COMMANDS.md` for a step-by-step recipe. For module-grouped commands specifically, see the "Adding Module-Grouped Commands" section in `docs/NEW_COMMANDS.md`.

## Configuration System

The application uses a `.code_search.json` configuration file in the current directory to specify database settings. The config file is **required** and must be present before running any commands.

**Configuration File Location:**
Place `.code_search.json` in the directory where you run code_search commands.

### Database Backend Options

#### SQLite (Embedded Database)

Recommended for local development. Database is stored as a single file.

```json
{
  "database": {
    "type": "sqlite",
    "path": "./cozo.sqlite"
  }
}
```

**Fields:**
- `type`: Must be `"sqlite"`
- `path`: File system path to the database (relative or absolute)

#### In-Memory Database (Testing)

For testing and ephemeral analysis. Data is lost when the process exits.

```json
{
  "database": {
    "type": "memory"
  }
}
```

**Fields:**
- `type`: Must be `"memory"`

#### PostgreSQL with Connection String (Not Yet Implemented)

Planned for future release.

```json
{
  "database": {
    "type": "postgres",
    "connection_string": "postgres://user:password@localhost:5432/mydb"
  }
}
```

#### PostgreSQL with Individual Options (Not Yet Implemented)

Planned for future release.

```json
{
  "database": {
    "type": "postgres",
    "host": "localhost",
    "port": 5432,
    "user": "myuser",
    "password": "mypassword",
    "database": "mydb",
    "ssl": false,
    "graph_name": "call_graph"
  }
}
```

### Configuration Resolution

The tool uses this priority order:

1. **Config File** (highest priority) - `.code_search.json` in current directory
2. **Environment Variables** (second priority) - `DATABASE_URL` or `COZO_PATH`
3. **Default** (lowest priority) - `./cozo.sqlite`

The config file takes precedence if present. If not, environment variables are checked, otherwise defaults to `./cozo.sqlite`.

### Security Considerations

**Protecting Credentials:**

1. **Add to .gitignore** to prevent credentials from being committed:
   ```
   .code_search.json
   ```

2. **Restrict file permissions** (Unix/Linux/macOS):
   ```bash
   chmod 600 .code_search.json
   ```

3. **Use environment-specific configs** for development vs. production:
   ```
   .code_search.local.json    # Local (in .gitignore)
   .code_search.staging.json  # Staging (in .gitignore)
   .code_search.prod.json     # Production (in .gitignore)
   ```

4. **Provide .code_search.example.json** in git for developers to copy:
   ```json
   {
     "database": {
       "type": "sqlite",
       "path": "./cozo.sqlite"
     }
   }
   ```

**Best Practices:**

- Never commit `.code_search.json` with credentials to version control
- Use `.gitignore` to ensure accidental commits are blocked
- For CI/CD, use environment variables instead of config files
- Rotate database passwords regularly if hardcoded
- Use file permissions to restrict access (chmod 600)

For detailed configuration guidance, see `docs/configuration.md`.

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
