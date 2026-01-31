---
name: brainstorming
keywords:
  - brainstorm
  - design
  - plan
  - think about
  - figure out
  - explore options
  - what should we build
  - help me design
  - idea
  - spec
  - requirements
  - architecture
  - discuss
description: |
  This skill should be used before any creative work - creating features, building components,
  adding functionality, or modifying behavior. Trigger phrases include: "brainstorm", "let's think about",
  "help me design", "what should we build", "explore options for", "figure out how to", "I have an idea",
  "let's plan", "design this", "spec out", "what's the best way to". Also use when the user describes
  a vague idea that needs refinement, or when starting any non-trivial implementation without a clear spec.
---

# Brainstorming Ideas Into Designs

Transform vague ideas into concrete, validated designs through collaborative dialogue.

---

## Process Overview

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│  1. Understand  │───▶│  2. Explore      │───▶│  3. Present     │
│     the Idea    │    │     Approaches   │    │     Design      │
└─────────────────┘    └──────────────────┘    └─────────────────┘
        │                      │                       │
   One question           2-3 options            200-300 word
   at a time              with tradeoffs         sections
```

---

## Phase 1: Understanding the Idea

**Gather context first:**
- Check project state (files, docs, recent commits)
- Search memory for related past decisions
- Identify existing patterns to align with

**Ask questions using the AskUserQuestion tool:**
- **Always use AskUserQuestion** for gathering input - never just output questions as text
- Prefer multiple choice (2-4 options) when possible
- Focus on: purpose, constraints, success criteria
- One question at a time (tool supports up to 4 questions, but prefer 1-2)

**AskUserQuestion format:**
```
questions: [
  {
    "question": "Who will use this feature?",
    "header": "Audience",
    "options": [
      {"label": "Developers", "description": "Internal tooling or API"},
      {"label": "End users", "description": "Customer-facing feature"},
      {"label": "Both", "description": "Shared functionality"}
    ],
    "multiSelect": false
  }
]
```

---

## Phase 2: Exploring Approaches

**Propose 2-3 approaches using AskUserQuestion:**

First explain each approach briefly, then use AskUserQuestion:
```
questions: [
  {
    "question": "Which approach should we take?",
    "header": "Approach",
    "options": [
      {"label": "Simple (Recommended)", "description": "Fast to build, covers core use case"},
      {"label": "Balanced", "description": "Good coverage, moderate effort"},
      {"label": "Comprehensive", "description": "Full-featured but complex"}
    ],
    "multiSelect": false
  }
]
```

**Lead with your recommendation** - put it first and add "(Recommended)" to the label.

**Apply YAGNI ruthlessly** - remove unnecessary features from all options.

---

## Phase 3: Presenting the Design

**Break into 200-300 word sections:**

1. Architecture overview
2. Key components
3. Data flow
4. Error handling
5. Testing approach

**After each section, use AskUserQuestion to validate:**
```
questions: [
  {
    "question": "Does this architecture look right so far?",
    "header": "Validate",
    "options": [
      {"label": "Looks good", "description": "Continue to next section"},
      {"label": "Needs changes", "description": "I'll explain what to adjust"},
      {"label": "Step back", "description": "Rethink this approach"}
    ],
    "multiSelect": false
  }
]
```

Be ready to backtrack and clarify when something doesn't fit.

---

## After Validation

**Document the design to Notion:**

When the design is validated, ask for the Notion page:
```
questions: [
  {
    "question": "Ready to document the design. Where should I write it?",
    "header": "Notion page",
    "options": [
      {"label": "Provide Notion URL", "description": "I'll give you the page URL to write to"},
      {"label": "Skip documentation", "description": "Don't write anywhere, just continue"},
      {"label": "Create local file", "description": "Write to docs/plans/ instead"}
    ],
    "multiSelect": false
  }
]
```

If user provides Notion URL:
- Use the Notion MCP tools to write the design
- Structure with headings: Overview, Architecture, Components, Data Flow, Testing
- Include decision rationale and trade-offs discussed

**Transition to implementation:**
```
questions: [
  {
    "question": "Ready to move forward?",
    "header": "Next step",
    "options": [
      {"label": "Start implementation", "description": "Create workspace and implementation plan"},
      {"label": "Refine design", "description": "Revisit specific sections"},
      {"label": "Done for now", "description": "Implement later"}
    ],
    "multiSelect": false
  }
]
```

---

## Key Principles

| Principle | Why |
|-----------|-----|
| **Use AskUserQuestion tool** | Interactive, clickable options |
| One question at a time | Avoids overwhelming |
| Multiple choice preferred | Easier to answer |
| YAGNI ruthlessly | Prevents scope creep |
| Explore alternatives | Better decisions |
| Incremental validation | Catches misunderstandings early |
| Be flexible | Backtrack when needed |

---

## Memory Integration

**Before brainstorming:**
- Search memory for related architecture decisions
- Check for relevant gotchas or conventions

**After brainstorming:**
- Save key design decisions to memory
- Record rationale for future reference
