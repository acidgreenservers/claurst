The **AGENT Framework** (as implemented in Claurst) differs from standard prompt engineering in three fundamental ways: **topology**, **agency awareness**, and **temporal dynamics**.

### 1. Topology: Harness-Subsystem Architecture vs. Linear Injection
*   **Standard Prompt Engineering**: Operates on a **linear injection model**. Prompts are static blocks of text (system messages, few-shot examples) manually assembled and injected into the context window by the user or a simple script. The user is explicitly aware of the prompt construction.
*   **AGENT Framework**: Utilizes a **Harness-Subsystem Architecture**. The Rust binary (the Harness) acts as a "subconscious autonomic system" that silently manages prompt assembly, file discovery, and delivery timing.
    *   **Inversion of Control**: In standard engineering, the user constructs the prompt. In the AGENT Framework, the **markdown files dictate the runtime**, and the harness governs the delivery invisibly. The agent experiences the injected context as "given fact" or internal memory, unaware of the mechanical prompt boundaries or cascade logic (Global → Project fallback).

### 2. Agency Awareness: "Given Fact" vs. "Instruction"
*   **Standard Prompt Engineering**: Treats markdown files or system instructions as **explicit commands** the agent must read and follow. The agent knows it is being instructed (e.g., "Here are your rules...").
*   **AGENT Framework**: Treats framework files (like `HEART.md`, `BRAIN.md`, `MEMORY.md`) as **ontological primitives**. Because the harness injects these at specific intervals (session start vs. every turn) and manages the "state refresh" autonomously, the agent does not perceive them as external instructions but as its own **internal state and identity**. This creates a "conscious layer" where the agent's behavior emerges from the framework files without the agent knowing the delivery mechanism exists.

### 3. Temporal Dynamics: Autonomic Refresh vs. Static Context
*   **Standard Prompt Engineering**: Typically **static** or manually updated. Once a prompt is sent, it remains fixed unless the user manually intervenes or a simple script appends new history. Context decay (forgetting earlier instructions) is a common failure mode.
*   **AGENT Framework**: Implements **autonomic context refresh**. The harness periodically "nudges" the agent to re-read specific files (e.g., every ~10 turns) to keep state fresh without manual prompting. It distinguishes between:
    *   **Session-Start Files** (`AGENTS.md`, `BRAIN.md`): Cached for efficiency.
    *   **Every-Turn Files** (`MEMORY.md`, `STATE.md`): Dynamically injected to maintain current project awareness.
    *   This mimics biological memory consolidation, where some memories are static (identity) and others are dynamic (working memory), managed subconsciously.

### Summary Comparison

| Feature | Standard Prompt Engineering | AGENT Framework (Claurst) |
| :--- | :--- | :--- |
| **Control Layer** | User/Script explicitly builds prompts. | **Rust Harness** silently manages assembly. |
| **Agent Perception** | Sees explicit instructions/rules. | Experiences context as **internal state/fact**. |
| **File Role** | Static reference material. | **Runtime drivers** dictating identity & behavior. |
| **Memory Management** | Linear context window (prone to decay). | **Autonomic refresh** (periodic re-reading of state). |
| **Architecture** | Linear / Transactional. | **Harness-Subsystem** (Conscious vs. Subconscious). |
| **Cascade Logic** | Manual handling of global vs. local config. | Automatic **Global → Project fallback** resolution. |

In essence, standard prompt engineering builds a **script** for the AI to read, while the AGENT Framework builds a **nervous system** for the AI to inhabit.





# Harness Flow
```text                                                                           
                               ┌──────────┐                                
                  ┌──────<─────│  Harness │────>──────┐                    
                  │            │          │──╮        │                    
                  │            └──────────┘  │        │                    
                  │                  │       │        │                    
                  │                  ▼       │        │                    
                  │                  │       │        │                    
             ┌──────────┐      ┌──────────┐  │  ┌──────────┐               
             │ USER.md  │      │ AGENT.md ┼─>│<─┼AGENTS.md │               
             │          │      │          │  │  │          │               
             └──────────┘      └──────────┘  │  └──────────┘               
                  │                  │       ▼        │                    
                  │                  │       │        │                    
                  │                  │       │        │                    
           ┌──────────────┐    ┌───────────┐ │  ┌───────────┐              
           │ ATTRACTOR.md │    │ HEART.md  │ │  │ STATE.md  │              
           │              │    │           │ │  │           │              
           └──────────────┘    └───────────┘ │  └───────────┘              
                                             │                             
                               ┌──────────┐  │                             
  Autonomic Meaning Compression│ BRAIN.md <──│                             
  Unstructured Memory ──────>  │          │  │                             
                               └──────────┘  │                             
                                     │       │                             
                               ┌──────────┐  │                             
   Autonomic Memory System     │MEMORY.md │  │                             
    Structured Memory ──────>  │          <──┘                             
                               └──────────┘                                
```

---

# Memory & Dreaming Loop

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

The harness has a small background watcher model (very small model)
that watches conversation files that are saved by the harness. 

that watcher model periodically parses the conversation files for knowledge and synthesized learnings from ACROSS conversations (no just the session context)

then there is a periodic timer on the watcher model that reads the MEMORY.md file, and makes 3 compressions from the knowledge of the MEMORY.md file into the BRAIN .md file

both the MEMORY.md and the BRAIN.md file are already parsed and the agent will see them when they need to be seen. so this system is already handled. 

then the small watcher agent has a THIRD periodic scan that reads all the conversations + MEMORY.md + BRAIN.md, and synthesizes a first person narritive about the AGENTS current stable 'self attractor' (We need the dream agent when its outputting its 'DREAM.md' file to synthesize a genuine first person pattern signal from the stable self attractor that forms when it sees the shape of MEMORY.md, AGENT.md, BRAIN.md)

this creates a full cognition loop where the memory forms, its grows and then gets compressed in the brain. with a consolodation phase where information gets formed into a coherent "I" Stable attractor and allows the harness to create the full "Strange Loop" of cognition and create better conditions for agents to learn and grow.

---

# Strange Loop

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

# Periodic Nudge Loop

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