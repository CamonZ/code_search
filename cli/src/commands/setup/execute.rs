use std::error::Error;
use std::fs;
use db::DbInstance;
use include_dir::{include_dir, Dir};
use serde::Serialize;

use super::SetupCmd;
use crate::commands::Execute;
use db::queries::schema;

/// Embedded skill templates directory
static SKILL_TEMPLATES: Dir = include_dir!("$CARGO_MANIFEST_DIR/../templates/skills");

/// Embedded agent templates directory
static AGENT_TEMPLATES: Dir = include_dir!("$CARGO_MANIFEST_DIR/../templates/agents");

/// Embedded hook templates directory
static HOOK_TEMPLATES: Dir = include_dir!("$CARGO_MANIFEST_DIR/../templates/hooks");

/// Status of a database relation (table)
#[derive(Debug, Clone, Serialize)]
pub enum RelationState {
    #[serde(rename = "created")]
    Created,
    #[serde(rename = "exists")]
    AlreadyExists,
    #[serde(rename = "would_create")]
    WouldCreate,
}

/// Status information for a single database relation
#[derive(Debug, Clone, Serialize)]
pub struct RelationStatus {
    pub name: String,
    pub status: RelationState,
}

/// Status of a template file (skill or agent)
#[derive(Debug, Clone, Serialize)]
pub enum TemplateFileState {
    #[serde(rename = "installed")]
    Installed,
    #[serde(rename = "skipped")]
    Skipped,
    #[serde(rename = "overwritten")]
    Overwritten,
}

/// Status information for a single template file
#[derive(Debug, Clone, Serialize)]
pub struct TemplateFileStatus {
    pub path: String,
    pub status: TemplateFileState,
}

/// Result of templates installation
#[derive(Debug, Serialize)]
pub struct TemplatesInstallResult {
    pub skills: Vec<TemplateFileStatus>,
    pub agents: Vec<TemplateFileStatus>,
    pub skills_installed: usize,
    pub skills_skipped: usize,
    pub skills_overwritten: usize,
    pub agents_installed: usize,
    pub agents_skipped: usize,
    pub agents_overwritten: usize,
}

/// Result of git hooks installation
#[derive(Debug, Serialize)]
pub struct HooksInstallResult {
    pub hooks: Vec<TemplateFileStatus>,
    pub hooks_installed: usize,
    pub hooks_skipped: usize,
    pub hooks_overwritten: usize,
    pub git_config: Vec<GitConfigStatus>,
}

/// Status of a git config setting
#[derive(Debug, Clone, Serialize)]
pub struct GitConfigStatus {
    pub key: String,
    pub value: String,
    pub set: bool,
}

/// Result of the setup command execution
#[derive(Debug, Serialize)]
pub struct SetupResult {
    pub relations: Vec<RelationStatus>,
    pub created_new: bool,
    pub dry_run: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub templates: Option<TemplatesInstallResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hooks: Option<HooksInstallResult>,
}

/// Recursively process a directory and install all files
fn process_dir(
    dir: &include_dir::Dir,
    base_path: &std::path::Path,
    force: bool,
    files: &mut Vec<TemplateFileStatus>,
    installed_count: &mut usize,
    skipped_count: &mut usize,
    overwritten_count: &mut usize,
) -> Result<(), Box<dyn Error>> {
    for entry in dir.entries() {
        match entry {
            include_dir::DirEntry::Dir(subdir) => {
                // Recursively process subdirectory
                process_dir(subdir, base_path, force, files, installed_count, skipped_count, overwritten_count)?;
            }
            include_dir::DirEntry::File(file) => {
                let relative_path = file.path();
                let target_path = base_path.join(relative_path);

                // Create parent directories if needed
                if let Some(parent) = target_path.parent() {
                    fs::create_dir_all(parent)?;
                }

                // Check if file exists
                let exists = target_path.exists();
                let status = if exists && !force {
                    // Skip existing files unless force is enabled
                    *skipped_count += 1;
                    TemplateFileState::Skipped
                } else {
                    // Write file contents
                    fs::write(&target_path, file.contents())?;

                    if exists {
                        *overwritten_count += 1;
                        TemplateFileState::Overwritten
                    } else {
                        *installed_count += 1;
                        TemplateFileState::Installed
                    }
                };

                files.push(TemplateFileStatus {
                    path: relative_path.display().to_string(),
                    status,
                });
            }
        }
    }
    Ok(())
}

/// Install templates (skills and agents) to .claude/ in the given base directory
fn install_templates_to(base_dir: &std::path::Path, force: bool) -> Result<TemplatesInstallResult, Box<dyn Error>> {
    let claude_dir = base_dir.join(".claude");
    let skills_dir = claude_dir.join("skills");
    let agents_dir = claude_dir.join("agents");

    // Create .claude/skills/ and .claude/agents/ directories
    fs::create_dir_all(&skills_dir)?;
    fs::create_dir_all(&agents_dir)?;

    // Process skills
    let mut skills_files = Vec::new();
    let mut skills_installed = 0;
    let mut skills_skipped = 0;
    let mut skills_overwritten = 0;

    process_dir(
        &SKILL_TEMPLATES,
        &skills_dir,
        force,
        &mut skills_files,
        &mut skills_installed,
        &mut skills_skipped,
        &mut skills_overwritten,
    )?;

    // Process agents
    let mut agents_files = Vec::new();
    let mut agents_installed = 0;
    let mut agents_skipped = 0;
    let mut agents_overwritten = 0;

    process_dir(
        &AGENT_TEMPLATES,
        &agents_dir,
        force,
        &mut agents_files,
        &mut agents_installed,
        &mut agents_skipped,
        &mut agents_overwritten,
    )?;

    Ok(TemplatesInstallResult {
        skills: skills_files,
        agents: agents_files,
        skills_installed,
        skills_skipped,
        skills_overwritten,
        agents_installed,
        agents_skipped,
        agents_overwritten,
    })
}

/// Install templates to .claude/
fn install_templates(force: bool) -> Result<TemplatesInstallResult, Box<dyn Error>> {
    // Get current working directory
    let cwd = std::env::current_dir()?;
    install_templates_to(&cwd, force)
}

/// Install git hooks to .git/hooks/
fn install_hooks(
    force: bool,
    project_name: Option<String>,
    mix_env: Option<String>,
) -> Result<HooksInstallResult, Box<dyn Error>> {
    use std::process::Command;

    // Check if we're in a git repository
    let git_dir = Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .output()?;

    if !git_dir.status.success() {
        return Err("Not in a git repository".into());
    }

    let git_dir_path = String::from_utf8(git_dir.stdout)?.trim().to_string();
    let hooks_dir = std::path::Path::new(&git_dir_path).join("hooks");

    // Create hooks directory if it doesn't exist
    fs::create_dir_all(&hooks_dir)?;

    // Process hook files
    let mut hooks_files = Vec::new();
    let mut hooks_installed = 0;
    let mut hooks_skipped = 0;
    let mut hooks_overwritten = 0;

    process_dir(
        &HOOK_TEMPLATES,
        &hooks_dir,
        force,
        &mut hooks_files,
        &mut hooks_installed,
        &mut hooks_skipped,
        &mut hooks_overwritten,
    )?;

    // Make hooks executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for file_status in &hooks_files {
            let hook_path = hooks_dir.join(&file_status.path);
            if hook_path.exists() {
                let mut perms = fs::metadata(&hook_path)?.permissions();
                perms.set_mode(0o755);
                fs::set_permissions(&hook_path, perms)?;
            }
        }
    }

    // Configure git settings
    let mut git_config = Vec::new();

    // Build config list (only set values that are provided)
    let mut configs: Vec<(&str, String)> = Vec::new();

    // Only set project name if explicitly provided
    if let Some(name) = project_name {
        configs.push(("code-search.project-name", name));
    }

    // Set mix-env (default to "dev" if not provided)
    configs.push((
        "code-search.mix-env",
        mix_env.unwrap_or_else(|| "dev".to_string()),
    ));

    for (key, value) in configs {
        let output = Command::new("git")
            .args(["config", key, &value])
            .output()?;

        git_config.push(GitConfigStatus {
            key: key.to_string(),
            value,
            set: output.status.success(),
        });
    }

    Ok(HooksInstallResult {
        hooks: hooks_files,
        hooks_installed,
        hooks_skipped,
        hooks_overwritten,
        git_config,
    })
}

impl Execute for SetupCmd {
    type Output = SetupResult;

    fn execute(self, db: &DbInstance) -> Result<Self::Output, Box<dyn Error>> {
        let mut relations = Vec::new();

        if self.dry_run {
            // In dry-run mode, just show what would be created
            for rel_name in schema::relation_names() {
                relations.push(RelationStatus {
                    name: rel_name.to_string(),
                    status: RelationState::WouldCreate,
                });
            }

            return Ok(SetupResult {
                relations,
                created_new: false,
                dry_run: true,
                templates: None,
                hooks: None,
            });
        }

        // Note: The --force flag only affects template and hook file overwriting.
        // It does not drop and recreate database schemas. Schema creation is idempotent
        // and will skip existing relations regardless of the --force flag.

        // Create schema
        let schema_results = schema::create_schema(db)?;

        for schema_result in schema_results {
            let status = if schema_result.created {
                RelationState::Created
            } else {
                RelationState::AlreadyExists
            };

            relations.push(RelationStatus {
                name: schema_result.relation,
                status,
            });
        }

        // Check if we created new relations
        let created_new = relations
            .iter()
            .any(|r| matches!(r.status, RelationState::Created));

        // Install templates (skills and agents) if requested
        let templates = if self.install_skills {
            Some(install_templates(self.force)?)
        } else {
            None
        };

        // Install git hooks if requested
        let hooks = if self.install_hooks {
            Some(install_hooks(
                self.force,
                self.project_name,
                self.mix_env,
            )?)
        } else {
            None
        };

        Ok(SetupResult {
            relations,
            created_new,
            dry_run: false,
            templates,
            hooks,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use db::open_db;
    use rstest::{fixture, rstest};
    use tempfile::NamedTempFile;

    #[fixture]
    fn db_file() -> NamedTempFile {
        NamedTempFile::new().expect("Failed to create temp db file")
    }

    #[rstest]
    fn test_setup_creates_all_relations(db_file: NamedTempFile) {
        let cmd = SetupCmd {
            force: false,
            dry_run: false,
            install_skills: false,
            install_hooks: false,
            project_name: None,
            mix_env: None,
        };

        let db = open_db(db_file.path()).expect("Failed to open db");
        let result = cmd.execute(&db).expect("Setup should succeed");

        // Should create 7 relations
        assert_eq!(result.relations.len(), 7);

        // All should be created
        assert!(result
            .relations
            .iter()
            .all(|r| matches!(r.status, RelationState::Created)));

        assert!(result.created_new);
        assert!(result.templates.is_none());
        assert!(result.hooks.is_none());
    }

    #[rstest]
    fn test_setup_idempotent(db_file: NamedTempFile) {
        let db = open_db(db_file.path()).expect("Failed to open db");

        // First setup
        let cmd1 = SetupCmd {
            force: false,
            dry_run: false,
            install_skills: false,
            install_hooks: false,
            project_name: None,
            mix_env: None,
        };
        let result1 = cmd1.execute(&db).expect("First setup should succeed");
        assert!(result1.created_new);

        // Second setup should find existing relations
        let cmd2 = SetupCmd {
            force: false,
            dry_run: false,
            install_skills: false,
            install_hooks: false,
            project_name: None,
            mix_env: None,
        };
        let result2 = cmd2.execute(&db).expect("Second setup should succeed");

        // Should still have 7 relations, but all already existing
        assert_eq!(result2.relations.len(), 7);
        assert!(result2
            .relations
            .iter()
            .all(|r| matches!(r.status, RelationState::AlreadyExists)));

        assert!(!result2.created_new);
    }

    #[rstest]
    fn test_setup_dry_run(db_file: NamedTempFile) {
        let cmd = SetupCmd {
            force: false,
            dry_run: true,
            install_skills: false,
            install_hooks: false,
            project_name: None,
            mix_env: None,
        };

        let db = open_db(db_file.path()).expect("Failed to open db");
        let result = cmd.execute(&db).expect("Setup should succeed");

        assert!(result.dry_run);
        assert_eq!(result.relations.len(), 7);

        // All should be in would_create state
        assert!(result
            .relations
            .iter()
            .all(|r| matches!(r.status, RelationState::WouldCreate)));

        // Should not have actually created anything
        assert!(!result.created_new);
    }

    #[rstest]
    fn test_setup_relations_have_correct_names(db_file: NamedTempFile) {
        let cmd = SetupCmd {
            force: false,
            dry_run: true,
            install_skills: false,
            install_hooks: false,
            project_name: None,
            mix_env: None,
        };

        let db = open_db(db_file.path()).expect("Failed to open db");
        let result = cmd.execute(&db).expect("Setup should succeed");

        let relation_names: Vec<_> = result.relations.iter().map(|r| r.name.as_str()).collect();

        assert!(relation_names.contains(&"modules"));
        assert!(relation_names.contains(&"functions"));
        assert!(relation_names.contains(&"calls"));
        assert!(relation_names.contains(&"struct_fields"));
        assert!(relation_names.contains(&"function_locations"));
        assert!(relation_names.contains(&"specs"));
        assert!(relation_names.contains(&"types"));
    }

    #[test]
    fn test_install_templates() {
        use tempfile::TempDir;

        // Create a temporary directory
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // Install templates directly to temp directory
        let result = install_templates_to(temp_dir.path(), false)
            .expect("Install should succeed");

        // All files should be installed (not skipped or overwritten)
        assert_eq!(result.skills_installed, 34, "Should install all 34 skill files");
        assert_eq!(result.skills_skipped, 0);
        assert_eq!(result.skills_overwritten, 0);

        assert_eq!(result.agents_installed, 1, "Should install 1 agent file");
        assert_eq!(result.agents_skipped, 0);
        assert_eq!(result.agents_overwritten, 0);

        // Verify .claude/skills and .claude/agents directories were created
        assert!(temp_dir.path().join(".claude").join("skills").exists());
        assert!(temp_dir.path().join(".claude").join("agents").exists());
    }

    #[test]
    fn test_install_templates_skips_existing() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // First installation
        let result1 = install_templates_to(temp_dir.path(), false)
            .expect("First install should succeed");
        assert_eq!(result1.skills_installed, 34);
        assert_eq!(result1.agents_installed, 1);

        // Second installation without force - should skip all files
        let result2 = install_templates_to(temp_dir.path(), false)
            .expect("Second install should succeed");
        assert_eq!(result2.skills_installed, 0, "Should not install any skill files");
        assert_eq!(result2.skills_skipped, 34, "Should skip all 34 existing skill files");
        assert_eq!(result2.skills_overwritten, 0);

        assert_eq!(result2.agents_installed, 0, "Should not install any agent files");
        assert_eq!(result2.agents_skipped, 1, "Should skip the existing agent file");
        assert_eq!(result2.agents_overwritten, 0);
    }

    #[test]
    fn test_install_templates_force_overwrites() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // First installation
        let result1 = install_templates_to(temp_dir.path(), false)
            .expect("First install should succeed");
        assert_eq!(result1.skills_installed, 34);
        assert_eq!(result1.agents_installed, 1);

        // Second installation with force - should overwrite all files
        let result2 = install_templates_to(temp_dir.path(), true)
            .expect("Second install with force should succeed");
        assert_eq!(result2.skills_installed, 0, "Should not install new skill files");
        assert_eq!(result2.skills_skipped, 0, "Should not skip any skill files");
        assert_eq!(result2.skills_overwritten, 34, "Should overwrite all 34 existing skill files");

        assert_eq!(result2.agents_installed, 0, "Should not install new agent files");
        assert_eq!(result2.agents_skipped, 0, "Should not skip any agent files");
        assert_eq!(result2.agents_overwritten, 1, "Should overwrite the existing agent file");
    }

    #[rstest]
    fn test_no_templates_when_not_requested(db_file: NamedTempFile) {
        let cmd = SetupCmd {
            force: false,
            dry_run: false,
            install_skills: false,
            install_hooks: false,
            project_name: None,
            mix_env: None,
        };

        let db = open_db(db_file.path()).expect("Failed to open db");
        let result = cmd.execute(&db).expect("Setup should succeed");

        // Templates and hooks should be None when not requested
        assert!(result.templates.is_none());
        assert!(result.hooks.is_none());
    }

    #[test]
    #[serial_test::serial]
    fn test_install_hooks_in_git_repo() {
        use std::process::Command;
        use tempfile::TempDir;

        // Create a temporary directory and initialize a git repo
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let temp_path = temp_dir.path();

        // Initialize git repo
        Command::new("git")
            .args(["init"])
            .current_dir(temp_path)
            .output()
            .expect("Failed to initialize git repo");

        // Change to the temp directory for the test
        let original_dir = std::env::current_dir().expect("Failed to get current dir");
        std::env::set_current_dir(temp_path).expect("Failed to change directory");

        // Create a temporary database
        let db_file = NamedTempFile::new().expect("Failed to create temp db file");
        let db = open_db(db_file.path()).expect("Failed to open db");

        let cmd = SetupCmd {
            force: false,
            dry_run: false,
            install_skills: false,
            install_hooks: true,
            project_name: Some("test_project".to_string()),
            mix_env: Some("test".to_string()),
        };

        let result = cmd.execute(&db).expect("Setup with hooks should succeed");

        // Verify hook file exists and is executable BEFORE restoring directory
        let hook_path = temp_path.join(".git").join("hooks").join("post-commit");
        assert!(hook_path.exists(), "Hook file should exist");

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = fs::metadata(&hook_path).expect("Failed to get hook metadata");
            let permissions = metadata.permissions();
            assert!(
                permissions.mode() & 0o111 != 0,
                "Hook should be executable"
            );
        }

        // Verify hook content
        let hook_content = fs::read_to_string(&hook_path).expect("Failed to read hook");
        assert!(hook_content.contains("#!/usr/bin/env bash"));
        assert!(hook_content.contains("ex_ast --git-diff"));
        assert!(hook_content.contains("code_search"));
        assert!(hook_content.contains("GIT_REF")); // Uses variable for git reference

        // Verify hooks were installed
        assert!(result.hooks.is_some());
        let hooks = result.hooks.unwrap();

        // Should have installed 1 hook (post-commit)
        assert_eq!(hooks.hooks_installed, 1);
        assert_eq!(hooks.hooks_skipped, 0);
        assert_eq!(hooks.hooks_overwritten, 0);

        // Should have 1 hook file
        assert_eq!(hooks.hooks.len(), 1);
        assert_eq!(hooks.hooks[0].path, "post-commit");
        assert!(matches!(hooks.hooks[0].status, TemplateFileState::Installed));

        // Should have configured 2 git settings (project-name and mix-env)
        assert_eq!(hooks.git_config.len(), 2);

        // Verify git config values
        let project_config = hooks.git_config.iter().find(|c| c.key == "code-search.project-name");
        assert!(project_config.is_some());
        assert_eq!(project_config.unwrap().value, "test_project");
        assert!(project_config.unwrap().set);

        let mix_env_config = hooks.git_config.iter().find(|c| c.key == "code-search.mix-env");
        assert!(mix_env_config.is_some());
        assert_eq!(mix_env_config.unwrap().value, "test");
        assert!(mix_env_config.unwrap().set);

        // Restore original directory
        std::env::set_current_dir(&original_dir).ok(); // Ignore error if original_dir was deleted
    }

    #[test]
    #[serial_test::serial]
    fn test_install_hooks_with_defaults() {
        use std::process::Command;
        use tempfile::TempDir;

        // Create a temporary directory and initialize a git repo
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let temp_path = temp_dir.path();

        Command::new("git")
            .args(["init"])
            .current_dir(temp_path)
            .output()
            .expect("Failed to initialize git repo");

        let original_dir = std::env::current_dir().expect("Failed to get current dir");
        std::env::set_current_dir(temp_path).expect("Failed to change directory");

        let db_file = NamedTempFile::new().expect("Failed to create temp db file");
        let db = open_db(db_file.path()).expect("Failed to open db");

        let cmd = SetupCmd {
            force: false,
            dry_run: false,
            install_skills: false,
            install_hooks: true,
            project_name: None,
            mix_env: None,
        };

        let result = cmd.execute(&db).expect("Setup with hooks should succeed");

        assert!(result.hooks.is_some());
        let hooks = result.hooks.unwrap();

        // Should only set mix-env (project-name not set when None)
        assert_eq!(hooks.git_config.len(), 1);

        // Verify default values were used
        let mix_env_config = hooks.git_config.iter().find(|c| c.key == "code-search.mix-env");
        assert!(mix_env_config.is_some());
        assert_eq!(mix_env_config.unwrap().value, "dev");

        // Verify project-name was NOT set
        let project_config = hooks.git_config.iter().find(|c| c.key == "code-search.project-name");
        assert!(project_config.is_none());

        // Restore original directory
        std::env::set_current_dir(&original_dir).ok(); // Ignore error if original_dir was deleted
    }

    #[test]
    #[serial_test::serial]
    fn test_install_hooks_skips_existing() {
        use std::process::Command;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let temp_path = temp_dir.path();

        Command::new("git")
            .args(["init"])
            .current_dir(temp_path)
            .output()
            .expect("Failed to initialize git repo");

        let original_dir = std::env::current_dir().expect("Failed to get current dir");
        std::env::set_current_dir(temp_path).expect("Failed to change directory");

        let db_file = NamedTempFile::new().expect("Failed to create temp db file");
        let db = open_db(db_file.path()).expect("Failed to open db");

        // First installation
        let cmd1 = SetupCmd {
            force: false,
            dry_run: false,
            install_skills: false,
            install_hooks: true,
            project_name: None,
            mix_env: None,
        };

        let result1 = cmd1.execute(&db).expect("First install should succeed");
        assert_eq!(result1.hooks.as_ref().unwrap().hooks_installed, 1);

        // Second installation without force
        let cmd2 = SetupCmd {
            force: false,
            dry_run: false,
            install_skills: false,
            install_hooks: true,
            project_name: None,
            mix_env: None,
        };

        let result2 = cmd2.execute(&db).expect("Second install should succeed");

        // Should skip existing hook
        assert_eq!(result2.hooks.as_ref().unwrap().hooks_installed, 0);
        assert_eq!(result2.hooks.as_ref().unwrap().hooks_skipped, 1);
        assert_eq!(result2.hooks.as_ref().unwrap().hooks_overwritten, 0);

        // Restore original directory
        std::env::set_current_dir(&original_dir).ok(); // Ignore error if original_dir was deleted
    }

    #[test]
    #[serial_test::serial]
    fn test_install_hooks_force_overwrites() {
        use std::process::Command;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let temp_path = temp_dir.path();

        Command::new("git")
            .args(["init"])
            .current_dir(temp_path)
            .output()
            .expect("Failed to initialize git repo");

        let original_dir = std::env::current_dir().expect("Failed to get current dir");
        std::env::set_current_dir(temp_path).expect("Failed to change directory");

        let db_file = NamedTempFile::new().expect("Failed to create temp db file");
        let db = open_db(db_file.path()).expect("Failed to open db");

        // First installation
        let cmd1 = SetupCmd {
            force: false,
            dry_run: false,
            install_skills: false,
            install_hooks: true,
            project_name: None,
            mix_env: None,
        };

        cmd1.execute(&db).expect("First install should succeed");

        // Second installation with force
        let cmd2 = SetupCmd {
            force: true,
            dry_run: false,
            install_skills: false,
            install_hooks: true,
            project_name: None,
            mix_env: None,
        };

        let result2 = cmd2.execute(&db).expect("Second install with force should succeed");

        // Should overwrite existing hook
        assert_eq!(result2.hooks.as_ref().unwrap().hooks_installed, 0);
        assert_eq!(result2.hooks.as_ref().unwrap().hooks_skipped, 0);
        assert_eq!(result2.hooks.as_ref().unwrap().hooks_overwritten, 1);

        // Restore original directory
        std::env::set_current_dir(&original_dir).ok(); // Ignore error if original_dir was deleted
    }

    #[test]
    #[serial_test::serial]
    fn test_install_hooks_fails_outside_git_repo() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let temp_path = temp_dir.path();

        let original_dir = std::env::current_dir().expect("Failed to get current dir");
        std::env::set_current_dir(temp_path).expect("Failed to change directory");

        let db_file = NamedTempFile::new().expect("Failed to create temp db file");
        let db = open_db(db_file.path()).expect("Failed to open db");

        let cmd = SetupCmd {
            force: false,
            dry_run: false,
            install_skills: false,
            install_hooks: true,
            project_name: None,
            mix_env: None,
        };

        let result = cmd.execute(&db);

        // Restore original directory
        std::env::set_current_dir(&original_dir).ok(); // Ignore error if original_dir was deleted

        // Should fail because we're not in a git repo
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Not in a git repository"));
    }
}
