use sqlx::postgres::PgPool;
use uuid::Uuid;

use crate::db;
use crate::error::Result;
use crate::models::{
    AddMemoryData, Confidence, DeleteMemoryData, DuplicateResponse, ErrorResponse,
    GetMemoryData, MemoryType, Scope, SuccessResponse, Tier, UpdateMemoryData,
};

/// Options for adding a memory
pub struct AddMemoryOptions {
    pub memory_type: MemoryType,
    pub content: String,
    pub tags: Vec<String>,
    pub confidence: Confidence,
    pub tier: Tier,
    pub project_path: Option<String>,
    pub source_session_id: Option<Uuid>,
    pub source_turn_id: Option<Uuid>,
}

/// Result of add_memory operation
pub enum AddMemoryResult {
    Success(serde_json::Value),
    Duplicate(serde_json::Value),
}

/// Add a new memory with duplicate detection
pub async fn add_memory(pool: &PgPool, opts: AddMemoryOptions) -> Result<AddMemoryResult> {
    // Check for duplicates
    if let Some(dup) = db::find_duplicate(pool, opts.memory_type, &opts.content).await? {
        let response = DuplicateResponse::new(
            dup.id,
            &dup.scope,
            &dup.summary,
        );
        return Ok(AddMemoryResult::Duplicate(serde_json::to_value(response)?));
    }

    // Determine scope from tier
    let scope = match opts.tier {
        Tier::Global => Scope::Global,
        Tier::Project | Tier::Both => Scope::Project,
    };

    // Only include project_path for project-scoped memories
    let project_path = if scope == Scope::Project {
        opts.project_path.as_deref()
    } else {
        None
    };

    // Insert the memory
    let id = db::insert_memory(
        pool,
        opts.memory_type,
        scope,
        project_path,
        &opts.content,
        &opts.tags,
        opts.confidence,
        opts.source_session_id,
        opts.source_turn_id,
    )
    .await?;

    let response = SuccessResponse::new(AddMemoryData { id });
    Ok(AddMemoryResult::Success(serde_json::to_value(response)?))
}

/// Update an existing memory's content
pub async fn update_memory(
    pool: &PgPool,
    id: Uuid,
    content: &str,
    tier: Option<Tier>,
    project_path: Option<&str>,
) -> Result<serde_json::Value> {
    let scope = tier.map(|t| match t {
        Tier::Global => Scope::Global,
        Tier::Project | Tier::Both => Scope::Project,
    });

    let updated = db::update_memory(pool, id, content, scope, project_path).await?;

    if updated {
        let response = SuccessResponse::new(UpdateMemoryData { id });
        Ok(serde_json::to_value(response)?)
    } else {
        let response = ErrorResponse::new(format!("Memory not found: {}", id));
        Ok(serde_json::to_value(response)?)
    }
}

/// Delete a memory by ID
pub async fn delete_memory(pool: &PgPool, id: Uuid) -> Result<serde_json::Value> {
    let deleted = db::delete_memory(pool, id).await?;

    if deleted {
        let response = SuccessResponse::new(DeleteMemoryData { deleted: id });
        Ok(serde_json::to_value(response)?)
    } else {
        let response = ErrorResponse::new(format!("Memory not found: {}", id));
        Ok(serde_json::to_value(response)?)
    }
}

/// Get a memory by ID
pub async fn get_memory(pool: &PgPool, id: Uuid) -> Result<serde_json::Value> {
    match db::get_memory(pool, id).await? {
        Some(memory) => {
            let response = SuccessResponse::new(GetMemoryData {
                memory: memory.to_summary(),
            });
            Ok(serde_json::to_value(response)?)
        }
        None => {
            let response = ErrorResponse::new(format!("Memory not found: {}", id));
            Ok(serde_json::to_value(response)?)
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_memory_options_creation() {
        let opts = AddMemoryOptions {
            memory_type: MemoryType::Learning,
            content: "Test content".to_string(),
            tags: vec!["test".to_string()],
            confidence: Confidence::High,
            tier: Tier::Project,
            project_path: Some("/test/path".to_string()),
            source_session_id: None,
            source_turn_id: None,
        };

        assert_eq!(opts.memory_type, MemoryType::Learning);
        assert_eq!(opts.content, "Test content");
        assert_eq!(opts.tags.len(), 1);
        assert_eq!(opts.confidence, Confidence::High);
    }

    #[test]
    fn test_tier_to_scope_mapping_global() {
        let tier = Tier::Global;
        let scope = match tier {
            Tier::Global => Scope::Global,
            Tier::Project | Tier::Both => Scope::Project,
        };
        assert_eq!(scope, Scope::Global);
    }

    #[test]
    fn test_tier_to_scope_mapping_project() {
        let tier = Tier::Project;
        let scope = match tier {
            Tier::Global => Scope::Global,
            Tier::Project | Tier::Both => Scope::Project,
        };
        assert_eq!(scope, Scope::Project);
    }

    #[test]
    fn test_tier_to_scope_mapping_both() {
        // "Both" is used for searches, but when adding, it maps to Project
        let tier = Tier::Both;
        let scope = match tier {
            Tier::Global => Scope::Global,
            Tier::Project | Tier::Both => Scope::Project,
        };
        assert_eq!(scope, Scope::Project);
    }

    // Note: Full integration tests require a database connection
    // and are placed in tests/integration/memory_tests.rs
}
