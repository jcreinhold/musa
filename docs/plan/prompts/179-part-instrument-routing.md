---
id: 179
slug: part-instrument-routing
status: pending
depends_on: [31, 174, 177, 178]
phase: 3
---

# A Part Sounds Only Through Its Instrument Instance

> **Governed by the event-track and machine core installed by prompts 127a–127e and 171–174.** Routing must lower to
> typed machine wiring without identifying a part, instrument declaration, or prepared instance.

## Task

Repair the known shared-note-stream defect. Preserve `PartId` through audio preparation, instantiate an assigned
instrument per part, route each gesture lane only to that instance, and make sends/routes originate in the part's
instrument output. Two parts selecting the same instrument declaration remain two independently voiced instances.

## Read

- Roadmap §§6.5 narrow bridge; `docs/rules/language/08-performance-and-sound.md` routing-isolation and identity laws.
- Repaired prompts 177–178 and note 79: routing consumes checked source-declared gesture/instrument projections and must
  not introduce a Rust routing language.
- `PerformanceLane`, `lower_studio`/`distinct_patches`, event-window delivery in `plan.rs`, source-output/send lowering,
  default studio, offline renderer, engine prepared machine, and prompt 93's shared-stream ledger entry.

## Design

Carry three different identities: stable part, selected instrument declaration, and prepared instrument instance. Do not
encode one as another or infer routing by display name. Lane-scoped scheduled events bind to one instance during
preparation and become compact private indices only inside the prepared machine. Note-on/off identity is instance-safe;
voice stealing in one part cannot end another part's voice.

Part/instrument/route declarations remain source data. Rust receives only the exact checked binding projection needed to
allocate and connect instances; it owns prepared identities and compact indices, not source routing policy or a public
independently constructible route schema.

The default instrument is instantiated once per otherwise-unassigned part. A route/send whose source is a part uses that
part instance's output; a bus source remains a bus. If two parts share one instrument declaration, their sends may
differ and their audio is not deduplicated. Missing, duplicate, and cyclic bindings are compile-time diagnostics with
source spans.

Test the routing matrix, not only a warning string: isolated notes, simultaneous equal pitches, shared declaration,
different declarations, sends, default/explicit mixes, note stealing, and reinstall/seek. Remove the warning and its
expected-change ledger entry only when the positive isolation law passes offline and live-plan tests.

## Target

- Part-aware prepared event routing in `musa-dsp` and the minimal compiler/project facts it consumes.
- Removal of distinct-patch/global-event-stream lowering and its warning.
- Per-part output routing/send behavior and instrument-instance-safe voice identity.
- Differential audio fixtures proving no cross-talk and unchanged single-part/default output.

## Check

```sh
cargo nextest run -p musa-compiler -p musa-dsp -p musa-playback -p musa-project
cargo clippy --all-targets -p musa-compiler -p musa-dsp -p musa-playback -p musa-project -- -D warnings
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
cargo insta test --workspace --unreferenced=reject
cargo run -p musa -- render tests/fixtures/audio-bridge.musa --to wav -o /tmp/musa-routing.wav
```

Commit as `Route each part to its own instrument`.

## Stop

- No score-driven controls (prompt 180), samples, multitimbral plug-ins, or cross-part voice sharing.
- Do not place `PartId` on every public DSP node; resolve it at the preparation boundary and keep compact indices
  private.
- No mixer-track identity masquerading as part identity.
