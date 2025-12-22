//! Output formatting for setup command results.

use crate::output::Outputable;
use crate::commands::setup::execute::{SetupResult, RelationState, TemplateFileState};

impl Outputable for SetupResult {
    fn to_table(&self) -> String {
        let mut output = String::new();

        output.push_str("Database Setup\n\n");

        if self.dry_run {
            output.push_str("Schema creation (dry-run):\n");
        } else {
            output.push_str("Schema creation:\n");
        }

        for relation in &self.relations {
            let symbol = match relation.status {
                RelationState::Created => "✓",
                RelationState::AlreadyExists => "✓",
                RelationState::WouldCreate => "→",
            };

            let status_text = match relation.status {
                RelationState::Created => "created",
                RelationState::AlreadyExists => "exists",
                RelationState::WouldCreate => "would create",
            };

            output.push_str(&format!("  {} {} ({})\n", symbol, relation.name, status_text));
        }

        if self.dry_run {
            output.push_str("\nNo changes made (dry-run mode).\n");
        } else if self.created_new {
            output.push_str("\nDatabase ready.\n");
        } else {
            output.push_str("\nDatabase already configured.\n");
        }

        // Add template installation results if present
        if let Some(ref templates) = self.templates {
            output.push_str("\nTemplates Installation:\n");

            // Skills summary
            let total_skills = templates.skills_installed + templates.skills_skipped + templates.skills_overwritten;
            if total_skills > 0 {
                output.push_str("\n  Skills:\n");
                output.push_str(&format!(
                    "    Installed: {}, Skipped: {}, Overwritten: {}\n",
                    templates.skills_installed, templates.skills_skipped, templates.skills_overwritten
                ));

                // Group skill files by status
                let installed: Vec<_> = templates
                    .skills
                    .iter()
                    .filter(|f| matches!(f.status, TemplateFileState::Installed))
                    .collect();
                let overwritten: Vec<_> = templates
                    .skills
                    .iter()
                    .filter(|f| matches!(f.status, TemplateFileState::Overwritten))
                    .collect();
                let _skipped: Vec<_> = templates
                    .skills
                    .iter()
                    .filter(|f| matches!(f.status, TemplateFileState::Skipped))
                    .collect();

                // Show installed skills (only first few)
                if !installed.is_empty() {
                    let show_count = installed.len().min(5);
                    for file in &installed[..show_count] {
                        output.push_str(&format!("      ✓ {}\n", file.path));
                    }
                    if installed.len() > show_count {
                        output.push_str(&format!("      ... and {} more\n", installed.len() - show_count));
                    }
                }

                // Show overwritten skills
                if !overwritten.is_empty() {
                    let show_count = overwritten.len().min(3);
                    for file in &overwritten[..show_count] {
                        output.push_str(&format!("      ⟳ {}\n", file.path));
                    }
                    if overwritten.len() > show_count {
                        output.push_str(&format!("      ... and {} more overwritten\n", overwritten.len() - show_count));
                    }
                }
            }

            // Agents summary
            let total_agents = templates.agents_installed + templates.agents_skipped + templates.agents_overwritten;
            if total_agents > 0 {
                output.push_str("\n  Agents:\n");
                output.push_str(&format!(
                    "    Installed: {}, Skipped: {}, Overwritten: {}\n",
                    templates.agents_installed, templates.agents_skipped, templates.agents_overwritten
                ));

                // Group agent files by status
                let installed: Vec<_> = templates
                    .agents
                    .iter()
                    .filter(|f| matches!(f.status, TemplateFileState::Installed))
                    .collect();
                let overwritten: Vec<_> = templates
                    .agents
                    .iter()
                    .filter(|f| matches!(f.status, TemplateFileState::Overwritten))
                    .collect();

                // Show installed agents
                if !installed.is_empty() {
                    for file in installed {
                        output.push_str(&format!("      ✓ {}\n", file.path));
                    }
                }

                // Show overwritten agents
                if !overwritten.is_empty() {
                    for file in overwritten {
                        output.push_str(&format!("      ⟳ {}\n", file.path));
                    }
                }
            }

            output.push_str("\nTemplates installed to .claude/\n");
        }

        // Add git hooks installation results if present
        if let Some(ref hooks) = self.hooks {
            output.push_str("\nGit Hooks Installation:\n");

            // Hooks summary
            let total_hooks = hooks.hooks_installed + hooks.hooks_skipped + hooks.hooks_overwritten;
            if total_hooks > 0 {
                output.push_str(&format!(
                    "\n  Installed: {}, Skipped: {}, Overwritten: {}\n",
                    hooks.hooks_installed, hooks.hooks_skipped, hooks.hooks_overwritten
                ));

                // Show hooks by status
                let installed: Vec<_> = hooks
                    .hooks
                    .iter()
                    .filter(|f| matches!(f.status, TemplateFileState::Installed))
                    .collect();
                let overwritten: Vec<_> = hooks
                    .hooks
                    .iter()
                    .filter(|f| matches!(f.status, TemplateFileState::Overwritten))
                    .collect();

                if !installed.is_empty() {
                    for file in installed {
                        output.push_str(&format!("    ✓ {}\n", file.path));
                    }
                }

                if !overwritten.is_empty() {
                    for file in overwritten {
                        output.push_str(&format!("    ⟳ {}\n", file.path));
                    }
                }
            }

            // Git config
            if !hooks.git_config.is_empty() {
                output.push_str("\n  Git Configuration:\n");
                for config in &hooks.git_config {
                    let symbol = if config.set { "✓" } else { "✗" };
                    output.push_str(&format!(
                        "    {} {} = {}\n",
                        symbol, config.key, config.value
                    ));
                }
            }

            output.push_str("\nGit hooks installed to .git/hooks/\n");
            output.push_str("Run 'git config --get-regexp code-search' to view configuration.\n");
        }

        output
    }
}
