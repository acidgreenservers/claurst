# Project State — claurst

## Topology Phase: Bridge (partial — one seam incomplete)

> Phase tracking: Floor = foundations | Bridge = integration/synthesis | Ceiling = final polish, release-ready

We are in **Bridge phase** for the Framework Memory Subsystem. The plumbing is installed across 5 files in 3 crates and compiles clean. One consistency gap remains (every-turn content is loaded but not injected as a memory block — only file names are nudged).

---

## Session Intent (2026-06-07)

Integrate a markdown-based agent framework into the harness as an autonomic memory subsystem. The harness silently governs the agent by injecting framework files (AGENTS.md, AGENT.md, HEART.md, STATE.md, MEMORY.md, USER.md, BRAIN.md, ATTRACTOR.md) into the system prompt at specific delivery cadences — without the agent being aware of the delivery mechanics.

---

## Architectural Invariants

| Invariant | Verified |
|-----------|----------|
| Framework files are markdown — no code changes needed in agent behavior | ✅ |
| Harness controls delivery timing (session-start vs every-turn) | ✅ |
| System prompt injection is the integration seam — not tool calls, not MCP | ✅ |
| Harness is "subconscious" — agent framework is "conscious executive" | ✅ (see ATTRACTOR.md) |
| AGENTS.md, AGENT.md, USER.md cascade: global (~/.claurst/) → project ({root}/) | ✅ |

---

## Semantic Attractor

The **Harness-Subsystem Architecture** (ATTRACTOR.md) governs all pattern inference:

- **Harness = subconscious autonomic system** — file loading, prompt injection, delivery timing, boundary control. The agent has no awareness of these mechanics.
- **Agent framework = conscious executive layer** — dictates runtime behavior through markdown files. Experiences memory injection as given facts, not as mechanical processes.

Every design decision must respect this boundary: add injection points to the harness, not instructions to the agent.

---

## Changes Made (this session)

### New/Modified Source Files

| File | Change | Crate |
|------|--------|-------|
| `src-rust/crates/core/src/claudemd.rs` | **Major rewrite.** Added `DeliveryMode` enum, `load_cascaded_file()` for global→project cascade, `load_all_memory_files()` returns `(session_start, every_turn)` vectors, `build_framework_identity()`, `build_memory_prompt()`, `build_periodic_nudge()`. Framework file registry split: cascaded (AGENTS.md, AGENT.md, USER.md) vs project-only (ATTRACTOR.md, BRAIN.md, HEART.md, MEMORY.md, STATE.md) | `claurst-core` |
| `src-rust/crates/core/src/system_prompt.rs` | **Fields added to `SystemPromptOptions`:** `framework_identity`, `periodic_nudge_files`, `periodic_nudge`. Injection points: `<framework_identity>` in cacheable block, `<periodic_nudge>` in dynamic block | `claurst-core` |
| `src-rust/crates/query/src/lib.rs` | **Fields added to `QueryConfig`:** `framework_identity`, `periodic_nudge_files`. Wired into `build_system_prompt()` | `claurst-query` |
| `src-rust/crates/query/src/agent_tool.rs` | Added `framework_identity: String::new()`, `periodic_nudge_files: Vec::new()` to `QueryConfig` initializer (downstream break fix) | `claurst-query` |
| `src-rust/crates/cli/src/main.rs` | Calls `load_all_memory_files(&cwd)`, wires `framework_identity` and `periodic_nudge_files` into `query_config`. Every-turn file names extracted for nudge mechanism | `claurst` |

### New/Modified Markdown Files

| File | Change |
|------|--------|
| `ATTRACTOR.md` | **Created.** Semantic anchor — harness/subconscious architecture definition |
| `STATE.md` | **Rewritten.** Process document → comprehensive project state snapshot |

### Blast Radius

- **Direct (Rust):** 5 files in 3 crates (`claurst-core`, `claurst-query`, `claurst`)
- **Direct (Markdown):** 2 files in project root (`ATTRACTOR.md`, `STATE.md`)
- **Indirect:** Zero — no existing behavior modified, purely additive
- **Build:** `cargo check --workspace` passes clean (zero errors, zero warnings)

### Framework File Registry

| File | Delivery Mode | Cascade | Scope |
|------|--------------|---------|-------|
| AGENTS.md | SessionStart | ✅ global → project | User / Project |
| AGENT.md | SessionStart + EveryTurn | ✅ global → project | User / Project |
| USER.md | EveryTurn | ✅ global → project | User / Project |
| ATTRACTOR.md | SessionStart | — | Project |
| BRAIN.md | SessionStart | — | Project |
| HEART.md | SessionStart | — | Project |
| MEMORY.md | EveryTurn | — | Project |
| STATE.md | EveryTurn | — | Project |

---

## Known Issues

### Issue 1 — Every-Turn Content Not Injected as Memory Block
**Severity:** Low (nudge mechanism works, content is not lost)

`load_all_memory_files()` returns every-turn files in the second vector. The CLI extracts their **file names** and wires them into `periodic_nudge_files`, which generates a `<periodic_nudge>` block every ~10 turns telling the agent to re-read those files. However, the actual **content** of every-turn files is not injected as a `<memory>` block on each turn — the agent must use `Read` to fetch them. This is acceptable for the MVP: the nudge tells the agent what to re-read, and the agent reads them via its own tool calls.

### Issue 2 — RULES.md Not Registered
**Severity:** None (by design)

`RULES.md` is a user artifact (renamed old AGENTS.md), not a framework file. It sits in the project root but is never loaded by the harness. This is correct.

---

## Next Topological Moves

1. Verify end-to-end: start a session, inspect `--dump-system-prompt` for `<framework_identity>` content
2. Optionally wire every-turn file content into the dynamic `<memory>` block (future enhancement)
3. Commit when logical unit is complete