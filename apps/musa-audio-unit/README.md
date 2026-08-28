# `apps/musa-audio-unit`

The two macOS AUv3 components — the Music Device `aumu musa Musa` and the MIDI Processor `aumi musp Musa` — their
containing app, and the automated host that measures them.

A workstation loads the instrument, points it at a `.musa` project, and plays one part of one piece from its own MIDI.
It loads the processor to have a whole checked piece *scheduled* onto its timeline as MIDI, which it can then route to
the instrument or to anything else. Everything musical has already been decided by the time either component sees it:
`musa-project` compiles the source, verifies the asset closure, prepares the audio graph for the host's exact rate and
indexes the piece's MIDI by position, and `crates/musa-au` makes those reachable through a C ABI. What lives here is
Apple plumbing and nothing else.

## What is in it

| Path | What it is |
| --- | --- |
| `Sources/Kit` | `MusaAudioUnitKit.framework` — the `AUAudioUnit` subclass, the control worker, MIDI decoding, and the atomic slot a prepared instrument is published through. The extension, the app, and the harness all link this one framework, so the class a host loads out of process is the class the harness measures in process |
| `Sources/Extension` | the app-extension bundle. One class, which names whichever audio unit the host opened |
| `Sources/App` | the containing app: it opens a project, prepares a part, and prints what the extension would be handed |
| `Sources/Harness` | `musa-au-harness`, an automated host that writes its findings as JSON |
| `Sources/Tests` | the XCTest bundle over the framework's own Swift: four-character codes, MIDI 1.0 decoding, parameter descriptors, and the publication slot |
| `Config` | the two `Info.plist`s and the two entitlement files. The extension's declares both components |

The allocation probe the harness uses is `apps/musa-audio-unit-trial/Sources/AllocProbe/AllocProbe.c`, compiled by this
project rather than copied into it. One interposer that both harnesses agree on is what makes their numbers comparable;
two copies would drift.

## Building and checking it

```sh
scripts/check-audio-unit.sh midi-processor
```

The argument names which findings the run is required to have made: `instrument` is the boundary prompt 216 built,
`parameters-and-outputs` adds prompt 217's source controls and output buses, and `midi-processor` adds prompt 218's
schedule. The harness measures everything either way.

That builds both components, signs it ad hoc, registers it for this user with `pluginkit`, waits for the Audio Unit
registry to settle, runs Apple's `auval`, runs the harness with the allocation probe inserted, runs a second build under
Thread Sanitizer, asserts every finding with `scripts/check-audio-unit.py`, and unregisters the extension again. It
needs a full Xcode, and it needs neither a Developer Team nor an App Group.

The framework links `libmusa_au.a`, which a build phase produces with `cargo build -p musa-au`. Set `MUSA_AU_SKIP_CARGO`
to skip that when a caller has already built it.

## Parameters and outputs

The parameter tree is generated from the loaded instrument's own declaration and from nothing else. Rule D2 of
`docs/rules/across-stages/06-daw-boundary.md` says a plug-in parameter is exactly a source-declared exposed control, so
`musa-project` admits a control only when its declared kind carries a value domain a host float can hold, and every
control it cannot admit becomes a named loss rather than a coerced number. There is no catalogue of Musa controls in
Swift: `stdlib/src/performance/mod.musa`'s `note_instrument` yields five parameters and two losses, and a source that
declares a sixth control gets a sixth parameter without anything here being edited.

Each parameter's address comes from a versioned, collision-detecting table keyed by the control's canonical source
identity — not from a hash alone. The table is saved in the document, restored with it, and handed back to the next
preparation, which is what makes a host's automation lane still point at the control it was written for after a rename,
a reorder, a reformat, or an extension restart. A table this component cannot read refuses the preparation instead of
starting over, because silently re-deriving addresses would move automation without saying so.

A host's parameter change is an ephemeral performance overlay: it is a sample-offset event carrying a normalized value,
it reaches the instrument through the mapping the source already declared, and it agrees with the same change made over
MIDI. Nothing about it is written back to `.musa`.

The output buses are the declared points the loaded part actually reaches — bus zero is the piece's main output, then
the part's own output, then any studio bus it sends to — under the names the source gave them. A host that takes one
stereo output gets exactly the bus-zero frames a host that takes all of them gets; the fallback is not a different mix.
What no measurement here establishes is how any particular workstation *presents* those buses, so nothing claims that
GarageBand exposes Logic's routing surface.

## Scheduling a piece: the MIDI Processor

`aumi musp Musa` puts a whole checked piece on the host's timeline as MIDI. It emits MIDI and nothing else: it
instantiates no instrument, renders no audio, and reaches into no project document of the host's.

**In Logic Pro.** Make a software-instrument track, and on that track's channel strip open the MIDI FX slot → *Audio
Units* → *Musa* → *Musa: Processor*. Point it at a `.musa` project and choose the piece, the reading, and the timeline
in the plug-in's own window; the instrument below it in the strip can be `Musa: Instrument` or any other. The
processor's messages then reach that instrument exactly as a region's would. Transport, tempo, cycle, and position stay
the host's — this component follows them and owns none of them.

**Two readings and two timelines**, both saved in the document and never chosen implicitly:

| Choice | What it means |
| --- | --- |
| Reading: *score* | the piece as written — notated durations, no interpretation of dynamics |
| Reading: *performance* | the piece as played — the performance a checked source describes |
| Timeline: *piece* | positions are seconds on the piece's own exact schedule; the host's tempo is not consulted |
| Timeline: *host* | positions are quarter notes on the host's musical timeline, so its tempo track moves the piece |

A polytempo piece has no single quarter-note grid, so on the host's timeline it is *refused by name* rather than
flattened; on its own timeline it plays with every part at its own speed. A word in a saved document that this version
does not know is refused too — never defaulted.

**Do not run the processor and import the same piece as MIDI at once.** They are two projections of one piece and would
duplicate every message. Exporting MIDI is the way to hand the piece to a host permanently; the processor is the way to
have it played live, and the choice is one or the other.

**GarageBand is not supported for this.** Not "measured and found wanting": nothing here observed GarageBand at all —
the evidence in `docs/notes/research/93-the-audio-unit-shape.md` is `auval` and this harness — and no claim about that
application's MIDI FX surface is made from anything other than its own current documentation. Until that reading is
done, treat Logic Pro as the supported host, and treat the instrument's own GarageBand behaviour as what §"Parameters
and outputs" above says it is and no more.

## What it does not do

Neither component ever writes `.musa` source. Rule D1: the source is canonical, and a presentation that wrote back to it
would be a second editor.

## Where the assets come from

From what the host restored, never from a container the component goes looking for. Prompt 215 measured a sandboxed
ad-hoc-signed extension being taken down by the system the moment it read a file inside its App Group container, and a
real App Group identifier is team-prefixed and provisioned by Apple. So `Config/Extension.entitlements` deliberately
carries no App Group, and access arrives with the saved document as a security-scoped bookmark the containing app
granted. `docs/notes/research/93-the-audio-unit-shape.md` §4 records the measurement.

## Why one extension carries both components

Because two do not work. A containing app that ships two `AudioComponents`-declaring app extensions registers only one
of them on this system, and registering the second takes the first out of the registry;
`docs/notes/toolchain/two-audio-units-one-container.md` has the measurement and the table. So
`Config/Extension-Info.plist` declares both components and `MusaComponentFactory` dispatches on the type the host
opened. One bundle is still not one component: the two audio units share no mutable state and meet only in that factory.
