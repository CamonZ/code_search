//! Acceptance tests for the code_search CLI.
//!
//! These tests exercise the full workflow: setup -> import -> query commands.
//! They use `assert_cmd` to run the actual binary and `predicates` for assertions.
//!
//! To test with SurrealDB backend:
//!   cargo test --test acceptance --features backend-surrealdb --no-default-features

use assert_cmd::cargo::CommandCargoExt;
use predicates::prelude::*;
use std::fs;
use std::path::PathBuf;
use std::process::Command as StdCommand;
use tempfile::TempDir;

/// Test harness for acceptance tests.
///
/// Creates a temporary directory with a database and fixture files,
/// providing methods to run CLI commands against them.
struct TestProject {
    dir: TempDir,
    db_path: PathBuf,
}

impl TestProject {
    /// Create a new test project with an empty database.
    fn new() -> Self {
        let dir = TempDir::new().expect("Failed to create temp dir");
        let db_path = dir.path().join("test.db");
        Self { dir, db_path }
    }

    /// Get a Command configured to run code_search with this project's database.
    fn cmd(&self) -> assert_cmd::Command {
        let mut cmd = assert_cmd::Command::from_std(
            StdCommand::cargo_bin("code_search").unwrap()
        );
        cmd.arg("--db").arg(&self.db_path);
        cmd
    }

    /// Run the setup command to initialize the database schema.
    fn setup(&self) -> &Self {
        self.cmd()
            .arg("setup")
            .assert()
            .success();
        self
    }

    /// Write fixture JSON to a file in the temp directory and return the path.
    fn write_fixture(&self, name: &str, content: &str) -> PathBuf {
        let path = self.dir.path().join(name);
        fs::write(&path, content).expect("Failed to write fixture");
        path
    }

    /// Import a fixture file into the database.
    fn import(&self, fixture_path: &PathBuf) -> &Self {
        self.cmd()
            .args(["import", "--file"])
            .arg(fixture_path)
            .assert()
            .success();
        self
    }
}

/// Sample call graph fixture for testing.
fn call_graph_fixture() -> &'static str {
    include_str!("../../db/src/fixtures/call_graph.json")
}

#[test]
fn test_setup_creates_database() {
    let project = TestProject::new();

    project.cmd()
        .arg("setup")
        .assert()
        .success()
        .stdout(predicate::str::contains("modules"))
        .stdout(predicate::str::contains("functions"))
        .stdout(predicate::str::contains("calls"));
}

#[test]
fn test_setup_is_idempotent() {
    let project = TestProject::new();

    // First setup
    project.cmd()
        .arg("setup")
        .assert()
        .success();

    // Second setup should also succeed
    project.cmd()
        .arg("setup")
        .assert()
        .success()
        .stdout(predicate::str::contains("exists"));
}

#[test]
fn test_full_workflow_setup_import_query() {
    let project = TestProject::new();

    // 1. Setup
    project.setup();

    // 2. Import fixture
    let fixture_path = project.write_fixture("call_graph.json", call_graph_fixture());
    project.import(&fixture_path);

    // 3. Query - search for modules (use regex for partial match)
    project.cmd()
        .args(["search", "--regex", ".*Controller.*"])
        .assert()
        .success()
        .stdout(predicate::str::contains("MyApp.Controller"));
}

#[test]
fn test_search_finds_modules() {
    let project = TestProject::new();
    project.setup();

    let fixture_path = project.write_fixture("call_graph.json", call_graph_fixture());
    project.import(&fixture_path);

    // Search for Accounts module (use regex for partial match)
    project.cmd()
        .args(["search", "--regex", ".*Accounts.*"])
        .assert()
        .success()
        .stdout(predicate::str::contains("MyApp.Accounts"));
}

#[test]
fn test_search_finds_functions() {
    let project = TestProject::new();
    project.setup();

    let fixture_path = project.write_fixture("call_graph.json", call_graph_fixture());
    project.import(&fixture_path);

    // Search for get_user function (use regex for partial match)
    project.cmd()
        .args(["search", "--regex", ".*get_user.*", "-k", "functions"])
        .assert()
        .success()
        .stdout(predicate::str::contains("get_user"));
}

#[test]
fn test_location_finds_function_definition() {
    let project = TestProject::new();
    project.setup();

    let fixture_path = project.write_fixture("call_graph.json", call_graph_fixture());
    project.import(&fixture_path);

    // Find location of get_user/1 (function first, then module)
    project.cmd()
        .args(["location", "get_user", "MyApp.Accounts", "--arity", "1"])
        .assert()
        .success()
        .stdout(predicate::str::contains("accounts.ex"))
        .stdout(predicate::str::contains("10")); // line number
}

#[test]
fn test_calls_from_shows_outgoing_calls() {
    let project = TestProject::new();
    project.setup();

    let fixture_path = project.write_fixture("call_graph.json", call_graph_fixture());
    project.import(&fixture_path);

    // Check what Controller.index calls (positional args: MODULE FUNCTION)
    project.cmd()
        .args(["calls-from", "MyApp.Controller", "index"])
        .assert()
        .success()
        .stdout(predicate::str::contains("list_users")); // calls Accounts.list_users
}

#[test]
fn test_calls_to_shows_incoming_calls() {
    let project = TestProject::new();
    project.setup();

    let fixture_path = project.write_fixture("call_graph.json", call_graph_fixture());
    project.import(&fixture_path);

    // Check what calls Repo.get (positional args: MODULE FUNCTION)
    project.cmd()
        .args(["calls-to", "MyApp.Repo", "get"])
        .assert()
        .success()
        .stdout(predicate::str::contains("get_user")); // Accounts.get_user calls it
}

#[test]
fn test_browse_module_lists_functions() {
    let project = TestProject::new();
    project.setup();

    let fixture_path = project.write_fixture("call_graph.json", call_graph_fixture());
    project.import(&fixture_path);

    // Browse MyApp.Accounts module
    project.cmd()
        .args(["browse-module", "MyApp.Accounts"])
        .assert()
        .success()
        .stdout(predicate::str::contains("get_user"))
        .stdout(predicate::str::contains("list_users"))
        .stdout(predicate::str::contains("validate_email"));
}

#[test]
fn test_json_output_format() {
    let project = TestProject::new();
    project.setup();

    let fixture_path = project.write_fixture("call_graph.json", call_graph_fixture());
    project.import(&fixture_path);

    // Get JSON output (use regex for partial match)
    project.cmd()
        .args(["--format", "json", "search", "--regex", ".*Controller.*"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"MyApp.Controller\""));
}

#[test]
fn test_import_with_clear_flag() {
    let project = TestProject::new();
    project.setup();

    let fixture_path = project.write_fixture("call_graph.json", call_graph_fixture());

    // First import
    project.import(&fixture_path);

    // Second import with --clear
    project.cmd()
        .args(["import", "--clear", "--file"])
        .arg(&fixture_path)
        .assert()
        .success();

    // Verify data is still there (use regex for partial match)
    project.cmd()
        .args(["search", "--regex", ".*Controller.*"])
        .assert()
        .success()
        .stdout(predicate::str::contains("MyApp.Controller"));
}

#[test]
fn test_hotspots_command() {
    let project = TestProject::new();
    project.setup();

    let fixture_path = project.write_fixture("call_graph.json", call_graph_fixture());
    project.import(&fixture_path);

    // Find hotspots (functions with most calls) - just verify command runs successfully
    project.cmd()
        .args(["hotspots", "--limit", "5"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Hotspots"));
}

#[test]
fn test_unused_command() {
    let project = TestProject::new();
    project.setup();

    let fixture_path = project.write_fixture("call_graph.json", call_graph_fixture());
    project.import(&fixture_path);

    // Find unused functions
    project.cmd()
        .args(["unused"])
        .assert()
        .success();
    // Repo functions are called but never call anything that's tracked as unused
}

#[test]
fn test_depends_on_shows_module_dependencies() {
    let project = TestProject::new();
    project.setup();

    let fixture_path = project.write_fixture("call_graph.json", call_graph_fixture());
    project.import(&fixture_path);

    // Check what MyApp.Controller depends on
    project.cmd()
        .args(["depends-on", "MyApp.Controller"])
        .assert()
        .success()
        .stdout(predicate::str::contains("MyApp.Accounts")); // Controller calls Accounts
}

#[test]
fn test_depended_by_shows_reverse_dependencies() {
    let project = TestProject::new();
    project.setup();

    let fixture_path = project.write_fixture("call_graph.json", call_graph_fixture());
    project.import(&fixture_path);

    // Check what depends on MyApp.Repo
    project.cmd()
        .args(["depended-by", "MyApp.Repo"])
        .assert()
        .success()
        .stdout(predicate::str::contains("MyApp.Accounts")); // Accounts calls Repo
}

#[test]
fn test_trace_command() {
    let project = TestProject::new();
    project.setup();

    let fixture_path = project.write_fixture("call_graph.json", call_graph_fixture());
    project.import(&fixture_path);

    // Trace from Controller.index
    project.cmd()
        .args(["trace", "MyApp.Controller", "index", "--depth", "2"])
        .assert()
        .success()
        .stdout(predicate::str::contains("list_users")); // direct call
}

#[test]
fn test_import_nonexistent_file_fails() {
    let project = TestProject::new();
    project.setup();

    project.cmd()
        .args(["import", "--file", "/nonexistent/file.json"])
        .assert()
        .failure();
}

#[test]
fn test_import_invalid_json_fails() {
    let project = TestProject::new();
    project.setup();

    let fixture_path = project.write_fixture("invalid.json", "{ not valid json }");

    project.cmd()
        .args(["import", "--file"])
        .arg(&fixture_path)
        .assert()
        .failure();
}
