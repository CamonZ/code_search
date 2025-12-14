use clap::Parser;

mod cli;
mod commands;
mod config;
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
use commands::Command;
use db::DatabaseConfig;
use db::schema::get_current_version;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let config = DatabaseConfig::resolve(&args.db)?;
    let backend = config.connect()?;

    // Check if database is initialized, unless running setup
    if !matches!(args.command, Command::Setup(_)) {
        let version = get_current_version(backend.as_ref())?;
        if version == 0 {
            return Err(
                "Database not initialized. Please run 'code_search setup' first to create the schema."
                    .into()
            );
        }
    }

    let output = args.command.run(backend.as_ref(), args.format)?;
    println!("{}", output);
    Ok(())
}
