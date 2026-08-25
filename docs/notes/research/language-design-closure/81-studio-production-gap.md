# 81. Studio production gap inventory

**Status: governs nothing.** This is prompt 174b's exhaustive migration oracle for the source-owned studio cutover in
prompt 180a.

## What the checked bridge proves now

The checked artifact and DSP projection cover prompt 167's complete `std::sound::graph::StudioDescription`: schema
version, ordered input/node/output/connect/bind declarations, source anchors, named port paths, audio channel counts,
control and note-event kinds, ordered parameters, natural counts, exact plain ratios, and exact seconds. The standard
library owns descriptor/range/port/binding/cycle validation for its three trial descriptors: `poly_sine`, `scale`, and
`reverb`.

That is a bridge proof, not production parity. The pre-cutover `studio { ... }` path accepts every item below and must
remain only as a differential oracle until the corresponding ordinary source declarations exist.

## Declaration and topology gaps

- Named **patches** and **buses**, each containing an ordered processor-node graph and a designated output.
- Named top-level **signals**, whose node graphs act as modulation sources.
- Node labels addressable by later modulation paths, ordered node inputs, and the distinction between a node's complete
  call span and each written parameter-value span.
- Part-to-patch **assignments**, part/bus-to-bus-or-master **routes**, exact-decibel **sends**, and signal-to-node-
  parameter **modulations**.
- Imported libraries may contribute patches and signals but not piece-specific buses, assignments, routes, sends, or
  modulations. Forward references, duplicate names, routability, destination existence, and target resolution are part
  of the accepted behavior to preserve.
- The `StudioSpec` block span, assignment patch-name span, send-level span, node span, and parallel parameter spans used
  by structured edits and remapped through expansion provenance.
- The implicit default instrument for an unassigned part and the behavior of a partial studio, which must not silence
  unmentioned parts.

## Processor and parameter gaps

The legacy oracle has thirteen processor names; the trial has only `scale` and `reverb` plus its separate `poly_sine`
descriptor. Production parity therefore covers every row and its ordered signature:

1. `oscillator`
2. `gain`
3. `mix`
4. `envelope`
5. `lowpass`
6. `highpass`
7. `reverb`
8. `delay`
9. `chorus`
10. `scale`
11. `bias`
12. `clamp`
13. `smoothing`

For each row the source declaration must carry the public name, private registered-primitive agreement key, ordered
parameter names, DSP parameter mapping (including source `resonance` to primitive `q`), former spellings used only for
diagnostics, documentation, exact default, exact inclusive writable range, and unit. Positional parameters depend on
declaration order. An omitted parameter receives its source-declared default before projection; the host decoder does
not choose it.

## Exact quantity and unit gaps

The trial distinguishes `Count`, dimensionless `Plain`, and `Seconds`. Production accepts exact rational magnitudes in
four dimensions: hertz, linear, decibels, and seconds; `ms` is an input scale into exact seconds. It preserves written
unit/edit identity even though DSP preparation later converts decibels to linear gain and exact quantities to floats.
Bare-number policy, unit agreement, ranges, defaults, aliases, scale conversion, and normalization belong to source
declarations and total functions, not to a Rust `Unit`/`WrittenQuantity` table.

## Cutover proof required by prompt 180a

The cutover must exercise every row above through both the temporary oracle and checked source, comparing complete exact
projections, diagnostics, declaration order, identities, project facts, structured-edit ranges, and rendered fixtures.
Only then may it delete `StudioSpec`, `Patch`, `StudioNode`, `Assignment`, `Route`, `Send`, `Modulation`, `Processor`,
`ParamSpec`, `Unit`, `WrittenQuantity`, the Rust catalogue, the compiler studio resolver, and the normal
`musa-compiler -> musa-dsp` dependency.

Indexed quantities, controls, ports, and mappings share their indices in source. Omitted indices are solved by the
existing Miller-pattern unifier with distinct-variable spines, occurs/scope checks, postponement, and refusal of
unresolved constraints. The checked artifact is zonked data; neither this bridge nor the eventual DSP decoder may add
sound-specific inference, coercion, defaults, retries, or fallbacks.
