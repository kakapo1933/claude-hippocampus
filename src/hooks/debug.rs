//! Shared debug logging utilities for hooks.
//!
//! Provides consistent logging across all hook handlers with per-hook log files.

use chrono::Utc;
use std::fs::OpenOptions;
use std::io::Write;

/// Enable/disable debug logging globally
pub const DEBUG: bool = true;

/// Get log file path for a specific hook
pub fn get_log_path(hook_name: &str) -> String {
    format!("/tmp/hippocampus-{}-hook.log", hook_name)
}

/// Debug logging with hook name prefix
///
/// Writes timestamped log entries to hook-specific log files.
/// Each hook gets its own log file at `/tmp/hippocampus-{hook_name}-hook.log`
pub fn debug(hook_name: &str, msg: &str) {
    if !DEBUG {
        return;
    }
    let timestamp = Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ");
    let line = format!("[{}] [{}] {}\n", timestamp, hook_name, msg);
    let log_path = get_log_path(hook_name);
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&log_path) {
        let _ = file.write_all(line.as_bytes());
    }
}

/// Create a hook-specific logger macro
#[macro_export]
macro_rules! hook_debug {
    ($hook:expr, $($arg:tt)*) => {
        $crate::hooks::debug::debug($hook, &format!($($arg)*))
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_get_log_path() {
        assert_eq!(get_log_path("stop"), "/tmp/hippocampus-stop-hook.log");
        assert_eq!(get_log_path("session-start"), "/tmp/hippocampus-session-start-hook.log");
        assert_eq!(get_log_path("session-end"), "/tmp/hippocampus-session-end-hook.log");
    }

    #[test]
    fn test_debug_writes_to_file() {
        let hook_name = "test-debug";
        let log_path = get_log_path(hook_name);

        // Clean up any existing file
        let _ = fs::remove_file(&log_path);

        debug(hook_name, "test message");

        // Verify file was created and contains the message
        let content = fs::read_to_string(&log_path).unwrap();
        assert!(content.contains("test message"));
        assert!(content.contains("[test-debug]"));

        // Clean up
        let _ = fs::remove_file(&log_path);
    }

    #[test]
    fn test_debug_includes_timestamp() {
        let hook_name = "test-timestamp";
        let log_path = get_log_path(hook_name);

        let _ = fs::remove_file(&log_path);

        debug(hook_name, "timestamp test");

        let content = fs::read_to_string(&log_path).unwrap();
        // Check for ISO timestamp format
        assert!(content.contains("T"));
        assert!(content.contains("Z"));

        let _ = fs::remove_file(&log_path);
    }
}
