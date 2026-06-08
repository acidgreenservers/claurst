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