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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let db_path = cli::resolve_db_path(args.db);

    // Create .code_search directory if using default path
    if db_path == std::path::PathBuf::from(".code_search/cozo.sqlite") {
        std::fs::create_dir_all(".code_search").ok();
    }

    let db = db::open_db(&db_path)?;
    let output = args.command.run(&db, args.format)?;
    println!("{}", output);
    Ok(())
}
