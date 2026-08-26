---
id: 186
slug: soundfont-instruments
status: done
depends_on: [174, 182, 184]
phase: 4
---

# SoundFont Banks Are Another Instrument Adapter

> **Governed by the event-track and machine core installed by prompts 127a–127e and 171–174.** SoundFont adapts into the
> sampler machine; it does not add a core form.

## Task

Import SoundFont 2.04 banks and presets into the same native sample-map/instrument contract used by SFZ. Support the
large existing `.sf2` ecosystem without identifying a binary bank, General MIDI program number, Musa instrument, or
score part with one another.

## Read

- `docs/rules/language/09-assets-and-packages.md`; prompts 182 and 180; prompt 185's adapter boundary.
- `docs/notes/research/language-design-closure/85-soundfont-is-a-hierarchical-adapter.md` for the maintained-player
  disagreements and the reason no parser library is the semantic boundary.
- Creative/E-mu SoundFont 2.04 Technical Specification (preserved at
  `https://musescore.org/sites/musescore.org/files/2023-01/sfspec24.pdf`) in full: RIFF structure, presets,
  instruments/zones, generators, modulators, sample headers, stereo links, loops, and 24-bit extension.
- Current maintained Rust/C implementations only to evaluate reuse, security, license, and conformance. If adopted, keep
  all third-party types behind the adapter and record unsupported specification behavior explicitly.

## Design

Parse and validate RIFF bounds before allocation. Select a preset through §3.2's canonical asset fragment by explicit
bank/program or unambiguous name; no implicit General MIDI meaning. Flatten preset/instrument global/local zones with
the specification's generator combination rules into the native region model. Preserve root/tuning, key/velocity ranges,
attenuation/pan, loops, six-stage envelopes, the initial resonant low-pass, exclusive classes, stereo sample links, and
the typed note-on modulators admitted by §3.2.

Publish a generator/modulator support matrix. Unsupported sound-changing behavior prevents that preset from claiming
full support and yields a diagnostic; do not silently play a materially different instrument. A deliberate restricted
mode may accept a documented subset only when the user opts in and the loss report is attached to the instrument facts.

Embedded samples enter prompt 182's verified prepared-asset path under the bank digest. Decode/normalize off-thread;
bound chunks, zones, samples, names, and memory. The imported instrument exposes Musa standard controls and documented
custom controls; raw MIDI controller numbers and generator ids remain adapter-private.

Parsing foreign RIFF bytes and enforcing bounds is host work. The semantic adapter result is the same source-declared
sample-map/instrument contract used by native and SFZ maps, with exact equality before private normalization. No
SoundFont-specific Rust public instrument schema becomes authoritative.

Extend `std::sound::sample` first for the general constructs the accepted subset needs: delay/hold envelope phases,
exact seconds-or-timecents envelope durations, the SoundFont envelope curve, a resonant low-pass description, and typed
note-key/note-expression modulation. Irrational timecent conversion happens only at the DSP preparation edge; the
adapter must not decimalize it into a purportedly exact rational. Rust owns only the checked read-only projection and
real-time state. Do not introduce a generator-number table as a second public language or a SoundFont-only runtime
object.

## Target

- SoundFont 2.04 parser/adapter or a justified private dependency plus defensive validation.
- Small redistributable banks/fixtures for presets, zones, velocity layers, loops, stereo links, exclusive class,
  modulators, malformed RIFF, and unsupported behavior.
- Preset selection syntax/facts, hover/support summary, deterministic rendering, and local/package asset integration.
- Equality tests showing SFZ, SoundFont, and native maps with the same normalized regions produce the same prepared map.

## Check

```sh
cargo nextest run -p musa-dsp -p musa-project -p musa -p musa-lsp
cargo clippy --all-targets -p musa-dsp -p musa-project -p musa -p musa-lsp -- -D warnings
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
cargo deny check
cargo insta test --workspace --unreferenced=reject
```

Commit as `Import SoundFont instruments through the sampler`.

## Stop

- No General MIDI orchestration policy, MIDI-file player, bank editor, SF3 extension, or proprietary sampler format.
- No binary SoundFont data in source, event track facts, or desktop IPC.
- No claim of complete support while a sound-changing generator/modulator is ignored.
