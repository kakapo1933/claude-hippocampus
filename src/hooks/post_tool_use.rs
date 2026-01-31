//! PostToolUse hook handler.
//!
//! Records tool calls to the database for session tracking.
//! Input: JSON with { tool_name, tool_input, tool_response, session_id }
//! Output: JSON with decision: approve (always approve, just logging)

use serde::Deserialize;
use sqlx::PgPool;

use crate::db::queries::{find_session_by_claude_id, record_tool_call};
use crate::error::Result;
use crate::session::load_session_state;

use super::debug::debug as debug_log;
use super::HookOutput;

const HOOK_NAME: &str = "post-tool-use";

/// Debug logging wrapper for this hook
fn debug(msg: &str) {
    debug_log(HOOK_NAME, msg);
}

/// Input format for PostToolUse hook (different from standard HookInput)
#[derive(Debug, Clone, Deserialize)]
pub struct PostToolUseInput {
    /// Tool name
    #[serde(alias = "toolName")]
    pub tool_name: Option<String>,
    /// Tool input parameters
    #[serde(alias = "toolInput")]
    pub tool_input: Option<serde_json::Value>,
    /// Tool response/result
    #[serde(alias = "toolResponse")]
    pub tool_response: Option<serde_json::Value>,
    /// Claude session ID
    #[serde(alias = "sessionId")]
    pub session_id: Option<String>,
}

/// Handle the PostToolUse hook
pub async fn handle_post_tool_use(pool: &PgPool, input: &PostToolUseInput) -> Result<HookOutput> {
    debug("=== Post tool use hook started ===");

    let tool_name = input.tool_name.as_deref().unwrap_or("unknown");
    debug(&format!("Tool: {}", tool_name));

    // Get session and turn IDs
    let (session_id, turn_id) = if let Some(claude_session_id) = &input.session_id {
        debug(&format!("Session ID: {}", claude_session_id));
        // Try session state file first
        if let Ok(Some(state)) = load_session_state(Some(claude_session_id)) {
            debug(&format!("Loaded session state: session={:?}, turn={:?}", state.session_id, state.current_turn_id));
            (state.session_id, state.current_turn_id)
        } else {
            // Fallback to database lookup
            debug("Session state not found, checking database");
            let session = find_session_by_claude_id(pool, claude_session_id).await?;
            (session.map(|s| s.id), None)
        }
    } else {
        debug("No session ID provided");
        (None, None)
    };

    // Truncate response to summary (first 500 chars)
    let result_summary = input.tool_response.as_ref().map(|r| {
        let s = r.to_string();
        if s.len() > 500 {
            format!("{}...", &s[..497])
        } else {
            s
        }
    });

    debug(&format!("Result summary length: {} chars", result_summary.as_ref().map(|s| s.len()).unwrap_or(0)));

    // Record the tool call (ignore errors - don't block on logging failure)
    debug("Recording tool call to database");
    let _ = record_tool_call(
        pool,
        session_id,
        turn_id,
        tool_name,
        input.tool_input.clone(),
        result_summary,
    )
    .await;

    debug("=== Post tool use hook completed ===");

    // Always approve
    Ok(HookOutput::approve())
}

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // PostToolUseInput deserialization tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_post_tool_use_input_snake_case() {
        let json = r#"{
            "tool_name": "Read",
            "tool_input": {"file_path": "/tmp/test.txt"},
            "tool_response": "file contents",
            "session_id": "abc-123"
        }"#;
        let input: PostToolUseInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.tool_name, Some("Read".to_string()));
        assert_eq!(input.session_id, Some("abc-123".to_string()));
        assert!(input.tool_input.is_some());
        assert!(input.tool_response.is_some());
    }

    #[test]
    fn test_post_tool_use_input_camel_case() {
        let json = r#"{
            "toolName": "Write",
            "toolInput": {},
            "toolResponse": null,
            "sessionId": "xyz-456"
        }"#;
        let input: PostToolUseInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.tool_name, Some("Write".to_string()));
        assert_eq!(input.session_id, Some("xyz-456".to_string()));
    }

    #[test]
    fn test_post_tool_use_input_empty() {
        let json = "{}";
        let input: PostToolUseInput = serde_json::from_str(json).unwrap();
        assert!(input.tool_name.is_none());
        assert!(input.tool_input.is_none());
        assert!(input.tool_response.is_none());
        assert!(input.session_id.is_none());
    }

    #[test]
    fn test_post_tool_use_input_partial() {
        let json = r#"{"tool_name": "Bash"}"#;
        let input: PostToolUseInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.tool_name, Some("Bash".to_string()));
        assert!(input.session_id.is_none());
    }

    // -------------------------------------------------------------------------
    // handle_post_tool_use tests (require database - moved to integration tests)
    // -------------------------------------------------------------------------

    // Note: Tests for handle_post_tool_use require a database connection
    // and are placed in tests/integration/ or tested via the CLI.

    #[test]
    fn test_post_tool_use_input_tool_response_json() {
        let json = r#"{
            "tool_name": "Read",
            "tool_response": {"status": "ok", "lines": 100}
        }"#;
        let input: PostToolUseInput = serde_json::from_str(json).unwrap();
        assert!(input.tool_response.is_some());
        let response = input.tool_response.unwrap();
        assert_eq!(response["status"], "ok");
    }

    #[test]
    fn test_post_tool_use_input_tool_response_string() {
        let json = r#"{
            "tool_name": "Bash",
            "tool_response": "command output here"
        }"#;
        let input: PostToolUseInput = serde_json::from_str(json).unwrap();
        assert!(input.tool_response.is_some());
    }
}
