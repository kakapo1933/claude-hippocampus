use sqlx::postgres::PgPool;
use sqlx::Row;
use uuid::Uuid;

use crate::error::{HippocampusError, Result};
use crate::models::{Confidence, Memory, MemoryType, Scope};

/// Check for duplicate memory by matching first 100 chars of content
pub async fn find_duplicate(
    pool: &PgPool,
    memory_type: MemoryType,
    content: &str,
) -> Result<Option<DuplicateInfo>> {
    let content_prefix = content
        .chars()
        .take(100)
        .collect::<String>()
        .to_lowercase();

    let row = sqlx::query(
        r#"
        SELECT id, content, scope, confidence
        FROM memories
        WHERE type = $1
          AND LOWER(SUBSTRING(content, 1, 100)) = $2
        LIMIT 1
        "#,
    )
    .bind(memory_type.as_str())
    .bind(&content_prefix)
    .fetch_optional(pool)
    .await?;

    match row {
        Some(row) => {
            let content: String = row.get("content");
            let summary = if content.len() > 100 {
                format!("{}...", &content[..97])
            } else {
                content
            };

            Ok(Some(DuplicateInfo {
                id: row.get("id"),
                scope: row.get::<String, _>("scope"),
                summary,
            }))
        }
        None => Ok(None),
    }
}

/// Information about a duplicate memory
#[derive(Debug)]
pub struct DuplicateInfo {
    pub id: Uuid,
    pub scope: String,
    pub summary: String,
}

/// Insert a new memory entry
pub async fn insert_memory(
    pool: &PgPool,
    memory_type: MemoryType,
    scope: Scope,
    project_path: Option<&str>,
    content: &str,
    tags: &[String],
    confidence: Confidence,
    source_session_id: Option<Uuid>,
    source_turn_id: Option<Uuid>,
) -> Result<Uuid> {
    let row = sqlx::query(
        r#"
        INSERT INTO memories (type, scope, project_path, content, tags, confidence, source_session_id, source_turn_id)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING id
        "#,
    )
    .bind(memory_type.as_str())
    .bind(scope.as_str())
    .bind(project_path)
    .bind(content)
    .bind(tags)
    .bind(confidence.as_str())
    .bind(source_session_id)
    .bind(source_turn_id)
    .fetch_one(pool)
    .await?;

    Ok(row.get("id"))
}

/// Update an existing memory's content
pub async fn update_memory(
    pool: &PgPool,
    id: Uuid,
    content: &str,
    scope: Option<Scope>,
    project_path: Option<&str>,
) -> Result<bool> {
    let result = if let Some(s) = scope {
        sqlx::query(
            r#"
            UPDATE memories
            SET content = $2, scope = $3, project_path = $4, updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(content)
        .bind(s.as_str())
        .bind(project_path)
        .execute(pool)
        .await?
    } else {
        sqlx::query(
            r#"
            UPDATE memories
            SET content = $2, updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(content)
        .execute(pool)
        .await?
    };

    Ok(result.rows_affected() > 0)
}

/// Delete a memory by ID
pub async fn delete_memory(pool: &PgPool, id: Uuid) -> Result<bool> {
    let result = sqlx::query("DELETE FROM memories WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;

    Ok(result.rows_affected() > 0)
}

/// Get a single memory by ID
pub async fn get_memory(pool: &PgPool, id: Uuid) -> Result<Option<Memory>> {
    let row = sqlx::query(
        r#"
        SELECT id, type, scope, project_path, content, tags, confidence,
               source_session_id, source_turn_id, created_at, updated_at,
               accessed_at, access_count
        FROM memories
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    match row {
        Some(row) => Ok(Some(row_to_memory(&row)?)),
        None => Ok(None),
    }
}

/// Search memories by keyword (content or tags)
pub async fn search_keyword(
    pool: &PgPool,
    query: &str,
    scope_filter: Option<Scope>,
    project_path: Option<&str>,
    include_both_scopes: bool,
    limit: i32,
) -> Result<Vec<Memory>> {
    let query_pattern = format!("%{}%", query);

    // Build the WHERE clause based on scope filter
    let rows = if include_both_scopes {
        // Search both global and project (with matching path)
        sqlx::query(
            r#"
            SELECT id, type, scope, project_path, content, tags, confidence,
                   source_session_id, source_turn_id, created_at, updated_at,
                   accessed_at, access_count
            FROM memories
            WHERE (scope = 'global' OR (scope = 'project' AND project_path = $3))
              AND (content ILIKE $1 OR EXISTS (SELECT 1 FROM unnest(tags) AS t WHERE t ILIKE $1))
            ORDER BY
              CASE confidence WHEN 'high' THEN 0 WHEN 'medium' THEN 1 ELSE 2 END,
              created_at DESC
            LIMIT $2
            "#,
        )
        .bind(&query_pattern)
        .bind(limit as i64)
        .bind(project_path)
        .fetch_all(pool)
        .await?
    } else if let Some(scope) = scope_filter {
        // Search specific scope
        if scope == Scope::Project {
            sqlx::query(
                r#"
                SELECT id, type, scope, project_path, content, tags, confidence,
                       source_session_id, source_turn_id, created_at, updated_at,
                       accessed_at, access_count
                FROM memories
                WHERE scope = 'project' AND project_path = $3
                  AND (content ILIKE $1 OR EXISTS (SELECT 1 FROM unnest(tags) AS t WHERE t ILIKE $1))
                ORDER BY
                  CASE confidence WHEN 'high' THEN 0 WHEN 'medium' THEN 1 ELSE 2 END,
                  created_at DESC
                LIMIT $2
                "#,
            )
            .bind(&query_pattern)
            .bind(limit as i64)
            .bind(project_path)
            .fetch_all(pool)
            .await?
        } else {
            sqlx::query(
                r#"
                SELECT id, type, scope, project_path, content, tags, confidence,
                       source_session_id, source_turn_id, created_at, updated_at,
                       accessed_at, access_count
                FROM memories
                WHERE scope = 'global'
                  AND (content ILIKE $1 OR EXISTS (SELECT 1 FROM unnest(tags) AS t WHERE t ILIKE $1))
                ORDER BY
                  CASE confidence WHEN 'high' THEN 0 WHEN 'medium' THEN 1 ELSE 2 END,
                  created_at DESC
                LIMIT $2
                "#,
            )
            .bind(&query_pattern)
            .bind(limit as i64)
            .fetch_all(pool)
            .await?
        }
    } else {
        // No filter, search all
        sqlx::query(
            r#"
            SELECT id, type, scope, project_path, content, tags, confidence,
                   source_session_id, source_turn_id, created_at, updated_at,
                   accessed_at, access_count
            FROM memories
            WHERE content ILIKE $1 OR EXISTS (SELECT 1 FROM unnest(tags) AS t WHERE t ILIKE $1)
            ORDER BY
              CASE confidence WHEN 'high' THEN 0 WHEN 'medium' THEN 1 ELSE 2 END,
              created_at DESC
            LIMIT $2
            "#,
        )
        .bind(&query_pattern)
        .bind(limit as i64)
        .fetch_all(pool)
        .await?
    };

    rows.iter().map(row_to_memory).collect()
}

/// Get memories for context (high priority, recent)
pub async fn get_context_memories(
    pool: &PgPool,
    project_path: Option<&str>,
    limit: i32,
) -> Result<Vec<Memory>> {
    let rows = sqlx::query(
        r#"
        SELECT id, type, scope, project_path, content, tags, confidence,
               source_session_id, source_turn_id, created_at, updated_at,
               accessed_at, access_count
        FROM memories
        WHERE scope = 'global' OR (scope = 'project' AND project_path = $2)
        ORDER BY
          CASE confidence WHEN 'high' THEN 0 WHEN 'medium' THEN 1 ELSE 2 END,
          access_count DESC,
          created_at DESC
        LIMIT $1
        "#,
    )
    .bind(limit as i64)
    .bind(project_path)
    .fetch_all(pool)
    .await?;

    rows.iter().map(row_to_memory).collect()
}

/// List recent memories
pub async fn list_recent(
    pool: &PgPool,
    scope_filter: Option<Scope>,
    project_path: Option<&str>,
    include_both_scopes: bool,
    limit: i32,
) -> Result<(Vec<Memory>, i64)> {
    // Get total count
    let total: i64 = if include_both_scopes {
        sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM memories
            WHERE scope = 'global' OR (scope = 'project' AND project_path = $1)
            "#,
        )
        .bind(project_path)
        .fetch_one(pool)
        .await?
    } else if let Some(scope) = scope_filter {
        if scope == Scope::Project {
            sqlx::query_scalar(
                r#"SELECT COUNT(*) FROM memories WHERE scope = 'project' AND project_path = $1"#,
            )
            .bind(project_path)
            .fetch_one(pool)
            .await?
        } else {
            sqlx::query_scalar(r#"SELECT COUNT(*) FROM memories WHERE scope = 'global'"#)
                .fetch_one(pool)
                .await?
        }
    } else {
        sqlx::query_scalar(r#"SELECT COUNT(*) FROM memories"#)
            .fetch_one(pool)
            .await?
    };

    // Get recent entries
    let rows = if include_both_scopes {
        sqlx::query(
            r#"
            SELECT id, type, scope, project_path, content, tags, confidence,
                   source_session_id, source_turn_id, created_at, updated_at,
                   accessed_at, access_count
            FROM memories
            WHERE scope = 'global' OR (scope = 'project' AND project_path = $2)
            ORDER BY created_at DESC
            LIMIT $1
            "#,
        )
        .bind(limit as i64)
        .bind(project_path)
        .fetch_all(pool)
        .await?
    } else if let Some(scope) = scope_filter {
        if scope == Scope::Project {
            sqlx::query(
                r#"
                SELECT id, type, scope, project_path, content, tags, confidence,
                       source_session_id, source_turn_id, created_at, updated_at,
                       accessed_at, access_count
                FROM memories
                WHERE scope = 'project' AND project_path = $2
                ORDER BY created_at DESC
                LIMIT $1
                "#,
            )
            .bind(limit as i64)
            .bind(project_path)
            .fetch_all(pool)
            .await?
        } else {
            sqlx::query(
                r#"
                SELECT id, type, scope, project_path, content, tags, confidence,
                       source_session_id, source_turn_id, created_at, updated_at,
                       accessed_at, access_count
                FROM memories
                WHERE scope = 'global'
                ORDER BY created_at DESC
                LIMIT $1
                "#,
            )
            .bind(limit as i64)
            .fetch_all(pool)
            .await?
        }
    } else {
        sqlx::query(
            r#"
            SELECT id, type, scope, project_path, content, tags, confidence,
                   source_session_id, source_turn_id, created_at, updated_at,
                   accessed_at, access_count
            FROM memories
            ORDER BY created_at DESC
            LIMIT $1
            "#,
        )
        .bind(limit as i64)
        .fetch_all(pool)
        .await?
    };

    let memories: Result<Vec<Memory>> = rows.iter().map(row_to_memory).collect();
    Ok((memories?, total))
}

/// Find and remove duplicate memories (consolidate)
pub async fn consolidate_duplicates(
    pool: &PgPool,
    scope_filter: Option<Scope>,
    project_path: Option<&str>,
) -> Result<Vec<Uuid>> {
    // Find duplicates (same type, same first 100 chars)
    let duplicate_rows = if let Some(scope) = scope_filter {
        if scope == Scope::Project {
            sqlx::query(
                r#"
                SELECT m2.id
                FROM memories m1
                JOIN memories m2 ON m1.id < m2.id AND m1.type = m2.type
                WHERE LOWER(SUBSTRING(m1.content, 1, 100)) = LOWER(SUBSTRING(m2.content, 1, 100))
                  AND m1.scope = 'project' AND m2.scope = 'project'
                  AND m1.project_path = $1 AND m2.project_path = $1
                "#,
            )
            .bind(project_path)
            .fetch_all(pool)
            .await?
        } else {
            sqlx::query(
                r#"
                SELECT m2.id
                FROM memories m1
                JOIN memories m2 ON m1.id < m2.id AND m1.type = m2.type
                WHERE LOWER(SUBSTRING(m1.content, 1, 100)) = LOWER(SUBSTRING(m2.content, 1, 100))
                  AND m1.scope = 'global' AND m2.scope = 'global'
                "#,
            )
            .fetch_all(pool)
            .await?
        }
    } else {
        sqlx::query(
            r#"
            SELECT m2.id
            FROM memories m1
            JOIN memories m2 ON m1.id < m2.id AND m1.type = m2.type
            WHERE LOWER(SUBSTRING(m1.content, 1, 100)) = LOWER(SUBSTRING(m2.content, 1, 100))
            "#,
        )
        .fetch_all(pool)
        .await?
    };

    let duplicate_ids: Vec<Uuid> = duplicate_rows.iter().map(|r| r.get("id")).collect();

    // Delete duplicates
    for id in &duplicate_ids {
        sqlx::query("DELETE FROM memories WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?;
    }

    Ok(duplicate_ids)
}

/// Prune old low-confidence memories
pub async fn prune_old_memories(
    pool: &PgPool,
    days: i32,
    scope_filter: Option<Scope>,
    project_path: Option<&str>,
) -> Result<Vec<Uuid>> {
    let pruned_rows = if let Some(scope) = scope_filter {
        if scope == Scope::Project {
            sqlx::query(
                r#"
                DELETE FROM memories
                WHERE confidence = 'low'
                  AND access_count = 0
                  AND created_at < NOW() - INTERVAL '1 day' * $1
                  AND scope = 'project'
                  AND project_path = $2
                RETURNING id
                "#,
            )
            .bind(days)
            .bind(project_path)
            .fetch_all(pool)
            .await?
        } else {
            sqlx::query(
                r#"
                DELETE FROM memories
                WHERE confidence = 'low'
                  AND access_count = 0
                  AND created_at < NOW() - INTERVAL '1 day' * $1
                  AND scope = 'global'
                RETURNING id
                "#,
            )
            .bind(days)
            .fetch_all(pool)
            .await?
        }
    } else {
        sqlx::query(
            r#"
            DELETE FROM memories
            WHERE confidence = 'low'
              AND access_count = 0
              AND created_at < NOW() - INTERVAL '1 day' * $1
            RETURNING id
            "#,
        )
        .bind(days)
        .fetch_all(pool)
        .await?
    };

    Ok(pruned_rows.iter().map(|r| r.get("id")).collect())
}

/// Save session summary
pub async fn save_session_summary(
    pool: &PgPool,
    claude_session_id: &str,
    summary: &serde_json::Value,
) -> Result<Uuid> {
    let row = sqlx::query(
        r#"
        UPDATE sessions
        SET summary = $2, status = 'completed', ended_at = NOW()
        WHERE claude_session_id = $1
        RETURNING id
        "#,
    )
    .bind(claude_session_id)
    .bind(summary)
    .fetch_optional(pool)
    .await?;

    match row {
        Some(r) => Ok(r.get("id")),
        None => Err(HippocampusError::NotFound(format!(
            "Session not found: {}",
            claude_session_id
        ))),
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

fn row_to_memory(row: &sqlx::postgres::PgRow) -> Result<Memory> {
    let type_str: String = row.get("type");
    let scope_str: String = row.get("scope");
    let confidence_str: String = row.get("confidence");

    Ok(Memory {
        id: row.get("id"),
        memory_type: type_str.parse()?,
        scope: scope_str.parse()?,
        project_path: row.get("project_path"),
        content: row.get("content"),
        tags: row.get("tags"),
        confidence: confidence_str.parse()?,
        source_session_id: row.get("source_session_id"),
        source_turn_id: row.get("source_turn_id"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
        accessed_at: row.get("accessed_at"),
        access_count: row.get("access_count"),
    })
}

// ============================================================================
// Tests (unit tests - integration tests require database)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_duplicate_info_struct() {
        let info = DuplicateInfo {
            id: Uuid::new_v4(),
            scope: "project".to_string(),
            summary: "Test summary".to_string(),
        };
        assert_eq!(info.scope, "project");
        assert_eq!(info.summary, "Test summary");
    }

    // Note: Most query tests require a live database connection
    // and are placed in tests/integration/
}
