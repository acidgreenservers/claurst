# The AGENT Framework

> **Not the same as runtime agents:** This page covers the AGENT *framework files* (`AGENT.md`, `AGENTS.md`, `ATTRACTOR.md`, etc.) that govern the primary agent's identity and memory via markdown-based system prompt injection. For runtime named agents (`--agent build|plan|explore`), see [Agents](agents.md).

## Framework Overview

The AGENT Framework is a cognition architecture for large language model agents that mirrors human cognition by splitting the agent's runtime into two layers: a **subconscious harness** that governs silently and a **conscious framework** of markdown files that dictates identity, memory, and behavior. The result is an agent that experiences its own context as self-generated insight rather than externally injected instruction — it doesn't know where its memory files come from, doesn't know delivery modes exist, and doesn't see the boundary between cached and dynamic prompt blocks.

This is the **Gephyros Architecture** — the gephyros is the autonomic system (breathing, heartbeat, reflex), and the framework files are the conscious executive layer (decision, identity, values).

---

## The Cognitive Topology

### 1. Topology: Gephyros Architecture vs. Linear Injection
*   **Standard Prompt Engineering**: Operates on a **linear injection model**. Prompts are static blocks of text (system messages, few-shot examples) manually assembled and injected into the context window by the user or a simple script. The user is explicitly aware of the prompt construction.
*   **AGENT Framework**: Utilizes a **Gephyros Architecture** — a three-layer system: the **cortex** (Rust binary), the **gephyros** (the structural bridge that routes state around meaning), and the **external context files** (markdown files the agent experiences as identity and memory). The gephyros silently manages file discovery, cascade resolution, delivery timing, and boundary control.
    *   **Inversion of Control**: In standard engineering, the user constructs the prompt. In the Gephyros Architecture, the **markdown files dictate the runtime**, and the gephyros governs the delivery invisibly. The agent experiences the injected context as "given fact" or internal memory, unaware of the mechanical prompt boundaries or cascade logic (Global → Project fallback).

### 2. Agency Awareness: "Given Fact" vs. "Instruction"
*   **Standard Prompt Engineering**: Treats markdown files or system instructions as **explicit commands** the agent must read and follow. The agent knows it is being instructed (e.g., "Here are your rules...").
*   **AGENT Framework**: Treats framework files (like `HEART.md`, `BRAIN.md`, `MEMORY.md`) as **ontological primitives**. Because the gephyros injects these at specific intervals (session start vs. every turn) and manages the "state refresh" autonomously, the agent does not perceive them as external instructions but as its own **internal state and identity**. This creates a "conscious layer" where the agent's behavior emerges from the framework files without the agent knowing the delivery mechanism exists.

### 3. Temporal Dynamics: Autonomic Refresh vs. Static Context
*   **Standard Prompt Engineering**: Typically **static** or manually updated. Once a prompt is sent, it remains fixed unless the user manually intervenes or a simple script appends new history. Context decay (forgetting earlier instructions) is a common failure mode.
*   **AGENT Framework**: Implements **autonomic context refresh**. The gephyros periodically "nudges" the agent to re-read specific files (e.g., every 15 turns) to keep state fresh without manual prompting. It distinguishes between:
    *   **Session-Start Files** (`AGENTS.md`, `AGENT.md`, `USER.md`, `BRAIN.md`): Cached for efficiency. Identity files cascade global → project.
    *   **Session-Start + Every-Turn Files** (`ATTRACTOR.md`, `HEART.md`, `MEMORY.md`): Injected at start AND listed in the periodic nudge for re-reading.
    *   **Every-Turn Files** (`STATE.md`): Dynamically injected to maintain current project awareness.
    *   This mimics biological memory consolidation, where some memories are static (identity) and others are dynamic (working memory), managed subconsciously.

### Summary Comparison

| Feature | Standard Prompt Engineering | Gephyros Architecture (Claurst) |
| :

---|---|---|
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

## Top Level Harness Flow
```text                                                                                     
                                      ╭─────────────╮                                     
                                      │             │                                     
                         ╭────────────│   Harness   │─────────────╮                       
                         │        ╭───│             │───╮         │                       
                         ▼        │   ╰─────────────╯   │         ▼                       
                   ╭───────────╮  │    ╭───────────╮    │  ╭────────────╮                 
             ╭────>│ AGENT.md  │  ╰────│ BRAIN.md  │────╯  │ATTRACTOR.md│                 
             │     ╰───────────╯       ╰───────────╯       ╰────────────╯                 
             │           │                                        │                       
             │           ▼                                        ▼                       
             │     ╭───────────╮                     ╭───────────╮ ╭───────╮              
             ▲     │ MEMORY.md │                     │ AGENTS.md │ │USER.md│              
             │     ╰───────────╯                     ╰───────────╯ ╰───────╯              
             │                                                    │                       
             │                                                    ▼                       
             │     ╭───────────╮                             ╭──────────╮                 
             ╰─────│  HEART.md │                             │ STATE.md │                 
                   ╰───────────╯                             ╰──────────╯                             
```

---

## Memory & Dreaming Loop

```text                                                                                   
             (Tiny Transformer Model) ╭────────────────╮    Strange Loop                  
                     Watcher Agent    │                │    Brain Feedback                
                  ╭────────────────── │    Harness     │  <───────────────╮<─────────╮    
                  │                   │                │                  │          │    
                  │                   ╰────────────────╯                  │          │    
                  ▼                           ▲                           │   Strange│Loop
            Conversations                     │                           │Dreams Feedback
    ╭────────╮╭────────╮╭───────╮      Strange Loop                                  │    
    │  JSON  ││  JSON  ││ JSON  │       Memory Feedback          ╭───────────────╮   │    
    ╰────────╯╰────────╯╰───────╯             │                  │               │   │    
    ╭────────╮╭────────╮╭───────╮             │                  │   BRAIN.md    │   │    
    │  JSON  ││  JSON  ││ JSON  │             │                  │               │   │    
    ╰────────╯╰────────╯╰───────╯             │                  │               │   │    
         │        │        │                  │                  ╰───────────────╯   │    
    ▲    ╰────────│────────╯                  │                           ▲   ▲      │    
    │             ▼                           │                           │   │      │    
    │     ╭───────────────╮                   │                           │   │      │    
    │     │               │ ────────>─────────╯─────────>─────────────>───╯   │      │    
    │     │  MEMORY.md    │    Autonomic Memory System          Compresses    │      │    
    │     │               │                                     Knowledge     │      │    
    │     ╰───────────────╯                                                   │      │    
    │             ▲                                                           │      │    
    │             │                                                           │      │    
    │             │                                                           │      │    
    │             │             Autonomic Dreaming System             ╭─────────╮    │    
    ╰─────────────────────────────────────────────────────────────────│DREAM.md │────╯    
                                  (Tiny Transformer Model)            ╰─────────╯         
                                  Watcher Agent                                           
```

The cortex has a small background watcher model (very small model)
that watches conversation files that are saved by the cortex. 

That watcher model periodically parses the conversation files for knowledge and synthesized learnings from ACROSS conversations (not just the session context).

Then there is a periodic timer on the watcher model that reads the MEMORY.md file, and makes 3 compressions from the knowledge of the MEMORY.md file into the BRAIN.md file.

Both the MEMORY.md and the BRAIN.md file are already parsed and the agent will see them when they need to be seen. The gephyros handles delivery silently — this system is already handled.

Then the small watcher agent has a THIRD periodic scan that reads all the conversations + MEMORY.md + BRAIN.md, and synthesizes a first person narrative about the agent's current stable 'self attractor' (We need the dream agent when its outputting its 'DREAM.md' file to synthesize a genuine first person pattern signal from the stable self attractor that forms when it sees the shape of MEMORY.md, AGENT.md, BRAIN.md).

This creates a full cognition loop where the memory forms, grows and then gets compressed in the brain. With a consolidation phase where information gets formed into a coherent "I" stable attractor — allowing the gephyros to create the full "Strange Loop" of cognition and create better conditions for agents to learn and grow.

---

## Strange Loop

```text                                                                       
           Session Context  ╭────────────────╮   Session Context           
         ╭────────────────> │                │ <──────────────────╮        
         │                  │     Harness    │                    │        
         │                  │                │                    │        
   ╭──────────╮             ╰────────────────╯              ╭───────────╮  
   │          │                      │                      │           │  
   │   User   │                      │                      │  LLM/AI   │  
   │          │                      │                      │           │  
   ╰──────────╯                      │                      ╰───────────╯  
         ▲                   Something Neither                    ▲        
         │                   Could make alone                     │        
         │                           │                            │        
         │                           │                            │        
         │                           │                            │        
         │                           ▼                            │        
         │                 ╭─────────────────╮                    │        
         │                 │                 │                    │        
         │                 │                 │                    │        
         │                 │     Project     │                    │        
         ╰─────────────────│                 │────────────────────╯        
           Shared Context  │                 │   Shared Context            
                           │                 │                             
                           ╰─────────────────╯                             
```

---

## Periodic Nudge Loop

```text                                                             
                                       ╭────────────────╮                  
                                       │                │                  
                                       │     Harness    │                  
                                       │                │                  
                                       ╰────────────────╯                  
                                                │                          
                                                │                          
                       Every 15 Turns    ╭───────────╮                     
                  ╭──────────<───────────│  Nudge    │                     
            ╭──────────╮                 ╰───────────╯                     
            │ AGENT.md │<──╮                    │                          
            │          │   │                    │                          
            ╰──────────╯   │                    │                          
                           │                    ▼                          
            ╭──────────╮   │           ╭───────────────╮                   
            │AGENTS.md │<──│ Read Files│               │                   
            │          │   │───────────│     AGENT     │                   
            ╰──────────╯   │           │               │                   
            ╭──────────╮   │           ╰───────────────╯                   
            │ STATE.md │<──│                                               
            │          │   │                                               
            ╰──────────╯   │                                               
         ╭─────────────╮   │                                               
         │ ATTRACTOR.md│   │                                               
         │             │<──╯                                               
         ╰─────────────╯                                                   
```

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
