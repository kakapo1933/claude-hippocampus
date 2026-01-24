use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use uuid::Uuid;

use crate::error::HippocampusError;

// ============================================================================
// MemoryType
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MemoryType {
    Convention,
    Architecture,
    Gotcha,
    Api,
    Learning,
    Preference,
}

impl MemoryType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Convention => "convention",
            Self::Architecture => "architecture",
            Self::Gotcha => "gotcha",
            Self::Api => "api",
            Self::Learning => "learning",
            Self::Preference => "preference",
        }
    }
}

impl FromStr for MemoryType {
    type Err = HippocampusError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "convention" => Ok(Self::Convention),
            "architecture" => Ok(Self::Architecture),
            "gotcha" => Ok(Self::Gotcha),
            "api" => Ok(Self::Api),
            "learning" => Ok(Self::Learning),
            "preference" => Ok(Self::Preference),
            _ => Err(HippocampusError::InvalidMemoryType(s.to_string())),
        }
    }
}

// ============================================================================
// Confidence
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Confidence {
    High,
    Medium,
    Low,
}

impl Confidence {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::High => "high",
            Self::Medium => "medium",
            Self::Low => "low",
        }
    }

    /// Returns the sort order (high=0, medium=1, low=2) for confidence-based ordering
    pub fn sort_order(&self) -> i32 {
        match self {
            Self::High => 0,
            Self::Medium => 1,
            Self::Low => 2,
        }
    }

    /// Returns the display symbol for context formatting
    pub fn symbol(&self) -> &'static str {
        match self {
            Self::High => "★",
            Self::Medium => "◐",
            Self::Low => "○",
        }
    }
}

impl FromStr for Confidence {
    type Err = HippocampusError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "high" => Ok(Self::High),
            "medium" => Ok(Self::Medium),
            "low" => Ok(Self::Low),
            _ => Err(HippocampusError::InvalidConfidence(s.to_string())),
        }
    }
}

// ============================================================================
// Scope
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Scope {
    Project,
    Global,
}

impl Scope {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Project => "project",
            Self::Global => "global",
        }
    }
}

impl FromStr for Scope {
    type Err = HippocampusError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "project" => Ok(Self::Project),
            "global" => Ok(Self::Global),
            _ => Err(HippocampusError::InvalidScope(s.to_string())),
        }
    }
}

// ============================================================================
// Tier (for CLI - includes "both" option)
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Tier {
    Project,
    Global,
    Both,
}

impl Tier {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Project => "project",
            Self::Global => "global",
            Self::Both => "both",
        }
    }
}

impl FromStr for Tier {
    type Err = HippocampusError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "project" => Ok(Self::Project),
            "global" => Ok(Self::Global),
            "both" => Ok(Self::Both),
            _ => Err(HippocampusError::InvalidTier(s.to_string())),
        }
    }
}

// ============================================================================
// Memory (main struct)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Memory {
    pub id: Uuid,
    #[serde(rename = "type")]
    pub memory_type: MemoryType,
    pub scope: Scope,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_path: Option<String>,
    pub content: String,
    pub tags: Vec<String>,
    pub confidence: Confidence,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_session_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_turn_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accessed_at: Option<DateTime<Utc>>,
    pub access_count: i32,
}

/// Summary view of a memory (for list/search results)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemorySummary {
    pub id: Uuid,
    #[serde(rename = "type")]
    pub memory_type: MemoryType,
    pub tier: Scope,
    pub summary: String, // First 100 chars of content
    pub tags: Vec<String>,
    pub confidence: Confidence,
    pub created: DateTime<Utc>,
    pub access_count: i32,
}

impl Memory {
    /// Convert to summary view
    pub fn to_summary(&self) -> MemorySummary {
        let summary = if self.content.len() > 100 {
            format!("{}...", &self.content[..97])
        } else {
            self.content.clone()
        };

        MemorySummary {
            id: self.id,
            memory_type: self.memory_type,
            tier: self.scope,
            summary,
            tags: self.tags.clone(),
            confidence: self.confidence,
            created: self.created_at,
            access_count: self.access_count,
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // MemoryType tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_memory_type_parse_convention() {
        assert_eq!(
            "convention".parse::<MemoryType>().unwrap(),
            MemoryType::Convention
        );
    }

    #[test]
    fn test_memory_type_parse_architecture() {
        assert_eq!(
            "architecture".parse::<MemoryType>().unwrap(),
            MemoryType::Architecture
        );
    }

    #[test]
    fn test_memory_type_parse_gotcha() {
        assert_eq!("gotcha".parse::<MemoryType>().unwrap(), MemoryType::Gotcha);
    }

    #[test]
    fn test_memory_type_parse_api() {
        assert_eq!("api".parse::<MemoryType>().unwrap(), MemoryType::Api);
    }

    #[test]
    fn test_memory_type_parse_learning() {
        assert_eq!(
            "learning".parse::<MemoryType>().unwrap(),
            MemoryType::Learning
        );
    }

    #[test]
    fn test_memory_type_parse_preference() {
        assert_eq!(
            "preference".parse::<MemoryType>().unwrap(),
            MemoryType::Preference
        );
    }

    #[test]
    fn test_memory_type_parse_case_insensitive() {
        assert_eq!(
            "CONVENTION".parse::<MemoryType>().unwrap(),
            MemoryType::Convention
        );
        assert_eq!(
            "Architecture".parse::<MemoryType>().unwrap(),
            MemoryType::Architecture
        );
    }

    #[test]
    fn test_memory_type_parse_invalid() {
        assert!("invalid".parse::<MemoryType>().is_err());
        assert!("".parse::<MemoryType>().is_err());
    }

    #[test]
    fn test_memory_type_as_str() {
        assert_eq!(MemoryType::Convention.as_str(), "convention");
        assert_eq!(MemoryType::Architecture.as_str(), "architecture");
        assert_eq!(MemoryType::Gotcha.as_str(), "gotcha");
        assert_eq!(MemoryType::Api.as_str(), "api");
        assert_eq!(MemoryType::Learning.as_str(), "learning");
        assert_eq!(MemoryType::Preference.as_str(), "preference");
    }

    // -------------------------------------------------------------------------
    // Confidence tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_confidence_parse_high() {
        assert_eq!("high".parse::<Confidence>().unwrap(), Confidence::High);
    }

    #[test]
    fn test_confidence_parse_medium() {
        assert_eq!("medium".parse::<Confidence>().unwrap(), Confidence::Medium);
    }

    #[test]
    fn test_confidence_parse_low() {
        assert_eq!("low".parse::<Confidence>().unwrap(), Confidence::Low);
    }

    #[test]
    fn test_confidence_parse_case_insensitive() {
        assert_eq!("HIGH".parse::<Confidence>().unwrap(), Confidence::High);
        assert_eq!("Medium".parse::<Confidence>().unwrap(), Confidence::Medium);
    }

    #[test]
    fn test_confidence_parse_invalid() {
        assert!("invalid".parse::<Confidence>().is_err());
    }

    #[test]
    fn test_confidence_sort_order() {
        assert_eq!(Confidence::High.sort_order(), 0);
        assert_eq!(Confidence::Medium.sort_order(), 1);
        assert_eq!(Confidence::Low.sort_order(), 2);
    }

    #[test]
    fn test_confidence_symbol() {
        assert_eq!(Confidence::High.symbol(), "★");
        assert_eq!(Confidence::Medium.symbol(), "◐");
        assert_eq!(Confidence::Low.symbol(), "○");
    }

    // -------------------------------------------------------------------------
    // Scope tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_scope_parse_project() {
        assert_eq!("project".parse::<Scope>().unwrap(), Scope::Project);
    }

    #[test]
    fn test_scope_parse_global() {
        assert_eq!("global".parse::<Scope>().unwrap(), Scope::Global);
    }

    #[test]
    fn test_scope_parse_case_insensitive() {
        assert_eq!("PROJECT".parse::<Scope>().unwrap(), Scope::Project);
        assert_eq!("Global".parse::<Scope>().unwrap(), Scope::Global);
    }

    #[test]
    fn test_scope_parse_invalid() {
        assert!("both".parse::<Scope>().is_err()); // "both" is Tier, not Scope
        assert!("invalid".parse::<Scope>().is_err());
    }

    // -------------------------------------------------------------------------
    // Tier tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_tier_parse_project() {
        assert_eq!("project".parse::<Tier>().unwrap(), Tier::Project);
    }

    #[test]
    fn test_tier_parse_global() {
        assert_eq!("global".parse::<Tier>().unwrap(), Tier::Global);
    }

    #[test]
    fn test_tier_parse_both() {
        assert_eq!("both".parse::<Tier>().unwrap(), Tier::Both);
    }

    #[test]
    fn test_tier_parse_case_insensitive() {
        assert_eq!("BOTH".parse::<Tier>().unwrap(), Tier::Both);
    }

    #[test]
    fn test_tier_parse_invalid() {
        assert!("invalid".parse::<Tier>().is_err());
    }

    // -------------------------------------------------------------------------
    // Memory struct tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_memory_to_summary_short_content() {
        let memory = Memory {
            id: Uuid::new_v4(),
            memory_type: MemoryType::Learning,
            scope: Scope::Project,
            project_path: Some("/test".to_string()),
            content: "Short content".to_string(),
            tags: vec!["tag1".to_string()],
            confidence: Confidence::High,
            source_session_id: None,
            source_turn_id: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            accessed_at: None,
            access_count: 0,
        };

        let summary = memory.to_summary();
        assert_eq!(summary.summary, "Short content");
        assert_eq!(summary.memory_type, MemoryType::Learning);
        assert_eq!(summary.confidence, Confidence::High);
    }

    #[test]
    fn test_memory_to_summary_long_content_truncated() {
        let long_content = "x".repeat(150);
        let memory = Memory {
            id: Uuid::new_v4(),
            memory_type: MemoryType::Gotcha,
            scope: Scope::Global,
            project_path: None,
            content: long_content,
            tags: vec![],
            confidence: Confidence::Medium,
            source_session_id: None,
            source_turn_id: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            accessed_at: None,
            access_count: 5,
        };

        let summary = memory.to_summary();
        assert_eq!(summary.summary.len(), 100); // 97 chars + "..."
        assert!(summary.summary.ends_with("..."));
    }

    #[test]
    fn test_memory_json_serialization() {
        let memory = Memory {
            id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap(),
            memory_type: MemoryType::Api,
            scope: Scope::Project,
            project_path: Some("/test/project".to_string()),
            content: "API quirk discovered".to_string(),
            tags: vec!["api".to_string(), "quirk".to_string()],
            confidence: Confidence::High,
            source_session_id: None,
            source_turn_id: None,
            created_at: DateTime::parse_from_rfc3339("2024-01-15T10:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
            updated_at: DateTime::parse_from_rfc3339("2024-01-15T10:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
            accessed_at: None,
            access_count: 0,
        };

        let json = serde_json::to_string(&memory).unwrap();
        assert!(json.contains("\"type\":\"api\"")); // renamed field
        assert!(json.contains("\"projectPath\"")); // camelCase conversion
        assert!(json.contains("\"accessCount\":0")); // camelCase conversion
    }
}
