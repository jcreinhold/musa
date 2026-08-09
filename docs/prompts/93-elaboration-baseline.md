---
id: 93
slug: elaboration-baseline
status: pending
depends_on: [38, 49, 92]
phase: 3
---

# Baseline the Language Before Its Evaluator Changes

## Task

Establish the compatibility and performance oracle for the elaboration-language block before the parser or compiler
changes. Extend the existing semantic benchmark harness with workloads that distinguish plain source, repeated/shared
material, deep transform nesting, and many small declarations; record time and allocations; and freeze the semantic,
provenance, diagnostic, and backend outputs that prompts 94–119 must preserve for source using no new syntax.

## Read

- Prompt 38 and `docs/kernel/09-performance.md`: reuse P1–P5, the three existing fixtures, `divan`, and the rule that
  a benchmark seam stays private.
- Prompt 49's sharing workload and provenance-byte-identity requirement.
- `docs/language/05-verification.md` and `docs/interface/06-performance.md` B1/B2.
- `crates/musa-compiler/src/bench.rs`, `benches/pipeline.rs`, and every caller of `musa_compiler::compile`.

## Design

Add three scenario columns without changing what compilation does:

- **open-shape** — current syntax arranged as many motifs used under different prevailing contexts and placements;
  until prompt 101 adds scale, key/meter and bar checks supply the context pressure.
- **higher-order-shape** — a generated current-syntax fixture with the same note count expressed through nested
  repeat/use/transform structure; it measures traversal and sharing without pretending old syntax has functions.
- **declaration-heavy** — many small motifs, fragments, imports, and references, including unused declarations, to
  expose resolver/table costs that a type environment may amplify.

Measure end-to-end compile, parse, elaboration, term close/evaluate, projection, canonicalization, allocation count, and
peak resident bytes where the harness can report it credibly. Build fixtures outside timed sections. Record machine,
command, sample method, uncertainty, and the source generator. Do not turn a median from one run into a universal
absolute gate; keep the existing relative 10% review rule and B1/B2 end-to-end budgets.

Create a compatibility manifest from committed fixtures: semantic hash, normalized kernel text digest, stable
diagnostic codes/labels, Origin-path projection, and the existing MEI/LilyPond/MusicXML/MIDI/WAV goldens. It is a test
oracle, not a new serialization format and not a public API.

## Target

- `tests/fixtures/{open-shape,higher-order-shape,declaration-heavy}.musa` and their deterministic generator.
- `crates/musa-compiler/benches/pipeline.rs` and private benchmark seams only as needed.
- `docs/language/06-performance.md`: workloads, baseline table, measurement protocol, relative gate, B1/B2 relation,
  and the rule for resource-exhaustion benchmarks added at prompt 96.
- `crates/musa-compiler/tests/elaboration_compatibility.rs`: manifest generation/checking with an explicit update flag.
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

- Change no compiled meaning, parser behavior, diagnostic, public API, or backend output.
- Do not optimize anything the benchmark exposes; report the finding for prompt 118.
- No benchmark-only public fields, traits, or phase objects.
- No future-syntax fixture. A baseline must compile on the compiler it measures.
