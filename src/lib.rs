//! code_search library - Call graph analysis tool
//!
//! Provides the core database backend, command execution, and output formatting
//! infrastructure for analyzing Elixir project call graphs.

pub mod cli;
pub mod commands;
pub mod config;
pub mod db;
pub mod dedup;
pub mod output;
pub mod queries;
pub mod types;
pub mod utils;

#[macro_use]
pub mod test_macros;

#[cfg(test)]
pub mod fixtures;

#[cfg(test)]
pub mod test_utils;
