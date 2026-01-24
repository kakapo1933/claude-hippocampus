pub mod cli;
pub mod commands;
pub mod config;
pub mod db;
pub mod error;
pub mod git;
pub mod hooks;
pub mod logging;
pub mod models;
pub mod session;

pub use cli::{parse_tags, Cli, Command, HookType};
pub use config::DbConfig;
pub use error::{HippocampusError, Result};
pub use logging::{clear_logs, log, read_logs, LogEntry};
pub use session::{
    clear_session_state, get_session_state_path, load_session_state, save_session_state,
    SessionState,
};
pub use git::{get_git_status, GitStatus};
pub use hooks::{
    handle_session_end, handle_session_start, handle_stop, handle_user_prompt_submit,
    HookInput, HookOutput,
};
