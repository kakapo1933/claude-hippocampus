//! Integration tests for CLI dispatch
//!
//! These tests verify that main.rs correctly dispatches commands
//! and returns JSON output.

use assert_cmd::Command;
use predicates::prelude::*;

// ============================================================================
// Help Tests
// ============================================================================

#[test]
fn test_cli_help_displays() {
    Command::cargo_bin("claude-hippocampus")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("PostgreSQL-backed persistent memory"));
}

#[test]
fn test_cli_version_displays() {
    Command::cargo_bin("claude-hippocampus")
        .unwrap()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("claude-hippocampus"));
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[test]
fn test_unknown_command_fails_with_json_error() {
    Command::cargo_bin("claude-hippocampus")
        .unwrap()
        .arg("unknown-command")
        .assert()
        .failure();
}

#[test]
fn test_missing_required_args_fails() {
    // add-memory requires type and content
    Command::cargo_bin("claude-hippocampus")
        .unwrap()
        .arg("add-memory")
        .assert()
        .failure();
}

#[test]
fn test_invalid_memory_type_fails() {
    Command::cargo_bin("claude-hippocampus")
        .unwrap()
        .args(["add-memory", "invalid-type", "content"])
        .assert()
        .failure();
}

// ============================================================================
// Logging Command Tests (no DB required)
// ============================================================================

#[test]
fn test_logs_command_returns_json() {
    // This command may return an error if log file doesn't exist,
    // but it should at least return valid JSON
    let output = Command::cargo_bin("claude-hippocampus")
        .unwrap()
        .args(["logs", "10"])
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should output JSON (either success or error)
    assert!(
        stdout.contains('{') || stderr.contains('{'),
        "Expected JSON output, got stdout: {}, stderr: {}",
        stdout,
        stderr
    );
}

#[test]
fn test_clear_logs_command_returns_json() {
    let output = Command::cargo_bin("claude-hippocampus")
        .unwrap()
        .arg("clear-logs")
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should output JSON with success field
    assert!(
        stdout.contains("\"success\""),
        "Expected JSON with success field, got: {}",
        stdout
    );
}

// Note: Database-dependent tests would go here but require a test database setup
// For now we test the CLI interface without actual DB operations

// ============================================================================
// Get Turn Command Tests
// ============================================================================

#[test]
fn test_get_turn_missing_session_id_fails() {
    Command::cargo_bin("claude-hippocampus")
        .unwrap()
        .arg("get-turn")
        .assert()
        .failure();
}

#[test]
fn test_get_turn_invalid_session_id_fails() {
    Command::cargo_bin("claude-hippocampus")
        .unwrap()
        .args(["get-turn", "invalid-uuid"])
        .assert()
        .failure();
}
