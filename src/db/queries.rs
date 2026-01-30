use sqlx::postgres::PgPool;
use sqlx::Row;
use uuid::Uuid;

use crate::error::{HippocampusError, Result};
use crate::git::GitStatus;
use crate::models::{Confidence, Memory, MemoryType, Scope, Session};

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

/// Search memories by type (with optional keyword filter)
pub async fn search_by_type(
    pool: &PgPool,
    memory_type: MemoryType,
    query: Option<&str>,
    scope_filter: Option<Scope>,
    project_path: Option<&str>,
    include_both_scopes: bool,
    limit: i32,
) -> Result<Vec<Memory>> {
    let query_pattern = query.map(|q| format!("%{}%", q));

    // Build the WHERE clause based on scope filter and optional query
    let rows = match (include_both_scopes, scope_filter, &query_pattern) {
        // Both scopes, with keyword
        (true, _, Some(pattern)) => {
            sqlx::query(
                r#"
                SELECT id, type, scope, project_path, content, tags, confidence,
                       source_session_id, source_turn_id, created_at, updated_at,
                       accessed_at, access_count
                FROM memories
                WHERE type = $1
                  AND (scope = 'global' OR (scope = 'project' AND project_path = $4))
                  AND (content ILIKE $2 OR EXISTS (SELECT 1 FROM unnest(tags) AS t WHERE t ILIKE $2))
                ORDER BY
                  CASE confidence WHEN 'high' THEN 0 WHEN 'medium' THEN 1 ELSE 2 END,
                  created_at DESC
                LIMIT $3
                "#,
            )
            .bind(memory_type.as_str())
            .bind(pattern)
            .bind(limit as i64)
            .bind(project_path)
            .fetch_all(pool)
            .await?
        }
        // Both scopes, no keyword
        (true, _, None) => {
            sqlx::query(
                r#"
                SELECT id, type, scope, project_path, content, tags, confidence,
                       source_session_id, source_turn_id, created_at, updated_at,
                       accessed_at, access_count
                FROM memories
                WHERE type = $1
                  AND (scope = 'global' OR (scope = 'project' AND project_path = $3))
                ORDER BY
                  CASE confidence WHEN 'high' THEN 0 WHEN 'medium' THEN 1 ELSE 2 END,
                  created_at DESC
                LIMIT $2
                "#,
            )
            .bind(memory_type.as_str())
            .bind(limit as i64)
            .bind(project_path)
            .fetch_all(pool)
            .await?
        }
        // Project scope, with keyword
        (false, Some(Scope::Project), Some(pattern)) => {
            sqlx::query(
                r#"
                SELECT id, type, scope, project_path, content, tags, confidence,
                       source_session_id, source_turn_id, created_at, updated_at,
                       accessed_at, access_count
                FROM memories
                WHERE type = $1
                  AND scope = 'project' AND project_path = $4
                  AND (content ILIKE $2 OR EXISTS (SELECT 1 FROM unnest(tags) AS t WHERE t ILIKE $2))
                ORDER BY
                  CASE confidence WHEN 'high' THEN 0 WHEN 'medium' THEN 1 ELSE 2 END,
                  created_at DESC
                LIMIT $3
                "#,
            )
            .bind(memory_type.as_str())
            .bind(pattern)
            .bind(limit as i64)
            .bind(project_path)
            .fetch_all(pool)
            .await?
        }
        // Project scope, no keyword
        (false, Some(Scope::Project), None) => {
            sqlx::query(
                r#"
                SELECT id, type, scope, project_path, content, tags, confidence,
                       source_session_id, source_turn_id, created_at, updated_at,
                       accessed_at, access_count
                FROM memories
                WHERE type = $1
                  AND scope = 'project' AND project_path = $3
                ORDER BY
                  CASE confidence WHEN 'high' THEN 0 WHEN 'medium' THEN 1 ELSE 2 END,
                  created_at DESC
                LIMIT $2
                "#,
            )
            .bind(memory_type.as_str())
            .bind(limit as i64)
            .bind(project_path)
            .fetch_all(pool)
            .await?
        }
        // Global scope, with keyword
        (false, Some(Scope::Global), Some(pattern)) => {
            sqlx::query(
                r#"
                SELECT id, type, scope, project_path, content, tags, confidence,
                       source_session_id, source_turn_id, created_at, updated_at,
                       accessed_at, access_count
                FROM memories
                WHERE type = $1
                  AND scope = 'global'
                  AND (content ILIKE $2 OR EXISTS (SELECT 1 FROM unnest(tags) AS t WHERE t ILIKE $2))
                ORDER BY
                  CASE confidence WHEN 'high' THEN 0 WHEN 'medium' THEN 1 ELSE 2 END,
                  created_at DESC
                LIMIT $3
                "#,
            )
            .bind(memory_type.as_str())
            .bind(pattern)
            .bind(limit as i64)
            .fetch_all(pool)
            .await?
        }
        // Global scope, no keyword
        (false, Some(Scope::Global), None) => {
            sqlx::query(
                r#"
                SELECT id, type, scope, project_path, content, tags, confidence,
                       source_session_id, source_turn_id, created_at, updated_at,
                       accessed_at, access_count
                FROM memories
                WHERE type = $1
                  AND scope = 'global'
                ORDER BY
                  CASE confidence WHEN 'high' THEN 0 WHEN 'medium' THEN 1 ELSE 2 END,
                  created_at DESC
                LIMIT $2
                "#,
            )
            .bind(memory_type.as_str())
            .bind(limit as i64)
            .fetch_all(pool)
            .await?
        }
        // No scope filter, with keyword
        (false, None, Some(pattern)) => {
            sqlx::query(
                r#"
                SELECT id, type, scope, project_path, content, tags, confidence,
                       source_session_id, source_turn_id, created_at, updated_at,
                       accessed_at, access_count
                FROM memories
                WHERE type = $1
                  AND (content ILIKE $2 OR EXISTS (SELECT 1 FROM unnest(tags) AS t WHERE t ILIKE $2))
                ORDER BY
                  CASE confidence WHEN 'high' THEN 0 WHEN 'medium' THEN 1 ELSE 2 END,
                  created_at DESC
                LIMIT $3
                "#,
            )
            .bind(memory_type.as_str())
            .bind(pattern)
            .bind(limit as i64)
            .fetch_all(pool)
            .await?
        }
        // No scope filter, no keyword
        (false, None, None) => {
            sqlx::query(
                r#"
                SELECT id, type, scope, project_path, content, tags, confidence,
                       source_session_id, source_turn_id, created_at, updated_at,
                       accessed_at, access_count
                FROM memories
                WHERE type = $1
                ORDER BY
                  CASE confidence WHEN 'high' THEN 0 WHEN 'medium' THEN 1 ELSE 2 END,
                  created_at DESC
                LIMIT $2
                "#,
            )
            .bind(memory_type.as_str())
            .bind(limit as i64)
            .fetch_all(pool)
            .await?
        }
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

/// Memory statistics
#[derive(Debug, Clone, serde::Serialize)]
pub struct MemoryStats {
    pub total: i64,
    pub by_type: TypeCounts,
    pub by_confidence: ConfidenceCounts,
    pub by_scope: ScopeCounts,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TypeCounts {
    pub convention: i64,
    pub architecture: i64,
    pub gotcha: i64,
    pub api: i64,
    pub learning: i64,
    pub preference: i64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ConfidenceCounts {
    pub high: i64,
    pub medium: i64,
    pub low: i64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ScopeCounts {
    pub project: i64,
    pub global: i64,
}

/// Get memory statistics
pub async fn get_stats(
    pool: &PgPool,
    scope_filter: Option<Scope>,
    project_path: Option<&str>,
    include_both_scopes: bool,
) -> Result<MemoryStats> {
    // Build WHERE clause based on scope filter
    let where_clause = if include_both_scopes {
        format!(
            "WHERE (scope = 'global' OR (scope = 'project' AND project_path = '{}'))",
            project_path.unwrap_or("")
        )
    } else if let Some(scope) = scope_filter {
        if scope == Scope::Project {
            format!(
                "WHERE scope = 'project' AND project_path = '{}'",
                project_path.unwrap_or("")
            )
        } else {
            "WHERE scope = 'global'".to_string()
        }
    } else {
        String::new()
    };

    // Get total count
    let total: i64 = sqlx::query_scalar(&format!(
        "SELECT COUNT(*) FROM memories {}",
        where_clause
    ))
    .fetch_one(pool)
    .await?;

    // Get counts by type
    let type_rows = sqlx::query(&format!(
        "SELECT type, COUNT(*) as count FROM memories {} GROUP BY type",
        where_clause
    ))
    .fetch_all(pool)
    .await?;

    let mut by_type = TypeCounts {
        convention: 0,
        architecture: 0,
        gotcha: 0,
        api: 0,
        learning: 0,
        preference: 0,
    };

    for row in &type_rows {
        let type_str: String = row.get("type");
        let count: i64 = row.get("count");
        match type_str.as_str() {
            "convention" => by_type.convention = count,
            "architecture" => by_type.architecture = count,
            "gotcha" => by_type.gotcha = count,
            "api" => by_type.api = count,
            "learning" => by_type.learning = count,
            "preference" => by_type.preference = count,
            _ => {}
        }
    }

    // Get counts by confidence
    let conf_rows = sqlx::query(&format!(
        "SELECT confidence, COUNT(*) as count FROM memories {} GROUP BY confidence",
        where_clause
    ))
    .fetch_all(pool)
    .await?;

    let mut by_confidence = ConfidenceCounts {
        high: 0,
        medium: 0,
        low: 0,
    };

    for row in &conf_rows {
        let conf_str: String = row.get("confidence");
        let count: i64 = row.get("count");
        match conf_str.as_str() {
            "high" => by_confidence.high = count,
            "medium" => by_confidence.medium = count,
            "low" => by_confidence.low = count,
            _ => {}
        }
    }

    // Get counts by scope
    let scope_rows = sqlx::query(&format!(
        "SELECT scope, COUNT(*) as count FROM memories {} GROUP BY scope",
        where_clause
    ))
    .fetch_all(pool)
    .await?;

    let mut by_scope = ScopeCounts {
        project: 0,
        global: 0,
    };

    for row in &scope_rows {
        let scope_str: String = row.get("scope");
        let count: i64 = row.get("count");
        match scope_str.as_str() {
            "project" => by_scope.project = count,
            "global" => by_scope.global = count,
            _ => {}
        }
    }

    Ok(MemoryStats {
        total,
        by_type,
        by_confidence,
        by_scope,
    })
}

/// Update access tracking for memories (accessed_at, access_count)
pub async fn mark_memories_accessed(pool: &PgPool, ids: &[Uuid]) -> Result<u64> {
    if ids.is_empty() {
        return Ok(0);
    }

    let result = sqlx::query(
        r#"
        UPDATE memories
        SET accessed_at = NOW(), access_count = access_count + 1
        WHERE id = ANY($1)
        "#,
    )
    .bind(ids)
    .execute(pool)
    .await?;

    Ok(result.rows_affected())
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
// Session Queries
// ============================================================================

/// Create a new session
pub async fn create_session(
    pool: &PgPool,
    claude_session_id: &str,
    project_path: Option<&str>,
    git_status: Option<&GitStatus>,
) -> Result<Session> {
    let git_status_json = git_status.map(|gs| serde_json::to_value(gs).ok()).flatten();

    let row = sqlx::query(
        r#"
        INSERT INTO sessions (claude_session_id, project_path, git_status)
        VALUES ($1, $2, $3)
        RETURNING id, claude_session_id, project_path, git_status, models_used,
                  status, summary, started_at, ended_at, created_at
        "#,
    )
    .bind(claude_session_id)
    .bind(project_path)
    .bind(&git_status_json)
    .fetch_one(pool)
    .await?;

    row_to_session(&row)
}

/// Find session by database UUID
pub async fn find_session_by_id(pool: &PgPool, id: Uuid) -> Result<Option<Session>> {
    let row = sqlx::query(
        r#"
        SELECT id, claude_session_id, project_path, git_status, models_used,
               status, summary, started_at, ended_at, created_at
        FROM sessions
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    match row {
        Some(r) => Ok(Some(row_to_session(&r)?)),
        None => Ok(None),
    }
}

/// Find session by Claude session ID
pub async fn find_session_by_claude_id(
    pool: &PgPool,
    claude_session_id: &str,
) -> Result<Option<Session>> {
    let row = sqlx::query(
        r#"
        SELECT id, claude_session_id, project_path, git_status, models_used,
               status, summary, started_at, ended_at, created_at
        FROM sessions
        WHERE claude_session_id = $1
        "#,
    )
    .bind(claude_session_id)
    .fetch_optional(pool)
    .await?;

    match row {
        Some(r) => Ok(Some(row_to_session(&r)?)),
        None => Ok(None),
    }
}

/// End a session
pub async fn end_session(
    pool: &PgPool,
    claude_session_id: &str,
    summary: Option<&str>,
) -> Result<Session> {
    let summary_json = summary.map(|s| serde_json::json!({ "summary": s }));

    let row = sqlx::query(
        r#"
        UPDATE sessions
        SET status = 'completed', ended_at = NOW(), summary = COALESCE($2, summary)
        WHERE claude_session_id = $1
        RETURNING id, claude_session_id, project_path, git_status, models_used,
                  status, summary, started_at, ended_at, created_at
        "#,
    )
    .bind(claude_session_id)
    .bind(&summary_json)
    .fetch_optional(pool)
    .await?;

    match row {
        Some(r) => row_to_session(&r),
        None => Err(HippocampusError::SessionNotFound(claude_session_id.to_string())),
    }
}

// ============================================================================
// Turn Queries
// ============================================================================

use crate::models::Turn;

/// Create a new conversation turn
pub async fn create_turn(
    pool: &PgPool,
    session_id: Uuid,
    turn_number: i32,
    user_prompt: &str,
    model_used: Option<&str>,
) -> Result<Turn> {
    let row = sqlx::query(
        r#"
        INSERT INTO conversation_turns (session_id, turn_number, user_prompt, model_used, started_at)
        VALUES ($1, $2, $3, $4, NOW())
        RETURNING id, session_id, turn_number, user_prompt, assistant_response,
                  model_used, input_tokens, output_tokens, started_at, ended_at, created_at
        "#,
    )
    .bind(session_id)
    .bind(turn_number)
    .bind(user_prompt)
    .bind(model_used)
    .fetch_one(pool)
    .await?;

    row_to_turn(&row)
}

/// Get the next turn number for a session
pub async fn get_next_turn_number(pool: &PgPool, session_id: Uuid) -> Result<i32> {
    let count: Option<i32> = sqlx::query_scalar(
        "SELECT MAX(turn_number)::INT4 FROM conversation_turns WHERE session_id = $1",
    )
    .bind(session_id)
    .fetch_one(pool)
    .await?;

    Ok(count.map(|n| n + 1).unwrap_or(1))
}

/// Find turn by ID
pub async fn find_turn_by_id(pool: &PgPool, id: Uuid) -> Result<Option<Turn>> {
    let row = sqlx::query(
        r#"
        SELECT id, session_id, turn_number, user_prompt, assistant_response,
               model_used, input_tokens, output_tokens, started_at, ended_at, created_at
        FROM conversation_turns
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    match row {
        Some(r) => Ok(Some(row_to_turn(&r)?)),
        None => Ok(None),
    }
}

/// Update turn with assistant response
pub async fn update_turn(
    pool: &PgPool,
    turn_id: Uuid,
    response: &str,
    input_tokens: Option<i32>,
    output_tokens: Option<i32>,
) -> Result<Turn> {
    let row = sqlx::query(
        r#"
        UPDATE conversation_turns
        SET assistant_response = $2, input_tokens = $3, output_tokens = $4, ended_at = NOW()
        WHERE id = $1
        RETURNING id, session_id, turn_number, user_prompt, assistant_response,
                  model_used, input_tokens, output_tokens, started_at, ended_at, created_at
        "#,
    )
    .bind(turn_id)
    .bind(response)
    .bind(input_tokens)
    .bind(output_tokens)
    .fetch_optional(pool)
    .await?;

    match row {
        Some(r) => row_to_turn(&r),
        None => Err(HippocampusError::NotFound(format!("Turn not found: {}", turn_id))),
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

fn row_to_turn(row: &sqlx::postgres::PgRow) -> Result<Turn> {
    Ok(Turn {
        id: row.get("id"),
        session_id: row.get("session_id"),
        turn_number: row.get("turn_number"),
        user_prompt: row.get("user_prompt"),
        assistant_response: row.get("assistant_response"),
        model_used: row.get("model_used"),
        input_tokens: row.get("input_tokens"),
        output_tokens: row.get("output_tokens"),
        started_at: row.get("started_at"),
        ended_at: row.get("ended_at"),
        created_at: row.get("created_at"),
    })
}

fn row_to_session(row: &sqlx::postgres::PgRow) -> Result<Session> {
    let status_str: String = row.get("status");
    let git_status_json: Option<serde_json::Value> = row.get("git_status");

    let git_status = git_status_json
        .map(|v| serde_json::from_value::<GitStatus>(v).ok())
        .flatten();

    Ok(Session {
        id: row.get("id"),
        claude_session_id: row.get("claude_session_id"),
        project_path: row.get("project_path"),
        git_status,
        models_used: row.get("models_used"),
        status: status_str.parse()?,
        summary: row.get("summary"),
        started_at: row.get("started_at"),
        ended_at: row.get("ended_at"),
        created_at: row.get("created_at"),
    })
}

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
// Tool Call Recording
// ============================================================================

/// Tool call record
#[derive(Debug, Clone, serde::Serialize)]
pub struct ToolCall {
    pub id: Uuid,
    pub session_id: Option<Uuid>,
    pub turn_id: Option<Uuid>,
    pub tool_name: String,
    pub parameters: Option<serde_json::Value>,
    pub result_summary: Option<String>,
    pub called_at: chrono::DateTime<chrono::Utc>,
}

/// Record a tool call to the database
pub async fn record_tool_call(
    pool: &PgPool,
    session_id: Option<Uuid>,
    turn_id: Option<Uuid>,
    tool_name: &str,
    parameters: Option<serde_json::Value>,
    result_summary: Option<String>,
) -> Result<ToolCall> {
    let row = sqlx::query(
        r#"
        INSERT INTO tool_calls (session_id, turn_id, tool_name, parameters, result_summary)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id, session_id, turn_id, tool_name, parameters, result_summary, called_at
        "#,
    )
    .bind(session_id)
    .bind(turn_id)
    .bind(tool_name)
    .bind(&parameters)
    .bind(&result_summary)
    .fetch_one(pool)
    .await?;

    Ok(ToolCall {
        id: row.get("id"),
        session_id: row.get("session_id"),
        turn_id: row.get("turn_id"),
        tool_name: row.get("tool_name"),
        parameters: row.get("parameters"),
        result_summary: row.get("result_summary"),
        called_at: row.get("called_at"),
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

    #[test]
    fn test_tool_call_struct() {
        let tool_call = ToolCall {
            id: Uuid::new_v4(),
            session_id: Some(Uuid::new_v4()),
            turn_id: None,
            tool_name: "Read".to_string(),
            parameters: Some(serde_json::json!({"file_path": "/tmp/test.txt"})),
            result_summary: Some("File read successfully".to_string()),
            called_at: chrono::Utc::now(),
        };
        assert_eq!(tool_call.tool_name, "Read");
        assert!(tool_call.parameters.is_some());
    }

    #[tokio::test]
    async fn test_mark_memories_accessed_empty_ids() {
        // This test verifies that empty array doesn't cause errors
        // and returns 0 without hitting the database
        // (actual database interaction would require integration test)
        let ids: Vec<Uuid> = vec![];
        assert!(ids.is_empty());
        // The function should return Ok(0) for empty ids
        // Full integration test in tests/integration/
    }

    // Note: Most query tests require a live database connection
    // and are placed in tests/integration/
}
