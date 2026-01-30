---
topic: Learning Extraction
keywords: [extraction, learning, automatic, hook, stop, semantic]
---

# Learning Extraction

The memory system uses **aggressive extraction** to capture everything potentially useful.
memory-helper agent (Haiku) handles intelligent extraction.

---

## Extraction Categories

### HIGH Confidence (Always Capture)

**User Corrections**
- Pattern: "actually X", "no, use Y instead", "not X, Y"
- Example: "actually, use rawUrl not url for external APIs"
- Action: Immediate save as gotcha (via UserPromptSubmit hook)

**API/Integration Discoveries**
- Pattern: Error messages and their solutions
- Example: "FAL.ai returns 400 when using processed URLs"
- Action: Save as api type

**Decisions with Rationale**
- Pattern: "We chose X because Y"
- Example: "Using S3 pre-upload to avoid payload size limits"
- Action: Save as architecture type

---

### MEDIUM Confidence (Capture, May Prune Later)

**Code Patterns Observed**
- Pattern: Naming conventions, file organization
- Example: "This project uses camelCase for functions"
- Action: Save as convention type

**Tool/Command Outputs**
- Pattern: What commands produced what results
- Example: "npm run build requires NODE_ENV=production"
- Action: Save as gotcha type

**Build Configurations**
- Pattern: Environment setup, dependencies
- Example: "Project uses pnpm, not npm"
- Action: Save as convention type

---

### LOW Confidence (Capture, Will Prune)

**Observations**
- Pattern: "Seems like...", "Noticed that..."
- Example: "Seems like the API has rate limits"
- Action: Save as learning type

**Hypotheses**
- Pattern: "Might be because...", "Could be..."
- Example: "Might be failing due to timeout settings"
- Action: Save as learning type

---

## Extraction Process

1. **Stop hook captures** turn context (user prompt, assistant response)
2. **Outputs instructions** for memory-helper agent
3. **memory-helper (Haiku) analyzes** the turn content
4. **Classifies** type and extracts conclusion
5. **Checks for duplicates** (first 100 chars match)
6. **Saves** to PostgreSQL with source tracking

---

## What NOT to Extract

- Routine code changes without learnings
- Minor fixes ("fixed typo")
- Transient debugging info
- Personal opinions without factual basis
- Speculation without evidence

---

## Duplicate Detection

**Exact Match Detection:**
- First 100 characters of content compared
- Same type required for match
- If exact match → duplicate detected

**Consolidation:**
- `claude-hippocampus consolidate` merges exact duplicates
- Keeps first entry
- Run periodically for cleanup

---

## CLI Usage

```bash
# Add learning (with automatic duplicate check)
claude-hippocampus add-memory gotcha "Learning content" "tags" high project

# Consolidate duplicate entries
claude-hippocampus consolidate project

# Prune old low-confidence entries
claude-hippocampus prune 90 project
```

---

## Memory Types Reference

| Type | Use For | Typical Confidence |
|------|---------|-------------------|
| `gotcha` | Failure patterns, warnings | HIGH |
| `convention` | Code style, naming | MEDIUM |
| `architecture` | Design decisions | HIGH |
| `api` | API quirks, integration | HIGH/MEDIUM |
| `learning` | General observations | LOW/MEDIUM |
| `preference` | User preferences | HIGH |
