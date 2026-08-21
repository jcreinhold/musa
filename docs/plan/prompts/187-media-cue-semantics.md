---
id: 187
slug: media-cue-semantics
status: pending
depends_on: [174, 177, 182]
phase: 4
---

# A Recording Has Either Musical Extent or Physical Duration

> **Governed by the event-track and machine core installed by prompts 127a–127e and 171–174.** Recorded-media intent is
> event data; decoded playback is a machine implementation.

## Task

Settle and implement the source/compiler semantics of recorded media without pretending seconds are beats. Distinguish
note-driven samples, beat-fitted musical clips, and fixed-media cues; define how each behaves under sequence, overlay,
repeat, stretch, restriction, realization, tempo changes, provenance, notation projection, and semantic/audio identity
before any clip player is written.

## Read

- `docs/rules/constitution.md` §4 and §8 — the boundary this prompt draws at the surface is the same one the core
  boundary draws in the type system, and rule 4 is the constraint the cue payload has to satisfy.
  `docs/rules/events/12-payload-admission.md` from prompt 176a for what a media payload owes.
- `docs/rules/language/08-performance-and-sound.md` and `09-assets-and-packages.md`; event-track occurrence/transform
  laws; backend contract's Beat→Second realization; prompt 70's printed `sample`/`cue` marks; prompt 182 assets.
- OMT `098-twentieth-century-rhythmic-techniques.md` on timeline notation using seconds. Cite it for the musical
  distinction; state the exact Musa behavior as local definitions and laws.
- Existing `FactKind`, point occurrences, `Progress`, realization/provenance, notation loss reporting, and audio export
  tail behavior.

## Design

Define three disjoint constructs:

1. A **sample instrument** is triggered by note gestures and is not a media occurrence (prompts 184–186).
2. A **musical clip** is an interval occurrence `[s,e)` in `EventTrack<WrittenTime,MediaAction>` with an explicit fit
   policy. Initial policies are `crop`, `loop`, and honest playback `rate`; rate changes both duration and pitch unless
   a later pitch-preserving warp feature says otherwise.
3. A **fixed-media cue** is a point occurrence at written position `b` referencing an asset and playback settings. Its
   physical start is `tempo(b)` and its physical end is `tempo(b)+L`; its asset duration `L` is never stored as an event
   track extent.

A musical clip has written duration, so it is an event-track occurrence and every track law applies. A fixed-media cue
has physical duration but only a written onset, so its event-track support is a point. The decoded duration belongs to
the registered primitive configuration created during preparation, not to the written occurrence. State and test this
boundary directly.

Track transforms move, duplicate, or restrict occurrence support only. Stretching or repeating a fixed cue moves or
duplicates its onset but does not stretch its media. Retrograde relocates the cue and does not reverse audio. A musical
clip's beat interval transforms normally; its fit policy determines downstream playback. Make every non-law explicit.

Both facts retain Origin and asset identity. Notation renders an optional labelled cue/clip annotation and reports
losses per backend; the score UI may show a derived physical region, clearly distinguished from event track support.
Keep Keep prompt 70's generic printed marks valid, but do not infer playback from a matching string; executable media
uses typed declarations.

## Target

- Normative spec repair with definitions, equations, transformation table, counterexamples, and local proofs.
- Settled parser/CST/formatter/tree-sitter syntax and compiler facts for musical clips and fixed-media cues.
- Kernel/elaboration/provenance/normalization/property tests; notation and diagnostic fixtures.
- No audio playback yet; compiler/project facts contain asset references and policy, never decoded media.

## Check

```sh
cargo nextest run -p musa-syntax -p musa-events -p musa-compiler -p musa-notation -p musa-project
cargo clippy --all-targets -p musa-syntax -p musa-events -p musa-compiler -p musa-notation -p musa-project -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
cd editors/tree-sitter-musa && tree-sitter test
```

Commit as `Define musical clips and fixed media cues`.

## Stop

- No seconds-long written-time occurrence, automatic inference from `mark sample`, waveform editing, recording, or
  hidden tempo stretching.
- No sample count or frame index in any written or gesture payload. `L` is prepared machine configuration.
- No pitch-preserving time-warp promise; it needs a separate quality/performance design if requested later.
- No playback or DSP implementation — prompt 188.
