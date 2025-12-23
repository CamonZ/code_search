//! CLI parsing tests for path command using the test DSL.

#[cfg(test)]
mod tests {
    use crate::cli::Args;
    use clap::Parser;
    use rstest::rstest;

    // =========================================================================
    // Macro-generated tests (standard patterns)
    // =========================================================================

    // Path command has many required args, so we test them as edge cases below

    crate::cli_option_test! {
        command: "path",
        variant: Path,
        test_name: test_with_limit,
        args: [
            "--from-module", "MyApp",
            "--from-function", "foo",
            "--to-module", "MyApp",
            "--to-function", "bar",
            "--limit", "5"
        ],
        field: limit,
        expected: 5,
    }

    crate::cli_option_test! {
        command: "path",
        variant: Path,
        test_name: test_with_depth,
        args: [
            "--from-module", "MyApp",
            "--from-function", "foo",
            "--to-module", "MyApp",
            "--to-function", "bar",
            "--depth", "15"
        ],
        field: depth,
        expected: 15,
    }

    crate::cli_option_test! {
        command: "path",
        variant: Path,
        test_name: test_with_arities,
        args: [
            "--from-module", "MyApp.Controller",
            "--from-function", "index",
            "--from-arity", "2",
            "--to-module", "MyApp.Repo",
            "--to-function", "get",
            "--to-arity", "2"
        ],
        field: from_arity,
        expected: Some(2),
    }

    // =========================================================================
    // Edge case tests (multiple required args, depth validation)
    // =========================================================================

    #[rstest]
    fn test_requires_all_args() {
        let result = Args::try_parse_from(["code_search", "path"]);
        assert!(result.is_err());
    }

    #[rstest]
    fn test_requires_to_args() {
        let result = Args::try_parse_from([
            "code_search",
            "path",
            "--from-module",
            "MyApp.Controller",
            "--from-function",
            "index",
        ]);
        assert!(result.is_err());
    }

    #[rstest]
    fn test_with_all_required_args() {
        let args = Args::try_parse_from([
            "code_search",
            "path",
            "--from-module",
            "MyApp.Controller",
            "--from-function",
            "index",
            "--to-module",
            "MyApp.Repo",
            "--to-function",
            "get",
        ])
        .unwrap();
        match args.command {
            crate::commands::Command::Path(cmd) => {
                assert_eq!(cmd.from_module, "MyApp.Controller");
                assert_eq!(cmd.from_function, "index");
                assert_eq!(cmd.to_module, "MyApp.Repo");
                assert_eq!(cmd.to_function, "get");
                assert_eq!(cmd.depth, 10); // default
                assert_eq!(cmd.limit, 100); // default
            }
            _ => panic!("Expected Path command"),
        }
    }

    #[rstest]
    fn test_depth_zero_rejected() {
        let result = Args::try_parse_from([
            "code_search",
            "path",
            "--from-module",
            "MyApp",
            "--from-function",
            "foo",
            "--to-module",
            "MyApp",
            "--to-function",
            "bar",
            "--depth",
            "0",
        ]);
        assert!(result.is_err());
    }

    #[rstest]
    fn test_depth_exceeds_max_rejected() {
        let result = Args::try_parse_from([
            "code_search",
            "path",
            "--from-module",
            "MyApp",
            "--from-function",
            "foo",
            "--to-module",
            "MyApp",
            "--to-function",
            "bar",
            "--depth",
            "21",
        ]);
        assert!(result.is_err());
    }

    #[rstest]
    fn test_limit_zero_rejected() {
        let result = Args::try_parse_from([
            "code_search",
            "path",
            "--from-module",
            "MyApp",
            "--from-function",
            "foo",
            "--to-module",
            "MyApp",
            "--to-function",
            "bar",
            "--limit",
            "0",
        ]);
        assert!(result.is_err());
    }

    #[rstest]
    fn test_limit_exceeds_max_rejected() {
        let result = Args::try_parse_from([
            "code_search",
            "path",
            "--from-module",
            "MyApp",
            "--from-function",
            "foo",
            "--to-module",
            "MyApp",
            "--to-function",
            "bar",
            "--limit",
            "1001",
        ]);
        assert!(result.is_err());
    }
}
