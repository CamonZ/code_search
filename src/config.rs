//! Configuration file handling for database connections.
//!
//! This module provides loading and parsing of `.code_search.json` configuration files.
//! Supports multiple database backends via a structured JSON format.

use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs;
use std::path::PathBuf;

use crate::db::PostgresConfig;

/// Top-level configuration file structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigFile {
    /// Database configuration
    pub database: DatabaseConfigFile,
}

/// Database configuration variants for different backends.
///
/// Supports Sqlite, Memory, and Postgres backends with type-tagged enum.
/// JSON format uses "type" field with lowercase variant names.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum DatabaseConfigFile {
    /// SQLite backend with file path
    Sqlite {
        path: PathBuf,
    },
    /// In-memory backend for testing
    #[serde(rename = "memory")]
    Mem,
    /// PostgreSQL backend
    #[serde(rename = "postgres")]
    Postgres(PostgresConfig),
}

impl ConfigFile {
    /// Load configuration from `.code_search.json` in the current directory.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The config file doesn't exist
    /// - The file cannot be read
    /// - The JSON is invalid
    /// - Required fields are missing
    pub fn load() -> Result<Self, Box<dyn Error>> {
        let config_path = PathBuf::from(".code_search.json");

        if !config_path.exists() {
            return Err(format!(
                "Configuration file not found: .code_search.json\n\n\
                 Please create a .code_search.json file in the current directory.\n\n\
                 Examples:\n\
                 \n\
                 SQLite:\n\
                 {{\n  \
                   \"database\": {{\n    \
                     \"type\": \"sqlite\",\n    \
                     \"path\": \"./cozo.sqlite\"\n  \
                   }}\n\
                 }}\n\
                 \n\
                 In-memory:\n\
                 {{\n  \
                   \"database\": {{\n    \
                     \"type\": \"memory\"\n  \
                   }}\n\
                 }}\n\
                 \n\
                 PostgreSQL (with connection string):\n\
                 {{\n  \
                   \"database\": {{\n    \
                     \"type\": \"postgres\",\n    \
                     \"connection_string\": \"postgres://user:pass@localhost:5432/mydb\"\n  \
                   }}\n\
                 }}\n\
                 \n\
                 PostgreSQL (with individual options):\n\
                 {{\n  \
                   \"database\": {{\n    \
                     \"type\": \"postgres\",\n    \
                     \"host\": \"localhost\",\n    \
                     \"user\": \"myuser\",\n    \
                     \"database\": \"mydb\",\n    \
                     \"port\": 5432,\n    \
                     \"password\": \"mypass\",\n    \
                     \"ssl\": false,\n    \
                     \"graph_name\": \"call_graph\"\n  \
                   }}\n\
                 }}\n"
            ).into());
        }

        let content = fs::read_to_string(&config_path).map_err(|e| {
            Box::new(std::io::Error::new(std::io::ErrorKind::Other,
                format!("Failed to read .code_search.json: {}", e))) as Box<dyn Error>
        })?;

        let config: ConfigFile = serde_json::from_str(&content).map_err(|e| {
            Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData,
                format!("Invalid JSON in .code_search.json: {}", e))) as Box<dyn Error>
        })?;

        Ok(config)
    }
}

impl DatabaseConfigFile {
    /// Convert this configuration to a DatabaseConfig.
    pub fn to_database_config(&self) -> Result<crate::db::DatabaseConfig, Box<dyn Error>> {
        use crate::db::DatabaseConfig;
        match self {
            Self::Sqlite { path } => Ok(DatabaseConfig::CozoSqlite {
                path: path.clone(),
            }),
            Self::Mem => Ok(DatabaseConfig::CozoMem),
            Self::Postgres(pg_config) => Ok(DatabaseConfig::Postgres(pg_config.clone())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;

    #[test]
    fn test_sqlite_deserialization() {
        let json = r#"
        {
            "database": {
                "type": "sqlite",
                "path": "./cozo.sqlite"
            }
        }
        "#;
        let config: ConfigFile = serde_json::from_str(json).unwrap();
        assert!(matches!(config.database, DatabaseConfigFile::Sqlite { .. }));
    }

    #[test]
    fn test_memory_deserialization() {
        let json = r#"
        {
            "database": {
                "type": "memory"
            }
        }
        "#;
        let config: ConfigFile = serde_json::from_str(json).unwrap();
        assert!(matches!(config.database, DatabaseConfigFile::Mem));
    }

    #[test]
    fn test_postgres_with_connection_string() {
        let json = r#"
        {
            "database": {
                "type": "postgres",
                "connection_string": "postgres://user:pass@localhost:5432/mydb"
            }
        }
        "#;
        let config: ConfigFile = serde_json::from_str(json).unwrap();
        match config.database {
            DatabaseConfigFile::Postgres(pg_config) => {
                assert_eq!(
                    pg_config.connection_string,
                    Some("postgres://user:pass@localhost:5432/mydb".to_string())
                );
            }
            _ => panic!("Expected Postgres variant"),
        }
    }

    #[test]
    fn test_postgres_with_individual_fields() {
        let json = r#"
        {
            "database": {
                "type": "postgres",
                "host": "localhost",
                "user": "myuser",
                "database": "mydb",
                "port": 5432,
                "password": "mypass",
                "ssl": false
            }
        }
        "#;
        let config: ConfigFile = serde_json::from_str(json).unwrap();
        match config.database {
            DatabaseConfigFile::Postgres(pg_config) => {
                assert_eq!(pg_config.host, Some("localhost".to_string()));
                assert_eq!(pg_config.user, Some("myuser".to_string()));
                assert_eq!(pg_config.database, Some("mydb".to_string()));
                assert_eq!(pg_config.port, 5432);
                assert_eq!(pg_config.password, Some("mypass".to_string()));
                assert!(!pg_config.ssl);
            }
            _ => panic!("Expected Postgres variant"),
        }
    }

    #[test]
    fn test_postgres_default_graph_name() {
        let json = r#"
        {
            "database": {
                "type": "postgres",
                "host": "localhost",
                "user": "myuser",
                "database": "mydb"
            }
        }
        "#;
        let config: ConfigFile = serde_json::from_str(json).unwrap();
        match config.database {
            DatabaseConfigFile::Postgres(pg_config) => {
                assert_eq!(pg_config.graph_name, "call_graph");
            }
            _ => panic!("Expected Postgres variant"),
        }
    }

    #[test]
    fn test_postgres_custom_graph_name() {
        let json = r#"
        {
            "database": {
                "type": "postgres",
                "host": "localhost",
                "user": "myuser",
                "database": "mydb",
                "graph_name": "custom_graph"
            }
        }
        "#;
        let config: ConfigFile = serde_json::from_str(json).unwrap();
        match config.database {
            DatabaseConfigFile::Postgres(pg_config) => {
                assert_eq!(pg_config.graph_name, "custom_graph");
            }
            _ => panic!("Expected Postgres variant"),
        }
    }

    #[test]
    fn test_postgres_default_ssl_false() {
        let json = r#"
        {
            "database": {
                "type": "postgres",
                "host": "localhost",
                "user": "myuser",
                "database": "mydb"
            }
        }
        "#;
        let config: ConfigFile = serde_json::from_str(json).unwrap();
        match config.database {
            DatabaseConfigFile::Postgres(pg_config) => {
                assert!(!pg_config.ssl);
            }
            _ => panic!("Expected Postgres variant"),
        }
    }

    #[test]
    fn test_postgres_default_port_zero() {
        let json = r#"
        {
            "database": {
                "type": "postgres",
                "host": "localhost",
                "user": "myuser",
                "database": "mydb"
            }
        }
        "#;
        let config: ConfigFile = serde_json::from_str(json).unwrap();
        match config.database {
            DatabaseConfigFile::Postgres(pg_config) => {
                assert_eq!(pg_config.port, 0); // Port defaults to 0, will be set to 5432 in builder
            }
            _ => panic!("Expected Postgres variant"),
        }
    }

    #[test]
    fn test_load_missing_file() {
        // Change to a temp directory where .code_search.json doesn't exist
        // Use a lock to prevent test interference
        use std::sync::Mutex;
        use std::sync::OnceLock;

        fn test_lock() -> &'static Mutex<()> {
            static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
            LOCK.get_or_init(|| Mutex::new(()))
        }

        let _lock = test_lock().lock();
        let temp_dir = tempfile::tempdir().unwrap();
        let old_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let result = ConfigFile::load();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("not found"));

        std::env::set_current_dir(old_dir).unwrap();
    }

    #[test]
    fn test_load_invalid_json() {
        use std::sync::Mutex;
        use std::sync::OnceLock;

        fn test_lock() -> &'static Mutex<()> {
            static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
            LOCK.get_or_init(|| Mutex::new(()))
        }

        let _lock = test_lock().lock();
        let temp_dir = tempfile::tempdir().unwrap();

        let mut file = NamedTempFile::new_in(&temp_dir).unwrap();
        file.write_all(b"{ invalid json }").unwrap();
        file.flush().unwrap();

        let old_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let result = ConfigFile::load();
        assert!(result.is_err());

        std::env::set_current_dir(old_dir).unwrap();
    }

    #[test]
    fn test_load_valid_sqlite_file() {
        use std::sync::Mutex;
        use std::sync::OnceLock;

        fn test_lock() -> &'static Mutex<()> {
            static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
            LOCK.get_or_init(|| Mutex::new(()))
        }

        let _lock = test_lock().lock();
        let temp_dir = tempfile::tempdir().unwrap();
        let config_path = temp_dir.path().join(".code_search.json");

        let json = r#"
        {
            "database": {
                "type": "sqlite",
                "path": "./cozo.sqlite"
            }
        }
        "#;

        fs::write(&config_path, json).unwrap();

        let old_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let config = ConfigFile::load().unwrap();
        assert!(matches!(config.database, DatabaseConfigFile::Sqlite { .. }));

        std::env::set_current_dir(old_dir).unwrap();
    }

    // to_database_config tests

    #[test]
    fn test_to_database_config_sqlite() {
        use crate::db::DatabaseConfig;
        let config_file = DatabaseConfigFile::Sqlite {
            path: PathBuf::from("/tmp/test.db"),
        };
        let db_config = config_file.to_database_config().unwrap();
        match db_config {
            DatabaseConfig::CozoSqlite { path } => {
                assert_eq!(path, PathBuf::from("/tmp/test.db"));
            }
            _ => panic!("Expected CozoSqlite variant"),
        }
    }

    #[test]
    fn test_to_database_config_memory() {
        use crate::db::DatabaseConfig;
        let config_file = DatabaseConfigFile::Mem;
        let db_config = config_file.to_database_config().unwrap();
        assert!(matches!(db_config, DatabaseConfig::CozoMem));
    }

    #[test]
    fn test_to_database_config_postgres_with_individual_options() {
        use crate::db::DatabaseConfig;
        let config_file = DatabaseConfigFile::Postgres(PostgresConfig {
            connection_string: None,
            host: Some("localhost".to_string()),
            user: Some("testuser".to_string()),
            database: Some("testdb".to_string()),
            port: 5432,
            password: Some("testpass".to_string()),
            ssl: true,
            graph_name: "my_graph".to_string(),
        });
        let db_config = config_file.to_database_config().unwrap();
        match db_config {
            DatabaseConfig::Postgres(pg_config) => {
                // Components are passed through directly
                assert_eq!(pg_config.host, Some("localhost".to_string()));
                assert_eq!(pg_config.user, Some("testuser".to_string()));
                assert_eq!(pg_config.database, Some("testdb".to_string()));
                assert_eq!(pg_config.password, Some("testpass".to_string()));
                assert_eq!(pg_config.port, 5432);
                assert!(pg_config.ssl);
                assert_eq!(pg_config.graph_name, "my_graph");

                // Connection string is built on demand
                let conn_str = pg_config.build_connection_string().unwrap();
                assert!(conn_str.contains("testuser"));
                assert!(conn_str.contains("localhost"));
                assert!(conn_str.contains("testdb"));
            }
            _ => panic!("Expected Postgres variant"),
        }
    }

    #[test]
    fn test_to_database_config_postgres_default_port() {
        use crate::db::DatabaseConfig;
        let config_file = DatabaseConfigFile::Postgres(PostgresConfig {
            connection_string: None,
            host: Some("localhost".to_string()),
            user: Some("testuser".to_string()),
            database: Some("testdb".to_string()),
            port: 0, // Should default to 5432 when building connection string
            password: None,
            ssl: false,
            graph_name: "call_graph".to_string(),
        });
        let db_config = config_file.to_database_config().unwrap();
        match db_config {
            DatabaseConfig::Postgres(pg_config) => {
                assert_eq!(pg_config.port, 0); // Stored as 0
                // But connection string should use default 5432
                let conn_str = pg_config.build_connection_string().unwrap();
                assert!(conn_str.contains("5432"));
            }
            _ => panic!("Expected Postgres variant"),
        }
    }

    #[test]
    fn test_to_database_config_postgres_with_connection_string() {
        use crate::db::DatabaseConfig;
        let config_file = DatabaseConfigFile::Postgres(PostgresConfig {
            connection_string: Some("postgres://user:pass@localhost:5432/mydb".to_string()),
            host: None,
            user: None,
            database: None,
            port: 0,
            password: None,
            ssl: false,
            graph_name: "my_graph".to_string(),
        });
        let db_config = config_file.to_database_config().unwrap();
        match db_config {
            DatabaseConfig::Postgres(pg_config) => {
                assert_eq!(pg_config.connection_string, Some("postgres://user:pass@localhost:5432/mydb".to_string()));
                assert_eq!(pg_config.graph_name, "my_graph");
                // Connection string is used directly
                let conn_str = pg_config.build_connection_string().unwrap();
                assert_eq!(conn_str, "postgres://user:pass@localhost:5432/mydb");
            }
            _ => panic!("Expected Postgres config"),
        }
    }
}
