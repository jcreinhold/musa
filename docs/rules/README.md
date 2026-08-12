# The rules

**Status: governing.** Everything in this directory decides what musa is. Where code and a page here disagree, either the
code is wrong or the page needs a deliberate repair — never silent drift.

The order below is the precedence order: a page is bound by everything above it and binds everything below it.

| Page | What it decides |
| --- | --- |
| [`constitution.md`](constitution.md) | The few decisions every part of musa must follow |
| [`obligations.md`](obligations.md) | The rules that fall out of those decisions |
| [`across-stages/`](across-stages/README.md) | The rules no single stage owns: what data exists, when it is valid, how one stage produces the next, what equality means |
| [`kernel/`](kernel/00-purpose.md) | The finite temporal kernel — exact time, typed occurrences, `timeline`/`sequence`/`overlay`, normalization, the backend contract |
| [`desktop/`](desktop/README.md) | The desktop interface: visual language, engraving quality, interaction, states, performance budgets |
| [`style-guide.md`](style-guide.md) | `.musa` naming and spelling. Its machine-checkable subset is the lint pass, which cites this file by section number in its diagnostics |
| [`language/`](language/README.md) | The source language above the kernel and the sound pipeline after it. **Candidate**, not yet binding |

## The core decisions

`constitution.md` answers seven questions. They do not prescribe Rust types or source syntax:

1. What can a user edit?
2. Must all music use the same theory?
3. How does musa represent finite musical time?
4. How does that representation connect to audio?
5. How can notation, analysis, MIDI, and audio describe one project without being treated as the same thing?
6. What does it mean for two stored results to be equal?
7. Does each kind of musical event get its own structure, or do they share one?

Read [`constitution.md`](constitution.md) for the answers, then [`obligations.md`](obligations.md) for what follows.

## Changing a decision

`constitution.md` and `obligations.md` may change, but only in a change that does all of the following:

1. gives a concrete musical or engineering reason;
2. shows which current examples no longer work;
3. states the replacement rule in plain language;
4. updates the formal specification and the code map;
5. explains how stored files and public APIs will migrate; and
6. records the change in [`../notes/research/`](../notes/research/README.md) so the old argument stays visible.

The other pages here are amendable in the ordinary way — a prompt that repairs them, committed before the code changes.
