use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

use crate::error::{HippocampusError, Result};

/// Database configuration loaded from ~/.claude/config/db.json
#[derive(Debug, Clone, Deserialize)]
pub struct DbConfig {
    pub host: String,
    pub port: u16,
    pub database: String,
    pub user: String,
    #[serde(default = "default_max_connections")]
    pub max_connections: u32,
    #[serde(default)]
    pub embedding_model: Option<String>,
    #[serde(default)]
    pub embedding_dimensions: Option<u32>,
}

fn default_max_connections() -> u32 {
    10
}

impl Default for DbConfig {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 5432,
            database: "claude_memory".to_string(),
            user: std::env::var("USER").unwrap_or_else(|_| "postgres".to_string()),
            max_connections: 10,
            embedding_model: None,
            embedding_dimensions: None,
        }
    }
}

impl DbConfig {
    /// Load config from the standard location (~/.claude/config/db.json)
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path();
        Self::load_from_path(&config_path)
    }

    /// Load config from a specific path
    pub fn load_from_path(path: &PathBuf) -> Result<Self> {
        if path.exists() {
            let content = fs::read_to_string(path).map_err(|e| {
                HippocampusError::Config(format!("Failed to read config file: {}", e))
            })?;
            let config: DbConfig = serde_json::from_str(&content).map_err(|e| {
                HippocampusError::Config(format!("Failed to parse config JSON: {}", e))
            })?;
            Ok(config)
        } else {
            Ok(Self::default())
        }
    }

    /// Get the standard config file path
    pub fn config_path() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".claude")
            .join("config")
            .join("db.json")
    }

    /// Build PostgreSQL connection string
    pub fn connection_string(&self) -> String {
        format!(
            "postgres://{}@{}:{}/{}",
            self.user, self.host, self.port, self.database
        )
    }

    /// Build connection string with password (if provided)
    pub fn connection_string_with_password(&self, password: Option<&str>) -> String {
        match password {
            Some(pwd) => format!(
                "postgres://{}:{}@{}:{}/{}",
                self.user, pwd, self.host, self.port, self.database
            ),
            None => self.connection_string(),
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_default_config() {
        let config = DbConfig::default();
        assert_eq!(config.host, "localhost");
        assert_eq!(config.port, 5432);
        assert_eq!(config.database, "claude_memory");
        assert_eq!(config.max_connections, 10);
    }

    #[test]
    fn test_load_from_json() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(
            temp_file,
            r#"{{
                "host": "db.example.com",
                "port": 5433,
                "database": "test_db",
                "user": "testuser",
                "max_connections": 20
            }}"#
        )
        .unwrap();

        let config = DbConfig::load_from_path(&temp_file.path().to_path_buf()).unwrap();
        assert_eq!(config.host, "db.example.com");
        assert_eq!(config.port, 5433);
        assert_eq!(config.database, "test_db");
        assert_eq!(config.user, "testuser");
        assert_eq!(config.max_connections, 20);
    }

    #[test]
    fn test_load_missing_file_returns_default() {
        let path = PathBuf::from("/nonexistent/path/db.json");
        let config = DbConfig::load_from_path(&path).unwrap();
        assert_eq!(config.host, "localhost");
        assert_eq!(config.database, "claude_memory");
    }

    #[test]
    fn test_load_invalid_json_returns_error() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "not valid json").unwrap();

        let result = DbConfig::load_from_path(&temp_file.path().to_path_buf());
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Failed to parse config JSON"));
    }

    #[test]
    fn test_connection_string() {
        let config = DbConfig {
            host: "localhost".to_string(),
            port: 5432,
            database: "claude_memory".to_string(),
            user: "testuser".to_string(),
            max_connections: 10,
            embedding_model: None,
            embedding_dimensions: None,
        };

        assert_eq!(
            config.connection_string(),
            "postgres://testuser@localhost:5432/claude_memory"
        );
    }

    #[test]
    fn test_connection_string_with_password() {
        let config = DbConfig {
            host: "localhost".to_string(),
            port: 5432,
            database: "claude_memory".to_string(),
            user: "testuser".to_string(),
            max_connections: 10,
            embedding_model: None,
            embedding_dimensions: None,
        };

        assert_eq!(
            config.connection_string_with_password(Some("secret")),
            "postgres://testuser:secret@localhost:5432/claude_memory"
        );
    }

    #[test]
    fn test_connection_string_with_no_password() {
        let config = DbConfig::default();
        let with_pwd = config.connection_string_with_password(None);
        let without_pwd = config.connection_string();
        assert_eq!(with_pwd, without_pwd);
    }

    #[test]
    fn test_config_path_contains_expected_components() {
        let path = DbConfig::config_path();
        let path_str = path.to_string_lossy();
        assert!(path_str.contains(".claude"));
        assert!(path_str.contains("config"));
        assert!(path_str.ends_with("db.json"));
    }

    #[test]
    fn test_optional_fields_default_to_none() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(
            temp_file,
            r#"{{
                "host": "localhost",
                "port": 5432,
                "database": "test",
                "user": "user"
            }}"#
        )
        .unwrap();

        let config = DbConfig::load_from_path(&temp_file.path().to_path_buf()).unwrap();
        assert!(config.embedding_model.is_none());
        assert!(config.embedding_dimensions.is_none());
    }

    #[test]
    fn test_embedding_fields_loaded() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(
            temp_file,
            r#"{{
                "host": "localhost",
                "port": 5432,
                "database": "test",
                "user": "user",
                "embedding_model": "mxbai-embed-large",
                "embedding_dimensions": 1024
            }}"#
        )
        .unwrap();

        let config = DbConfig::load_from_path(&temp_file.path().to_path_buf()).unwrap();
        assert_eq!(config.embedding_model, Some("mxbai-embed-large".to_string()));
        assert_eq!(config.embedding_dimensions, Some(1024));
    }
}
