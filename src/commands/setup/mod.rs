mod execute;
mod output;

use std::error::Error;
use clap::Args;

use crate::commands::{CommandRunner, Execute};
use crate::db::DatabaseBackend;
use crate::output::{OutputFormat, Outputable};

/// Create database schema without importing data
#[derive(Args, Debug)]
#[command(after_help = "\
Examples:
  code_search setup                              # Setup schema only
  code_search setup --init-config sqlite         # Initialize SQLite config
  code_search setup --init-config postgres \\
    --pg-host localhost --pg-user myuser \\
    --pg-database mydb                           # Initialize Postgres config
  code_search setup --dry-run                    # Preview what would happen")]
pub struct SetupCmd {
    /// Drop existing schema and recreate
    #[arg(long, default_value_t = false)]
    pub force: bool,

    /// Show what would be created without doing it
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,

    /// Initialize .code_search.json config file
    /// Values: "sqlite", "memory", "postgres"
    #[arg(long, value_name = "TYPE")]
    pub init_config: Option<String>,

    /// For postgres: database host
    #[arg(long, requires = "init_config")]
    pub pg_host: Option<String>,

    /// For postgres: database port (default: 5432)
    #[arg(long)]
    pub pg_port: Option<u16>,

    /// For postgres: database username
    #[arg(long, requires = "init_config")]
    pub pg_user: Option<String>,

    /// For postgres: database password (optional)
    #[arg(long)]
    pub pg_password: Option<String>,

    /// For postgres: database name
    #[arg(long, requires = "init_config")]
    pub pg_database: Option<String>,

    /// For postgres: enable SSL
    #[arg(long, default_value_t = false)]
    pub pg_ssl: bool,

    /// For postgres: AGE graph name (default: "call_graph")
    #[arg(long, default_value = "call_graph")]
    pub pg_graph: String,

    /// SQLite: path to database file (default: "./cozo.sqlite")
    #[arg(long, default_value = "./cozo.sqlite")]
    pub sqlite_path: String,
}

impl CommandRunner for SetupCmd {
    fn run(self, db: &dyn DatabaseBackend, format: OutputFormat) -> Result<String, Box<dyn Error>> {
        let result = self.execute(db)?;
        Ok(result.format(format))
    }
}
