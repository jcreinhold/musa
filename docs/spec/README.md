# Cross-stage formal specification

This directory specifies how Musa's native presentations relate. It is authoritative for cross-stage semantics. It does
not replace the detailed stage specifications:

- `docs/language/` specifies source elaboration (candidate until its graduation prompt);
- `docs/kernel/` specifies exact finite temporal values and terms;
- `docs/interface/` specifies the user-facing projection; and
- `docs/spec/03-process-calculus.md` supplies the process semantics which the existing audio documents lacked.

The specification is an intended target. `docs/architecture/spec-to-implementation-map.md` records where current Rust
implements it, approximates it, or does not yet implement it. When the implementation map says “absent,” this spec does
not become false; the prompt stack owns the missing implementation.

## Reading order

1. [00-how-to-read.md](00-how-to-read.md)
2. [01-stage-judgments.md](01-stage-judgments.md)
3. [02-derivation-diagrams.md](02-derivation-diagrams.md)
4. [03-process-calculus.md](03-process-calculus.md)
5. [04-identity-and-realization.md](04-identity-and-realization.md)
6. [05-metatheory.md](05-metatheory.md)

Research alternatives and failed proofs remain in `docs/scratch/`. In particular, K₁, K₂, and K₃.1 are not hidden
prehistory: their reviews explain why several premises here are as precise as they are.
