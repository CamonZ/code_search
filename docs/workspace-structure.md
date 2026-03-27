# Workspace Structure

This is a Cargo workspace with two crates:

- **`db/`** - Database library crate
  - SurrealDB query layer (all `queries/` modules)
  - Database utilities (`db.rs`)
  - Shared types (`types/`)
  - Query builders (`query_builders.rs`)
  - Test utilities and fixtures (behind `test-utils` feature flag)

- **`cli/`** - CLI binary crate (package name: `code_search`)
  - Command-line interface (`cli.rs`, `main.rs`)
  - All command modules (`commands/`)
  - Output formatting (`output.rs`)
  - Presentation utilities (`utils.rs`, `dedup.rs`)
  - Test macros (`test_macros.rs`)

**Dependency flow:** `cli` depends on `db` via `db = { path = "../db" }`. The database layer is completely independent of the CLI.

**Test utilities:** Database test helpers and fixtures are available via the `test-utils` feature. CLI tests use: `db = { path = "../db", features = ["test-utils"] }`
