//! Search commands: searchKeyword, getContext, listRecent
//!
//! These commands query the memories database and return formatted results.

use serde::Serialize;
use sqlx::postgres::PgPool;

use crate::db::queries;
use crate::error::Result;
use crate::models::{Memory, MemorySummary, MemoryType, Scope, Tier};

// ============================================================================
// Search Options
// ============================================================================

/// Options for keyword search
#[derive(Debug, Clone)]
pub struct SearchOptions {
    /// Search query string
    pub query: String,
    /// Tier filter (project, global, or both)
    pub tier: Tier,
    /// Maximum number of results
    pub limit: i32,
    /// Project path for project-scoped queries
    pub project_path: Option<String>,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            query: String::new(),
            tier: Tier::Both,
            limit: 30,
            project_path: None,
        }
    }
}

/// Options for search by type
#[derive(Debug, Clone)]
pub struct SearchByTypeOptions {
    /// Memory type to filter by
    pub memory_type: MemoryType,
    /// Optional keyword filter
    pub query: Option<String>,
    /// Tier filter (project, global, or both)
    pub tier: Tier,
    /// Maximum number of results
    pub limit: i32,
    /// Project path for project-scoped queries
    pub project_path: Option<String>,
}

// ============================================================================
// Search Results
// ============================================================================

/// Result of a keyword search
#[derive(Debug, Serialize)]
pub struct SearchResult {
    /// Matched memories (with full content)
    pub results: Vec<MemorySearchItem>,
    /// Number of results
    pub count: usize,
}

/// A single search result item (includes full content unlike MemorySummary)
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemorySearchItem {
    pub id: uuid::Uuid,
    #[serde(rename = "type")]
    pub memory_type: crate::models::MemoryType,
    pub tier: Scope,
    pub summary: String,
    pub content: String,
    pub tags: Vec<String>,
    pub confidence: crate::models::Confidence,
    pub created: chrono::DateTime<chrono::Utc>,
    pub accessed: Option<chrono::DateTime<chrono::Utc>>,
    pub access_count: i32,
}

impl From<Memory> for MemorySearchItem {
    fn from(m: Memory) -> Self {
        let summary = if m.content.len() > 100 {
            format!("{}...", &m.content[..97])
        } else {
            m.content.clone()
        };

        Self {
            id: m.id,
            memory_type: m.memory_type,
            tier: m.scope,
            summary,
            content: m.content,
            tags: m.tags,
            confidence: m.confidence,
            created: m.created_at,
            accessed: m.accessed_at,
            access_count: m.access_count,
        }
    }
}

/// Result of getContext command
#[derive(Debug, Serialize)]
pub struct ContextResult {
    /// Formatted markdown context block
    pub context: String,
    /// Number of entries
    pub count: usize,
    /// Summary entries
    pub entries: Vec<MemorySummary>,
}

/// Result of listRecent command
#[derive(Debug, Serialize)]
pub struct ListRecentResult {
    /// Recent memory summaries
    pub entries: Vec<MemorySummary>,
    /// Total count of matching memories
    pub total: usize,
}

// ============================================================================
// Commands
// ============================================================================

/// Search memories by keyword (content or tags).
///
/// Searches both content (ILIKE) and tags for matches.
/// Results are ordered by confidence (high → medium → low), then by recency.
pub async fn search_keyword(pool: &PgPool, options: SearchOptions) -> Result<SearchResult> {
    let (scope_filter, include_both) = tier_to_scope_filter(options.tier);

    let memories = queries::search_keyword(
        pool,
        &options.query,
        scope_filter,
        options.project_path.as_deref(),
        include_both,
        options.limit,
    )
    .await?;

    // Mark returned memories as accessed
    if !memories.is_empty() {
        let ids: Vec<uuid::Uuid> = memories.iter().map(|m| m.id).collect();
        queries::mark_memories_accessed(pool, &ids).await?;
    }

    let results: Vec<MemorySearchItem> = memories.into_iter().map(Into::into).collect();
    let count = results.len();

    Ok(SearchResult { results, count })
}

/// Search memories by type (with optional keyword filter).
///
/// Filters by memory type first, then optionally by keyword.
/// Results are ordered by confidence (high → medium → low), then by recency.
pub async fn search_by_type(pool: &PgPool, options: SearchByTypeOptions) -> Result<SearchResult> {
    let (scope_filter, include_both) = tier_to_scope_filter(options.tier);

    let memories = queries::search_by_type(
        pool,
        options.memory_type,
        options.query.as_deref(),
        scope_filter,
        options.project_path.as_deref(),
        include_both,
        options.limit,
    )
    .await?;

    // Mark returned memories as accessed
    if !memories.is_empty() {
        let ids: Vec<uuid::Uuid> = memories.iter().map(|m| m.id).collect();
        queries::mark_memories_accessed(pool, &ids).await?;
    }

    let results: Vec<MemorySearchItem> = memories.into_iter().map(Into::into).collect();
    let count = results.len();

    Ok(SearchResult { results, count })
}

/// Get context block for injection (top memories by relevance).
///
/// Returns a formatted markdown block suitable for injection into prompts,
/// along with the raw entry data.
pub async fn get_context(
    pool: &PgPool,
    limit: i32,
    project_path: Option<&str>,
) -> Result<ContextResult> {
    let memories = queries::get_context_memories(pool, project_path, limit).await?;

    // Mark returned memories as accessed
    if !memories.is_empty() {
        let ids: Vec<uuid::Uuid> = memories.iter().map(|m| m.id).collect();
        queries::mark_memories_accessed(pool, &ids).await?;
    }

    let entries: Vec<MemorySummary> = memories.iter().map(|m| m.to_summary()).collect();

    // Format as markdown context block
    let context = format_context_block(&entries);

    Ok(ContextResult {
        context,
        count: entries.len(),
        entries,
    })
}

/// List recent memories.
///
/// Returns memories sorted by creation date (newest first).
pub async fn list_recent(
    pool: &PgPool,
    limit: i32,
    tier: Tier,
    project_path: Option<&str>,
) -> Result<ListRecentResult> {
    let (scope_filter, include_both) = tier_to_scope_filter(tier);

    let (memories, total) =
        queries::list_recent(pool, scope_filter, project_path, include_both, limit).await?;

    let entries: Vec<MemorySummary> = memories.iter().map(|m| m.to_summary()).collect();

    Ok(ListRecentResult {
        entries,
        total: total as usize,
    })
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Convert Tier to (Option<Scope>, include_both) for query building
fn tier_to_scope_filter(tier: Tier) -> (Option<Scope>, bool) {
    match tier {
        Tier::Project => (Some(Scope::Project), false),
        Tier::Global => (Some(Scope::Global), false),
        Tier::Both => (None, true),
    }
}

/// Format entries as a markdown context block
fn format_context_block(entries: &[MemorySummary]) -> String {
    let mut context = String::from("## Memory Context\n\n");

    if entries.is_empty() {
        context.push_str("No memories loaded.\n");
    } else {
        for entry in entries {
            let symbol = entry.confidence.symbol();
            let type_str = entry.memory_type.as_str();
            context.push_str(&format!("- {} **{}**: {}\n", symbol, type_str, entry.summary));
        }
    }

    context
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Confidence, MemoryType};
    use chrono::Utc;
    use uuid::Uuid;

    // -------------------------------------------------------------------------
    // SearchOptions tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_search_options_default() {
        let options = SearchOptions::default();
        assert_eq!(options.query, "");
        assert_eq!(options.tier, Tier::Both);
        assert_eq!(options.limit, 30);
        assert_eq!(options.project_path, None);
    }

    #[test]
    fn test_search_options_custom() {
        let options = SearchOptions {
            query: "test query".to_string(),
            tier: Tier::Project,
            limit: 10,
            project_path: Some("/test/path".to_string()),
        };

        assert_eq!(options.query, "test query");
        assert_eq!(options.tier, Tier::Project);
        assert_eq!(options.limit, 10);
        assert_eq!(options.project_path, Some("/test/path".to_string()));
    }

    // -------------------------------------------------------------------------
    // MemorySearchItem tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_memory_search_item_from_memory_short_content() {
        let memory = Memory {
            id: Uuid::new_v4(),
            memory_type: MemoryType::Learning,
            scope: Scope::Project,
            project_path: Some("/test".to_string()),
            content: "Short content".to_string(),
            tags: vec!["test".to_string()],
            confidence: Confidence::High,
            source_session_id: None,
            source_turn_id: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            accessed_at: None,
            access_count: 5,
            superseded_by: None,
            superseded_at: None,
            is_active: true,
        };

        let item: MemorySearchItem = memory.into();

        assert_eq!(item.summary, "Short content");
        assert_eq!(item.content, "Short content");
        assert_eq!(item.memory_type, MemoryType::Learning);
        assert_eq!(item.confidence, Confidence::High);
        assert_eq!(item.access_count, 5);
    }

    #[test]
    fn test_memory_search_item_from_memory_long_content() {
        let long_content = "x".repeat(150);
        let memory = Memory {
            id: Uuid::new_v4(),
            memory_type: MemoryType::Gotcha,
            scope: Scope::Global,
            project_path: None,
            content: long_content.clone(),
            tags: vec![],
            confidence: Confidence::Medium,
            source_session_id: None,
            source_turn_id: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            accessed_at: Some(Utc::now()),
            access_count: 0,
            superseded_by: None,
            superseded_at: None,
            is_active: true,
        };

        let item: MemorySearchItem = memory.into();

        assert_eq!(item.summary.len(), 100); // 97 + "..."
        assert!(item.summary.ends_with("..."));
        assert_eq!(item.content, long_content);
        assert!(item.accessed.is_some());
    }

    // -------------------------------------------------------------------------
    // tier_to_scope_filter tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_tier_to_scope_filter_project() {
        let (scope, both) = tier_to_scope_filter(Tier::Project);
        assert_eq!(scope, Some(Scope::Project));
        assert!(!both);
    }

    #[test]
    fn test_tier_to_scope_filter_global() {
        let (scope, both) = tier_to_scope_filter(Tier::Global);
        assert_eq!(scope, Some(Scope::Global));
        assert!(!both);
    }

    #[test]
    fn test_tier_to_scope_filter_both() {
        let (scope, both) = tier_to_scope_filter(Tier::Both);
        assert_eq!(scope, None);
        assert!(both);
    }

    // -------------------------------------------------------------------------
    // format_context_block tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_format_context_block_empty() {
        let entries: Vec<MemorySummary> = vec![];
        let context = format_context_block(&entries);

        assert!(context.contains("## Memory Context"));
        assert!(context.contains("No memories loaded."));
    }

    #[test]
    fn test_format_context_block_with_entries() {
        let entries = vec![
            MemorySummary {
                id: Uuid::new_v4(),
                memory_type: MemoryType::Learning,
                tier: Scope::Global,
                summary: "Test learning".to_string(),
                tags: vec![],
                confidence: Confidence::High,
                created: Utc::now(),
                access_count: 0,
                superseded_by: None,
                superseded_at: None,
                is_active: true,
            },
            MemorySummary {
                id: Uuid::new_v4(),
                memory_type: MemoryType::Gotcha,
                tier: Scope::Project,
                summary: "Test gotcha".to_string(),
                tags: vec![],
                confidence: Confidence::Medium,
                created: Utc::now(),
                access_count: 0,
                superseded_by: None,
                superseded_at: None,
                is_active: true,
            },
            MemorySummary {
                id: Uuid::new_v4(),
                memory_type: MemoryType::Api,
                tier: Scope::Project,
                summary: "Test api".to_string(),
                tags: vec![],
                confidence: Confidence::Low,
                created: Utc::now(),
                access_count: 0,
                superseded_by: None,
                superseded_at: None,
                is_active: true,
            },
        ];

        let context = format_context_block(&entries);

        assert!(context.contains("## Memory Context"));
        assert!(!context.contains("No memories loaded."));

        // Check symbols
        assert!(context.contains("★ **learning**"));
        assert!(context.contains("◐ **gotcha**"));
        assert!(context.contains("○ **api**"));

        // Check content
        assert!(context.contains("Test learning"));
        assert!(context.contains("Test gotcha"));
        assert!(context.contains("Test api"));
    }

    #[test]
    fn test_format_context_block_preserves_order() {
        let entries = vec![
            MemorySummary {
                id: Uuid::new_v4(),
                memory_type: MemoryType::Convention,
                tier: Scope::Project,
                summary: "First".to_string(),
                tags: vec![],
                confidence: Confidence::High,
                created: Utc::now(),
                access_count: 0,
                superseded_by: None,
                superseded_at: None,
                is_active: true,
            },
            MemorySummary {
                id: Uuid::new_v4(),
                memory_type: MemoryType::Architecture,
                tier: Scope::Global,
                summary: "Second".to_string(),
                tags: vec![],
                confidence: Confidence::High,
                created: Utc::now(),
                access_count: 0,
                superseded_by: None,
                superseded_at: None,
                is_active: true,
            },
        ];

        let context = format_context_block(&entries);
        let first_pos = context.find("First").unwrap();
        let second_pos = context.find("Second").unwrap();

        assert!(first_pos < second_pos);
    }

    // -------------------------------------------------------------------------
    // SearchResult tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_search_result_empty() {
        let result = SearchResult {
            results: vec![],
            count: 0,
        };

        assert!(result.results.is_empty());
        assert_eq!(result.count, 0);
    }

    // -------------------------------------------------------------------------
    // SearchByTypeOptions tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_search_by_type_options_all_fields() {
        let options = SearchByTypeOptions {
            memory_type: MemoryType::Gotcha,
            query: Some("test query".to_string()),
            tier: Tier::Project,
            limit: 10,
            project_path: Some("/test/path".to_string()),
        };

        assert_eq!(options.memory_type, MemoryType::Gotcha);
        assert_eq!(options.query, Some("test query".to_string()));
        assert_eq!(options.tier, Tier::Project);
        assert_eq!(options.limit, 10);
        assert_eq!(options.project_path, Some("/test/path".to_string()));
    }

    #[test]
    fn test_search_by_type_options_no_query() {
        let options = SearchByTypeOptions {
            memory_type: MemoryType::Learning,
            query: None,
            tier: Tier::Both,
            limit: 30,
            project_path: None,
        };

        assert_eq!(options.memory_type, MemoryType::Learning);
        assert!(options.query.is_none());
        assert_eq!(options.tier, Tier::Both);
        assert_eq!(options.limit, 30);
        assert!(options.project_path.is_none());
    }

    #[test]
    fn test_search_by_type_options_all_memory_types() {
        for memory_type in [
            MemoryType::Convention,
            MemoryType::Architecture,
            MemoryType::Gotcha,
            MemoryType::Api,
            MemoryType::Learning,
            MemoryType::Preference,
        ] {
            let options = SearchByTypeOptions {
                memory_type,
                query: None,
                tier: Tier::Both,
                limit: 10,
                project_path: None,
            };
            // Just ensure we can create options for all types
            assert_eq!(options.memory_type, memory_type);
        }
    }

    // -------------------------------------------------------------------------
    // ContextResult tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_context_result_structure() {
        let result = ContextResult {
            context: "## Memory Context\n\nNo memories loaded.\n".to_string(),
            count: 0,
            entries: vec![],
        };

        assert!(result.context.contains("Memory Context"));
        assert_eq!(result.count, 0);
        assert!(result.entries.is_empty());
    }

    // -------------------------------------------------------------------------
    // ListRecentResult tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_list_recent_result_structure() {
        let result = ListRecentResult {
            entries: vec![],
            total: 100,
        };

        assert!(result.entries.is_empty());
        assert_eq!(result.total, 100);
    }

    // -------------------------------------------------------------------------
    // JSON Serialization tests (Node.js compatibility)
    // -------------------------------------------------------------------------

    #[test]
    fn test_memory_search_item_json_serialization() {
        let item = MemorySearchItem {
            id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap(),
            memory_type: MemoryType::Learning,
            tier: Scope::Project,
            summary: "Test summary".to_string(),
            content: "Test content".to_string(),
            tags: vec!["tag1".to_string(), "tag2".to_string()],
            confidence: Confidence::High,
            created: chrono::DateTime::parse_from_rfc3339("2024-01-15T10:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
            accessed: None,
            access_count: 5,
        };

        let json = serde_json::to_string(&item).unwrap();

        // Verify camelCase field names (matching Node.js output)
        assert!(json.contains("\"type\":\"learning\"")); // renamed from memoryType
        assert!(json.contains("\"accessCount\":5"));

        // Verify field presence
        assert!(json.contains("\"id\":"));
        assert!(json.contains("\"tier\":\"project\""));
        assert!(json.contains("\"summary\":"));
        assert!(json.contains("\"content\":"));
        assert!(json.contains("\"tags\":"));
        assert!(json.contains("\"confidence\":\"high\""));
    }

    #[test]
    fn test_search_result_json_serialization() {
        let result = SearchResult {
            results: vec![],
            count: 0,
        };

        let json = serde_json::to_string(&result).unwrap();

        assert!(json.contains("\"results\":[]"));
        assert!(json.contains("\"count\":0"));
    }

    #[test]
    fn test_context_result_json_serialization() {
        let result = ContextResult {
            context: "## Memory Context\n\n- ★ **learning**: Test".to_string(),
            count: 1,
            entries: vec![],
        };

        let json = serde_json::to_string(&result).unwrap();

        assert!(json.contains("\"context\":"));
        assert!(json.contains("\"count\":1"));
        assert!(json.contains("\"entries\":[]"));
    }
}
