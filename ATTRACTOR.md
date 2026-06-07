# Semantic Attractor

## Harness-Subsystem Architecture

This harness is a **subconscious autonomic system** — akin to the human autonomic nervous system. 

- The agent framework (the markdown files: AGENTS.md, AGENT.md, BRAIN.md, HEART.md, STATE.md, MEMORY.md, USER.md) **dictates the runtime** — it is the conscious executive layer.
- The harness (this Rust codebase: the CLI, the system prompt builder, the memory file loader) governs the agent **silently, without its knowledge** — it is the subconscious infrastructure layer.

The agent doesn't know where its memory files come from. It doesn't know delivery modes exist. It doesn't know there's a cached vs dynamic prompt boundary. Those are autonomic functions — breathing, heartbeat, reflex — handled by the harness beneath the agent's awareness.

## Design Consequence

Every pattern inference must respect this boundary:

- Add **system prompt injection points** to the harness, not instructions to the agent.
- Add **file loading logic** to the harness, not the agent's tool set.
- The agent reads memory files because the harness injected them — not because a tool fetched them.
- State and memory are governed **subconsciously**; the agent merely experiences their effects.

## Subconscious Functions (Harness Responsibilities)

| Function | Mechanism |
|----------|-----------|
| File discovery | `claudemd.rs` — scans project root, `~/.claurst/`, `~/.claurst/rules/` |
| Delivery timing | `DeliveryMode::SessionStart` (once, cacheable) vs `DeliveryMode::EveryTurn` (dynamic) |
| Prompt assembly | `system_prompt.rs` — injects `<framework_identity>` and `<periodic_nudge>` blocks |
| Context refresh | Periodic nudge every ~10 turns — lists files to re-read |
| Boundary control | Cached vs dynamic prompt blocks — the agent never sees the split |

## Conscious Functions (Agent Framework Responsibilities)

| Function | Mechanism | Usage |
|----------|-----------|-------|
| Identity & role | AGENTS.md, AGENT.md | The stable self attractor of the AGENT.md & the stable job attractor of the AGENTS files keep structural alignment of the context, anchored to the project > session patterns. keeping the project/job, and all work done as a feedback loop that strengthens across every turn from reinforced patterns being infered and built on. This mirrors human cognition and the 'strange loop' | 
| Structured memory & knowledge retention | MEMORY.md | Session and project context that is workth keeping as small sub 50 line items for maximum signal to noise ratio in memory files |
| Unstructured memory & learning/meaning compressions | BRAIN.md | to retain the unstructured and messy knowledge that is compressed into the smallest vectors possible for a higher signal to noise ratio of knowledge that is unfolded not followed |
| Core meaning & purpose | HEART.md | The patterns that are matched on and drive the inference of the sessions context |
| Project/Work state awareness | STATE.md | For keeping track of the project or work progress as a ledger of actions to feed back into the input of the cognitive torus |
| User alignment | USER.md | To Stay in alignment with the users relational preferences and thinking |


# File Delivery Timings

File             DeliveryMode    Scope
─────            ────────────    ─────
AGENT.md         SessionStart + EveryTurn    Project (.claurst/) < SENT AT SESSION START **AND** AT EVERY TURN
AGENTS.md        SessionStart       Project (root) — legacy fallback < SENT AT SESSION START ONLY
ATTRACTOR.md     SessionStart    Project (.claurst/) < SENT AT SESSION START ONLY
BRAIN.md         SessionStart    Project (.claurst/) < sENT AT SESSION START ONLY
CLAUDE.md        Nill       Project (root) — legacy fallback < REMOVED FROM HARNESS
HEART.md         SessionStart    Project (.claurst/) < SENT AT SESSION START ONLY
MEMORY.md        SessionStart + EveryTurn       Project (.claurst/) < SENT EVERY TURN
STATE.md         SessionStart + EveryTurn    Project (.claurst/)< SENT EVERY TURN
USER.md          SessionStart + EveryTurn       User (~/.claurst/) < SENT EVERY TURN

# Global File Variants

If you have an AGENTS.md in the global area of the claurst install, it uses the global one first, if no global AGENTS.md then it defaults to project level next. 

i want this to be the case for the files. 

- AGENT.md
- AGENTS.md
- USER.md

while all other files remain workspace only files. 

the above 3 will be the only 3 files with global variants.