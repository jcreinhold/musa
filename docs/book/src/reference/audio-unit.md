# The Audio Units

Musa publishes two AUv3 components on macOS. They are what a workstation loads; the files a workstation opens are the
[DAW bundle](daw-bundle.md), and the two are separate ways in. The task-shaped page is
[Use the Musa Audio Units in a host](../how-to/audio-unit.md).

|  | Music Device | MIDI Processor |
| --- | --- | --- |
| Type, subtype, manufacturer | `aumu musa Musa` | `aumi musp Musa` |
| Name in a host | Musa: Instrument | Musa: Processor |
| What it does | plays one part of one piece from the host's MIDI | puts a whole checked piece on the host's timeline as MIDI |
| What it emits | audio, on the buses the source declares | MIDI, and nothing else |
| Where it goes | an instrument slot | a MIDI FX slot, above an instrument |

Both live in one app extension, `MusaComponents.appex`, inside the containing app. That is not a packaging preference: a
containing app that ships two `AudioComponents`-declaring extensions gets only one of them registered, and registering
the second removes the first. One extension declaring two components publishes both. The two audio units share no
mutable state and meet only in the factory class that dispatches on the type the host opened.

Nothing musical is decided in either component. The source is compiled, the asset closure verified, the audio graph
prepared for the host's exact sample rate, and the piece's MIDI indexed by position, all before either one is handed
anything.

## What is decided in the host, and what is not

| Decided in the host | Decided in the source |
| --- | --- |
| which project, piece, and part | every note, duration, dynamic, and articulation |
| the reading and the timeline (Processor) | which controls exist and what they mean |
| parameter values and their automation | the range, default, and unit of each control |
| the sample rate, block size, and transport | which outputs exist and what they are called |

Neither component ever writes `.musa` source. A host's parameter change is an ephemeral performance overlay — a
sample-offset event carrying a normalized value, reaching the instrument through the mapping the source already
declared, and agreeing with the same change made over MIDI. It is not written back.

## Parameters

The parameter tree is generated from the loaded instrument's own declaration and from nothing else. There is no
catalogue of Musa controls in the plug-in: a source that declares one more control publishes one more parameter, with no
Swift edited.

- A control becomes a parameter when its declared kind carries a value domain a host float can hold. Its domain,
  default, and whether it ramps all come from the declaration.
- A control that cannot become one is a **named loss**, never a coerced number. The losses are saved in the document and
  readable from the host's plug-in window; each is a sentence saying which control and why.
- A parameter's **address** comes from a versioned, collision-detecting table keyed by the control's canonical source
  identity — not from a hash alone. The table is saved in the document, restored with it, and handed to the next
  preparation, which is what makes a host's automation lane still point at the control it was written for after a
  rename, a reorder, a reformat, or an extension restart.
- A table this component cannot read **refuses the preparation** rather than starting over. Silently re-deriving
  addresses would move automation without saying so.
- A parameter event for an unknown address is ignored, not guessed at.

## Output buses

Bus zero is the piece's main output; then the loaded part's own output; then any studio bus it sends to — each under the
name the source gave it, not a workstation's word for a track.

A host that takes only bus zero gets exactly the bus-zero frames a host that takes all of them gets. The one-output
fallback is the same main output, not a different mix. How a particular workstation *presents* those buses is not
something Musa measures or claims.

## The MIDI Processor's two choices

Both are saved in the document and neither is ever chosen implicitly.

| Key | Values | What it means |
| --- | --- | --- |
| reading | `score` | the piece as written — notated durations, no interpretation of dynamics |
|  | `performance` | the piece as played — the performance a checked source describes |
| timeline | `piece` | positions are seconds on the piece's own exact schedule; the host's tempo is not consulted |
|  | `host` | positions are quarter notes on the host's musical timeline, so its tempo track moves the piece |

A polytempo piece has no single quarter-note grid, so on the host's timeline it is refused by name rather than
flattened; on its own timeline it plays with every part at its own speed. A host that supplies no tempo gets no guess. A
stopped transport is silent, and a seek starts where the host is rather than replaying what it passed.

## The saved document

The component merges a property-list-safe dictionary onto the host's own full state. It carries identities and never
asset bytes.

| Key | What it holds |
| --- | --- |
| `musa.state.version` | the schema version of this dictionary. Currently **3** |
| `musa.state.abiVersion` | the ABI version the component that wrote it was built against |
| `musa.state.project` | where the source lives, as the containing app granted it |
| `musa.state.access` | a security-scoped bookmark to that location |
| `musa.state.piece` | which piece of that project |
| `musa.state.part` | which part of that piece (Instrument) |
| `musa.state.musicIdentity` | the compiled music's semantic identity |
| `musa.state.assetIdentity` | the verified asset closure's identity |
| `musa.state.inputs` | the MIDI dimensions the instrument's source binds |
| `musa.state.controlTable` | the address table the parameters were published under |
| `musa.state.controlLosses` | the declared controls that could not become parameters |
| `musa.state.outputs` | the outputs projected, in bus order |
| `musa.state.scheduleMode` | `score` or `performance` (Processor) |
| `musa.state.scheduleTimeline` | `piece` or `host` (Processor) |
| `musa.state.parts` | the parts the projection carries, each as a name and its channel (Processor) |
| `musa.state.refusal` | why the component is silent, when it is |

Version 1 named a source and a part and carried no automation addresses. Version 2 added the control table, the
projection losses, and the output names. Version 3 added the Processor's reading and timeline. Both components write
into one schema rather than two disjoint ones, so a host that hands back the wrong dictionary finds a key it does not
understand instead of a key that means something else.

A document from a **future** version is refused rather than partially read, and so is a word in a saved field this
version does not know. Nonsense in the dictionary leaves the component silent and saying why, not crashing.

## Where the assets come from

From what the host restored, never from a container the component goes looking for. A sandboxed, ad-hoc-signed extension
that reads inside its App Group container is taken down by the system, and a real App Group identifier is team-prefixed
and provisioned by Apple. So the extension's entitlements carry no App Group, and access arrives with the saved document
as a security-scoped bookmark the containing app granted.

## Hosts

`auval` validates both components, and an automated host in this repository instantiates them out of process, renders,
saves, restores, and measures. That harness is the contract.

Beyond it: the Instrument is an ordinary AUv3 instrument and behaves as one wherever AUv3 instruments load. The
**Processor is supported in Logic Pro** — a MIDI FX slot on a software-instrument track. **GarageBand is not supported
for the Processor**: nothing here observed GarageBand's MIDI FX surface, and a claim made from anything other than
observation would be a claim this book does not make.

**Do not run the Processor and import the same piece's MIDI at once.** They are two projections of one piece and would
duplicate every message. Exporting is how a piece is handed to a host permanently; the Processor is how it is played
live, and the choice is one or the other.
