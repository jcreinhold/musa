# `apps/musa-audio-unit`

The macOS AUv3 Music Device `aumu musa Musa`, its containing app, and the automated host that measures both.

A workstation loads this component, points it at a `.musa` project, and plays one part of one piece from its own MIDI.
Everything musical has already been decided by the time the component sees it: `musa-project` compiles the source,
verifies the asset closure, and prepares the audio graph for the host's exact rate, and `crates/musa-au` makes that one
preparation reachable through a C ABI. What lives here is Apple plumbing and nothing else.

## What is in it

| Path | What it is |
| --- | --- |
| `Sources/Kit` | `MusaAudioUnitKit.framework` — the `AUAudioUnit` subclass, the control worker, MIDI decoding, and the atomic slot a prepared instrument is published through. The extension, the app, and the harness all link this one framework, so the class a host loads out of process is the class the harness measures in process |
| `Sources/Instrument` | the app-extension bundle. One class, which names the audio unit |
| `Sources/App` | the containing app: it opens a project, prepares a part, and prints what the extension would be handed |
| `Sources/Harness` | `musa-au-harness`, an automated host that writes its findings as JSON |
| `Sources/Tests` | the XCTest bundle over the framework's own Swift: four-character codes, MIDI 1.0 decoding, and the publication slot |
| `Config` | the two `Info.plist`s and the two entitlement files |

The allocation probe the harness uses is `apps/musa-audio-unit-trial/Sources/AllocProbe/AllocProbe.c`, compiled by this
project rather than copied into it. One interposer that both harnesses agree on is what makes their numbers comparable;
two copies would drift.

## Building and checking it

```sh
scripts/check-audio-unit.sh instrument
```

That builds the component, signs it ad hoc, registers it for this user with `pluginkit`, waits for the Audio Unit
registry to settle, runs Apple's `auval`, runs the harness with the allocation probe inserted, runs a second build under
Thread Sanitizer, asserts every finding with `scripts/check-audio-unit.py`, and unregisters the component again. It
needs a full Xcode, and it needs neither a Developer Team nor an App Group.

The framework links `libmusa_au.a`, which a build phase produces with `cargo build -p musa-au`. Set `MUSA_AU_SKIP_CARGO`
to skip that when a caller has already built it.

## What it does not do

No whole-piece sequencer, no MIDI Processor, no parameter tree, and no multi-output routing — prompts 217 and 218 own
those. The parameter tree is empty on purpose: Rule D2 of `docs/rules/across-stages/06-daw-boundary.md` says a plug-in
parameter is exactly a source-declared exposed control, and inventing one here to make the tree non-empty would break
that rule to make a screenshot look finished.

The component never writes `.musa` source. Rule D1: the source is canonical, and a presentation that wrote back to it
would be a second editor.

## Where the assets come from

From what the host restored, never from a container the component goes looking for. Prompt 215 measured a sandboxed
ad-hoc-signed extension being taken down by the system the moment it read a file inside its App Group container, and a
real App Group identifier is team-prefixed and provisioned by Apple. So `Config/Instrument.entitlements` deliberately
carries no App Group, and access arrives with the saved document as a security-scoped bookmark the containing app
granted. `docs/notes/research/93-the-audio-unit-shape.md` §4 records the measurement.
