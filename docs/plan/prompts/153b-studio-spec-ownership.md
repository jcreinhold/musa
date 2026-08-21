---
id: 153b
slug: studio-spec-ownership
status: pending
depends_on: [153, 153a]
phase: 3
---

# Give the Studio Graph Back to the Studio Crate

## Task

`musa-dsp`'s crate doc says it owns "the declarative studio graph: `StudioGraphSpec` (editable, serializable intent) →
compiled `RenderPlan`". It does not own the first half. Every type in that spec is defined in
[`crates/musa-compiler/src/studio.rs`](../../../crates/musa-compiler/src/studio.rs) — 1,176 lines of `StudioSpec`,
`StudioNode`, `Patch`, `Processor`, `Route`, `Send`, `Assignment`, `Modulation`, `ParamSpec`, `NodeIndex`, `Unit`,
`Value` — and `musa-dsp` imports them back:

```sh
$ grep -n 'musa_compiler' crates/musa-dsp/src/*.rs
crates/musa-dsp/src/studio.rs:28:use musa_compiler::{NodeIndex, Patch, Processor, StudioNode, StudioSpec};
crates/musa-dsp/src/spec.rs:42:pub use musa_compiler::Unit;
```

So the audio crate depends on the compiler, and this is the one edge in the workspace that runs against the rule every
other output obeys. `ScoreSnapshot` is the compiler's other product and it lives in `musa-score`, *below* the compiler,
which depends on it. The studio spec is the same kind of thing — an editable, serializable value a compilation produces
— and it is the exception. Reverse it.

## Read

- [`crates/musa-dsp/src/lib.rs`](../../../crates/musa-dsp/src/lib.rs)'s crate doc, which already claims this ownership,
  and [`crates/musa-dsp/src/spec.rs`](../../../crates/musa-dsp/src/spec.rs), which holds `ProcessorSpec` beside the
  `Processor` it re-exports from the compiler — the two halves of one vocabulary in two crates.
- [`crates/musa-compiler/src/studio.rs`](../../../crates/musa-compiler/src/studio.rs) in full: which of it is the
  vocabulary and which is the pass that builds one from resolved source.
- [`crates/musa-score/src/score.rs`](../../../crates/musa-score/src/score.rs)'s `ScoreSnapshot` — the pattern being
  matched. The compiler depends on the vocabulary of what it produces; the vocabulary knows nothing about the compiler.
- Prompt [146](146-studio-rewrite.md), which rewrote the studio adapter as an unprivileged package, and prompts
  [150](150-machine-runtime.md)–[153](153-core-calculus-conformance.md), the clean-break cutover. This prompt is
  sequenced after all of them deliberately: it moves where a type lives and must not compete with work that decides what
  the type *says*.
- Roadmap §13.3 and §13.8 for the spec→plan split, and §15's crate list.
- *Simple Made Easy*: the spec and the pass that produces it are two things, and one crate holding both is why the arrow
  had to point backwards.

## Design

**The vocabulary moves; the pass stays.** `musa-dsp` gains the spec types. `musa-compiler` keeps everything that reads
resolved source and *builds* a `StudioSpec`, and gains a `musa-dsp` dependency to name what it produces — exactly as it
already depends on `musa-score` to name a `ScoreSnapshot`.

**No re-export bridge.** `musa-compiler` does not re-export the moved types, because that is the laundering 153a deleted
one prompt earlier. Consumers name `musa_dsp::StudioSpec`. `musa-project` already depends on both crates, and the shells
reach them through it.

**Nothing about the spec changes.** Not a type name, not a field, not a serialization. The prompt is a move, and the
check is that a studio project written today loads, renders, and exports byte-identically afterwards.

**The resulting graph, which is the point.** `musa-dsp` sits beside `musa-score` at the vocabulary layer, depending on
`musa-score` and nothing above it. `musa-compiler` sits above both. `musa-playback` depends on `musa-dsp`. No cycle
exists and no crate below the compiler names it — the rule 153c writes down and enforces.

## Target

- `crates/musa-dsp/`: the spec types, beside the `ProcessorSpec` and `RenderPlan` that already live there, with the
  crate doc no longer claiming an ownership it did not have.
- `crates/musa-dsp/Cargo.toml`: no `musa-compiler`.
- `crates/musa-compiler/src/studio.rs`: the pass alone, producing `musa_dsp::StudioSpec`.
- `crates/musa-compiler/Cargo.toml`: `musa-dsp` added.
- `crates/musa-compiler/src/lib.rs`: the studio re-export block deleted from the facade.
- `crates/musa-project`, `crates/musa`, `apps/musa-desktop/src-tauri`: call sites naming `musa_dsp::…`.
- `AGENTS.md` and roadmap §15: the corrected layer for `musa-dsp`.

## Check

```sh
cargo nextest run --workspace
cargo nextest run --run-ignored all
cargo clippy --workspace --all-targets -- -D warnings
make fmt-check && make docs-check
```

And the move-not-rewrite checks:

- `grep musa-compiler crates/musa-dsp/Cargo.toml` is empty, and `musa-compiler` defines no studio type.
- **Rendering is byte-identical.** Every studio fixture in `examples/` produces the same `RenderPlan` and the same
  exported audio as before; the `wav_export_is_deterministic_for_all_examples` slow test and every `insta` snapshot are
  unchanged.

## Stop

- **No change to the spec's shape.** No renamed type, no renamed field, no changed serialization, no new processor, no
  removed one.
- **No change to compilation.** The pass that builds a spec from source behaves identically.
- **No compiler split.** `musa-compiler` loses `studio.rs`'s types and nothing else; what else it should shed is not
  this prompt's question, and see the note in 153c.
- **No re-export bridge in `musa-compiler`.**
- Nothing about `musa-playback`'s own layering, the audio callback, or `RenderPlan`'s internals.
- No revisiting of 146's adapter decisions, or 150–153's cutover.

Commit as `Give the studio graph back to the studio crate`.
