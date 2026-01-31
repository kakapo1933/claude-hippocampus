use sqlx::postgres::PgPool;

use crate::db;
use crate::error::Result;
use crate::models::{
    ChainData, ConsolidateData, ListSupersededData, PruneDataResult, PurgeSupersededData,
    SaveSessionSummaryData, Scope, SuccessResponse, SupersededMemory, Tier, TieredPruneData,
};

/// Consolidate duplicate memories (remove exact duplicates)
pub async fn consolidate(
    pool: &PgPool,
    tier: Tier,
    project_path: Option<&str>,
) -> Result<serde_json::Value> {
    let scope_filter = match tier {
        Tier::Global => Some(Scope::Global),
        Tier::Project => Some(Scope::Project),
        Tier::Both => None,
    };

    let duplicate_ids = db::consolidate_duplicates(pool, scope_filter, project_path).await?;

    let response = SuccessResponse::new(ConsolidateData {
        removed: duplicate_ids.len(),
        duplicate_ids,
    });

    Ok(serde_json::to_value(response)?)
}

/// Prune old low-confidence memories with no access using tiered retention
/// - LOW confidence: pruned after `low_days` days with access_count=0
/// - MEDIUM confidence: pruned after `medium_days` days with access_count=0
/// - HIGH confidence: never pruned
pub async fn prune(
    pool: &PgPool,
    low_days: i32,
    medium_days: i32,
    tier: Tier,
    project_path: Option<&str>,
) -> Result<serde_json::Value> {
    let scope_filter = match tier {
        Tier::Global => Some(Scope::Global),
        Tier::Project => Some(Scope::Project),
        Tier::Both => None,
    };

    let (low_pruned_ids, medium_pruned_ids) =
        db::prune_old_memories_tiered(pool, low_days, medium_days, scope_filter, project_path)
            .await?;

    let total = low_pruned_ids.len() + medium_pruned_ids.len();

    let response = SuccessResponse::new(TieredPruneData {
        low_pruned: low_pruned_ids.len(),
        low_pruned_ids,
        medium_pruned: medium_pruned_ids.len(),
        medium_pruned_ids,
        total_pruned: total,
    });

    Ok(serde_json::to_value(response)?)
}

/// Save session summary to database
pub async fn save_session_summary(
    pool: &PgPool,
    claude_session_id: &str,
    summary: &serde_json::Value,
) -> Result<serde_json::Value> {
    let session_id = db::save_session_summary(pool, claude_session_id, summary).await?;

    let response = SuccessResponse::new(SaveSessionSummaryData { session_id });

    Ok(serde_json::to_value(response)?)
}

/// Show the supersession chain for a memory
pub async fn show_chain(pool: &PgPool, memory_id: uuid::Uuid) -> Result<serde_json::Value> {
    let chain = db::show_chain(pool, memory_id).await?;

    let response = SuccessResponse::new(ChainData {
        memory: chain.memory,
        predecessors: chain.predecessors,
        successors: chain.successors,
    });

    Ok(serde_json::to_value(response)?)
}

/// List superseded (inactive) memories
pub async fn list_superseded(
    pool: &PgPool,
    tier: Tier,
    limit: i64,
    project_path: Option<&str>,
) -> Result<serde_json::Value> {
    let entries = db::list_superseded(pool, tier, limit, project_path).await?;
    let count = entries.len();

    let response = SuccessResponse::new(ListSupersededData {
        entries: entries
            .into_iter()
            .map(|e| SupersededMemory {
                memory: e.memory,
                superseded_by_id: e.superseded_by_id,
                superseded_at: e.superseded_at,
            })
            .collect(),
        count,
    });

    Ok(serde_json::to_value(response)?)
}

/// Purge old superseded memories
pub async fn purge_superseded(
    pool: &PgPool,
    days: i32,
    tier: Tier,
    project_path: Option<&str>,
) -> Result<serde_json::Value> {
    let scope_filter = match tier {
        Tier::Global => Some(Scope::Global),
        Tier::Project => Some(Scope::Project),
        Tier::Both => None,
    };

    let purged_ids = db::purge_superseded(pool, days, scope_filter, project_path).await?;

    let response = SuccessResponse::new(PurgeSupersededData {
        purged: purged_ids.len(),
        purged_ids,
    });

    Ok(serde_json::to_value(response)?)
}

/// Prune lifecycle data (tool calls, turns, sessions)
pub async fn prune_data(
    pool: &PgPool,
    tool_calls_days: i64,
    turns_days: i64,
    sessions_days: i64,
    dry_run: bool,
) -> Result<serde_json::Value> {
    let result = db::prune_lifecycle_data(
        pool,
        tool_calls_days as i32,
        turns_days as i32,
        sessions_days as i32,
        dry_run,
    )
    .await?;

    let response = SuccessResponse::new(PruneDataResult {
        tool_calls_pruned: result.tool_calls_pruned,
        turns_pruned: result.turns_pruned,
        sessions_pruned: result.sessions_pruned,
        dry_run,
    });

    Ok(serde_json::to_value(response)?)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;
    use crate::models::{Confidence, MemoryType, MemorySummary, PruneData};

    #[test]
    fn test_tier_to_scope_filter_global() {
        let tier = Tier::Global;
        let scope = match tier {
            Tier::Global => Some(Scope::Global),
            Tier::Project => Some(Scope::Project),
            Tier::Both => None,
        };
        assert_eq!(scope, Some(Scope::Global));
    }

    #[test]
    fn test_tier_to_scope_filter_project() {
        let tier = Tier::Project;
        let scope = match tier {
            Tier::Global => Some(Scope::Global),
            Tier::Project => Some(Scope::Project),
            Tier::Both => None,
        };
        assert_eq!(scope, Some(Scope::Project));
    }

    #[test]
    fn test_tier_to_scope_filter_both() {
        let tier = Tier::Both;
        let scope: Option<Scope> = match tier {
            Tier::Global => Some(Scope::Global),
            Tier::Project => Some(Scope::Project),
            Tier::Both => None,
        };
        assert!(scope.is_none()); // Both means no filter
    }

    #[test]
    fn test_consolidate_data_serialization() {
        let data = ConsolidateData {
            removed: 2,
            duplicate_ids: vec![Uuid::new_v4(), Uuid::new_v4()],
        };
        let response = SuccessResponse::new(data);
        let json = serde_json::to_value(&response).unwrap();

        assert_eq!(json["success"], true);
        assert_eq!(json["removed"], 2);
        assert!(json["duplicateIds"].is_array());
    }

    #[test]
    fn test_tiered_prune_data_serialization() {
        let data = TieredPruneData {
            low_pruned: 3,
            low_pruned_ids: vec![Uuid::new_v4()],
            medium_pruned: 2,
            medium_pruned_ids: vec![Uuid::new_v4()],
            total_pruned: 5,
        };
        let response = SuccessResponse::new(data);
        let json = serde_json::to_value(&response).unwrap();

        assert_eq!(json["success"], true);
        assert_eq!(json["lowPruned"], 3);
        assert_eq!(json["mediumPruned"], 2);
        assert_eq!(json["totalPruned"], 5);
    }

    #[test]
    fn test_chain_data_serialization() {
        let summary = MemorySummary {
            id: Uuid::new_v4(),
            memory_type: MemoryType::Learning,
            tier: Scope::Project,
            summary: "Test".to_string(),
            tags: vec![],
            confidence: Confidence::High,
            created: Utc::now(),
            access_count: 0,
            superseded_by: None,
            superseded_at: None,
            is_active: true,
        };
        let data = ChainData {
            memory: summary.clone(),
            predecessors: vec![],
            successors: vec![summary],
        };
        let response = SuccessResponse::new(data);
        let json = serde_json::to_value(&response).unwrap();

        assert_eq!(json["success"], true);
        assert!(json["memory"].is_object());
        assert!(json["predecessors"].is_array());
        assert!(json["successors"].is_array());
    }

    #[test]
    fn test_prune_data_result_serialization() {
        let data = PruneDataResult {
            tool_calls_pruned: 10,
            turns_pruned: 5,
            sessions_pruned: 2,
            dry_run: true,
        };
        let response = SuccessResponse::new(data);
        let json = serde_json::to_value(&response).unwrap();

        assert_eq!(json["success"], true);
        assert_eq!(json["toolCallsPruned"], 10);
        assert_eq!(json["turnsPruned"], 5);
        assert_eq!(json["sessionsPruned"], 2);
        assert_eq!(json["dryRun"], true);
    }

    #[test]
    fn test_save_session_summary_data_serialization() {
        let data = SaveSessionSummaryData {
            session_id: Uuid::new_v4(),
        };
        let response = SuccessResponse::new(data);
        let json = serde_json::to_value(&response).unwrap();

        assert_eq!(json["success"], true);
        assert!(json["sessionId"].is_string());
    }

    // Note: Full integration tests require a database connection
    // and are placed in tests/integration/
}
