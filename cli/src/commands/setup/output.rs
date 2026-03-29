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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::setup::execute::{
        GitConfigStatus, HooksInstallResult, RelationState, RelationStatus, SetupResult,
        TemplateFileState, TemplateFileStatus, TemplatesInstallResult,
    };

    // ---------------------------------------------------------------
    // Helpers to build SetupResult values without needing Default
    // ---------------------------------------------------------------

    fn base_result(relations: Vec<RelationStatus>, created_new: bool, dry_run: bool) -> SetupResult {
        SetupResult {
            relations,
            created_new,
            dry_run,
            templates: None,
            hooks: None,
        }
    }

    fn relation(name: &str, status: RelationState) -> RelationStatus {
        RelationStatus {
            name: name.to_string(),
            status,
        }
    }

    fn template_file(path: &str, status: TemplateFileState) -> TemplateFileStatus {
        TemplateFileStatus {
            path: path.to_string(),
            status,
        }
    }

    fn templates_result(
        skills: Vec<TemplateFileStatus>,
        agents: Vec<TemplateFileStatus>,
        skills_installed: usize,
        skills_skipped: usize,
        skills_overwritten: usize,
        agents_installed: usize,
        agents_skipped: usize,
        agents_overwritten: usize,
    ) -> TemplatesInstallResult {
        TemplatesInstallResult {
            skills,
            agents,
            skills_installed,
            skills_skipped,
            skills_overwritten,
            agents_installed,
            agents_skipped,
            agents_overwritten,
        }
    }

    fn hooks_result(
        hooks: Vec<TemplateFileStatus>,
        hooks_installed: usize,
        hooks_skipped: usize,
        hooks_overwritten: usize,
        git_config: Vec<GitConfigStatus>,
    ) -> HooksInstallResult {
        HooksInstallResult {
            hooks,
            hooks_installed,
            hooks_skipped,
            hooks_overwritten,
            git_config,
        }
    }

    fn git_config(key: &str, value: &str, set: bool) -> GitConfigStatus {
        GitConfigStatus {
            key: key.to_string(),
            value: value.to_string(),
            set,
        }
    }

    // ---------------------------------------------------------------
    // Basic header / dry_run / created_new / already_exists branches
    // ---------------------------------------------------------------

    #[test]
    fn test_to_table_dry_run() {
        let result = base_result(
            vec![relation("modules", RelationState::WouldCreate)],
            false,
            true,
        );

        let table = result.to_table();

        assert!(table.contains("Database Setup"), "should contain header");
        assert!(table.contains("Schema creation (dry-run):"), "dry-run label");
        assert!(table.contains("→ modules (would create)"), "would-create symbol");
        assert!(table.contains("No changes made (dry-run mode)."), "dry-run footer");
        // Must NOT contain the non-dry-run footers
        assert!(!table.contains("Database ready."));
        assert!(!table.contains("Database already configured."));
    }

    #[test]
    fn test_to_table_created_new() {
        let result = base_result(
            vec![relation("modules", RelationState::Created)],
            true,
            false,
        );

        let table = result.to_table();

        assert!(table.contains("Schema creation:\n"), "non-dry-run label");
        assert!(table.contains("✓ modules (created)"));
        assert!(table.contains("Database ready."));
        assert!(!table.contains("Database already configured."));
    }

    #[test]
    fn test_to_table_already_exists() {
        let result = base_result(
            vec![relation("modules", RelationState::AlreadyExists)],
            false,
            false,
        );

        let table = result.to_table();

        assert!(table.contains("✓ modules (exists)"));
        assert!(table.contains("Database already configured."));
        assert!(!table.contains("Database ready."));
    }

    #[test]
    fn test_to_table_no_templates_no_hooks() {
        let result = base_result(vec![], false, false);
        let table = result.to_table();

        assert!(!table.contains("Templates Installation:"));
        assert!(!table.contains("Git Hooks Installation:"));
    }

    // ---------------------------------------------------------------
    // Templates section -- skills
    // Kills mutants on lines 47 (+), 48 (>), 73 (!), 78 (>), 84 (!), 89 (>)
    // ---------------------------------------------------------------

    /// When skills_installed=1, skills_skipped=0, skills_overwritten=0, total_skills=1.
    /// Mutating + to - would give 1-0-0=1 (same), + to * would give 1*0*0=0 (different).
    /// We need values where + vs * diverge: use skills_installed=1, skills_skipped=1.
    /// + : 1+1+0=2 (>0 => show)
    /// - : 1-1-0=0 (==0 => skip)
    /// * : 1*1*0=0 (==0 => skip)
    #[test]
    fn test_skills_total_arithmetic_kills_minus_and_star() {
        // With installed=1, skipped=1, overwritten=0 the total via + is 2.
        // Mutating to - gives 0, mutating to * gives 0. Both would skip the section.
        let tpl = templates_result(
            vec![
                template_file("skill_a.md", TemplateFileState::Installed),
                template_file("skill_b.md", TemplateFileState::Skipped),
            ],
            vec![],
            1, 1, 0, // skills: installed=1, skipped=1, overwritten=0
            0, 0, 0, // agents: all zero
        );
        let mut result = base_result(vec![], false, false);
        result.templates = Some(tpl);

        let table = result.to_table();

        // The Skills section MUST appear because total=2>0
        assert!(table.contains("Skills:"), "skills section should appear when total > 0");
        assert!(table.contains("Installed: 1, Skipped: 1, Overwritten: 0"));
        assert!(table.contains("✓ skill_a.md"), "installed skill should be listed");
    }

    /// When total_skills == 0 (all counts zero), the skills section must NOT appear.
    /// This kills the >= mutant on line 48 (> replaced with >=).
    #[test]
    fn test_skills_section_hidden_when_total_zero() {
        let tpl = templates_result(vec![], vec![], 0, 0, 0, 0, 0, 0);
        let mut result = base_result(vec![], false, false);
        result.templates = Some(tpl);

        let table = result.to_table();

        // Must still show the Templates Installation header
        assert!(table.contains("Templates Installation:"));
        // But NOT the Skills sub-section
        assert!(!table.contains("Skills:"), "skills section must be hidden when total is 0");
    }

    /// Test !installed.is_empty() on line 73: with installed files, "✓" markers appear.
    /// Deleting the ! would skip the block even when installed is non-empty.
    #[test]
    fn test_installed_skills_listed_when_present() {
        let tpl = templates_result(
            vec![template_file("s1.md", TemplateFileState::Installed)],
            vec![],
            1, 0, 0,
            0, 0, 0,
        );
        let mut result = base_result(vec![], false, false);
        result.templates = Some(tpl);

        let table = result.to_table();

        assert!(table.contains("✓ s1.md"), "installed skill must appear");
    }

    /// Test that when installed is empty, no "✓" skill lines appear.
    /// This is the counterpart that ensures the ! guard works.
    #[test]
    fn test_no_installed_skills_when_empty() {
        let tpl = templates_result(
            vec![template_file("s1.md", TemplateFileState::Skipped)],
            vec![],
            0, 1, 0,
            0, 0, 0,
        );
        let mut result = base_result(vec![], false, false);
        result.templates = Some(tpl);

        let table = result.to_table();

        // The Skills section header should appear (total=1)
        assert!(table.contains("Skills:"));
        // But no installed skill lines (no "✓" inside the skills section)
        assert!(!table.contains("✓ s1.md"), "skipped skill must not show ✓ line");
    }

    /// Test installed.len() > show_count (line 78) with more than 5 installed skills.
    /// show_count = min(len, 5) = 5, so len > show_count triggers "... and N more".
    #[test]
    fn test_skills_overflow_shows_more_message() {
        let skills: Vec<_> = (1..=7)
            .map(|i| template_file(&format!("skill_{i}.md"), TemplateFileState::Installed))
            .collect();
        let tpl = templates_result(skills, vec![], 7, 0, 0, 0, 0, 0);
        let mut result = base_result(vec![], false, false);
        result.templates = Some(tpl);

        let table = result.to_table();

        // First 5 should be shown
        assert!(table.contains("✓ skill_1.md"));
        assert!(table.contains("✓ skill_5.md"));
        // 6th and 7th should NOT be individually listed
        assert!(!table.contains("✓ skill_6.md"));
        // Overflow message should appear: "... and 2 more"
        assert!(table.contains("... and 2 more"), "overflow msg for skills");
    }

    /// Test installed.len() == show_count (exactly 5) -- no overflow message.
    /// This kills the == mutant on line 78.
    #[test]
    fn test_skills_exactly_at_limit_no_overflow() {
        let skills: Vec<_> = (1..=5)
            .map(|i| template_file(&format!("skill_{i}.md"), TemplateFileState::Installed))
            .collect();
        let tpl = templates_result(skills, vec![], 5, 0, 0, 0, 0, 0);
        let mut result = base_result(vec![], false, false);
        result.templates = Some(tpl);

        let table = result.to_table();

        assert!(table.contains("✓ skill_5.md"));
        assert!(!table.contains("... and"), "no overflow when exactly at limit");
    }

    /// Test !overwritten.is_empty() on line 84: overwritten skills show "⟳" markers.
    #[test]
    fn test_overwritten_skills_listed_when_present() {
        let tpl = templates_result(
            vec![template_file("s1.md", TemplateFileState::Overwritten)],
            vec![],
            0, 0, 1,
            0, 0, 0,
        );
        let mut result = base_result(vec![], false, false);
        result.templates = Some(tpl);

        let table = result.to_table();

        assert!(table.contains("⟳ s1.md"), "overwritten skill must appear with ⟳");
    }

    /// Test that when overwritten is empty, no "⟳" skill lines appear.
    #[test]
    fn test_no_overwritten_skills_when_empty() {
        let tpl = templates_result(
            vec![template_file("s1.md", TemplateFileState::Installed)],
            vec![],
            1, 0, 0,
            0, 0, 0,
        );
        let mut result = base_result(vec![], false, false);
        result.templates = Some(tpl);

        let table = result.to_table();

        assert!(!table.contains("⟳"), "no overwritten markers when none exist");
    }

    /// Test overwritten.len() > show_count (line 89) with more than 3 overwritten skills.
    #[test]
    fn test_overwritten_skills_overflow() {
        let skills: Vec<_> = (1..=5)
            .map(|i| template_file(&format!("ow_{i}.md"), TemplateFileState::Overwritten))
            .collect();
        let tpl = templates_result(skills, vec![], 0, 0, 5, 0, 0, 0);
        let mut result = base_result(vec![], false, false);
        result.templates = Some(tpl);

        let table = result.to_table();

        // First 3 shown
        assert!(table.contains("⟳ ow_1.md"));
        assert!(table.contains("⟳ ow_3.md"));
        // 4th+ not individually shown
        assert!(!table.contains("⟳ ow_4.md"));
        assert!(table.contains("... and 2 more overwritten"));
    }

    /// Test overwritten exactly at limit (3) -- no overflow.
    #[test]
    fn test_overwritten_skills_exactly_at_limit() {
        let skills: Vec<_> = (1..=3)
            .map(|i| template_file(&format!("ow_{i}.md"), TemplateFileState::Overwritten))
            .collect();
        let tpl = templates_result(skills, vec![], 0, 0, 3, 0, 0, 0);
        let mut result = base_result(vec![], false, false);
        result.templates = Some(tpl);

        let table = result.to_table();

        assert!(table.contains("⟳ ow_3.md"));
        assert!(!table.contains("... and"), "no overflow at exact limit");
    }

    // ---------------------------------------------------------------
    // Templates section -- agents
    // Kills mutants on lines 96 (+), 97 (>), 117 (!), 124 (!)
    // ---------------------------------------------------------------

    /// Arithmetic mutant on line 96: total_agents = installed + skipped + overwritten.
    /// Use agents_installed=1, agents_skipped=1 so + gives 2 but - gives 0, * gives 0.
    #[test]
    fn test_agents_total_arithmetic() {
        let tpl = templates_result(
            vec![],
            vec![
                template_file("agent_a.md", TemplateFileState::Installed),
                template_file("agent_b.md", TemplateFileState::Skipped),
            ],
            0, 0, 0,
            1, 1, 0, // agents: installed=1, skipped=1, overwritten=0
        );
        let mut result = base_result(vec![], false, false);
        result.templates = Some(tpl);

        let table = result.to_table();

        assert!(table.contains("Agents:"), "agents section should appear");
        assert!(table.contains("Installed: 1, Skipped: 1, Overwritten: 0"));
    }

    /// When total_agents == 0, the agents section must NOT appear.
    #[test]
    fn test_agents_section_hidden_when_total_zero() {
        let tpl = templates_result(vec![], vec![], 0, 0, 0, 0, 0, 0);
        let mut result = base_result(vec![], false, false);
        result.templates = Some(tpl);

        let table = result.to_table();

        assert!(!table.contains("Agents:"), "agents section hidden when total is 0");
    }

    /// Test !installed.is_empty() for agents (line 117).
    #[test]
    fn test_installed_agents_listed() {
        let tpl = templates_result(
            vec![],
            vec![template_file("agent1.md", TemplateFileState::Installed)],
            0, 0, 0,
            1, 0, 0,
        );
        let mut result = base_result(vec![], false, false);
        result.templates = Some(tpl);

        let table = result.to_table();

        assert!(table.contains("✓ agent1.md"));
    }

    /// When no agents are installed, no "✓" agent lines should appear.
    #[test]
    fn test_no_installed_agents_when_empty() {
        let tpl = templates_result(
            vec![],
            vec![template_file("agent1.md", TemplateFileState::Skipped)],
            0, 0, 0,
            0, 1, 0,
        );
        let mut result = base_result(vec![], false, false);
        result.templates = Some(tpl);

        let table = result.to_table();

        assert!(table.contains("Agents:"));
        // Should not show the ✓ for the skipped agent
        assert!(!table.contains("✓ agent1.md"));
    }

    /// Test !overwritten.is_empty() for agents (line 124).
    #[test]
    fn test_overwritten_agents_listed() {
        let tpl = templates_result(
            vec![],
            vec![template_file("agent1.md", TemplateFileState::Overwritten)],
            0, 0, 0,
            0, 0, 1,
        );
        let mut result = base_result(vec![], false, false);
        result.templates = Some(tpl);

        let table = result.to_table();

        assert!(table.contains("⟳ agent1.md"));
    }

    /// When no agents are overwritten, no "⟳" agent lines should appear.
    #[test]
    fn test_no_overwritten_agents_when_empty() {
        let tpl = templates_result(
            vec![],
            vec![template_file("agent1.md", TemplateFileState::Installed)],
            0, 0, 0,
            1, 0, 0,
        );
        let mut result = base_result(vec![], false, false);
        result.templates = Some(tpl);

        let table = result.to_table();

        // ⟳ must NOT appear in the agents section
        // Note: there could be ⟳ in skills section, but we have no skills here
        assert!(!table.contains("⟳"), "no overwritten markers for agents");
    }

    // ---------------------------------------------------------------
    // Hooks section
    // Kills mutants on lines 139 (+), 140 (>), 158 (!), 164 (!), 172 (!)
    // ---------------------------------------------------------------

    /// Arithmetic mutant on line 139: total_hooks = installed + skipped + overwritten.
    /// Use hooks_installed=1, hooks_skipped=1 so + gives 2, - gives 0, * gives 0.
    #[test]
    fn test_hooks_total_arithmetic() {
        let hooks = hooks_result(
            vec![
                template_file("post-commit", TemplateFileState::Installed),
                template_file("pre-push", TemplateFileState::Skipped),
            ],
            1, 1, 0, // installed=1, skipped=1, overwritten=0
            vec![],
        );
        let mut result = base_result(vec![], false, false);
        result.hooks = Some(hooks);

        let table = result.to_table();

        assert!(table.contains("Git Hooks Installation:"));
        assert!(table.contains("Installed: 1, Skipped: 1, Overwritten: 0"));
    }

    /// When total_hooks == 0, the hooks summary section must NOT appear.
    #[test]
    fn test_hooks_summary_hidden_when_total_zero() {
        let hooks = hooks_result(vec![], 0, 0, 0, vec![]);
        let mut result = base_result(vec![], false, false);
        result.hooks = Some(hooks);

        let table = result.to_table();

        assert!(table.contains("Git Hooks Installation:"));
        // The "Installed: ..." summary line should NOT appear
        assert!(!table.contains("Installed:"), "no summary when total hooks is 0");
    }

    /// Test !installed.is_empty() for hooks (line 158).
    #[test]
    fn test_installed_hooks_listed() {
        let hooks = hooks_result(
            vec![template_file("post-commit", TemplateFileState::Installed)],
            1, 0, 0,
            vec![],
        );
        let mut result = base_result(vec![], false, false);
        result.hooks = Some(hooks);

        let table = result.to_table();

        assert!(table.contains("✓ post-commit"));
    }

    /// When no hooks are installed, no "✓" hook lines appear.
    #[test]
    fn test_no_installed_hooks_when_empty() {
        let hooks = hooks_result(
            vec![template_file("post-commit", TemplateFileState::Skipped)],
            0, 1, 0,
            vec![],
        );
        let mut result = base_result(vec![], false, false);
        result.hooks = Some(hooks);

        let table = result.to_table();

        assert!(!table.contains("✓ post-commit"));
    }

    /// Test !overwritten.is_empty() for hooks (line 164).
    #[test]
    fn test_overwritten_hooks_listed() {
        let hooks = hooks_result(
            vec![template_file("post-commit", TemplateFileState::Overwritten)],
            0, 0, 1,
            vec![],
        );
        let mut result = base_result(vec![], false, false);
        result.hooks = Some(hooks);

        let table = result.to_table();

        assert!(table.contains("⟳ post-commit"));
    }

    /// When no hooks are overwritten, no "⟳" hook lines appear.
    #[test]
    fn test_no_overwritten_hooks_when_empty() {
        let hooks = hooks_result(
            vec![template_file("post-commit", TemplateFileState::Installed)],
            1, 0, 0,
            vec![],
        );
        let mut result = base_result(vec![], false, false);
        result.hooks = Some(hooks);

        let table = result.to_table();

        assert!(!table.contains("⟳"));
    }

    /// Test !hooks.git_config.is_empty() (line 172).
    #[test]
    fn test_git_config_listed_when_present() {
        let hooks = hooks_result(
            vec![],
            0, 0, 0,
            vec![git_config("code-search.mix-env", "dev", true)],
        );
        let mut result = base_result(vec![], false, false);
        result.hooks = Some(hooks);

        let table = result.to_table();

        assert!(table.contains("Git Configuration:"));
        assert!(table.contains("✓ code-search.mix-env = dev"));
    }

    /// When git_config is empty, no Git Configuration section.
    #[test]
    fn test_no_git_config_when_empty() {
        let hooks = hooks_result(vec![], 0, 0, 0, vec![]);
        let mut result = base_result(vec![], false, false);
        result.hooks = Some(hooks);

        let table = result.to_table();

        assert!(!table.contains("Git Configuration:"));
    }

    /// Test git config with set=false shows ✗ symbol.
    #[test]
    fn test_git_config_failed_symbol() {
        let hooks = hooks_result(
            vec![],
            0, 0, 0,
            vec![git_config("code-search.mix-env", "dev", false)],
        );
        let mut result = base_result(vec![], false, false);
        result.hooks = Some(hooks);

        let table = result.to_table();

        assert!(table.contains("✗ code-search.mix-env = dev"));
        assert!(!table.contains("✓ code-search.mix-env"));
    }

    /// Hooks footer always appears when hooks section is present.
    #[test]
    fn test_hooks_footer() {
        let hooks = hooks_result(vec![], 0, 0, 0, vec![]);
        let mut result = base_result(vec![], false, false);
        result.hooks = Some(hooks);

        let table = result.to_table();

        assert!(table.contains("Git hooks installed to .git/hooks/"));
        assert!(table.contains("Run 'git config --get-regexp code-search' to view configuration."));
    }

    /// Templates footer always appears when templates section is present.
    #[test]
    fn test_templates_footer() {
        let tpl = templates_result(vec![], vec![], 0, 0, 0, 0, 0, 0);
        let mut result = base_result(vec![], false, false);
        result.templates = Some(tpl);

        let table = result.to_table();

        assert!(table.contains("Templates installed to .claude/"));
    }

    // ---------------------------------------------------------------
    // Combined scenario: both templates and hooks present
    // ---------------------------------------------------------------

    #[test]
    fn test_full_output_with_all_sections() {
        let tpl = templates_result(
            vec![
                template_file("skill1.md", TemplateFileState::Installed),
                template_file("skill2.md", TemplateFileState::Overwritten),
            ],
            vec![template_file("agent1.md", TemplateFileState::Installed)],
            1, 0, 1,
            1, 0, 0,
        );
        let hooks = hooks_result(
            vec![template_file("post-commit", TemplateFileState::Installed)],
            1, 0, 0,
            vec![
                git_config("code-search.mix-env", "dev", true),
            ],
        );
        let mut result = base_result(
            vec![
                relation("modules", RelationState::Created),
                relation("functions", RelationState::AlreadyExists),
            ],
            true,
            false,
        );
        result.templates = Some(tpl);
        result.hooks = Some(hooks);

        let table = result.to_table();

        // Database section
        assert!(table.contains("Database Setup"));
        assert!(table.contains("✓ modules (created)"));
        assert!(table.contains("✓ functions (exists)"));
        assert!(table.contains("Database ready."));

        // Templates section
        assert!(table.contains("Templates Installation:"));
        assert!(table.contains("Skills:"));
        assert!(table.contains("✓ skill1.md"));
        assert!(table.contains("⟳ skill2.md"));
        assert!(table.contains("Agents:"));
        assert!(table.contains("✓ agent1.md"));
        assert!(table.contains("Templates installed to .claude/"));

        // Hooks section
        assert!(table.contains("Git Hooks Installation:"));
        assert!(table.contains("✓ post-commit"));
        assert!(table.contains("Git Configuration:"));
        assert!(table.contains("✓ code-search.mix-env = dev"));
        assert!(table.contains("Git hooks installed to .git/hooks/"));
    }

    // ---------------------------------------------------------------
    // to_table returns non-empty (kills replace -> String::new() mutant)
    // ---------------------------------------------------------------

    #[test]
    fn test_to_table_returns_non_empty() {
        let result = base_result(vec![], false, false);
        let table = result.to_table();
        assert!(!table.is_empty(), "to_table must return non-empty string");
        assert!(table.contains("Database Setup"), "must contain header");
    }

    // ---------------------------------------------------------------
    // Edge: both + operands are non-zero to distinguish + from *
    // For line 47: skills_installed + skills_skipped + skills_overwritten
    // Use (2, 3, 0): + gives 5, - gives -1 (wraps or differs), * gives 0
    // ---------------------------------------------------------------

    #[test]
    fn test_skills_arithmetic_all_nonzero() {
        // installed=2, skipped=3, overwritten=0
        // + : 2+3+0 = 5
        // * : 2*3*0 = 0 (would skip section)
        // - : 2-3-0 = wraps (usize) or differs
        let skills: Vec<_> = (1..=2)
            .map(|i| template_file(&format!("s{i}.md"), TemplateFileState::Installed))
            .chain((1..=3).map(|i| template_file(&format!("sk{i}.md"), TemplateFileState::Skipped)))
            .collect();
        let tpl = templates_result(skills, vec![], 2, 3, 0, 0, 0, 0);
        let mut result = base_result(vec![], false, false);
        result.templates = Some(tpl);

        let table = result.to_table();
        assert!(table.contains("Skills:"), "must show skills section with total=5");
        assert!(table.contains("Installed: 2, Skipped: 3, Overwritten: 0"));
    }

    #[test]
    fn test_agents_arithmetic_all_nonzero() {
        // installed=2, skipped=3, overwritten=0
        let agents: Vec<_> = (1..=2)
            .map(|i| template_file(&format!("a{i}.md"), TemplateFileState::Installed))
            .chain((1..=3).map(|i| template_file(&format!("ak{i}.md"), TemplateFileState::Skipped)))
            .collect();
        let tpl = templates_result(vec![], agents, 0, 0, 0, 2, 3, 0);
        let mut result = base_result(vec![], false, false);
        result.templates = Some(tpl);

        let table = result.to_table();
        assert!(table.contains("Agents:"), "must show agents section with total=5");
    }

    #[test]
    fn test_hooks_arithmetic_all_nonzero() {
        // installed=2, skipped=3, overwritten=0
        let hook_files: Vec<_> = (1..=2)
            .map(|i| template_file(&format!("hook{i}"), TemplateFileState::Installed))
            .chain((1..=3).map(|i| template_file(&format!("hooksk{i}"), TemplateFileState::Skipped)))
            .collect();
        let hooks = hooks_result(hook_files, 2, 3, 0, vec![]);
        let mut result = base_result(vec![], false, false);
        result.hooks = Some(hooks);

        let table = result.to_table();
        assert!(table.contains("Installed: 2, Skipped: 3, Overwritten: 0"));
    }
}
