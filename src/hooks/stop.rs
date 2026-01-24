//! Stop hook handler.
//!
//! Runs after each Claude response. Manages marker files to prevent duplicate processing.

use std::fs;
use std::path::Path;

use crate::error::Result;

use super::{HookInput, HookOutput};

/// Marker file path for stop hook coordination
fn get_marker_file(claude_session_id: &str) -> String {
    format!("/tmp/claude-memory-extract-{}", claude_session_id)
}

/// Handle the stop hook.
///
/// 1. Skip if extraction instance (prevent recursion)
/// 2. Check marker file - skip if already processed this turn
/// 3. Set marker file to prevent duplicate processing
/// 4. Return approval
///
/// Note: Memory extraction (spawning headless Claude) is handled separately.
pub async fn handle_stop(input: &HookInput) -> Result<HookOutput> {
    // Skip if this is an extraction instance (prevent recursion)
    if std::env::var("CLAUDE_MEMORY_EXTRACTION").is_ok() {
        return Ok(HookOutput::approve());
    }

    let claude_session_id = input.session_id.clone().unwrap_or_else(|| "unknown".to_string());

    // Check marker file - skip if already processed
    let marker_file = get_marker_file(&claude_session_id);
    if Path::new(&marker_file).exists() {
        // Already processed this turn
        return Ok(HookOutput::approve());
    }

    // Set marker to prevent duplicate processing
    let _ = fs::write(&marker_file, "1");

    Ok(HookOutput::approve())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn cleanup_marker(session_id: &str) {
        let path = get_marker_file(session_id);
        let _ = fs::remove_file(&path);
    }

    // -------------------------------------------------------------------------
    // Marker file path tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_get_marker_file() {
        let path = get_marker_file("test-session");
        assert_eq!(path, "/tmp/claude-memory-extract-test-session");
    }

    #[test]
    fn test_get_marker_file_prefix() {
        let path = get_marker_file("any-id");
        assert!(path.starts_with("/tmp/claude-memory-extract-"));
    }

    #[test]
    fn test_get_marker_file_unique_per_session() {
        let path1 = get_marker_file("session-1");
        let path2 = get_marker_file("session-2");
        assert_ne!(path1, path2);
    }

    // -------------------------------------------------------------------------
    // handle_stop tests
    // -------------------------------------------------------------------------

    #[tokio::test]
    async fn test_handle_stop_creates_marker() {
        let session_id = format!("test-stop-{}", uuid::Uuid::new_v4());
        cleanup_marker(&session_id);

        let input = HookInput {
            session_id: Some(session_id.clone()),
            prompt: None,
            transcript_path: None,
            cwd: None,
            permission_mode: None,
            hook_event_name: None,
        };

        let result = handle_stop(&input).await.unwrap();
        assert_eq!(result.decision, "approve");

        // Verify marker was created
        let marker_file = get_marker_file(&session_id);
        assert!(Path::new(&marker_file).exists());

        cleanup_marker(&session_id);
    }

    #[tokio::test]
    async fn test_handle_stop_skips_if_marker_exists() {
        let session_id = format!("test-stop-skip-{}", uuid::Uuid::new_v4());
        let marker_file = get_marker_file(&session_id);

        // Create marker first
        fs::write(&marker_file, "1").unwrap();

        let input = HookInput {
            session_id: Some(session_id.clone()),
            prompt: None,
            transcript_path: None,
            cwd: None,
            permission_mode: None,
            hook_event_name: None,
        };

        let result = handle_stop(&input).await.unwrap();
        assert_eq!(result.decision, "approve");

        cleanup_marker(&session_id);
    }

    #[tokio::test]
    async fn test_handle_stop_no_session_id() {
        let input = HookInput {
            session_id: None,
            prompt: None,
            transcript_path: None,
            cwd: None,
            permission_mode: None,
            hook_event_name: None,
        };

        let result = handle_stop(&input).await.unwrap();
        assert_eq!(result.decision, "approve");

        // Cleanup marker for "unknown"
        cleanup_marker("unknown");
    }

    #[tokio::test]
    async fn test_handle_stop_always_approves() {
        let session_id = format!("test-stop-always-{}", uuid::Uuid::new_v4());
        cleanup_marker(&session_id);

        let input = HookInput {
            session_id: Some(session_id.clone()),
            prompt: Some("some prompt".to_string()),
            transcript_path: Some("/tmp/test.jsonl".to_string()),
            cwd: Some("/tmp".to_string()),
            permission_mode: Some("acceptEdits".to_string()),
            hook_event_name: Some("Stop".to_string()),
        };

        let result = handle_stop(&input).await.unwrap();
        assert_eq!(result.decision, "approve");
        // Stop hook should never block
        assert!(result.reason.is_none());

        cleanup_marker(&session_id);
    }

    #[tokio::test]
    async fn test_handle_stop_marker_content() {
        let session_id = format!("test-stop-content-{}", uuid::Uuid::new_v4());
        cleanup_marker(&session_id);

        let input = HookInput {
            session_id: Some(session_id.clone()),
            prompt: None,
            transcript_path: None,
            cwd: None,
            permission_mode: None,
            hook_event_name: None,
        };

        handle_stop(&input).await.unwrap();

        // Verify marker content
        let marker_file = get_marker_file(&session_id);
        let content = fs::read_to_string(&marker_file).unwrap();
        assert_eq!(content, "1");

        cleanup_marker(&session_id);
    }

    #[tokio::test]
    async fn test_handle_stop_idempotent() {
        let session_id = format!("test-stop-idempotent-{}", uuid::Uuid::new_v4());
        cleanup_marker(&session_id);

        let input = HookInput {
            session_id: Some(session_id.clone()),
            prompt: None,
            transcript_path: None,
            cwd: None,
            permission_mode: None,
            hook_event_name: None,
        };

        // Call twice
        let result1 = handle_stop(&input).await.unwrap();
        let result2 = handle_stop(&input).await.unwrap();

        // Both should approve
        assert_eq!(result1.decision, "approve");
        assert_eq!(result2.decision, "approve");

        cleanup_marker(&session_id);
    }
}
