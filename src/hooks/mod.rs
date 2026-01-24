//! Hook handlers for Claude Code settings.json integration.
//!
//! Each hook reads JSON from stdin and outputs JSON with decision/reason fields.

pub mod session_start;
pub mod user_prompt_submit;
pub mod stop;
pub mod session_end;

pub use session_start::handle_session_start;
pub use user_prompt_submit::handle_user_prompt_submit;
pub use stop::handle_stop;
pub use session_end::handle_session_end;

use serde::{Deserialize, Serialize};

/// Standard input format for hooks (from Claude Code)
#[derive(Debug, Clone, Deserialize)]
pub struct HookInput {
    /// Claude's session identifier
    #[serde(alias = "sessionId")]
    pub session_id: Option<String>,
    /// User's prompt (for UserPromptSubmit)
    pub prompt: Option<String>,
    /// Path to transcript file
    pub transcript_path: Option<String>,
    /// Current working directory
    pub cwd: Option<String>,
    /// Permission mode
    pub permission_mode: Option<String>,
    /// Hook event name
    pub hook_event_name: Option<String>,
}

/// Standard output format for hooks
#[derive(Debug, Clone, Serialize)]
pub struct HookOutput {
    /// Decision: "approve" or "block"
    pub decision: String,
    /// Optional reason/context message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

impl HookOutput {
    /// Create an approve response
    pub fn approve() -> Self {
        Self {
            decision: "approve".to_string(),
            reason: None,
        }
    }

    /// Create an approve response with reason
    pub fn approve_with_reason(reason: String) -> Self {
        Self {
            decision: "approve".to_string(),
            reason: Some(reason),
        }
    }

    /// Create a block response
    pub fn block(reason: String) -> Self {
        Self {
            decision: "block".to_string(),
            reason: Some(reason),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hook_output_approve() {
        let output = HookOutput::approve();
        assert_eq!(output.decision, "approve");
        assert!(output.reason.is_none());
    }

    #[test]
    fn test_hook_output_approve_with_reason() {
        let output = HookOutput::approve_with_reason("context loaded".to_string());
        assert_eq!(output.decision, "approve");
        assert_eq!(output.reason, Some("context loaded".to_string()));
    }

    #[test]
    fn test_hook_output_block() {
        let output = HookOutput::block("error occurred".to_string());
        assert_eq!(output.decision, "block");
        assert_eq!(output.reason, Some("error occurred".to_string()));
    }

    #[test]
    fn test_hook_output_serialization() {
        let output = HookOutput::approve_with_reason("test".to_string());
        let json = serde_json::to_string(&output).unwrap();
        assert!(json.contains("\"decision\":\"approve\""));
        assert!(json.contains("\"reason\":\"test\""));
    }

    #[test]
    fn test_hook_output_serialization_no_reason() {
        let output = HookOutput::approve();
        let json = serde_json::to_string(&output).unwrap();
        assert!(json.contains("\"decision\":\"approve\""));
        assert!(!json.contains("reason")); // skip_serializing_if
    }

    #[test]
    fn test_hook_input_deserialization() {
        let json = r#"{"session_id": "abc-123", "prompt": "hello"}"#;
        let input: HookInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.session_id, Some("abc-123".to_string()));
        assert_eq!(input.prompt, Some("hello".to_string()));
    }

    #[test]
    fn test_hook_input_deserialization_camel_case() {
        let json = r#"{"sessionId": "abc-123"}"#;
        let input: HookInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.session_id, Some("abc-123".to_string()));
    }

    // -------------------------------------------------------------------------
    // Additional HookInput tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_hook_input_all_fields() {
        let json = r#"{
            "session_id": "sess-123",
            "prompt": "test prompt",
            "transcript_path": "/tmp/transcript.jsonl",
            "cwd": "/home/user/project",
            "permission_mode": "acceptEdits",
            "hook_event_name": "UserPromptSubmit"
        }"#;
        let input: HookInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.session_id, Some("sess-123".to_string()));
        assert_eq!(input.prompt, Some("test prompt".to_string()));
        assert_eq!(input.transcript_path, Some("/tmp/transcript.jsonl".to_string()));
        assert_eq!(input.cwd, Some("/home/user/project".to_string()));
        assert_eq!(input.permission_mode, Some("acceptEdits".to_string()));
        assert_eq!(input.hook_event_name, Some("UserPromptSubmit".to_string()));
    }

    #[test]
    fn test_hook_input_empty_json() {
        let json = "{}";
        let input: HookInput = serde_json::from_str(json).unwrap();
        assert!(input.session_id.is_none());
        assert!(input.prompt.is_none());
        assert!(input.transcript_path.is_none());
        assert!(input.cwd.is_none());
    }

    #[test]
    fn test_hook_input_partial_fields() {
        let json = r#"{"prompt": "hello world"}"#;
        let input: HookInput = serde_json::from_str(json).unwrap();
        assert!(input.session_id.is_none());
        assert_eq!(input.prompt, Some("hello world".to_string()));
    }

    #[test]
    fn test_hook_input_unicode_prompt() {
        let json = r#"{"prompt": "你好世界 🌍"}"#;
        let input: HookInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.prompt, Some("你好世界 🌍".to_string()));
    }

    #[test]
    fn test_hook_input_multiline_prompt() {
        let json = r#"{"prompt": "line1\nline2\nline3"}"#;
        let input: HookInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.prompt, Some("line1\nline2\nline3".to_string()));
    }

    #[test]
    fn test_hook_input_special_chars_in_prompt() {
        let json = r#"{"prompt": "test \"quoted\" and 'single' and \\backslash"}"#;
        let input: HookInput = serde_json::from_str(json).unwrap();
        assert!(input.prompt.is_some());
        assert!(input.prompt.unwrap().contains("quoted"));
    }

    // -------------------------------------------------------------------------
    // HookOutput edge case tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_hook_output_empty_reason() {
        let output = HookOutput::approve_with_reason(String::new());
        assert_eq!(output.decision, "approve");
        assert_eq!(output.reason, Some(String::new()));
    }

    #[test]
    fn test_hook_output_long_reason() {
        let long_reason = "x".repeat(10000);
        let output = HookOutput::approve_with_reason(long_reason.clone());
        assert_eq!(output.reason, Some(long_reason));
    }

    #[test]
    fn test_hook_output_unicode_reason() {
        let output = HookOutput::approve_with_reason("Memory: 記憶 🧠".to_string());
        let json = serde_json::to_string(&output).unwrap();
        assert!(json.contains("記憶"));
    }

    #[test]
    fn test_hook_output_block_serialization() {
        let output = HookOutput::block("validation failed".to_string());
        let json = serde_json::to_string(&output).unwrap();
        assert!(json.contains("\"decision\":\"block\""));
        assert!(json.contains("\"reason\":\"validation failed\""));
    }
}
