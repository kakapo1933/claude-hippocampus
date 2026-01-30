//! Claude-Hippocampus: Memory System CLI
//!
//! Main entry point for the CLI application.
//! Dispatches commands to the appropriate handlers and outputs JSON results.

use clap::Parser;
use std::env;
use uuid::Uuid;

use std::io::{self, BufRead};

use claude_hippocampus::{
    clear_logs, parse_tags, read_logs, Cli, Command, DbConfig, HookType, Result,
    HookInput, handle_session_start, handle_user_prompt_submit, handle_stop, handle_session_end,
};
use claude_hippocampus::commands::{
    add_memory, consolidate, delete_memory, get_context, get_memory, get_stats, list_recent, prune,
    save_session_summary, search_by_type, search_keyword, update_memory, AddMemoryOptions,
    SearchByTypeOptions, SearchOptions, StatsOptions,
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

        Command::Stats { tier } => {
            // Stats requires database connection
            let config = DbConfig::load()?;
            let pool = create_pool(&config).await?;
            let project_path = env::var("PROJECT_PATH")
                .or_else(|_| env::current_dir().map(|p| p.to_string_lossy().to_string()))
                .ok();

            let options = StatsOptions {
                tier,
                project_path,
            };
            let result = get_stats(&pool, options).await?;
            Ok(serde_json::to_value(SuccessResponse::new(result))?)
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

        Command::SearchByType {
            memory_type,
            query,
            tier,
            limit,
        } => {
            let options = SearchByTypeOptions {
                memory_type,
                query,
                tier,
                limit: limit as i32,
                project_path: project_path.map(|s| s.to_string()),
            };
            let result = search_by_type(pool, options).await?;
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

        // Session commands
        Command::CreateSession {
            claude_session_id,
            project_path: project_path_arg,
        } => {
            use claude_hippocampus::db::queries::create_session;
            use claude_hippocampus::git::get_git_status;

            // Get project path (from arg, env, or cwd)
            let path = project_path_arg
                .or_else(|| project_path.map(|p| p.to_string()))
                .or_else(|| env::current_dir().ok().map(|p| p.to_string_lossy().to_string()));

            // Capture git status if we have a path
            let git_status = path.as_ref()
                .and_then(|p| get_git_status(p).ok())
                .flatten();

            let session = create_session(pool, &claude_session_id, path.as_deref(), git_status.as_ref()).await?;
            Ok(serde_json::to_value(SuccessResponse::new(session))?)
        }

        Command::GetSession { id } => {
            use claude_hippocampus::db::queries::{find_session_by_id, find_session_by_claude_id};

            // Try parsing as UUID first, fall back to claude_session_id lookup
            let session = if let Ok(uuid) = Uuid::parse_str(&id) {
                find_session_by_id(pool, uuid).await?
            } else {
                find_session_by_claude_id(pool, &id).await?
            };

            match session {
                Some(s) => Ok(serde_json::to_value(SuccessResponse::new(s))?),
                None => Err(claude_hippocampus::error::HippocampusError::SessionNotFound(id)),
            }
        }

        Command::EndSession { id, summary } => {
            use claude_hippocampus::db::queries::end_session;

            let session = end_session(pool, &id, summary.as_deref()).await?;
            Ok(serde_json::to_value(SuccessResponse::new(session))?)
        }

        // Turn commands
        Command::CreateTurn {
            session_id,
            prompt,
            model,
        } => {
            use claude_hippocampus::db::queries::{
                create_turn, find_session_by_claude_id, get_next_turn_number,
            };

            // Find session by claude_session_id
            let session = find_session_by_claude_id(pool, &session_id).await?;
            let session = session.ok_or_else(|| {
                claude_hippocampus::error::HippocampusError::SessionNotFound(session_id.clone())
            })?;

            // Get next turn number
            let turn_number = get_next_turn_number(pool, session.id).await?;

            // Create turn
            let turn = create_turn(pool, session.id, turn_number, &prompt, model.as_deref()).await?;
            Ok(serde_json::to_value(SuccessResponse::new(turn))?)
        }

        Command::UpdateTurn {
            turn_id,
            response,
            input_tokens,
            output_tokens,
        } => {
            use claude_hippocampus::db::queries::update_turn;

            let uuid = Uuid::parse_str(&turn_id)?;
            let turn = update_turn(pool, uuid, &response, input_tokens, output_tokens).await?;
            Ok(serde_json::to_value(SuccessResponse::new(turn))?)
        }

        // Hook commands
        Command::Hook { hook_type } => {
            use claude_hippocampus::hooks::{handle_post_tool_use, PostToolUseInput};

            // PostToolUse has different input format, handle separately
            if hook_type == HookType::PostToolUse {
                let raw_input = read_raw_stdin()?;
                let input: PostToolUseInput = serde_json::from_str(&raw_input)
                    .unwrap_or_else(|_| PostToolUseInput {
                        tool_name: None,
                        tool_input: None,
                        tool_response: None,
                        session_id: None,
                    });
                let output = handle_post_tool_use(pool, &input).await?;
                return Ok(serde_json::to_value(&output)?);
            }

            // Read JSON input from stdin for standard hooks
            let input = read_hook_input()?;

            let output = match hook_type {
                HookType::SessionStart => handle_session_start(pool, &input).await?,
                HookType::UserPromptSubmit => handle_user_prompt_submit(pool, &input).await?,
                HookType::Stop => handle_stop(&input).await?,
                HookType::SessionEnd => handle_session_end(pool, &input).await?,
                HookType::PostToolUse => unreachable!("Handled above"),
            };

            Ok(serde_json::to_value(&output)?)
        }

        // These are handled in run() before this function is called
        Command::Logs { .. } | Command::ClearLogs | Command::Stats { .. } => {
            unreachable!("These commands are handled in run() before database dispatch")
        }
    }
}

/// Read raw stdin as string
fn read_raw_stdin() -> Result<String> {
    let stdin = io::stdin();
    let mut input = String::new();

    for line in stdin.lock().lines() {
        match line {
            Ok(l) => input.push_str(&l),
            Err(_) => break,
        }
    }

    Ok(input)
}

/// Read hook input from stdin
fn read_hook_input() -> Result<HookInput> {
    let input = read_raw_stdin()?;

    if input.is_empty() {
        // Return empty input if no stdin
        Ok(HookInput {
            session_id: None,
            prompt: None,
            transcript_path: None,
            cwd: None,
            permission_mode: None,
            hook_event_name: None,
        })
    } else {
        serde_json::from_str(&input).map_err(|e| {
            claude_hippocampus::error::HippocampusError::Config(format!(
                "Failed to parse hook input: {}",
                e
            ))
        })
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
