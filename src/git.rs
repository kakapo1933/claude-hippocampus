//! Git integration module for capturing repository status
//!
//! Provides functionality to capture git status including branch name,
//! modified files, untracked files, and staged files.

use std::process::Command;

use serde::{Deserialize, Serialize};

use crate::error::{HippocampusError, Result};

/// Represents the current state of a git repository
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct GitStatus {
    /// Current branch name (e.g., "main", "feature/foo")
    pub branch: String,
    /// Files that have been modified but not staged
    pub modified: Vec<String>,
    /// Files that are not tracked by git
    pub untracked: Vec<String>,
    /// Files that have been staged for commit
    pub staged: Vec<String>,
}

/// Parse `git status --porcelain` output into a GitStatus struct
///
/// Porcelain format uses two-character status codes:
/// - XY where X is staged status, Y is worktree status
/// - " M" = modified in worktree only
/// - "M " = modified and staged
/// - "MM" = staged with additional worktree changes
/// - "A " = new file added to staging
/// - "D " = deleted and staged
/// - "R " = renamed and staged
/// - "??" = untracked
fn parse_porcelain(output: &str) -> GitStatus {
    let mut status = GitStatus::default();

    for line in output.lines() {
        if line.len() < 3 {
            continue;
        }

        let staged_code = line.chars().next().unwrap_or(' ');
        let worktree_code = line.chars().nth(1).unwrap_or(' ');
        let filename = line[3..].to_string();

        // Check for untracked files first
        if staged_code == '?' && worktree_code == '?' {
            status.untracked.push(filename);
            continue;
        }

        // Check staged changes (first character)
        if staged_code != ' ' && staged_code != '?' {
            status.staged.push(filename.clone());
        }

        // Check worktree modifications (second character)
        if worktree_code == 'M' {
            status.modified.push(filename);
        }
    }

    status
}

/// Get the git status for a given directory path
///
/// Returns:
/// - `Ok(Some(GitStatus))` if the path is inside a git repository
/// - `Ok(None)` if the path is not a git repository
/// - `Err(_)` if there's an IO error or the path doesn't exist
pub fn get_git_status(path: &str) -> Result<Option<GitStatus>> {
    // Check if we're in a git repo by running git rev-parse
    let rev_parse = Command::new("git")
        .args(["rev-parse", "--is-inside-work-tree"])
        .current_dir(path)
        .output();

    let output = match rev_parse {
        Ok(o) => o,
        Err(e) => {
            // If the directory doesn't exist, return None
            if e.kind() == std::io::ErrorKind::NotFound {
                return Ok(None);
            }
            return Err(HippocampusError::Io(e));
        }
    };

    // Not a git repo
    if !output.status.success() {
        return Ok(None);
    }

    // Get branch name
    let branch_output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(path)
        .output()
        .map_err(HippocampusError::Io)?;

    let branch = String::from_utf8_lossy(&branch_output.stdout)
        .trim()
        .to_string();

    // Get status in porcelain format
    let status_output = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(path)
        .output()
        .map_err(HippocampusError::Io)?;

    let porcelain = String::from_utf8_lossy(&status_output.stdout);
    let mut status = parse_porcelain(&porcelain);
    status.branch = branch;

    Ok(Some(status))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_git_status_struct_creation() {
        let status = GitStatus {
            branch: "main".to_string(),
            modified: vec!["file1.rs".to_string()],
            untracked: vec!["new_file.rs".to_string()],
            staged: vec!["ready.rs".to_string()],
        };

        assert_eq!(status.branch, "main");
        assert_eq!(status.modified, vec!["file1.rs"]);
        assert_eq!(status.untracked, vec!["new_file.rs"]);
        assert_eq!(status.staged, vec!["ready.rs"]);
    }

    #[test]
    fn test_git_status_default() {
        let status = GitStatus::default();

        assert_eq!(status.branch, "");
        assert!(status.modified.is_empty());
        assert!(status.untracked.is_empty());
        assert!(status.staged.is_empty());
    }

    // ========================================================================
    // Porcelain parsing tests
    // ========================================================================

    #[test]
    fn test_parse_porcelain_empty() {
        let status = parse_porcelain("");
        assert!(status.modified.is_empty());
        assert!(status.untracked.is_empty());
        assert!(status.staged.is_empty());
    }

    #[test]
    fn test_parse_porcelain_modified_file() {
        // " M" = modified in worktree, not staged
        let status = parse_porcelain(" M src/main.rs");
        assert_eq!(status.modified, vec!["src/main.rs"]);
        assert!(status.staged.is_empty());
        assert!(status.untracked.is_empty());
    }

    #[test]
    fn test_parse_porcelain_staged_file() {
        // "M " = modified and staged
        let status = parse_porcelain("M  src/lib.rs");
        assert_eq!(status.staged, vec!["src/lib.rs"]);
        assert!(status.modified.is_empty());
        assert!(status.untracked.is_empty());
    }

    #[test]
    fn test_parse_porcelain_untracked_file() {
        // "??" = untracked
        let status = parse_porcelain("?? new_file.rs");
        assert_eq!(status.untracked, vec!["new_file.rs"]);
        assert!(status.modified.is_empty());
        assert!(status.staged.is_empty());
    }

    #[test]
    fn test_parse_porcelain_added_file() {
        // "A " = new file added to staging
        let status = parse_porcelain("A  brand_new.rs");
        assert_eq!(status.staged, vec!["brand_new.rs"]);
        assert!(status.modified.is_empty());
        assert!(status.untracked.is_empty());
    }

    #[test]
    fn test_parse_porcelain_deleted_file() {
        // "D " = deleted and staged
        let status = parse_porcelain("D  removed.rs");
        assert_eq!(status.staged, vec!["removed.rs"]);
    }

    #[test]
    fn test_parse_porcelain_mixed_status() {
        let output = " M modified.rs\n\
                      M  staged.rs\n\
                      ?? untracked.rs\n\
                      A  added.rs";
        let status = parse_porcelain(output);

        assert_eq!(status.modified, vec!["modified.rs"]);
        assert_eq!(status.staged, vec!["staged.rs", "added.rs"]);
        assert_eq!(status.untracked, vec!["untracked.rs"]);
    }

    #[test]
    fn test_parse_porcelain_renamed_file() {
        // "R " = renamed and staged, format: "R  new_name -> old_name"
        let status = parse_porcelain("R  new_name.rs -> old_name.rs");
        assert_eq!(status.staged, vec!["new_name.rs -> old_name.rs"]);
    }

    #[test]
    fn test_parse_porcelain_modified_and_staged() {
        // "MM" = staged AND has unstaged changes
        let status = parse_porcelain("MM both.rs");
        assert_eq!(status.staged, vec!["both.rs"]);
        assert_eq!(status.modified, vec!["both.rs"]);
    }

    // ========================================================================
    // get_git_status integration tests
    // ========================================================================

    #[test]
    fn test_get_git_status_in_git_repo() {
        // Create a temp directory and init a git repo in it
        use std::process::Command;

        let temp_dir = std::env::temp_dir().join("claude-hippocampus-git-test");
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();

        // Initialize git repo
        Command::new("git")
            .args(["init"])
            .current_dir(&temp_dir)
            .output()
            .expect("Failed to init git repo");

        // Configure git user for the test repo
        Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(&temp_dir)
            .output()
            .expect("Failed to set git email");
        Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(&temp_dir)
            .output()
            .expect("Failed to set git name");

        // Create an initial commit so HEAD exists
        std::fs::write(temp_dir.join("test.txt"), "test").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(&temp_dir)
            .output()
            .expect("Failed to add file");
        Command::new("git")
            .args(["commit", "-m", "Initial commit"])
            .current_dir(&temp_dir)
            .output()
            .expect("Failed to commit");

        let result = get_git_status(temp_dir.to_str().unwrap()).unwrap();
        assert!(result.is_some(), "Should return Some in a git repo");

        let status = result.unwrap();
        // The repo should have a branch name (main or master)
        assert!(!status.branch.is_empty(), "Branch name should not be empty");

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_get_git_status_non_git_directory() {
        // /tmp is typically not a git repo
        let result = get_git_status("/tmp").unwrap();
        assert!(result.is_none(), "Should return None for non-git directory");
    }

    #[test]
    fn test_get_git_status_nonexistent_path() {
        let result = get_git_status("/nonexistent/path/that/does/not/exist");
        // Should return Ok(None) or an error - either is acceptable
        // The important thing is it doesn't panic
        match result {
            Ok(None) => {} // Expected for nonexistent path
            Ok(Some(_)) => panic!("Should not return status for nonexistent path"),
            Err(_) => {} // Also acceptable - IO error
        }
    }
}
