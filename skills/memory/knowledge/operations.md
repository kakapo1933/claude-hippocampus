---
topic: Memory CLI Operations
keywords: [add, update, delete, search, memory, crud, postgresql, cli]
---

# Memory Operations

All operations use the `claude-hippocampus` CLI with PostgreSQL backend.

## Adding Memories

```bash
claude-hippocampus add-memory <type> "<content>" "<tags>" <confidence> <scope>
```

**Parameters:**
- `type`: convention | architecture | gotcha | api | learning | preference
- `content`: The learning text (quote if contains spaces)
- `tags`: Comma-separated tags for search
- `confidence`: high | medium | low
- `scope`: project | global

**Example:**
```bash
claude-hippocampus add-memory gotcha "FAL.ai requires rawUrl not url for external APIs" "api,fal,video" high project
```

**Features:**
- Automatic duplicate detection (first 100 chars match)
- Links to source session/turn when `--claude-session` provided

---

## Searching Memories

### By Keyword
```bash
claude-hippocampus search-keyword "<query>" [tier] [limit]
```

**Parameters:**
- `query`: Keyword to search for
- `tier`: project | global | both (default: both)
- `limit`: Maximum results (default: 30)

### By Type
```bash
claude-hippocampus search-by-type <type> [keyword] [tier] [limit]
```

Filter by memory type with optional keyword.

---

## Getting Context

```bash
claude-hippocampus get-context [limit]
```

Returns formatted markdown context block for injection.
Sorted by: confidence → access_count → created_at

---

## Updating Memories

```bash
claude-hippocampus update-memory <id> "<content>" [tier]
```

Updates content and timestamps.

---

## Deleting Memories

```bash
claude-hippocampus delete-memory <id> [tier]
```

---

## Maintenance

### Consolidate
Merge exact duplicate entries:
```bash
claude-hippocampus consolidate [tier]
```

### Prune
Remove old low-confidence entries:
```bash
claude-hippocampus prune [days] [tier]
```
Default: 90 days, removes confidence='low' with access_count=0

### List Recent
```bash
claude-hippocampus list-recent [n] [tier]
```

### Statistics
```bash
claude-hippocampus stats
```

---

## Session Management

```bash
# Create session
claude-hippocampus create-session <claude-id> <project-path> [git-status]

# End session with summary
claude-hippocampus end-session <id> [summary]

# Create conversation turn
claude-hippocampus create-turn <session-id> <turn-number> "<user-prompt>"

# Update turn with response
claude-hippocampus update-turn <turn-id> "<response>" [tokens]
```

---

## Database Schema

```
PostgreSQL Database: claude_memory
├── memories          # All memory entries
│   ├── id           # UUID primary key
│   ├── type         # Memory type
│   ├── scope        # 'global' or 'project'
│   ├── project_path # For project-scoped entries
│   ├── content      # Full content
│   ├── tags         # Array of tags
│   ├── confidence   # high/medium/low
│   └── timestamps   # created_at, updated_at, accessed_at
├── sessions         # Session tracking
├── conversation_turns
└── tool_calls
```
