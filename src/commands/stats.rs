//! Stats command: get memory statistics
//!
//! Returns counts by type, confidence, and scope.

use sqlx::postgres::PgPool;

use crate::db::queries;
use crate::error::Result;
use crate::models::{Scope, Tier};

pub use crate::db::queries::{ConfidenceCounts, MemoryStats, ScopeCounts, TypeCounts};

/// Options for stats command
#[derive(Debug, Clone)]
pub struct StatsOptions {
    /// Tier filter (project, global, or both)
    pub tier: Tier,
    /// Project path for project-scoped queries
    pub project_path: Option<String>,
}

/// Get memory statistics.
///
/// Returns counts grouped by type, confidence level, and scope.
pub async fn get_stats(pool: &PgPool, options: StatsOptions) -> Result<MemoryStats> {
    let (scope_filter, include_both) = tier_to_scope_filter(options.tier);

    queries::get_stats(
        pool,
        scope_filter,
        options.project_path.as_deref(),
        include_both,
    )
    .await
}

/// Convert Tier to (Option<Scope>, include_both) for query building
fn tier_to_scope_filter(tier: Tier) -> (Option<Scope>, bool) {
    match tier {
        Tier::Project => (Some(Scope::Project), false),
        Tier::Global => (Some(Scope::Global), false),
        Tier::Both => (None, true),
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stats_options_both() {
        let options = StatsOptions {
            tier: Tier::Both,
            project_path: Some("/test/path".to_string()),
        };

        assert_eq!(options.tier, Tier::Both);
        assert_eq!(options.project_path, Some("/test/path".to_string()));
    }

    #[test]
    fn test_stats_options_project() {
        let options = StatsOptions {
            tier: Tier::Project,
            project_path: Some("/my/project".to_string()),
        };

        assert_eq!(options.tier, Tier::Project);
    }

    #[test]
    fn test_stats_options_global() {
        let options = StatsOptions {
            tier: Tier::Global,
            project_path: None,
        };

        assert_eq!(options.tier, Tier::Global);
        assert!(options.project_path.is_none());
    }

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

    #[test]
    fn test_type_counts_struct() {
        let counts = TypeCounts {
            convention: 5,
            architecture: 3,
            gotcha: 10,
            api: 2,
            learning: 15,
            preference: 1,
        };

        assert_eq!(counts.convention, 5);
        assert_eq!(counts.gotcha, 10);
        assert_eq!(counts.learning, 15);
    }

    #[test]
    fn test_confidence_counts_struct() {
        let counts = ConfidenceCounts {
            high: 20,
            medium: 10,
            low: 5,
        };

        assert_eq!(counts.high, 20);
        assert_eq!(counts.medium, 10);
        assert_eq!(counts.low, 5);
    }

    #[test]
    fn test_scope_counts_struct() {
        let counts = ScopeCounts {
            project: 25,
            global: 15,
        };

        assert_eq!(counts.project, 25);
        assert_eq!(counts.global, 15);
    }

    #[test]
    fn test_memory_stats_struct() {
        let stats = MemoryStats {
            total: 40,
            by_type: TypeCounts {
                convention: 5,
                architecture: 5,
                gotcha: 10,
                api: 5,
                learning: 10,
                preference: 5,
            },
            by_confidence: ConfidenceCounts {
                high: 20,
                medium: 15,
                low: 5,
            },
            by_scope: ScopeCounts {
                project: 25,
                global: 15,
            },
        };

        assert_eq!(stats.total, 40);
        assert_eq!(stats.by_type.gotcha, 10);
        assert_eq!(stats.by_confidence.high, 20);
        assert_eq!(stats.by_scope.project, 25);
    }
}
