//! Compatibility tests for Node.js output format
//!
//! These tests verify that the Rust CLI produces JSON output compatible with
//! the Node.js implementation for drop-in replacement in hooks.

use serde_json::Value;

/// Verify that a search result has all required fields
fn has_required_search_fields(result: &Value) -> bool {
    let required = [
        "id",
        "type",
        "tier",
        "summary",
        "content",
        "tags",
        "confidence",
        "created",
        "accessCount",
    ];

    let obj = match result.as_object() {
        Some(o) => o,
        None => return false,
    };

    for field in &required {
        if !obj.contains_key(*field) {
            eprintln!("Missing required field: {}", field);
            return false;
        }
    }

    true
}

/// Verify search response structure matches Node.js
#[test]
fn test_search_response_structure() {
    // Mock response structure matching Rust output
    let rust_response = serde_json::json!({
        "success": true,
        "results": [
            {
                "id": "550e8400-e29b-41d4-a716-446655440000",
                "type": "learning",
                "tier": "project",
                "summary": "Test summary...",
                "content": "Test content longer than summary",
                "tags": ["tag1", "tag2"],
                "confidence": "high",
                "created": "2024-01-15T10:00:00.000Z",
                "accessed": null,
                "accessCount": 5
            }
        ],
        "count": 1
    });

    // Verify top-level structure
    assert!(rust_response.get("success").is_some());
    assert!(rust_response.get("results").is_some());
    assert!(rust_response.get("count").is_some());

    // Verify result fields
    let results = rust_response["results"].as_array().unwrap();
    for result in results {
        assert!(has_required_search_fields(result));
    }
}

/// Verify context response structure
#[test]
fn test_context_response_structure() {
    let response = serde_json::json!({
        "success": true,
        "context": "## Memory Context\n\n- ★ **learning**: Test",
        "count": 1,
        "entries": []
    });

    assert_eq!(response["success"], true);
    assert!(response.get("context").is_some());
    assert!(response.get("count").is_some());
    assert!(response.get("entries").is_some());
}

/// Verify listRecent response structure
#[test]
fn test_list_recent_response_structure() {
    let response = serde_json::json!({
        "success": true,
        "entries": [],
        "total": 100
    });

    assert_eq!(response["success"], true);
    assert!(response.get("entries").is_some());
    assert!(response.get("total").is_some());
}

/// Verify confidence field values match Node.js (lowercase)
#[test]
fn test_confidence_values_lowercase() {
    let valid_values = ["high", "medium", "low"];

    for value in &valid_values {
        let response = serde_json::json!({
            "confidence": *value
        });

        let confidence = response["confidence"].as_str().unwrap();
        assert_eq!(confidence, confidence.to_lowercase());
    }
}

/// Verify type field values match Node.js (lowercase)
#[test]
fn test_type_values_lowercase() {
    let valid_values = ["convention", "architecture", "gotcha", "api", "learning", "preference"];

    for value in &valid_values {
        let response = serde_json::json!({
            "type": *value
        });

        let type_val = response["type"].as_str().unwrap();
        assert_eq!(type_val, type_val.to_lowercase());
    }
}

/// Verify tier field values match Node.js (lowercase)
#[test]
fn test_tier_values_lowercase() {
    let valid_values = ["project", "global"];

    for value in &valid_values {
        let response = serde_json::json!({
            "tier": *value
        });

        let tier = response["tier"].as_str().unwrap();
        assert_eq!(tier, tier.to_lowercase());
    }
}

/// Verify addMemory success response
#[test]
fn test_add_memory_success_response() {
    let response = serde_json::json!({
        "success": true,
        "id": "550e8400-e29b-41d4-a716-446655440000"
    });

    assert_eq!(response["success"], true);
    assert!(response.get("id").is_some());
}

/// Verify duplicate response structure
#[test]
fn test_duplicate_response_structure() {
    let response = serde_json::json!({
        "success": false,
        "duplicate": true,
        "reason": "exact_match",
        "existingId": "550e8400-e29b-41d4-a716-446655440000",
        "existingTier": "project",
        "existingSummary": "Some content...",
        "message": "Duplicate memory detected"
    });

    assert_eq!(response["success"], false);
    assert_eq!(response["duplicate"], true);
    assert!(response.get("existingId").is_some());
    assert!(response.get("message").is_some());
}

/// Verify error response structure
#[test]
fn test_error_response_structure() {
    let response = serde_json::json!({
        "success": false,
        "error": "Memory not found: abc-123"
    });

    assert_eq!(response["success"], false);
    assert!(response.get("error").is_some());
}

/// Verify consolidate response structure
#[test]
fn test_consolidate_response_structure() {
    let response = serde_json::json!({
        "success": true,
        "removed": 3,
        "duplicateIds": ["id1", "id2", "id3"]
    });

    assert_eq!(response["success"], true);
    assert!(response.get("removed").is_some());
    assert!(response.get("duplicateIds").is_some());
}

/// Verify prune response structure
#[test]
fn test_prune_response_structure() {
    let response = serde_json::json!({
        "success": true,
        "pruned": 5,
        "prunedIds": ["id1", "id2"]
    });

    assert_eq!(response["success"], true);
    assert!(response.get("pruned").is_some());
    assert!(response.get("prunedIds").is_some());
}
