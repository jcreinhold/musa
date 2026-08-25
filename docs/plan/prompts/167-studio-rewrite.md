---
id: 167
slug: studio-rewrite
status: pending
depends_on: [166, 166a]
phase: 3
---

> **Reinstated and repaired by notes [`51`](../../notes/research/language-design-closure/51-the-terseness-audit.md) and
> [`52`](../../notes/research/language-design-closure/52-the-musical-algebra.md).** The second adapter, on the language
> of 142a–142e. Its generality claim is now sharper: the two adapters must differ in every musical row and in no
> compiler-facing row, and neither may need a builtin the other does not.
>
> **Repaired after prompt 166a and against the live cutover boundary.** Ordinary source cannot store the expansion
> phase's `Syntax` values, so declaration anchors are the `Nat` values produced by `syntax_anchor`, exactly as in the
> staff package. The current Rust `StudioSpec` is the pre-cutover sound path; there is no source-value bridge by which
> it could consume `StudioDescription`, and prompt 174 explicitly owns deleting that path. This prompt proves that
> ordinary Musa can describe and validate the finite graph. It does not pretend that an unused source value already
> drives the render-plan compiler.

# Write the Studio Adapter as an Unprivileged Package on the New Language

## Task

The second adapter. Write the complete studio-graph adapter and the finite studio description package it expands into,
as ordinary unprivileged Musa on the new language, and discharge the eight coverage groups. One adapter proves nothing
about generality; two adapters with different musical ideas and one syntax machinery is what the trial is for. This
prompt absorbs 127dcg, whose historical superseded file remains because completed prompts and notes link to it.

## Read

- The superseded prompt [127dcg](127dcg-studio-trial.md), whose whole Task this prompt absorbs. Its eight-item coverage
  list, its division between what expansion checks and what `validate` checks, its four required diagnostics, and its
  `3/10` → `2/5` edit are this prompt's obligations verbatim. Absorbing it was right because writing the studio adapter
  on the old language and then rewriting it on the new one within four prompts would be the same file written twice for
  no additional evidence.
- `docs/notes/research/language-design-closure/27-adapter-trials.md` §3 in full — the source block, `PortKind`,
  `StudioDecl`, `StudioDescription`, and §4's table, which is the claim this prompt has to make true: the two adapters
  differ in every musical row and in no compiler-facing row.
- `docs/rules/constitution.md` §4 and §7 — a graph *description* is finite data; the process it describes is not. The
  adapter neither allocates a processor nor steps audio.
- `crates/musa-dsp/src/` and `crates/musa-compiler`'s `StudioSpec` — the existing pre-cutover studio path. Compare its
  vocabulary and finite-graph invariants with the source package, but do not claim it consumes a core value when no such
  bridge exists. Prompt 174 owns the clean break; this prompt records the temporary parallel representation.
- Prompt [166](166-staff-rewrite.md)'s measurement and its per-section breakdown — the method this prompt reuses, and
  the staff numbers this adapter's numbers are compared against.
- Prompt 132's studio program, written on paper before any of this existed. A divergence between that program and this
  implementation is a finding about the trial, not a detail.

## Design

**The division of labour is 127dcg's and does not move.** Expansion checks what it owns — name shape, balanced paths,
unit spelling, duplicate parameter text on one node, and the grammar of a connection. Everything about the *graph* is
`validate`, an ordinary total package function: descriptors, parameter units and ranges, named ports, exact port-kind
equality, bindings, and instantaneous cycles. Every declaration carries its anchor, so `validate`'s complaint about the
fourth connection is a complaint about the fourth connection rather than about the region.

**Use the language that exists.** The region is imported as `std::adapters::graph` and written with braces. An ordinary
package value carries `Nat` anchors, never phase-local `Syntax`; the adapter obtains each one from `syntax_anchor` on a
node it was handed. `PortKind.equal` is one written namespace definition, as prompt 166a decided — there is no `Eq`
instance. The eight coverage groups are processors, named ports, connections, parameters, instrument bindings, graph
inputs, graph outputs, and the four required diagnostic classes collectively. The edit and printer laws are separate
obligations rather than a ninth and tenth reading feature.

**The source description is not wired to audio in this prompt.** `StudioDescription` is the finite value the future
machine cutover can consume. The existing built-in `studio` grammar still produces Rust `StudioSpec`, and keeping that
legacy route temporarily is work already assigned to prompt 174. This trial compares their vocabulary and invariants and
records the gap; adding an implicit bridge or claiming one exists would be a second hidden semantic path.

**This is where the generality claim is actually tested.** The staff adapter drove every design decision in prompts
138–141, so it is the worst possible witness for whether those decisions generalize. The studio adapter was chosen
before them and is musically unlike staff in every way that matters — a graph rather than a sequence, headers that
select the grammar of their bodies, validation that is about global structure rather than local shape. Report what the
new mechanisms did for it, including where they did nothing.

**Selective descent versus the bottom-up fold, answered against a complete program.** Prompt 127dcfae's paper trial
asked whether the sealed-step recursor helps for studio or whether a derived bottom-up fold reads better, and — with
prompt 140's syntax patterns now available — whether either is the right tool at all. "The fold was clearer for studio"
and "patterns replaced both" are both legitimate answers. Record whichever is true as staff/studio asymmetry rather than
averaging it away.

**Measure it the same way staff was measured.** Lines and bytes, and the same five eliminations: no `callN`, no
hand-allocated role integers, no string dispatch on token kinds, records rather than wide destructures, forward
accumulation. A second adapter that needs a workaround the first did not is the single most useful finding this prompt
can produce.

## Target

- `stdlib/src/studio/graph.musa` — the description data, `validate`, and the error type its complaints are values of.
- `stdlib/src/adapters/graph.musa` — the adapter, declared generative, with `expand`, `edit`, and `print`.
- `examples/live-studio.musa` — the trial block, covering all eight groups, compiling as a finite description.
- Tests: one per coverage item; the four §3.4 diagnostics carrying the anchors they name; the §3.5 edit that replaces
  `3/10` with `2/5` and moves nothing else; the printer round-trip law at the level the adapter claims.
- §4's table filled in from what was actually built, and the staff/studio asymmetry, recorded under
  `docs/notes/research/language-design-closure/` beside prompt 166's measurement.

## Check

```sh
cargo build --workspace
cargo nextest run --workspace
cargo nextest run --run-ignored all
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
cargo run -p musa -- format --check stdlib/src/studio/graph.musa stdlib/src/adapters/graph.musa
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
```

Commit as `Write the studio adapter as an unprivileged package`.

## Stop

- No compiler privilege, no private parser or checker access, and no inferred type reaching the adapter. An adapter that
  needs one is evidence against the boundary and is reported as such rather than granted.
- No allocation of processors, no audio stepping, and no change to `musa-dsp`'s render-plan compiler — the adapter
  produces a description and stops.
- No freeze, no proofs, and no conformance script. Prompt 168.
- No change to `docs/rules/`.
- No change to `stdlib/src/adapters/staff.musa` — prompt 166's measurement is fixed once it is taken.
- No new language feature. If the second adapter needs one, that is the generality claim failing, and it is reported,
  not implemented here.
