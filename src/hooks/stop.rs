//! Stop hook handler.
//!
//! Runs after each Claude response. Manages marker files to prevent duplicate processing.
//! Spawns headless Claude to extract conclusions and save them to memory.

use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};

use crate::error::Result;
use crate::session::load_session_state;

use super::debug::debug as debug_log;
use super::{HookInput, HookOutput};

const HOOK_NAME: &str = "stop";

/// Debug logging wrapper for this hook
fn debug(msg: &str) {
    debug_log(HOOK_NAME, msg);
}

/// Safely truncate a string at char boundaries (not byte boundaries)
fn truncate_str(s: &str, max_chars: usize) -> String {
    s.chars().take(max_chars).collect()
}

/// Marker file path for stop hook coordination
fn get_marker_file(claude_session_id: &str) -> String {
    format!("/tmp/hippocampus-brain-cells-extract-{}", claude_session_id)
}

/// Update the conversation turn with the assistant response
fn update_turn_with_response(turn_id: &str, assistant_response: &str) {
    if turn_id.is_empty() {
        debug("Skipping turn update - no turn_id");
        return;
    }

    debug(&format!(
        "Updating turn {} with response (len: {})",
        turn_id,
        assistant_response.len()
    ));

    // Run update-turn command synchronously (it's fast)
    match Command::new("claude-hippocampus")
        .arg("update-turn")
        .arg("--turn-id")
        .arg(turn_id)
        .arg("--response")
        .arg(assistant_response)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
    {
        Ok(output) => {
            if output.status.success() {
                debug("Turn updated successfully");
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                debug(&format!("Turn update failed: {}", stderr));
            }
        }
        Err(e) => {
            debug(&format!("Failed to run update-turn: {}", e));
        }
    }
}

/// Handle the stop hook.
///
/// 1. Skip if extraction instance (prevent recursion)
/// 2. Check marker file - skip if already processed this turn
/// 3. Read transcript and extract last user/assistant messages
/// 4. If substantive, spawn background extraction process
/// 5. Set marker file to prevent duplicate processing
/// 6. Return approval
pub async fn handle_stop(input: &HookInput) -> Result<HookOutput> {
    debug("=== Stop hook started ===");

    // Skip if this is an extraction instance (prevent recursion)
    if std::env::var("CLAUDE_MEMORY_EXTRACTION").is_ok() {
        debug("Skipping - extraction instance");
        return Ok(HookOutput::approve());
    }

    let claude_session_id = input.session_id.clone().unwrap_or_else(|| "unknown".to_string());
    debug(&format!("Session: {}", claude_session_id));

    // Load session state to get database IDs
    let state = load_session_state(Some(&claude_session_id))
        .ok()
        .flatten();
    let db_session_id = state.as_ref().and_then(|s| s.session_id);
    let turn_id = state.as_ref().and_then(|s| s.current_turn_id);
    debug(&format!(
        "Session state loaded: db_session_id={:?}, turn_id={:?}",
        db_session_id, turn_id
    ));

    // Check marker file - skip if already processed
    let marker_file = get_marker_file(&claude_session_id);
    if Path::new(&marker_file).exists() {
        debug(&format!("Skipping - marker file exists: {}", marker_file));
        return Ok(HookOutput::approve());
    }

    // Set marker to prevent duplicate processing
    let _ = fs::write(&marker_file, "1");

    // Read transcript file if available
    let transcript = input
        .transcript_path
        .as_ref()
        .and_then(|path| fs::read_to_string(path).ok())
        .unwrap_or_default();

    // Extract last user and assistant messages
    let (user_msg, assistant_msg) = extract_last_messages(&transcript);
    debug(&format!(
        "Extracted - user: {:?}, assistant: {:?}",
        user_msg.as_ref().map(|s| truncate_str(s, 50)),
        assistant_msg.as_ref().map(|s| truncate_str(s, 50))
    ));

    // Skip if we don't have both messages
    let (user_msg, assistant_msg) = match (user_msg, assistant_msg) {
        (Some(u), Some(a)) => (u, a),
        _ => {
            debug("Skipping - missing user or assistant message");
            return Ok(HookOutput::approve());
        }
    };

    // Update conversation turn with assistant response (always, even if not substantive)
    let turn_id_str = turn_id.map(|u| u.to_string()).unwrap_or_default();
    if !turn_id_str.is_empty() {
        debug(&format!(
            "Saving assistant response to turn: {} (response len: {} chars)",
            turn_id_str,
            assistant_msg.len()
        ));
        update_turn_with_response(&turn_id_str, &assistant_msg);
    } else {
        debug("Warning: No turn_id available, cannot save assistant response to database");
    }

    // Skip if not substantive
    if !should_extract(&user_msg, &assistant_msg) {
        debug("Skipping - turn not substantive");
        return Ok(HookOutput::approve());
    }

    // Build extraction context
    let ctx = ExtractionContext::new(
        user_msg.clone(),
        assistant_msg.clone(),
        claude_session_id.clone(),
        db_session_id.map(|u| u.to_string()).unwrap_or_default(),
        turn_id_str.clone(),
    );

    debug(&format!(
        "Spawning extraction, confidence: {}",
        ctx.confidence()
    ));

    // Spawn background extraction process
    spawn_extraction(&ctx);

    debug("=== Stop hook completed ===");
    Ok(HookOutput::approve())
}

/// Spawn background process to extract conclusions using claude --print
fn spawn_extraction(ctx: &ExtractionContext) {
    let prompt = build_extraction_prompt(&ctx.user_msg, &ctx.assistant_response);
    let confidence = ctx.confidence().to_string();
    let claude_session_id = ctx.claude_session_id.clone();
    let db_session_id = ctx.db_session_id.clone();
    let turn_id = ctx.turn_id.clone();

    debug(&format!(
        "Spawning detached extraction for session: {}, db_session: {}, turn: {}",
        claude_session_id, db_session_id, turn_id
    ));

    // Escape prompt for shell (replace single quotes with escaped version)
    let escaped_prompt = prompt.replace('\'', "'\"'\"'");

    // Build optional flags for session and turn IDs
    let session_flag = if !db_session_id.is_empty() {
        format!("--session \"{}\"", db_session_id)
    } else {
        String::new()
    };
    let turn_flag = if !turn_id.is_empty() {
        format!("--turn \"{}\"", turn_id)
    } else {
        String::new()
    };

    // Build shell command that runs in background
    // The script: runs claude --print, parses JSON, saves to memory
    let script = format!(
        r#"
LOG="/tmp/claude-stop-hook-rust.log"
log() {{ echo "[$(date -u +%Y-%m-%dT%H:%M:%S.000Z)] $1" >> "$LOG"; }}

log "Extraction subprocess started"

# Run claude --print
OUTPUT=$(CLAUDE_MEMORY_EXTRACTION=1 claude --print -p '{escaped_prompt}' 2>/dev/null)
log "claude --print completed, output length: ${{#OUTPUT}}"

# Extract JSON from output (find first {{ to last }})
JSON=$(echo "$OUTPUT" | grep -o '{{.*}}' | head -1)
if [ -z "$JSON" ]; then
    log "Failed to extract JSON from output: ${{OUTPUT:0:200}}"
    exit 1
fi

log "Extracted JSON: $JSON"

# Parse JSON fields using jq
TYPE=$(echo "$JSON" | jq -r '.type // empty')
CONCLUSION=$(echo "$JSON" | jq -r '.conclusion // empty')
TAGS=$(echo "$JSON" | jq -r '.tags // empty')

if [ -z "$TYPE" ] || [ -z "$CONCLUSION" ]; then
    log "Missing required fields: type=$TYPE, conclusion=$CONCLUSION"
    exit 1
fi

log "Parsed: type=$TYPE, conclusion=${{CONCLUSION:0:50}}..."

# Save to memory
claude-hippocampus add-memory "$TYPE" "$CONCLUSION" "$TAGS" "{confidence}" project --claude-session "{claude_session_id}" {session_flag} {turn_flag} >> "$LOG" 2>&1
log "Memory saved successfully"
"#,
        escaped_prompt = escaped_prompt,
        confidence = confidence,
        claude_session_id = claude_session_id,
        session_flag = session_flag,
        turn_flag = turn_flag
    );

    // Spawn as detached background process using nohup
    match Command::new("sh")
        .arg("-c")
        .arg(format!("nohup sh -c '{}' >/dev/null 2>&1 &", script.replace('\'', "'\\''")))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(_) => debug("Detached extraction process spawned"),
        Err(e) => debug(&format!("Failed to spawn extraction: {}", e)),
    }
}

/// Represents a parsed transcript entry
#[derive(Debug, Clone)]
pub struct TranscriptEntry {
    pub entry_type: String,
    pub content: Option<String>,
}

/// Represents an extracted memory decision from Claude
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ExtractionResult {
    pub memory_type: String,
    pub conclusion: String,
    pub tags: String,
}

/// Context for extraction process
#[derive(Debug, Clone)]
pub struct ExtractionContext {
    pub user_msg: String,
    pub assistant_response: String,
    /// Claude's session identifier (for marker files)
    pub claude_session_id: String,
    /// Database session UUID
    pub db_session_id: String,
    /// Database turn UUID
    pub turn_id: String,
}

impl ExtractionContext {
    pub fn new(
        user_msg: String,
        assistant_response: String,
        claude_session_id: String,
        db_session_id: String,
        turn_id: String,
    ) -> Self {
        Self {
            user_msg,
            assistant_response,
            claude_session_id,
            db_session_id,
            turn_id,
        }
    }

    /// Determine confidence level based on user message patterns
    pub fn confidence(&self) -> &'static str {
        if is_correction(&self.user_msg) {
            "high"
        } else {
            "medium"
        }
    }
}

/// Parse JSON response from Claude extraction
#[allow(dead_code)]
fn parse_extraction_response(output: &str) -> Option<ExtractionResult> {
    // Find JSON in output (might have extra text before/after)
    let start = output.find('{')?;
    let end = output.rfind('}')? + 1;
    let json_str = &output[start..end];

    let json: serde_json::Value = serde_json::from_str(json_str).ok()?;

    // Extract required fields
    let memory_type = json.get("type")?.as_str()?.to_string();
    let conclusion = json.get("conclusion")?.as_str()?.to_string();
    let tags = json
        .get("tags")
        .and_then(|t| t.as_str())
        .unwrap_or("")
        .to_string();

    // Validate required fields are present
    if memory_type.is_empty() || conclusion.is_empty() {
        return None;
    }

    Some(ExtractionResult {
        memory_type,
        conclusion,
        tags,
    })
}

/// Build the extraction prompt for Claude --print
fn build_extraction_prompt(user_msg: &str, assistant_response: &str) -> String {
    // Truncate inputs for efficiency
    let user_preview: String = user_msg.chars().take(500).collect();
    let response_preview: String = assistant_response.chars().take(1000).collect();

    format!(
        r#"You are a memory extraction assistant. Extract a conclusion from this conversation turn.

USER PROMPT:
{}

ASSISTANT RESPONSE:
{}

TASK: Extract a concise conclusion from this turn. ALWAYS save something unless it's completely trivial (just "yes", "ok", greeting).

Output JSON:
{{"type": "<learning|gotcha|convention|architecture|api|preference>", "conclusion": "<max 150 chars summarizing the turn>", "tags": "<comma,separated>"}}

For most turns, use type "learning". Use "gotcha" for corrections or warnings, "convention" for patterns, "architecture" for design decisions.

Output ONLY the JSON, nothing else."#,
        user_preview, response_preview
    )
}

/// Detect if user message is a correction (warrants high confidence)
fn is_correction(user_msg: &str) -> bool {
    let lower = user_msg.to_lowercase();

    // Correction patterns (from JS implementation)
    let patterns = [
        "actually",     // "actually, use X instead"
        "no, use",      // "no, use sqlx instead"
        "no, it",       // "no, it's not like that"
        "always use",   // "always use async here"
        "always do",
        "never use",    // "never do that in production"
        "never do",
        "should be",    // "should be different"
        "must be",
        "must use",
        "not like that",
        "the correct",  // "the correct way is..."
        "remember that",
        "remember to",
        "this project uses",
        "this project requires",
    ];

    patterns.iter().any(|p| lower.contains(p))
}

/// Check if a turn is substantive enough to warrant extraction
fn should_extract(user_msg: &str, assistant_response: &str) -> bool {
    // Skip very short interactions
    if user_msg.len() < 20 && assistant_response.len() < 100 {
        return false;
    }

    let user_trimmed = user_msg.trim().to_lowercase();

    // Skip simple acknowledgments
    let skip_patterns = [
        "yes", "no", "ok", "okay", "sure", "thanks", "got it", "done",
        "commit", "push", "pull", "test", "build", "run", "help",
    ];
    if skip_patterns.contains(&user_trimmed.as_str()) {
        return false;
    }

    // Skip slash commands
    if user_trimmed.starts_with('/') {
        return false;
    }

    true
}

/// Extract the last user message and last assistant response from transcript
fn extract_last_messages(transcript: &str) -> (Option<String>, Option<String>) {
    let lines: Vec<&str> = transcript.lines().collect();

    let mut last_user_msg: Option<String> = None;
    let mut last_assistant_msg: Option<String> = None;

    // Iterate backwards to find the last of each type
    for line in lines.iter().rev() {
        if let Some(entry) = parse_transcript_line(line) {
            match entry.entry_type.as_str() {
                "user" if last_user_msg.is_none() => {
                    // Only use user messages with string content (not tool results)
                    if entry.content.is_some() {
                        last_user_msg = entry.content;
                    }
                }
                "assistant" if last_assistant_msg.is_none() => {
                    last_assistant_msg = entry.content;
                }
                _ => {}
            }
        }

        // Stop once we have both
        if last_user_msg.is_some() && last_assistant_msg.is_some() {
            break;
        }
    }

    (last_user_msg, last_assistant_msg)
}

/// Parse a single JSONL line from the transcript
fn parse_transcript_line(line: &str) -> Option<TranscriptEntry> {
    let json: serde_json::Value = serde_json::from_str(line).ok()?;

    let entry_type = json.get("type")?.as_str()?.to_string();

    // Extract content based on message structure
    let content = json
        .get("message")
        .and_then(|m| m.get("content"))
        .and_then(|c| {
            // Content can be a string (user) or array of blocks (assistant)
            if let Some(s) = c.as_str() {
                Some(s.to_string())
            } else if let Some(arr) = c.as_array() {
                // Extract text from text blocks, join with newlines
                let texts: Vec<String> = arr
                    .iter()
                    .filter_map(|block| {
                        if block.get("type")?.as_str()? == "text" {
                            block.get("text")?.as_str().map(String::from)
                        } else {
                            None
                        }
                    })
                    .collect();
                if texts.is_empty() {
                    None
                } else {
                    Some(texts.join("\n"))
                }
            } else {
                None
            }
        });

    Some(TranscriptEntry {
        entry_type,
        content,
    })
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
    // Transcript parsing tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_parse_transcript_line_user_message() {
        let line = r#"{"type":"user","message":{"content":"Hello world"}}"#;
        let entry = parse_transcript_line(line).expect("should parse");
        assert_eq!(entry.entry_type, "user");
        assert_eq!(entry.content, Some("Hello world".to_string()));
    }

    #[test]
    fn test_parse_transcript_line_assistant_response() {
        let line = r#"{"type":"assistant","message":{"content":[{"type":"text","text":"This is the response."},{"type":"text","text":" More text."}]}}"#;
        let entry = parse_transcript_line(line).expect("should parse");
        assert_eq!(entry.entry_type, "assistant");
        assert_eq!(entry.content, Some("This is the response.\n More text.".to_string()));
    }

    #[test]
    fn test_parse_transcript_line_invalid_json() {
        let line = "not valid json";
        assert!(parse_transcript_line(line).is_none());
    }

    #[test]
    fn test_parse_transcript_line_tool_result() {
        // Tool results have content as array, not string - should return None for content
        let line = r#"{"type":"user","message":{"content":[{"type":"tool_result","content":"result"}]}}"#;
        let entry = parse_transcript_line(line).expect("should parse type");
        assert_eq!(entry.entry_type, "user");
        // Content should be None for non-text user messages
        assert!(entry.content.is_none());
    }

    // -------------------------------------------------------------------------
    // Extract last messages tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_extract_last_messages_basic() {
        let transcript = r#"{"type":"user","message":{"content":"What is Rust?"}}
{"type":"assistant","message":{"content":[{"type":"text","text":"Rust is a systems programming language."}]}}"#;

        let (user_msg, assistant_msg) = extract_last_messages(transcript);
        assert_eq!(user_msg, Some("What is Rust?".to_string()));
        assert_eq!(assistant_msg, Some("Rust is a systems programming language.".to_string()));
    }

    #[test]
    fn test_extract_last_messages_multiple_turns() {
        let transcript = r#"{"type":"user","message":{"content":"Hello"}}
{"type":"assistant","message":{"content":[{"type":"text","text":"Hi there!"}]}}
{"type":"user","message":{"content":"What is 2+2?"}}
{"type":"assistant","message":{"content":[{"type":"text","text":"4"}]}}"#;

        let (user_msg, assistant_msg) = extract_last_messages(transcript);
        // Should get the LAST user text and LAST assistant response
        assert_eq!(user_msg, Some("What is 2+2?".to_string()));
        assert_eq!(assistant_msg, Some("4".to_string()));
    }

    #[test]
    fn test_extract_last_messages_empty() {
        let (user_msg, assistant_msg) = extract_last_messages("");
        assert!(user_msg.is_none());
        assert!(assistant_msg.is_none());
    }

    // -------------------------------------------------------------------------
    // Substantive check tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_should_extract_substantive_content() {
        assert!(should_extract("How do I implement a binary tree?", "Here's how you implement..."));
    }

    #[test]
    fn test_should_extract_skips_trivial() {
        assert!(!should_extract("yes", "ok"));
        assert!(!should_extract("ok", "done"));
        assert!(!should_extract("thanks", "You're welcome"));
    }

    #[test]
    fn test_should_extract_skips_slash_commands() {
        assert!(!should_extract("/commit", "Creating commit..."));
        assert!(!should_extract("/help", "Here's help..."));
    }

    #[test]
    fn test_should_extract_skips_short_interactions() {
        assert!(!should_extract("hi", "hello"));
    }

    // -------------------------------------------------------------------------
    // Confidence detection tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_detect_correction_actually() {
        assert!(is_correction("actually, use tokio instead"));
        assert!(is_correction("Actually it's the other way"));
    }

    #[test]
    fn test_detect_correction_no_use() {
        assert!(is_correction("no, use sqlx instead"));
        assert!(is_correction("No, it's not like that"));
    }

    #[test]
    fn test_detect_correction_always_never() {
        assert!(is_correction("always use async here"));
        assert!(is_correction("never do that in production"));
    }

    #[test]
    fn test_detect_correction_normal_message() {
        assert!(!is_correction("How do I implement this?"));
        assert!(!is_correction("What's the best way to do X?"));
    }

    // -------------------------------------------------------------------------
    // Build extraction prompt tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_build_extraction_prompt_contains_user_msg() {
        let prompt = build_extraction_prompt("How to use async?", "Use tokio...");
        assert!(prompt.contains("How to use async?"));
    }

    #[test]
    fn test_build_extraction_prompt_contains_response() {
        let prompt = build_extraction_prompt("Question", "Use tokio for async runtime");
        assert!(prompt.contains("Use tokio for async runtime"));
    }

    #[test]
    fn test_build_extraction_prompt_contains_json_format() {
        let prompt = build_extraction_prompt("Q", "A");
        assert!(prompt.contains("\"type\""));
        assert!(prompt.contains("\"conclusion\""));
        assert!(prompt.contains("\"tags\""));
    }

    #[test]
    fn test_build_extraction_prompt_truncates_long_input() {
        let long_msg = "x".repeat(1000);
        let long_response = "y".repeat(2000);
        let prompt = build_extraction_prompt(&long_msg, &long_response);
        // Should truncate to reasonable limits
        assert!(prompt.len() < 3000);
    }

    // -------------------------------------------------------------------------
    // Parse extraction response tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_parse_extraction_response_valid() {
        let output = r#"{"type": "learning", "conclusion": "Use tokio for async", "tags": "rust,async"}"#;
        let result = parse_extraction_response(output).expect("should parse");
        assert_eq!(result.memory_type, "learning");
        assert_eq!(result.conclusion, "Use tokio for async");
        assert_eq!(result.tags, "rust,async");
    }

    #[test]
    fn test_parse_extraction_response_with_extra_text() {
        let output = "Here's the extraction:\n{\"type\": \"gotcha\", \"conclusion\": \"Watch out\", \"tags\": \"warning\"}";
        let result = parse_extraction_response(output).expect("should parse");
        assert_eq!(result.memory_type, "gotcha");
    }

    #[test]
    fn test_parse_extraction_response_invalid() {
        assert!(parse_extraction_response("not json").is_none());
        assert!(parse_extraction_response("").is_none());
    }

    #[test]
    fn test_parse_extraction_response_missing_fields() {
        let output = r#"{"type": "learning"}"#;
        assert!(parse_extraction_response(output).is_none());
    }

    // -------------------------------------------------------------------------
    // Extraction context tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_extraction_context_new() {
        let ctx = ExtractionContext::new(
            "user msg".to_string(),
            "assistant response".to_string(),
            "claude-session-123".to_string(),
            "db-session-456".to_string(),
            "turn-789".to_string(),
        );
        assert_eq!(ctx.user_msg, "user msg");
        assert_eq!(ctx.assistant_response, "assistant response");
        assert_eq!(ctx.claude_session_id, "claude-session-123");
        assert_eq!(ctx.db_session_id, "db-session-456");
        assert_eq!(ctx.turn_id, "turn-789");
    }

    #[test]
    fn test_extraction_context_confidence_high_for_correction() {
        let ctx = ExtractionContext::new(
            "actually, use tokio instead".to_string(),
            "Got it, using tokio".to_string(),
            "claude-s".to_string(),
            "db-s".to_string(),
            "t".to_string(),
        );
        assert_eq!(ctx.confidence(), "high");
    }

    #[test]
    fn test_extraction_context_confidence_medium_normally() {
        let ctx = ExtractionContext::new(
            "How do I implement a queue?".to_string(),
            "Here's how...".to_string(),
            "claude-s".to_string(),
            "db-s".to_string(),
            "t".to_string(),
        );
        assert_eq!(ctx.confidence(), "medium");
    }

    // -------------------------------------------------------------------------
    // Integration test - full extraction flow
    // -------------------------------------------------------------------------

    #[tokio::test]
    async fn test_handle_stop_with_transcript() {
        // Create a temp transcript file
        let session_id = format!("test-integration-{}", uuid::Uuid::new_v4());
        cleanup_marker(&session_id);

        let transcript = r#"{"type":"user","message":{"content":"How do I implement async in Rust?"}}
{"type":"assistant","message":{"content":[{"type":"text","text":"Use tokio as your async runtime. Add tokio to Cargo.toml and use #[tokio::main] on your main function."}]}}"#;

        let temp_file = format!("/tmp/test-transcript-{}.jsonl", session_id);
        fs::write(&temp_file, transcript).unwrap();

        let input = HookInput {
            session_id: Some(session_id.clone()),
            prompt: None,
            transcript_path: Some(temp_file.clone()),
            cwd: None,
            permission_mode: None,
            hook_event_name: Some("Stop".to_string()),
        };

        let result = handle_stop(&input).await.unwrap();
        assert_eq!(result.decision, "approve");

        // Verify marker was created
        let marker_file = get_marker_file(&session_id);
        assert!(Path::new(&marker_file).exists());

        // Cleanup
        cleanup_marker(&session_id);
        let _ = fs::remove_file(&temp_file);
    }

    #[tokio::test]
    async fn test_handle_stop_skips_trivial_transcript() {
        let session_id = format!("test-trivial-{}", uuid::Uuid::new_v4());
        cleanup_marker(&session_id);

        // Trivial interaction
        let transcript = r#"{"type":"user","message":{"content":"ok"}}
{"type":"assistant","message":{"content":[{"type":"text","text":"Got it."}]}}"#;

        let temp_file = format!("/tmp/test-transcript-trivial-{}.jsonl", session_id);
        fs::write(&temp_file, transcript).unwrap();

        let input = HookInput {
            session_id: Some(session_id.clone()),
            prompt: None,
            transcript_path: Some(temp_file.clone()),
            cwd: None,
            permission_mode: None,
            hook_event_name: Some("Stop".to_string()),
        };

        let result = handle_stop(&input).await.unwrap();
        assert_eq!(result.decision, "approve");

        // Cleanup
        cleanup_marker(&session_id);
        let _ = fs::remove_file(&temp_file);
    }

    // -------------------------------------------------------------------------
    // Marker file path tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_get_marker_file() {
        let path = get_marker_file("test-session");
        assert_eq!(path, "/tmp/hippocampus-brain-cells-extract-test-session");
    }

    #[test]
    fn test_get_marker_file_prefix() {
        let path = get_marker_file("any-id");
        assert!(path.starts_with("/tmp/hippocampus-brain-cells-extract-"));
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
