use clap::Parser;

mod cli;
mod commands;
mod dedup;
pub mod output;
mod utils;
#[macro_use]
mod test_macros;
use cli::Args;
use commands::CommandRunner;
use db::open_db;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let db_path = cli::resolve_db_path(args.db);

    // Create .code_search directory if using default path
    if db_path.as_path() == std::path::Path::new(".code_search/cozo.sqlite") {
        std::fs::create_dir_all(".code_search").ok();
    }

    let db = open_db(&db_path)?;
    let output = args.command.run(&db, args.format)?;
    println!("{}", output);
    Ok(())
}
