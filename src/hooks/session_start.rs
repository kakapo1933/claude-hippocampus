//! Session start hook handler.
//!
//! Creates a new session record and loads memory context.

use sqlx::postgres::PgPool;

use crate::commands::get_context;
use crate::db::queries::{create_session, find_session_by_id};
use crate::error::Result;
use crate::git::get_git_status;
use crate::session::{load_session_state, save_session_state, SessionState};

use super::{HookInput, HookOutput};

/// Handle the session-start hook.
///
/// 1. Check if session already exists (reconnection case)
/// 2. Create new session if needed
/// 3. Load memory context
/// 4. Return approval with context
pub async fn handle_session_start(pool: &PgPool, input: &HookInput) -> Result<HookOutput> {
    let claude_session_id = input
        .session_id
        .clone()
        .unwrap_or_else(|| format!("session_{}", chrono::Utc::now().timestamp_millis()));

    let project_path = input
        .cwd
        .clone()
        .or_else(|| std::env::var("PROJECT_PATH").ok())
        .or_else(|| std::env::current_dir().ok().map(|p| p.to_string_lossy().to_string()));

    // Check for existing session (reconnection case)
    let existing_state = load_session_state(Some(&claude_session_id))?;
    let mut session_id = None;

    if let Some(ref state) = existing_state {
        // Only reuse if same Claude session and session exists
        if state.claude_session_id.as_deref() == Some(&claude_session_id) {
            if let Some(ref id) = state.session_id {
                // Verify session is still active
                if let Some(session) = find_session_by_id(pool, *id).await? {
                    if session.status.as_str() == "active" {
                        session_id = Some(*id);
                    }
                }
            }
        }
    }

    // Create new session if needed
    if session_id.is_none() {
        let git_status = project_path.as_ref().and_then(|p| get_git_status(p).ok()).flatten();
        let session = create_session(pool, &claude_session_id, project_path.as_deref(), git_status.as_ref()).await?;
        session_id = Some(session.id);

        // Save session state for other hooks
        let new_state = SessionState {
            session_id,
            claude_session_id: Some(claude_session_id.clone()),
            turn_number: 0,
            current_turn_id: None,
        };
        save_session_state(&new_state)?;
    }

    // Load memory context
    let context_result = get_context(pool, 5, project_path.as_deref()).await?;

    // Build context message from entries
    let mut context_message = String::new();
    if !context_result.entries.is_empty() {
        context_message.push_str(&format!("\n<memory-context loaded=\"{}\">\n", context_result.count));
        for entry in &context_result.entries {
            let conf = match entry.confidence.as_str() {
                "high" => "★",
                "medium" => "◐",
                _ => "○",
            };
            let entry_type = entry.memory_type.as_str();
            let content = if entry.summary.len() > 80 {
                &entry.summary[..80]
            } else {
                &entry.summary
            };
            context_message.push_str(&format!("{} [{}] {}\n", conf, entry_type, content));
        }
        context_message.push_str("</memory-context>\n");
    }

    if context_message.is_empty() {
        Ok(HookOutput::approve())
    } else {
        Ok(HookOutput::approve_with_reason(context_message))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // Input parsing tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_hook_input_parsing() {
        let json = r#"{"session_id": "test-123", "cwd": "/tmp/test"}"#;
        let input: HookInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.session_id, Some("test-123".to_string()));
        assert_eq!(input.cwd, Some("/tmp/test".to_string()));
    }

    #[test]
    fn test_hook_input_parsing_session_start_full() {
        let json = r#"{
            "session_id": "abc-123-def-456",
            "cwd": "/Users/test/project",
            "permission_mode": "acceptEdits",
            "hook_event_name": "SessionStart"
        }"#;
        let input: HookInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.session_id, Some("abc-123-def-456".to_string()));
        assert_eq!(input.cwd, Some("/Users/test/project".to_string()));
        assert_eq!(input.hook_event_name, Some("SessionStart".to_string()));
    }

    #[test]
    fn test_hook_input_parsing_empty() {
        let json = "{}";
        let input: HookInput = serde_json::from_str(json).unwrap();
        assert!(input.session_id.is_none());
        assert!(input.cwd.is_none());
    }

    // -------------------------------------------------------------------------
    // Output format tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_hook_output_approve_format() {
        let output = HookOutput::approve();
        let json = serde_json::to_string(&output).unwrap();

        // Parse back and verify
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["decision"], "approve");
        assert!(parsed.get("reason").is_none());
    }

    #[test]
    fn test_hook_output_with_context_format() {
        let context = "<memory-context loaded=\"3\">\n★ [gotcha] Test\n</memory-context>".to_string();
        let output = HookOutput::approve_with_reason(context.clone());
        let json = serde_json::to_string(&output).unwrap();

        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["decision"], "approve");
        assert_eq!(parsed["reason"], context);
    }

    // -------------------------------------------------------------------------
    // Session state tests (unit tests without DB)
    // -------------------------------------------------------------------------

    #[test]
    fn test_session_state_creation() {
        let state = SessionState {
            session_id: Some(uuid::Uuid::new_v4()),
            claude_session_id: Some("test-session".to_string()),
            turn_number: 0,
            current_turn_id: None,
        };

        assert_eq!(state.turn_number, 0);
        assert!(state.session_id.is_some());
        assert_eq!(state.claude_session_id, Some("test-session".to_string()));
    }

    #[test]
    fn test_session_state_serialization_roundtrip() {
        let state = SessionState {
            session_id: Some(uuid::Uuid::new_v4()),
            claude_session_id: Some("roundtrip-test".to_string()),
            turn_number: 5,
            current_turn_id: Some(uuid::Uuid::new_v4()),
        };

        let json = serde_json::to_string(&state).unwrap();
        let parsed: SessionState = serde_json::from_str(&json).unwrap();

        assert_eq!(state.claude_session_id, parsed.claude_session_id);
        assert_eq!(state.turn_number, parsed.turn_number);
    }

    // Note: Full integration tests require a database connection
    // and are placed in tests/integration/
}
