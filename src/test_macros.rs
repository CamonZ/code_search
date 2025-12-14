//! Declarative macros for generating CLI parsing tests.
//!
//! This module provides macros to reduce boilerplate in CLI argument parsing tests.
//! Instead of writing repetitive test functions, you can declare the test cases
//! and let the macro generate the actual test code.

/// Generate a test for default values when a command is invoked with minimal args.
#[macro_export]
macro_rules! cli_defaults_test {
    (
        command: $cmd:literal,
        variant: $variant:ident,
        required_args: [$($req_arg:literal),*],
        defaults: {
            $($($def_field:ident).+ : $def_expected:expr),* $(,)?
        } $(,)?
    ) => {
        #[rstest]
        fn test_defaults() {
            let args = Args::try_parse_from(["code_search", $cmd, $($req_arg),*]).unwrap();
            match args.command {
                crate::commands::Command::$variant(cmd) => {
                    $(
                        assert_eq!(cmd.$($def_field).+, $def_expected,
                            concat!("Default value mismatch for field: ", stringify!($($def_field).+)));
                    )*
                }
                _ => panic!(concat!("Expected ", stringify!($variant), " command")),
            }
        }
    };
}

/// Generate a single CLI option test.
#[macro_export]
macro_rules! cli_option_test {
    (
        command: $cmd:literal,
        variant: $variant:ident,
        test_name: $test_name:ident,
        args: [$($arg:literal),+],
        field: $($field:ident).+,
        expected: $expected:expr $(,)?
    ) => {
        #[rstest]
        fn $test_name() {
            let args = Args::try_parse_from([
                "code_search",
                $cmd,
                $($arg),+
            ]).unwrap();
            match args.command {
                crate::commands::Command::$variant(cmd) => {
                    assert_eq!(cmd.$($field).+, $expected,
                        concat!("Field ", stringify!($($field).+), " mismatch"));
                }
                _ => panic!(concat!("Expected ", stringify!($variant), " command")),
            }
        }
    };
}

/// Generate a single CLI option test with required args.
#[macro_export]
macro_rules! cli_option_test_with_required {
    (
        command: $cmd:literal,
        variant: $variant:ident,
        required_args: [$($req_arg:literal),+],
        test_name: $test_name:ident,
        args: [$($arg:literal),+],
        field: $($field:ident).+,
        expected: $expected:expr $(,)?
    ) => {
        #[rstest]
        fn $test_name() {
            let args = Args::try_parse_from([
                "code_search",
                $cmd,
                $($req_arg,)+
                $($arg),+
            ]).unwrap();
            match args.command {
                crate::commands::Command::$variant(cmd) => {
                    assert_eq!(cmd.$($field).+, $expected,
                        concat!("Field ", stringify!($($field).+), " mismatch"));
                }
                _ => panic!(concat!("Expected ", stringify!($variant), " command")),
            }
        }
    };
}

/// Generate limit validation tests (zero rejected, max exceeded rejected, default value).
#[macro_export]
macro_rules! cli_limit_tests {
    (
        command: $cmd:literal,
        variant: $variant:ident,
        required_args: [$($req_arg:literal),*],
        limit: {
            field: $($limit_field:ident).+,
            default: $limit_default:expr,
            max: $limit_max:expr $(,)?
        } $(,)?
    ) => {
        #[rstest]
        fn test_limit_default() {
            let args = Args::try_parse_from(["code_search", $cmd, $($req_arg),*]).unwrap();
            match args.command {
                crate::commands::Command::$variant(cmd) => {
                    assert_eq!(cmd.$($limit_field).+, $limit_default);
                }
                _ => panic!(concat!("Expected ", stringify!($variant), " command")),
            }
        }

        #[rstest]
        fn test_limit_zero_rejected() {
            let result = Args::try_parse_from([
                "code_search",
                $cmd,
                $($req_arg,)*
                "--limit",
                "0"
            ]);
            assert!(result.is_err(), "Limit of 0 should be rejected");
        }

        #[rstest]
        fn test_limit_exceeds_max_rejected() {
            let max_plus_one = ($limit_max + 1).to_string();
            let result = Args::try_parse_from([
                "code_search",
                $cmd,
                $($req_arg,)*
                "--limit",
                &max_plus_one
            ]);
            assert!(result.is_err(),
                concat!("Limit exceeding ", stringify!($limit_max), " should be rejected"));
        }
    };
}

/// Generate a test that verifies a command requires a specific argument.
///
/// # Example
///
/// ```ignore
/// cli_required_arg_test! {
///     command: "search",
///     test_name: test_requires_pattern,
///     required_arg: "--pattern",
/// }
/// ```
#[macro_export]
macro_rules! cli_required_arg_test {
    (
        command: $cmd:literal,
        test_name: $test_name:ident,
        required_arg: $arg:literal $(,)?
    ) => {
        #[rstest]
        fn $test_name() {
            let result = Args::try_parse_from(["code_search", $cmd]);
            assert!(result.is_err(), concat!("Command should require ", $arg));
            assert!(
                result.unwrap_err().to_string().contains($arg),
                concat!("Error should mention ", $arg)
            );
        }
    };
}

/// Generate a test that verifies parsing fails with specific invalid args.
///
/// # Example
///
/// ```ignore
/// cli_error_test! {
///     command: "search",
///     test_name: test_limit_zero_rejected,
///     args: ["--pattern", "test", "--limit", "0"],
/// }
/// ```
#[macro_export]
macro_rules! cli_error_test {
    (
        command: $cmd:literal,
        test_name: $test_name:ident,
        args: [$($arg:literal),+] $(,)?
    ) => {
        #[rstest]
        fn $test_name() {
            let result = Args::try_parse_from([
                "code_search",
                $cmd,
                $($arg),+
            ]);
            assert!(result.is_err());
        }
    };
}

// =============================================================================
// Execute Test Macros
// =============================================================================

/// Generate a fixture that creates a populated test database.
///
/// This creates the standard `populated_db` fixture used by execute tests.
/// For inline JSON content.
#[macro_export]
macro_rules! execute_test_fixture {
    (
        fixture_name: $name:ident,
        json: $json:expr,
        project: $project:literal $(,)?
    ) => {
        #[fixture]
        fn $name() -> Box<dyn crate::db::DatabaseBackend> {
            crate::test_utils::setup_test_db($json, $project)
        }
    };
}

/// Generate a fixture using a shared fixture file.
///
/// Available fixtures: `call_graph`, `type_signatures`, `structs`
///
/// # Example
/// ```ignore
/// crate::shared_fixture! {
///     fixture_name: populated_db,
///     fixture_type: call_graph,
///     project: "test_project",
/// }
/// ```
#[macro_export]
macro_rules! shared_fixture {
    (
        fixture_name: $name:ident,
        fixture_type: call_graph,
        project: $project:literal $(,)?
    ) => {
        #[fixture]
        fn $name() -> Box<dyn crate::db::DatabaseBackend> {
            crate::test_utils::call_graph_db($project)
        }
    };
    (
        fixture_name: $name:ident,
        fixture_type: type_signatures,
        project: $project:literal $(,)?
    ) => {
        #[fixture]
        fn $name() -> Box<dyn crate::db::DatabaseBackend> {
            crate::test_utils::type_signatures_db($project)
        }
    };
    (
        fixture_name: $name:ident,
        fixture_type: structs,
        project: $project:literal $(,)?
    ) => {
        #[fixture]
        fn $name() -> Box<dyn crate::db::DatabaseBackend> {
            crate::test_utils::structs_db($project)
        }
    };
}

/// Generate a test that verifies command execution against an empty database fails.
#[macro_export]
macro_rules! execute_empty_db_test {
    (
        cmd_type: $cmd_type:ty,
        cmd: $cmd:expr $(,)?
    ) => {
        #[rstest]
        fn test_empty_db() {
            let result = crate::test_utils::execute_on_empty_db($cmd);
            assert!(result.is_err());
        }
    };
}

/// Generate an execute test with custom assertions.
///
/// This is the core macro for execute tests. It handles the boilerplate of
/// executing a command against a fixture and lets you write custom assertions.
///
/// # Example
/// ```ignore
/// execute_test! {
///     test_name: test_search_finds_modules,
///     fixture: populated_db,
///     cmd: SearchCmd {
///         pattern: "MyApp".to_string(),
///         kind: SearchKind::Modules,
///         project: "test_project".to_string(),
///         limit: 100,
///         regex: false,
///     },
///     assertions: |result| {
///         assert_eq!(result.modules.len(), 2);
///         assert_eq!(result.kind, "modules");
///     },
/// }
/// ```
#[macro_export]
macro_rules! execute_test {
    (
        test_name: $test_name:ident,
        fixture: $fixture:ident,
        cmd: $cmd:expr,
        assertions: |$result:ident| $assertions:expr $(,)?
    ) => {
        #[rstest]
        fn $test_name($fixture: Box<dyn crate::db::DatabaseBackend>) {
            use crate::commands::Execute;
            let $result = $cmd.execute($fixture.as_ref()).expect("Execute should succeed");
            $assertions
        }
    };
}

/// Generate a test that verifies command returns empty results for no match.
///
/// # Example
/// ```ignore
/// execute_no_match_test! {
///     test_name: test_search_no_match,
///     fixture: populated_db,
///     cmd: SearchCmd { pattern: "NonExistent".into(), ... },
///     empty_field: modules,
/// }
/// ```
#[macro_export]
macro_rules! execute_no_match_test {
    (
        test_name: $test_name:ident,
        fixture: $fixture:ident,
        cmd: $cmd:expr,
        empty_field: $field:ident $(,)?
    ) => {
        #[rstest]
        fn $test_name($fixture: Box<dyn crate::db::DatabaseBackend>) {
            use crate::commands::Execute;
            let result = $cmd.execute($fixture.as_ref()).expect("Execute should succeed");
            assert!(result.$field.is_empty(), concat!(stringify!($field), " should be empty"));
        }
    };
}

/// Generate a test that verifies result count.
///
/// # Example
/// ```ignore
/// execute_count_test! {
///     test_name: test_search_finds_two,
///     fixture: populated_db,
///     cmd: SearchCmd { ... },
///     field: modules,
///     expected: 2,
/// }
/// ```
#[macro_export]
macro_rules! execute_count_test {
    (
        test_name: $test_name:ident,
        fixture: $fixture:ident,
        cmd: $cmd:expr,
        field: $field:ident,
        expected: $expected:expr $(,)?
    ) => {
        #[rstest]
        fn $test_name($fixture: Box<dyn crate::db::DatabaseBackend>) {
            use crate::commands::Execute;
            let result = $cmd.execute($fixture.as_ref()).expect("Execute should succeed");
            assert_eq!(result.$field.len(), $expected,
                concat!("Expected ", stringify!($expected), " ", stringify!($field)));
        }
    };
}

/// Generate a test that verifies a field value on the result.
///
/// # Example
/// ```ignore
/// execute_field_test! {
///     test_name: test_search_kind,
///     fixture: populated_db,
///     cmd: SearchCmd { kind: SearchKind::Modules, ... },
///     field: kind,
///     expected: "modules",
/// }
/// ```
#[macro_export]
macro_rules! execute_field_test {
    (
        test_name: $test_name:ident,
        fixture: $fixture:ident,
        cmd: $cmd:expr,
        field: $field:ident,
        expected: $expected:expr $(,)?
    ) => {
        #[rstest]
        fn $test_name($fixture: Box<dyn crate::db::DatabaseBackend>) {
            use crate::commands::Execute;
            let result = $cmd.execute($fixture.as_ref()).expect("Execute should succeed");
            assert_eq!(result.$field, $expected,
                concat!("Field ", stringify!($field), " mismatch"));
        }
    };
}

/// Generate a test that verifies a field on the first result item.
///
/// # Example
/// ```ignore
/// execute_first_item_test! {
///     test_name: test_first_function_name,
///     fixture: populated_db,
///     cmd: SearchCmd { ... },
///     collection: functions,
///     field: name,
///     expected: "get_user",
/// }
/// ```
#[macro_export]
macro_rules! execute_first_item_test {
    (
        test_name: $test_name:ident,
        fixture: $fixture:ident,
        cmd: $cmd:expr,
        collection: $collection:ident,
        field: $field:ident,
        expected: $expected:expr $(,)?
    ) => {
        #[rstest]
        fn $test_name($fixture: Box<dyn crate::db::DatabaseBackend>) {
            use crate::commands::Execute;
            let result = $cmd.execute($fixture.as_ref()).expect("Execute should succeed");
            assert!(!result.$collection.is_empty(), concat!(stringify!($collection), " should not be empty"));
            assert_eq!(result.$collection[0].$field, $expected,
                concat!("First item ", stringify!($field), " mismatch"));
        }
    };
}

/// Generate a test that verifies all items match a condition.
///
/// # Example
/// ```ignore
/// execute_all_match_test! {
///     test_name: test_all_from_project,
///     fixture: populated_db,
///     cmd: SearchCmd { project: "test_project".into(), ... },
///     collection: modules,
///     condition: |item| item.project == "test_project",
/// }
/// ```
#[macro_export]
macro_rules! execute_all_match_test {
    (
        test_name: $test_name:ident,
        fixture: $fixture:ident,
        cmd: $cmd:expr,
        collection: $collection:ident,
        condition: |$item:ident| $cond:expr $(,)?
    ) => {
        #[rstest]
        fn $test_name($fixture: Box<dyn crate::db::DatabaseBackend>) {
            use crate::commands::Execute;
            let result = $cmd.execute($fixture.as_ref()).expect("Execute should succeed");
            assert!(result.$collection.iter().all(|$item| $cond),
                concat!("Not all ", stringify!($collection), " matched condition"));
        }
    };
}

/// Generate a test that verifies limit is respected.
///
/// # Example
/// ```ignore
/// execute_limit_test! {
///     test_name: test_respects_limit,
///     fixture: populated_db,
///     cmd: SearchCmd { limit: 1, ... },
///     collection: modules,
///     limit: 1,
/// }
/// ```
#[macro_export]
macro_rules! execute_limit_test {
    (
        test_name: $test_name:ident,
        fixture: $fixture:ident,
        cmd: $cmd:expr,
        collection: $collection:ident,
        limit: $limit:expr $(,)?
    ) => {
        #[rstest]
        fn $test_name($fixture: Box<dyn crate::db::DatabaseBackend>) {
            use crate::commands::Execute;
            let result = $cmd.execute($fixture.as_ref()).expect("Execute should succeed");
            assert!(result.$collection.len() <= $limit,
                concat!("Expected at most ", stringify!($limit), " ", stringify!($collection)));
        }
    };
}

// =============================================================================
// Output Test Macros
// =============================================================================

/// Generate a test that verifies table output matches expected string.
///
/// Works with rstest fixtures by accepting a fixture parameter.
///
/// # Example
/// ```ignore
/// output_table_test! {
///     test_name: test_to_table_empty,
///     fixture: empty_result,
///     fixture_type: SearchResult,
///     expected: EMPTY_TABLE_OUTPUT,
/// }
/// ```
#[macro_export]
macro_rules! output_table_test {
    // With format parameter (Json, Toon)
    (
        test_name: $test_name:ident,
        fixture: $fixture:ident,
        fixture_type: $fixture_type:ty,
        expected: $expected:expr,
        format: $format:ident $(,)?
    ) => {
        #[rstest]
        fn $test_name($fixture: $fixture_type) {
            use crate::output::{Outputable, OutputFormat};
            assert_eq!($fixture.format(OutputFormat::$format), $expected);
        }
    };
    // Default table format
    (
        test_name: $test_name:ident,
        fixture: $fixture:ident,
        fixture_type: $fixture_type:ty,
        expected: $expected:expr $(,)?
    ) => {
        #[rstest]
        fn $test_name($fixture: $fixture_type) {
            use crate::output::Outputable;
            assert_eq!($fixture.to_table(), $expected);
        }
    };
}

/// Generate a test that verifies table output contains expected strings.
///
/// Use this when exact string matching is too brittle.
#[macro_export]
macro_rules! output_table_contains_test {
    (
        test_name: $test_name:ident,
        fixture: $fixture:ident,
        fixture_type: $fixture_type:ty,
        contains: [$($needle:literal),* $(,)?] $(,)?
    ) => {
        #[rstest]
        fn $test_name($fixture: $fixture_type) {
            use crate::output::Outputable;
            let output = $fixture.to_table();
            $(
                assert!(output.contains($needle), concat!("Table output should contain: ", $needle));
            )*
        }
    };
}

/// Generate a test that verifies JSON output is valid and contains expected fields.
///
/// # Example
/// ```ignore
/// output_json_test! {
///     test_name: test_format_json,
///     fixture: single_result,
///     fixture_type: SearchResult,
///     assertions: {
///         "pattern": "MyApp",
///         "modules".len(): 2,
///     },
/// }
/// ```
#[macro_export]
macro_rules! output_json_test {
    (
        test_name: $test_name:ident,
        fixture: $fixture:ident,
        fixture_type: $fixture_type:ty,
        assertions: { $($field:literal : $expected:expr),* $(,)? } $(,)?
    ) => {
        #[rstest]
        fn $test_name($fixture: $fixture_type) {
            use crate::output::{Outputable, OutputFormat};
            let output = $fixture.format(OutputFormat::Json);
            let parsed: serde_json::Value = serde_json::from_str(&output)
                .expect("Should produce valid JSON");
            $(
                assert_eq!(parsed[$field], $expected, concat!("JSON field mismatch: ", $field));
            )*
        }
    };
}

/// Generate a test that verifies Toon output contains expected strings.
///
/// # Example
/// ```ignore
/// output_toon_test! {
///     test_name: test_format_toon,
///     fixture: single_result,
///     fixture_type: SearchResult,
///     contains: ["pattern: MyApp", "modules["],
/// }
/// ```
#[macro_export]
macro_rules! output_toon_test {
    (
        test_name: $test_name:ident,
        fixture: $fixture:ident,
        fixture_type: $fixture_type:ty,
        contains: [$($needle:literal),* $(,)?] $(,)?
    ) => {
        #[rstest]
        fn $test_name($fixture: $fixture_type) {
            use crate::output::{Outputable, OutputFormat};
            let output = $fixture.format(OutputFormat::Toon);
            $(
                assert!(output.contains($needle), concat!("Toon output should contain: ", $needle));
            )*
        }
    };
}

#[cfg(test)]
mod tests {
    //! Tests for the test macros themselves.
    //!
    //! These verify that the macros compile and generate working tests.

    // We can't easily test macros here since they generate test functions,
    // but we can at least verify they compile by using them in actual test modules.
}
