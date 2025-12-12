# Adding New Commands

This guide walks through adding a new command to the CLI.

## Overview

Each command is a directory module under `src/commands/` with this structure:

```
src/commands/<name>/
├── mod.rs         # Command struct with clap attributes
├── cli_tests.rs   # CLI parsing tests (using test macros)
├── execute.rs     # Execute trait impl, result type, tests
├── output.rs      # Outputable impl for result type
├── output_tests.rs # Output formatting tests (using test macros)
└── models.rs      # (optional) Data models
```

See [TESTING_STRATEGY.md](./TESTING_STRATEGY.md) for details on the test macros and when to use them.

## Step-by-Step Recipe

### 1. Create the command directory

```bash
mkdir src/commands/<name>
```

### 2. Define the command struct (`mod.rs`)

```rust
mod cli_tests;
mod execute;
mod output;
mod output_tests;

use clap::Args;

/// Description of what the command does
#[derive(Args, Debug)]
#[command(after_help = "\
Examples:
  code_search <name> --arg value    # Example usage")]
pub struct <Name>Cmd {
    /// Description of the argument
    #[arg(short, long)]
    pub some_arg: String,

    /// Project to search in
    #[arg(long, default_value = "default")]
    pub project: String,

    /// Maximum number of results to return (1-1000)
    #[arg(short, long, default_value_t = 100, value_parser = clap::value_parser!(u32).range(1..=1000))]
    pub limit: u32,
}
```

### 3. Add CLI tests (`cli_tests.rs`)

See [examples/cli_tests.rs.example](./examples/cli_tests.rs.example) for a reference showing the available test patterns. Refer to [TESTING_STRATEGY.md](./TESTING_STRATEGY.md) for details on when to use each macro.

### 4. Implement Query Logic (`src/queries/<name>.rs`)

Create a new file in `src/queries/` to handle the database interaction. This keeps the Datalog queries separate from the command logic.

```rust
use std::error::Error;
use cozo::{DataValue, DbInstance};
use crate::db::{run_query, Params, extract_string};

pub fn <name>_query(
    db: &DbInstance,
    arg: &str,
) -> Result<Vec<String>, Box<dyn Error>> {
    let script = "?[value] := *relation{value}, value = $arg";
    let params = Params::from([("arg".to_string(), DataValue::Str(arg.into()))]);
    
    let rows = run_query(db, script, params)?;
    
    // ... extraction logic ...
    Ok(vec![])
}
```

Don't forget to register the new module in `src/queries/mod.rs`.

### 5. Implement Execute (`execute.rs`)

This file contains the core command logic and its tests. It orchestrates the execution by calling the query function.

See [examples/execute_impl.rs.example](./examples/execute_impl.rs.example) for the full boilerplate including imports, error handling, and test macros.

```rust
impl Execute for <Name>Cmd {
    type Output = <Name>Result;

    fn execute(self, db: &DbInstance) -> Result<Self::Output, Box<dyn Error>> {
        // Call the query function from src/queries/<name>.rs
        let results = crate::queries::<name>::<name>_query(db, &self.some_arg)?;

        Ok(<Name>Result {
            results,
            ..Default::default()
        })
    }
}
```

### 6. Implement Outputable (`output.rs`)

```rust
use crate::output::Outputable;
use super::execute::<Name>Result;

impl Outputable for <Name>Result {
    fn to_table(&self) -> String {
        // Human-readable table format
        todo!()
    }
}
```

### 7. Add output tests (`output_tests.rs`)

See [examples/output_tests.rs.example](./examples/output_tests.rs.example) for a reference showing the snapshot testing pattern. The example includes a helper for generating the actual output values to use in your snapshots.

### 8. Register the command (`src/commands/mod.rs`)

Add the module declaration:

```rust
mod <name>;

pub use <name>::<Name>Cmd;
```

Add the variant to the `Command` enum:

```rust
#[derive(Subcommand, Debug)]
pub enum Command {
    /// Existing commands...
    Import(ImportCmd),

    /// Description of your command
    <Name>(<Name>Cmd),

    #[command(external_subcommand)]
    Unknown(Vec<String>),
}
```

Add the match arm in `run()`:

```rust
impl Command {
    pub fn run(self, db: &DbInstance, format: OutputFormat) -> Result<String, Box<dyn Error>> {
        match self {
            Command::Import(cmd) => {
                let result = cmd.execute(db)?;
                Ok(result.format(format))
            }
            Command::<Name>(cmd) => {
                let result = cmd.execute(db)?;
                Ok(result.format(format))
            }
            Command::Unknown(args) => {
                Err(format!("Unknown command: {}", args.first().unwrap_or(&String::new())).into())
            }
        }
    }
}
```

### 9. Verify

```bash
cargo build
cargo test
cargo run -- <name> --help
```

## Checklist

- [ ] Created `src/commands/<name>/` directory
- [ ] Defined command struct with clap attributes in `mod.rs`
- [ ] Added `#[command(after_help = "...")]` with usage examples
- [ ] Added `--limit` with range validation (1-1000)
- [ ] Created `cli_tests.rs` with test macros (see [TESTING_STRATEGY.md](./TESTING_STRATEGY.md))
  - [ ] Required argument tests (`cli_required_arg_test!`)
  - [ ] Option tests (`cli_option_test!`)
  - [ ] Limit validation tests (`cli_limit_tests!`)
  - [ ] Edge case tests (regular tests for `matches!` etc.)
- [ ] Created `src/queries/<name>.rs` and implemented query logic
- [ ] Registered query module in `src/queries/mod.rs`
- [ ] Implemented `Execute` trait in `execute.rs`
- [ ] Added execution tests in `execute.rs` using macros
  - [ ] Use shared fixture (`shared_fixture!`) or inline JSON (`execute_test_fixture!`)
  - [ ] Empty database test (`execute_empty_db_test!`)
  - [ ] No match test (`execute_no_match_test!`)
  - [ ] Core functionality tests (`execute_test!`, `execute_count_test!`, etc.)
- [ ] Created result type with `#[derive(Debug, Default, Serialize)]`
- [ ] Implemented `Outputable` in `output.rs`
- [ ] Created `output_tests.rs` with test macros
  - [ ] Table format tests (empty and populated)
  - [ ] JSON format test with snapshot
  - [ ] Toon format tests (empty and populated)
- [ ] Registered command in `src/commands/mod.rs`
- [ ] Added match arm in `Command::run()`
- [ ] Verified with `cargo build && cargo test`

---

## Adding Module-Grouped Commands

When building a command that groups results by module (e.g., showing functions per module, dependencies per module), follow these architectural patterns to reduce boilerplate and maintain consistency.

### 1. Use Generic Result Types

Instead of defining a custom result type per command, use shared containers:

- **`ModuleGroupResult<E>`** - For simple two-parameter results (module pattern + optional function pattern)
- **`ModuleCollectionResult<E>`** - For results with additional filter metadata (kind_filter, name_filter, etc.)

Both use `ModuleGroup<E>` as the grouping container with fields: `name`, `file`, `entries`.

**Benefits:**
- Single source of truth for module-grouped result structure
- Automatic JSON/Toon serialization consistency
- No more custom result type boilerplate

### 2. Use Deduplication Utilities

The codebase provides two reusable deduplication strategies in `crate::dedup`:

**Strategy A: deduplicate_retain()** - For post-sort deduplication
```rust
calls.sort_by_key(|c| c.line);
crate::dedup::deduplicate_retain(&mut calls, |c| {
    (c.callee.module.clone(), c.callee.name.clone(), c.callee.arity)
});
```
Use when: Items are sorted and you want to keep first occurrence

**Strategy B: DeduplicationFilter** - For prevention during collection
```rust
let mut filter = crate::dedup::DeduplicationFilter::new();
if filter.should_process(key) {
    // Add entry to result
}
```
Use when: Building results incrementally and preventing duplicates before adding

**Benefits:**
- No more HashSet boilerplate scattered across commands
- Consistent deduplication approach codebase-wide
- Clear intent via strategy choice

### 3. Implement TableFormatter Trait

Instead of implementing `Outputable` with 40+ lines of layout boilerplate, implement the `TableFormatter` trait in `output.rs`:

```rust
impl TableFormatter for ModuleGroupResult<YourEntry> {
    type Entry = YourEntry;

    // Required methods
    fn format_header(&self) -> String { /* e.g., "Function: module.pattern" */ }
    fn format_empty_message(&self) -> String { /* e.g., "No functions found." */ }
    fn format_summary(&self, total: usize, module_count: usize) -> String {
        format!("Found {} result(s) in {} module(s):", total, module_count)
    }
    fn format_module_header(&self, module_name: &str, module_file: &str) -> String {
        format!("{}:", module_name)  // Simple format, or include file if available
    }
    fn format_entry(&self, entry: &Self::Entry, module: &str, file: &str) -> String {
        // Format a single entry
    }

    // Optional: provide additional details
    fn format_entry_details(&self, entry: &Self::Entry, module: &str, file: &str) -> Vec<String> {
        vec![]  // Return empty for simple entries, override for complex ones
    }

    // Optional: customize spacing
    fn blank_after_summary(&self) -> bool { true }     // Blank line after summary?
    fn blank_before_module(&self) -> bool { false }    // Blank line before each module?
}
```

The default `Outputable` implementation will handle all layout logic. The trait default methods let you customize spacing as needed.

**Benefits:**
- Default implementation handles all boilerplate layout (~40 lines per command)
- Eliminates ~320 lines of duplicated code across module-grouped commands
- Automatic consistency in table formatting
- JSON and Toon formats work automatically via `#[derive(Serialize)]`

### 4. Document File Field Decisions

The `ModuleGroup.file` field should be populated when the module's entries are associated with a specific file location. Document your decision:

**Populate file when:**
- Entries originate from a specific file (e.g., calls from a function in a specific file)
- File information is available in the query and semantically meaningful

**Leave file empty when:**
- Targets/dependents are the grouping key (a module can be defined across multiple files)
- File location is not meaningful for the command (e.g., specs, types)
- Add a comment explaining:

```rust
ModuleGroup {
    name,
    // File is intentionally empty because callees are the grouping key,
    // and a module can be defined across multiple files.
    file: String::new(),
    entries,
}
```

### Implementation Checklist for Module-Grouped Commands

- [ ] Use `ModuleGroupResult<E>` or `ModuleCollectionResult<E>` for result type
- [ ] Use `ModuleGroup<E>` for the grouping container
- [ ] Implement `TableFormatter` trait in `output.rs` (NOT `Outputable`)
- [ ] Use `crate::dedup::*` utilities for deduplication (if needed)
- [ ] Document `file` field population decision with inline comments
- [ ] Test `to_table()` output against expected string constants
- [ ] Verify all tests pass (`cargo test`)
- [ ] JSON/Toon formats automatically work via `#[derive(Serialize)]`
