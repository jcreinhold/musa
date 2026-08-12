---
id: 140
slug: media-cue-semantics
status: pending
depends_on: [92, 130, 135]
phase: 4
---

# A Recording Has Either Musical Extent or Physical Duration

> **Governed by `docs/governance/01-constitution.md` §7 and §4.** Prompt 126 decided that the core is a calculus of
> occurrences of any canonical payload, that signals stay outside it, and what that forbids. Read them before this
> prompt's Design.

## Task

Settle and implement the source/compiler semantics of recorded media without pretending seconds are beats. Distinguish
note-driven samples, beat-fitted musical clips, and fixed-media cues; define how each behaves under sequence, overlay,
repeat, stretch, restriction, realization, tempo changes, provenance, notation projection, and semantic/audio identity
before any clip player is written.

## Read

- `docs/governance/01-constitution.md` §4 and §7 — the boundary this prompt draws at the surface is the same one the
  core boundary draws in the type system, and rule 4 is the constraint the cue payload has to satisfy.
  `docs/kernel/12-payload-admission.md` from prompt 129a for what a media payload owes.
- `docs/language/08-performance-and-sound.md` and `09-assets-and-packages.md`; kernel occurrence/transform laws; backend
  contract's Beat→Second realization; prompt 70's printed `sample`/`cue` marks; prompt 135 assets.
- OMT `098-twentieth-century-rhythmic-techniques.md` on timeline notation using seconds. Cite it for the musical
  distinction; state the exact Musa behavior as local definitions and laws.
- Existing `FactKind`, point occurrences, `Progress`, realization/provenance, notation loss reporting, and audio export
  tail behavior.

## Design

Define three disjoint constructs:

1. A **sample instrument** is triggered by note gestures and is not a media occurrence (prompts 139–139).
2. A **musical clip** is an interval occurrence `[s,e]` in beats with an explicit fit policy. Initial policies are
   `crop`, `loop`, and honest playback `rate`; rate changes both duration and pitch unless a later pitch-preserving warp
   feature says otherwise.
3. A **fixed-media cue** is a point occurrence at beat `b` referencing an asset and playback settings. Its physical
   start is `tempo(b)` and its physical end is `tempo(b)+L`; its asset duration `L` is never stored as a kernel extent.

**This distinction is `docs/governance/01-constitution.md` §4 at the surface, and the Design must say so.** A musical
clip has musical extent, so it is an occurrence in a timeline and every kernel law applies to it. A fixed-media cue has
physical duration, which the core has no vocabulary for, so it is a *point* occurrence carrying an asset reference and
playback settings — and its duration `L` belongs to the prepared plan, not to any payload. §6 rule 4 forbids absolute
time in a payload outright: `L` is read from the asset at preparation, and a cue payload that stored it would be a
payload the kernel could not be a calculus of. State that as a local invariant with a test, not as a convention.

Kernel transforms move/duplicate/restrict the occurrence support only. Stretching/repeating a fixed cue moves or
duplicates its onset but does not stretch its media. Retrograde relocates the cue and does not reverse audio. A musical
clip's beat interval transforms normally; its fit policy determines downstream playback. Make every non-law explicit.

Both facts retain Origin and asset identity. Notation renders an optional labelled cue/clip annotation and reports
losses per backend; the score UI may show a derived physical region, clearly distinguished from kernel support. Keep
prompt 70's generic printed marks source-compatible, but do not infer playback from a matching string; executable media
uses typed declarations.

## Target

- Normative spec repair with definitions, equations, transformation table, counterexamples, and local proofs.
- Settled parser/CST/formatter/tree-sitter syntax and compiler facts for musical clips and fixed-media cues.
- Kernel/elaboration/provenance/normalization/property tests; notation and diagnostic fixtures.
- No audio playback yet; compiler/project facts contain asset references and policy, never decoded media.

## Check

```sh
cargo nextest run -p musa-language -p musa-kernel -p musa-compiler -p musa-render -p musa-project
cargo clippy --all-targets -p musa-language -p musa-kernel -p musa-compiler -p musa-render -p musa-project -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
cd editors/tree-sitter-musa && tree-sitter test
```

Commit as `Define musical clips and fixed media cues`.

## Stop

- No seconds-long kernel occurrence, automatic inference from `mark sample`, waveform editing, recording, or hidden
  tempo stretching.
- No physical duration, sample count, or frame index in any payload, including a cue's. `L` is a prepared-plan fact
  (`docs/governance/01-constitution.md` §4).
- No pitch-preserving time-warp promise; it needs a separate quality/performance design if requested later.
- No playback or DSP implementation — prompt 141.
