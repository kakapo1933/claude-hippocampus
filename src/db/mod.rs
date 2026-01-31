pub mod pool;
pub mod queries;

pub use pool::{create_pool, create_pool_with_password, get_project_path};
pub use queries::{
    consolidate_duplicates, delete_memory, find_duplicate, get_context_memories, get_memory,
    insert_memory, list_recent, prune_old_memories_tiered, save_session_summary, search_keyword,
    update_memory, DuplicateInfo,
    // Session queries
    create_session, end_session, find_session_by_claude_id, find_session_by_id,
    // Turn queries
    create_turn, find_turn_by_id, get_next_turn_number, update_turn,
    // Supersession queries
    list_superseded, prune_lifecycle_data, purge_superseded, show_chain, supersede_memory,
    ChainResult, LifecyclePruneResult, SupersededMemoryInfo,
};
