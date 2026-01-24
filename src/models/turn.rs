//! Turn model for conversation turns in the memory system.
//!
//! Represents individual prompt/response pairs within a session.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A conversation turn (prompt/response pair).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Turn {
    /// Unique identifier
    pub id: Uuid,
    /// Parent session ID
    pub session_id: Option<Uuid>,
    /// Turn number within the session (1-indexed)
    pub turn_number: i32,
    /// User's prompt text
    pub user_prompt: String,
    /// Assistant's response (None until turn completes)
    pub assistant_response: Option<String>,
    /// Model used for this turn
    pub model_used: Option<String>,
    /// Input tokens consumed
    pub input_tokens: Option<i32>,
    /// Output tokens generated
    pub output_tokens: Option<i32>,
    /// When the turn started
    pub started_at: DateTime<Utc>,
    /// When the turn ended (None if still in progress)
    pub ended_at: Option<DateTime<Utc>>,
    /// Record creation timestamp
    pub created_at: DateTime<Utc>,
}

/// Data for creating a new turn.
#[derive(Debug, Clone)]
pub struct CreateTurn {
    pub session_id: Uuid,
    pub turn_number: i32,
    pub user_prompt: String,
    pub model_used: Option<String>,
}

/// Data for updating a turn with response.
#[derive(Debug, Clone)]
pub struct UpdateTurn {
    pub assistant_response: String,
    pub input_tokens: Option<i32>,
    pub output_tokens: Option<i32>,
}

/// Summary of a turn for list responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TurnSummary {
    pub id: Uuid,
    pub turn_number: i32,
    pub user_prompt_preview: String,
    pub has_response: bool,
    pub started_at: DateTime<Utc>,
}

impl Turn {
    /// Create a summary from a full turn.
    pub fn to_summary(&self) -> TurnSummary {
        let preview = if self.user_prompt.len() > 100 {
            format!("{}...", &self.user_prompt[..97])
        } else {
            self.user_prompt.clone()
        };

        TurnSummary {
            id: self.id,
            turn_number: self.turn_number,
            user_prompt_preview: preview,
            has_response: self.assistant_response.is_some(),
            started_at: self.started_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_turn_struct_creation() {
        let id = Uuid::new_v4();
        let session_id = Uuid::new_v4();
        let now = Utc::now();

        let turn = Turn {
            id,
            session_id: Some(session_id),
            turn_number: 1,
            user_prompt: "Hello, world!".to_string(),
            assistant_response: None,
            model_used: Some("claude-3-opus".to_string()),
            input_tokens: None,
            output_tokens: None,
            started_at: now,
            ended_at: None,
            created_at: now,
        };

        assert_eq!(turn.id, id);
        assert_eq!(turn.session_id, Some(session_id));
        assert_eq!(turn.turn_number, 1);
        assert_eq!(turn.user_prompt, "Hello, world!");
        assert!(turn.assistant_response.is_none());
    }

    #[test]
    fn test_turn_serialization_camel_case() {
        let turn = Turn {
            id: Uuid::nil(),
            session_id: Some(Uuid::nil()),
            turn_number: 5,
            user_prompt: "test".to_string(),
            assistant_response: Some("response".to_string()),
            model_used: None,
            input_tokens: Some(100),
            output_tokens: Some(200),
            started_at: Utc::now(),
            ended_at: Some(Utc::now()),
            created_at: Utc::now(),
        };

        let json = serde_json::to_string(&turn).unwrap();

        // Verify camelCase field names (matching Node.js)
        assert!(json.contains("sessionId"));
        assert!(json.contains("turnNumber"));
        assert!(json.contains("userPrompt"));
        assert!(json.contains("assistantResponse"));
        assert!(json.contains("modelUsed"));
        assert!(json.contains("inputTokens"));
        assert!(json.contains("outputTokens"));
        assert!(json.contains("startedAt"));
        assert!(json.contains("endedAt"));
        assert!(json.contains("createdAt"));

        // Verify no snake_case
        assert!(!json.contains("session_id"));
        assert!(!json.contains("turn_number"));
        assert!(!json.contains("user_prompt"));
    }

    #[test]
    fn test_turn_deserialization() {
        let id = Uuid::new_v4();
        let session_id = Uuid::new_v4();

        let json = format!(
            r#"{{
                "id": "{}",
                "sessionId": "{}",
                "turnNumber": 3,
                "userPrompt": "test prompt",
                "assistantResponse": null,
                "modelUsed": "claude-3-sonnet",
                "inputTokens": 50,
                "outputTokens": null,
                "startedAt": "2024-01-15T10:00:00Z",
                "endedAt": null,
                "createdAt": "2024-01-15T10:00:00Z"
            }}"#,
            id, session_id
        );

        let turn: Turn = serde_json::from_str(&json).unwrap();

        assert_eq!(turn.id, id);
        assert_eq!(turn.session_id, Some(session_id));
        assert_eq!(turn.turn_number, 3);
        assert_eq!(turn.user_prompt, "test prompt");
        assert!(turn.assistant_response.is_none());
        assert_eq!(turn.model_used, Some("claude-3-sonnet".to_string()));
        assert_eq!(turn.input_tokens, Some(50));
    }

    #[test]
    fn test_turn_to_summary() {
        let turn = Turn {
            id: Uuid::new_v4(),
            session_id: Some(Uuid::new_v4()),
            turn_number: 1,
            user_prompt: "Short prompt".to_string(),
            assistant_response: Some("Response".to_string()),
            model_used: None,
            input_tokens: None,
            output_tokens: None,
            started_at: Utc::now(),
            ended_at: Some(Utc::now()),
            created_at: Utc::now(),
        };

        let summary = turn.to_summary();

        assert_eq!(summary.turn_number, 1);
        assert_eq!(summary.user_prompt_preview, "Short prompt");
        assert!(summary.has_response);
    }

    #[test]
    fn test_turn_to_summary_long_prompt() {
        let long_prompt = "a".repeat(150);
        let turn = Turn {
            id: Uuid::new_v4(),
            session_id: Some(Uuid::new_v4()),
            turn_number: 1,
            user_prompt: long_prompt,
            assistant_response: None,
            model_used: None,
            input_tokens: None,
            output_tokens: None,
            started_at: Utc::now(),
            ended_at: None,
            created_at: Utc::now(),
        };

        let summary = turn.to_summary();

        assert_eq!(summary.user_prompt_preview.len(), 100); // 97 chars + "..."
        assert!(summary.user_prompt_preview.ends_with("..."));
        assert!(!summary.has_response);
    }

    #[test]
    fn test_create_turn_struct() {
        let create = CreateTurn {
            session_id: Uuid::new_v4(),
            turn_number: 1,
            user_prompt: "Hello".to_string(),
            model_used: Some("claude".to_string()),
        };

        assert_eq!(create.turn_number, 1);
        assert_eq!(create.user_prompt, "Hello");
    }

    #[test]
    fn test_update_turn_struct() {
        let update = UpdateTurn {
            assistant_response: "Hi there!".to_string(),
            input_tokens: Some(10),
            output_tokens: Some(20),
        };

        assert_eq!(update.assistant_response, "Hi there!");
        assert_eq!(update.input_tokens, Some(10));
        assert_eq!(update.output_tokens, Some(20));
    }
}
