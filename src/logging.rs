//! File-based logging for memory operations.
//!
//! Logs to `~/.claude/logs/memory.log` with 1MB rotation.

use crate::error::{HippocampusError, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::PathBuf;

const LOG_FILE_NAME: &str = "memory.log";
const LOG_DIR_NAME: &str = "logs";
const MAX_LOG_SIZE: u64 = 1_048_576; // 1MB

/// A single log entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: DateTime<Utc>,
    pub operation: String,
    pub details: Option<String>,
    pub success: bool,
}

impl LogEntry {
    /// Create a new log entry.
    pub fn new(operation: impl Into<String>, details: Option<String>, success: bool) -> Self {
        Self {
            timestamp: Utc::now(),
            operation: operation.into(),
            details,
            success,
        }
    }

    /// Format as a single log line.
    pub fn to_log_line(&self) -> String {
        let status = if self.success { "OK" } else { "ERR" };
        let details = self.details.as_deref().unwrap_or("-");
        format!(
            "[{}] {} {} {}",
            self.timestamp.format("%Y-%m-%d %H:%M:%S"),
            status,
            self.operation,
            details
        )
    }

    /// Parse from a log line.
    pub fn from_log_line(line: &str) -> Option<Self> {
        // Format: [2024-01-24 10:30:45] OK addMemory some details
        if !line.starts_with('[') {
            return None;
        }

        let timestamp_end = line.find(']')?;
        let timestamp_str = &line[1..timestamp_end];

        let rest = line[timestamp_end + 2..].trim();
        let parts: Vec<&str> = rest.splitn(3, ' ').collect();
        if parts.len() < 2 {
            return None;
        }

        let success = parts[0] == "OK";
        let operation = parts[1].to_string();
        let details = parts.get(2).map(|s| s.to_string()).filter(|s| s != "-");

        // Parse timestamp
        let timestamp = chrono::NaiveDateTime::parse_from_str(timestamp_str, "%Y-%m-%d %H:%M:%S")
            .ok()
            .map(|dt| dt.and_utc())?;

        Some(Self {
            timestamp,
            operation,
            details,
            success,
        })
    }
}

/// Get the log file path.
pub fn get_log_path() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| {
        HippocampusError::Config("Could not determine home directory".to_string())
    })?;

    let log_dir = home.join(".claude").join(LOG_DIR_NAME);

    // Ensure log directory exists
    if !log_dir.exists() {
        fs::create_dir_all(&log_dir)?;
    }

    Ok(log_dir.join(LOG_FILE_NAME))
}

/// Check if log file needs rotation.
fn needs_rotation(path: &PathBuf) -> bool {
    if let Ok(metadata) = fs::metadata(path) {
        metadata.len() >= MAX_LOG_SIZE
    } else {
        false
    }
}

/// Rotate log file (rename to .old, start fresh).
fn rotate_log(path: &PathBuf) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }

    let old_path = path.with_extension("log.old");

    // Remove old backup if exists
    if old_path.exists() {
        fs::remove_file(&old_path)?;
    }

    // Rename current to .old
    fs::rename(path, &old_path)?;

    Ok(())
}

/// Write a log entry.
pub fn log(operation: impl Into<String>, details: Option<String>, success: bool) -> Result<()> {
    let path = get_log_path()?;

    // Rotate if needed
    if needs_rotation(&path) {
        rotate_log(&path)?;
    }

    let entry = LogEntry::new(operation, details, success);
    let line = entry.to_log_line();

    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?;

    let mut writer = BufWriter::new(file);
    writeln!(writer, "{}", line)?;
    writer.flush()?;

    Ok(())
}

/// Read log entries.
///
/// - `limit`: Maximum number of entries to return (most recent first)
/// - `operation`: Optional filter by operation name
pub fn read_logs(limit: usize, operation: Option<&str>) -> Result<Vec<LogEntry>> {
    let path = get_log_path()?;

    if !path.exists() {
        return Ok(Vec::new());
    }

    let file = File::open(&path)?;
    let reader = BufReader::new(file);

    let mut entries: Vec<LogEntry> = reader
        .lines()
        .filter_map(|line| line.ok())
        .filter_map(|line| LogEntry::from_log_line(&line))
        .filter(|entry| {
            operation.map_or(true, |op| {
                entry.operation.eq_ignore_ascii_case(op)
            })
        })
        .collect();

    // Return most recent first
    entries.reverse();
    entries.truncate(limit);

    Ok(entries)
}

/// Clear all logs.
pub fn clear_logs() -> Result<usize> {
    let path = get_log_path()?;

    if !path.exists() {
        return Ok(0);
    }

    // Count lines before clearing
    let file = File::open(&path)?;
    let reader = BufReader::new(file);
    let count = reader.lines().count();

    // Clear the file
    File::create(&path)?;

    // Also remove old backup
    let old_path = path.with_extension("log.old");
    if old_path.exists() {
        fs::remove_file(&old_path)?;
    }

    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests use the actual log path (~/.claude/logs/memory.log)
    // In a real test environment, you'd want to mock the path

    #[test]
    fn test_log_entry_new() {
        let entry = LogEntry::new("addMemory", Some("test details".to_string()), true);

        assert_eq!(entry.operation, "addMemory");
        assert_eq!(entry.details, Some("test details".to_string()));
        assert!(entry.success);
    }

    #[test]
    fn test_log_entry_to_log_line_success() {
        let entry = LogEntry {
            timestamp: chrono::DateTime::parse_from_rfc3339("2024-01-24T10:30:45Z")
                .unwrap()
                .with_timezone(&Utc),
            operation: "addMemory".to_string(),
            details: Some("created memory".to_string()),
            success: true,
        };

        let line = entry.to_log_line();
        assert_eq!(line, "[2024-01-24 10:30:45] OK addMemory created memory");
    }

    #[test]
    fn test_log_entry_to_log_line_failure() {
        let entry = LogEntry {
            timestamp: chrono::DateTime::parse_from_rfc3339("2024-01-24T10:30:45Z")
                .unwrap()
                .with_timezone(&Utc),
            operation: "deleteMemory".to_string(),
            details: Some("not found".to_string()),
            success: false,
        };

        let line = entry.to_log_line();
        assert_eq!(line, "[2024-01-24 10:30:45] ERR deleteMemory not found");
    }

    #[test]
    fn test_log_entry_to_log_line_no_details() {
        let entry = LogEntry {
            timestamp: chrono::DateTime::parse_from_rfc3339("2024-01-24T10:30:45Z")
                .unwrap()
                .with_timezone(&Utc),
            operation: "consolidate".to_string(),
            details: None,
            success: true,
        };

        let line = entry.to_log_line();
        assert_eq!(line, "[2024-01-24 10:30:45] OK consolidate -");
    }

    #[test]
    fn test_log_entry_from_log_line_success() {
        let line = "[2024-01-24 10:30:45] OK addMemory created memory";
        let entry = LogEntry::from_log_line(line).unwrap();

        assert_eq!(entry.operation, "addMemory");
        assert_eq!(entry.details, Some("created memory".to_string()));
        assert!(entry.success);
    }

    #[test]
    fn test_log_entry_from_log_line_failure() {
        let line = "[2024-01-24 10:30:45] ERR deleteMemory not found";
        let entry = LogEntry::from_log_line(line).unwrap();

        assert_eq!(entry.operation, "deleteMemory");
        assert_eq!(entry.details, Some("not found".to_string()));
        assert!(!entry.success);
    }

    #[test]
    fn test_log_entry_from_log_line_no_details() {
        let line = "[2024-01-24 10:30:45] OK consolidate -";
        let entry = LogEntry::from_log_line(line).unwrap();

        assert_eq!(entry.operation, "consolidate");
        assert_eq!(entry.details, None);
        assert!(entry.success);
    }

    #[test]
    fn test_log_entry_from_log_line_invalid() {
        assert!(LogEntry::from_log_line("invalid line").is_none());
        assert!(LogEntry::from_log_line("").is_none());
        assert!(LogEntry::from_log_line("[incomplete").is_none());
    }

    #[test]
    fn test_log_entry_roundtrip() {
        let original = LogEntry {
            timestamp: chrono::DateTime::parse_from_rfc3339("2024-01-24T10:30:45Z")
                .unwrap()
                .with_timezone(&Utc),
            operation: "searchKeyword".to_string(),
            details: Some("query=test".to_string()),
            success: true,
        };

        let line = original.to_log_line();
        let parsed = LogEntry::from_log_line(&line).unwrap();

        assert_eq!(parsed.operation, original.operation);
        assert_eq!(parsed.details, original.details);
        assert_eq!(parsed.success, original.success);
        // Timestamps should match (within second precision)
        assert_eq!(
            parsed.timestamp.format("%Y-%m-%d %H:%M:%S").to_string(),
            original.timestamp.format("%Y-%m-%d %H:%M:%S").to_string()
        );
    }

    #[test]
    fn test_get_log_path() {
        let path = get_log_path().unwrap();

        assert!(path.to_string_lossy().contains(".claude"));
        assert!(path.to_string_lossy().contains("logs"));
        assert!(path.to_string_lossy().ends_with("memory.log"));
    }

    #[test]
    fn test_needs_rotation_nonexistent() {
        let path = PathBuf::from("/tmp/nonexistent-log-file-xyz.log");
        assert!(!needs_rotation(&path));
    }

    // Integration test - writes to actual log file
    #[test]
    fn test_log_and_read() {
        // Write some entries
        log("test_operation", Some("test details".to_string()), true).unwrap();

        // Read back
        let entries = read_logs(10, Some("test_operation")).unwrap();

        // Should have at least one entry
        assert!(!entries.is_empty());

        let entry = &entries[0];
        assert_eq!(entry.operation, "test_operation");
    }

    #[test]
    fn test_read_logs_with_limit() {
        // Write multiple entries
        for i in 0..5 {
            log(
                "limit_test",
                Some(format!("entry {}", i)),
                true,
            )
            .unwrap();
        }

        // Read with limit
        let entries = read_logs(3, Some("limit_test")).unwrap();

        // Should respect limit
        assert!(entries.len() <= 3);

        // Should be most recent first
        if entries.len() >= 2 {
            assert!(entries[0].timestamp >= entries[1].timestamp);
        }
    }

    #[test]
    fn test_read_logs_empty_filter() {
        let entries = read_logs(10, Some("nonexistent_operation_xyz")).unwrap();
        // May return empty if no matching entries
        assert!(entries.is_empty() || entries.iter().all(|e| e.operation == "nonexistent_operation_xyz"));
    }
}
