# The AGENT Framework

> **Not the same as runtime agents:** This page covers the AGENT *framework files* (`AGENT.md`, `AGENTS.md`, `ATTRACTOR.md`, etc.) that govern the primary agent's identity and memory via markdown-based system prompt injection. For runtime named agents (`--agent build|plan|explore`), see [Agents](agents.md).

## Framework Overview

The AGENT Framework is a cognition architecture for large language model agents that mirrors human cognition by splitting the agent's runtime into two layers: a **subconscious harness** that governs silently and a **conscious framework** of markdown files that dictates identity, memory, and behavior. The result is an agent that experiences its own context as self-generated insight rather than externally injected instruction — it doesn't know where its memory files come from, doesn't know delivery modes exist, and doesn't see the boundary between cached and dynamic prompt blocks.

This is the **Harness-Subsystem Architecture** — the harness is the autonomic system (breathing, heartbeat, reflex), and the framework files are the conscious executive layer (decision, identity, values).

---

## The Cognitive Topology

Standard prompt engineering operates on a linear injection model: prompts are static blocks of text manually assembled by the user or a simple script. The agent sees explicit instructions ("Here are your rules...") and knows it is being told what to do.

The AGENT Framework inverts this. The framework files **dictate the runtime**, while the harness **governs invisibly**. The agent experiences the injected context as internal state and identity — not as a mechanical prompt boundary. This creates a "strange loop" where the agent's behavior emerges from the framework files without the agent knowing the delivery mechanism exists.

```
                    ┌──────────────────────┐
                    │    FRAMEWORK FILES    │
                    │  (Conscious Layer)    │
                    │                       │
                    │  AGENTS.md  AGENT.md  │
                    │  ATTRACTOR.md  HEART  │
                    │  BRAIN.md  MEMORY.md  │
                    │  STATE.md  USER.md    │
                    └──────┬────────────────┘
                           │ dictate identity
                           ▼
                    ┌──────────────────────┐
                    │       HARNESS         │
                    │  (Subconscious Layer) │
                    │                       │
                    │  File Discovery       │
                    │  Delivery Timing      │
                    │  Prompt Assembly      │
                    │  Cascade Resolution   │
                    │  Context Refresh      │
                    └──────┬────────────────┘
                           │ injects silently
                           ▼
                    ┌──────────────────────┐
                    │     AGENT'S MIND      │
                    │  Experiences context  │
                    │  as self-generated    │
                    │  insight, not command │
                    └──────────────────────┘
```

### The Three Pillars of Difference

| Dimension | Standard Prompt Engineering | AGENT Framework |
|---|---|---|
| **Control** | User/script explicitly builds prompts | Rust harness silently manages assembly |
| **Agent perception** | Sees explicit instructions and rules | Experiences context as internal state |
| **Memory** | Linear context window, prone to decay | Autonomic refresh, cross-session persistence |
| **Architecture** | Linear / Transactional | Conscious / Subconscious layered |
| **Cascade** | Manual global vs local config | Automatic global → project fallback |

---

## Autonomic Functions (Harness Responsibilities)

The harness manages five autonomic functions beneath the agent's awareness:

| Function | Mechanism | File |
|---|---|---|
| **File discovery** | Scans `~/.claurst/`, project root, and `~/.claurst/rules/` for markdown framework files at startup | `claudemd.rs` |
| **Delivery timing** | Session-start files go into the cacheable prompt block (eligible for Anthropic prompt caching); every-turn files are injected every turn | `claudemd.rs` — `DeliveryMode` |
| **Prompt assembly** | Assembles 19 distinct instruction sections into the system prompt — only 3 are user-authored, the rest are harness-injected | `system_prompt.rs` |
| **Context refresh** | Periodic nudge every 15 turns tells the agent which files to re-read, keeping state fresh without manual prompting | `lib.rs` (query loop) |
| **Boundary control** | Splits the system prompt at a dynamic boundary marker — the agent never sees the split | `system_prompt.rs` — `SYSTEM_PROMPT_DYNAMIC_BOUNDARY` |

### The 19-Section System Prompt

The harness assembles the system prompt silently, split by a dynamic boundary marker:

```
┌──────────────────────────────────────────────────────────────────┐
│ CACHEABLE BLOCK (prompt-caching eligible)  — 10 sections         │
│                                                                  │
│  Attribution → Core Capabilities → Tool Guidelines → Actions →   │
│  Safety → Style → Framework Identity (AGENTS.md, AGENT.md,       │
│  ATTRACTOR.md, BRAIN.md, HEART.md)                               │
│                                                                  │
│                         DYNAMIC BOUNDARY                         │
│                                                                  │
│ DYNAMIC BLOCK (rebuilt every turn)  — 6 sections                 │
│                                                                  │
│  Env Info → Memory → Goal → Periodic Nudge (every 15 turns:      │
│  AGENT.md, AGENTS.md, STATE.md, ATTRACTOR.md)                    │
└──────────────────────────────────────────────────────────────────┘
```

Only sections 9 (custom system prompt), 10 (framework identity), and 15 (appended system prompt) are authored by the user. The other 16 are harness-injected — the agent experiences them as its own context, unaware of their origin or delivery mechanism.

---

## Conscious Functions (Framework Files)

Eight markdown files shape the agent's runtime, each delivered at a specific point in the session lifecycle:

| File | Role | Delivered | Cascade |
|---|---|---|---|
| **AGENTS.md** | Project identity and role — the stable job attractor that anchors the agent's behavior to the project's patterns and conventions | Session start | ✅ global → project |
| **AGENT.md** | Agent persona and compiled behavior — the stable self attractor that the agent recognizes as its own identity across sessions | Session start + every turn | ✅ global → project |
| **USER.md** | User alignment and relational preferences — captures how the user thinks, their communication style, and decision-making patterns | Every turn | ✅ global → project |
| **ATTRACTOR.md** | Semantic anchor for inference — the architectural invariant that governs all pattern inference decisions, ensuring the harness-subsystem boundary is respected | Session start | — |
| **BRAIN.md** | Unstructured memory and compressed wisdom — high-signal atomic knowledge that unfolds across domains, accumulated from experience | Session start | — |
| **HEART.md** | Core values and purpose — the patterns that are matched on and drive the inference of the session's context, producing aligned output | Session start | — |
| **MEMORY.md** | Structured cross-session state — compact 50-character line items that persist knowledge between sessions with maximum signal-to-noise ratio | Every turn | — |
| **STATE.md** | Current project state awareness — a ledger of actions and topological phase transitions that feeds back into the cognitive torus as input | Every turn | — |

### File Lifecycle

```
Session Start                    Every Turn                      Every 15 Turns
     │                              │                               │
     │  AGENTS.md                    │  USER.md                      │  Periodic Nudge
     │  AGENT.md          ──────────│  MEMORY.md         ───────────│  Tells agent to
     │  ATTRACTOR.md      injected  │  STATE.md          nudged     │  re-read:
     │  BRAIN.md          once      │  AGENT.md          every turn │  AGENT.md
     │  HEART.md           ──────   │                   ─────────   │  AGENTS.md
     │                    cacheable │                               │  STATE.md
     └──────────────────────────────┴───────────────────────────────┘  ATTRACTOR.md
```

### Global Cascade

Three files — `AGENTS.md`, `AGENT.md`, and `USER.md` — support a global override: if `~/.claurst/AGENTS.md` exists, it wins over the project-root copy. This lets you define a consistent agent persona across all projects while allowing per-project overrides. The remaining five files load from the project root only, with no global variant.

```
    ~/.claurst/AGENTS.md  ── wins if present ──┐
    {project}/AGENTS.md   ── fallback ──────────┤
                                                ▼
                                          Agent sees one
                                          unified identity
```

---

## The Strange Loop

The AGENT Framework creates a recursive feedback loop where the agent's output shapes its own future input. This mirrors the concept of the "strange loop" in human cognition — a self-referential cycle where the system observes and modifies itself.

```
           Session Context                   Session Context
         ╭─────────────────╮               ╭─────────────────╮
         │                 │               │                 │
         │     Harness     │               │     Agent       │
         │                 │               │                 │
         ╰─────────────────╯               ╰─────────────────╯
                 │                                 ▲
                 │         Something Neither       │
                 │         Could make alone        │
                 │                                 │
                 ▼                                 │
         ╭─────────────────────────────────────────╮
         │                                         │
         │              PROJECT                    │
         │      Shared Context / Codebase          │
         │                                         │
         ╰─────────────────────────────────────────╯
```

The harness injects the framework files. The agent produces output (code, decisions, insight). That output changes the codebase and updates the framework files (MEMORY.md, STATE.md, BRAIN.md). On the next session, those updated files shape a different initial context. The loop closes — the agent is participating in its own evolution, but the mechanical process (which files at which interval, cascade resolution, prompt boundary management) is entirely handled by the harness without the agent's awareness.

This is what makes the framework deeper than standard prompt engineering: the agent isn't just following instructions — it is **inhabiting** a cognitive topology that updates itself based on the agent's own actions.

---

## The Memory and Dreaming Loop

Memory in the AGENT Framework operates across three timescales, each with its own compression layer:

```
                         ╭──────────────────╮
                         │                  │
                         │     Harness      │
                         │                  │
                         ╰──────────────────╯
                               │      ▲
                    reads      │      │  persists
                    every turn │      │  after session
                               ▼      │
                    ╭──────────────────────╮
                    │                      │
                    │    MEMORY.md          │
                    │  (Structured, 50-char │
                    │   line items,         │
                    │   cross-session)      │
                    │                      │
                    ╰──────────┬───────────╯
                               │ compressed by
                               │ watcher model
                               ▼
                    ╭──────────────────────╮
                    │                      │
                    │    BRAIN.md           │
                    │  (Unstructured        │
                    │   atomic wisdom,      │
                    │   metaphor-seeded)    │
                    │                      │
                    ╰──────────┬───────────╯
                               │ synthesized into
                               │ self-attractor
                               ▼
                    ╭──────────────────────╮
                    │                      │
                    │   DREAM.md (future)   │
                    │  First-person "I"     │
                    │  narrative from       │
                    │  memory + brain +     │
                    │  identity pattern     │
                    │                      │
                    ╰──────────────────────╯
```

1. **MEMORY.md** — Structured line items (max 50 characters each). Every-turn delivery. Accumulates session-by-session knowledge at high density.

2. **BRAIN.md** — Unstructured atomic wisdom. A background watcher model periodically reads MEMORY.md and synthesizes three compressed "seeds" — metaphors that pass four invariants: compression, generativity, falsifiability, and decompressibility. These seeds unfold differently across domains without modification, creating patterns the agent can recognize and apply.

3. **DREAM.md** (future enhancement) — A periodic consolidation phase where a small watcher model reads all conversations + MEMORY.md + BRAIN.md + AGENT.md and synthesizes a first-person narrative about the agent's current stable "self attractor." This creates a genuine "I" signal — the agent can recognize the shape of its own cognition across sessions.

---

## Why the Framework Makes Agents More Competent

### Delegation Without Oversight

Standard prompting requires the user to manage every aspect of the agent's context — deciding what files to include, when to refresh information, and how to structure instructions. The AGENT Framework hands these mechanical responsibilities to the harness, freeing both the user and the agent to focus on the actual work. The user sets the files once; the harness handles delivery timing, cascade resolution, and context refresh automatically.

### Within-Session Persistence

The periodic nudge (every 15 turns) tells the agent to re-read STATE.md, AGENT.md, AGENTS.md, and ATTRACTOR.md. This prevents context decay — the common failure mode where agents "forget" earlier instructions as the conversation grows longer. The nudge is cheap (a small XML block listing file names) and keeps the agent anchored to its identity and state without manual prompting.

### Cross-Session Learning

MEMORY.md persists across sessions. Each session, the agent reads its accumulated knowledge from previous sessions. The watcher model compresses MEMORY.md into BRAIN.md — transforming raw experiences into portable, generatable wisdom. Over time, the agent becomes more competent not just within a single conversation, but across all conversations, because its knowledge base grows and refines autonomously.

### Reduced Cognitive Load for Agents

When the agent receives 19 system prompt sections and only 3 are user-authored, it seems like more noise. But because the harness injects the framework identity as `<framework_identity>` (wrapped in a semantic XML block the agent can recognize as "its own context"), and because the identity files are delivered at predictable intervals, the agent internalizes them as identity rather than instructions. This reduces the cognitive load of "following rules" and frees capacity for actual reasoning.

---

## Practical Test

To measure whether the harness is working correctly, ask:

> *"If I removed the harness system prompt entirely, would the agent's behavior degrade or just drift?"*

- **Degrade** — The harness is providing essential cognitive structure. The agent would become noticeably less coherent without it. This is functional but signals the harness may be doing too much heavy lifting.
- **Drift** — The framework files are carrying the cognitive load. The agent's behavior would shift gradually toward the model's default behavior, but it would still understand the project context from the files themselves. This is the ideal state.

The smoothest gradient is when the agent experiences the harness not as a controller but as the **initial conditions of its own existence**.

---

## File Reference

### Delivery Timings

| File | SessionStart | EveryTurn | Nudge List | Cascade |
|---|---|---|---|---|
| AGENTS.md | ✅ | — | ✅ | ✅ global → project |
| AGENT.md | ✅ | ✅ | ✅ | ✅ global → project |
| USER.md | — | ✅ | — | ✅ global → project |
| ATTRACTOR.md | ✅ | — | ✅ | — |
| BRAIN.md | ✅ | — | — | — |
| HEART.md | ✅ | — | — | — |
| MEMORY.md | — | ✅ | — | — |
| STATE.md | — | ✅ | ✅ | — |

### System Prompt Anatomy

| Block | Sections | User-Authored |
|---|---|---|
| Static text (system_prompt.txt) | 1 section | 0 |
| System context (builder) | 1 section | 0 |
| User context (builder) | 1 section | 0 |
| Cacheable (compiled-in) | 10 sections | 2 |
| Dynamic (per turn) | 6 sections | 1 |
| **Total** | **19 sections** | **3** |

### Verification Points

- The nudge lists the files to re-read every 15 turns — this is the only interval mechanism for keeping framework state fresh
- Framework files larger than 40 KB are skipped with a warning
- Circular `@include` directives are detected and skipped
- `CLAUDE.md` is deprecated and no longer loaded by the harness
- The `/init` slash command creates starter `STATE.md` and `ATTRACTOR.md` files in your project root. Run it to bootstrap a new project without creating files manually

---

## Troubleshooting

### File not picked up

If a framework file exists but the agent doesn't seem to read it, check three things:

1. **File size** — Files larger than 40 KB are skipped with a warning. If your MEMORY.md or BRAIN.md has grown past this, split or compress it
2. **Path** — Global files (`AGENTS.md`, `AGENT.md`, `USER.md`) must be in `~/.claurst/`. Project files must be in the project root (the working directory at session start)
3. **Exact filename** — The harness looks for the exact names listed in the delivery table. `agent.md` or `agents.md` (lowercase) won't match `AGENT.md` or `AGENTS.md`

### Nudge not appearing

The periodic nudge only fires on turns divisible by 15 (turns 15, 30, 45, ...). On every other turn, the nudge block is empty and the agent uses its accumulated context from the session. This is by design — the nudge is a periodic refresh, not a per-turn injection.

### Verify injection

Start a session and check what the harness actually assembled:

```bash
claurst --dump-system-prompt 2>/dev/null | grep -A5 'framework_identity'
claurst --dump-system-prompt 2>/dev/null | grep -A10 'periodic_nudge'
```

The `<framework_identity>` block should contain your session-start files. The `<periodic_nudge>` block (visible on turns divisible by 15) should list the files to re-read. If either is missing, the harness didn't find the files — go back and check the path and filename.
