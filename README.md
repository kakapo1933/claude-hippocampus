# Claude Hippocampus

A high-performance Rust CLI for persistent memory management in Claude Code sessions.

## Overview

Claude Hippocampus is a native Rust replacement for the Node.js memory system, providing PostgreSQL-backed persistent memory with ~10x faster startup times. It serves as a drop-in replacement for `~/.claude/scripts/memory.js` with identical JSON output.

## Features

- **Fast**: ~5ms startup vs ~50-100ms for Node.js
- **Persistent**: PostgreSQL-backed storage with project/global scoping
- **Compatible**: JSON output matches Node.js for seamless hook integration
- **Complete**: All memory operations (CRUD, search, maintenance)
- **Tested**: 148 tests (unit + integration + compatibility)

## Installation

### Prerequisites

- Rust 1.70+ (`rustup install stable`)
- PostgreSQL with the memory schema (see [schema setup](#database-setup))

### Build

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

### Commands

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

# Maintenance
claude-hippocampus consolidate project  # Remove duplicates
claude-hippocampus prune 90 project     # Remove old low-confidence entries

# View logs
claude-hippocampus logs 50
claude-hippocampus clear-logs
```

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

| Level | Symbol | Description |
|-------|--------|-------------|
| `high` | ★ | Verified, critical information |
| `medium` | ◐ | Likely correct, moderate importance |
| `low` | ○ | Uncertain or low priority |

### Tiers

| Tier | Scope |
|------|-------|
| `project` | Current project only |
| `global` | All projects |
| `both` | Search both (default) |

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
  claude_session_id TEXT,
  project_path TEXT,
  git_status JSONB,
  models_used JSONB DEFAULT '{}',
  status VARCHAR(20) DEFAULT 'active',
  summary JSONB,
  started_at TIMESTAMPTZ DEFAULT NOW(),
  ended_at TIMESTAMPTZ
);

-- Indexes
CREATE INDEX idx_memories_type ON memories(type);
CREATE INDEX idx_memories_scope ON memories(scope);
CREATE INDEX idx_memories_project ON memories(project_path);
CREATE INDEX idx_memories_confidence ON memories(confidence);
CREATE INDEX idx_memories_created ON memories(created_at DESC);
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
├── session.rs        # Session state management
├── logging.rs        # File-based logging
├── commands/
│   ├── memory.rs     # CRUD operations
│   ├── search.rs     # Search commands
│   └── maintenance.rs # Consolidate, prune
├── db/
│   ├── pool.rs       # Connection pool
│   └── queries.rs    # SQL operations
└── models/
    ├── memory.rs     # Memory types
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
