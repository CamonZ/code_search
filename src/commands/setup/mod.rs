mod execute;
mod output;

use std::error::Error;
use clap::Args;
use cozo::DbInstance;

use crate::commands::{CommandRunner, Execute};
use crate::output::{OutputFormat, Outputable};

/// Create database schema without importing data
#[derive(Args, Debug)]
#[command(after_help = "\
Examples:
  code_search setup --db ./my_project.db      # Create schema
  code_search setup --db ./my_project.db --force  # Drop and recreate
  code_search setup --db ./my_project.db --dry-run  # Show what would be created
  code_search setup --db ./my_project.db --install-skills  # Create schema and install skill templates
  code_search setup --db ./my_project.db --install-skills --force  # Overwrite existing skill files")]
pub struct SetupCmd {
    /// Drop existing schema and recreate
    #[arg(long, default_value_t = false)]
    pub force: bool,

    /// Show what would be created without doing it
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,

    /// Install skill templates to .claude/skills/
    #[arg(long, default_value_t = false)]
    pub install_skills: bool,
}

impl CommandRunner for SetupCmd {
    fn run(self, db: &DbInstance, format: OutputFormat) -> Result<String, Box<dyn Error>> {
        let result = self.execute(db)?;
        Ok(result.format(format))
    }
}
