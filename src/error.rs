use thiserror::Error;

#[derive(Error, Debug)]
pub enum HippocampusError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Invalid memory type: {0}. Must be one of: convention, architecture, gotcha, api, learning, preference")]
    InvalidMemoryType(String),

    #[error("Invalid confidence level: {0}. Must be one of: high, medium, low")]
    InvalidConfidence(String),

    #[error("Invalid tier: {0}. Must be one of: project, global, both")]
    InvalidTier(String),

    #[error("Invalid scope: {0}. Must be one of: project, global")]
    InvalidScope(String),

    #[error("Memory not found: {0}")]
    NotFound(String),

    #[error("Session state error: {0}")]
    SessionState(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("UUID parse error: {0}")]
    UuidParse(#[from] uuid::Error),
}

pub type Result<T> = std::result::Result<T, HippocampusError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_memory_type_error_display() {
        let err = HippocampusError::InvalidMemoryType("foo".to_string());
        assert!(err.to_string().contains("Invalid memory type: foo"));
        assert!(err.to_string().contains("convention"));
    }

    #[test]
    fn test_invalid_confidence_error_display() {
        let err = HippocampusError::InvalidConfidence("bar".to_string());
        assert!(err.to_string().contains("Invalid confidence level: bar"));
        assert!(err.to_string().contains("high"));
    }

    #[test]
    fn test_invalid_tier_error_display() {
        let err = HippocampusError::InvalidTier("baz".to_string());
        assert!(err.to_string().contains("Invalid tier: baz"));
        assert!(err.to_string().contains("both"));
    }

    #[test]
    fn test_config_error_display() {
        let err = HippocampusError::Config("missing file".to_string());
        assert_eq!(err.to_string(), "Configuration error: missing file");
    }

    #[test]
    fn test_not_found_error_display() {
        let err = HippocampusError::NotFound("abc-123".to_string());
        assert_eq!(err.to_string(), "Memory not found: abc-123");
    }
}
