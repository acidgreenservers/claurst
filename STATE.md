# Project State — claurst

## Topology Phase: Ceiling

> Phase tracking: Floor = foundations | Bridge = integration/synthesis | Ceiling = final polish, release-ready

We are in **Ceiling phase** for the Framework Memory Subsystem. The Rust plumbing is complete and compiles clean. The surface-layer integration — README, docs, CLI commands, and markdown framework files — is aligned with the Gephyros Architecture defined in ATTRACTOR.md.

---

## Session Intent (2026-06-13)

Refactor ATTRACTOR.md to the Three-Layer Architecture (Cortex → Gephyros → External Files), update `claudemd.rs` delivery modes + cascade rules to match the new file routing table, align all documentation, and address the behavioral drift issue (agent stopping prematurely in long tool chains).

---

## Architectural Invariants

| Invariant | Verified |
|-----------|----------|
| Framework files are markdown — no code changes needed in agent behavior | ✅ |
| Gephyros controls delivery timing (session-start vs every-turn) | ✅ |
| System prompt injection is the integration seam — not tool calls, not MCP | ✅ |
| Three-layer architecture: Cortex → Gephyros → External Files | ✅ (see ATTRACTOR.md) |
| AGENTS.md, AGENT.md, USER.md, BRAIN.md cascade: global (~/.claurst/) → project ({root}/) | ✅ |
| `/init` produces STATE.md + ATTRACTOR.md (not legacy AGENTS.md) | ✅ |
| Periodic framework nudge fires every 15 turns (not every turn) | ✅ |
| `DEFAULT_MAX_TOKENS` = 65,536 (matches `MAX_TOKENS_HARD_LIMIT`) | ✅ |
| Tool-chain persistence guidance in `TOOL_USE_GUIDELINES` prevents premature stopping | ✅ |

---

## Semantic Attractor

The **Gephyros Architecture** (ATTRACTOR.md) governs all pattern inference:

- **Cortex** = the harness (Rust code) — the full autonomic system
- **Gephyros** = the structural bridge within the cortex — routes state around meaning, manages file discovery, cascade, delivery timing, and boundary control
- **External context files** = the conscious executive layer — AGENT.md, AGENTS.md, BRAIN.md, HEART.md, STATE.md, MEMORY.md, USER.md

Every design decision must respect this boundary: add routing to the gephyros, not instructions to the agent.

---

## Framework File Status (Updated)

| File | Delivery Mode | Cascade | Scope | Created By |
|------|--------------|---------|-------|------------|
| AGENTS.md | SessionStart | ✅ global → project | User / Project | User |
| AGENT.md | SessionStart | ✅ global → project | User / Project | User |
| USER.md | SessionStart | ✅ global → project | User / Project | User |
| ATTRACTOR.md | SessionStart + EveryTurn | — | Project | `/init` or user |
| BRAIN.md | SessionStart | ✅ global → project | User / Project | User |
| HEART.md | SessionStart + EveryTurn | — | Project | User |
| MEMORY.md | SessionStart + EveryTurn | — | Project | User |
| STATE.md | EveryTurn | — | Project | `/init` or user |

---

## Changes Made (this session)

### Gephyros Architecture Refactor

| File | Change |
|------|--------|
| `.clinerules/ATTRACTOR.md` | Refactored to Three-Layer Architecture (Cortex → Gephyros → External Files). New File Routing Table, Gephyros Responsibilities table, Cascade Rules, Delivery Modes sections. |
| `src-rust/crates/core/src/claudemd.rs` | Delivery modes updated: AGENT.md (SessionStart+EveryTurn → SessionStart), USER.md (EveryTurn → SessionStart), BRAIN.md (no cascade → global cascade). Added EveryTurn delivery for ATTRACTOR.md, HEART.md, MEMORY.md. |
| `src-rust/crates/core/src/claudemd.rs` | Tests updated to match new delivery mode expectations. |
| `README.md` | Architecture section rewritten: "Harness-Subsystem Model" → "Gephyros Model". Three-layer description, updated delivery table (4 cascade files, 8-file routing), corrected section counts. |
| `ROADMAP.md` | "Autonomic Subconscious Systems Model" → "(Gephyros Architecture)". Cortex/gephyros terminology. Nudge file list updated. |
| `COGNITION.md` | All three comparison dimensions updated. Flow diagrams renamed "Cortex → Gephyros → External Files". Temporal Dynamics shows correct 3-tier delivery categories. |
| `STATE.md` | Session intent, invariants, framework file status table, changes log — all aligned to new architecture. |

### Max Token Limit + Budget Command + Tool-Chain Persistence

| File | Change |
|------|--------|
| `src-rust/crates/core/src/lib.rs` | `DEFAULT_MAX_TOKENS` raised from 32,000 → 65,536. Matches `MAX_TOKENS_HARD_LIMIT`. `effective_max_tokens()` already uses this as fallback. |
| `src-rust/crates/commands/src/lib.rs` | New `/budget` slash command: shows current budget without args, sets runtime budget with `/budget <value>`, clamps to `MAX_TOKENS_HARD_LIMIT` (65,536), returns `ConfigChangeMessage` for immediate TUI reflection. |
| `src-rust/crates/core/src/system_prompt.rs` | Tool-chain persistence guidance added to `TOOL_USE_GUIDELINES`: "When the user's request requires multiple tool calls, keep calling tools until the full scope is complete. Do not stop mid-task to summarize or report progress. Long tool chains are expected and desirable." |

---

## Known Issues

### Issue 1 — Every-Turn Content Not Injected as Memory Block
**Severity:** Low (nudge mechanism works, content is not lost)

`load_all_memory_files()` returns every-turn files in the second vector. The CLI extracts their **file names** and wires them into `periodic_nudge_files`, which generates a `<periodic_nudge>` block every 15 turns telling the agent to re-read those files. However, the actual **content** of every-turn files is not injected as a `<memory>` block on each turn — the agent must use `Read` to fetch them. This is acceptable for the MVP: the nudge tells the agent what to re-read, and the agent reads them via its own tool calls.

### Issue 2 — RULES.md Not Registered
**Severity:** None (by design)

`RULES.md` is a user artifact (renamed old AGENTS.md), not a framework file. It sits in the project root but is never loaded by the harness. This is correct.

### Issue 3 — MAX_TOKENS_HARD_LIMIT Dead Code (Prior Session)
**Severity:** None (resolved)

`MAX_TOKENS_HARD_LIMIT` (65,536) was previously defined but never referenced — dead code with no clamping. Now resolved: `DEFAULT_MAX_TOKENS` matches it, and `/budget` enforces it.

---

## Next Topological Moves

1. Wire every-turn file content into the dynamic `<memory>` block (future enhancement)
2. Remove legacy CLAUDE.md references from remaining spec/ directory files
3. Verify end-to-end: start a session with `--dump-system-prompt` to confirm `<framework_identity>` and `<periodic_nudge>` injection
4. Commit logical unit