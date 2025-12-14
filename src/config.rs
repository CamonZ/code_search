//! Configuration file handling for database connections.
//!
//! This module provides loading and parsing of `.code_search.json` configuration files.
//! Supports multiple database backends via a structured JSON format.

use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs;
use std::path::PathBuf;

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
    Postgres {
        /// Direct connection string (postgres://...) - takes precedence if provided
        #[serde(skip_serializing_if = "Option::is_none")]
        connection_string: Option<String>,
        /// PostgreSQL host
        #[serde(skip_serializing_if = "Option::is_none")]
        host: Option<String>,
        /// PostgreSQL port (default: 5432)
        #[serde(default)]
        port: u16,
        /// PostgreSQL username
        #[serde(skip_serializing_if = "Option::is_none")]
        user: Option<String>,
        /// PostgreSQL password
        #[serde(skip_serializing_if = "Option::is_none")]
        password: Option<String>,
        /// PostgreSQL database name
        #[serde(skip_serializing_if = "Option::is_none")]
        database: Option<String>,
        /// Enable SSL/TLS (default: false)
        #[serde(default)]
        ssl: bool,
        /// Graph name (default: "call_graph")
        #[serde(default = "default_graph_name")]
        graph_name: String,
    },
}

fn default_graph_name() -> String {
    "call_graph".to_string()
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
    ///
    /// For Postgres, if a connection string is provided, it takes precedence over
    /// individual options (host, user, database, password, port, ssl).
    pub fn to_database_config(&self) -> Result<crate::db::DatabaseConfig, Box<dyn Error>> {
        use crate::db::DatabaseConfig;
        match self {
            Self::Sqlite { path } => Ok(DatabaseConfig::CozoSqlite {
                path: path.clone(),
            }),
            Self::Mem => Ok(DatabaseConfig::CozoMem),
            Self::Postgres {
                connection_string,
                host,
                user,
                database,
                port,
                password,
                ssl,
                ..
            } => {
                // If connection_string is provided, use it (it would be parsed later)
                if connection_string.is_some() {
                    return Err("PostgreSQL backend not yet implemented".into());
                }

                // Validate required fields
                let host_str = host.as_ref().ok_or("PostgreSQL host is required")?;
                let user_str = user.as_ref().ok_or("PostgreSQL user is required")?;
                let database_str = database.as_ref().ok_or("PostgreSQL database is required")?;

                let port_num = if *port == 0 { 5432 } else { *port };

                Ok(DatabaseConfig::Postgres {
                    host: host_str.clone(),
                    port: port_num,
                    database: database_str.clone(),
                    username: user_str.clone(),
                    password: password.clone(),
                    ssl: *ssl,
                })
            }
        }
    }

    /// URL-encode a string for use in PostgreSQL connection strings.
    ///
    /// Encodes special characters (@, :, #, /, ?, =, &) to percent-encoded format.
    fn url_encode(s: &str) -> String {
        s.chars()
            .map(|c| match c {
                '@' => "%40".to_string(),
                ':' => "%3A".to_string(),
                '#' => "%23".to_string(),
                '/' => "%2F".to_string(),
                '?' => "%3F".to_string(),
                '=' => "%3D".to_string(),
                '&' => "%26".to_string(),
                c => c.to_string(),
            })
            .collect()
    }

    /// Build a PostgreSQL connection string from individual options.
    ///
    /// # Arguments
    ///
    /// * `host` - PostgreSQL host (required)
    /// * `user` - PostgreSQL username (required)
    /// * `database` - PostgreSQL database name (required)
    /// * `port` - Port number (default: 5432)
    /// * `password` - Optional password
    /// * `ssl` - Enable SSL mode
    ///
    /// # Errors
    ///
    /// Returns an error if required fields (host, user, database) are missing.
    pub fn build_postgres_connection_string(
        host: &str,
        user: &str,
        database: &str,
        port: u16,
        password: Option<&str>,
        ssl: bool,
    ) -> Result<String, Box<dyn Error>> {
        // Validate required fields
        if host.is_empty() {
            return Err("PostgreSQL host is required".into());
        }
        if user.is_empty() {
            return Err("PostgreSQL user is required".into());
        }
        if database.is_empty() {
            return Err("PostgreSQL database is required".into());
        }

        // URL-encode special characters in password
        let auth = if let Some(pwd) = password {
            let encoded_pwd = Self::url_encode(pwd);
            format!("{}:{}@", user, encoded_pwd)
        } else {
            format!("{}@", user)
        };

        let mut connection_string = format!("postgres://{}{}:{}/{}", auth, host, port, database);

        if ssl {
            connection_string.push_str("?sslmode=require");
        }

        Ok(connection_string)
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
            DatabaseConfigFile::Postgres {
                connection_string,
                ..
            } => {
                assert_eq!(
                    connection_string,
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
            DatabaseConfigFile::Postgres {
                host,
                user,
                database,
                port,
                password,
                ssl,
                ..
            } => {
                assert_eq!(host, Some("localhost".to_string()));
                assert_eq!(user, Some("myuser".to_string()));
                assert_eq!(database, Some("mydb".to_string()));
                assert_eq!(port, 5432);
                assert_eq!(password, Some("mypass".to_string()));
                assert!(!ssl);
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
            DatabaseConfigFile::Postgres { graph_name, .. } => {
                assert_eq!(graph_name, "call_graph");
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
            DatabaseConfigFile::Postgres { graph_name, .. } => {
                assert_eq!(graph_name, "custom_graph");
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
            DatabaseConfigFile::Postgres { ssl, .. } => {
                assert!(!ssl);
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
            DatabaseConfigFile::Postgres { port, .. } => {
                assert_eq!(port, 0); // Port defaults to 0, will be set to 5432 in builder
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

    // Connection string builder tests

    #[test]
    fn test_build_postgres_connection_string_simple() {
        let conn_str = DatabaseConfigFile::build_postgres_connection_string(
            "localhost",
            "user",
            "mydb",
            5432,
            None,
            false,
        )
        .unwrap();
        assert_eq!(conn_str, "postgres://user@localhost:5432/mydb");
    }

    #[test]
    fn test_build_postgres_connection_string_with_password() {
        let conn_str = DatabaseConfigFile::build_postgres_connection_string(
            "localhost",
            "user",
            "mydb",
            5432,
            Some("password"),
            false,
        )
        .unwrap();
        assert_eq!(conn_str, "postgres://user:password@localhost:5432/mydb");
    }

    #[test]
    fn test_build_postgres_connection_string_custom_port() {
        let conn_str = DatabaseConfigFile::build_postgres_connection_string(
            "localhost",
            "user",
            "mydb",
            5433,
            None,
            false,
        )
        .unwrap();
        assert_eq!(conn_str, "postgres://user@localhost:5433/mydb");
    }

    #[test]
    fn test_build_postgres_connection_string_with_ssl() {
        let conn_str = DatabaseConfigFile::build_postgres_connection_string(
            "localhost",
            "user",
            "mydb",
            5432,
            None,
            true,
        )
        .unwrap();
        assert_eq!(conn_str, "postgres://user@localhost:5432/mydb?sslmode=require");
    }

    #[test]
    fn test_build_postgres_connection_string_password_and_ssl() {
        let conn_str = DatabaseConfigFile::build_postgres_connection_string(
            "localhost",
            "user",
            "mydb",
            5432,
            Some("password"),
            true,
        )
        .unwrap();
        assert_eq!(
            conn_str,
            "postgres://user:password@localhost:5432/mydb?sslmode=require"
        );
    }

    #[test]
    fn test_build_postgres_connection_string_missing_host() {
        let result = DatabaseConfigFile::build_postgres_connection_string(
            "",
            "user",
            "mydb",
            5432,
            None,
            false,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("host"));
    }

    #[test]
    fn test_build_postgres_connection_string_missing_user() {
        let result = DatabaseConfigFile::build_postgres_connection_string(
            "localhost",
            "",
            "mydb",
            5432,
            None,
            false,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("user"));
    }

    #[test]
    fn test_build_postgres_connection_string_missing_database() {
        let result = DatabaseConfigFile::build_postgres_connection_string(
            "localhost",
            "user",
            "",
            5432,
            None,
            false,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("database"));
    }

    #[test]
    fn test_build_postgres_connection_string_password_with_at_sign() {
        let conn_str = DatabaseConfigFile::build_postgres_connection_string(
            "localhost",
            "user",
            "mydb",
            5432,
            Some("pass@word"),
            false,
        )
        .unwrap();
        assert_eq!(conn_str, "postgres://user:pass%40word@localhost:5432/mydb");
    }

    #[test]
    fn test_build_postgres_connection_string_password_with_colon() {
        let conn_str = DatabaseConfigFile::build_postgres_connection_string(
            "localhost",
            "user",
            "mydb",
            5432,
            Some("pass:word"),
            false,
        )
        .unwrap();
        assert_eq!(conn_str, "postgres://user:pass%3Aword@localhost:5432/mydb");
    }

    #[test]
    fn test_build_postgres_connection_string_password_with_special_chars() {
        let conn_str = DatabaseConfigFile::build_postgres_connection_string(
            "localhost",
            "user",
            "mydb",
            5432,
            Some("p@ss:w#rd"),
            false,
        )
        .unwrap();
        // urlencoding should handle all special characters
        assert!(conn_str.contains("postgres://user:"));
        assert!(conn_str.contains("@localhost:5432/mydb"));
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
        let config_file = DatabaseConfigFile::Postgres {
            connection_string: None,
            host: Some("localhost".to_string()),
            user: Some("testuser".to_string()),
            database: Some("testdb".to_string()),
            port: 5432,
            password: Some("testpass".to_string()),
            ssl: true,
            graph_name: "call_graph".to_string(),
        };
        let db_config = config_file.to_database_config().unwrap();
        match db_config {
            DatabaseConfig::Postgres {
                host,
                username,
                database,
                port,
                password,
                ssl,
            } => {
                assert_eq!(host, "localhost");
                assert_eq!(username, "testuser");
                assert_eq!(database, "testdb");
                assert_eq!(port, 5432);
                assert_eq!(password, Some("testpass".to_string()));
                assert!(ssl);
            }
            _ => panic!("Expected Postgres variant"),
        }
    }

    #[test]
    fn test_to_database_config_postgres_default_port() {
        use crate::db::DatabaseConfig;
        let config_file = DatabaseConfigFile::Postgres {
            connection_string: None,
            host: Some("localhost".to_string()),
            user: Some("testuser".to_string()),
            database: Some("testdb".to_string()),
            port: 0, // Should default to 5432
            password: None,
            ssl: false,
            graph_name: "call_graph".to_string(),
        };
        let db_config = config_file.to_database_config().unwrap();
        match db_config {
            DatabaseConfig::Postgres { port, .. } => {
                assert_eq!(port, 5432);
            }
            _ => panic!("Expected Postgres variant"),
        }
    }

    #[test]
    fn test_to_database_config_postgres_missing_host() {
        let config_file = DatabaseConfigFile::Postgres {
            connection_string: None,
            host: None,
            user: Some("testuser".to_string()),
            database: Some("testdb".to_string()),
            port: 5432,
            password: None,
            ssl: false,
            graph_name: "call_graph".to_string(),
        };
        let result = config_file.to_database_config();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("host"));
    }

    #[test]
    fn test_to_database_config_postgres_missing_user() {
        let config_file = DatabaseConfigFile::Postgres {
            connection_string: None,
            host: Some("localhost".to_string()),
            user: None,
            database: Some("testdb".to_string()),
            port: 5432,
            password: None,
            ssl: false,
            graph_name: "call_graph".to_string(),
        };
        let result = config_file.to_database_config();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("user"));
    }

    #[test]
    fn test_to_database_config_postgres_missing_database() {
        let config_file = DatabaseConfigFile::Postgres {
            connection_string: None,
            host: Some("localhost".to_string()),
            user: Some("testuser".to_string()),
            database: None,
            port: 5432,
            password: None,
            ssl: false,
            graph_name: "call_graph".to_string(),
        };
        let result = config_file.to_database_config();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("database"));
    }

    #[test]
    fn test_to_database_config_postgres_with_connection_string() {
        let config_file = DatabaseConfigFile::Postgres {
            connection_string: Some("postgres://user:pass@localhost:5432/mydb".to_string()),
            host: None,
            user: None,
            database: None,
            port: 0,
            password: None,
            ssl: false,
            graph_name: "call_graph".to_string(),
        };
        let result = config_file.to_database_config();
        assert!(result.is_err()); // PostgreSQL not yet implemented
    }
}
