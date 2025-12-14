use clap::Parser;

mod cli;
mod commands;
mod db;
mod dedup;
pub mod output;
mod queries;
pub mod types;
mod utils;
#[macro_use]
mod test_macros;
#[cfg(test)]
pub mod fixtures;
#[cfg(test)]
pub mod test_utils;
use cli::Args;
use commands::CommandRunner;
use db::DatabaseConfig;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let config = DatabaseConfig::resolve(&args.db)?;
    let backend = config.connect()?;
    let db = backend.as_db_instance();
    let output = args.command.run(db, args.format)?;
    println!("{}", output);
    Ok(())
}
