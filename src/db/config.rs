//! Database configuration for runtime backend selection.
//!
//! This module provides configuration parsing and backend instantiation for the database abstraction layer.
//! Supports multiple database backends (currently only CozoDB/SQLite and in-memory for tests).

use std::error::Error;
use std::fs;
use std::path::PathBuf;

use super::backend::DatabaseBackend;
use super::connection::{CozoMemBackend, CozoSqliteBackend};
use cozo::DbInstance;

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

    /// PostgreSQL connection (future).
    Postgres {
        host: String,
        port: u16,
        database: String,
        username: String,
        password: Option<String>,
        ssl: bool,
    },

    /// Remote CozoDB server (future).
    RemoteCozo {
        host: String,
        port: u16,
        tls: bool,
        auth_token: Option<String>,
    },
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
            Self::Postgres { .. } => {
                return Err("PostgreSQL backend not yet implemented".into());
            }
            Self::RemoteCozo { .. } => {
                return Err("Remote Cozo backend not yet implemented".into());
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
    /// - `rocksdb:///path/to/db` → CozoRocksdb (future)
    /// - `postgres://user:pass@host:port/db` → Postgres (future)
    /// - `cozo://host:port` → RemoteCozo (future)
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
            return Err("PostgreSQL backend not yet implemented".into());
        }

        if url.starts_with("cozo://") || url.starts_with("cozo+tcp://") {
            return Err("Remote Cozo backend not yet implemented".into());
        }

        // Default: treat as file path (CozoSqlite)
        Ok(Self::CozoSqlite {
            path: PathBuf::from(url),
        })
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
    use std::sync::Mutex;
    use std::sync::OnceLock;

    // Mutex to serialize tests that modify global state (current directory, env vars)
    fn test_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

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
    fn test_from_url_postgres_not_implemented() {
        let result = DatabaseConfig::from_url("postgres://localhost/test");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("not yet implemented"));
    }

    #[test]
    fn test_from_url_postgres_alternate_scheme() {
        let result = DatabaseConfig::from_url("postgresql://localhost/test");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("not yet implemented"));
    }

    #[test]
    fn test_from_url_cozo_not_implemented() {
        let result = DatabaseConfig::from_url("cozo://localhost:9000");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("not yet implemented"));
    }

    #[test]
    fn test_from_url_cozo_tcp_not_implemented() {
        let result = DatabaseConfig::from_url("cozo+tcp://localhost:9000");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("not yet implemented"));
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
    fn test_connect_postgres_not_implemented() {
        let config = DatabaseConfig::Postgres {
            host: "localhost".to_string(),
            port: 5432,
            database: "test".to_string(),
            username: "user".to_string(),
            password: None,
            ssl: false,
        };
        let result = config.connect();
        assert!(result.is_err());
        if let Err(e) = result {
            let err_msg = format!("{}", e);
            assert!(err_msg.contains("not yet implemented"));
        }
    }

    #[test]
    fn test_connect_remote_cozo_not_implemented() {
        let config = DatabaseConfig::RemoteCozo {
            host: "localhost".to_string(),
            port: 9000,
            tls: false,
            auth_token: None,
        };
        let result = config.connect();
        assert!(result.is_err());
        if let Err(e) = result {
            let err_msg = format!("{}", e);
            assert!(err_msg.contains("not yet implemented"));
        }
    }

    #[test]
    fn test_from_env_none() {
        let _lock = test_lock().lock();
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
        let _lock = test_lock().lock();
        unsafe {
            std::env::set_var("DATABASE_URL", "sqlite:///tmp/test.db");
        }
        let config = DatabaseConfig::from_env().unwrap().unwrap();
        assert!(matches!(config, DatabaseConfig::CozoSqlite { .. }));
        // Note: Cleanup would be needed in production, but tests are isolated
    }

    #[test]
    fn test_from_env_cozo_path() {
        let _lock = test_lock().lock();
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
        let _lock = test_lock().lock();
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
        let _lock = test_lock().lock();
        let temp_dir = tempfile::tempdir().unwrap();
        let old_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();
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
        let _lock = test_lock().lock();
        let temp_dir = tempfile::tempdir().unwrap();
        let old_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();
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
        let _lock = test_lock().lock();
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
        std::env::set_current_dir(&temp_dir).unwrap();

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
        let _lock = test_lock().lock();
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
        std::env::set_current_dir(&temp_dir).unwrap();

        let result = DatabaseConfig::resolve();
        std::env::set_current_dir(old_dir).unwrap();

        let config = result.unwrap();
        assert!(matches!(config, DatabaseConfig::CozoMem));
    }

    #[test]
    fn test_resolve_fallback_to_env_when_no_config_file() {
        let temp_dir = tempfile::tempdir().unwrap();

        let old_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();
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
        let _lock = test_lock().lock();
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
        std::env::set_current_dir(&temp_dir).unwrap();
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
