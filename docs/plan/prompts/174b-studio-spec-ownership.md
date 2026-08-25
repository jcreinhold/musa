---
id: 174b
slug: studio-spec-ownership
status: pending
depends_on: [167, 174, 174a]
phase: 3
---

# Checked Source Owns Studio Intent

> **Reopened by note 79.** The first execution moved the legacy Rust `StudioSpec` from `musa-compiler` to `musa-dsp`.
> That repaired a Cargo edge but missed the prior question: prompt 167 had already proved that studio intent is
> declarable ordinary Musa data. This execution removes the Rust vocabulary as semantic authority.

## Task

Cut the production studio path over from the independently constructed Rust `StudioSpec` to the checked
`std::sound::graph::StudioDescription` value produced by the ordinary adapter and evaluator. Keep a narrow exact
projection for DSP preparation and caller facts, but make it impossible for Rust callers to construct a competing studio
language.

## Read

- `docs/rules/language/00-semantics.md`'s ownership test and `08-performance-and-sound.md` §0; note
  [`79`](../../notes/research/language-design-closure/79-source-owns-the-sound-language.md).
- Prompt 167 and note 66, especially the measured source `StudioDescription` and the explicitly missing bridge to the
  legacy Rust path; `stdlib/src/{sound/graph,adapters/graph}.musa` in full.
- The public `musa-dsp::StudioSpec`/`WrittenQuantity` callers in compiler, project, LSP, UI facts, preparation, and
  tests; distinguish editable semantics from a consumer projection and from private render state.
- Prompt 164 and note 61 for the builtin-ownership precedent.
- Peyton Jones chapter 3 and Ousterhout chapters 7–8: source-to-substrate translation and the cost of adjacent layers
  exposing the same abstraction.

## Design

`StudioDescription`, its exact quantity/unit data, validation errors, and total validation functions are ordinary source
declarations. The graph adapter returns one such expression with complete anchors; the one checker and evaluator accept
it. Compiler/project facts derive from that checked value and the lossless CST. Structured edits still rewrite source
tokens; no mutable or independently editable Rust AST replaces them.

The DSP boundary may decode the checked value into a private or field-private exact preparation projection. That
projection:

- is constructible only from a successfully checked source artifact;
- carries a source schema/version and complete exact canonical bytes;
- has no independent defaults, aliases, validation policy, or public field constructors;
- is compared field-for-field and byte-for-byte with the source value by differential laws; and
- becomes floating point or compact runtime indices only during later preparation.

Registered primitive identity, private port/state formats, work/memory bounds, and runtime descriptor agreement remain
host-owned. A source processor wrapper may name one registered primitive; this prompt does not expose its state or make
the registry source data.

Remove the public Rust `StudioSpec`, `Processor`, `Unit`, `WrittenQuantity`, routing enums, and node-tree construction
surface where those items duplicate source declarations. A small opaque artifact or read-only project fact is allowed
only with a real caller and the derivation law above. Do not expose calculus `Value` to achieve the cutover.

## Target

- Production compilation and preparation consume the checked `std::sound` result, not the legacy grammar-to-Rust
  `StudioSpec` path.
- An opaque exact DSP preparation projection with schema/version and complete source-equivalence laws.
- Compiler/project/LSP/desktop facts and edits derived from checked source plus CST/resolution spans.
- Legacy Rust construction APIs deleted or made private differential oracles, then deleted when parity is established.
- Code map, roadmap, crate docs, and dependency descriptions naming source as the semantic owner.

## Check

```sh
cargo nextest run -p musa-calculus -p musa-compiler -p musa-dsp -p musa-project -p musa-lsp
cargo nextest run --run-ignored all
cargo clippy --workspace --all-targets -- -D warnings
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
cargo insta test --workspace --unreferenced=reject
rg -n "pub (struct|enum) (StudioSpec|WrittenQuantity|Processor|Unit)" crates
```

The final `rg` is empty unless a surviving item is an explicitly documented opaque projection rather than source
vocabulary; any exception is named in the completion note and audited again at prompt 192.

Commit as `Make checked source authoritative for studio intent`.

## Stop

- No new processor, instrument, control, routing feature, or syntax spelling.
- No source evaluator, type checker, or resolver in `musa-dsp`; it receives an already checked exact artifact.
- No public calculus value, public primitive state, or editable projection beside source.
- No float conversion, scheduling, or machine instantiation; prompts 176 and 178 own those boundaries.
