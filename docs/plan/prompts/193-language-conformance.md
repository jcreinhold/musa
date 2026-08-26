---
id: 193
slug: language-conformance
status: in-progress
depends_on: [120, 121, 122, 124, 125, 174, 192]
phase: 4
---

# Whole-Language Conformance and Graduation

> **Governed by the event-track and machine core installed by prompts 127a–127e and 171–174.** Graduation covers the
> clean-break language only; removed syntax and semantic paths are not compatibility obligations.

## Task

Audit the current language produced by prompts 92–192, including the clean replacement in 127a–127e and 171–174. Close
every law, theory, provenance, tooling, documentation, performance, sound, asset, package, and real-time obligation, and
graduate `docs/rules/language/` from candidate to governing. This prompt adds no feature. It demonstrates that one
well-typed source semantics reaches the event track, renderers, performance gestures, instruments, audio, project,
editors, and desktop without a competing evaluator or undocumented exception.

## Read

- Prompt 92's acceptance matrix, the prompt-127a clean-break ledger, and all completion/repair notes through prompt 192.
- All of `docs/rules/language/`, `docs/rules/events/`, `docs/rules/desktop/`, `docs/rules/`,
  `docs/rules/across-stages/`, the roadmap, AGENTS.md, and the prompt README. The reasoning behind the language and the
  corrections applied to it is `docs/notes/research/60-language-decision-record.md`; it is history, and is read for
  context rather than audited against.
- Prompt 93's historical baseline, prompt 174's core report, prompt 191's audio performance report, and prompt 192's
  audio conformance matrix.
- The OMT/source citation map and local proof obligations delivered by prompts 125 and 186.

## Design

Build a machine-checked conformance matrix from each normative rule/law in `docs/rules/language/` to its implementation
owner and at least one positive, negative, algebraic, differential, end-to-end, or compile-fail test. Generate the
matrix where possible; review every manual bridge. At minimum it must cover:

- principal inferred types, value/data kinds, total evaluation, finite data, complete calls, deterministic budgets, and
  module abstraction;
- the surface elaborations that add no core term — expression `if`, nominal record update, and `Result`-specific `?` —
  each shown observationally equal to the core form it elaborates to, at the same charge, with its subject and each
  right-hand side evaluated exactly once;
- the sealed-step syntax traversal: sealed formation and association, inherited context, repeatability of a captured
  step, local structural decrease plus the reducibility/fundamental-lemma cases for higher-order contexts/results,
  capture, duplication, delayed use, and nested traversal, opacity, derivation of the bottom-up fold, budget accounting,
  and phase conservativity;
- event-track algebra, machine formation/steps, initialized feedback, structural folds, template identity, provenance
  multiplicity, and exact cached/uncached arguments where a cache actually exists;
- written pitch/interval action, scales/degrees/context, chord class/voicing, pc12/set/row operations, transformations,
  tonal construction, schemas, assertions, tonal analysis, and voice-leading/counterpoint profiles;
- `.musa.events` document inclusion and local typed quote/antiquote, including hygiene, unknown payloads, source maps,
  and the context-neutral boundary;
- exact performance gestures/control curves, checked scheduling, tempo/tuning realization, typed instrument machines,
  part routing, mix, assets/packages, sampler adapters, media cues/clips, and every prompt 192 row;
- source/host ownership: declarable sound vocabulary and policy live in ordinary packages, host registrations satisfy
  only `00-semantics.md`'s ownership test, Rust projections have exact source derivations, and dependent sound
  relationships use the one pattern unifier rather than a domain-specific solver;
- parser recovery, formatting idempotence, tree-sitter drift, LSP facts, editor extension assets, desktop navigation,
  all exports, playback scheduling, last-valid-artifact behavior, and prompt 127/184 budgets.

Five rows exist because these boundaries are cheap to hold and expensive to recover:

- **Nested patterns still compile to the one case tree.** Prompt 155 installed the case-tree compiler and prompt 162c
  consequently replaced the earlier depth-one restriction: a constructor, list, or record sub-position holds another
  pattern recursively. Audit the differential law against the equivalent explicit nested match, unchanged coverage and
  impossible-branch refinement, repeated-variable refusal, and arbitrary-depth parser/tree-sitter agreement. There is
  still no guard, fall-through equation, or pattern on the left of a definition, and no evaluator beside the compiled
  case tree. Prompt 127dcfab's expression `if` remains an elaboration to the boolean case tree rather than a guarded arm.
- **Structural descent is not general recursion.** Prompt 127dcfaf's sealed steps let an adapter enter a strict subtree;
  nothing in the language lets it enter itself. Audit that no `fix`, recursive binding, self-application, or unsealed
  child value exists in source or adapter code, that sealing still enforces association, and that the reducibility proof
  still discharges higher-order capture, duplication, delayed use, and nested recursors. A local proper-child lemma is
  not the whole termination argument, and that distinction is the one a later convenience is most likely to blur.
- **Finite source evaluation and running machines remain different actions.** A source term may construct and connect a
  machine but never advances its unbounded history. The compiler evaluator contains no machine state or audio callback;
  the runtime contains no source closure or evaluator environment.
- **Notation adapters stay before inference and type blind.** No adapter or macro reads an inferred type, runs an audio
  machine, or creates a second checker. Expanded terms retain exact source maps.
- **Every normative refusal has provenance.** `docs/rules/language/02-core-calculus.md` §7 and
  `docs/rules/events/10-term-calculus.md` §"Provenance of the sharing discipline" cite the literature the design's
  refusals are priced against. Audit that every "deliberately absent" item across `docs/rules/language/` and
  `docs/rules/events/` either carries a citation or a musical falsifier. The project cites Open Music Theory by filename
  for every claim about music; a claim about programming languages is held to the same standard or it is an opinion.

Run the representative corpus through source parse/format/reparse, adapter expansion, inference/evaluation, event-track
normalization/round-trip, every applicable render/export backend, audio preparation/offline/live-plan paths, project
facts, offline package resolution, and editor protocol fixtures. Compare results at the strongest lawful level: byte
identity where promised, otherwise documented semantic normal-form or observation equality. Review all diagnostic
wording with both category-correct technical terminology and a musician-comprehensible first sentence.

Search for and remove stale alternate paths — `docs/plan/clean-break-ledger.md` is the list, and a row that still
resolves in the workspace is a finding: contextual `Music`, partial/default calls, `Timeline`, old event track
spellings, public HIR/evaluator types, `StudioGraphSpec`, `compile_graph`, global note streams, raw public DSP parameter
ids, block-defined feedback, unchecked asset paths, handwritten editor vocabularies, mutable expanded ASTs, and
source-independent widget state. Removed forms stay removed; do not restore them for compatibility.

Also reject a subtler alternate path: an authoritative Rust `StudioSpec`, `Gesture`/`ControlKey` enum, `InstrumentSpec`,
quantity/unit table, sample-map ontology, or tooling catalogue that mirrors source declarations. An opaque runtime
projection is allowed only when prompt 192 names its checked source owner and exact differential law.

Graduation is conditional. If any row lacks implementation or evidence, leave `docs/rules/language/` candidate, repair
the smallest responsible prompt or add a narrowly scoped follow-up, and stop. Only a fully green score and audio matrix
may update governing-document precedence in `docs/README.md`, the roadmap, the prompt README, and AGENTS.md.

## Target

- `scripts/check-language-conformance.sh`, its generated whole-language matrix, and an audit report including named
  exceptions (ideally none), clean-break deltas, theory citation/proof coverage, audio conformance, public-surface
  audit, and final performance comparison.
- End-to-end fixtures spanning musician-facing tonal/modal music, flexible time, phrase-led and audio-led work,
  templates/modules, post-tonal/serial material, analysis/counterpoint evidence, imports/packages, event-track
  documents/quotes, instruments/controls, sample adapters, routing/media, deterministic offline playback, and one live
  machine protocol.
- Discharge of the retained corrections through prompt 114: every musical base type and compiler-owned operation
  classified and covered by `docs/rules/language/02-core-calculus.md` §5.8's theorem with its premises checked, no
  bundled source file reachable from no import, no surviving hand-maintained parallel module list, and no accepted
  removed syntax anywhere in the corpus. Every hard-error migration rule emits its applicable fix.
- Repairs required solely to satisfy already-specified behavior; repair prompt/design text in the same commit and record
  why implementation evidence required it.
- On a completely green audit only: graduate `docs/rules/language/` and update its status line and the precedence ladder
  in `docs/README.md`. The documentation reconciliation this step once also owned — dissolving the design essay and the
  correction memo into `docs/rules/language/` — was done ahead of this prompt in the documentation reorganization; only
  the graduation itself remains.

## Check

```sh
./scripts/check-language-conformance.sh
./scripts/check-audio-language-conformance.sh
cargo nextest run --workspace
cargo clippy --workspace --all-targets -- -D warnings
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
cargo deny check
cargo insta test --workspace --unreferenced=reject
cd editors/tree-sitter-musa && tree-sitter test
cd apps/musa-desktop/ui && npx pnpm run check && npx pnpm run test:unit
cd apps/musa-desktop/ui && npx playwright test
git -C ../vscode-musa diff --check
git -C ../zed-musa diff --check
```

Repeat prompts 127 and 187 release comparisons on their recorded benchmark hosts and attach the results. Record manual
smoke tests for VS Code, Zed, and the desktop workbench. Commit each affected repository intentionally, record
cross-repository commit ids, and commit Musa as `Graduate the Musa language`.

## Stop

- No new surface construct, theory/audio feature, backend, syntax alias, or opportunistic refactor.
- No weakened golden, deleted failing test, hidden retained path, or undocumented conformance exception.
- No graduation with a red or unowned matrix row or a red prompt 192 audit.
- No claim that passing tests proves a theoretical convention or audio-format interpretation universal beyond its
  documented domain.
