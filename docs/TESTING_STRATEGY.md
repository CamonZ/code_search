# Testing Strategy

This document describes the testing patterns and macros used in this codebase.

## Test Organization

Tests are organized by file type within each command module:

| File | Purpose | Test Patterns |
|------|---------|---------------|
| `cli_tests.rs` | CLI argument parsing | Defaults, options, required args, limit validation |
| `execute.rs` | Database query execution | Empty DB, no match, core functionality, filters |
| `output.rs` | Output formatting | Table output, JSON format, Toon format |

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

#### `output_table_test!`

Tests that `to_table()` produces an expected string.

```rust
crate::output_table_test! {
    test_name: test_to_table_empty,
    result: SearchResult { pattern: "test".into(), modules: vec![], functions: vec![] },
    expected: "Search: test (modules)\n\nNo results found.",
}
```

#### `output_json_test!`

Tests that JSON output is valid and contains expected fields.

```rust
crate::output_json_test! {
    test_name: test_format_json,
    result: SearchResult { ... },
    assertions: {
        "pattern": "MyApp",
        "kind": "modules",
    },
}
```

#### `output_toon_test!`

Tests that Toon output contains expected strings.

```rust
crate::output_toon_test! {
    test_name: test_format_toon,
    result: SearchResult { ... },
    contains: ["pattern: MyApp", "modules["],
}
```

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

## Example: Converting a Command's Tests

Here's the pattern for organizing a command's CLI tests:

```rust
// src/commands/<name>/cli_tests.rs

#[cfg(test)]
mod tests {
    use crate::cli::Args;
    use clap::Parser;
    use rstest::rstest;

    // =========================================================================
    // Macro-generated tests (standard patterns)
    // =========================================================================

    crate::cli_required_arg_test! {
        command: "<name>",
        test_name: test_requires_<arg>,
        required_arg: "--<arg>",
    }

    crate::cli_option_test! {
        command: "<name>",
        variant: <Name>,
        test_name: test_with_<option>,
        args: ["--<required>", "value", "--<option>", "value"],
        field: <option>,
        expected: "value",
    }

    crate::cli_limit_tests! {
        command: "<name>",
        variant: <Name>,
        required_args: ["--<required>", "value"],
        limit: { field: limit, default: 100, max: 1000 },
    }

    // =========================================================================
    // Edge case tests (require regular test syntax)
    // =========================================================================

    #[rstest]
    fn test_<edge_case>() {
        // Custom test logic
    }
}
```

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
- [ ] In `output.rs`, add tests for:
  - [ ] Empty result formatting
  - [ ] Single/multiple result formatting
  - [ ] JSON format validity
  - [ ] Toon format field presence
