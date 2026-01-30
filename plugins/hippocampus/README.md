# Hippocampus Plugin

PostgreSQL-backed persistent memory for Claude Code with semantic search.

## Requirements

- PostgreSQL database with the memory schema
- `claude-hippocampus` CLI binary installed (Rust)
- Database configuration at `~/.claude/config/db.json`

## Installation

### 1. Build the CLI

```bash
cd /path/to/claude-hippocampus
cargo build --release
cp target/release/claude-hippocampus ~/.claude/bin/
```

### 2. Set up PostgreSQL

Create database and run the schema from `README.md` in the project root.

### 3. Configure Database

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

### 4. Install Plugin

```bash
# Copy plugin to Claude Code plugins directory
cp -r plugins/hippocampus ~/.claude/plugins/

# Or use plugin-dir flag
claude --plugin-dir /path/to/plugins/hippocampus
```

## Components

### Skills

- **brain-cells** - Knowledge about memory operations, types, confidence levels, and search patterns

### Agents

- **memory-manager** - Handles remember/recall requests using the claude-hippocampus CLI

## Usage

The plugin activates when you use phrases like:
- "Remember this learning..."
- "What did we learn about..."
- "Recall any gotchas for..."
- "Save this decision..."

Or use the slash commands:
- `/me:remember "learning"` - Save a learning
- `/me:recall "query"` - Search memories

## Memory Types

| Type | Use For |
|------|---------|
| `gotcha` | Failure patterns, warnings |
| `convention` | Code style, naming |
| `architecture` | Design decisions |
| `api` | API quirks, integration |
| `learning` | General observations |
| `preference` | User preferences |

## License

MIT
