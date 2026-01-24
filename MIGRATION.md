# Migration Guide: Node.js to Rust CLI

## Overview

The Rust CLI (`claude-hippocampus`) is a drop-in replacement for the Node.js memory.js script. Both produce identical JSON output formats.

## Command Name Mapping

| Node.js (camelCase) | Rust (kebab-case) | Notes |
|---------------------|-------------------|-------|
| `addMemory` | `add-memory` | Same args |
| `updateMemory` | `update-memory` | Same args |
| `deleteMemory` | `delete-memory` | Same args |
| `getMemory` | `get-memory` | Same args |
| `searchKeyword` | `search-keyword` | Same args |
| `getContext` | `get-context` | Same args |
| `listRecent` | `list-recent` | Same args |
| `consolidate` | `consolidate` | Same args |
| `prune` | `prune` | Same args |
| `saveSessionSummary` | `save-session-summary` | Same args |
| `logs` | `logs` | Same args |
| `clearLogs` | `clear-logs` | Same args |

## Environment Variables

| Variable | Purpose | Default |
|----------|---------|---------|
| `PROJECT_PATH` | Project scope path | Current working directory |

## Output Format Compatibility

### Verified Compatible

- ✅ `success` field: Always boolean
- ✅ `error` field: String on failure
- ✅ All enum values lowercase: `type`, `confidence`, `tier`
- ✅ Field names use camelCase: `accessCount`, `existingId`, `duplicateIds`
- ✅ UUIDs as strings
- ✅ Timestamps as ISO 8601 strings

### Minor Differences

1. **Timestamp precision**: Rust includes microseconds (`.510906Z`), Node.js truncates (`.510Z`)
2. **Summary truncation**: Both truncate at ~100 chars with `...` suffix

## Switching in Hooks

### Before (Node.js)
```bash
node ~/.claude/scripts/memory.js searchKeyword "$query" both 10
```

### After (Rust)
```bash
~/.claude/bin/claude-hippocampus search-keyword "$query" both 10
```

## Installation

```bash
# Build release binary
cd ~/Desktop/claude-hippocampus
cargo build --release

# Install to ~/.claude/bin (optional)
mkdir -p ~/.claude/bin
cp target/release/claude-hippocampus ~/.claude/bin/
```

## Performance Comparison

| Metric | Node.js | Rust |
|--------|---------|------|
| Startup | ~50-100ms | ~5ms |
| Query | ~10-20ms | ~5-10ms |
| Binary size | N/A (script) | ~3MB |

## Verification

Run the compatibility test:
```bash
cargo test --test compatibility
```

Compare outputs:
```bash
# Node.js
node ~/.claude/scripts/memory.js searchKeyword "test" both 5 > /tmp/node.json

# Rust
./target/release/claude-hippocampus search-keyword "test" both 5 > /tmp/rust.json

# Compare structure (ignoring timestamp precision)
jq -S 'del(.results[].created)' /tmp/node.json > /tmp/node_normalized.json
jq -S 'del(.results[].created)' /tmp/rust.json > /tmp/rust_normalized.json
diff /tmp/node_normalized.json /tmp/rust_normalized.json
```
