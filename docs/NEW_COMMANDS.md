# Adding New Commands

This guide walks through adding a new command to the CLI.

## Key Improvement: enum_dispatch

This codebase uses the `enum_dispatch` crate to automatically generate command dispatch logic. This means:

- ✅ **No manual match arms** - The `#[enum_dispatch(CommandRunner)]` macro on the `Command` enum generates all dispatch logic automatically
- ✅ **Simpler registration** - Just add a variant to the enum, and dispatch works automatically
- ✅ **Better organization** - Each command implements `CommandRunner` in its own module (not in a central match statement)
- ✅ **Type-safe & fast** - Compile-time generated, zero-cost dispatch

When adding a new command, you don't need to touch the dispatch logic in `src/commands/mod.rs`. Just implement `CommandRunner` in your command's `mod.rs` file!

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

use std::error::Error;

use clap::Args;
use cozo::DbInstance;

use crate::commands::{CommandRunner, Execute};
use crate::output::{OutputFormat, Outputable};

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

impl CommandRunner for <Name>Cmd {
    fn run(self, db: &DbInstance, format: OutputFormat) -> Result<String, Box<dyn Error>> {
        let result = self.execute(db)?;
        Ok(result.format(format))
    }
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

Add the module declaration and public export:

```rust
mod <name>;

pub use <name>::<Name>Cmd;
```

Add the variant to the `Command` enum:

```rust
#[derive(Subcommand, Debug)]
#[enum_dispatch(CommandRunner)]
pub enum Command {
    /// Existing commands...
    Import(ImportCmd),

    /// Description of your command
    <Name>(<Name>Cmd),

    #[command(external_subcommand)]
    Unknown(Vec<String>),
}
```

**Note:** The `#[enum_dispatch(CommandRunner)]` attribute is already on the `Command` enum. The `enum_dispatch` crate automatically generates the dispatch logic for all variants. You do NOT need to add a match arm in `Command::run()` - the `CommandRunner` implementation you added to your command's `mod.rs` file (in step 2) is all that's needed!

The dispatch is handled entirely by the `enum_dispatch` procedural macro at compile time, which is faster and more maintainable than manual match arms.

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
- [ ] **Implemented `CommandRunner` trait in `mod.rs`** (new with enum_dispatch)
  - [ ] Added imports: `std::error::Error`, `cozo::DbInstance`
  - [ ] Added imports: `crate::commands::{CommandRunner, Execute}`, `crate::output::{OutputFormat, Outputable}`
  - [ ] Implemented `impl CommandRunner for <Name>Cmd` with `run()` method
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
  - [ ] Added module declaration: `mod <name>;`
  - [ ] Added public export: `pub use <name>::<Name>Cmd;`
  - [ ] Added enum variant to `Command` enum (dispatch is automatic via `#[enum_dispatch(CommandRunner)]`)
  - [ ] **No match arm needed** - enum_dispatch handles it automatically!
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

The codebase provides reusable deduplication strategies in `crate::dedup`:

**sort_and_deduplicate()** - Combined sort + deduplicate (most common)
```rust
use crate::dedup::sort_and_deduplicate;

sort_and_deduplicate(
    &mut calls,
    |c| c.line,  // Sort key
    |c| (c.callee.module.clone(), c.callee.name.clone(), c.callee.arity),  // Dedup key
);
```
Use when: You need to sort by one key and deduplicate by another (e.g., sort by line, keep first call to each function)

**deduplicate_retain()** - For post-sort deduplication
```rust
calls.sort_by_key(|c| c.line);
crate::dedup::deduplicate_retain(&mut calls, |c| {
    (c.callee.module.clone(), c.callee.name.clone(), c.callee.arity)
});
```
Use when: Items are already sorted and you want to remove duplicates while preserving order

**DeduplicationFilter** - For prevention during collection
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

### 3. Use Module Grouping Utilities

The codebase provides helpers in `crate::utils` for grouping results by module:

**group_by_module()** - Simple grouping without file tracking
```rust
use crate::utils::group_by_module;

let groups = group_by_module(items, |item| {
    (item.module.clone(), entry_from_item(item))
});
```
Use when: File information is not needed (file defaults to empty string)

**group_by_module_with_file()** - Grouping with file tracking
```rust
use crate::utils::group_by_module_with_file;

let groups = group_by_module_with_file(items, |item| {
    (item.module.clone(), entry_from_item(item), item.file.clone())
});
```
Use when: You need to track which file each module group belongs to

**convert_to_module_groups()** - For two-level nested maps
```rust
use crate::utils::convert_to_module_groups;

// When you have: BTreeMap<module, BTreeMap<function_key, Vec<Call>>>
let groups = convert_to_module_groups(
    by_module,
    |key, calls| build_entry(key, calls),           // Entry builder
    |_module, functions| extract_file(functions),   // File strategy
);
```
Use when: You've grouped by module and then by function, and need to flatten to `Vec<ModuleGroup<E>>`

**Benefits:**
- Automatic BTreeMap ordering (consistent output)
- Single source of truth for grouping logic
- Clear separation of entry building and file extraction

### 4. Implement TableFormatter Trait

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

### 5. Document File Field Decisions

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

### Complete Example: calls_from Command

Here's how the `calls_from` command combines these patterns:

```rust
// execute.rs
use crate::dedup::sort_and_deduplicate;
use crate::utils::convert_to_module_groups;
use crate::types::{Call, ModuleGroupResult};

impl ModuleGroupResult<CallerFunction> {
    pub fn from_calls(module_pattern: String, function_pattern: String, calls: Vec<Call>) -> Self {
        let total_items = calls.len();

        // Step 1: Group by module -> function -> calls using BTreeMap
        let mut by_module: BTreeMap<String, BTreeMap<CallerFunctionKey, Vec<Call>>> = BTreeMap::new();
        for call in calls {
            let fn_key = CallerFunctionKey { /* ... */ };
            by_module
                .entry(call.caller.module.clone())
                .or_default()
                .entry(fn_key)
                .or_default()
                .push(call);
        }

        // Step 2: Convert to ModuleGroups with deduplication
        let items = convert_to_module_groups(
            by_module,
            |key, mut calls| {
                // Deduplicate calls within each function
                sort_and_deduplicate(
                    &mut calls,
                    |c| c.line,
                    |c| (c.callee.module.clone(), c.callee.name.clone(), c.callee.arity),
                );
                CallerFunction { name: key.name, arity: key.arity, calls, /* ... */ }
            },
            // File strategy: extract from first call
            |_module, functions_map| {
                functions_map.values().next()
                    .and_then(|calls| calls.first())
                    .and_then(|call| call.caller.file.clone())
                    .unwrap_or_default()
            },
        );

        ModuleGroupResult { module_pattern, function_pattern: Some(function_pattern), total_items, items }
    }
}
```

```rust
// output.rs
impl TableFormatter for ModuleGroupResult<CallerFunction> {
    type Entry = CallerFunction;

    fn format_header(&self) -> String {
        format!("Calls from: {}.{}", self.module_pattern, self.function_pattern.as_deref().unwrap_or(""))
    }
    fn format_empty_message(&self) -> String { "No calls found.".to_string() }
    fn format_summary(&self, total: usize, _: usize) -> String { format!("Found {} call(s):", total) }
    fn format_module_header(&self, name: &str, file: &str) -> String { format!("{} ({})", name, file) }
    fn format_entry(&self, func: &CallerFunction, _: &str, _: &str) -> String {
        format!("{}/{} ({}:{})", func.name, func.arity, func.start_line, func.end_line)
    }
    fn format_entry_details(&self, func: &CallerFunction, module: &str, file: &str) -> Vec<String> {
        func.calls.iter().map(|c| c.format_outgoing(module, file)).collect()
    }
}
```

### Implementation Checklist for Module-Grouped Commands

- [ ] Use `ModuleGroupResult<E>` or `ModuleCollectionResult<E>` for result type
- [ ] Use `ModuleGroup<E>` for the grouping container
- [ ] Use `crate::utils::*` helpers for module grouping:
  - [ ] `group_by_module()` for simple cases
  - [ ] `group_by_module_with_file()` when file tracking is needed
  - [ ] `convert_to_module_groups()` for two-level nested maps
- [ ] Use `crate::dedup::*` utilities for deduplication (if needed):
  - [ ] `sort_and_deduplicate()` for combined sort + dedup
  - [ ] `deduplicate_retain()` for post-sort dedup
  - [ ] `DeduplicationFilter` for incremental collection
- [ ] Implement `TableFormatter` trait in `output.rs` (NOT `Outputable`)
- [ ] Document `file` field population decision with inline comments
- [ ] Test `to_table()` output against expected string constants
- [ ] Verify all tests pass (`cargo test`)
- [ ] JSON/Toon formats automatically work via `#[derive(Serialize)]`
