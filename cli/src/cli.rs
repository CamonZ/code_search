//! CLI argument definitions.
//!
//! This module contains the top-level CLI structure and shared types.
//! Individual command definitions are in the `commands` module.

use clap::Parser;
use std::path::PathBuf;

use crate::commands::Command;
use crate::output::OutputFormat;

/// Database filename based on backend
#[cfg(feature = "backend-surrealdb")]
pub const DB_FILENAME: &str = "surrealdb.rocksdb";

#[cfg(not(feature = "backend-surrealdb"))]
pub const DB_FILENAME: &str = "cozo.sqlite";

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Path to the database file
    ///
    /// If not specified, searches for database in:
    ///   1. .code_search/<db_file> (project-local)
    ///   2. ./<db_file> (current directory)
    ///   3. ~/.code_search/<db_file> (user-global)
    #[arg(long, global = true)]
    pub db: Option<PathBuf>,

    /// Output format
    #[arg(short = 'o', long, value_enum, default_value_t = OutputFormat::Table, global = true)]
    pub format: OutputFormat,

    #[command(subcommand)]
    pub command: Command,
}

/// Resolve database path by checking multiple locations in order of preference
pub fn resolve_db_path(explicit_path: Option<PathBuf>) -> PathBuf {
    // If explicitly specified, use that
    if let Some(path) = explicit_path {
        return path;
    }

    // 1. Check .code_search/<db_file> (project-local)
    let project_db = PathBuf::from(format!(".code_search/{}", DB_FILENAME));
    if project_db.exists() {
        return project_db;
    }

    // 2. Check ./<db_file> (current directory)
    let local_db = PathBuf::from(format!("./{}", DB_FILENAME));
    if local_db.exists() {
        return local_db;
    }

    // 3. Check ~/.code_search/<db_file> (user-global)
    if let Some(home_dir) = home::home_dir() {
        let global_db = home_dir.join(format!(".code_search/{}", DB_FILENAME));
        if global_db.exists() {
            return global_db;
        }
    }

    // Default: .code_search/<db_file> (will be created if needed)
    project_db
}
