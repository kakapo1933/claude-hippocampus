use sqlx::postgres::PgPool;

use crate::db;
use crate::error::Result;
use crate::models::{ConsolidateData, PruneData, SaveSessionSummaryData, Scope, SuccessResponse, Tier};

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

/// Prune old low-confidence memories with no access
pub async fn prune(
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

    let pruned_ids = db::prune_old_memories(pool, days, scope_filter, project_path).await?;

    let response = SuccessResponse::new(PruneData {
        pruned: pruned_ids.len(),
        pruned_ids,
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

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

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
    fn test_prune_data_serialization() {
        let data = PruneData {
            pruned: 5,
            pruned_ids: vec![Uuid::new_v4()],
        };
        let response = SuccessResponse::new(data);
        let json = serde_json::to_value(&response).unwrap();

        assert_eq!(json["success"], true);
        assert_eq!(json["pruned"], 5);
        assert!(json["prunedIds"].is_array());
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
