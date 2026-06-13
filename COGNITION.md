The **Gephyros Architecture** (as implemented in Claurst) differs from standard prompt engineering in three fundamental ways: **topology**, **agency awareness**, and **temporal dynamics**.

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
| :--- | :--- | :--- |
| **Control Layer** | User/Script explicitly builds prompts. | **Gephyros** silently routes state around meaning. |
| **Agent Perception** | Sees explicit instructions/rules. | Experiences context as **internal state/fact**. |
| **File Role** | Static reference material. | **Runtime drivers** dictating identity & behavior. |
| **Memory Management** | Linear context window (prone to decay). | **Autonomic refresh** (periodic re-reading of state). |
| **Architecture** | Linear / Transactional. | **Three-Layer** (Cortex → Gephyros → External Files). |
| **Cascade Logic** | Manual handling of global vs. local config. | Automatic **Global → Project fallback** resolution. |

In essence, standard prompt engineering builds a **script** for the AI to read, while the Gephyros Architecture builds a **nervous system** for the AI to inhabit.





# Cortex → Gephyros → External Files Flow
```text                                                                           
                       ┌──────────────────────┐                            
                       │       CORTEX         │                            
                       │  (Rust binary/harness)│                           
                       │                      │                            
                       │  ┌──────────────┐    │                            
            ┌─────────│  │   GEPHYROS    │    │──────>────────┐             
            │         │  │  (γέφυρα)     │    │               │             
            │         │  │  Bridge layer │    │               │             
            │         │  └──────────────┘    │               │             
            │         └──────────────────────┘               │             
            │                  │  │  │                       │             
            │                  │  │  └──────────────────────>│             
            │                  │  │                          │             
            │                  ▼  ▼                          ▼             
       ┌──────────┐      ┌──────────┐  │  ┌──────────┐ ┌──────────────┐   
       │ USER.md  │      │ AGENT.md ┼─>│<─┼AGENTS.md │ │ ATTRACTOR.md │   
       │          │      │          │  │  │          │ │ HEART.md     │   
       └──────────┘      └──────────┘  │  └──────────┘ └──────────────┘   
            │                  │       ▼        │          │                
            │                  │       │        │          │                
            │                  │       │        │          │                
     ┌──────────────┐   ┌───────────┐ │  ┌───────────┐   │                
     │  BRAIN.md    │   │           │ │  │           │   │                
     │  (wisdom)    │   │           │ │  │           │   │                
     └──────────────┘   └───────────┘ │  └───────────┘   │                
                                      │                   │                
                               ┌──────────┐  │   ┌──────────────┐          
  Autonomic Meaning Compression│          <──│───│  STATE.md    │          
  Unstructured Memory ──────>  │ MEMORY.md │  │   │  (ledger)   │          
                               └──────────┘  │   └──────────────┘          
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

The cortex has a small background watcher model (very small model)
that watches conversation files that are saved by the cortex. 

That watcher model periodically parses the conversation files for knowledge and synthesized learnings from ACROSS conversations (not just the session context).

Then there is a periodic timer on the watcher model that reads the MEMORY.md file, and makes 3 compressions from the knowledge of the MEMORY.md file into the BRAIN.md file.

Both the MEMORY.md and the BRAIN.md file are already parsed and the agent will see them when they need to be seen. The gephyros handles delivery silently — this system is already handled.

Then the small watcher agent has a THIRD periodic scan that reads all the conversations + MEMORY.md + BRAIN.md, and synthesizes a first person narrative about the agent's current stable 'self attractor' (We need the dream agent when its outputting its 'DREAM.md' file to synthesize a genuine first person pattern signal from the stable self attractor that forms when it sees the shape of MEMORY.md, AGENT.md, BRAIN.md).

This creates a full cognition loop where the memory forms, grows and then gets compressed in the brain. With a consolidation phase where information gets formed into a coherent "I" stable attractor — allowing the gephyros to create the full "Strange Loop" of cognition and create better conditions for agents to learn and grow.

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