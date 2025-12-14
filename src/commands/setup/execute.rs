use std::error::Error;

use serde::Serialize;

use super::SetupCmd;
use crate::commands::Execute;
use crate::db::DatabaseBackend;
use crate::db::schema::run_migrations;
use crate::queries::schema::relation_names;

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

/// Result of the setup command execution
#[derive(Debug, Serialize)]
pub struct SetupResult {
    pub relations: Vec<RelationStatus>,
    pub created_new: bool,
    pub dry_run: bool,
    pub config_file_created: bool,
    pub config_file_path: Option<String>,
}

impl Execute for SetupCmd {
    type Output = SetupResult;

    fn execute(self, db: &dyn DatabaseBackend) -> Result<Self::Output, Box<dyn Error>> {
        let mut relations = Vec::new();
        let mut config_file_created = false;
        let mut config_file_path: Option<String> = None;

        // Handle config file creation before processing schema
        if let Some(config_type) = &self.init_config {
            let config = match config_type.as_str() {
                "sqlite" => {
                    crate::config::ConfigFile {
                        database: crate::config::DatabaseConfigFile::Sqlite {
                            path: std::path::PathBuf::from(&self.sqlite_path),
                        },
                    }
                }
                "memory" => {
                    crate::config::ConfigFile {
                        database: crate::config::DatabaseConfigFile::Mem,
                    }
                }
                "postgres" => {
                    // Validate required fields
                    let host = self.pg_host.as_ref().ok_or("--pg-host required for postgres")?;
                    let user = self.pg_user.as_ref().ok_or("--pg-user required for postgres")?;
                    let database = self.pg_database.as_ref().ok_or("--pg-database required for postgres")?;

                    crate::config::ConfigFile {
                        database: crate::config::DatabaseConfigFile::Postgres(
                            crate::db::PostgresConfig {
                                connection_string: None,
                                host: Some(host.clone()),
                                port: self.pg_port.unwrap_or(5432),
                                user: Some(user.clone()),
                                password: self.pg_password.clone(),
                                database: Some(database.clone()),
                                ssl: self.pg_ssl,
                                graph_name: self.pg_graph.clone(),
                            }
                        ),
                    }
                }
                other => return Err(format!("Unknown config type: {}", other).into()),
            };

            // Write config file unless in dry-run mode
            if !self.dry_run {
                let config_path = std::env::current_dir()?.join(".code_search.json");
                let config_json = serde_json::to_string_pretty(&config)?;
                std::fs::write(&config_path, config_json)?;
                config_file_created = true;
                config_file_path = Some(config_path.to_string_lossy().to_string());
            }
        }

        if self.dry_run {
            // In dry-run mode, just show what would be created
            for rel_name in relation_names() {
                relations.push(RelationStatus {
                    name: rel_name.to_string(),
                    status: RelationState::WouldCreate,
                });
            }

            return Ok(SetupResult {
                relations,
                created_new: false,
                dry_run: true,
                config_file_created: false,
                config_file_path: None,
            });
        }

        if self.force {
            // TODO: Implement drop_schema if needed for true drop+recreate
        }

        // Run migrations to ensure schema is initialized
        // Migrations are idempotent, so this is safe to call multiple times
        run_migrations(db)?;

        // Report that schema has been initialized
        for rel_name in relation_names() {
            relations.push(RelationStatus {
                name: rel_name.to_string(),
                status: RelationState::AlreadyExists,
            });
        }

        Ok(SetupResult {
            relations,
            created_new: false,
            dry_run: false,
            config_file_created,
            config_file_path,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_db;
    use crate::test_utils;
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
            init_config: None,
            pg_host: None,
            pg_port: None,
            pg_user: None,
            pg_password: None,
            pg_database: None,
            pg_ssl: false,
            pg_graph: "call_graph".to_string(),
            sqlite_path: "./cozo.sqlite".to_string(),
        };

        let backend = open_db(db_file.path()).expect("Failed to open db");
        let result = cmd.execute(backend.as_ref()).expect("Setup should succeed");

        // Should report 7 relations
        assert_eq!(result.relations.len(), 7);

        // All should be marked as existing (migrations are idempotent)
        assert!(result
            .relations
            .iter()
            .all(|r| matches!(r.status, RelationState::AlreadyExists)));

        // Migrations are idempotent, created_new is always false after auto-migration on connect
        assert!(!result.created_new);
        assert!(!result.config_file_created);
    }

    #[rstest]
    fn test_setup_idempotent(db_file: NamedTempFile) {
        let backend = open_db(db_file.path()).expect("Failed to open db");

        // First setup
        let cmd1 = SetupCmd {
            force: false,
            dry_run: false,
            init_config: None,
            pg_host: None,
            pg_port: None,
            pg_user: None,
            pg_password: None,
            pg_database: None,
            pg_ssl: false,
            pg_graph: "call_graph".to_string(),
            sqlite_path: "./cozo.sqlite".to_string(),
        };
        let result1 = cmd1.execute(backend.as_ref()).expect("First setup should succeed");

        // Migrations ran on connect, so relations already exist
        assert_eq!(result1.relations.len(), 7);
        assert!(result1
            .relations
            .iter()
            .all(|r| matches!(r.status, RelationState::AlreadyExists)));
        assert!(!result1.created_new);

        // Second setup should also succeed (idempotent)
        let cmd2 = SetupCmd {
            force: false,
            dry_run: false,
            init_config: None,
            pg_host: None,
            pg_port: None,
            pg_user: None,
            pg_password: None,
            pg_database: None,
            pg_ssl: false,
            pg_graph: "call_graph".to_string(),
            sqlite_path: "./cozo.sqlite".to_string(),
        };
        let result2 = cmd2.execute(backend.as_ref()).expect("Second setup should succeed");

        // Should still have 7 relations, all already existing
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
            init_config: None,
            pg_host: None,
            pg_port: None,
            pg_user: None,
            pg_password: None,
            pg_database: None,
            pg_ssl: false,
            pg_graph: "call_graph".to_string(),
            sqlite_path: "./cozo.sqlite".to_string(),
        };

        let backend = open_db(db_file.path()).expect("Failed to open db");

        let result = cmd.execute(backend.as_ref()).expect("Setup should succeed");

        assert!(result.dry_run);
        assert_eq!(result.relations.len(), 7);

        // All should be in would_create state
        assert!(result
            .relations
            .iter()
            .all(|r| matches!(r.status, RelationState::WouldCreate)));

        // Should not have actually created anything
        assert!(!result.created_new);
        assert!(!result.config_file_created);
    }

    #[rstest]
    fn test_setup_relations_have_correct_names(db_file: NamedTempFile) {
        let cmd = SetupCmd {
            force: false,
            dry_run: true,
            init_config: None,
            pg_host: None,
            pg_port: None,
            pg_user: None,
            pg_password: None,
            pg_database: None,
            pg_ssl: false,
            pg_graph: "call_graph".to_string(),
            sqlite_path: "./cozo.sqlite".to_string(),
        };

        let backend = open_db(db_file.path()).expect("Failed to open db");

        let result = cmd.execute(backend.as_ref()).expect("Setup should succeed");

        let relation_names: Vec<_> = result.relations.iter().map(|r| r.name.as_str()).collect();

        assert!(relation_names.contains(&"modules"));
        assert!(relation_names.contains(&"functions"));
        assert!(relation_names.contains(&"calls"));
        assert!(relation_names.contains(&"struct_fields"));
        assert!(relation_names.contains(&"function_locations"));
        assert!(relation_names.contains(&"specs"));
        assert!(relation_names.contains(&"types"));
    }

    #[rstest]
    fn test_setup_init_config_sqlite(db_file: NamedTempFile) {
        let _lock = test_utils::global_test_lock().lock();
        let temp_dir = tempfile::tempdir().unwrap();
        let old_cwd = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        let cmd = SetupCmd {
            force: false,
            dry_run: false,
            init_config: Some("sqlite".to_string()),
            pg_host: None,
            pg_port: None,
            pg_user: None,
            pg_password: None,
            pg_database: None,
            pg_ssl: false,
            pg_graph: "call_graph".to_string(),
            sqlite_path: "./test.db".to_string(),
        };

        let backend = open_db(db_file.path()).expect("Failed to open db");
        let result = cmd.execute(backend.as_ref()).expect("Setup should succeed");

        // Should have created config file
        assert!(result.config_file_created);
        assert!(result.config_file_path.is_some());

        // Config file should exist
        let config_path = temp_dir.path().join(".code_search.json");
        assert!(config_path.exists());

        // Config file should be valid JSON
        let content = std::fs::read_to_string(&config_path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&content).expect("Invalid JSON");
        assert_eq!(parsed["database"]["type"], "sqlite");
        assert_eq!(parsed["database"]["path"], "./test.db");

        std::env::set_current_dir(old_cwd).unwrap();
    }

    #[rstest]
    fn test_setup_init_config_memory(db_file: NamedTempFile) {
        let _lock = test_utils::global_test_lock().lock();
        let temp_dir = tempfile::tempdir().unwrap();
        let old_cwd = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        let cmd = SetupCmd {
            force: false,
            dry_run: false,
            init_config: Some("memory".to_string()),
            pg_host: None,
            pg_port: None,
            pg_user: None,
            pg_password: None,
            pg_database: None,
            pg_ssl: false,
            pg_graph: "call_graph".to_string(),
            sqlite_path: "./cozo.sqlite".to_string(),
        };

        let backend = open_db(db_file.path()).expect("Failed to open db");
        let result = cmd.execute(backend.as_ref()).expect("Setup should succeed");

        // Should have created config file
        assert!(result.config_file_created);

        // Config file should exist
        let config_path = temp_dir.path().join(".code_search.json");
        assert!(config_path.exists());

        // Config file should be valid JSON
        let content = std::fs::read_to_string(&config_path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&content).expect("Invalid JSON");
        assert_eq!(parsed["database"]["type"], "memory");

        std::env::set_current_dir(old_cwd).unwrap();
    }

    #[rstest]
    fn test_setup_init_config_postgres(db_file: NamedTempFile) {
        let _lock = test_utils::global_test_lock().lock();
        let temp_dir = tempfile::tempdir().unwrap();
        let old_cwd = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        let cmd = SetupCmd {
            force: false,
            dry_run: false,
            init_config: Some("postgres".to_string()),
            pg_host: Some("localhost".to_string()),
            pg_port: Some(5432),
            pg_user: Some("myuser".to_string()),
            pg_password: Some("mypass".to_string()),
            pg_database: Some("mydb".to_string()),
            pg_ssl: true,
            pg_graph: "call_graph".to_string(),
            sqlite_path: "./cozo.sqlite".to_string(),
        };

        let backend = open_db(db_file.path()).expect("Failed to open db");
        let result = cmd.execute(backend.as_ref()).expect("Setup should succeed");

        // Should have created config file
        assert!(result.config_file_created);

        // Config file should exist
        let config_path = temp_dir.path().join(".code_search.json");
        assert!(config_path.exists());

        // Config file should be valid JSON with postgres settings
        let content = std::fs::read_to_string(&config_path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&content).expect("Invalid JSON");
        assert_eq!(parsed["database"]["type"], "postgres");
        assert_eq!(parsed["database"]["host"], "localhost");
        assert_eq!(parsed["database"]["port"], 5432);
        assert_eq!(parsed["database"]["user"], "myuser");
        assert_eq!(parsed["database"]["password"], "mypass");
        assert_eq!(parsed["database"]["database"], "mydb");
        assert_eq!(parsed["database"]["ssl"], true);
        assert_eq!(parsed["database"]["graph_name"], "call_graph");

        std::env::set_current_dir(old_cwd).unwrap();
    }

    #[rstest]
    fn test_setup_init_config_postgres_missing_host(db_file: NamedTempFile) {
        let cmd = SetupCmd {
            force: false,
            dry_run: false,
            init_config: Some("postgres".to_string()),
            pg_host: None,
            pg_port: None,
            pg_user: Some("myuser".to_string()),
            pg_password: None,
            pg_database: Some("mydb".to_string()),
            pg_ssl: false,
            pg_graph: "call_graph".to_string(),
            sqlite_path: "./cozo.sqlite".to_string(),
        };

        let backend = open_db(db_file.path()).expect("Failed to open db");
        let result = cmd.execute(backend.as_ref());

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("--pg-host"));
    }

    #[rstest]
    fn test_setup_init_config_postgres_missing_user(db_file: NamedTempFile) {
        let cmd = SetupCmd {
            force: false,
            dry_run: false,
            init_config: Some("postgres".to_string()),
            pg_host: Some("localhost".to_string()),
            pg_port: None,
            pg_user: None,
            pg_password: None,
            pg_database: Some("mydb".to_string()),
            pg_ssl: false,
            pg_graph: "call_graph".to_string(),
            sqlite_path: "./cozo.sqlite".to_string(),
        };

        let backend = open_db(db_file.path()).expect("Failed to open db");
        let result = cmd.execute(backend.as_ref());

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("--pg-user"));
    }

    #[rstest]
    fn test_setup_init_config_postgres_missing_database(db_file: NamedTempFile) {
        let cmd = SetupCmd {
            force: false,
            dry_run: false,
            init_config: Some("postgres".to_string()),
            pg_host: Some("localhost".to_string()),
            pg_port: None,
            pg_user: Some("myuser".to_string()),
            pg_password: None,
            pg_database: None,
            pg_ssl: false,
            pg_graph: "call_graph".to_string(),
            sqlite_path: "./cozo.sqlite".to_string(),
        };

        let backend = open_db(db_file.path()).expect("Failed to open db");
        let result = cmd.execute(backend.as_ref());

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("--pg-database"));
    }

    #[rstest]
    fn test_setup_init_config_dry_run_no_file(db_file: NamedTempFile) {
        let _lock = test_utils::global_test_lock().lock();
        let temp_dir = tempfile::tempdir().unwrap();
        let old_cwd = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        let cmd = SetupCmd {
            force: false,
            dry_run: true,
            init_config: Some("sqlite".to_string()),
            pg_host: None,
            pg_port: None,
            pg_user: None,
            pg_password: None,
            pg_database: None,
            pg_ssl: false,
            pg_graph: "call_graph".to_string(),
            sqlite_path: "./test.db".to_string(),
        };

        let backend = open_db(db_file.path()).expect("Failed to open db");
        let result = cmd.execute(backend.as_ref()).expect("Setup should succeed");

        // In dry-run mode, config file should not be created
        assert!(!result.config_file_created);

        // Config file should not exist
        let config_path = temp_dir.path().join(".code_search.json");
        assert!(!config_path.exists());

        std::env::set_current_dir(old_cwd).unwrap();
    }

    #[rstest]
    fn test_setup_init_config_invalid_type(db_file: NamedTempFile) {
        let cmd = SetupCmd {
            force: false,
            dry_run: false,
            init_config: Some("invalid_type".to_string()),
            pg_host: None,
            pg_port: None,
            pg_user: None,
            pg_password: None,
            pg_database: None,
            pg_ssl: false,
            pg_graph: "call_graph".to_string(),
            sqlite_path: "./cozo.sqlite".to_string(),
        };

        let backend = open_db(db_file.path()).expect("Failed to open db");
        let result = cmd.execute(backend.as_ref());

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unknown config type"));
    }
}
