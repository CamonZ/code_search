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
  code_search setup                           # Create schema in .code_search/cozo.sqlite
  code_search setup --force                   # Overwrite existing templates/hooks
  code_search setup --dry-run                 # Show what would be created
  code_search setup --install-skills          # Create schema and install skill templates
  code_search setup --install-skills --force  # Overwrite existing skill files
  code_search setup --install-hooks           # Install git hooks for incremental updates
  code_search setup --install-hooks --project-name my_app  # Configure project name
  code_search setup --install-skills --install-hooks      # Install both skills and hooks")]
pub struct SetupCmd {
    /// Overwrite existing template and hook files (does not affect schema)
    #[arg(long, default_value_t = false)]
    pub force: bool,

    /// Show what would be created without doing it
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,

    /// Install skill templates to .claude/skills/
    #[arg(long, default_value_t = false)]
    pub install_skills: bool,

    /// Install git hooks for incremental database updates
    #[arg(long, default_value_t = false)]
    pub install_hooks: bool,

    /// Project name to configure in git hooks (only used with --install-hooks)
    #[arg(long)]
    pub project_name: Option<String>,

    /// Mix environment to configure in git hooks (defaults to 'dev', only used with --install-hooks)
    #[arg(long)]
    pub mix_env: Option<String>,
}

impl CommandRunner for SetupCmd {
    fn run(self, db: &DbInstance, format: OutputFormat) -> Result<String, Box<dyn Error>> {
        let result = self.execute(db)?;
        Ok(result.format(format))
    }
}
