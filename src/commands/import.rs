use std::path::PathBuf;

use clap::Args;

fn validate_file_exists(s: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(s);
    if path.exists() {
        Ok(path)
    } else {
        Err(format!("File not found: {}", path.display()))
    }
}

#[derive(Args, Debug)]
pub struct ImportCmd {
    /// Path to the call graph JSON file
    #[arg(short, long, value_parser = validate_file_exists)]
    pub file: PathBuf,
    /// Project name for namespacing (allows multiple projects in same DB)
    #[arg(short, long)]
    pub project: Option<String>,
    /// Clear all existing data before import (or just project data if --project is set)
    #[arg(long)]
    pub clear: bool,
    /// Sync mode: remove stale entries not in the new import (requires --project)
    #[arg(long, requires = "project")]
    pub sync: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::Args;
    use clap::Parser;
    use rstest::{fixture, rstest};
    use std::fs::File;
    use tempfile::{tempdir, TempDir};

    #[fixture]
    fn temp_file() -> (TempDir, PathBuf) {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.json");
        File::create(&path).unwrap();
        (dir, path)
    }

    #[rstest]
    fn test_import_requires_file() {
        let result = Args::try_parse_from(["code_search", "import"]);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("--file"));
    }

    #[rstest]
    fn test_import_file_must_exist() {
        let result =
            Args::try_parse_from(["code_search", "import", "--file", "nonexistent_file.json"]);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("File not found"));
    }

    #[rstest]
    fn test_import_with_existing_file(temp_file: (TempDir, PathBuf)) {
        let (_dir, path) = temp_file;
        let result = Args::try_parse_from([
            "code_search",
            "import",
            "--file",
            path.to_str().unwrap(),
        ]);
        assert!(result.is_ok());
    }

    #[rstest]
    fn test_import_sync_requires_project(temp_file: (TempDir, PathBuf)) {
        let (_dir, path) = temp_file;
        let result = Args::try_parse_from([
            "code_search",
            "import",
            "--file",
            path.to_str().unwrap(),
            "--sync",
        ]);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("--project"));
    }

    #[rstest]
    fn test_import_sync_with_project_succeeds(temp_file: (TempDir, PathBuf)) {
        let (_dir, path) = temp_file;
        let result = Args::try_parse_from([
            "code_search",
            "import",
            "--file",
            path.to_str().unwrap(),
            "--sync",
            "--project",
            "myproject",
        ]);
        assert!(result.is_ok());
    }

    #[rstest]
    fn test_db_has_default_value(temp_file: (TempDir, PathBuf)) {
        let (_dir, path) = temp_file;
        let args = Args::try_parse_from([
            "code_search",
            "import",
            "--file",
            path.to_str().unwrap(),
        ])
        .unwrap();
        assert_eq!(args.db, PathBuf::from("./cozo.sqlite"));
    }

    #[rstest]
    fn test_db_can_be_overridden(temp_file: (TempDir, PathBuf)) {
        let (_dir, path) = temp_file;
        let args = Args::try_parse_from([
            "code_search",
            "--db",
            "/custom/path.db",
            "import",
            "--file",
            path.to_str().unwrap(),
        ])
        .unwrap();
        assert_eq!(args.db, PathBuf::from("/custom/path.db"));
    }
}
