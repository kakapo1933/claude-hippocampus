pub mod maintenance;
pub mod memory;
pub mod search;
pub mod stats;

pub use maintenance::{consolidate, prune, save_session_summary};
pub use memory::{
    add_memory, delete_memory, get_memory, update_memory, AddMemoryOptions, AddMemoryResult,
};
pub use search::{
    get_context, list_recent, search_by_type, search_keyword, ContextResult, ListRecentResult,
    MemorySearchItem, SearchByTypeOptions, SearchOptions, SearchResult,
};
pub use stats::{get_stats, ConfidenceCounts, MemoryStats, ScopeCounts, StatsOptions, TypeCounts};
