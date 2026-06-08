# Project State — claurst

## Topology Phase: Ceiling

> Phase tracking: Floor = foundations | Bridge = integration/synthesis | Ceiling = final polish, release-ready

We are in **Ceiling phase** for the Framework Memory Subsystem. The Rust plumbing is complete and compiles clean. The surface-layer integration — README, docs, CLI commands, and markdown framework files — is aligned with the Harness-Subsystem Architecture defined in ATTRACTOR.md.

---

## Session Intent (2026-06-07)

Align the entire project surface — README, documentation, `/init` command, and STATE.md — to the Harness-Subsystem Architecture. The Claurst Harness using the AGENT framework at its core is now the project's identity statement across all user-facing material.

---

## Architectural Invariants

| Invariant | Verified |
|-----------|----------|
| Framework files are markdown — no code changes needed in agent behavior | ✅ |
| Harness controls delivery timing (session-start vs every-turn) | ✅ |
| System prompt injection is the integration seam — not tool calls, not MCP | ✅ |
| Harness is "subconscious" — agent framework is "conscious executive" | ✅ (see ATTRACTOR.md) |
| AGENTS.md, AGENT.md, USER.md cascade: global (~/.claurst/) → project ({root}/) | ✅ |
| `/init` produces STATE.md + ATTRACTOR.md (not legacy AGENTS.md) | ✅ |

---

## Semantic Attractor

The **Harness-Subsystem Architecture** (ATTRACTOR.md) governs all pattern inference:

- **Harness = subconscious autonomic system** — file loading, prompt injection, delivery timing, boundary control. The agent has no awareness of these mechanics.
- **Agent framework = conscious executive layer** — dictates runtime behavior through markdown files. Experiences memory injection as given facts, not as mechanical processes.

Every design decision must respect this boundary: add injection points to the harness, not instructions to the agent.

---

## Changes Made (this session)

### Surface Alignment (docs/ + README.md)

| File | Change |
|------|--------|
| `README.md` | Architecture section heading + opening reframed to "This version of Claurst Harness has the AGENT Framework integrated into it" — transparent, no fluff |
| `docs/index.md` | Entire architecture section rewritten with Conscious/Subconscious layer split, 8-file delivery table, topology inversion. Opening language aligned to transparent anchor |
| `docs/agents.md` | Disambiguation callout added: separates runtime named agents from AGENT framework files |
| `docs/advanced.md` | "AGENTS.md hierarchical memory" section completely rewritten → "AGENT framework files" with 8-file registry, delivery modes, cascade, CLAUDE.md deprecation |
| `docs/configuration.md` | "AGENTS.md Memory Files" section completely rewritten → "AGENT Framework Files" with same 8-file table |
| `docs/commands.md` | `/init` entry updated to describe STATE.md + ATTRACTOR.md creation |
| `docs/hooks.md` | `InstructionsLoaded` event description updated ("CLAUDE.md" → "AGENT framework file") |
| `docs/plugins.md` | `InstructionsLoaded` event description updated ("CLAUDE.md" → "AGENT framework files") |

### Code Changes

| File | Change |
|------|--------|
| `src-rust/crates/commands/src/lib.rs` | `/init` command rewritten: produces STATE.md + ATTRACTOR.md templates. No longer creates AGENTS.md |
| `src-rust/crates/tui/src/app.rs` | `/init` slash-command description changed to "Initialize STATE.md and ATTRACTOR.md for this project" |

### Framework File Status

| File | Delivery Mode | Cascade | Scope | Created By |
|------|--------------|---------|-------|------------|
| AGENTS.md | SessionStart | ✅ global → project | User / Project | User |
| AGENT.md | SessionStart + EveryTurn | ✅ global → project | User / Project | User |
| USER.md | EveryTurn | ✅ global → project | User / Project | User |
| ATTRACTOR.md | SessionStart | — | Project | `/init` or user |
| BRAIN.md | SessionStart | — | Project | User |
| HEART.md | SessionStart | — | Project | User |
| MEMORY.md | EveryTurn | — | Project | User |
| STATE.md | EveryTurn | — | Project | `/init` or user |

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

1. Wire every-turn file content into the dynamic `<memory>` block (future enhancement)
2. Remove legacy CLAUDE.md references from remaining spec/ directory files
3. Verify end-to-end: start a session with `--dump-system-prompt` to confirm `<framework_identity>` and `<periodic_nudge>` injection
4. Commit logical unit
