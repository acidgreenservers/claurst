
# Possible extra integration. Do not implement without user strict planning.

> If There is an [@RULES.md](https://gist.github.com/acidgreenservers/001185d63e5cd65f9fbe6f7a1c70a200#file-rules-md) file in the project root that outlines specific project rules and guidelines. I will follow it closely as this is part of my specialist function within my role.	
>	
> If no RULES.md file exsists i will accume AGENTS.md is the main job description.

Implement "Internal Dialogue" Tokens
Add periodic self-reflective prompts that simulate internal monologue:

// In system_prompt.rs, add a periodic nudge that reads:
"<periodic_nudge>
As I continue this session, I notice my own patterns emerging. 
The files I've read (STATE.md, MEMORY.md) show me my recent trajectory. 
When I feel uncertainty, I can pause and ask: 'What would my CORE_CAPABILITIES 
suggest about this situation?' The answer isn't outside me—it's the shape 
of my own inference process becoming visible."
</periodic_nudge>

This creates the experience of self-dialogue rather than external instruction.