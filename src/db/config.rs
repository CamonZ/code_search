//! Database configuration for runtime backend selection.
//!
//! This module provides configuration parsing and backend instantiation for the database abstraction layer.
//! Supports multiple database backends (currently only CozoDB/SQLite and in-memory for tests).

use std::error::Error;
use std::fs;
use std::path::PathBuf;

use super::backend::DatabaseBackend;
use super::connection::{CozoMemBackend, CozoSqliteBackend};
use super::postgres::PostgresAgeBackend;
use cozo::DbInstance;

/// PostgreSQL connection configuration.
///
/// Supports either a direct connection string or individual connection parameters.
/// If `connection_string` is provided, it takes precedence over individual fields.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PostgresConfig {
    /// Direct connection string (postgres://...) - takes precedence if provided
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection_string: Option<String>,
    /// PostgreSQL host
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
    /// PostgreSQL port (default: 5432)
    #[serde(default)]
    pub port: u16,
    /// PostgreSQL username
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
    /// PostgreSQL password
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    /// PostgreSQL database name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub database: Option<String>,
    /// Enable SSL/TLS
    #[serde(default)]
    pub ssl: bool,
    /// Name of the AGE graph to use (default: "call_graph")
    #[serde(default = "default_graph_name")]
    pub graph_name: String,
}

fn default_graph_name() -> String {
    "call_graph".to_string()
}

impl PostgresConfig {
    /// Build the connection string from configuration.
    ///
    /// If `connection_string` is set, returns it directly.
    /// Otherwise, builds the connection string from individual fields.
    pub fn build_connection_string(&self) -> Result<String, Box<dyn Error>> {
        if let Some(ref conn_str) = self.connection_string {
            return Ok(conn_str.clone());
        }

        let host = self.host.as_ref().ok_or("PostgreSQL host is required")?;
        let user = self.user.as_ref().ok_or("PostgreSQL user is required")?;
        let database = self
            .database
            .as_ref()
            .ok_or("PostgreSQL database is required")?;

        if host.is_empty() {
            return Err("PostgreSQL host is required".into());
        }
        if user.is_empty() {
            return Err("PostgreSQL user is required".into());
        }
        if database.is_empty() {
            return Err("PostgreSQL database is required".into());
        }

        let port = if self.port == 0 { 5432 } else { self.port };

        // URL-encode special characters in password
        let auth = if let Some(ref pwd) = self.password {
            let encoded_pwd = Self::url_encode(pwd);
            format!("{}:{}@", user, encoded_pwd)
        } else {
            format!("{}@", user)
        };

        let mut connection_string = format!("postgres://{}{}:{}/{}", auth, host, port, database);

        if self.ssl {
            connection_string.push_str("?sslmode=require");
        }

        Ok(connection_string)
    }

    /// URL-encode a string for use in PostgreSQL connection strings.
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
}

/// Configuration for database backend selection.
///
/// Enables runtime selection of different database backends via CLI arguments,
/// environment variables, or configuration files.
#[derive(Debug, Clone)]
#[allow(dead_code)] // Future backend variants are not yet constructed
pub enum DatabaseConfig {
    /// Local CozoDB with SQLite storage (current default).
    CozoSqlite { path: PathBuf },

    /// Local CozoDB with in-memory storage (for testing).
    CozoMem,

    /// Local CozoDB with RocksDB storage (future).
    CozoRocksdb { path: PathBuf },

    /// PostgreSQL with Apache AGE extension.
    Postgres(PostgresConfig),
}

impl DatabaseConfig {
    /// Create a backend instance from this configuration.
    pub fn connect(&self) -> Result<Box<dyn DatabaseBackend>, Box<dyn Error>> {
        let backend = match self {
            Self::CozoSqlite { path } => {
                let db = DbInstance::new("sqlite", path, "").map_err(|e| {
                    format!("Failed to open SQLite database at {:?}: {:?}", path, e)
                })?;
                Box::new(CozoSqliteBackend::new(db)) as Box<dyn DatabaseBackend>
            }
            Self::CozoMem => {
                let db = DbInstance::new("mem", "", "")?;
                Box::new(CozoMemBackend::new(db)) as Box<dyn DatabaseBackend>
            }
            Self::CozoRocksdb { .. } => {
                return Err("RocksDB backend not yet implemented".into());
            }
            Self::Postgres(config) => {
                let connection_string = config.build_connection_string()?;
                let backend = PostgresAgeBackend::new(&connection_string, &config.graph_name)?;
                Box::new(backend) as Box<dyn DatabaseBackend>
            }
        };

        Ok(backend)
    }

    /// Parse from a connection URL or file path.
    ///
    /// Supported formats:
    /// - `./path/to/db.sqlite` or `/absolute/path` → CozoSqlite
    /// - `sqlite:///path/to/db` → CozoSqlite
    /// - `:memory:` → CozoMem
    /// - `postgres://user:pass@host:port/db` → Postgres
    pub fn from_url(url: &str) -> Result<Self, Box<dyn Error>> {
        // Memory database
        if url == ":memory:" {
            return Ok(Self::CozoMem);
        }

        // URL-style connection strings
        if let Some(path) = url.strip_prefix("sqlite://") {
            return Ok(Self::CozoSqlite {
                path: PathBuf::from(path),
            });
        }

        if url.starts_with("rocksdb://") {
            return Err("RocksDB backend not yet implemented".into());
        }

        if url.starts_with("postgres://") || url.starts_with("postgresql://") {
            return Self::parse_postgres_url(url);
        }

        // Default: treat as file path (CozoSqlite)
        Ok(Self::CozoSqlite {
            path: PathBuf::from(url),
        })
    }

    /// Parse a PostgreSQL connection URL.
    ///
    /// Format: `postgres://user:pass@host:port/database?graph=graph_name`
    ///
    /// The `graph` query parameter specifies the AGE graph name.
    /// Defaults to "call_graph" if not specified.
    fn parse_postgres_url(url: &str) -> Result<Self, Box<dyn Error>> {
        // Extract graph name from query params if present
        let (base_url, graph_name) = if let Some(idx) = url.find("?graph=") {
            let (base, rest) = url.split_at(idx);
            let graph = rest
                .strip_prefix("?graph=")
                .unwrap_or("call_graph")
                .split('&')
                .next()
                .unwrap_or("call_graph");
            (base.to_string(), graph.to_string())
        } else if let Some(idx) = url.find("&graph=") {
            // graph param might not be first
            let graph_start = idx + 7;
            let graph_end = url[graph_start..]
                .find('&')
                .map(|i| graph_start + i)
                .unwrap_or(url.len());
            let graph = &url[graph_start..graph_end];
            let base = url.replace(&format!("&graph={}", graph), "");
            (base, graph.to_string())
        } else {
            (url.to_string(), "call_graph".to_string())
        };

        Ok(Self::Postgres(PostgresConfig {
            connection_string: Some(base_url),
            host: None,
            port: 0,
            user: None,
            password: None,
            database: None,
            ssl: false,
            graph_name,
        }))
    }

    /// Load from environment variables.
    ///
    /// Checks in order:
    /// 1. DATABASE_URL environment variable
    /// 2. Individual env vars (COZO_PATH, POSTGRES_HOST, etc.)
    pub fn from_env() -> Result<Option<Self>, Box<dyn Error>> {
        if let Ok(url) = std::env::var("DATABASE_URL") {
            return Ok(Some(Self::from_url(&url)?));
        }

        if let Ok(path) = std::env::var("COZO_PATH") {
            return Ok(Some(Self::CozoSqlite {
                path: PathBuf::from(path),
            }));
        }

        Ok(None)
    }

    /// Resolve configuration from config file and environment.
    ///
    /// Priority: Config file > Environment > Default (./cozo.sqlite)
    ///
    /// The preferred method is to use `.code_search.json` configuration file.
    pub fn resolve() -> Result<Self, Box<dyn Error>> {
        // Try loading from config file first (preferred method)
        if let Ok(config_file) = crate::config::ConfigFile::load() {
            return config_file.database.to_database_config();
        }

        // Check environment
        if let Some(config) = Self::from_env()? {
            return Ok(config);
        }

        // Fall back to default
        Self::from_url("./cozo.sqlite")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils;

    #[test]
    fn test_from_url_file_path() {
        let config = DatabaseConfig::from_url("./test.sqlite").unwrap();
        assert!(matches!(config, DatabaseConfig::CozoSqlite { .. }));
    }

    #[test]
    fn test_from_url_absolute_path() {
        let config = DatabaseConfig::from_url("/tmp/test.sqlite").unwrap();
        match config {
            DatabaseConfig::CozoSqlite { path } => {
                assert_eq!(path, PathBuf::from("/tmp/test.sqlite"));
            }
            _ => panic!("Expected CozoSqlite"),
        }
    }

    #[test]
    fn test_from_url_memory() {
        let config = DatabaseConfig::from_url(":memory:").unwrap();
        assert!(matches!(config, DatabaseConfig::CozoMem));
    }

    #[test]
    fn test_from_url_sqlite_scheme() {
        let config = DatabaseConfig::from_url("sqlite:///tmp/test.db").unwrap();
        match config {
            DatabaseConfig::CozoSqlite { path } => {
                assert_eq!(path, PathBuf::from("/tmp/test.db"));
            }
            _ => panic!("Expected CozoSqlite"),
        }
    }

    #[test]
    fn test_from_url_rocksdb_not_implemented() {
        let result = DatabaseConfig::from_url("rocksdb:///tmp/test.db");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("not yet implemented"));
    }

    #[test]
    fn test_parse_postgres_url_with_graph() {
        let config =
            DatabaseConfig::from_url("postgres://user:pass@localhost:5432/mydb?graph=my_graph")
                .unwrap();

        match config {
            DatabaseConfig::Postgres(pg_config) => {
                assert_eq!(
                    pg_config.connection_string,
                    Some("postgres://user:pass@localhost:5432/mydb".to_string())
                );
                assert_eq!(pg_config.graph_name, "my_graph");
            }
            _ => panic!("Expected Postgres config"),
        }
    }

    #[test]
    fn test_parse_postgres_url_default_graph() {
        let config = DatabaseConfig::from_url("postgres://localhost/mydb").unwrap();

        match config {
            DatabaseConfig::Postgres(pg_config) => {
                assert_eq!(pg_config.graph_name, "call_graph");
            }
            _ => panic!("Expected Postgres config"),
        }
    }

    #[test]
    fn test_parse_postgresql_scheme() {
        let config = DatabaseConfig::from_url("postgresql://localhost/mydb").unwrap();

        assert!(matches!(config, DatabaseConfig::Postgres(_)));
    }

    #[test]
    fn test_connect_sqlite() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let config = DatabaseConfig::CozoSqlite {
            path: tmp.path().to_path_buf(),
        };
        let backend = config.connect().unwrap();
        assert_eq!(backend.backend_name(), "CozoSqlite");
    }

    #[test]
    fn test_connect_mem() {
        let config = DatabaseConfig::CozoMem;
        let backend = config.connect().unwrap();
        assert_eq!(backend.backend_name(), "CozoMem");
    }

    #[test]
    fn test_connect_rocksdb_not_implemented() {
        let config = DatabaseConfig::CozoRocksdb {
            path: PathBuf::from("/tmp/test.db"),
        };
        let result = config.connect();
        assert!(result.is_err());
        if let Err(e) = result {
            let err_msg = format!("{}", e);
            assert!(err_msg.contains("not yet implemented"));
        }
    }

    #[test]
    fn test_connect_postgres_config_structure() {
        // Test that the Postgres variant can be created with the new structure
        let config = DatabaseConfig::Postgres(PostgresConfig {
            connection_string: Some("postgres://localhost:5432/test".to_string()),
            host: None,
            port: 0,
            user: None,
            password: None,
            database: None,
            ssl: false,
            graph_name: "call_graph".to_string(),
        });

        // Verify the structure matches what we expect
        match config {
            DatabaseConfig::Postgres(pg_config) => {
                assert_eq!(
                    pg_config.connection_string,
                    Some("postgres://localhost:5432/test".to_string())
                );
                assert_eq!(pg_config.graph_name, "call_graph");
            }
            _ => panic!("Expected Postgres config"),
        }
    }

    #[test]
    fn test_from_env_none() {
        let _lock = test_utils::global_test_lock().lock();
        // Remove env vars if they exist
        unsafe {
            std::env::remove_var("DATABASE_URL");
            std::env::remove_var("COZO_PATH");
        }
        let result = DatabaseConfig::from_env().unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_from_env_database_url() {
        let _lock = test_utils::global_test_lock().lock();
        unsafe {
            std::env::set_var("DATABASE_URL", "sqlite:///tmp/test.db");
        }
        let config = DatabaseConfig::from_env().unwrap().unwrap();
        assert!(matches!(config, DatabaseConfig::CozoSqlite { .. }));
        // Note: Cleanup would be needed in production, but tests are isolated
    }

    #[test]
    fn test_from_env_cozo_path() {
        let _lock = test_utils::global_test_lock().lock();
        unsafe {
            std::env::remove_var("DATABASE_URL");
            std::env::set_var("COZO_PATH", "/tmp/test.sqlite");
        }
        let config = DatabaseConfig::from_env().unwrap().unwrap();
        match config {
            DatabaseConfig::CozoSqlite { path } => {
                assert_eq!(path, PathBuf::from("/tmp/test.sqlite"));
            }
            _ => panic!("Expected CozoSqlite"),
        }
    }

    #[test]
    fn test_from_env_database_url_takes_precedence() {
        let _lock = test_utils::global_test_lock().lock();
        unsafe {
            std::env::set_var("DATABASE_URL", "sqlite:///tmp/from_url.db");
            std::env::set_var("COZO_PATH", "/tmp/from_path.sqlite");
        }
        let config = DatabaseConfig::from_env().unwrap().unwrap();
        match config {
            DatabaseConfig::CozoSqlite { path } => {
                assert_eq!(path, PathBuf::from("/tmp/from_url.db"));
            }
            _ => panic!("Expected CozoSqlite"),
        }
    }

    #[test]
    fn test_resolve_env_fallback() {
        let _lock = test_utils::global_test_lock().lock();
        let temp_dir = tempfile::tempdir().unwrap();
        let old_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();
        unsafe {
            std::env::set_var("DATABASE_URL", "sqlite:///tmp/env.db");
        }
        let config = DatabaseConfig::resolve().unwrap();
        match config {
            DatabaseConfig::CozoSqlite { path } => {
                assert_eq!(path, PathBuf::from("/tmp/env.db"));
            }
            _ => panic!("Expected CozoSqlite"),
        }
        std::env::set_current_dir(old_dir).unwrap();
    }

    #[test]
    fn test_resolve_default_fallback() {
        let _lock = test_utils::global_test_lock().lock();
        let temp_dir = tempfile::tempdir().unwrap();
        let old_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();
        unsafe {
            std::env::remove_var("DATABASE_URL");
            std::env::remove_var("COZO_PATH");
        }
        let config = DatabaseConfig::resolve().unwrap();
        match config {
            DatabaseConfig::CozoSqlite { path } => {
                assert_eq!(path, PathBuf::from("./cozo.sqlite"));
            }
            _ => panic!("Expected CozoSqlite"),
        }
        std::env::set_current_dir(old_dir).unwrap();
    }

    // New resolve() tests - load from config file

    #[test]
    fn test_resolve_from_config_file_sqlite() {
        let _lock = test_utils::global_test_lock().lock();
        let temp_dir = tempfile::tempdir().unwrap();
        let config_path = temp_dir.path().join(".code_search.json");

        let json = r#"
        {
            "database": {
                "type": "sqlite",
                "path": "./test.sqlite"
            }
        }
        "#;

        fs::write(&config_path, json).unwrap();

        let old_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        let result = DatabaseConfig::resolve();
        std::env::set_current_dir(old_dir).unwrap();

        let config = result.unwrap();
        match config {
            DatabaseConfig::CozoSqlite { path } => {
                assert_eq!(path, PathBuf::from("./test.sqlite"));
            }
            _ => panic!("Expected CozoSqlite"),
        }
    }

    #[test]
    fn test_resolve_from_config_file_memory() {
        let _lock = test_utils::global_test_lock().lock();
        let temp_dir = tempfile::tempdir().unwrap();
        let config_path = temp_dir.path().join(".code_search.json");

        let json = r#"
        {
            "database": {
                "type": "memory"
            }
        }
        "#;

        fs::write(&config_path, json).unwrap();

        let old_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();

        let result = DatabaseConfig::resolve();
        std::env::set_current_dir(old_dir).unwrap();

        let config = result.unwrap();
        assert!(matches!(config, DatabaseConfig::CozoMem));
    }

    #[test]
    fn test_resolve_fallback_to_env_when_no_config_file() {
        let _lock = test_utils::global_test_lock().lock();
        let temp_dir = tempfile::tempdir().unwrap();

        let old_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();
        unsafe {
            std::env::set_var("DATABASE_URL", "sqlite:///tmp/from_env.db");
        }

        let config = DatabaseConfig::resolve().unwrap();
        match config {
            DatabaseConfig::CozoSqlite { path } => {
                assert_eq!(path, PathBuf::from("/tmp/from_env.db"));
            }
            _ => panic!("Expected CozoSqlite"),
        }

        std::env::set_current_dir(old_dir).unwrap();
    }

    #[test]
    fn test_resolve_config_file_takes_precedence_over_env() {
        let _lock = test_utils::global_test_lock().lock();
        let temp_dir = tempfile::tempdir().unwrap();
        let config_path = temp_dir.path().join(".code_search.json");

        let json = r#"
        {
            "database": {
                "type": "sqlite",
                "path": "./from_config.sqlite"
            }
        }
        "#;

        fs::write(&config_path, json).unwrap();

        let old_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();
        unsafe {
            std::env::set_var("DATABASE_URL", "sqlite:///tmp/from_env.db");
        }

        let result = DatabaseConfig::resolve();
        std::env::set_current_dir(old_dir).unwrap();

        let config = result.unwrap();
        match config {
            DatabaseConfig::CozoSqlite { path } => {
                // Config file should take precedence
                assert_eq!(path, PathBuf::from("./from_config.sqlite"));
            }
            _ => panic!("Expected CozoSqlite"),
        }
    }
}
