use serde::Serialize;
use uuid::Uuid;

use super::memory::MemorySummary;

// ============================================================================
// Base Response Types
// ============================================================================

/// Wrapper for successful responses with data
#[derive(Debug, Serialize)]
pub struct SuccessResponse<T: Serialize> {
    pub success: bool,
    #[serde(flatten)]
    pub data: T,
}

impl<T: Serialize> SuccessResponse<T> {
    pub fn new(data: T) -> Self {
        Self {
            success: true,
            data,
        }
    }
}

/// Error response format
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub success: bool,
    pub error: String,
}

impl ErrorResponse {
    pub fn new(error: impl Into<String>) -> Self {
        Self {
            success: false,
            error: error.into(),
        }
    }
}

// ============================================================================
// Memory Operation Responses
// ============================================================================

/// Response for successful memory creation
#[derive(Debug, Serialize)]
pub struct AddMemoryData {
    pub id: Uuid,
}

/// Response when duplicate memory is detected
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateResponse {
    pub success: bool,
    pub duplicate: bool,
    pub reason: String,
    pub existing_id: Uuid,
    pub existing_tier: String,
    pub existing_summary: String,
    pub message: String,
}

impl DuplicateResponse {
    pub fn new(
        existing_id: Uuid,
        existing_tier: &str,
        existing_summary: &str,
    ) -> Self {
        Self {
            success: false,
            duplicate: true,
            reason: "Duplicate memory detected (matching first 100 chars)".to_string(),
            existing_id,
            existing_tier: existing_tier.to_string(),
            existing_summary: existing_summary.to_string(),
            message: format!(
                "Memory with similar content already exists (id: {})",
                existing_id
            ),
        }
    }
}

/// Response for memory update
#[derive(Debug, Serialize)]
pub struct UpdateMemoryData {
    pub id: Uuid,
}

/// Response for memory deletion
#[derive(Debug, Serialize)]
pub struct DeleteMemoryData {
    pub deleted: Uuid,
}

/// Response for single memory retrieval
#[derive(Debug, Serialize)]
pub struct GetMemoryData {
    pub memory: MemorySummary,
}

// ============================================================================
// Search Responses
// ============================================================================

/// Response for keyword search
#[derive(Debug, Serialize)]
pub struct SearchResultData {
    pub results: Vec<MemorySummary>,
    pub count: usize,
}

/// Response for context retrieval
#[derive(Debug, Serialize)]
pub struct ContextData {
    pub context: String,
    pub count: usize,
    pub entries: Vec<MemorySummary>,
}

/// Response for listing recent memories
#[derive(Debug, Serialize)]
pub struct ListRecentData {
    pub entries: Vec<MemorySummary>,
    pub total: usize,
}

// ============================================================================
// Maintenance Responses
// ============================================================================

/// Response for consolidate operation
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConsolidateData {
    pub removed: usize,
    pub duplicate_ids: Vec<Uuid>,
}

/// Response for prune operation
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PruneData {
    pub pruned: usize,
    pub pruned_ids: Vec<Uuid>,
}

/// Response for tiered prune operation (low/medium confidence)
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TieredPruneData {
    pub low_pruned: usize,
    pub low_pruned_ids: Vec<Uuid>,
    pub medium_pruned: usize,
    pub medium_pruned_ids: Vec<Uuid>,
    pub total_pruned: usize,
}

/// Response for session summary save
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveSessionSummaryData {
    pub session_id: Uuid,
}

// ============================================================================
// Log Responses
// ============================================================================

/// Single log entry
#[derive(Debug, Serialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub operation: String,
    #[serde(flatten)]
    pub details: serde_json::Value,
}

/// Response for reading logs
#[derive(Debug, Serialize)]
pub struct LogsData {
    pub entries: Vec<LogEntry>,
    pub count: usize,
    pub total: usize,
}

/// Response for clearing logs
#[derive(Debug, Serialize)]
pub struct ClearLogsData {
    pub cleared: bool,
}

// ============================================================================
// Supersession Responses
// ============================================================================

use chrono::{DateTime, Utc};

/// Data for showing a memory's supersession chain
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChainData {
    /// The memory itself
    pub memory: MemorySummary,
    /// Memories that this one superseded (predecessors)
    pub predecessors: Vec<MemorySummary>,
    /// Memories that superseded this one (successors)
    pub successors: Vec<MemorySummary>,
}

/// A superseded memory with its replacement info
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SupersededMemory {
    pub memory: MemorySummary,
    pub superseded_by_id: Uuid,
    pub superseded_at: DateTime<Utc>,
}

/// Response for listing superseded memories
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListSupersededData {
    pub entries: Vec<SupersededMemory>,
    pub count: usize,
}

/// Response for purging superseded memories
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PurgeSupersededData {
    pub purged: usize,
    pub purged_ids: Vec<Uuid>,
}

/// Response for lifecycle data pruning
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PruneDataResult {
    pub tool_calls_pruned: usize,
    pub turns_pruned: usize,
    pub sessions_pruned: usize,
    pub dry_run: bool,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use crate::models::memory::{Confidence, MemoryType, Scope};

    #[test]
    fn test_success_response_serialization() {
        let data = AddMemoryData {
            id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap(),
        };
        let response = SuccessResponse::new(data);

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"id\":\"550e8400-e29b-41d4-a716-446655440000\""));
    }

    #[test]
    fn test_error_response_serialization() {
        let response = ErrorResponse::new("Something went wrong");

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"success\":false"));
        assert!(json.contains("\"error\":\"Something went wrong\""));
    }

    #[test]
    fn test_duplicate_response_serialization() {
        let response = DuplicateResponse::new(
            Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap(),
            "project",
            "Some existing memory content...",
        );

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"success\":false"));
        assert!(json.contains("\"duplicate\":true"));
        assert!(json.contains("\"existingId\"")); // camelCase
        assert!(json.contains("\"existingTier\":\"project\""));
    }

    #[test]
    fn test_search_result_data_serialization() {
        let summary = MemorySummary {
            id: Uuid::new_v4(),
            memory_type: MemoryType::Learning,
            tier: Scope::Global,
            summary: "Test summary".to_string(),
            tags: vec!["test".to_string()],
            confidence: Confidence::High,
            created: Utc::now(),
            access_count: 5,
            superseded_by: None,
            superseded_at: None,
            is_active: true,
        };

        let data = SearchResultData {
            results: vec![summary],
            count: 1,
        };
        let response = SuccessResponse::new(data);

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"count\":1"));
        assert!(json.contains("\"results\""));
    }

    #[test]
    fn test_consolidate_data_serialization() {
        let data = ConsolidateData {
            removed: 3,
            duplicate_ids: vec![Uuid::new_v4(), Uuid::new_v4()],
        };
        let response = SuccessResponse::new(data);

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"removed\":3"));
        assert!(json.contains("\"duplicateIds\"")); // camelCase
    }

    #[test]
    fn test_prune_data_serialization() {
        let data = PruneData {
            pruned: 5,
            pruned_ids: vec![Uuid::new_v4()],
        };
        let response = SuccessResponse::new(data);

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"pruned\":5"));
        assert!(json.contains("\"prunedIds\"")); // camelCase
    }

    #[test]
    fn test_context_data_has_correct_fields() {
        let data = ContextData {
            context: "## Memory Context\n\n- ★ learning: Test".to_string(),
            count: 1,
            entries: vec![],
        };
        let response = SuccessResponse::new(data);

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"context\""));
        assert!(json.contains("\"count\":1"));
        assert!(json.contains("\"entries\""));
    }
}
