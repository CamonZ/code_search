use serde::Serialize;
use std::error::Error;

use super::descriptions::{descriptions_by_category, get_description, CommandDescription};
use super::DescribeCmd;
use crate::commands::Execute;
use crate::db::DatabaseBackend;

/// Output for listing all commands by category
#[derive(Debug, Clone, Serialize)]
pub struct CategoryListing {
    pub category: String,
    pub commands: Vec<(String, String)>, // (name, brief)
}

/// Output for describe mode
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum DescribeMode {
    ListAll {
        categories: Vec<CategoryListing>,
    },
    Specific {
        descriptions: Vec<CommandDescription>,
    },
}

/// Result of the describe command
#[derive(Debug, Serialize)]
pub struct DescribeResult {
    #[serde(flatten)]
    pub mode: DescribeMode,
}

impl Execute for DescribeCmd {
    type Output = DescribeResult;

    fn execute(self, _db: &dyn DatabaseBackend) -> Result<Self::Output, Box<dyn Error>> {
        if self.commands.is_empty() {
            // List all commands grouped by category
            let categories_map = descriptions_by_category();
            let mut categories = Vec::new();

            for (category, commands) in categories_map {
                categories.push(CategoryListing {
                    category: category.to_string(),
                    commands,
                });
            }

            Ok(DescribeResult {
                mode: DescribeMode::ListAll { categories },
            })
        } else {
            // Get descriptions for specified commands
            let mut descriptions = Vec::new();

            for cmd_name in self.commands {
                match get_description(&cmd_name) {
                    Some(desc) => descriptions.push(desc),
                    None => {
                        return Err(format!("Unknown command: '{}'", cmd_name).into());
                    }
                }
            }

            Ok(DescribeResult {
                mode: DescribeMode::Specific { descriptions },
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_describe_all_lists_categories() {
        use crate::db::open_mem_db;
        let cmd = DescribeCmd { commands: vec![] };

        let db = open_mem_db(true).expect("Failed to create test database");
        let result = cmd.execute(db.as_ref()).expect("Should succeed");

        match result.mode {
            DescribeMode::ListAll { ref categories } => {
                assert!(!categories.is_empty());
                // Check we have commands in categories
                let total_commands: usize = categories.iter().map(|c| c.commands.len()).sum();
                assert!(total_commands > 0);
            }
            _ => panic!("Expected ListAll mode"),
        }
    }

    #[test]
    fn test_describe_specific_command() {
        use crate::db::open_mem_db;
        let cmd = DescribeCmd {
            commands: vec!["calls-to".to_string()],
        };

        let db = open_mem_db(true).expect("Failed to create test database");
        let result = cmd.execute(db.as_ref()).expect("Should succeed");

        match result.mode {
            DescribeMode::Specific { ref descriptions } => {
                assert_eq!(descriptions.len(), 1);
                assert_eq!(descriptions[0].name, "calls-to");
            }
            _ => panic!("Expected Specific mode"),
        }
    }

    #[test]
    fn test_describe_multiple_commands() {
        use crate::db::open_mem_db;
        let cmd = DescribeCmd {
            commands: vec![
                "calls-to".to_string(),
                "calls-from".to_string(),
                "trace".to_string(),
            ],
        };

        let db = open_mem_db(true).expect("Failed to create test database");
        let result = cmd.execute(db.as_ref()).expect("Should succeed");

        match result.mode {
            DescribeMode::Specific { ref descriptions } => {
                assert_eq!(descriptions.len(), 3);
                let names: Vec<_> = descriptions.iter().map(|d| d.name.as_str()).collect();
                assert!(names.contains(&"calls-to"));
                assert!(names.contains(&"calls-from"));
                assert!(names.contains(&"trace"));
            }
            _ => panic!("Expected Specific mode"),
        }
    }

    #[test]
    fn test_describe_unknown_command() {
        use crate::db::open_mem_db;
        let cmd = DescribeCmd {
            commands: vec!["nonexistent".to_string()],
        };

        let db = open_mem_db(true).expect("Failed to create test database");
        let result = cmd.execute(db.as_ref());
        assert!(result.is_err());
    }
}
