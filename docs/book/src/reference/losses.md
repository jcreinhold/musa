# Losses

A **loss** is something the piece has that a reading of it could not carry. Musa never absorbs one silently: it is
either refused outright, or recorded in the artifact and in the report you get back.

Every loss has a *kind* — a stable word you can match on — and a sentence saying what it means for this piece. The
sentences differ by piece; the kinds do not.

## Where losses appear

| Crossing | Where to read them |
| --- | --- |
| Bundle export | `musa-manifest.json`'s `losses` array, and the CLI's `warning:` lines |
| Live MIDI | the run report, before the first note is sent |
| Audio Unit instrument | the plug-in's own window, as control losses |
| Audio Unit MIDI Processor | the plug-in's own window, as projection losses |

## The kinds

### `polytempo`

The piece's parts move at their own speeds, so there is no single tempo map to hand over. A Standard MIDI File has one
conductor track; MIDI clock counts one stream of pulses.

In a bundle this is recorded and the export continues — the MIDI is written on one reading of the clock and the note
says so. On the Audio Unit MIDI Processor's *host* timeline it is **refused**, because a wrong grid there would play the
piece wrongly rather than merely describe it incompletely. The same piece plays correctly on the processor's *piece*
timeline, where every part keeps its own speed.

### `polymeter`

Two barrings at once. A workstation shows one meter at a time, so the bar lines you see there will be one of them.

### `tuning`

Neither a Standard MIDI File nor a WAV states what A was tuned to. The audio was rendered at the piece's own tuning; the
MIDI will be played at whatever the receiving instrument is tuned to. If your piece is not at A = 440 Hz, this matters.

### `controller`

The exported MIDI carries notes, tempo, meter, and key — not continuous controllers. Dynamics, articulation, and profile
shaping live in the rendered audio, not in the MIDI file. Import the audio if you want to hear the performance; import
the MIDI if you want the notes to play through your own instruments.

### `channel`

The piece has more parts than MIDI has channels, so two parts share one. Workstations that read *tracks* keep them
apart; a workstation reading *channels* will merge them. The manifest's `parts` array says exactly which parts share.

### `notation`

Something the notation file could not express, or — in a GarageBand-profile bundle — the fact that there is no notation
file at all, because GarageBand does not document MusicXML import. Export for Logic, or run `musa render --to musicxml`,
if you want one.

### `conductor`

Specific to the MIDI Processor: a MIDI effect emits channel messages, so the piece's tempo, meter, and key are the
host's to set. Set them in the host to match the piece, or use the piece's own timeline.

### Control losses (Audio Unit)

Not a kind but a shape: a source-declared control whose value kind cannot be a host parameter. Each names the control's
canonical source identity, its kind, and one clause saying why — typically that its value is a typed relation rather
than a quantity, or that its declaration states no value domain. A control in this list is still part of the piece; it
is simply not automatable from the host.

## Reading them from a script

The manifest's array is the machine-readable form:

```json
{
  "losses": [
    { "kind": "tuning", "message": "the audio was rendered at A = 440 Hz; …" },
    { "kind": "controller", "message": "the MIDI files carry notes, tempo, meter, and key …" }
  ]
}
```

Match on `kind`. The `message` is for a person, and its wording may improve.
