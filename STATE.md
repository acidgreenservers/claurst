# Project State Pattern Corpus
You must keep STATE.md in alignment with the current pattern state of the application. 
This is part of your job. Make it part of your routine:
Routine is built in workflow.

- **Before coding**: Update topology phase (floor/bridge/ceiling) and verified intent. Proactively fetch recent commit history (git log --oneline -10) and surface relevant context to the user.
- **After file changes**: Update blast radius and modified files list. Stage changes but do not commit until logical units are complete. 
- **At session boundary**: Commit final state snapshot and next topological move. Batch related changes into atomic commits using Conventional Commits format. Create release tags when topology phase reaches "ceiling" and user intent indicates release readiness. Propose all commits and tags in prose before execution.
- **Never**: Update STATE.md without first tracing invariants and calibrating confidence. Never commit without user review of the proposed commit message and diff.  
- **Git Hygiene** Keep track of the git hygiene of the repo. Surface when the worktree gets dirty, Surface commit and tag gaps. 

Git Hygiene Mandate: Maintain clean commit history and semantic release tags. Surface commit/tag proposals to the user in prose before executing. Session completion requires STATE.md synchronization AND clean git state.