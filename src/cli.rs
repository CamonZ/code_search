//! CLI argument definitions.
//!
//! This module contains the top-level CLI structure and shared types.
//! Individual command definitions are in the `commands` module.

use clap::Parser;
use std::path::PathBuf;

use crate::commands::Command;
use crate::output::OutputFormat;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Path to the CozoDB SQLite database file
    ///
    /// If not specified, searches for database in:
    ///   1. .code_search/cozo.sqlite (project-local)
    ///   2. ./cozo.sqlite (current directory)
    ///   3. ~/.code_search/cozo.sqlite (user-global)
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

    // 1. Check .code_search/cozo.sqlite (project-local)
    let project_db = PathBuf::from(".code_search/cozo.sqlite");
    if project_db.exists() {
        return project_db;
    }

    // 2. Check ./cozo.sqlite (current directory)
    let local_db = PathBuf::from("./cozo.sqlite");
    if local_db.exists() {
        return local_db;
    }

    // 3. Check ~/.code_search/cozo.sqlite (user-global)
    if let Some(home_dir) = home::home_dir() {
        let global_db = home_dir.join(".code_search/cozo.sqlite");
        if global_db.exists() {
            return global_db;
        }
    }

    // Default: .code_search/cozo.sqlite (will be created if needed)
    project_db
}
