# Adding New Commands

This guide walks through adding a new command to the CLI.

## Overview

Each command is a directory module under `src/commands/` with this structure:

```
src/commands/<name>/
├── mod.rs      # Command struct with clap attributes
├── execute.rs  # Execute trait impl, result type, tests
├── output.rs   # Outputable impl for result type
└── models.rs   # (optional) Data models
```

## Step-by-Step Recipe

### 1. Create the command directory

```bash
mkdir src/commands/<name>
```

### 2. Define the command struct (`mod.rs`)

```rust
mod execute;
mod output;

use clap::Args;

#[derive(Args, Debug)]
pub struct <Name>Cmd {
    /// Description of the argument
    #[arg(short, long)]
    pub some_arg: String,

    /// Maximum number of results to return (1-1000)
    #[arg(short, long, default_value_t = 100, value_parser = clap::value_parser!(u32).range(1..=1000))]
    pub limit: u32,
}

#[cfg(test)]
mod tests {
    // CLI parsing tests including limit validation:
    // - test_<name>_limit_zero_rejected
    // - test_<name>_limit_exceeds_max_rejected
}
```

### 3. Implement Execute (`execute.rs`)

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
    use tempfile::NamedTempFile;

    #[fixture]
    fn db_file() -> NamedTempFile {
        NamedTempFile::new().expect("Failed to create temp db file")
    }

    #[rstest]
    fn test_execute_success(db_file: NamedTempFile) {
        let cmd = <Name>Cmd { /* args */ };
        let result = cmd.execute(db_file.path());
        assert!(result.is_ok());
    }
}
```

### 4. Implement Outputable (`output.rs`)

```rust
use crate::output::Outputable;
use super::execute::<Name>Result;

impl Outputable for <Name>Result {
    fn to_table(&self) -> String {
        // Human-readable table format
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::OutputFormat;
    use rstest::{fixture, rstest};

    const EXPECTED_TABLE_OUTPUT: &str = "...";

    #[fixture]
    fn result() -> <Name>Result {
        <Name>Result { /* fields */ }
    }

    #[rstest]
    fn test_to_table(result: <Name>Result) {
        assert_eq!(result.to_table(), EXPECTED_TABLE_OUTPUT);
    }

    #[rstest]
    fn test_format_json(result: <Name>Result) {
        let output = result.format(OutputFormat::Json);
        let _: serde_json::Value = serde_json::from_str(&output).expect("Valid JSON");
    }

    #[rstest]
    fn test_format_toon(result: <Name>Result) {
        let output = result.format(OutputFormat::Toon);
        // Verify toon format contains expected fields
        assert!(output.contains("field_name:"));
    }
}
```

### 5. Register the command (`src/commands/mod.rs`)

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

### 6. Verify

```bash
cargo build
cargo nextest run
cargo run -- <name> --help
```

## Checklist

- [ ] Created `src/commands/<name>/` directory
- [ ] Defined command struct with clap attributes in `mod.rs`
- [ ] Added `--limit` with range validation (1-1000) using `value_parser = clap::value_parser!(u32).range(1..=1000)`
- [ ] Added limit validation tests (zero rejected, exceeds max rejected)
- [ ] Implemented `Execute` trait in `execute.rs`
- [ ] Added execution tests in `execute.rs`
- [ ] Created result type with `#[derive(Debug, Default, Serialize)]`
- [ ] Implemented `Outputable` in `output.rs`
- [ ] Added output tests with expected string constants in `output.rs`
- [ ] Registered command in `src/commands/mod.rs`
- [ ] Added match arm in `Command::run()`
- [ ] Verified with `cargo build && cargo test`
