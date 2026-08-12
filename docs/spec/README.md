# Formal rules across Musa’s compiler stages

Musa turns source text into several different results: an internal score, engraving, analysis, performance gestures, an
audio plan, and sound. This directory defines the rules at the boundaries between those results.

It does not replace the detailed specifications for each stage:

- `docs/language/` defines the source language;
- `docs/kernel/` defines finite timelines;
- `docs/interface/` defines the desktop interface; and
- [03-process-calculus.md](03-process-calculus.md) defines the audio graph that the current audio documents were
  missing.

Some rules describe work that is not implemented yet.
[The implementation map](../architecture/spec-to-implementation-map.md) marks each part as implemented, partial, or
absent.

## Reading order

1. [00-how-to-read.md](00-how-to-read.md) defines the terms and notation used here.
2. [01-stage-judgments.md](01-stage-judgments.md) lists the compiler stages and the values passed between them.
3. [02-derivation-diagrams.md](02-derivation-diagrams.md) defines origin tracking across several conversions.
4. [03-process-calculus.md](03-process-calculus.md) defines valid audio graphs and one audio step.
5. [04-identity-and-realization.md](04-identity-and-realization.md) defines equality, encoding, hashes, and caches.
6. [05-metatheory.md](05-metatheory.md) says what has been proved and what remains open.

Failed designs and proof reviews remain in `docs/scratch/`. They explain why some rules here are stricter than an
ordinary implementation sketch.
