# Persistent agent handoffs

Each zero-context agent creates exactly one handoff here using the filename
specified by its task. Handoffs are durable execution records, not summaries of
conversation.

Every handoff must include:

1. final status (`DONE` or `BLOCKED`);
2. implementation/design summary;
3. files added or changed;
4. exact validation commands and results;
5. artifact paths and SHA-256 hashes where applicable;
6. measured ABI/code/static RAM/SDRAM facts relevant to later tasks;
7. unresolved risks, waivers, or user decisions;
8. the next task's prerequisite smoke command.

A later agent must verify the filesystem and rerun smoke checks rather than
trusting this prose alone.
