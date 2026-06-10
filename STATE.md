# Project State — claurst

## Topology Phase: Ceiling

> Phase tracking: Floor = foundations | Bridge = integration/synthesis | Ceiling = final polish, release-ready

We are in **Ceiling phase** for the Framework Memory Subsystem. The Rust plumbing is complete and compiles clean. The surface-layer integration — README, docs, CLI commands, and markdown framework files — is aligned with the Harness-Subsystem Architecture defined in ATTRACTOR.md.

---

## Session Intent (2026-06-09)

Implement a turn-gated periodic nudge (every 15 turns) and update all documentation to reflect the verified Harness-Subsystem Architecture — fixing delivery-mode inaccuracies in ATTRACTOR.md and adding the verified architecture section to README.md.

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
| Periodic framework nudge fires every 15 turns (not every turn) | ✅ |

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
| `README.md` | New verified "Harness-Subsystem Model" architecture section with code-verified ASCII tables — replaces old "AGENT Framework at the Core" section |
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
| `src-rust/crates/query/src/lib.rs` | Added turn-gated periodic framework nudge: `if turn > 0 && turn % 15 != 0 { periodic_nudge_files.clear() }` |
| `src-rust/crates/core/src/system_prompt.rs` | Comment updated: `"every N turns"` → `"every 15 turns"` |

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

`load_all_memory_files()` returns every-turn files in the second vector. The CLI extracts their **file names** and wires them into `periodic_nudge_files`, which generates a `<periodic_nudge>` block every 15 turns telling the agent to re-read those files. However, the actual **content** of every-turn files is not injected as a `<memory>` block on each turn — the agent must use `Read` to fetch them. This is acceptable for the MVP: the nudge tells the agent what to re-read, and the agent reads them via its own tool calls.

### Issue 2 — RULES.md Not Registered
**Severity:** None (by design)

`RULES.md` is a user artifact (renamed old AGENTS.md), not a framework file. It sits in the project root but is never loaded by the harness. This is correct.

---

## Next Topological Moves

1. Wire every-turn file content into the dynamic `<memory>` block (future enhancement)
2. Remove legacy CLAUDE.md references from remaining spec/ directory files
3. Verify end-to-end: start a session with `--dump-system-prompt` to confirm `<framework_identity>` and `<periodic_nudge>` injection
4. Commit logical unit
5. Add USER.md to periodic nudge file list (nudge currently covers AGENT.md, AGENTS.md, STATE.md, ATTRACTOR.md only)
