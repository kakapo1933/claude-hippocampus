# Claude Hippocampus

A high-performance Rust CLI for persistent memory management in Claude Code sessions.

## Overview

Claude Hippocampus is a native Rust replacement for the Node.js memory system, providing PostgreSQL-backed persistent memory with ~10x faster startup times. It integrates directly with Claude Code via hooks for automatic session tracking, memory extraction, and tool call logging.

## Features

- **Fast**: ~5ms startup vs ~50-100ms for Node.js
- **Persistent**: PostgreSQL-backed storage with project/global scoping
- **Plugin**: Install via `/plugin` command with skill and agent included
- **Hook Integration**: Direct Claude Code settings.json hook support
- **Session Tracking**: Automatic session and turn management with git status capture
- **Tool Call Logging**: Records all tool usage for analysis
- **Compatible**: JSON output matches Node.js for seamless integration
- **Complete**: All memory operations (CRUD, search, maintenance)
- **Tested**: 290 tests (unit + integration)

## Installation

### Option 1: Claude Code Plugin (Recommended)

Install directly in Claude Code:

```
/plugin https://github.com/kakapo1933/claude-hippocampus
```

This installs:
- **CLI**: `claude-hippocampus` command
- **Skill**: `/brain-cells` - memory operations guidance
- **Agent**: `hippocampus:memory-manager` - autonomous memory management

### Option 2: Build from Source

#### Prerequisites

- Rust 1.70+ (`rustup install stable`)
- PostgreSQL with the memory schema (see [schema setup](#database-setup))

#### Build

```bash
# Clone the repository
git clone https://github.com/kakapo1933/claude-hippocampus.git
cd claude-hippocampus

# Build release binary
cargo build --release

# Install to PATH (optional)
mkdir -p ~/.claude/bin
cp target/release/claude-hippocampus ~/.claude/bin/
```

## Usage

### Memory Commands

```bash
# Add a memory
claude-hippocampus add-memory learning "API requires auth header" "api,auth" high project

# Search memories
claude-hippocampus search-keyword "auth" both 10

# Get context block for prompt injection
claude-hippocampus get-context 10

# List recent memories
claude-hippocampus list-recent 5 both

# Update a memory
claude-hippocampus update-memory <uuid> "Updated content" project

# Delete a memory
claude-hippocampus delete-memory <uuid>

# Get a specific memory
claude-hippocampus get-memory <uuid>

# Maintenance
claude-hippocampus consolidate project  # Remove duplicates
claude-hippocampus prune --low-days=30 --medium-days=90 project  # Tiered retention

# Supersession management
claude-hippocampus add-memory learning "New info" --supersedes=<old-id>  # Replace memory
claude-hippocampus show-chain <memory-id>         # Show supersession chain
claude-hippocampus list-superseded both 50        # List inactive memories
claude-hippocampus purge-superseded 30 project    # Delete old superseded

# Lifecycle data cleanup
claude-hippocampus prune-data --tool-calls-days=14 --turns-days=30 --sessions-days=90
claude-hippocampus prune-data --dry-run           # Preview what would be deleted

# View logs
claude-hippocampus logs 50
claude-hippocampus clear-logs
```

### Session Management

```bash
# Create a new session (captures git status automatically)
claude-hippocampus create-session --claude-session-id=abc-123-def

# Get session by ID (UUID or claude_session_id)
claude-hippocampus get-session abc-123-def

# End a session with optional summary
claude-hippocampus end-session abc-123-def --summary="Implemented feature X"
```

### Turn Management

```bash
# Create a conversation turn
claude-hippocampus create-turn --session=abc-123-def --prompt="How do I..."

# Update turn with response
claude-hippocampus update-turn --turn-id=<uuid> --response="Here's how..." \
  --input-tokens=100 --output-tokens=250
```

### Hook Commands (Claude Code Integration)

Hooks integrate directly with Claude Code's `settings.json`:

```bash
# Session start - creates session, loads context
claude-hippocampus hook session-start

# User prompt submit - creates turn, outputs memory search instructions
claude-hippocampus hook user-prompt-submit

# Stop - runs after each response (memory extraction)
claude-hippocampus hook stop

# Post tool use - records tool calls to database
claude-hippocampus hook post-tool-use

# Session end - marks session complete
claude-hippocampus hook session-end
```

All hooks read JSON from stdin and output JSON with `decision` and optional `reason` fields.

### Memory Types

| Type | Description |
|------|-------------|
| `convention` | Coding standards and patterns |
| `architecture` | System design decisions |
| `gotcha` | Pitfalls and warnings |
| `api` | API quirks and usage |
| `learning` | General learnings |
| `preference` | User preferences |

### Confidence Levels

| Level | Symbol | Description | Retention |
|-------|--------|-------------|-----------|
| `high` | ★ | Verified, critical information | Never pruned |
| `medium` | ◐ | Likely correct, moderate importance | Pruned after 90 days (if never accessed) |
| `low` | ○ | Uncertain or low priority | Pruned after 30 days (if never accessed) |

### Tiers

| Tier | Scope |
|------|-------|
| `project` | Current project only |
| `global` | All projects |
| `both` | Search both (default) |

## Claude Code Integration

Add hooks to your `~/.claude/settings.json`:

```json
{
  "hooks": {
    "SessionStart": [
      {
        "type": "command",
        "command": "~/.claude/bin/claude-hippocampus hook session-start"
      }
    ],
    "UserPromptSubmit": [
      {
        "type": "command",
        "command": "~/.claude/bin/claude-hippocampus hook user-prompt-submit"
      }
    ],
    "Stop": [
      {
        "type": "command",
        "command": "~/.claude/bin/claude-hippocampus hook stop"
      }
    ],
    "PostToolUse": [
      {
        "type": "command",
        "command": "~/.claude/bin/claude-hippocampus hook post-tool-use"
      }
    ],
    "SessionEnd": [
      {
        "type": "command",
        "command": "~/.claude/bin/claude-hippocampus hook session-end"
      }
    ]
  }
}
```

### What Each Hook Does

| Hook | Purpose |
|------|---------|
| `SessionStart` | Creates session record, captures git status, loads top 10 memories (newest first) |
| `UserPromptSubmit` | Creates turn record, outputs memory search instructions |
| `Stop` | Extracts learnings from responses, saves to memory |
| `PostToolUse` | Records tool calls with parameters and results |
| `SessionEnd` | Marks session complete with optional summary |

### Context Memory Ordering

Session start loads memories ordered by:
1. **Recency** - newest memories first (`created_at DESC`)
2. **Confidence** - within same time, higher confidence first (high → medium → low)

## Configuration

### Database

Create `~/.claude/config/db.json`:

```json
{
  "host": "localhost",
  "port": 5432,
  "database": "claude_memory",
  "user": "your_user",
  "password": "your_password"
}
```

### Environment Variables

| Variable | Purpose | Default |
|----------|---------|---------|
| `PROJECT_PATH` | Project scope path | Current directory |

## Database Setup

```sql
-- Create database
CREATE DATABASE claude_memory;

-- Memories table
CREATE TABLE memories (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  type VARCHAR(20) NOT NULL,
  scope VARCHAR(10) NOT NULL,
  project_path TEXT,
  content TEXT NOT NULL,
  tags TEXT[] DEFAULT '{}',
  confidence VARCHAR(10) DEFAULT 'medium',
  source_session_id UUID,
  source_turn_id UUID,
  created_at TIMESTAMPTZ DEFAULT NOW(),
  updated_at TIMESTAMPTZ DEFAULT NOW(),
  accessed_at TIMESTAMPTZ,
  access_count INT DEFAULT 0
);

-- Sessions table
CREATE TABLE sessions (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  claude_session_id TEXT UNIQUE,
  project_path TEXT,
  git_status JSONB,
  models_used JSONB DEFAULT '{}',
  status VARCHAR(20) DEFAULT 'active',
  summary JSONB,
  started_at TIMESTAMPTZ DEFAULT NOW(),
  ended_at TIMESTAMPTZ,
  created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Conversation turns table
CREATE TABLE conversation_turns (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  session_id UUID REFERENCES sessions(id),
  turn_number INT NOT NULL,
  user_prompt TEXT NOT NULL,
  assistant_response TEXT,
  model_used VARCHAR(50),
  input_tokens INT,
  output_tokens INT,
  started_at TIMESTAMPTZ DEFAULT NOW(),
  ended_at TIMESTAMPTZ,
  created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Tool calls table
CREATE TABLE tool_calls (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  session_id UUID REFERENCES sessions(id),
  turn_id UUID REFERENCES conversation_turns(id),
  tool_name VARCHAR(100) NOT NULL,
  parameters JSONB,
  result_summary TEXT,
  called_at TIMESTAMPTZ DEFAULT NOW()
);

-- Indexes
CREATE INDEX idx_memories_type ON memories(type);
CREATE INDEX idx_memories_scope ON memories(scope);
CREATE INDEX idx_memories_project ON memories(project_path);
CREATE INDEX idx_memories_confidence ON memories(confidence);
CREATE INDEX idx_memories_created ON memories(created_at DESC);
CREATE INDEX idx_memories_is_active ON memories(is_active);
CREATE INDEX idx_memories_superseded_by ON memories(superseded_by);
CREATE INDEX idx_sessions_claude_id ON sessions(claude_session_id);
CREATE INDEX idx_turns_session ON conversation_turns(session_id);
CREATE INDEX idx_tool_calls_session ON tool_calls(session_id);
CREATE INDEX idx_tool_calls_turn ON tool_calls(turn_id);
```

### Schema Migration (v2 - Retention Policy)

If upgrading from an earlier version, apply this migration:

```sql
-- Add supersession tracking and active status fields
ALTER TABLE memories ADD COLUMN IF NOT EXISTS superseded_by UUID REFERENCES memories(id);
ALTER TABLE memories ADD COLUMN IF NOT EXISTS superseded_at TIMESTAMPTZ;
ALTER TABLE memories ADD COLUMN IF NOT EXISTS is_active BOOLEAN DEFAULT true;

-- Add indexes for the new fields
CREATE INDEX IF NOT EXISTS idx_memories_is_active ON memories(is_active);
CREATE INDEX IF NOT EXISTS idx_memories_superseded_by ON memories(superseded_by);
```

## JSON Output Examples

### Search Results

```json
{
  "success": true,
  "results": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "type": "learning",
      "tier": "project",
      "summary": "API requires authentication header...",
      "content": "API requires authentication header for all requests",
      "tags": ["api", "auth"],
      "confidence": "high",
      "created": "2024-01-15T10:00:00.000Z",
      "accessed": null,
      "accessCount": 5
    }
  ],
  "count": 1
}
```

### Context Block

```json
{
  "success": true,
  "context": "## Memory Context\n\n- ★ **learning**: API requires auth header\n- ◐ **gotcha**: Rate limit is 100/min\n",
  "count": 2,
  "entries": [...]
}
```

## Development

### Running Tests

```bash
# All tests
cargo test

# Specific module
cargo test search::

# Compatibility tests
cargo test --test compatibility

# With output
cargo test -- --nocapture
```

### Project Structure

```
src/
├── main.rs           # CLI entry point
├── lib.rs            # Library exports
├── cli.rs            # Clap argument definitions
├── config.rs         # Database configuration
├── error.rs          # Error types
├── git.rs            # Git status capture
├── session.rs        # Session state management
├── logging.rs        # File-based logging
├── commands/
│   ├── mod.rs        # Command exports
│   ├── memory.rs     # CRUD operations
│   ├── search.rs     # Search commands
│   └── maintenance.rs # Consolidate, prune
├── db/
│   ├── mod.rs        # Database exports
│   ├── pool.rs       # Connection pool
│   └── queries.rs    # SQL operations
├── hooks/
│   ├── mod.rs        # Hook exports
│   ├── session_start.rs    # SessionStart handler
│   ├── user_prompt_submit.rs # UserPromptSubmit handler
│   ├── stop.rs       # Stop handler (memory extraction)
│   ├── post_tool_use.rs    # PostToolUse handler
│   └── session_end.rs      # SessionEnd handler
└── models/
    ├── mod.rs        # Model exports
    ├── memory.rs     # Memory types
    ├── session.rs    # Session model
    ├── turn.rs       # Turn model
    └── response.rs   # JSON response types
```

## Migration from Node.js

See [MIGRATION.md](MIGRATION.md) for detailed migration instructions.

Quick reference:

| Node.js | Rust |
|---------|------|
| `node memory.js addMemory ...` | `claude-hippocampus add-memory ...` |
| `node memory.js searchKeyword ...` | `claude-hippocampus search-keyword ...` |

## Performance

| Metric | Node.js | Rust |
|--------|---------|------|
| Startup | ~50-100ms | ~5ms |
| Query | ~10-20ms | ~5-10ms |
| Binary | N/A | ~3MB |

## License

MIT

## Contributing

1. Fork the repository
2. Create a feature branch
3. Write tests (TDD encouraged)
4. Submit a pull request
