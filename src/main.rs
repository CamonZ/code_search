use clap::Parser;

mod cli;
mod commands;
mod db;
pub mod output;
#[macro_use]
mod test_macros;
use cli::Args;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let output = args.command.run(&args.db, args.format)?;
    println!("{}", output);
    Ok(())
}
