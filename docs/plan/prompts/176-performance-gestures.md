---
id: 176
slug: performance-gestures
status: pending
depends_on: [119, 173, 175, 175a]
phase: 3
---

# Performance Produces Gestures, Not Knob Addresses

> **Governed by the event-track and machine core installed by prompts 127a–127e and 170–173.** Gestures use the same
> `EventTrack` structure as written facts and reach sound through the checked scheduler.

## Task

Introduce the missing instrument-independent performance object between `EventTrack<WrittenTime,ScoreFact>` and a
scheduled event-source machine. Profiles interpret notation into exact note gestures, technique/grouping information,
and typed musical control curves; tempo/groove/tuning then schedule those gestures. No gesture names a machine
primitive, processor, MIDI controller, or render-plan parameter index.

## Read

- `docs/rules/constitution.md` §8 (the decision and what it forbids, in particular: no bespoke temporal structure above
  the core). `docs/rules/events/12-payload-admission.md` from prompt 175a — the rule this prompt's payload must satisfy,
  written before the payload existed so that it could not be fitted to it.
- `crates/musa-events/src/{term,timeline,occurrence}.rs` and `tests/laws.rs`, which already prove L1–L24 at a payload
  that is not `ScoreFact`.
- `docs/rules/language/08-performance-and-sound.md`; event track `Progress` semantics and backend contract; roadmap
  §§6.4–6.5.
- OMT `007-other-aspects-of-notation.md` for dynamics/articulation and `114-core-principles-of-orchestration.md` for why
  loudness change is not one DSP operation.
- Current `profile.rs` and `performance.rs`; hairpin, slur, pedal, ornament, grace, groove, tuning, MIDI, and polytempo
  consumers. Prompt 93's performance-lane baseline.

## Design

The gesture object is `EventTrack<PerformedTime,Gesture>`, not a new structure. It receives the event-track ordering,
payload serialization, exact equality, and exact encoding rather than specifying them again. Define `Gesture`, implement
the revised storable-data contract for it, and add its payload-admission row. State what the exact encoding includes and
what any separate musical comparison deliberately ignores. Writing a private `Vec<(Ratio,Ratio,Gesture)>` with its own
ordering or equality is a defect, and prompt 191 audits for it.

Nothing is added to `musa-events`: no term form, operation, or public-surface change. `follow`, `together`, and retained
observations on a gesture track keep their existing laws. A control curve remains payload data; frame sampling stays in
the scheduler or instrument machine.

A note gesture carries stable event/part identity, written pitch until tuning, onset/extent, separation/hold/emphasis
intent, symbolic technique tags, legato/phrase grouping, and per-note controls. A lane also carries piecewise exact
`ControlCurve`s keyed by semantic `ControlKey` and typed by a small control value family. Standard keys include
expression, emphasis, separation, brightness, sustain, and legato; namespaced custom keys are admitted only with a
declaration in prompt 177.

For a hairpin on `[s,e]`, specify and test `E(b) = d0 + (d1-d0) * p((b-s)/(e-s))`, with exact profile endpoints and
events `Progress p`. Curve construction is normative; frame/control-rate sampling is downstream. Preserve symbolic
technique/group identity even when a numeric fallback is also available, so a sample instrument may select legato or
staccato regions rather than receiving only a gate multiplier.

Keep the public compiler facade narrow. Compare (and record) a separate public `PerformanceIntent` artifact with the
chosen design in which private exact gestures are scheduled into a caller-oriented `PerformancePlan`; publish only the
minimum immutable lane/control information required by MIDI and `musa-dsp`. `ParameterId(u32)` is not reused as
`ControlKey`.

## Target

- Private exact gesture construction as `EventTrack<PerformedTime,Gesture>`, its exact encoding and admission-table row,
  and revised caller-facing performance facts.
- Profile-to-gesture interpretation for all existing marks with byte/semantic parity where the old model was expressive
  enough; explicit retained/fallback information where it was not.
- Algebraic/property tests for curve endpoints, monotonic hairpins, grouping, exactness, context changes, and
  scheduling.
- MIDI remains a separate consumer with documented mappings/losses; no studio dependency enters the compiler pass.

## Check

```sh
cargo nextest run -p musa-compiler -p musa-notation -p musa-project
cargo clippy --all-targets -p musa-compiler -p musa-notation -p musa-project -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
cargo bench -p musa-compiler
```

Commit as `Interpret notation as performance gestures`.

## Stop

- No instrument implementation, sample selection, part routing, or DSP parameter resolution.
- No event-track operation, new term form, or physical-time value in any score or gesture occurrence. Exact physical
  time and frames appear in scheduling decisions, not in the event payload.
- No second temporal structure. If `EventTrack<PerformedTime,Gesture>` will not carry something, that is a finding to
  report against `docs/rules/constitution.md` §8, not a licence to write a parallel container.
- No universal ontology of expression; standard controls have documented Musa meanings and custom controls remain
  explicitly declared.
