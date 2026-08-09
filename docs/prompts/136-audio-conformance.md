---
id: 136
slug: audio-conformance
status: pending
depends_on: [119, 120, 121, 122, 123, 124, 125, 126, 127, 128, 129, 130, 131, 132, 133, 134, 135]
phase: 4
---

# Audit the Performance and Sound Language

## Task

Audit prompts 119–135 as one coherent performance/sound implementation. Trace every rule and compatibility claim in
`docs/language/08-performance-and-sound.md` and `09-assets-and-packages.md` to an owner and executable evidence; close
every routing, exactness, control, instrument, asset, package, sample-format, media, UI, tooling, determinism, and
real-time row before the whole-language graduation prompt may run. This prompt adds no feature.

## Read

- Prompt 93 baseline and expected-change ledger; all prompt 119–135 completion/repair notes and benchmark artifacts.
- The candidate sound/assets specs, roadmap/course-correction/kernel boundaries, interface specification, handbook,
  SFZ support matrix, SoundFont support matrix, package/asset schemas, and public crate facades.
- Every audio preparation/offline/live/project/CLI/LSP/desktop caller and every unsafe/RT-sensitive block.

## Design

Generate a conformance matrix mapping each normative rule to owner, positive/negative/law/differential/end-to-end test,
and observed result. At minimum cover:

- score independence; exact gesture/control construction; tempo realization; frame rounding; curve endpoints;
- instrument signature checking, private topology, swapping, standard/custom controls, technique/fallback policy;
- part/declaration/instance identity, routing isolation, sends/buses/main, same-frame and block-partition behavior;
- exact written quantities and the unique DSP conversion boundary;
- asset root/digest/invalidation, exact package lock/offline behavior, path/archive/resource defenses;
- native sample maps, deterministic selection, SFZ and SoundFont claimed support/loss matrices;
- musical clips versus fixed cues, transform/tempo/seek/tail behavior;
- source-edit authority, Origin/navigation, generated docs, editor drift, accessibility, stale/error states;
- offline/live equality, deterministic outputs, NaN/silence laws, callback/retirement instrumentation, and prompt 135
  budgets.

The prompt 93 expected-change ledger must be empty. Test unsupported SFZ/SoundFont behavior rather than counting rows in
a support table. Run fault injection for missing/corrupt assets, digest drift, offline package cache, plan preparation
failure, reinstall, and callback underrun if streaming was admitted. Audit dependency direction and public surfaces;
remove dead compatibility internals but retain accepted source aliases according to their deprecation policy.

If any row is red or unowned, repair the smallest responsible prompt/design and stop this prompt. Do not weaken a law,
support claim, golden, or budget to make the matrix green. This audit does not yet graduate `docs/language/`; prompt 137
does so only after combining it with the score/elaboration audit.

## Target

- `scripts/check-audio-language-conformance.sh`, generated matrix, audit report, and named exceptions (ideally none).
- End-to-end fixture project using functions/templates/theory, two profiles/instruments, controls, room/send, native
  sampler, SFZ, SoundFont, locked package asset, musical clip, and fixed cue.
- Repairs required solely for specified behavior, with rationale linked to the owning prompt.
- Public API/dependency/RT audit and final prompt 135 comparison attached.

## Check

```sh
./scripts/check-audio-language-conformance.sh
cargo nextest run --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo deny check
cargo insta test --workspace --unreferenced=reject
cd apps/musa-desktop/ui && npx pnpm run check && npx pnpm run test:unit
cd apps/musa-desktop/ui && npx playwright test
```

Commit as `Audit the Musa sound language`.

## Stop

- No new syntax, processor, format, package feature, UI mode, optimization, or opportunistic refactor.
- No ignored unsupported opcode/generator, hidden callback fallback, or undocumented conformance exception.
- No graduation while a matrix row or expected-change entry remains open.
