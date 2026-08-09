---
id: 119
slug: elaboration-conformance
status: pending
depends_on: [113, 114, 115, 116, 117, 118]
phase: 3
---

# Elaboration Language Conformance and Graduation

## Task

Audit prompts 92–118 as one language release, close every compatibility, law, theory, provenance, tooling,
documentation, and performance obligation, and graduate `docs/language/` from candidate to governing. This prompt adds
no feature. It demonstrates that one well-typed source semantics reaches the kernel, renderers, playback, project,
editors, and desktop without a competing evaluator or an undocumented exception.

## Read

- Prompt 92's acceptance matrix and all completion/repair notes from prompts 93–118.
- `docs/elaboration-language.md`, all of `docs/language/`, `docs/kernel/`, `docs/interface/`, the roadmap, course
  correction, AGENTS.md, and prompt README.
- Prompt 93's compatibility baseline and prompt 118's final benchmark report.
- The OMT citation map and local proof obligations delivered by prompt 117.

## Design

Build a machine-checked conformance matrix from each normative rule/law in `docs/language/` to its implementation
owner and at least one positive, negative, algebraic, differential, end-to-end, or compile-fail test. Generate the
matrix where possible; review every manual bridge. At minimum it must cover:

- total evaluation, finite data, budget determinism, closure/partial-application semantics, and module abstraction;
- contextual `Music`, structural folds, canonical composition laws, template identity, provenance multiplicity, and
  cached/uncached equality;
- written pitch/interval action, scales/degrees/context, chord class/voicing, pc12/set/row operations,
  transformations, tonal construction, schemas, assertions, tonal analysis, and voice-leading/counterpoint profiles;
- `.musa.kernel` document inclusion and local typed quote/antiquote, including hygiene, unknown payloads, source maps,
  and the context-neutral boundary;
- parser recovery, formatting idempotence, tree-sitter drift, LSP facts, editor extension assets, desktop navigation,
  all exports, playback scheduling, and last-valid-artifact behavior;
- prompt 93 compatibility results and prompt 118 latency/memory/allocation budgets on the recorded environment.

Run the representative corpus through source parse/format/reparse, elaboration, kernel normalization/round-trip, every
applicable render/export backend, project facts, and editor protocol fixtures. Compare results at the strongest lawful
level: byte identity where promised, otherwise documented semantic normal-form or observation equality. Review all
diagnostic wording with both category-correct technical terminology and a musician-comprehensible first sentence.

Search for and remove or document stale alternate paths: legacy direct lowering, duplicate theory algorithms, public
HIR/evaluator types, handwritten editor vocabularies, undocumented keyword aliases, mutable expanded ASTs, and
source-independent widget state. Do not delete compatibility behavior merely because the new path exists.

Graduation is conditional. If any row lacks implementation or evidence, leave `docs/language/` candidate, repair the
smallest responsible prompt or add a narrowly scoped follow-up prompt, and stop this prompt. Only a fully green matrix
may update governing-document precedence in the roadmap, course correction, prompt README, and AGENTS.md.

## Target

- `scripts/check-language-conformance.sh`, its generated conformance matrix, and an audit report including named
  exceptions (ideally none), compatibility deltas, theory citation/proof coverage, public-surface audit, and final
  performance comparison.
- An end-to-end fixture set spanning musician-facing tonal/modal music, templates/modules, post-tonal/serial material,
  analysis/counterpoint evidence, assertions, imports, kernel documents, and quotes.
- Repairs required solely to satisfy already-specified behavior; repair prompt/design text in the same commit and record
  why implementation evidence required it.
- On a completely green audit only: graduate `docs/language/`, update governing-document precedence and status, and
  reconcile `docs/elaboration-language.md` as non-governing design history.

## Check

```sh
./scripts/check-language-conformance.sh
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

Repeat prompt 118's release comparison on the recorded benchmark host and attach the results. Record manual smoke tests
for VS Code, Zed, and the desktop workbench. Commit each affected repository intentionally, record cross-repository
commit ids, and commit Musa as `Graduate the elaboration language`.

## Stop

- No new surface construct, theory feature, backend, syntax alias, or opportunistic refactor.
- No weakened golden, deleted failing test, hidden compatibility delta, or undocumented conformance exception.
- No graduation with a red or unowned matrix row; repair the responsible earlier prompt or create a follow-up.
- No claim that passing tests proves a theoretical convention universal beyond its documented domain.
