---
id: 130
slug: performance-gestures
status: pending
depends_on: [93, 119, 129, 129a]
phase: 3
---

# Performance Produces Gestures, Not Knob Addresses

> **Governed by `docs/core-boundary.md`.** Prompt 126 decided that the core is a calculus of occurrences of any
> canonical payload, that signals stay outside it, and what that forbids. Read it before this prompt's Design.

## Task

Introduce the missing instrument-independent performance object between `Timeline<ScoreFact>` and scheduled DSP events.
Profiles interpret notation into exact note gestures, technique/grouping information, and typed musical control curves;
tempo/groove/tuning then schedule those gestures. No gesture names a graph node, processor, MIDI controller, or
render-plan parameter index.

## Read

- `docs/core-boundary.md` §4 (the decision and its obligations) and §6 (what it forbids, in particular rule 2: no
  bespoke temporal structure above the kernel). `docs/kernel/12-payload-admission.md` from prompt 129a — the rule this
  prompt's payload must satisfy, written before the payload existed so that it could not be fitted to it.
- `crates/musa-kernel/src/{term,timeline,occurrence}.rs` and `tests/laws.rs`, which already prove L1–L24 at a payload
  that is not `ScoreFact`.
- `docs/language/08-performance-and-sound.md`; kernel `Progress` semantics and backend contract; roadmap §§6.4–6.5.
- OMT `007-other-aspects-of-notation.md` for dynamics/articulation and `114-core-principles-of-orchestration.md` for why
  loudness change is not one DSP operation.
- Current `profile.rs` and `performance.rs`; hairpin, slur, pedal, ornament, grace, groove, tuning, MIDI, and polytempo
  consumers. Prompt 93's performance-lane baseline.

## Design

**`GestureTimeline` is `Timeline<Gesture>` — the kernel at a second payload, not a new structure.** This is prompt 126's
decision applied: the object between `Timeline<ScoreFact>` and scheduled DSP events has a rational extent and finitely
many things positioned in it, so it is a kernel timeline, and its ordering (N2), payload serialization (N3), semantic
equality (N4), and semantic hash (N6) come from `musa-kernel` rather than being specified again here. Define `Gesture`,
implement `Canonical` for it against prompt 129a's rule, and add its row to that document's admission table — stating
what the key includes and what it deliberately quotients away. Writing a `Vec<(Beat, Beat, Gesture)>` with its own
ordering and its own equality is the defect this repair exists to prevent (`docs/core-boundary.md` §6 rule 2), and
prompt 145 audits for it.

Nothing is added to `musa-kernel`: no term form, no operation, no public-surface change. `sequence`, `overlay`, `scale`,
and `restrict` on a gesture timeline mean what L1–L24 already say they mean, and L24 is what keeps a control curve a
payload *value* — a `Progress` transforms by its span alone.

A note gesture carries stable event/part identity, written pitch until tuning, onset/extent, separation/hold/emphasis
intent, symbolic technique tags, legato/phrase grouping, and per-note controls. A lane also carries piecewise exact
`ControlCurve`s keyed by semantic `ControlKey` and typed by a small control value family. Standard keys include
expression, emphasis, separation, brightness, sustain, and legato; namespaced custom keys are admitted only with a
declaration in prompt 131.

For a hairpin on `[s,e]`, specify and test `E(b) = d0 + (d1-d0) * p((b-s)/(e-s))`, with exact profile endpoints and
kernel `Progress p`. Curve construction is normative; frame/control-rate sampling is downstream. Preserve symbolic
technique/group identity even when a numeric fallback is also available, so a sample instrument may select legato or
staccato regions rather than receiving only a gate multiplier.

Keep the public compiler facade narrow. Compare (and record) a separate public `PerformanceIntent` artifact with the
chosen design in which private exact gestures are scheduled into a caller-oriented `PerformancePlan`; publish only the
minimum immutable lane/control information required by MIDI and `musa-audio`. `ParameterId(u32)` is not reused as
`ControlKey`.

## Target

- Private exact gesture construction as `Timeline<Gesture>`, with `Canonical for Gesture` and its admission-table row,
  and revised scheduled performance lanes/events.
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
- No kernel operation, no new term form, no change to `musa-kernel`'s public surface, and no physical-time value in any
  occurrence — score or gesture. Seconds and frames appear at the prepared-plan boundary (prompt 131), not before.
- No second temporal structure. If `Timeline<Gesture>` will not carry something, that is a finding to report against
  `docs/core-boundary.md`, not a licence to write a parallel container.
- No universal ontology of expression; standard controls have documented Musa meanings and custom controls remain
  explicitly declared.
