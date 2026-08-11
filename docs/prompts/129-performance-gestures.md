---
id: 129
slug: performance-gestures
status: pending
depends_on: [93, 119, 128]
phase: 3
---

# Performance Produces Gestures, Not Knob Addresses

> **Contingent on prompt 125.** The core-boundary decision may repair this prompt's Design, fold it into another, or
> replace it. Read `docs/core-boundary.md` first.

## Task

Introduce the missing instrument-independent performance object between `Timeline<ScoreFact>` and scheduled DSP events.
Profiles interpret notation into exact note gestures, technique/grouping information, and typed musical control curves;
tempo/groove/tuning then schedule those gestures. No gesture names a graph node, processor, MIDI controller, or
render-plan parameter index.

## Read

- `docs/language/08-performance-and-sound.md`; kernel `Progress` semantics and backend contract; roadmap §§6.4–6.5.
- OMT `007-other-aspects-of-notation.md` for dynamics/articulation and `114-core-principles-of-orchestration.md` for why
  loudness change is not one DSP operation.
- Current `profile.rs` and `performance.rs`; hairpin, slur, pedal, ornament, grace, groove, tuning, MIDI, and polytempo
  consumers. Prompt 93's performance-lane baseline.

## Design

Define exact internal `GestureTimeline`/`GestureLane` values. A note gesture carries stable event/part identity, written
pitch until tuning, onset/extent, separation/hold/emphasis intent, symbolic technique tags, legato/phrase grouping, and
per-note controls. A lane also carries piecewise exact `ControlCurve`s keyed by semantic `ControlKey` and typed by a
small control value family. Standard keys include expression, emphasis, separation, brightness, sustain, and legato;
namespaced custom keys are admitted only with a declaration in prompt 130.

For a hairpin on `[s,e]`, specify and test `E(b) = d0 + (d1-d0) * p((b-s)/(e-s))`, with exact profile endpoints and
kernel `Progress p`. Curve construction is normative; frame/control-rate sampling is downstream. Preserve symbolic
technique/group identity even when a numeric fallback is also available, so a sample instrument may select legato or
staccato regions rather than receiving only a gate multiplier.

Keep the public compiler facade narrow. Compare (and record) a separate public `PerformanceIntent` artifact with the
chosen design in which private exact gestures are scheduled into a caller-oriented `PerformancePlan`; publish only the
minimum immutable lane/control information required by MIDI and `musa-audio`. `ParameterId(u32)` is not reused as
`ControlKey`.

## Target

- Private exact gesture construction and revised scheduled performance lanes/events.
- Profile-to-gesture interpretation for all existing marks with byte/semantic parity where the old model was expressive
  enough; explicit retained/fallback information where it was not.
- Algebraic/property tests for curve endpoints, monotonic hairpins, grouping, exactness, context changes, and
  scheduling.
- MIDI remains a separate consumer with documented mappings/losses; no studio dependency enters the compiler pass.

## Check

```sh
cargo nextest run -p musa-compiler -p musa-render -p musa-project
cargo clippy --all-targets -p musa-compiler -p musa-render -p musa-project -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
cargo bench -p musa-compiler
```

Commit as `Interpret notation as performance gestures`.

## Stop

- No instrument graph, sample selection, part routing, or DSP parameter resolution.
- No kernel operation or physical-time value in a score occurrence.
- No universal ontology of expression; standard controls have documented Musa meanings and custom controls remain
  explicitly declared.
