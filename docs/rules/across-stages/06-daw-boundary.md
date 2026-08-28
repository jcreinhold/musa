# Crossing the DAW boundary

**Status: governing.** How a Musa project reaches Logic Pro, GarageBand, and other hosts without Musa becoming one of
them.

Bound by `../constitution.md` §§1, 4, 5, 6, 7, and 9 and `../obligations.md` §§1, 6, 9, 12, and 15. It binds every
transport built for a digital audio workstation: portable export bundles, live CoreMIDI, and macOS Audio Unit
components.

A workstation is not a stage of Musa's pipeline. It is a *consumer* that has its own document, its own clock, and its
own idea of what a track is. This page fixes what may cross to it, who owns time on each crossing, and what must be
refused or written down rather than silently flattened. It is written before any of those transports exist so that the
implementations have a contract to be wrong against.

## 1. One canonical record, several downstream presentations

The `.musa` project and its exact locked source and asset closure are canonical (constitution §1,
`../language/09-assets-and-packages.md` §1). Each of the following is a **presentation** made from that record by a
named conversion:

- a portable export bundle and every file in it;
- a live MIDI stream;
- an Audio Unit component's parameter tree and its saved state;
- a host automation lane; and
- a Logic Pro or GarageBand document that a person built from any of those.

**Rule D1 — no back edge.** No presentation may write, patch, or re-derive `.musa` source, and none may serve as an
alternative semantic cache that some later pass reads instead of the source. A presentation carries exact source
identity, origin links, and explicit loss records; that is the whole of its authority.

Musa neither generates nor rewrites proprietary workstation documents. It does not write `.logicx` or `.band`, and it
does not parse them. What a person does inside their workstation is theirs; Musa's claim ends at the open files it
produced and the identity they carry.

This is constitution §6 applied to one consumer: representations stay connected by recorded conversions, and a
conversion may be reversible only where that has been proved for that conversion. No workstation round trip has been
proved, so none is claimed.

## 2. Three boundaries, not one integration

"DAW integration" names three different crossings. They have different identities, different owners of time, and
different failure language, and a fact proved of one says nothing about the others.

| Boundary | What crosses | Who consumes it |
| --- | --- | --- |
| **Export** | a deterministic directory: open interchange files, frame-aligned audio, a versioned manifest, an origin and loss sidecar | a person, importing into a project |
| **Live** | a CoreMIDI projection of a checked score or performance | a running host, in real time |
| **Hosted** | an Audio Unit component that renders or projects checked Musa | a host process, in its own render thread |

Each boundary states its own derivation record (§5), its own losses (§6), and its own clock authority (§4). An
implementation may share code between them; it may not share a claim.

## 3. Audio Unit production, and the split inside it

**Musa is hosted. Musa does not host.** Logic Pro and GarageBand may load a Musa component. Musa loads no third-party
Audio Unit, CLAP, or VST plug-in — the rejection in roadmap §4 stands unchanged, and this page does not reopen it.
Producing a component and hosting other people's are different engineering problems, and only the first is in scope.

The production side splits along a semantic line, not a packaging one:

- A **Music Device** receives host MIDI and renders exactly one checked source-declared instrument. It is the
  instrument, in the host's terms.
- A **MIDI Processor** projects a checked Musa piece onto the host's timeline, so the host schedules it and routes the
  result to a Music Device or to any other instrument. It is the piece, in the host's terms.

These are not two configurations of one component. One consumes performance and produces sound; the other consumes
transport and produces performance. GarageBand receives the Music Device only. It receives a MIDI Processor only if
Apple documents such a surface for it *and* a host probe measures it (prompt 215) — an undocumented surface that happens
to work is not a supported boundary.

**Rule D2 — the parameter tree is a projection.** A component's native parameters are exactly the controls the source
declares as public (`../language/08-performance-and-sound.md` §§3–4, 6). A DSP node, an internal graph edge, a private
primitive configuration, or a host-side convenience knob never becomes a plug-in parameter. Each exposed parameter
carries one stable source identity, and a differential law compares the projected tree against the checked source value,
the way every other caller-oriented projection is held (`../language/08-performance-and-sound.md` §0).

State restoration is a presentation too: a saved component state names the project, source, and lock identities it was
made from, and restoring it against a different closure is refused with those identities rather than approximated.

## 4. Who owns time on each crossing

Written time, performed time, physical seconds, and host sample time are four coordinates. Nothing below converts one
into another because their numbers happen to agree (obligations §1, `01-stage-judgments.md` §6).

**Offline files.** Every artifact states its frame-zero alignment, its common frame count, its sample format and rate,
its rounding policy, and its tail policy. Two artifacts of one export begin at the same project frame and end at the
same frame, including declared release and effect tails. An importer that lines them up at zero must get the alignment
Musa measured.

**Live MIDI.** Exactly one **clock authority** is declared for a session, `musa` or `external`. The authority owns
tempo, position, and start and stop; the other side follows and never silently corrects the leader. Operating-system
timestamps are facts of the performance edge — they are physical time, not musical time, and they never enter a score
value. What the chosen protocol cannot represent is reported before the session starts, not discovered by drift: MIDI
clock and MTC carry no meter, no key, no spelling, and no exact rational position, and a tempo map they cannot express
is a refusal (§6).

**Hosted rendering.** A component consumes host sample time, the host's scheduled MIDI and parameter events, and the
host's musical and transport context. It never treats a host block boundary as meaning: block size is a partition of
frames the host chose, and by Theorem R1-batch (`04-identity-and-realization.md` §6) a different partition of the same
frames is unobservable. A component whose output changed with the host's buffer size would be reporting a bug, not a
musical fact.

## 5. The DAW derivation record

Every artifact that leaves by any of the three boundaries carries a versioned **derivation record**. It is a conversion
record in the sense of `02-derivation-diagrams.md` §2, with the fields a consumer outside Musa needs:

```text
DawDerivation = {
    record version,
    project identity, source identity, lock identity,
    realization seed, performance profile identity,
    render format : { sample rate, channel layout, sample format },
    render policy : { rounding, tail, frame zero, common length },
    parts    : List<{ stable part identity, display name }>,
    controls : List<{ stable control identity, source identity, kind }>,
    artifacts: List<{ relative path, role, exact digest }>,
    compatibility target,
    origins  : List<OriginPath>,
    losses   : List<Loss>,
}
```

Its encoding, equality, and versioning follow `04-identity-and-realization.md` §§1–3: fields are length-prefixed,
rationals are reduced, the equality version changes when the fields or their encoding change, and a digest indexes a
candidate rather than deciding equality (obligations §6). Display names are for readability only and are never the
identity (§3 of that chapter).

Origins keep their intermediate steps (obligations §9). A path from a source note to a frame range in a stem keeps the
occurrence and the gesture that produced it, and both conversions, so a person can walk it back.

## 6. Every loss is refused or recorded

A presentation may lose information. It may not lose it silently.

| Musical fact | At a workstation boundary |
| --- | --- |
| Polytempo | refused where the target has one tempo map; recorded where a projection is exact |
| Polymeter | refused or recorded per boundary; never rewritten into one meter |
| Microtonal pitch | recorded with the exact deviation and the channel or protocol used; refused where neither can carry it |
| Per-note expression | recorded with the protocol that carried it; refused where the target has only channel-wide control |
| Unsupported marks | recorded, named individually |
| Nonlinear routing | recorded as the routing that produced the artifact; never presented as reconstructible by summation |
| Written spelling, ties, voices, beams | recorded as lost on every MIDI boundary — MIDI carries note numbers |

A refusal names the fact, the boundary, and what would carry it. "Flatten and hope" is not an option this page leaves
open, and neither is a loss list that says *some information may be lost*.

## 7. Real-time code inherits the rules it already had

Every callback introduced by these boundaries — a CoreMIDI send or receive, an Audio Unit render block — is real-time
code under obligations §12 and roadmap §13.2. It does not allocate, lock, do I/O, log, compile, decode an asset, or
destroy a large object. Preparation happens on the control side and crosses on the existing wait-free queues, and the
private runtime rules of `03-machine-calculus.md` §8 apply unchanged: a layout chosen for a host is a private
optimization that must preserve the step equations exactly.

A host's real-time contract is *additional*, not alternative. Where Apple's render-block contract is stricter than
Musa's, the stricter one holds.

## 8. Logic Pro and GarageBand: what is documented, what is measured, what Musa declines

**Normative.** A row may be relied on only at the strength its column states. "Documented" means Apple's current primary
documentation, cited below. "Measured" means a Musa host probe recorded against named Logic Pro, GarageBand, macOS, and
Xcode versions; nothing here is measured yet, because prompt 210 builds no transport. "Musa" is a decision of this
project, and it does not change with a host version.

| Capability | Logic Pro | GarageBand | Measured | Musa's decision |
| --- | --- | --- | --- | --- |
| Standard MIDI File import | documented | documented (drag to a software-instrument track) | not yet | produced by the Export boundary |
| MusicXML import | documented | not documented | not yet | produced by the Export boundary |
| Audio file import | documented | documented: AIFF, CAF, WAV, AAC, Apple Lossless, MP3 | not yet | 32-bit float WAV only; other formats are the host's job |
| Audio Unit instrument | documented | documented, after Audio Units are enabled in settings | not yet | produced: one Music Device per checked instrument |
| Audio Unit effect | documented | documented | not yet | **declined** — Musa's studio is the work's room, not an effect for other people's audio |
| Audio Unit MIDI Processor | not read here | not mentioned in the cited GarageBand page | not yet | produced for Logic only, and only if prompt 215 measures the surface |
| Hosting third-party plug-ins | — | — | — | **declined**, permanently (roadmap §4) |
| Transport synchronization | documented | not documented | not yet | one declared clock authority per session (§4) |
| Reading a host's document | — | — | — | **declined**: Musa neither writes nor parses `.logicx` or `.band` |
| Round trip back into source | — | — | — | **declined**: no proved reverse conversion exists (§1) |

Sources, current as cited by prompt 210:
[Logic MIDI import](https://support.apple.com/en-ie/guide/logicpro/lgcpdf6a3851/mac),
[Logic MusicXML import](https://support.apple.com/en-ae/guide/logicpro/lgcp67fa6594/mac),
[GarageBand audio and MIDI import](https://support.apple.com/guide/garageband/import-audio-and-midi-files-gbndd01649ed/mac),
[Logic Audio Units](https://support.apple.com/en-ie/guide/logicpro/lgcp22a0dab0/mac),
[GarageBand Audio Units](https://support.apple.com/guide/garageband/use-audio-units-plug-ins-gbnde06a4e4d/mac),
[Logic synchronization](https://support.apple.com/guide/logicpro/general-synchronization-settings-lgcp7c04a41a/mac), and
[`AUAudioUnit`](https://developer.apple.com/documentation/audiotoolbox/auaudiounit).

The Logic rows above are cited at the strength of the pages' own titles and section structure: the article bodies of the
Logic guide were not readable when this table was written, and the GarageBand and `AUAudioUnit` rows quote what was.
Prompt 215 pins exact versions and replaces every "not yet" with a measurement or a refusal. Until then, no
implementation may cite this table as evidence that a host behaves a particular way.

## 9. What this boundary is not

It is not audio recording, waveform editing, comping, or a take timeline — roadmap §4's non-goal is unchanged, and
`../desktop/10-keyboard-composition.md` §4 says the same thing at the interface. It is not a mixer that other people's
sessions run through, not a plug-in host, not a document converter, and not a round trip. A workstation is where a
person finishes a record. Musa's claim is that the piece they carried in is exactly the piece the source says, and that
everything it could not carry is written down.

Implementation status for each boundary is claimed only by
[the implementation map](../../plan/code-map/spec-to-implementation-map.md), never by this page.
