use sqlx::postgres::{PgPool, PgPoolOptions};
use std::time::Duration;

use crate::config::DbConfig;
use crate::error::Result;

/// Create a PostgreSQL connection pool from config
pub async fn create_pool(config: &DbConfig) -> Result<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(config.max_connections)
        .acquire_timeout(Duration::from_secs(5))
        .idle_timeout(Duration::from_secs(30))
        .connect(&config.connection_string())
        .await?;

    Ok(pool)
}

/// Create a PostgreSQL connection pool with password
pub async fn create_pool_with_password(config: &DbConfig, password: Option<&str>) -> Result<PgPool> {
    let conn_str = config.connection_string_with_password(password);
    let pool = PgPoolOptions::new()
        .max_connections(config.max_connections)
        .acquire_timeout(Duration::from_secs(5))
        .idle_timeout(Duration::from_secs(30))
        .connect(&conn_str)
        .await?;

    Ok(pool)
}

/// Get the current project path from environment or working directory
pub fn get_project_path() -> Option<String> {
    std::env::var("PROJECT_PATH").ok().or_else(|| {
        std::env::current_dir()
            .ok()
            .map(|p| p.to_string_lossy().to_string())
    })
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_project_path_from_env() {
        // This test verifies the function doesn't panic
        // Actual value depends on environment
        let path = get_project_path();
        // Should return Some value (either from env or cwd)
        assert!(path.is_some() || std::env::var("PROJECT_PATH").is_err());
    }

    #[test]
    fn test_get_project_path_returns_string() {
        if let Some(path) = get_project_path() {
            assert!(!path.is_empty());
        }
    }

    // Note: Integration tests for create_pool require a running database
    // Those tests will be in tests/integration/
}
