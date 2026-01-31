//! Session end hook handler.
//!
//! Ends the session and cleans up state files.

use std::fs;

use sqlx::postgres::PgPool;

use crate::db::queries::end_session;
use crate::error::Result;
use crate::session::{clear_session_state, load_session_state};

use super::debug::debug as debug_log;
use super::{HookInput, HookOutput};

const HOOK_NAME: &str = "session-end";

/// Debug logging wrapper for this hook
fn debug(msg: &str) {
    debug_log(HOOK_NAME, msg);
}

/// Get marker file path for a session
fn get_marker_file(claude_session_id: &str) -> String {
    format!("/tmp/hippocampus-brain-cells-extract-{}", claude_session_id)
}

/// Handle the session-end hook.
///
/// 1. Load session state
/// 2. End session in database
/// 3. Clean up session state file
/// 4. Return approval
pub async fn handle_session_end(pool: &PgPool, input: &HookInput) -> Result<HookOutput> {
    debug("=== Session end hook started ===");

    let claude_session_id = input.session_id.clone().unwrap_or_default();

    if claude_session_id.is_empty() {
        debug("No session ID provided, skipping");
        return Ok(HookOutput::approve());
    }

    debug(&format!("Session ID: {}", claude_session_id));

    // Load session state
    let _state = load_session_state(Some(&claude_session_id))?;
    debug("Session state loaded");

    // End session in database
    debug("Ending session in database");
    match end_session(pool, &claude_session_id, None).await {
        Ok(_) => {
            debug("Session ended successfully in database");
        }
        Err(crate::error::HippocampusError::SessionNotFound(_)) => {
            // Session not found is OK - may have been cleaned up already
            debug("Session not found in database (already cleaned up)");
        }
        Err(e) => return Err(e),
    }

    // Clean up session state file
    debug("Clearing session state file");
    clear_session_state(Some(&claude_session_id))?;

    // Clean up marker file for this session only (safe for concurrent sessions)
    let marker_file = get_marker_file(&claude_session_id);
    debug(&format!("Removing marker file: {}", marker_file));
    let _ = fs::remove_file(&marker_file);

    debug("=== Session end hook completed ===");
    Ok(HookOutput::approve())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::{save_session_state, get_session_state_path, SessionState};
    use std::fs;

    fn cleanup_session_file(claude_session_id: &str) {
        let path = get_session_state_path(Some(claude_session_id));
        let _ = fs::remove_file(&path);
    }

    // -------------------------------------------------------------------------
    // Input handling tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_empty_session_id() {
        let input = HookInput {
            session_id: None,
            prompt: None,
            transcript_path: None,
            cwd: None,
            permission_mode: None,
            hook_event_name: None,
        };

        assert!(input.session_id.is_none());
    }

    #[test]
    fn test_input_with_session_id() {
        let input = HookInput {
            session_id: Some("end-session-test".to_string()),
            prompt: None,
            transcript_path: None,
            cwd: None,
            permission_mode: None,
            hook_event_name: Some("SessionEnd".to_string()),
        };

        assert_eq!(input.session_id, Some("end-session-test".to_string()));
        assert_eq!(input.hook_event_name, Some("SessionEnd".to_string()));
    }

    // -------------------------------------------------------------------------
    // Session state cleanup tests (without DB)
    // -------------------------------------------------------------------------

    #[test]
    fn test_clear_session_state_file_exists() {
        let session_id = format!("test-end-{}", uuid::Uuid::new_v4());

        // Create a session state file
        let state = SessionState {
            session_id: Some(uuid::Uuid::new_v4()),
            claude_session_id: Some(session_id.clone()),
            turn_number: 5,
            current_turn_id: None,
        };
        save_session_state(&state).unwrap();

        // Verify file exists
        let path = get_session_state_path(Some(&session_id));
        assert!(path.exists());

        // Clear it
        clear_session_state(Some(&session_id)).unwrap();

        // Verify file is removed
        assert!(!path.exists());
    }

    #[test]
    fn test_clear_session_state_file_not_exists() {
        let session_id = format!("nonexistent-{}", uuid::Uuid::new_v4());

        // Should not error even if file doesn't exist
        let result = clear_session_state(Some(&session_id));
        assert!(result.is_ok());
    }

    #[test]
    fn test_clear_session_state_empty_id() {
        // Empty ID should use legacy path, but shouldn't panic
        let result = clear_session_state(Some(""));
        // This will try to clear legacy file, may or may not exist
        assert!(result.is_ok());
    }

    // -------------------------------------------------------------------------
    // Output format tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_session_end_output_format() {
        let output = HookOutput::approve();
        let json = serde_json::to_string(&output).unwrap();

        // Session end should return simple approve
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["decision"], "approve");
    }

    // Note: Full integration tests require a database connection
    // and are placed in tests/integration/
}
