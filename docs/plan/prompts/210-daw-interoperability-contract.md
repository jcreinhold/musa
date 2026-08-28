---
id: 210
slug: daw-interoperability-contract
status: done
depends_on: [209]
phase: 4
---

# Make the DAW Boundary Explicit Before Crossing It

## Task

Amend the governing specifications and roadmap to define how Musa interoperates with DAWs without becoming one. Fix the
identity, authority, timing, loss, and real-time contracts for portable export bundles, live CoreMIDI performance, and
macOS Audio Unit extensions before any of those transports are implemented.

## Read

- `docs/rules/README.md`'s amendment procedure; constitution §§1, 4, 5, 7, and 9; obligations §§1, 4, 6, and 9;
  `docs/rules/across-stages/01-stage-judgments.md`, `03-machine-calculus.md`, and `04-identity-and-realization.md`.
- `docs/rules/language/00-semantics.md`, `08-performance-and-sound.md`, and `09-assets-and-packages.md`;
  `docs/rules/desktop/09-sound-and-mix.md`; roadmap §§2, 12.4–12.5, 13.2, 18 Phase 4, and 19.
- Prompts 28, 32, 173, 179–180, 182–188, 191–193, and their current code-map entries. Inspect the actual MIDI, MusicXML,
  WAV, routing, asset-closure, project-export, and prepared-audio facades rather than restating an old plan.
- Apple's current primary documentation for
  [Logic MIDI import](https://support.apple.com/en-ie/guide/logicpro/lgcpdf6a3851/mac),
  [Logic MusicXML import](https://support.apple.com/en-ae/guide/logicpro/lgcp67fa6594/mac),
  [GarageBand audio/MIDI import](https://support.apple.com/guide/garageband/import-audio-and-midi-files-gbndd01649ed/mac),
  [Logic Audio Units](https://support.apple.com/en-ie/guide/logicpro/lgcp22a0dab0/mac),
  [GarageBand Audio Units](https://support.apple.com/guide/garageband/use-audio-units-plug-ins-gbnde06a4e4d/mac),
  [Logic synchronization](https://support.apple.com/guide/logicpro/general-synchronization-settings-lgcp7c04a41a/mac),
  and [`AUAudioUnit`](https://developer.apple.com/documentation/audiotoolbox/auaudiounit).

## Design

The `.musa` project and its exact locked source/asset closure remain canonical. A DAW bundle, MIDI stream, Audio Unit
parameter tree, host automation lane, and Logic/GarageBand document are downstream presentations. They may carry exact
source identity, origin links, and explicit loss records; none may silently edit source or become an alternative
semantic cache. Musa neither generates nor rewrites proprietary `.logicx` or `.band` documents.

Define three different boundaries rather than one "DAW integration":

1. a deterministic directory export containing open interchange files, aligned audio, a versioned manifest, and an
   origin/loss sidecar;
2. a live CoreMIDI projection whose OS timestamps and clock protocol are performance-edge facts, not musical time; and
3. Audio Unit **production**: Logic and GarageBand host Musa. Musa does not host third-party Audio Units, CLAP, or VST.

The Audio Unit split is semantic. A Music Device receives host MIDI and renders one checked source-declared instrument.
A separate Logic MIDI Processor may project a checked Musa piece onto the host timeline. GarageBand gets only the Music
Device unless Apple documents and a host probe prove a MIDI-processor surface. A component's native parameter tree is a
projection of source-declared exposed controls; DSP nodes and private parameters never become plug-in parameters.

Fix ownership of time. Offline files state their frame-zero alignment, common length, sample format, rounding policy,
and tail policy. Live MIDI has one declared clock authority (`musa` or `external`) and reports what MIDI clock/MTC
cannot represent. An Audio Unit consumes host sample time, scheduled MIDI/parameter events, and the host's
musical/transport context; it never treats host block boundaries as meaning. Every callback inherits the no allocation,
lock, I/O, log, compile, decode, or large destruction rules.

Specify a versioned DAW derivation record containing complete project/source/lock identities, realization seed and
profile, render format and policy, stable part/control identities, artifact digests, compatibility target, provenance
links, and every representational loss. Polytempo, polymeter, microtonal pitch, per-note expression, unsupported marks,
and nonlinear routing are refused or recorded—never flattened silently.

Record the rejected alternatives and migration argument in a new research note. Update the code map and roadmap so
"Audio Unit bridge" means the producer boundary above and the earlier third-party-hosting rejection stays intact. No
music-theory amendment is needed: this prompt changes presentation and host boundaries, not Musa's musical ontology.

## Target

- A governing DAW interoperability section in the appropriate across-stage and language specifications, with adjacent
  desktop wording updated only where export/status UI is owned.
- A research note satisfying every item in `docs/rules/README.md`'s amendment procedure.
- Roadmap, code-map, rules indexes, and book navigation reconciled with the new boundary and the 211–219 execution cone.
- A normative compatibility/loss table for Logic Pro and GarageBand, distinguishing documented support from measured
  host behavior and from deliberate Musa exclusions.

## Check

```sh
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
make docs-check
python3 scripts/renumber-prompts.py audit
```

Commit as `Define the DAW interoperability boundary`.

## Stop

- No Rust, Swift, Xcode-project, CLI, desktop, MIDI, or audio implementation.
- No Audio Unit/CLAP/VST hosting, proprietary DAW-document writer, audio recording, waveform editing, or DAW round trip.
- No source keyword, data family, control catalogue, or host-side semantic default.
