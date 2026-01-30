---
name: memory
description: PostgreSQL-backed persistent memory with Haiku-powered semantic search
domain: persistence
keywords: [memory, remember, recall, learning, decision, gotcha, convention, semantic]
load_trigger: "remember|recall|memory|learned|decision|correction|gotcha"
priority: 5
---

# Memory Skill

PostgreSQL-backed persistent memory for Claude Code sessions with Haiku-powered semantic search.

---

## Quick Reference

| Operation | CLI Command | Purpose |
|-----------|-------------|---------|
| **Add** | `claude-hippocampus add-memory` | Save a learning |
| **Search** | `claude-hippocampus search-keyword` | Search memories |
| **Context** | `claude-hippocampus get-context` | Get context block |
| **Stats** | `claude-hippocampus stats` | View statistics |

---

## CLI Commands

```bash
# Core operations
claude-hippocampus add-memory <type> "<content>" "<tags>" <confidence> <scope>
claude-hippocampus search-keyword "<query>" [tier] [limit]
claude-hippocampus search-by-type <type> [keyword] [tier] [limit]
claude-hippocampus get-context [limit]
claude-hippocampus get-memory <id>
claude-hippocampus update-memory <id> "<content>" [tier]
claude-hippocampus delete-memory <id> [tier]

# Maintenance
claude-hippocampus consolidate [tier]     # Merge duplicates
claude-hippocampus prune [days] [tier]    # Remove old low-confidence
claude-hippocampus list-recent [n] [tier] # View recent entries
claude-hippocampus stats                  # Show statistics

# Session management
claude-hippocampus create-session <claude-id> <project-path> [git-status]
claude-hippocampus get-session <id>
claude-hippocampus end-session <id> [summary]
claude-hippocampus create-turn <session-id> <turn-number> "<user-prompt>"
claude-hippocampus update-turn <turn-id> "<response>" [tokens]
```

---

## Memory Types

| Type | Use For | Typical Confidence |
|------|---------|-------------------|
| `gotcha` | Failure patterns, warnings | HIGH |
| `convention` | Code style, naming | MEDIUM |
| `architecture` | Design decisions | HIGH |
| `api` | API quirks, integration | HIGH/MEDIUM |
| `learning` | General observations | LOW/MEDIUM |
| `preference` | User preferences | HIGH |

---

## Confidence Levels

| Level | When | Retention |
|-------|------|-----------|
| **HIGH** (★) | User corrections, explicit decisions | Long-term |
| **MEDIUM** (◐) | Observed patterns, tool outputs | 90 days |
| **LOW** (○) | Observations, hypotheses | 30 days (pruned) |

---

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    PostgreSQL Database                      │
├──────────────┬────────────────┬────────────────┬────────────┤
│   sessions   │ conversation_  │  tool_calls    │  memories  │
│              │    turns       │                │            │
├──────────────┼────────────────┼────────────────┼────────────┤
│ claude_id    │ session_id     │ session_id     │ type       │
│ project_path │ turn_number    │ turn_id        │ scope      │
│ git_status   │ user_prompt    │ tool_name      │ content    │
│ status       │ response       │ parameters     │ embedding  │
│ summary      │ tokens         │ file_path      │ confidence │
└──────────────┴────────────────┴────────────────┴────────────┘
        │                                              │
        └──────── Keyword Search + Haiku Ranking ──────┘
```

---

## Scopes

| Scope | Description | Storage |
|-------|-------------|---------|
| `global` | Cross-project preferences | `scope='global'` |
| `project` | Project-specific learnings | `scope='project'` + `project_path` |

---

## When to Save Memories

Save to memory when you discover:
- User corrections or preferences
- API quirks or unexpected behavior
- Project conventions or patterns
- Architecture decisions with rationale
- Error resolutions

---

## When to Search Memory

Search proactively when:
- **Task shifts domain**: Moving to unfamiliar codebase area
- **User references past work**: "Remember when...", "Like we did before"
- **Unexpected error**: Check for known gotchas before debugging
- **Before implementation**: Look for architecture decisions
- **API integration**: Check for known quirks or patterns

---

## Detailed Knowledge

- [knowledge/operations.md](knowledge/operations.md) - CLI commands reference
- [knowledge/extraction.md](knowledge/extraction.md) - Learning extraction patterns
- [knowledge/retrieval.md](knowledge/retrieval.md) - Search and ranking
