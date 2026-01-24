//! Claude-Hippocampus: Memory System CLI
//!
//! Main entry point for the CLI application.
//! Dispatches commands to the appropriate handlers and outputs JSON results.

use clap::Parser;
use std::env;
use uuid::Uuid;

use claude_hippocampus::{
    clear_logs, parse_tags, read_logs, Cli, Command, DbConfig, Result,
};
use claude_hippocampus::commands::{
    add_memory, consolidate, delete_memory, get_context, get_memory, list_recent, prune,
    save_session_summary, search_keyword, update_memory, AddMemoryOptions, SearchOptions,
};
use claude_hippocampus::db::create_pool;
use claude_hippocampus::models::{
    ClearLogsData, ErrorResponse, LogsData, Scope, SuccessResponse, Tier,
};

#[tokio::main]
async fn main() {
    // Parse CLI arguments
    let cli = Cli::parse();

    // Run the command and handle errors
    match run(cli).await {
        Ok(json) => {
            println!("{}", serde_json::to_string_pretty(&json).unwrap());
        }
        Err(e) => {
            let error_response = ErrorResponse::new(e.to_string());
            println!("{}", serde_json::to_string_pretty(&error_response).unwrap());
            std::process::exit(1);
        }
    }
}

/// Run the dispatched command
async fn run(cli: Cli) -> Result<serde_json::Value> {
    match cli.command {
        // Commands that don't require database connection
        Command::Logs { n, operation } => {
            let entries = read_logs(n as usize, operation.as_deref())?;
            let count = entries.len();
            let log_data = LogsData {
                entries: entries
                    .into_iter()
                    .map(|e| claude_hippocampus::models::response::LogEntry {
                        timestamp: e.timestamp.to_rfc3339(),
                        level: if e.success { "info".to_string() } else { "error".to_string() },
                        operation: e.operation,
                        details: e.details.map(|d| serde_json::json!({"message": d}))
                            .unwrap_or(serde_json::json!({})),
                    })
                    .collect(),
                count,
                total: count,
            };
            Ok(serde_json::to_value(SuccessResponse::new(log_data))?)
        }

        Command::ClearLogs => {
            let _ = clear_logs()?;
            Ok(serde_json::to_value(SuccessResponse::new(ClearLogsData {
                cleared: true,
            }))?)
        }

        // Commands that require database connection
        _ => {
            // Load database configuration
            let config = DbConfig::load()?;
            let pool = create_pool(&config).await?;

            // Get project path from environment (same as Node.js: PROJECT_PATH)
            // Falls back to current working directory
            let project_path = env::var("PROJECT_PATH")
                .or_else(|_| env::current_dir().map(|p| p.to_string_lossy().to_string()))
                .ok();

            dispatch_db_command(cli.command, &pool, project_path.as_deref()).await
        }
    }
}

/// Dispatch commands that require database access
async fn dispatch_db_command(
    command: Command,
    pool: &sqlx::postgres::PgPool,
    project_path: Option<&str>,
) -> Result<serde_json::Value> {
    match command {
        Command::AddMemory {
            memory_type,
            content,
            tags,
            confidence,
            tier,
            source_session_id,
            source_turn_id,
            claude_session_id: _,
        } => {
            let tags_vec = parse_tags(&tags);
            let source_session = source_session_id
                .as_deref()
                .and_then(|s| Uuid::parse_str(s).ok());
            let source_turn = source_turn_id
                .as_deref()
                .and_then(|s| Uuid::parse_str(s).ok());

            let opts = AddMemoryOptions {
                memory_type,
                content,
                tags: tags_vec,
                confidence,
                tier: scope_to_tier(tier),
                project_path: project_path.map(|s| s.to_string()),
                source_session_id: source_session,
                source_turn_id: source_turn,
            };

            let result = add_memory(pool, opts).await?;
            match result {
                claude_hippocampus::commands::AddMemoryResult::Success(json) => Ok(json),
                claude_hippocampus::commands::AddMemoryResult::Duplicate(json) => Ok(json),
            }
        }

        Command::UpdateMemory { id, content, tier } => {
            let uuid = Uuid::parse_str(&id)?;
            update_memory(pool, uuid, &content, Some(scope_to_tier(tier)), project_path).await
        }

        Command::DeleteMemory { id, tier: _ } => {
            let uuid = Uuid::parse_str(&id)?;
            delete_memory(pool, uuid).await
        }

        Command::GetMemory { id } => {
            let uuid = Uuid::parse_str(&id)?;
            get_memory(pool, uuid).await
        }

        Command::SearchKeyword { query, tier, limit } => {
            let options = SearchOptions {
                query,
                tier,
                limit: limit as i32,
                project_path: project_path.map(|s| s.to_string()),
            };
            let result = search_keyword(pool, options).await?;
            Ok(serde_json::to_value(SuccessResponse::new(result))?)
        }

        Command::GetContext { limit } => {
            let result = get_context(pool, limit as i32, project_path).await?;
            Ok(serde_json::to_value(SuccessResponse::new(result))?)
        }

        Command::ListRecent { n, tier } => {
            let result = list_recent(pool, n as i32, tier, project_path).await?;
            Ok(serde_json::to_value(SuccessResponse::new(result))?)
        }

        Command::Consolidate { tier } => {
            consolidate(pool, scope_to_tier(tier), project_path).await
        }

        Command::Prune { days, tier } => {
            prune(pool, days as i32, scope_to_tier(tier), project_path).await
        }

        Command::SaveSessionSummary { summary } => {
            // Use empty session ID if not provided - the function will auto-detect
            let session_id = env::var("CLAUDE_SESSION_ID").unwrap_or_else(|_| String::new());
            let summary_json = serde_json::json!({ "summary": summary });
            save_session_summary(pool, &session_id, &summary_json).await
        }

        // These are handled in run() before this function is called
        Command::Logs { .. } | Command::ClearLogs => {
            unreachable!("Logs commands should be handled before database dispatch")
        }
    }
}

/// Convert Scope to Tier (Scope doesn't have Both, so we need this conversion)
fn scope_to_tier(scope: Scope) -> Tier {
    match scope {
        Scope::Project => Tier::Project,
        Scope::Global => Tier::Global,
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scope_to_tier_project() {
        assert_eq!(scope_to_tier(Scope::Project), Tier::Project);
    }

    #[test]
    fn test_scope_to_tier_global() {
        assert_eq!(scope_to_tier(Scope::Global), Tier::Global);
    }
}
