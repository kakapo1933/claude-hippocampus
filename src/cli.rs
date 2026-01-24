// CLI Parser - Clap derive definitions
// Matches exact argument signatures from Node.js memory.js

use clap::{Parser, Subcommand};

use crate::models::memory::{Confidence, MemoryType, Scope, Tier};

/// Claude-Hippocampus: Memory System CLI
#[derive(Parser, Debug)]
#[command(name = "claude-hippocampus")]
#[command(version)]
#[command(about = "PostgreSQL-backed persistent memory for Claude Code sessions")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Add a new memory entry
    AddMemory {
        /// Memory type: convention, architecture, gotcha, api, learning, preference
        #[arg(value_parser = parse_memory_type)]
        memory_type: MemoryType,
        /// The content of the memory
        content: String,
        /// Comma-separated tags (optional)
        #[arg(default_value = "")]
        tags: String,
        /// Confidence level: high, medium, low
        #[arg(default_value = "high", value_parser = parse_confidence)]
        confidence: Confidence,
        /// Tier: project, global
        #[arg(default_value = "project", value_parser = parse_scope)]
        tier: Scope,
        /// Source session ID
        #[arg(long = "session")]
        source_session_id: Option<String>,
        /// Source turn ID
        #[arg(long = "turn")]
        source_turn_id: Option<String>,
        /// Claude session ID (for session state file lookup)
        #[arg(long = "claude-session")]
        claude_session_id: Option<String>,
    },

    /// Update an existing memory entry
    UpdateMemory {
        /// Memory ID (UUID)
        id: String,
        /// New content
        content: String,
        /// Tier: project, global
        #[arg(default_value = "project", value_parser = parse_scope)]
        tier: Scope,
    },

    /// Delete a memory entry
    DeleteMemory {
        /// Memory ID (UUID)
        id: String,
        /// Tier: project, global
        #[arg(default_value = "project", value_parser = parse_scope)]
        tier: Scope,
    },

    /// Get a memory entry by ID
    GetMemory {
        /// Memory ID (UUID)
        id: String,
    },

    /// Search memories by keyword
    SearchKeyword {
        /// Search query
        query: String,
        /// Tier filter: project, global, both
        #[arg(default_value = "both", value_parser = parse_tier)]
        tier: Tier,
        /// Maximum results to return
        #[arg(default_value = "30")]
        limit: i64,
    },

    /// Get context block for injection
    GetContext {
        /// Maximum entries to return
        #[arg(default_value = "10")]
        limit: i64,
    },

    /// List recent memory entries
    ListRecent {
        /// Number of entries
        #[arg(default_value = "10")]
        n: i64,
        /// Tier filter: project, global, both
        #[arg(default_value = "both", value_parser = parse_tier)]
        tier: Tier,
    },

    /// Merge duplicate memory entries
    Consolidate {
        /// Tier: project, global
        #[arg(default_value = "project", value_parser = parse_scope)]
        tier: Scope,
    },

    /// Remove old low-confidence entries
    Prune {
        /// Days threshold (remove entries older than this)
        #[arg(default_value = "90")]
        days: i64,
        /// Tier: project, global
        #[arg(default_value = "project", value_parser = parse_scope)]
        tier: Scope,
    },

    /// Save session summary
    SaveSessionSummary {
        /// Summary text
        summary: String,
    },

    /// View operation logs
    Logs {
        /// Number of log entries
        #[arg(default_value = "50")]
        n: i64,
        /// Filter by operation type
        operation: Option<String>,
    },

    /// Clear all logs
    ClearLogs,

    // =========================================================================
    // Session Management Commands
    // =========================================================================

    /// Create a new session
    CreateSession {
        /// Claude's session identifier (required)
        #[arg(long = "claude-session-id")]
        claude_session_id: String,
        /// Project path (optional, defaults to current dir)
        #[arg(long = "project-path")]
        project_path: Option<String>,
    },

    /// Get session by ID
    GetSession {
        /// Session ID (UUID or claude_session_id)
        id: String,
    },

    /// End a session
    EndSession {
        /// Session ID (UUID or claude_session_id)
        id: String,
        /// Session summary (optional)
        #[arg(long = "summary")]
        summary: Option<String>,
    },

    // =========================================================================
    // Turn Management Commands
    // =========================================================================

    /// Create a new conversation turn
    CreateTurn {
        /// Session ID (claude_session_id, required)
        #[arg(long = "session")]
        session_id: String,
        /// User's prompt text
        #[arg(long = "prompt")]
        prompt: String,
        /// Model used (optional)
        #[arg(long = "model")]
        model: Option<String>,
    },

    /// Update a turn with assistant response
    UpdateTurn {
        /// Turn ID (UUID, required)
        #[arg(long = "turn-id")]
        turn_id: String,
        /// Assistant's response text
        #[arg(long = "response")]
        response: String,
        /// Input tokens consumed (optional)
        #[arg(long = "input-tokens")]
        input_tokens: Option<i32>,
        /// Output tokens generated (optional)
        #[arg(long = "output-tokens")]
        output_tokens: Option<i32>,
    },
}

// Custom parsers for enum types
fn parse_memory_type(s: &str) -> Result<MemoryType, String> {
    s.parse::<MemoryType>().map_err(|e| format!("{}", e))
}

fn parse_confidence(s: &str) -> Result<Confidence, String> {
    s.parse::<Confidence>().map_err(|e| format!("{}", e))
}

fn parse_scope(s: &str) -> Result<Scope, String> {
    s.parse::<Scope>().map_err(|e| format!("{}", e))
}

fn parse_tier(s: &str) -> Result<Tier, String> {
    s.parse::<Tier>().map_err(|e| format!("{}", e))
}

/// Parse comma-separated tags into a vector
pub fn parse_tags(tags_str: &str) -> Vec<String> {
    if tags_str.is_empty() {
        Vec::new()
    } else {
        tags_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    // -------------------------------------------------------------------------
    // AddMemory command tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_add_memory_minimal_args() {
        let cli = Cli::parse_from(["claude-hippocampus", "add-memory", "learning", "Test content"]);
        match cli.command {
            Command::AddMemory {
                memory_type,
                content,
                tags,
                confidence,
                tier,
                source_session_id,
                source_turn_id,
                claude_session_id,
            } => {
                assert_eq!(memory_type, MemoryType::Learning);
                assert_eq!(content, "Test content");
                assert_eq!(tags, "");
                assert_eq!(confidence, Confidence::High);
                assert_eq!(tier, Scope::Project);
                assert!(source_session_id.is_none());
                assert!(source_turn_id.is_none());
                assert!(claude_session_id.is_none());
            }
            _ => panic!("Expected AddMemory command"),
        }
    }

    #[test]
    fn test_add_memory_all_args() {
        let cli = Cli::parse_from([
            "claude-hippocampus",
            "add-memory",
            "gotcha",
            "Found a bug",
            "bug,important",
            "high",
            "global",
            "--session=sess-123",
            "--turn=turn-456",
            "--claude-session=claude-789",
        ]);
        match cli.command {
            Command::AddMemory {
                memory_type,
                content,
                tags,
                confidence,
                tier,
                source_session_id,
                source_turn_id,
                claude_session_id,
            } => {
                assert_eq!(memory_type, MemoryType::Gotcha);
                assert_eq!(content, "Found a bug");
                assert_eq!(tags, "bug,important");
                assert_eq!(confidence, Confidence::High);
                assert_eq!(tier, Scope::Global);
                assert_eq!(source_session_id, Some("sess-123".to_string()));
                assert_eq!(source_turn_id, Some("turn-456".to_string()));
                assert_eq!(claude_session_id, Some("claude-789".to_string()));
            }
            _ => panic!("Expected AddMemory command"),
        }
    }

    #[test]
    fn test_add_memory_all_types() {
        for (type_str, expected) in [
            ("convention", MemoryType::Convention),
            ("architecture", MemoryType::Architecture),
            ("gotcha", MemoryType::Gotcha),
            ("api", MemoryType::Api),
            ("learning", MemoryType::Learning),
            ("preference", MemoryType::Preference),
        ] {
            let cli = Cli::parse_from(["claude-hippocampus", "add-memory", type_str, "content"]);
            match cli.command {
                Command::AddMemory { memory_type, .. } => {
                    assert_eq!(memory_type, expected);
                }
                _ => panic!("Expected AddMemory command"),
            }
        }
    }

    #[test]
    fn test_add_memory_confidence_levels() {
        for (conf_str, expected) in [
            ("high", Confidence::High),
            ("medium", Confidence::Medium),
            ("low", Confidence::Low),
        ] {
            let cli = Cli::parse_from([
                "claude-hippocampus",
                "add-memory",
                "learning",
                "content",
                "tags",
                conf_str,
            ]);
            match cli.command {
                Command::AddMemory { confidence, .. } => {
                    assert_eq!(confidence, expected);
                }
                _ => panic!("Expected AddMemory command"),
            }
        }
    }

    // -------------------------------------------------------------------------
    // UpdateMemory command tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_update_memory_minimal() {
        let cli = Cli::parse_from([
            "claude-hippocampus",
            "update-memory",
            "550e8400-e29b-41d4-a716-446655440000",
            "New content",
        ]);
        match cli.command {
            Command::UpdateMemory { id, content, tier } => {
                assert_eq!(id, "550e8400-e29b-41d4-a716-446655440000");
                assert_eq!(content, "New content");
                assert_eq!(tier, Scope::Project);
            }
            _ => panic!("Expected UpdateMemory command"),
        }
    }

    #[test]
    fn test_update_memory_with_tier() {
        let cli = Cli::parse_from([
            "claude-hippocampus",
            "update-memory",
            "550e8400-e29b-41d4-a716-446655440000",
            "New content",
            "global",
        ]);
        match cli.command {
            Command::UpdateMemory { tier, .. } => {
                assert_eq!(tier, Scope::Global);
            }
            _ => panic!("Expected UpdateMemory command"),
        }
    }

    // -------------------------------------------------------------------------
    // DeleteMemory command tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_delete_memory_minimal() {
        let cli = Cli::parse_from([
            "claude-hippocampus",
            "delete-memory",
            "550e8400-e29b-41d4-a716-446655440000",
        ]);
        match cli.command {
            Command::DeleteMemory { id, tier } => {
                assert_eq!(id, "550e8400-e29b-41d4-a716-446655440000");
                assert_eq!(tier, Scope::Project);
            }
            _ => panic!("Expected DeleteMemory command"),
        }
    }

    #[test]
    fn test_delete_memory_with_tier() {
        let cli = Cli::parse_from([
            "claude-hippocampus",
            "delete-memory",
            "550e8400-e29b-41d4-a716-446655440000",
            "global",
        ]);
        match cli.command {
            Command::DeleteMemory { tier, .. } => {
                assert_eq!(tier, Scope::Global);
            }
            _ => panic!("Expected DeleteMemory command"),
        }
    }

    // -------------------------------------------------------------------------
    // GetMemory command tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_get_memory() {
        let cli = Cli::parse_from([
            "claude-hippocampus",
            "get-memory",
            "550e8400-e29b-41d4-a716-446655440000",
        ]);
        match cli.command {
            Command::GetMemory { id } => {
                assert_eq!(id, "550e8400-e29b-41d4-a716-446655440000");
            }
            _ => panic!("Expected GetMemory command"),
        }
    }

    // -------------------------------------------------------------------------
    // SearchKeyword command tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_search_keyword_minimal() {
        let cli = Cli::parse_from(["claude-hippocampus", "search-keyword", "test query"]);
        match cli.command {
            Command::SearchKeyword { query, tier, limit } => {
                assert_eq!(query, "test query");
                assert_eq!(tier, Tier::Both);
                assert_eq!(limit, 30);
            }
            _ => panic!("Expected SearchKeyword command"),
        }
    }

    #[test]
    fn test_search_keyword_with_tier() {
        let cli = Cli::parse_from([
            "claude-hippocampus",
            "search-keyword",
            "query",
            "project",
        ]);
        match cli.command {
            Command::SearchKeyword { tier, .. } => {
                assert_eq!(tier, Tier::Project);
            }
            _ => panic!("Expected SearchKeyword command"),
        }
    }

    #[test]
    fn test_search_keyword_with_limit() {
        let cli = Cli::parse_from([
            "claude-hippocampus",
            "search-keyword",
            "query",
            "both",
            "50",
        ]);
        match cli.command {
            Command::SearchKeyword { limit, .. } => {
                assert_eq!(limit, 50);
            }
            _ => panic!("Expected SearchKeyword command"),
        }
    }

    // -------------------------------------------------------------------------
    // GetContext command tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_get_context_default() {
        let cli = Cli::parse_from(["claude-hippocampus", "get-context"]);
        match cli.command {
            Command::GetContext { limit } => {
                assert_eq!(limit, 10);
            }
            _ => panic!("Expected GetContext command"),
        }
    }

    #[test]
    fn test_get_context_with_limit() {
        let cli = Cli::parse_from(["claude-hippocampus", "get-context", "25"]);
        match cli.command {
            Command::GetContext { limit } => {
                assert_eq!(limit, 25);
            }
            _ => panic!("Expected GetContext command"),
        }
    }

    // -------------------------------------------------------------------------
    // ListRecent command tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_list_recent_default() {
        let cli = Cli::parse_from(["claude-hippocampus", "list-recent"]);
        match cli.command {
            Command::ListRecent { n, tier } => {
                assert_eq!(n, 10);
                assert_eq!(tier, Tier::Both);
            }
            _ => panic!("Expected ListRecent command"),
        }
    }

    #[test]
    fn test_list_recent_with_args() {
        let cli = Cli::parse_from(["claude-hippocampus", "list-recent", "20", "global"]);
        match cli.command {
            Command::ListRecent { n, tier } => {
                assert_eq!(n, 20);
                assert_eq!(tier, Tier::Global);
            }
            _ => panic!("Expected ListRecent command"),
        }
    }

    // -------------------------------------------------------------------------
    // Consolidate command tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_consolidate_default() {
        let cli = Cli::parse_from(["claude-hippocampus", "consolidate"]);
        match cli.command {
            Command::Consolidate { tier } => {
                assert_eq!(tier, Scope::Project);
            }
            _ => panic!("Expected Consolidate command"),
        }
    }

    #[test]
    fn test_consolidate_with_tier() {
        let cli = Cli::parse_from(["claude-hippocampus", "consolidate", "global"]);
        match cli.command {
            Command::Consolidate { tier } => {
                assert_eq!(tier, Scope::Global);
            }
            _ => panic!("Expected Consolidate command"),
        }
    }

    // -------------------------------------------------------------------------
    // Prune command tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_prune_default() {
        let cli = Cli::parse_from(["claude-hippocampus", "prune"]);
        match cli.command {
            Command::Prune { days, tier } => {
                assert_eq!(days, 90);
                assert_eq!(tier, Scope::Project);
            }
            _ => panic!("Expected Prune command"),
        }
    }

    #[test]
    fn test_prune_with_args() {
        let cli = Cli::parse_from(["claude-hippocampus", "prune", "30", "global"]);
        match cli.command {
            Command::Prune { days, tier } => {
                assert_eq!(days, 30);
                assert_eq!(tier, Scope::Global);
            }
            _ => panic!("Expected Prune command"),
        }
    }

    // -------------------------------------------------------------------------
    // SaveSessionSummary command tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_save_session_summary() {
        let cli = Cli::parse_from([
            "claude-hippocampus",
            "save-session-summary",
            "This session was about implementing TDD",
        ]);
        match cli.command {
            Command::SaveSessionSummary { summary } => {
                assert_eq!(summary, "This session was about implementing TDD");
            }
            _ => panic!("Expected SaveSessionSummary command"),
        }
    }

    // -------------------------------------------------------------------------
    // Logs command tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_logs_default() {
        let cli = Cli::parse_from(["claude-hippocampus", "logs"]);
        match cli.command {
            Command::Logs { n, operation } => {
                assert_eq!(n, 50);
                assert!(operation.is_none());
            }
            _ => panic!("Expected Logs command"),
        }
    }

    #[test]
    fn test_logs_with_args() {
        let cli = Cli::parse_from(["claude-hippocampus", "logs", "100", "addMemory"]);
        match cli.command {
            Command::Logs { n, operation } => {
                assert_eq!(n, 100);
                assert_eq!(operation, Some("addMemory".to_string()));
            }
            _ => panic!("Expected Logs command"),
        }
    }

    // -------------------------------------------------------------------------
    // ClearLogs command tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_clear_logs() {
        let cli = Cli::parse_from(["claude-hippocampus", "clear-logs"]);
        match cli.command {
            Command::ClearLogs => {}
            _ => panic!("Expected ClearLogs command"),
        }
    }

    // -------------------------------------------------------------------------
    // Error case tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_invalid_memory_type_fails() {
        let result = Cli::try_parse_from(["claude-hippocampus", "add-memory", "invalid", "content"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_confidence_fails() {
        let result = Cli::try_parse_from([
            "claude-hippocampus",
            "add-memory",
            "learning",
            "content",
            "tags",
            "invalid",
        ]);
        assert!(result.is_err());
    }

    #[test]
    fn test_missing_required_args_fails() {
        // AddMemory requires type and content
        let result = Cli::try_parse_from(["claude-hippocampus", "add-memory"]);
        assert!(result.is_err());

        // GetMemory requires id
        let result = Cli::try_parse_from(["claude-hippocampus", "get-memory"]);
        assert!(result.is_err());
    }

    // -------------------------------------------------------------------------
    // parse_tags helper tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_parse_tags_empty() {
        assert_eq!(parse_tags(""), Vec::<String>::new());
    }

    #[test]
    fn test_parse_tags_single() {
        assert_eq!(parse_tags("tag1"), vec!["tag1"]);
    }

    #[test]
    fn test_parse_tags_multiple() {
        assert_eq!(parse_tags("tag1,tag2,tag3"), vec!["tag1", "tag2", "tag3"]);
    }

    #[test]
    fn test_parse_tags_with_spaces() {
        assert_eq!(parse_tags("tag1 , tag2 , tag3"), vec!["tag1", "tag2", "tag3"]);
    }

    #[test]
    fn test_parse_tags_with_empty_parts() {
        assert_eq!(parse_tags("tag1,,tag2"), vec!["tag1", "tag2"]);
    }

    // -------------------------------------------------------------------------
    // CreateSession command tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_create_session_with_claude_session_id() {
        let cli = Cli::parse_from([
            "claude-hippocampus",
            "create-session",
            "--claude-session-id=abc-123-def",
        ]);
        match cli.command {
            Command::CreateSession {
                claude_session_id,
                project_path,
            } => {
                assert_eq!(claude_session_id, "abc-123-def");
                assert!(project_path.is_none());
            }
            _ => panic!("Expected CreateSession command"),
        }
    }

    #[test]
    fn test_create_session_with_project_path() {
        let cli = Cli::parse_from([
            "claude-hippocampus",
            "create-session",
            "--claude-session-id=abc-123",
            "--project-path=/path/to/project",
        ]);
        match cli.command {
            Command::CreateSession {
                claude_session_id,
                project_path,
            } => {
                assert_eq!(claude_session_id, "abc-123");
                assert_eq!(project_path, Some("/path/to/project".to_string()));
            }
            _ => panic!("Expected CreateSession command"),
        }
    }

    #[test]
    fn test_create_session_missing_required_arg_fails() {
        // claude-session-id is required
        let result = Cli::try_parse_from(["claude-hippocampus", "create-session"]);
        assert!(result.is_err());
    }

    // -------------------------------------------------------------------------
    // GetSession command tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_get_session_with_uuid() {
        let cli = Cli::parse_from([
            "claude-hippocampus",
            "get-session",
            "550e8400-e29b-41d4-a716-446655440000",
        ]);
        match cli.command {
            Command::GetSession { id } => {
                assert_eq!(id, "550e8400-e29b-41d4-a716-446655440000");
            }
            _ => panic!("Expected GetSession command"),
        }
    }

    #[test]
    fn test_get_session_with_claude_id() {
        let cli = Cli::parse_from([
            "claude-hippocampus",
            "get-session",
            "claude-session-abc123",
        ]);
        match cli.command {
            Command::GetSession { id } => {
                assert_eq!(id, "claude-session-abc123");
            }
            _ => panic!("Expected GetSession command"),
        }
    }

    #[test]
    fn test_get_session_missing_id_fails() {
        let result = Cli::try_parse_from(["claude-hippocampus", "get-session"]);
        assert!(result.is_err());
    }

    // -------------------------------------------------------------------------
    // EndSession command tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_end_session_minimal() {
        let cli = Cli::parse_from([
            "claude-hippocampus",
            "end-session",
            "abc-123",
        ]);
        match cli.command {
            Command::EndSession { id, summary } => {
                assert_eq!(id, "abc-123");
                assert!(summary.is_none());
            }
            _ => panic!("Expected EndSession command"),
        }
    }

    #[test]
    fn test_end_session_with_summary() {
        let cli = Cli::parse_from([
            "claude-hippocampus",
            "end-session",
            "abc-123",
            "--summary=Implemented TDD workflow successfully",
        ]);
        match cli.command {
            Command::EndSession { id, summary } => {
                assert_eq!(id, "abc-123");
                assert_eq!(summary, Some("Implemented TDD workflow successfully".to_string()));
            }
            _ => panic!("Expected EndSession command"),
        }
    }

    #[test]
    fn test_end_session_missing_id_fails() {
        let result = Cli::try_parse_from(["claude-hippocampus", "end-session"]);
        assert!(result.is_err());
    }

    // -------------------------------------------------------------------------
    // CreateTurn command tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_create_turn_basic() {
        let cli = Cli::parse_from([
            "claude-hippocampus",
            "create-turn",
            "--session=abc-123",
            "--prompt=Hello, how are you?",
        ]);
        match cli.command {
            Command::CreateTurn {
                session_id,
                prompt,
                model,
            } => {
                assert_eq!(session_id, "abc-123");
                assert_eq!(prompt, "Hello, how are you?");
                assert!(model.is_none());
            }
            _ => panic!("Expected CreateTurn command"),
        }
    }

    #[test]
    fn test_create_turn_with_model() {
        let cli = Cli::parse_from([
            "claude-hippocampus",
            "create-turn",
            "--session=abc-123",
            "--prompt=Test prompt",
            "--model=claude-3-opus",
        ]);
        match cli.command {
            Command::CreateTurn { model, .. } => {
                assert_eq!(model, Some("claude-3-opus".to_string()));
            }
            _ => panic!("Expected CreateTurn command"),
        }
    }

    #[test]
    fn test_create_turn_missing_required_args_fails() {
        // Missing session
        let result = Cli::try_parse_from([
            "claude-hippocampus",
            "create-turn",
            "--prompt=Test",
        ]);
        assert!(result.is_err());

        // Missing prompt
        let result = Cli::try_parse_from([
            "claude-hippocampus",
            "create-turn",
            "--session=abc",
        ]);
        assert!(result.is_err());
    }

    // -------------------------------------------------------------------------
    // UpdateTurn command tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_update_turn_basic() {
        let cli = Cli::parse_from([
            "claude-hippocampus",
            "update-turn",
            "--turn-id=550e8400-e29b-41d4-a716-446655440000",
            "--response=Here is my response",
        ]);
        match cli.command {
            Command::UpdateTurn {
                turn_id,
                response,
                input_tokens,
                output_tokens,
            } => {
                assert_eq!(turn_id, "550e8400-e29b-41d4-a716-446655440000");
                assert_eq!(response, "Here is my response");
                assert!(input_tokens.is_none());
                assert!(output_tokens.is_none());
            }
            _ => panic!("Expected UpdateTurn command"),
        }
    }

    #[test]
    fn test_update_turn_with_tokens() {
        let cli = Cli::parse_from([
            "claude-hippocampus",
            "update-turn",
            "--turn-id=abc-123",
            "--response=Response text",
            "--input-tokens=100",
            "--output-tokens=250",
        ]);
        match cli.command {
            Command::UpdateTurn {
                input_tokens,
                output_tokens,
                ..
            } => {
                assert_eq!(input_tokens, Some(100));
                assert_eq!(output_tokens, Some(250));
            }
            _ => panic!("Expected UpdateTurn command"),
        }
    }

    #[test]
    fn test_update_turn_missing_required_args_fails() {
        // Missing turn-id
        let result = Cli::try_parse_from([
            "claude-hippocampus",
            "update-turn",
            "--response=Test",
        ]);
        assert!(result.is_err());

        // Missing response
        let result = Cli::try_parse_from([
            "claude-hippocampus",
            "update-turn",
            "--turn-id=abc",
        ]);
        assert!(result.is_err());
    }
}

