//! Session model for tracking Claude Code sessions.
//!
//! Represents database sessions with status, git context, and timing.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use uuid::Uuid;

use crate::error::HippocampusError;
use crate::git::GitStatus;

// ============================================================================
// SessionStatus
// ============================================================================

/// Status of a session
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionStatus {
    Active,
    Completed,
    Orphaned,
}

impl SessionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Completed => "completed",
            Self::Orphaned => "orphaned",
        }
    }
}

impl FromStr for SessionStatus {
    type Err = HippocampusError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "active" => Ok(Self::Active),
            "completed" => Ok(Self::Completed),
            "orphaned" => Ok(Self::Orphaned),
            _ => Err(HippocampusError::InvalidSessionStatus(s.to_string())),
        }
    }
}

impl Default for SessionStatus {
    fn default() -> Self {
        Self::Active
    }
}

// ============================================================================
// Session
// ============================================================================

/// Represents a Claude Code session in the database
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    /// Database ID (UUID)
    pub id: Uuid,
    /// Claude's session identifier
    pub claude_session_id: String,
    /// Project path where session was started
    pub project_path: Option<String>,
    /// Git status at session start
    pub git_status: Option<GitStatus>,
    /// Models used during the session
    pub models_used: Option<Vec<String>>,
    /// Session status (active, completed, orphaned)
    pub status: SessionStatus,
    /// Session summary (set on completion)
    pub summary: Option<serde_json::Value>,
    /// When the session started
    pub started_at: DateTime<Utc>,
    /// When the session ended
    pub ended_at: Option<DateTime<Utc>>,
    /// Record creation timestamp
    pub created_at: DateTime<Utc>,
}

impl Session {
    /// Create a new session with default values
    pub fn new(claude_session_id: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            claude_session_id,
            project_path: None,
            git_status: None,
            models_used: None,
            status: SessionStatus::Active,
            summary: None,
            started_at: now,
            ended_at: None,
            created_at: now,
        }
    }

    /// Create with project path
    pub fn with_project_path(mut self, path: String) -> Self {
        self.project_path = Some(path);
        self
    }

    /// Create with git status
    pub fn with_git_status(mut self, status: GitStatus) -> Self {
        self.git_status = Some(status);
        self
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // SessionStatus tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_session_status_as_str() {
        assert_eq!(SessionStatus::Active.as_str(), "active");
        assert_eq!(SessionStatus::Completed.as_str(), "completed");
        assert_eq!(SessionStatus::Orphaned.as_str(), "orphaned");
    }

    #[test]
    fn test_session_status_from_str() {
        assert_eq!("active".parse::<SessionStatus>().unwrap(), SessionStatus::Active);
        assert_eq!("completed".parse::<SessionStatus>().unwrap(), SessionStatus::Completed);
        assert_eq!("orphaned".parse::<SessionStatus>().unwrap(), SessionStatus::Orphaned);
        assert_eq!("ACTIVE".parse::<SessionStatus>().unwrap(), SessionStatus::Active);
    }

    #[test]
    fn test_session_status_from_str_invalid() {
        assert!("invalid".parse::<SessionStatus>().is_err());
    }

    #[test]
    fn test_session_status_default() {
        assert_eq!(SessionStatus::default(), SessionStatus::Active);
    }

    // -------------------------------------------------------------------------
    // Session tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_session_new() {
        let session = Session::new("test-session-123".to_string());

        assert_eq!(session.claude_session_id, "test-session-123");
        assert!(session.project_path.is_none());
        assert!(session.git_status.is_none());
        assert!(session.models_used.is_none());
        assert_eq!(session.status, SessionStatus::Active);
        assert!(session.summary.is_none());
        assert!(session.ended_at.is_none());
    }

    #[test]
    fn test_session_with_project_path() {
        let session = Session::new("test-123".to_string())
            .with_project_path("/path/to/project".to_string());

        assert_eq!(session.project_path, Some("/path/to/project".to_string()));
    }

    #[test]
    fn test_session_with_git_status() {
        let git_status = GitStatus {
            branch: "main".to_string(),
            modified: vec!["file.rs".to_string()],
            untracked: vec![],
            staged: vec![],
        };

        let session = Session::new("test-123".to_string())
            .with_git_status(git_status.clone());

        assert_eq!(session.git_status, Some(git_status));
    }

    #[test]
    fn test_session_serialization() {
        let session = Session::new("test-session".to_string());
        let json = serde_json::to_string(&session).unwrap();

        // Verify camelCase field names (matching Node.js)
        assert!(json.contains("claudeSessionId"));
        assert!(json.contains("projectPath"));
        assert!(json.contains("gitStatus"));
        assert!(json.contains("modelsUsed"));
        assert!(json.contains("startedAt"));
        assert!(json.contains("endedAt"));
        assert!(json.contains("createdAt"));

        // Verify no snake_case
        assert!(!json.contains("claude_session_id"));
        assert!(!json.contains("project_path"));
        assert!(!json.contains("git_status"));
        assert!(!json.contains("models_used"));
        assert!(!json.contains("started_at"));
        assert!(!json.contains("ended_at"));
        assert!(!json.contains("created_at"));
    }

    #[test]
    fn test_session_deserialization() {
        let json = r#"{
            "id": "550e8400-e29b-41d4-a716-446655440000",
            "claudeSessionId": "test-session",
            "projectPath": "/path/to/project",
            "gitStatus": null,
            "modelsUsed": ["claude-3-opus"],
            "status": "active",
            "summary": null,
            "startedAt": "2025-01-24T10:00:00Z",
            "endedAt": null,
            "createdAt": "2025-01-24T10:00:00Z"
        }"#;

        let session: Session = serde_json::from_str(json).unwrap();

        assert_eq!(session.claude_session_id, "test-session");
        assert_eq!(session.project_path, Some("/path/to/project".to_string()));
        assert_eq!(session.models_used, Some(vec!["claude-3-opus".to_string()]));
        assert_eq!(session.status, SessionStatus::Active);
    }
}
