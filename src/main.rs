use clap::Parser;

mod cli;
mod commands;

use cli::Args;

fn main() {
    let _args = Args::parse();
}
