mod cli_tests;
mod execute;
mod execute_tests;
mod output;
mod output_tests;

use std::error::Error;

use clap::Args;
use db::backend::Database;

use crate::commands::{CommandRunner, Execute};
use crate::output::{OutputFormat, Outputable};

/// Find a call path between two functions
#[derive(Args, Debug)]
#[command(after_help = "\
Examples:
  code_search path --from-module MyApp.Web --from-function index \\
                   --to-module MyApp.Repo --to-function get
  code_search path --from-module MyApp.API --from-function create \\
                   --to-module Ecto.Repo --to-function insert --depth 15")]
pub struct PathCmd {
    /// Source module name
    #[arg(long)]
    pub from_module: String,

    /// Source function name
    #[arg(long)]
    pub from_function: String,

    /// Source function arity (optional)
    #[arg(long)]
    pub from_arity: Option<i64>,

    /// Target module name
    #[arg(long)]
    pub to_module: String,

    /// Target function name
    #[arg(long)]
    pub to_function: String,

    /// Target function arity (optional)
    #[arg(long)]
    pub to_arity: Option<i64>,

    /// Project to search in
    #[arg(long, default_value = "default")]
    pub project: String,

    /// Maximum depth to search (1-20)
    #[arg(long, default_value_t = 10, value_parser = clap::value_parser!(u32).range(1..=20))]
    pub depth: u32,

    /// Maximum number of paths to return (1-1000)
    #[arg(short, long, default_value_t = 100, value_parser = clap::value_parser!(u32).range(1..=1000))]
    pub limit: u32,
}

impl CommandRunner for PathCmd {
    fn run(self, db: &dyn Database, format: OutputFormat) -> Result<String, Box<dyn Error>> {
        let result = self.execute(db)?;
        Ok(result.format(format))
    }
}
