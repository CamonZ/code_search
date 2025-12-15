use clap::Parser;

use code_search::cli::Args;
use code_search::commands::CommandRunner;
use code_search::commands::Command;
use code_search::db::DatabaseConfig;
use code_search::db::schema::get_current_version;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let config = DatabaseConfig::resolve()
        .map_err(|e| {
            eprintln!("Error: {}", e);
            eprintln!();
            eprintln!("Please create a .code_search.json file in the current directory.");
            eprintln!("Example for SQLite:");
            eprintln!(r#"  {{"database": {{"type": "sqlite", "path": "./cozo.sqlite"}}}}"#);
            eprintln!();
            eprintln!("Example for PostgreSQL:");
            eprintln!(r#"  {{"database": {{"type": "postgres", "connection_string": "postgres://user@host/db"}}}}"#);
            e
        })?;
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
