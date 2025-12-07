# Testing Strategy

This document describes the testing patterns and macros used in this codebase.

## Test Organization

Tests are organized by file type within each command module:

| File | Purpose | Test Patterns |
|------|---------|---------------|
| `cli_tests.rs` | CLI argument parsing | Defaults, options, required args, limit validation |
| `execute.rs` | Database query execution | Empty DB, no match, core functionality, filters |
| `output.rs` | Output formatting implementation | (no tests - implementation only) |
| `output_tests.rs` | Output formatting tests | Table/JSON/Toon snapshots using macros |

## Test Macros

The `src/test_macros.rs` module provides declarative macros to reduce boilerplate. Use these macros for standard test patterns; use regular tests for edge cases.

### CLI Parsing Macros

#### `cli_defaults_test!`

Tests that a command has expected default values when invoked with minimal arguments.

```rust
crate::cli_defaults_test! {
    command: "hotspots",
    variant: Hotspots,
    required_args: [],
    defaults: {
        project: "default",
        regex: false,
        limit: 20,
    },
}
```

**When to use:** Command has no required arguments and you want to verify multiple default values at once.

#### `cli_option_test!`

Tests that a specific CLI option sets a field correctly.

```rust
crate::cli_option_test! {
    command: "search",
    variant: Search,
    test_name: test_with_project,
    args: ["--pattern", "User", "--project", "my_app"],
    field: project,
    expected: "my_app",
}
```

**When to use:** Testing that an option sets a field to an expected value using `assert_eq!`.

**When NOT to use:** When you need `matches!` macro (e.g., for enums) or custom assertions.

#### `cli_limit_tests!`

Generates three standard limit validation tests: default value, zero rejected, max exceeded.

```rust
crate::cli_limit_tests! {
    command: "search",
    variant: Search,
    required_args: ["--pattern", "User"],
    limit: {
        field: limit,
        default: 100,
        max: 1000,
    },
}
```

**When to use:** Every command with a `--limit` flag should use this macro.

#### `cli_required_arg_test!`

Tests that a command fails when a required argument is missing.

```rust
crate::cli_required_arg_test! {
    command: "search",
    test_name: test_search_requires_pattern,
    required_arg: "--pattern",
}
```

**When to use:** For each required argument on a command.

#### `cli_error_test!`

Tests that specific invalid arguments cause parsing to fail.

```rust
crate::cli_error_test! {
    command: "search",
    test_name: test_invalid_kind,
    args: ["--pattern", "test", "--kind", "invalid"],
}
```

**When to use:** Testing invalid argument values beyond the standard limit validation.

### Execute Test Macros

#### `execute_test_fixture!`

Generates a fixture that creates a populated test database.

```rust
crate::execute_test_fixture! {
    fixture_name: populated_db,
    json: r#"{ "calls": [], ... }"#,
    project: "test_project",
}
```

#### `execute_empty_db_test!`

Tests that a command fails gracefully on an empty (uninitialized) database.

```rust
crate::execute_empty_db_test! {
    cmd_type: SearchCmd,
    cmd: SearchCmd { pattern: "test".into(), ... },
}
```

### Output Test Macros

Output tests live in a separate `output_tests.rs` file and use the `output_table_test!` macro with string literal snapshots. This approach provides exact output verification and makes test failures easy to debug.

#### `output_table_test!` (Recommended)

Tests that output exactly matches an expected string. Works with fixtures and supports all output formats.

**For Table format (default):**
```rust
crate::output_table_test! {
    test_name: test_to_table_empty,
    fixture: empty_result,
    fixture_type: SearchResult,
    expected: EMPTY_TABLE,
}
```

**For JSON format:**
```rust
crate::output_table_test! {
    test_name: test_format_json,
    fixture: single_result,
    fixture_type: SearchResult,
    expected: SINGLE_JSON,
    format: Json,
}
```

**For Toon format:**
```rust
crate::output_table_test! {
    test_name: test_format_toon,
    fixture: single_result,
    fixture_type: SearchResult,
    expected: SINGLE_TOON,
    format: Toon,
}
```

#### `output_json_test!` (Partial Matching)

Tests that JSON output contains expected field values. Use when exact matching is too brittle.

```rust
crate::output_json_test! {
    test_name: test_format_json,
    fixture: single_result,
    fixture_type: SearchResult,
    assertions: {
        "pattern": "MyApp",
        "kind": "modules",
    },
}
```

#### `output_toon_test!` (Partial Matching)

Tests that Toon output contains expected strings. Use when exact matching is too brittle.

```rust
crate::output_toon_test! {
    test_name: test_format_toon,
    fixture: single_result,
    fixture_type: SearchResult,
    contains: ["pattern: MyApp", "modules["],
}
```

#### Getting Snapshot Values

To capture the actual output for your snapshot constants, add a temporary test:

```rust
#[test]
fn print_outputs() {
    use crate::output::{Outputable, OutputFormat};
    let result = single_result();
    println!("JSON:\n{}\n", result.format(OutputFormat::Json));
    println!("TOON:\n{}\n", result.format(OutputFormat::Toon));
}
```

Run with `cargo test <cmd>::output_tests::tests::print_outputs -- --nocapture`

## When to Use Regular Tests

Use regular tests (not macros) when:

1. **Using `matches!` macro** - Enum variant matching requires `matches!` which can't be expressed in `assert_eq!`
   ```rust
   #[rstest]
   fn test_search_kind_default_is_modules() {
       let args = Args::try_parse_from(["code_search", "search", "--pattern", "test"]).unwrap();
       match args.command {
           Command::Search(cmd) => {
               assert!(matches!(cmd.kind, SearchKind::Modules));
           }
           _ => panic!("Expected Search command"),
       }
   }
   ```

2. **Complex assertions** - When you need multiple assertions or conditional logic

3. **Custom setup/teardown** - When test requires special setup beyond the standard fixtures

4. **Testing error messages** - When you need to verify specific error message content

5. **Parameterized edge cases** - Using rstest's `#[case]` for multiple inputs

## Example Test Files

Annotated examples are available in the `docs/examples/` directory as a reference for the patterns described above:

- **[cli_tests.rs.example](./examples/cli_tests.rs.example)** - CLI argument parsing test patterns
- **[output_tests.rs.example](./examples/output_tests.rs.example)** - Output formatting test patterns with snapshots

These are guidelines, not templates to copy blindly. Each command has different requirements - use the patterns that apply to your specific case.

## Checklist for New Command Tests

- [ ] Create `cli_tests.rs` with macro-generated tests for:
  - [ ] Required arguments (`cli_required_arg_test!`)
  - [ ] Default values (`cli_defaults_test!` if no required args)
  - [ ] Each option (`cli_option_test!`)
  - [ ] Limit validation (`cli_limit_tests!`)
- [ ] Add regular tests for edge cases (enum matching, complex assertions)
- [ ] In `execute.rs`, add tests for:
  - [ ] Empty database handling
  - [ ] No match scenarios
  - [ ] Core functionality with populated database
  - [ ] Filter combinations
- [ ] Create `output_tests.rs` with snapshot tests for:
  - [ ] Empty result table formatting
  - [ ] Single/multiple result table formatting
  - [ ] JSON format exact snapshot
  - [ ] Toon format exact snapshots (empty and populated)
