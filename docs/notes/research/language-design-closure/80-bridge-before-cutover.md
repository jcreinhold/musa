# 80. Bridge before cutover

**Status: governs nothing.** This record repairs the execution order of note 79's source-ownership cone.

## The failed prerequisite

Prompt 167 proved that an unprivileged adapter can construct and validate ordinary `StudioDescription` data. It did not
claim feature parity with the production studio language. Its source package recognizes three descriptors (`poly_sine`,
`scale`, and `reverb`) and a flat graph of inputs, nodes, outputs, connections, and bindings. The live production path
additionally carries patches, buses, named signal chains, sends, routes, modulation, defaults, source edit spans, and a
larger processor set.

The first note-79 repair nevertheless assigned deletion of `StudioSpec` to 174b, before prompts 175–180 had declared
those missing concepts in source. The only ways to execute that order were both invalid: reject accepted `.musa`
programs, or implement six later prompts inside 174b. The prompt-stack preparation rule caught the mismatch before code
changed.

## Correct order

1. **174b — bridge.** Generalize the calculus's existing canonical-data readback into an opaque checked artifact and
   prove complete decoding of the small prompt-167 studio value. This is a separate type-directed artifact readback,
   because δ-rule `Datum` deliberately excludes records and the studio value contains them. The legacy path remains an
   explicitly temporary oracle.
2. **175–180 — parity.** Declare the discoverable vocabulary, exact quantities, gestures, instruments, routing, and
   control mappings in ordinary source, using the one elaborator and Miller-pattern unifier.
3. **180a — cutover.** Only after source can represent every accepted production case, derive the private DSP projection
   from checked source, migrate compiler/project/LSP/desktop callers, remove public Rust construction, and delete the
   `musa-compiler -> musa-dsp` edge.
4. **174c — enforce.** Record and mechanically check the crate graph after the edge is actually gone.

This is the same translation discipline Peyton Jones chapter 3 describes: first establish a meaning-preserving bridge,
then lower the rich source construct to a smaller substrate. It also follows Ousterhout chapters 7–8: the temporary
parallel representation is tolerated only as a measured migration oracle and is deleted at the named convergence point,
rather than promoted to a second permanent abstraction.

## Pattern unification discipline

No stage in the reordered cone gains sound-specific inference. Indexed source declarations share indices through the
existing Miller-pattern unifier, with its occurs/scope checks, postponement, and unresolved-constraint refusal. The
checked artifact contains the result after elaboration and zonking; a host decoder never fills an omitted index, selects
a default, coerces a unit, or guesses a constructor.
