---
id: 145
slug: audio-conformance
status: pending
depends_on: [127i, 128, 129, 129a, 130, 131, 132, 133, 134, 135, 136, 137, 138, 139, 140, 141, 142, 143, 144]
phase: 4
---

# Audit the Performance and Sound Language

> **Governed by the event-track and machine core installed by prompts 127a–127i.** This audit must catch any sound
> prompt that quietly restored a second event container, another machine semantics, or block-defined audio path.

## Task

Audit prompts 128–144 as one coherent performance/sound implementation. Trace every rule and canonical-source claim in
`docs/rules/language/08-performance-and-sound.md` and `09-assets-and-packages.md` to an owner and executable evidence;
close every routing, exactness, control, instrument, asset, package, sample-format, media, UI, tooling, determinism, and
real-time row before the whole-language graduation prompt may run. This prompt adds no feature.

## Read

- The revised constitution and obligations in full; the event-track payload-admission table.
- The revised machine, scheduling, identity, and audio-step specifications and architecture map.
- Prompt 127i's core conformance matrix and every completion/repair note from prompts 127a–127i.
- Prompt 93 baseline and expected-change ledger; all prompt 128–142 completion/repair notes and benchmark artifacts.
- The candidate sound/assets specs, roadmap/kernel boundaries, interface specification, handbook, SFZ support matrix,
  SoundFont support matrix, package/asset schemas, and public crate facades.
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
- offline/live equality, deterministic outputs, NaN/silence laws, callback/retirement instrumentation, and prompt 144
  budgets;
- **the core boundary itself** — executable evidence for every integration risk:
  - no type outside `musa-kernel` pairs a finite rational length with positioned events and defines its own ordering or
    equality;
  - every event payload and machine port/configuration is storable data with versioned injective encoding; no source
    closure appears at any depth and no hash-only comparison is exact equality;
  - no public `lift`, uninitialized feedback, zero-delay loop, or alternate machine interpreter exists;
  - every audio primitive has one functional build-local registration, bounded state/work, deterministic frame step, and
    explicit seed where it uses chance;
  - scheduling records every time/frame decision, keeps handles distinct under merging, reaches a fixed finished state,
    and preserves `together` only under its stated success and locality premises;
  - one sample frame remains the reference meaning; every optimized batch is checked against repeated frame steps,
    including feedback, modulation, envelopes, media, and seek;
  - notation, gestures, event tracks, machine descriptions, private machine state, and audio history remain distinct;
  - part, voice, meter, tuning, and transposition remain payload data rather than dependent core indices; and
  - `musa-kernel` still depends on no musical or audio type.

The clean-break and prompt-93 expected-change ledgers must be empty. Test unsupported SFZ/SoundFont behavior rather than
counting rows in a support table. Run fault injection for missing/corrupt assets, digest drift, offline package cache,
plan preparation failure, reinstall, and callback underrun if streaming was admitted. Audit dependency direction and
public surfaces; remove dead migration code and verify that removed source aliases are hard errors with certain fixes.

A red core row is repaired at its owning prompt. If implementation evidence refutes a governing rule, stop and use the
amendment procedure; this audit may not weaken the rule. This prompt does not yet graduate the language.

## Target

- `scripts/check-audio-language-conformance.sh`, generated matrix, audit report, and named exceptions (ideally none).
- End-to-end fixture project using functions/templates/theory, two profiles/instruments, controls, room/send, native
  sampler, SFZ, SoundFont, locked package asset, musical clip, and fixed cue.
- Repairs required solely for specified behavior, with rationale linked to the owning prompt.
- Public API/dependency/RT audit and final prompt 144 comparison attached.

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
