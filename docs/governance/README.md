# Musa’s core design decisions

These pages state the few decisions that every part of Musa must follow. They do not prescribe Rust types or source
syntax. They answer seven questions:

1. What can a user edit?
2. Must all music use the same theory?
3. How does Musa represent finite musical time?
4. How does that representation connect to audio?
5. How can notation, analysis, MIDI, and audio describe one project without being treated as the same thing?
6. What does it mean for two stored results to be equal?
7. Does each kind of musical event get its own structure, or do they share one?

Read [01-constitution.md](01-constitution.md) for the answers. Then read [02-obligations.md](02-obligations.md) for
rules that follow from them.

These decisions govern `docs/spec/`, `docs/kernel/`, `docs/language/`, the architecture documents, and the work plan in
`docs/prompts/`. Research notes in `docs/scratch/` do not govern the project. They record how the decisions were
reached, including ideas that failed.

## Changing a decision

A core decision may change, but only in a change that does all of the following:

1. gives a concrete musical or engineering reason;
2. shows which current examples no longer work;
3. states the replacement rule in plain language;
4. updates the formal specification and architecture;
5. explains how stored files and public APIs will migrate; and
6. records the change in `docs/scratch/` so the old argument remains visible.
