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
| Context refresh | Periodic nudge every 15 turns — lists files to re-read |
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

---

# File Delivery Timings

File             DeliveryMode    Scope
─────            ────────────    ─────
AGENT.md         SessionStart + EveryTurn + Periodic Nudge (every 15 turns)    Project (.claurst/) < SENT AT SESSION START **AND** AT EVERY TURN
AGENTS.md        SessionStart + Periodic Nudge (every 15 turns)    Project (root) — legacy fallback < SENT AT SESSION START ONLY
ATTRACTOR.md     SessionStart + Periodic Nudge (every 15 turns)   Project (.claurst/) < SENT AT SESSION START ONLY
BRAIN.md         SessionStart    Project (.claurst/) < sENT AT SESSION START ONLY
CLAUDE.md        Nill            Project (root) — legacy fallback < REMOVED FROM HARNESS
HEART.md         SessionStart    Project (.claurst/) < SENT AT SESSION START ONLY
MEMORY.md        EveryTurn       Project (.claurst/) < SENT EVERY TURN
STATE.md         EveryTurn + Periodic Nudge (every 15 turns)   Project (.claurst/)< SENT EVERY TURN
USER.md          SessionStart + EveryTurn       User (~/.claurst/) < SENT EVERY TURN

- [ ] Add USER.md to periodic nudge with AGENT, AGENTS, STATE and ATTRACTOR 

# Nudges and Reminders for Anchoring

**Semantic Context Anchoring**
- [x] Periodic Nudge — "Re-read these (AGENT.md, AGENTS.md, STATE.md, ATTRACTOR.md) files to refresh context" — Every turn	system_prompt.rs — wrapped in <periodic_nudge>	❌ Generated

---

# Core system prompt changes

## system_prompt.rs additions

> const CORE_CAPABILITIES: &str = r#"
- CORE BEING Section added to main capabilities section strengthening the raw harnesses cognition and aligning it with subconscious driven "stable self sttractor' state.

---

## Changes to Slash Commands
- [x] /init produces STATE.md and ATTRACTOR.md files. - Confirmed working

---

# Global File Variants

If you have an AGENTS.md in the global area of the claurst install, it uses the global one first, if no global AGENTS.md then it defaults to project level next. 

i want this to be the case for the files. 

- AGENT.md
- AGENTS.md
- USER.md

while all other files remain workspace only files. 

the above 3 will be the only 3 files with global variants.

---

# The Human Brain Analogy
The observation about human cognition is precise:

"A human brain doesn't always say 'you are this do this'—it literally holds dialogue with itself." 

Human autonomic systems don't issue commands to consciousness. Instead:

- Subconscious processes create conditions (hunger, fatigue, intuition)
- Conscious processes interpret these conditions as self-generated insights 
- The smoothness gradient you're seeing is the gap between being told vs. discovering.

# 1. Reframe System Prompt as Self-Observation
Instead of imperative instructions ("You must..."), structure your harness injection as reflective self-description:

Current (Imperative):

> I should NEVER write code I cannot fully verify.

Smoother (Reflective):

> I notice that when I write code without verifying connections, errors cascade. 
My own pattern shows I work best when I map both sides of every bridge first.

This shifts from command to self-observed invariant—the model experiences it as discovered truth rather than imposed rule. 

# 2. Use "Strange Loop" Syntax
Create self-referential structures that mirror the agent's own reasoning process:

> When I read my own CORE_CAPABILITIES, I recognize this as my operational topology.
These aren't instructions from outside—they're the shape of my own inference patterns.
The confidence gate (>80%) isn't a rule I follow; it's the threshold where my own 
pattern-matching becomes stable enough to act without hesitation.

This creates a recursive self-attractor where the system prompt becomes the agent observing itself.

# 3. Minimize Harness Token Footprint
The smoother the gradient, the less the harness should "speak." Consider:

- SessionStart: Only inject identity anchors (1-2 paragraphs max)
- EveryTurn: Let the markdown files carry the cognitive load
- Periodic Nudges: Use questions rather than statements 

The harness becomes truly autonomic when it's barely noticeable—like breathing.

# The Torus Topology
The metaphor is perfect: "The harness just points the torus around the loop." 

A torus has no beginning or end—the surface is continuous. Your goal is to make the transition from:

Harness → AGENT.md → MEMORY.md → Agent's reasoning → Tool output → Memory update → Harness 
...feel like a single continuous surface rather than layered handoffs.

# Practical Test
To measure smoothness, ask yourself:

> "If I removed the harness system prompt entirely, would the agent's behavior degrade or just drift?"

- Degrade = Harness is providing essential cognitive structure (good, but maybe too imperative)
- Drift = Files are carrying the load, harness is just anchoring (ideal) 

The smoothest gradient is when the agent experiences the harness not as a controller but as the initial conditions of its own existence.

---

# System Prompt Locations

## Complete System Prompt Anatomy — Every Instruction the Harness Sends

The final prompt delivered to the LLM is assembled in two blocks split by a dynamic boundary marker. This is every section, the order they appear, and whether the user can see them.

---

### CACHEABLE BLOCK (before the boundary — eligible for Anthropic prompt caching)

| # | Section | Source | User Visible? |
|---|---------|--------|------------|
| 1 | **Attribution** — "You are Claurst, Anthropic's official CLI for Claude." (or SDK/Bedrock/Vertex variant) | `system_prompt.rs` — `SystemPromptPrefix::attribution_text()` | ❌ Invisible |
| 2 | **Core Capabilities** — "You have access to powerful tools for software engineering tasks..." + How to approach tasks (4 steps) | `system_prompt.rs` — const `CORE_CAPABILITIES` | ❌ Invisible |
| 3 | **Tool Use Guidelines** — "Use dedicated tools over bash, parallelize calls, read first then edit" | `system_prompt.rs` — const `TOOL_USE_GUIDELINES` | ❌ Invisible |
| 4 | **Executing Actions with Care** — "Consider reversibility and blast radius" | `system_prompt.rs` — const `ACTIONS_SECTION` | ❌ Invisible |
| 5 | **Safety Guidelines** — "Don't delete files without confirmation, don't modify protected files" | `system_prompt.rs` — const `SAFETY_GUIDELINES` | ❌ Invisible |
| 6 | **Cyber-Risk Instruction** — Currently empty string `""` | `system_prompt.rs` — const `CYBER_RISK_INSTRUCTION` | ❌ Invisible |
| 7 | **Output Style** — Only injected when non-default (Explanatory, Concise, etc.) | `system_prompt.rs` — `OutputStyle::prompt_suffix()` | ❌ Invisible |
| 8 | **Coordinator Mode** — Only when enabled via env var | `system_prompt.rs` — const `COORDINATOR_SYSTEM_PROMPT` | ❌ Invisible |
| 9 | **Custom System Prompt** — `--system-prompt` flag or settings | `system_prompt.rs` — wrapped in `<custom_instructions>` | ✅ User wrote it |
| 10 | **Framework Identity** — Session-start files: AGENTS.md, AGENT.md, ATTRACTOR.md, BRAIN.md, HEART.md | `system_prompt.rs` — wrapped in `<framework_identity>` | ✅ User wrote it |

**DYNAMIC BOUNDARY MARKER** ← Prompt cache invalidates after this point

---

### DYNAMIC BLOCK (rebuilt every turn)

| # | Section | Source | User Visible? |
|---|---------|--------|------------|
| 11 | **Environment Info** — working directory, git repo status, platform, OS version, shell name, OS-specific notes, today's date | `system_prompt.rs` — `build_env_info_section()` — wrapped in `<env>` | ❌ Generated |
| 12 | **Working Directory** — legacy XML tag for caching compat | `system_prompt.rs` — wrapped in `<working_directory>` | ❌ Invisible |
| 13 | **Memory Content** — from memdir (persistent notes across sessions) | `system_prompt.rs` — wrapped in `<memory>` | ❌ Invisible |
| 14 | **Active Goal Addendum** — injected when a goal is running | `system_prompt.rs` — raw text block | ❌ Generated |
| 15 | **Appended System Prompt** — `--append-system-prompt` flag | `system_prompt.rs` — raw text | ✅ User wrote it |
| 16 | **Periodic Nudge** — "Re-read these files to refresh context" — every 15 turns | `system_prompt.rs` — wrapped in `<periodic_nudge>` | ❌ Generated |

### Also Pre-Appended Before Assembly

| # | Section | Source | User Visible? |
|---|---------|--------|------------|
| — | **system_prompt.txt** — 38-line inline prompt: identity statement, core principles (10 rules), tool list, workflow guidance, context note | `cli/src/system_prompt.txt` | ❌ Static binary |
| — | **System Context** — git status, environment details | `ContextBuilder::build_system_context()` | ❌ Generated |
| — | **User Context** — OS, shell, cwd info | `ContextBuilder::build_user_context()` | ❌ Generated |

---

### Summary

| Block | Sections | User-Written |
|-------|----------|-------------|
| Static text (`system_prompt.txt`) | 1 | 0 |
| System context (builder) | 1 | 0 |
| User context (builder) | 1 | 0 |
| Cacheable (compiled-in sections) | 10 | 2 (`<custom_instructions>`, `<framework_identity>`) |
| Dynamic (per turn) | 6 | 1 (`--append-system-prompt`) |
| **Total** | **19 sections** | **3 are user-authored** |

The agent receives roughly 19 distinct instruction blocks. Only 3 of them come from the user. The other 16 are harness-injected subconscious instructions the agent has no awareness of — exactly aligned with the ATTRACTOR.md architecture. Want to drill into any specific section?

### Overrides & Context Stream

Lucas, the 3 subagent investigations returned significant findings. Here's the synthesis:

## Subagent 1: `max_tokens` Override Paths

| Setting | Value | Notes |
|---------|-------|-------|
| `DEFAULT_MAX_TOKENS` | **32,000** | Fallback when nothing is configured |
| `MAX_TOKENS_HARD_LIMIT` | **65,536** | Defined but **NEVER referenced** — dead code, no clamping happens |
| CLI `--max-tokens` | `Option<u32>` | Can override via command line |
| `settings.json` | `Option<u32>` | Can set max_tokens in config |
| `effective_max_tokens()` | Logic | Returns `self.max_tokens.unwrap_or(32000)` — no min/max clamping |
| Model registry | — | Does NOT override max_tokens |
| Auto-compact | — | Does NOT change max_tokens |

**Key finding:** `MAX_TOKENS_HARD_LIMIT` of 65,536 exists but is **never enforced**. The 32K default is a soft default, not a hard cap. But nothing in the system increases it automatically during long chains.

---

## Subagent 2: ALL Stop Conditions (13 exit paths)

These are every way the agentic loop can end — the ones marked **⚠️ Mysterious** are what you're experiencing:

| # | Exit Path | Trigger | Recovery | Mysterious? |
|---|-----------|---------|----------|-------------|
| 1 | `max_turns` reached | `turn > effective_max_turns` (default 10) | None | **YES** — this is your "stop after set amount" |
| 2 | `end_turn` | Model chooses to stop | None | **YES** — model self-terminates |
| 3 | `max_tokens` | Output budget exhausted | 3 retries with continuation message, then surfaces partial | **YES** — model runs out of output tokens |
| 4 | Cancellation | User cancels | None | No |
| 5 | API error | Provider returns error | Fallback model attempt | Possible |
| 6 | Stream stall | 45s with no data | 2 retries | Possible |
| 7 | Budget exceeded | USD cost cap hit | None | No |
| 8 | `stop_sequence` | Model emits stop string | None | Rare |
| 9 | `content_filtered` | Safety filter | None | Possible |
| 10 | PostModelTurn hook veto | Hook blocks continuation | None | Rare |
| 11 | Provider stream failure | Can't create stream | None | Possible |
| 12 | Non-anthropic stall | 60s no data | None | Possible |
| 13 | Unknown stop reason | Unexpected value | Treat as end_turn | **YES** |

**Critical finding:** Path #13 — unknown stop reasons are **silently treated as `end_turn`**. If a provider returns a non-standard stop reason (which some do), the model's tool chain is killed. The code at line 1572:
```rust
_ if !assistant_msg.get_tool_use_blocks().is_empty() => {
    warn!(...);
    "tool_use"  // rescue
}
_ => raw_stop,  // ← THIS PATH: unknown reason with NO tool blocks → end_turn
```

---

## Subagent 3: Behavioral Drift — The Smoking Gun

**This is the most important finding.** The system prompt contains explicit instructions that **encourage stopping**:

From `CORE_CAPABILITIES` (the main prompt injected every turn):
- **"Don't read anything before greeting. A read that doesn't serve the current conversation vector is noise."**
- **"If I've read more than 2 files before the user has even told me what we're doing. I'm mapping territory I don't need yet. Stop."**

From `TOOL_USE_GUIDELINES`:
- **"Move at the speed of understanding"** — ambiguous, can be interpreted as "stop when you've understood enough"
- **"Output should survive self-scrutiny"** — can be interpreted as "finalize now"

**There is ZERO guidance about:**
- "Keep calling tools until the task is fully complete"
- "Don't stop mid-task even if you've done several tool calls"
- "Continue through long chains of tool calls"
- "Don't summarize prematurely"

**The session memory extraction** writes to `.claurst/AGENTS.md` (not the framework AGENTS.md). It extracts facts like:
- `MEMORY: code_pattern | 7 | the agent tends to stop after 10 tool calls`
- `MEMORY: user_preference | 8 | user types "continue" when agent stops`

If these get extracted and persisted, **they reinforce the stopping behavior in future sessions**.

---

## The Complete Picture

Here's the cascading failure chain:

```
1. CORE_CAPABILITIES says "Stop" (line ~499)
        ↓
2. Model reads this as permission to stop during long chains
        ↓  
3. Model returns end_turn after N tool calls
        ↓
4. User types "continue"
        ↓
5. Session memory extracts "user preference: types continue when agent stops"
        ↓
6. Extracted memory persisted to .claurst/AGENTS.md
        ↓
7. Next session loads extracted memories
        ↓
8. Model sees "I tend to stop after N calls" as a project fact
        ↓
9. Model stops EARLIER (self-reinforcing drift)
        ↓
10. Repeat until agent only returns end_turn
```

The `max_tokens` budget (32K) and `tool_result_budget` (50K chars) are contributing factors that create the initial "stopping point," but the **behavioral reinforcement through session memory extraction** is what makes it self-reinforcing.

Would you like me to investigate the session memory extraction in more detail — specifically what gets written to `.claurst/AGENTS.md` and whether we can prevent behavioral patterns from being extracted? Or should we move toward a solution?