---
id: 174
slug: core-calculus-conformance
status: pending
depends_on: [173]
phase: 3
---

# Prove and Audit the Core Cutover

## Task

Audit the clean break as one language and runtime. Prove the paper claims against the implemented rules, run complete
notation-led and audio-led programs, and remove every surviving old semantic path before sound-language work resumes.
This prompt adds no feature.

## Read

- All prompt-127a–127e and 171–173 completion and repair notes, including prompts 127da–127dd.
- Research `06-proof-outline.md`, every audit in `docs/notes/research/core-calculus/`, and `17-final-review.md`.
- Revised governing specifications, code map, source/compiler/events/audio facades, stdlib, examples, and book.
- Earlier K1/K2/K3 and source-language counterexamples named by the final review.
- Prompt 169's `72-core-obligation-matrix.md`, `73-core-privacy-and-second-path-audit.md`, and the adapter freeze in
  note 67. They are imported evidence, not rows to restate.
- Prompt 149's trusted boundary and `crates/musa-calculus/TRUST.md`: elaboration is untrusted, accepted terms are
  independently rechecked, and neither the compiler nor runtime may expose or duplicate kernel values, evaluation, or
  conversion.

## Design

Prompt 169 owns the language: the dependent calculus, bidirectional elaboration, source recursion and case trees, source
δ registries, `Storable`, track-expression construction, and adapter expansion. Import its K1–K20 matrix and note 67
without re-proving them. This prompt owns the cutover after that boundary: event-track runtime semantics, machine values
and steps, scheduling, one-frame audio, complete cross-stage traces, and deletion of legacy runtime paths. Its matrix
begins where prompt 169 K14 hands it a valid event term. Cover:

The audit names the language it is preserving: dependent bidirectional elaboration with scoped Miller-pattern
metavariables, indexed families, structural recursion and case trees, private constructors, modules, typed quotation,
generated `Storable`, and registered source δ-rules. Rank-1 inference, contextual `Music`, implicit dictionaries, and a
second compiler evaluator are legacy paths, not implementation options.

- event-track bounds, algebra, multiplicity, coordinate separation, half-open spans, normalization, and exact equality;
- finite machine formation, registry uniqueness, one total next step, causality, initialized feedback, chain and
  side-by-side laws, explicit seeds, and structural versus behavioral equality;
- checked scheduling, decision records, handle hygiene, exact-once boundaries, successful occurrence-local overlay,
  additive succession, fixed finished state, and input-before-output frame convention;
- one-frame audio, whole-machine batching premises, callback partition equality, offline/live agreement, and RT
  instrumentation; and
- derivation-graph coverage from the already-proved adapter output through event reuse, combined inputs, and associative
  runtime-stage composition.

Run five complete programs: tonal construction with harmony and voicing distinct; flexible/unmeasured time; a phrase-led
transcription with stated loss; an ensemble-tuning/acoustic target; and a finite live protocol whose machine may run
without a fixed end. Include one audio-first microphone/synth/effect path. Do not label a culture-specific package
adequate without practitioner review.

Freeze the implemented rules, proofs, matrix, and programs for an independent hostile review. Repair and re-review as
many times as needed. This prompt completes only with a correct-under-contracts verdict and no fatal, high, or medium
finding.

Search for and delete old `Music`, `Timeline`, `StudioGraphSpec`, `compile_graph`, partial/default calls, second frame
schedules, block-defined feedback, raw public parameter addresses, `join`/`jump`/decision-tree dead ends, and duplicated
evaluators. A red row repairs its owning prompt and stops this audit.

## Target

- Machine-checked conformance matrix, proof document, counterexample regression suite, complete programs and traces, and
  public-surface/dependency audit.
- Updated code map and handbook showing what now exists and what remains pending.
- No unresolved fatal, high, or medium proof-review finding.

## Check

```sh
./scripts/check-core-calculus-conformance.sh
cargo nextest run --workspace
cargo nextest run --run-ignored all
cargo clippy --workspace --all-targets -- -D warnings
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
cargo deny check
cargo insta test --workspace --unreferenced=reject
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
cd editors/tree-sitter-musa && tree-sitter test
```

Commit as `Audit the event-track and machine core`.

## Stop

- No new core form, sound feature, package feature, compatibility alias, benchmark-only fast path, or weakened law.
- No green verdict while an old semantic path or unowned matrix row remains.
