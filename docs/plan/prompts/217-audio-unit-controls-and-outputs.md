---
id: 217
slug: audio-unit-controls-and-outputs
status: pending
depends_on: [216]
phase: 4
---

# Project Source Controls and Outputs into the Host

## Task

Expose a loaded instrument's source-declared public controls as stable AU parameters and its declared channels as host
output buses. Preserve dependent control kinds, source identity, automation timing, and the distinction between a part,
an instrument instance, and a mixer track.

## Read

- Prompt 215's trial findings and prompt 216's production boundary; prompts 176b–180 and 189; governing performance
  controls in `docs/rules/language/08-performance-and-sound.md` and Mix ownership in
  `docs/rules/desktop/09-sound-and-mix.md`.
- Apple's current `AUParameterTree`, parameter address/ramp/event, bus array, channel layout, full-state, and host
  notification documentation. Re-run the host behavior probes that prompt 215 found necessary.

## Design

Generate the AU parameter tree from the selected checked `Instrument` signature. A parameter is admitted only when its
source `ControlKey(K)`, value domain, update rate, default, unit/display policy, and mapping are representable under the
trial's AU contract. Unsupported dependent kinds remain available in Musa and produce a precise projection loss; they do
not become floats by coercion.

Assign each admitted control a stable 64-bit `AUParameterAddress` through a versioned, collision-detecting table keyed
by the control's canonical source identity. Hashing alone is not uniqueness. Unchanged keys retain addresses across
source formatting, display-name changes, parameter-tree rebuilds, extension restarts, and compatible instrument
revisions. Removed addresses are not silently reused inside restored document state. Persist the complete table and
validate it when restoring automation.

Host parameter events are an ephemeral performance overlay. They are sample-offset events passed to the existing
source-defined control mapping and smoothing semantics; they never rewrite `.musa`, change the instrument declaration,
or become canonical. Parameter ramps and point changes must agree with native control histories under every host block
partition. The UI reads the same generated descriptors and may reveal the declaration in Musa desktop; it owns no
defaults or mappings.

Project only source-declared output channels and named instrument outputs that prompt 215 proved hosts can negotiate.
Logic may receive multiple output buses; GarageBand gets a documented stereo fallback when it does not expose the same
routing surface. Bus order and identity are stable, format negotiation is explicit, and no source bus is renamed to a
DAW "track" in the semantic layer.

## Target

- Generated AU parameter descriptors/address table, host-event bridge, state migration, and differential control laws.
- Output-bus projection with stable identity, Logic multi-output behavior where supported, and GarageBand stereo
  fallback with no hidden mix changes.
- Collision, removal/addition/reorder/rename, range/unit, incompatible-kind, automation-ramp, block-partition, state
  restoration, and host bus-renegotiation tests.
- Accessible native parameter/instrument UI generated from project facts; no hand-maintained Swift control catalogue.

## Check

```sh
cargo nextest run -p musa-dsp -p musa-project -p musa-au
cargo clippy --all-targets -p musa-dsp -p musa-project -p musa-au -- -D warnings
xcodebuild -project apps/musa-audio-unit/MusaAudioUnit.xcodeproj -scheme MusaAudioUnit -configuration Debug CODE_SIGNING_ALLOWED=NO test
scripts/check-audio-unit.sh parameters-and-outputs
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
cargo insta test --workspace --unreferenced=reject
```

Commit as `Expose Musa controls and outputs to Audio Unit hosts`.

## Stop

- No host automation written back to source, host-side control inference, MIDI Processor, or whole-piece sequencing.
- No private DSP parameter/node exposed merely because AU supports a float parameter.
- No promise that GarageBand exposes Logic's multi-output or automation workflow without measured evidence.
