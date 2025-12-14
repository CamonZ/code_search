//! CLI parsing tests for import command.
//!
//! Note: Import command has special file existence validation that requires
//! fixtures with temp files, so these tests remain as regular tests.

#[cfg(test)]
mod tests {
    use crate::cli::Args;
    use clap::Parser;
    use rstest::{fixture, rstest};
    use std::fs::File;
    use std::path::PathBuf;
    use tempfile::{tempdir, TempDir};

    #[fixture]
    fn temp_file() -> (TempDir, PathBuf) {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.json");
        File::create(&path).unwrap();
        (dir, path)
    }

    // =========================================================================
    // Macro-generated tests (standard patterns)
    // =========================================================================

    crate::cli_required_arg_test! {
        command: "import",
        test_name: test_requires_file,
        required_arg: "--file",
    }

    // =========================================================================
    // Edge case tests (file validation)
    // =========================================================================

    #[rstest]
    fn test_file_must_exist() {
        let result =
            Args::try_parse_from(["code_search", "import", "--file", "nonexistent_file.json"]);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("File not found"));
    }

    #[rstest]
    fn test_with_existing_file(temp_file: (TempDir, PathBuf)) {
        let (_dir, path) = temp_file;
        let result =
            Args::try_parse_from(["code_search", "import", "--file", path.to_str().unwrap()]);
        assert!(result.is_ok());
    }
}
