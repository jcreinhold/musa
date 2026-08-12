---
id: 146
slug: language-conformance
status: pending
depends_on: [120, 121, 122, 124, 125, 127, 145]
phase: 4
---

# Whole-Language Conformance and Graduation

> **Governed by `docs/core-boundary.md`.** Prompt 126 decided that the core is a calculus of occurrences of any
> canonical payload, that signals stay outside it, and what that forbids. Read it before this prompt's Design.

## Task

Audit prompts 92–145 as one language release, close every compatibility, law, theory, provenance, tooling,
documentation, performance, sound, asset, package, and real-time obligation, and graduate `docs/language/` from
candidate to governing. This prompt adds no feature. It demonstrates that one well-typed source semantics reaches the
kernel, renderers, performance gestures, instruments, audio, project, editors, and desktop without a competing evaluator
or undocumented exception.

## Read

- Prompt 92's acceptance matrix and all completion/repair notes from prompts 93–145.
- All of `docs/language/`, `docs/kernel/`, `docs/interface/`, `docs/governance/`, `docs/spec/`, the roadmap, AGENTS.md,
  and the prompt README. The reasoning behind the language and the corrections applied to it is
  `docs/scratch/60-language-decision-record.md`; it is history, and is read for context rather than audited against.
- Prompt 93's compatibility baseline, prompt 127's score-elaboration report, prompt 144's audio performance report, and
  prompt 145's audio conformance matrix.
- The OMT/source citation map and local proof obligations delivered by prompts 125 and 139.

## Design

Build a machine-checked conformance matrix from each normative rule/law in `docs/language/` to its implementation owner
and at least one positive, negative, algebraic, differential, end-to-end, or compile-fail test. Generate the matrix
where possible; review every manual bridge. At minimum it must cover:

- total evaluation, finite data, budget determinism, closure/partial-application semantics, and module abstraction;
- contextual `Music`, structural folds, canonical composition laws, template identity, provenance multiplicity, and
  cached/uncached equality;
- written pitch/interval action, scales/degrees/context, chord class/voicing, pc12/set/row operations, transformations,
  tonal construction, schemas, assertions, tonal analysis, and voice-leading/counterpoint profiles;
- `.musa.kernel` document inclusion and local typed quote/antiquote, including hygiene, unknown payloads, source maps,
  and the context-neutral boundary;
- exact performance gestures/control curves, tempo/tuning realization, typed instrument contracts, private sound
  implementations, part routing, mix, assets/packages, sampler adapters, media cues/clips, and every prompt 145 row;
- parser recovery, formatting idempotence, tree-sitter drift, LSP facts, editor extension assets, desktop navigation,
  all exports, playback scheduling, last-valid-artifact behavior, and prompt 127/137 budgets.

Three rows exist because a boundary is cheap to hold and expensive to recover once crossed. Each is a check that
something is still *absent*:

- **Patterns are still depth one.** `docs/language/02-core-calculus.md` §6.2 fixes the invariant that every sub-position
  of a pattern is a binder and never another pattern, with no repeated variables, guards, or patterns on the left of a
  definition. This is mechanically checkable and should be checked that way: `Pattern` in
  `crates/musa-compiler/src/core.rs` must remain non-recursive, and the surface grammar must not admit a pattern inside
  a pattern. Nesting would require a pattern-match compiler and a failure mechanism between equations, a subsystem whose
  only purpose is compiling a convenience into eliminators the language already writes directly. If a prompt between 92
  and 144 added nesting, it took on that subsystem; the row fails unless that prompt says so and cites it.
- **The two stages are still two.** `docs/language/02-core-calculus.md` §6.1 states that this calculus and the temporal
  kernel are staged, not layered: no simplifying transformation connects them, and `Term[ScoreFact]` is a stage
  boundary. Check the direction mechanically — no kernel term mentions a closure or a core `Value`, and no core term
  observes a `Timeline` — and check that both totality proofs are still independent.
- **Every normative refusal has provenance.** `docs/language/02-core-calculus.md` §7 and
  `docs/kernel/10-term-calculus.md` §"Provenance of the sharing discipline" cite the literature the design's refusals
  are priced against. Audit that every "deliberately absent" item across `docs/language/` and `docs/kernel/` either
  carries a citation or a musical falsifier. The project cites Open Music Theory by filename for every claim about
  music; a claim about programming languages is held to the same standard or it is an opinion.

Run the representative corpus through source parse/format/reparse, elaboration, kernel normalization/round-trip, every
applicable render/export backend, audio preparation/offline/live-plan paths, project facts, offline package resolution,
and editor protocol fixtures. Compare results at the strongest lawful level: byte identity where promised, otherwise
documented semantic normal-form or observation equality. Review all diagnostic wording with both category-correct
technical terminology and a musician-comprehensible first sentence.

Search for and remove or document stale alternate paths: legacy direct lowering, duplicate theory algorithms, public
HIR/evaluator types, global note streams, raw public DSP parameter ids, exposed private graph addresses, unchecked asset
paths, handwritten editor vocabularies, mutable expanded ASTs, and source-independent widget state. Do not delete
accepted compatibility behavior merely because the new path exists.

Graduation is conditional. If any row lacks implementation or evidence, leave `docs/language/` candidate, repair the
smallest responsible prompt or add a narrowly scoped follow-up, and stop. Only a fully green score and audio matrix may
update governing-document precedence in the roadmap, course correction, prompt README, and AGENTS.md.

## Target

- `scripts/check-language-conformance.sh`, its generated whole-language matrix, and an audit report including named
  exceptions (ideally none), compatibility deltas, theory citation/proof coverage, audio conformance, public-surface
  audit, and final performance comparison.
- End-to-end fixtures spanning musician-facing tonal/modal music, templates/modules, post-tonal/serial material,
  analysis/counterpoint evidence, imports/packages, kernel documents/quotes, instruments/controls, sample adapters,
  routing/media, and deterministic offline playback.
- Discharge of the corrections applied through prompt 114: every musical base type and compiler-owned operation
  classified and covered by `docs/language/02-core-calculus.md` §5.8's theorem with its premises checked, no bundled
  source file reachable from no import, no surviving hand-maintained parallel module list, no accepted `use` in import
  position anywhere in the corpus, and each of the three hard-error migration rules in `docs/language/01-surface.md` §1
  emitting its applicable fix. The list of what those corrections deliberately did *not* change
  (`docs/scratch/60-language-decision-record.md`, final section) is part of the audit, not a footnote to it.
- Repairs required solely to satisfy already-specified behavior; repair prompt/design text in the same commit and record
  why implementation evidence required it.
- On a completely green audit only: graduate `docs/language/` and update its status line and the precedence ladder in
  `docs/README.md`. The documentation reconciliation this step once also owned — dissolving the design essay and the
  correction memo into `docs/language/` — was done ahead of this prompt in the documentation reorganization; only the
  graduation itself remains.

## Check

```sh
./scripts/check-language-conformance.sh
./scripts/check-audio-language-conformance.sh
cargo nextest run --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo deny check
cargo insta test --workspace --unreferenced=reject
cd editors/tree-sitter-musa && tree-sitter test
cd apps/musa-desktop/ui && npx pnpm run check && npx pnpm run test:unit
cd apps/musa-desktop/ui && npx playwright test
git -C ../vscode-musa diff --check
git -C ../zed-musa diff --check
```

Repeat prompts 127 and 140 release comparisons on their recorded benchmark hosts and attach the results. Record manual
smoke tests for VS Code, Zed, and the desktop workbench. Commit each affected repository intentionally, record
cross-repository commit ids, and commit Musa as `Graduate the Musa language`.

## Stop

- No new surface construct, theory/audio feature, backend, syntax alias, or opportunistic refactor.
- No weakened golden, deleted failing test, hidden compatibility delta, or undocumented conformance exception.
- No graduation with a red or unowned matrix row or a red prompt 145 audit.
- No claim that passing tests proves a theoretical convention or audio-format interpretation universal beyond its
  documented domain.
