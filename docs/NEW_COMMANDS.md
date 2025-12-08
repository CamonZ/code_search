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
