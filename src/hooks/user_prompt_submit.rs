//! User prompt submit hook handler.
//!
//! Creates a conversation turn and outputs memory search instructions.

use sqlx::postgres::PgPool;
use std::fs;

use crate::db::queries::{create_turn, find_session_by_claude_id, get_next_turn_number};
use crate::error::Result;
use crate::session::{load_session_state, save_session_state};

use super::{HookInput, HookOutput};

/// Marker file path for stop hook coordination
fn get_marker_file(claude_session_id: &str) -> String {
    format!("/tmp/claude-memory-extract-{}", claude_session_id)
}

/// Check if prompt is substantive enough to warrant memory search
fn should_search_memory(prompt: &str) -> bool {
    if prompt.len() < 15 {
        return false;
    }

    let skip_patterns = [
        "yes", "no", "ok", "okay", "sure", "thanks", "thank you", "got it", "done",
        "commit", "push", "pull", "test", "build", "run", "help",
    ];

    let trimmed = prompt.trim().to_lowercase();

    // Skip if matches simple patterns
    if skip_patterns.iter().any(|p| trimmed == *p) {
        return false;
    }

    // Skip slash commands
    if trimmed.starts_with('/') {
        let cmd = trimmed.split_whitespace().next().unwrap_or("");
        if ["/commit", "/test", "/review", "/help", "/clear"].contains(&cmd) {
            return false;
        }
    }

    true
}

/// Handle the user-prompt-submit hook.
///
/// 1. Skip if extraction instance (prevent recursion)
/// 2. Create conversation turn
/// 3. Clear marker file
/// 4. Output memory search instructions
pub async fn handle_user_prompt_submit(pool: &PgPool, input: &HookInput) -> Result<HookOutput> {
    // Skip if this is an extraction instance (prevent recursion)
    if std::env::var("CLAUDE_MEMORY_EXTRACTION").is_ok() {
        return Ok(HookOutput::approve());
    }

    let prompt = match &input.prompt {
        Some(p) if !p.is_empty() => p.clone(),
        _ => return Ok(HookOutput::approve()),
    };

    let claude_session_id = input.session_id.clone().unwrap_or_default();

    // Load session state
    let _state = load_session_state(Some(&claude_session_id))?;

    // Find session and create turn
    if let Some(session) = find_session_by_claude_id(pool, &claude_session_id).await? {
        let turn_number = get_next_turn_number(pool, session.id).await?;
        let turn = create_turn(pool, session.id, turn_number, &prompt, None).await?;

        // Update session state
        let new_state = crate::session::SessionState {
            session_id: Some(session.id),
            claude_session_id: Some(claude_session_id.clone()),
            turn_number,
            current_turn_id: Some(turn.id),
        };
        save_session_state(&new_state)?;
    }

    // Clear stop hook marker to allow response recording
    let marker_file = get_marker_file(&claude_session_id);
    let _ = fs::remove_file(&marker_file);

    // Build output text
    let mut output_text = String::new();

    // Memory search instructions (if prompt is substantive)
    if should_search_memory(&prompt) {
        let escaped_prompt = prompt
            .replace('"', "\\\"")
            .replace('\n', " ")
            .chars()
            .take(300)
            .collect::<String>();

        output_text.push_str(&format!(r#"<system-reminder>
Search memory for context relevant to this prompt.

Use Task tool with subagent_type="memory-helper" to search memory:

**User prompt:** "{}"

**Process:**
1. Extract 2-4 relevant search keywords from the user prompt
2. Run keyword searches: `claude-hippocampus search-keyword "<keyword>" both 15`
3. Merge results from multiple keyword searches
4. Filter: Remove memories not relevant to the user's question
5. Rank by semantic relevance to the original prompt
6. Summarize: Create concise bullet points (max 5, each under 80 chars)

**Output format:**
```
╭────────────────────────────── 🧠 Memory Found ───────────────────────────────╮
│ ★ [type] Key point (id: abc123)                                              │
│ ◐ [type] Key point (id: def456)                                              │
╰──────────────────────────────────────────────────────────────────────────────╯
```

If nothing relevant:
```
╭────────────────────────────── 🔍 Memory Search ──────────────────────────────╮
│ No relevant memories found                                                   │
╰──────────────────────────────────────────────────────────────────────────────╯
```

Rules:
- Maximum 5 results
- Include memory ID in parentheses: (id: xxxx)
- Use ★ high, ◐ medium, ○ low confidence
- Only include highly relevant memories

**MANDATORY**: After the memory-helper completes, ALWAYS display the memory box to the user in your response. Do not silently process results - the user wants to see what memories were found.
</system-reminder>

"#, escaped_prompt));
    }

    // Correction detection instructions
    output_text.push_str(&format!(r#"<task>Check if this message contains a correction to save to memory.</task>

<correction-patterns>
Look for these patterns indicating a correction:
- "actually" + instruction (e.g., "actually, use X instead")
- "no, use X instead" or "not X, Y"
- "always do X" or "never do Y"
- "the correct way is..."
- "you should X, not Y"
- Explicit corrections of previous behavior
</correction-patterns>

<action>
IF a correction is detected:
1. Extract the core learning (what TO do, not what was wrong)
2. Save immediately as HIGH confidence gotcha:
   claude-hippocampus add-memory gotcha "<learning>" "<relevant-tags>" high project --claude-session {}
3. Display confirmation (wrap in code fence):
   ```
   ╭───────────────────────────── 💾 Memory Saved ────────────────────────────────╮
   │ [learning summary here]                                                      │
   ╰──────────────────────────────────────────────────────────────────────────────╯
   ```

IF no correction detected:
- Do nothing, proceed normally
</action>"#, claude_session_id));

    // Output the instructions followed by JSON
    println!("{}", output_text);
    Ok(HookOutput::approve())
}

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // should_search_memory tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_should_search_memory_short_prompt() {
        assert!(!should_search_memory("yes"));
        assert!(!should_search_memory("no"));
        assert!(!should_search_memory("ok"));
    }

    #[test]
    fn test_should_search_memory_simple_commands() {
        assert!(!should_search_memory("commit"));
        assert!(!should_search_memory("test"));
    }

    #[test]
    fn test_should_search_memory_slash_commands() {
        assert!(!should_search_memory("/commit"));
        assert!(!should_search_memory("/test"));
        assert!(!should_search_memory("/help"));
    }

    #[test]
    fn test_should_search_memory_substantive() {
        assert!(should_search_memory("How do I implement authentication?"));
        assert!(should_search_memory("Fix the bug in the login form"));
        assert!(should_search_memory("What's the architecture of this project?"));
    }

    #[test]
    fn test_should_search_memory_case_insensitive() {
        assert!(!should_search_memory("YES"));
        assert!(!should_search_memory("No"));
        assert!(!should_search_memory("COMMIT"));
    }

    #[test]
    fn test_should_search_memory_with_whitespace() {
        assert!(!should_search_memory("  yes  "));
        assert!(!should_search_memory("\tok\n"));
    }

    #[test]
    fn test_should_search_memory_thanks_variants() {
        assert!(!should_search_memory("thanks"));
        assert!(!should_search_memory("thank you"));
        assert!(!should_search_memory("got it"));
        assert!(!should_search_memory("done"));
    }

    #[test]
    fn test_should_search_memory_slash_clear() {
        assert!(!should_search_memory("/clear"));
        assert!(!should_search_memory("/review"));
    }

    #[test]
    fn test_should_search_memory_too_short() {
        assert!(!should_search_memory("hi"));
        assert!(!should_search_memory("a"));
        assert!(!should_search_memory("")); // empty
    }

    #[test]
    fn test_should_search_memory_just_long_enough() {
        // 15 chars is the minimum
        assert!(!should_search_memory("12345678901234")); // 14 chars
        assert!(should_search_memory("123456789012345")); // 15 chars
    }

    #[test]
    fn test_should_search_memory_complex_question() {
        assert!(should_search_memory("How does the authentication flow work in this codebase?"));
        assert!(should_search_memory("Can you explain the database schema?"));
        assert!(should_search_memory("Where are the API endpoints defined?"));
    }

    #[test]
    fn test_should_search_memory_code_request() {
        assert!(should_search_memory("Write a function to validate email addresses"));
        assert!(should_search_memory("Refactor the user service to use dependency injection"));
    }

    // -------------------------------------------------------------------------
    // Marker file tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_get_marker_file() {
        let path = get_marker_file("abc-123");
        assert_eq!(path, "/tmp/claude-memory-extract-abc-123");
    }

    #[test]
    fn test_get_marker_file_with_special_chars() {
        let path = get_marker_file("session-2024-01-15T10:30:00");
        assert_eq!(path, "/tmp/claude-memory-extract-session-2024-01-15T10:30:00");
    }

    #[test]
    fn test_get_marker_file_empty_session() {
        let path = get_marker_file("");
        assert_eq!(path, "/tmp/claude-memory-extract-");
    }

    #[test]
    fn test_get_marker_file_uuid() {
        let path = get_marker_file("550e8400-e29b-41d4-a716-446655440000");
        assert!(path.contains("550e8400-e29b-41d4-a716-446655440000"));
    }
}
