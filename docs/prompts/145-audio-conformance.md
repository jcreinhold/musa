---
id: 145
slug: audio-conformance
status: pending
depends_on: [128, 129, 129a, 130, 131, 132, 133, 134, 135, 136, 137, 138, 139, 140, 141, 142, 143, 144]
phase: 4
---

# Audit the Performance and Sound Language

> **Governed by `docs/governance/01-constitution.md` §7 and §4.** Prompt 126 decided that the core is a calculus of
> occurrences of any canonical payload, that signals stay outside it, and what that forbids. Read them before this
> prompt's Design.

## Task

Audit prompts 130–144 as one coherent performance/sound implementation. Trace every rule and compatibility claim in
`docs/language/08-performance-and-sound.md` and `09-assets-and-packages.md` to an owner and executable evidence; close
every routing, exactness, control, instrument, asset, package, sample-format, media, UI, tooling, determinism, and
real-time row before the whole-language graduation prompt may run. This prompt adds no feature.

## Read

- `docs/governance/01-constitution.md` §7 and §4 in full — the things the decision forbids are audit rows here, and this is the prompt
  that catches a later prompt having quietly re-opened one. `docs/kernel/12-payload-admission.md`'s admission table.
- `docs/spec/03-process-calculus.md`, `04-identity-and-realization.md`, and the architecture spec-to-implementation map.
- Prompt 93 baseline and expected-change ledger; all prompt 128–142 completion/repair notes and benchmark artifacts.
- The candidate sound/assets specs, roadmap/course-correction/kernel boundaries, interface specification, handbook, SFZ
  support matrix, SoundFont support matrix, package/asset schemas, and public crate facades.
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
- **the core boundary itself** — one row per rule forbidden by `docs/governance/01-constitution.md` §7 and §4, each with executable evidence rather than
  a reading:
  - no fourth combinator, and no addition to `musa-kernel`'s public surface since prompt 126 (`git diff` on the facade
    is the evidence);
  - **no bespoke temporal structure above the kernel** — no type outside `musa-kernel` pairs a rational extent with a
    positioned collection and defines its own ordering or equality. This is the rule most likely to have been broken by
    convenience, and finding one is a repair of the owning prompt, not an exception here;
  - every payload instantiated anywhere has a row in the admission table; owner/quotient versions match its key, framed
    semantic encoding survives adversarial delimiters, and no hash-only comparison is treated as exact equality;
  - no signal, stream, or other coinductive value in a payload, and no absolute time (seconds, frames, samples) in one —
    including a fixed cue's asset duration;
  - no dependent indices in the kernel; part, voice, meter, tuning, and transposition remain payload data;
  - no `join` over `Timeline[Timeline[A]]` at any payload;
  - `StudioSpec` is still a finite dataflow description with no extent, and its prepared private process IR satisfies
    whole-node scheduling, registered-feedback, fixed-semantic-tick, causality, and host-block-partition laws;
  - `musa-kernel` still depends on no musical type.

The prompt 93 expected-change ledger must be empty. Test unsupported SFZ/SoundFont behavior rather than counting rows in
a support table. Run fault injection for missing/corrupt assets, digest drift, offline package cache, plan preparation
failure, reinstall, and callback underrun if streaming was admitted. Audit dependency direction and public surfaces;
remove dead compatibility internals but retain accepted source aliases according to their deprecation policy.

A red row in the core-boundary group is repaired at the owning prompt, or — if the rule itself turns out to be wrong —
by amending `docs/governance/01-constitution.md` §7, by its README's amendment procedure, which is a decision and therefore
not this prompt's to make alone. If any row is red or unowned, repair the smallest responsible prompt/design and stop
this prompt. Do not weaken a law, support claim, golden, or budget to make the matrix green. This audit does not yet
graduate `docs/language/`; prompt 146 does so only after combining it with the score/elaboration audit.

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
