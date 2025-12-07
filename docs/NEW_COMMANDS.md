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

### 4. Implement Execute (`execute.rs`)

This file contains the core command logic and its tests.

```rust
use std::error::Error;
use std::path::Path;

use serde::Serialize;
use thiserror::Error;

use super::<Name>Cmd;
use crate::commands::Execute;
use crate::db::{open_db, run_query, Params};

#[derive(Error, Debug)]
enum <Name>Error {
    #[error("Description: {message}")]
    SomeError { message: String },
}

/// Result of the command execution
#[derive(Debug, Default, Serialize)]
pub struct <Name>Result {
    // Fields for your result
}

impl Execute for <Name>Cmd {
    type Output = <Name>Result;

    fn execute(self, db_path: &Path) -> Result<Self::Output, Box<dyn Error>> {
        let db = open_db(db_path)?;

        // Command logic here

        Ok(<Name>Result::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::{fixture, rstest};

    // Use shared fixtures when possible (see src/fixtures/mod.rs)
    // Available: call_graph, type_signatures, structs
    crate::shared_fixture! {
        fixture_name: populated_db,
        fixture_type: call_graph,  // or type_signatures, structs
        project: "test_project",
    }

    // =========================================================================
    // Core functionality tests
    // =========================================================================

    crate::execute_test! {
        test_name: test_basic_functionality,
        fixture: populated_db,
        cmd: <Name>Cmd {
            // ... command args
        },
        assertions: |result| {
            // Your assertions here
        },
    }

    // =========================================================================
    // No match / empty result tests
    // =========================================================================

    crate::execute_no_match_test! {
        test_name: test_no_match,
        fixture: populated_db,
        cmd: <Name>Cmd {
            // ... args that should return empty
        },
        empty_field: results,  // field that should be empty
    }

    // =========================================================================
    // Error handling tests
    // =========================================================================

    crate::execute_empty_db_test! {
        cmd_type: <Name>Cmd,
        cmd: <Name>Cmd {
            // ... any valid args
        },
    }
}
```

**Note:** If your tests need specific data that differs from the shared fixtures, you can still use inline JSON:

```rust
const TEST_JSON: &str = r#"{ ... }"#;

crate::execute_test_fixture! {
    fixture_name: custom_db,
    json: TEST_JSON,
    project: "test_project",
}
```

### 5. Implement Outputable (`output.rs`)

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

### 6. Add output tests (`output_tests.rs`)

See [examples/output_tests.rs.example](./examples/output_tests.rs.example) for a reference showing the snapshot testing pattern. The example includes a helper for generating the actual output values to use in your snapshots.

### 7. Register the command (`src/commands/mod.rs`)

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
    pub fn run(self, db_path: &Path, format: OutputFormat) -> Result<String, Box<dyn Error>> {
        match self {
            Command::Import(cmd) => {
                let result = cmd.execute(db_path)?;
                Ok(result.format(format))
            }
            Command::<Name>(cmd) => {
                let result = cmd.execute(db_path)?;
                Ok(result.format(format))
            }
            Command::Unknown(args) => {
                Err(format!("Unknown command: {}", args.first().unwrap_or(&String::new())).into())
            }
        }
    }
}
```

### 8. Verify

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
