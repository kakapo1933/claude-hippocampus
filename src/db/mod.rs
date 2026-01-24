pub mod pool;
pub mod queries;

pub use pool::{create_pool, create_pool_with_password, get_project_path};
pub use queries::{
    consolidate_duplicates, delete_memory, find_duplicate, get_context_memories, get_memory,
    insert_memory, list_recent, prune_old_memories, save_session_summary, search_keyword,
    update_memory, DuplicateInfo,
    // Session queries
    create_session, end_session, find_session_by_claude_id, find_session_by_id,
    // Turn queries
    create_turn, get_latest_turn, get_next_turn_number, get_turn, list_turns_by_session,
    update_turn,
};
