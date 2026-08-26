# Formal rules across Musa’s compiler stages

**Status: governing.** Bound by `../constitution.md` and `../obligations.md`; binds every per-stage specification at the
boundaries between stages.

Musa turns source text into several different results: a score event track, engraving, analysis, performance gestures, a
prepared machine, and sound. This directory defines the rules at the boundaries between those results.

It does not replace the detailed specifications for each stage:

- `docs/rules/language/` defines the one source language;
- `docs/rules/events/` defines finite event tracks;
- `docs/rules/desktop/` defines the desktop interface; and
- [03-machine-calculus.md](03-machine-calculus.md) defines machines, their step, their preparation, and the checked
  scheduler that connects a track to a running source.

The rules and their implementation status are separate claims. [The implementation
map](../../plan/code-map/spec-to-implementation-map.md) marks each boundary implemented, partial, or absent, while
[05-metatheory.md](05-metatheory.md) distinguishes reviewed results, executable evidence, and remaining limits.

## Reading order

1. [00-how-to-read.md](00-how-to-read.md) defines the terms and notation used here.
2. [01-stage-judgments.md](01-stage-judgments.md) lists the compiler stages and the values passed between them.
3. [02-derivation-diagrams.md](02-derivation-diagrams.md) defines origin tracking across several conversions.
4. [03-machine-calculus.md](03-machine-calculus.md) defines machines, one step, preparation, and scheduling.
5. [04-identity-and-realization.md](04-identity-and-realization.md) defines equality, encoding, hashes, and caches.
6. [05-metatheory.md](05-metatheory.md) says what has been proved and what remains open.

Failed designs and proof reviews remain in `docs/notes/research/`. They explain why some rules here are stricter than an
ordinary implementation sketch — in particular why the audio side is a calculus of machines rather than a graph with a
scheduler.
