---
id: 93
slug: elaboration-baseline
status: done
depends_on: [38, 49, 92]
phase: 3
---

# Baseline the Language Before Its Evaluator Changes

## Task

Establish the compatibility and performance oracle for the elaboration-language block before the parser or compiler
changes. Extend the existing semantic benchmark harness with workloads that distinguish plain source, repeated/shared
material, deep transform nesting, many small declarations, and the existing performance/studio/audio pipeline; record
time and allocations; and freeze the semantic, provenance, diagnostic, graph, scheduling, and backend outputs that
prompts 94–146 must preserve for source using no new syntax, except where the baseline explicitly records a defect for
one named repairing prompt.

## Read

- Prompt 38 and `docs/kernel/09-performance.md`: reuse P1–P5, the three existing fixtures, `divan`, and the rule that a
  benchmark seam stays private.
- Prompt 49's sharing workload and provenance-byte-identity requirement.
- `docs/language/05-verification.md` and `docs/interface/06-performance.md` B1/B2.
- `crates/musa-compiler/src/bench.rs`, `benches/pipeline.rs`, and every caller of `musa_compiler::compile`.
- `crates/musa-compiler/src/{profile,performance,studio}.rs`, `crates/musa-audio/src/{studio,plan,offline}.rs`, and
  `crates/musa-engine`'s prepared-plan handoff. Read the actual code paths: do not copy claims about exact studio
  values, event routing, or parameter consumption from prose without verifying them.

## Design

Add three scenario columns without changing what compilation does:

- **open-shape** — current syntax arranged as many motifs used under different prevailing contexts and placements; until
  prompt 101 adds scale, key/meter and bar checks supply the context pressure.
- **higher-order-shape** — a generated current-syntax fixture with the same note count expressed through nested
  repeat/use/transform structure; it measures traversal and sharing without pretending old syntax has functions.
- **declaration-heavy** — many small motifs, fragments, imports, and references, including unused declarations, to
  expose resolver/table costs that a type environment may amplify.
- **audio-bridge** — two parts, two profiles, two assigned patches, one shared room bus, a modulated processor, a
  hairpin, articulations, and a fixed sample rate. Record each `PerformanceLane` with its `PartId`, the lowered graph,
  the studio-lowering notes, scheduled event digest, rendered WAV digest, block sizes, allocations, and render time.

Measure end-to-end compile, parse, elaboration, term close/evaluate, projection, canonicalization, allocation count, and
peak resident bytes where the harness can report it credibly. Build fixtures outside timed sections. Record machine,
command, sample method, uncertainty, and the source generator. Do not turn a median from one run into a universal
absolute gate; keep the existing relative 10% review rule and B1/B2 end-to-end budgets.

Create a compatibility manifest from committed fixtures: semantic hash, normalized kernel text digest, stable diagnostic
codes/labels, Origin-path projection, performance lanes, studio intent, prepared graph summary, and the existing
MEI/LilyPond/MusicXML/MIDI/WAV goldens. It is a test oracle, not a new serialization format and not a public API.

Keep a machine-readable expected-change ledger beside the manifest. It records, rather than blesses, the current shared
note-stream warning, ignored `Parameter` event, processor-hover gap, graph-topology modulation address, and eager studio
`f64` conversion. Each entry names exactly one repairing prompt, **by slug**: a rank is an execution position and moves
whenever a prompt is inserted, so an identity must not be spelled as one. Any other change is a compatibility failure.
When its prompt lands, replace the defect observation with the positive law and remove the ledger entry. Never preserve
known-wrong audio merely because it was baselined.

## Target

- `tests/fixtures/{open-shape,higher-order-shape,declaration-heavy,audio-bridge}.musa` and their deterministic
  generator.
- `crates/musa-compiler/benches/pipeline.rs` and private benchmark seams only as needed.
- `docs/language/06-performance.md`: workloads, baseline table, measurement protocol, relative gate, B1/B2 relation, and
  the rule for resource-exhaustion benchmarks added at prompt 96.
- `crates/musa-compiler/tests/elaboration_compatibility.rs`: manifest generation/checking with an explicit update flag.
- An audio compatibility/benchmark test at the narrowest existing owner; do not publish graph or DSP internals merely so
  the manifest can inspect them.
- Committed compatibility manifest under `tests/fixtures/` or the existing snapshot location.

## Check

```sh
cargo bench -p musa-compiler
cargo nextest run -p musa-compiler
cargo clippy --all-targets -p musa-compiler -- -D warnings
cargo fmt --check
test -s docs/language/06-performance.md
git diff --check
```

Commit as `Baseline the elaboration language migration`.

## Stop

- Change no compiled meaning, parser behavior, diagnostic, public API, or backend output; document observed audio
  defects instead of repairing them here.
- Do not optimize anything the benchmark exposes; report the finding for prompt 127.
- No benchmark-only public fields, traits, or phase objects.
- No future-syntax fixture. A baseline must compile on the compiler it measures.
