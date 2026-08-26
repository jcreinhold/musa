# SoundFont is a hierarchical adapter

**Status:** implementation evidence for prompt 186. This note governs nothing.

## Sources read

- The [SoundFont 2.04 Technical Specification](https://musescore.org/sites/musescore.org/files/2023-01/sfspec24.pdf) was
  read in full. Its 2.04 change is the `sm24` low-byte pool; the retained 2.01 text owns RIFF/Hydra structure, generator
  precedence, default and explicit modulators, the synthesis model, and error handling.
- [FluidSynth 2.6.0](https://github.com/FluidSynth/fluidsynth/releases/tag/v2.6.0), released 2026-08-11, and its
  [SoundFont implementation notes](https://www.fluidsynth.org/wiki/SoundFont/) are the maintained C comparison.
  FluidSynth disables the specification's velocity-to-filter default, does not implement linked modulators, does not
  give both stereo channels the right channel's pitch generators, and deliberately accepts loops the specification says
  may be refused. Those are useful differential observations, not semantics for Musa to inherit.
- [RustySynth 1.3.6](https://github.com/sinshu/rustysynth/tree/ccfa1a7a34bbc13fd92f6d7f32db08a9be2418b6), inspected at
  its 2026-05-17 head, is the maintained pure-Rust comparison. Its loader discards `pmod` and `imod`, and its public
  model normalizes directly into synthesizer-specific regions. It is therefore neither a complete conformance oracle nor
  a suitable source-owned boundary.

## The prompt's missing distinction

A SoundFont preset is not merely a bag of PCM ranges. The preset and instrument global/local zones combine by two
different rules: instrument value generators replace defaults, preset value generators add, range generators intersect,
and explicit modulators replace or add according to their complete identity. The implicit velocity-to-attenuation and
velocity-to-filter routes are part of the format even when neither modulator chunk mentions them.

Consequently, a Rust parser that returns a private `Sf2Region` cannot be the semantic result. The adapter must first
emit ordinary `std::sound::sample` values for the behavior it accepts: six-stage envelopes, a resonant low-pass
description, typed note-on modulation, stereo layering, and embedded sample identities. The DSP crate may then project
and normalize that checked value. This is the same ownership rule that repaired SFZ, applied one layer deeper.

## Boundary chosen for `sf2@1`

`sf2@1` is strict rather than approximate. It supports the common note-on subset fixed in the governing table: exact
RIFF/Hydra validation, 16/24-bit samples, preset/instrument zone combination, address/range/tuning/pan/attenuation,
volume envelope, initial filter, loops, stereo links, exclusive classes, and note-key/note-expression modulators over
the supported destinations. The implicit note-expression routes are installed. A modulator that depends on an ambient
MIDI channel controller, pressure, pitch wheel, a secondary controller, or a link is an error for that preset. The same
is true of LFOs, the modulation envelope, effects sends, forced key/velocity, and nonstandard scale tuning.

That refusal is narrower than FluidSynth and many hardware players, but it is honest: accepting those rows before the
standard library and runtime can denote them would be materially different playback. A later `sf2@2` may widen the
source contract without changing what `sf2@1` means.

Embedded samples receive private logical identities framed by the verified bank digest and sample-header index. They are
decoded off-thread and handed to the ordinary sampler resolver; neither RIFF offsets nor binary bytes enter Musa source,
event facts, or desktop IPC.

Preset selection remains an adapter option on the existing asset address, not a new language construct:
`bank.sf2#preset=0:40` selects bank 0/program 40, while `#preset-name=Violin` requires one exact unambiguous bank name.
The fragment participates in adapter identity but is removed before resolving the locked bank path.

