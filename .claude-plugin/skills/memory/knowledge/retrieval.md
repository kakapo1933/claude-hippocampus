---
topic: Memory Retrieval
keywords: [search, query, semantic, retrieval, context]
---

# Memory Retrieval

Keyword search with Haiku-powered semantic ranking.

## Search Algorithm

```
User Query
    │
    ▼
┌───────────────────┐
│ memory-helper     │  ← Extract 2-4 keywords from prompt
│ (Haiku)           │
└─────────┬─────────┘
          │
          ▼
┌───────────────────┐
│ Keyword Search    │  ← PostgreSQL ILIKE on content/tags
│ (search-keyword)  │
└─────────┬─────────┘
          │
          ▼
┌───────────────────┐
│ Semantic Ranking  │  ← Haiku ranks by relevance to original prompt
│ (memory-helper)   │
└─────────┬─────────┘
          │
          ▼
    Top 5 Results
```

---

## Keyword Search

Uses PostgreSQL text matching:

```sql
SELECT id, content, type, confidence
FROM memories
WHERE content ILIKE '%' || $query || '%'
   OR EXISTS (SELECT 1 FROM unnest(tags) AS t WHERE t ILIKE '%' || $query || '%')
ORDER BY confidence, created_at DESC
LIMIT 30;
```

**Matching:** Case-insensitive ILIKE on content and tags
**Ranking:** Confidence level, then recency

---

## CLI Search Commands

```bash
# Keyword search
claude-hippocampus search-keyword "video" both 20

# Search by type
claude-hippocampus search-by-type gotcha "api" project 10

# Get formatted context
claude-hippocampus get-context 10

# List recent
claude-hippocampus list-recent 5 both
```

---

## Automatic Retrieval

When a user sends a substantive prompt, the UserPromptSubmit hook:
1. Outputs instructions for memory-helper agent
2. memory-helper extracts 2-4 keywords from prompt
3. Runs keyword searches for each keyword
4. Semantically ranks and filters results
5. Returns top 5 most relevant memories

Ranking factors:
- Semantic relevance (Haiku judgment)
- Confidence (HIGH > MEDIUM > LOW)
- Recency (newer first)

---

## Context Block Format

When injected, memories appear as:

```markdown
╭────────────────────────────── 🧠 Memory Found ───────────────────────────────╮
│ ★ [gotcha] FAL.ai requires rawUrl not processed URLs (id: abc123)           │
│ ★ [architecture] Use S3 pre-upload for payloads >1MB (id: def456)           │
│ ◐ [convention] Project uses camelCase for functions (id: ghi789)            │
╰──────────────────────────────────────────────────────────────────────────────╯
```

**Icons:**
- ★ = HIGH confidence
- ◐ = MEDIUM confidence
- ○ = LOW confidence

---

## Scope Filtering

| Tier Parameter | Searches |
|----------------|----------|
| `project` | Only current project memories |
| `global` | Only global memories |
| `both` (default) | Global + current project |

---

## Access Tracking

Each search updates access metadata:
- `accessed_at` = current timestamp
- `access_count` = access_count + 1

Top 5 results get access recorded, enabling:
- Frequently used entries rank higher
- Unused entries get pruned eventually

---

## Token Budget

To stay within context limits:

| Tier | Max Entries | Est. Tokens |
|------|-------------|-------------|
| Project | 10 | ~500 |
| Global | 5 | ~250 |
| Total | 15 | ~750 |

If memories exceed budget:
1. Prioritize HIGH confidence
2. Prioritize frequently accessed
3. Truncate summaries if needed
