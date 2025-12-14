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
}

impl Execute for SetupCmd {
    type Output = SetupResult;

    fn execute(self, db: &dyn DatabaseBackend) -> Result<Self::Output, Box<dyn Error>> {
        let mut relations = Vec::new();
        let mut created_new = false;

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
            created_new,
            dry_run: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_db;
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
    }

    #[rstest]
    fn test_setup_idempotent(db_file: NamedTempFile) {
        let backend = open_db(db_file.path()).expect("Failed to open db");

        // First setup
        let cmd1 = SetupCmd {
            force: false,
            dry_run: false,
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
    }

    #[rstest]
    fn test_setup_relations_have_correct_names(db_file: NamedTempFile) {
        let cmd = SetupCmd {
            force: false,
            dry_run: true,
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
}
