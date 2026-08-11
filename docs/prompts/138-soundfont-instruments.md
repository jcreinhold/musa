---
id: 138
slug: soundfont-instruments
status: pending
depends_on: [134, 136]
phase: 4
---

# SoundFont Banks Are Another Instrument Adapter

> **Contingent on prompt 125.** The core-boundary decision may repair this prompt's Design, fold it into
> another, or replace it. Read `docs/core-boundary.md` first.

## Task

Import SoundFont 2.04 banks and presets into the same native sample-map/instrument contract used by SFZ. Support the
large existing `.sf2` ecosystem without identifying a binary bank, General MIDI program number, Musa instrument, or
score part with one another.

## Read

- `docs/language/09-assets-and-packages.md`; prompts 134 and 132; prompt 137's adapter boundary.
- Creative/E-mu SoundFont 2.04 Technical Specification (preserved at
  `https://musescore.org/sites/musescore.org/files/2023-01/sfspec24.pdf`) in full: RIFF structure, presets,
  instruments/zones, generators, modulators, sample headers, stereo links, loops, and 24-bit extension.
- Current maintained Rust/C implementations only to evaluate reuse, security, license, and conformance. If adopted, keep
  all third-party types behind the adapter and record unsupported specification behavior explicitly.

## Design

Parse and validate RIFF bounds before allocation. Select a preset by explicit bank/program or unambiguous name; no
implicit General MIDI meaning. Flatten preset/instrument global/local zones with the specification's generator
combination rules into the native region model. Preserve root/tuning, key/velocity ranges, attenuation/pan, loops,
envelopes, exclusive classes, stereo sample links, and supported modulators.

Publish a generator/modulator support matrix. Unsupported sound-changing behavior prevents that preset from claiming
full support and yields a diagnostic; do not silently play a materially different instrument. A deliberate restricted
mode may accept a documented subset only when the user opts in and the loss report is attached to the instrument facts.

Embedded samples enter prompt 134's verified prepared-asset path under the bank digest. Decode/normalize off-thread;
bound chunks, zones, samples, names, and memory. The imported instrument exposes Musa standard controls and documented
custom controls; raw MIDI controller numbers and generator ids remain adapter-private.

## Target

- SoundFont 2.04 parser/adapter or a justified private dependency plus defensive validation.
- Small redistributable banks/fixtures for presets, zones, velocity layers, loops, stereo links, exclusive class,
  modulators, malformed RIFF, and unsupported behavior.
- Preset selection syntax/facts, hover/support summary, deterministic rendering, and local/package asset integration.
- Equality tests showing SFZ, SoundFont, and native maps with the same normalized regions produce the same prepared map.

## Check

```sh
cargo nextest run -p musa-audio -p musa-project -p musa -p musa-lsp
cargo clippy --all-targets -p musa-audio -p musa-project -p musa -p musa-lsp -- -D warnings
cargo fmt --check
cargo deny check
cargo insta test --workspace --unreferenced=reject
```

Commit as `Import SoundFont instruments through the sampler`.

## Stop

- No General MIDI orchestration policy, MIDI-file player, bank editor, SF3 extension, or proprietary sampler format.
- No binary SoundFont data in source, kernel facts, or desktop IPC.
- No claim of complete support while a sound-changing generator/modulator is ignored.
