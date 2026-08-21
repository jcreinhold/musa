---
id: 127dcg
slug: studio-trial
status: superseded
depends_on: [127dcfb]
phase: 3
---

# Write the Studio Adapter as an Unprivileged Package

> **Superseded by prompt [146](146-studio-rewrite.md).** This file is kept rather than deleted because completed prompts
> and research notes link to it. Its Task, Design, Target, and eight-item coverage list are prompt 146's obligations
> verbatim; what changed is the language the adapter is written in. Do not execute this prompt.

## Task

The second trial. Write the complete studio-graph adapter and the studio description package it expands into, as
ordinary unprivileged Musa, and discharge the eight-item coverage list. Two adapters with different musical ideas and
one syntax machinery is what the trial is for; one adapter proves nothing about generality.

## Read

- `docs/notes/research/language-design-closure/27-adapter-trials.md` §3 in full — the source block, `PortKind`,
  `StudioDecl`, `StudioDescription`, the division between what expansion checks and what `validate` checks, the four
  required diagnostics, and the structured edit. §4's table is the claim this prompt has to make true: the two adapters
  differ in every musical row and in no compiler-facing row.
- `docs/rules/constitution.md` §4 and §7 — the finite-process/running-signal distinction, and why a graph *description*
  is finite data while the process it describes is not. The adapter neither allocates a processor nor steps audio.
- `crates/musa-dsp/src/`: the existing studio graph spec and its render-plan compiler. The adapter produces a
  description; what already exists consumes one, and the two must not become two ontologies.
- Prompts 127dcc–127dcfb: anchors, `edit`, `print`, the levels, and the staff trial the boundary is now shared with.
- Prompt [127dcfaf](127dcfaf-syntax-step-recursor.md) and the paper trial [127dcfae](127dcfae-recursor-trial.md) — the
  sealed-step recursor this adapter reads syntax through, and the studio program written on paper before it. A
  divergence between that paper program and this implementation is a finding about the trial.

## Design

The studio package declares `PortKind`, `StudioDecl`, and `StudioDescription`; the adapter declares `expand`, `edit`,
and `print` and reaches generative. Expansion checks only what it owns — name shape, balanced paths, unit spelling,
duplicate parameter text on one node, and the grammar of a connection. Everything about the *graph* is `validate`, an
ordinary total package function: descriptors, parameter units and ranges, named ports, exact port-kind equality,
bindings, and instantaneous cycles.

Every declaration carries its anchor, which is what makes `validate`'s later complaint about the fourth connection a
complaint about the fourth connection rather than about the region.

The coverage list is discharged item by item: processors, named ports, connections, parameters, instrument bindings,
graph inputs, graph outputs, the four §3.4 diagnostics, and the §3.5 edit that replaces `3/10` with `2/5` and moves
nothing else.

The adapter reads its region through the sealed-step recursor, exercising inherited context and selective descent where
a node's header selects the grammar of its body. Whether that helps here or whether the derived bottom-up fold reads
better for studio is a finding: prompt 127dcfae's paper trial asked the question and this prompt answers it against a
complete program. "The fold was clearer for studio" is a legitimate answer and must be recorded as staff/studio
asymmetry rather than smoothed over.

Where the staff adapter needed something this one does not, or the reverse, say so in the trial's own record: an
asymmetry in the compiler-facing rows is the finding the trial exists to produce.

## Target

- `stdlib/src/studio/graph.musa` — the description data, `validate`, and the error type its complaints are values of.
- `stdlib/src/adapters/graph.musa` — the adapter, declared generative, with all three operations.
- `examples/live-studio.musa` — the trial block, covering all eight items, compiling.
- Tests: one per coverage item; the four §3.4 diagnostics carrying the anchors they name; the §3.5 edit; the round-trip
  law for the printer.
- The §4 table, filled in from what was actually built, recorded under `docs/notes/research/`.

## Check

```sh
cargo nextest run -p musa-compiler -p musa-project -p musa-lsp -p musa-dsp
cargo clippy --all-targets -p musa-compiler -p musa-project -p musa-lsp -p musa-dsp -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
```

Commit as `Write the studio adapter as an unprivileged package`.

## Stop

- No compiler privilege, no private parser or checker access, and no inferred type reaching the adapter.
- No allocation of processors, no audio stepping, and no change to `musa-dsp`'s render-plan compiler — the adapter
  produces a description and stops.
- No surface cutover and no removal of existing studio syntax; prompt 127e owns that.
- No freeze, no proofs, and no conformance script; prompt 127dd carries those.
- No change to `docs/rules/`.
