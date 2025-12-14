use std::error::Error;

use serde::Serialize;

use super::SetupCmd;
use crate::commands::Execute;
use crate::db::DatabaseBackend;
use crate::queries::schema;

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
            });
        }

        if self.force {
            // Drop existing schema by attempting to recreate
            // The schema module will handle checking if relations exist
            // For now, we'll just proceed with creation which handles the existing case
            // TODO: Implement drop_schema if needed for true drop+recreate
        }

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

        // Should create 7 relations
        assert_eq!(result.relations.len(), 7);

        // All should be created
        assert!(result
            .relations
            .iter()
            .all(|r| matches!(r.status, RelationState::Created)));

        assert!(result.created_new);
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
        assert!(result1.created_new);

        // Second setup should find existing relations
        let cmd2 = SetupCmd {
            force: false,
            dry_run: false,
        };
        let result2 = cmd2.execute(backend.as_ref()).expect("Second setup should succeed");

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
