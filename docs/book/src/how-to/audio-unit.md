# Use the Musa Audio Units in a host

A bundle hands a workstation files and live MIDI hands it a performance. The Audio Units put Musa *inside* the
workstation: an instrument it plays and a MIDI FX plug-in that scheduled the piece onto its own timeline.

You need the containing app installed, because that is what registers the components and what grants a plug-in access to
your project folder. macOS only.

## Check that the host can see them

```bash
auval -a | grep Musa
```

Two lines, one per component:

```text
aumu musa Musa  -  Musa: Musa: Instrument
aumi musp Musa  -  Musa: Musa: Processor
```

If only one appears, or neither, see [When a workstation cannot see it](daw-troubleshooting.md).

## Play one part from the host's keyboard

1. Make a software-instrument track.
2. In its instrument slot, choose **AU Instruments ▸ Musa ▸ Musa: Instrument**.
3. Open the plug-in window and point it at your `.musa` project. Choose the piece, then the part.
4. Play. The host's MIDI reaches the part's instrument through the mapping the source declares.

The plug-in window lists the piece's identity, the asset closure's identity, and every declared control that could not
become a host parameter. Read the last of those before you go looking for a knob that is not there.

**Automation.** Each admitted control appears as an ordinary host parameter with the range, default, and unit the source
declared. Write automation against it as you would any plug-in's. The address behind a lane is saved in the host's
document and restored with it, so renaming a control in the source, reordering the declarations, or reformatting the
file does not move your automation. A change the plug-in cannot make safely is refused with a message instead.

Nothing you do here is written back to `.musa`. Parameter moves are a performance, not an edit.

## Take more than one output

Instantiate the Instrument as a **multi-output** instrument, and the host gets bus zero (the piece's main output), then
the part's own output, then any studio bus it sends to, each under the name the source gave it.

A host that takes only one output gets exactly the same bus-zero frames. There is no separate "stereo mix" — the
fallback is the main output itself.

## Schedule a whole piece onto the host's timeline

This is the MIDI Processor, and **Logic Pro** is the host it is supported in.

1. Make a software-instrument track.
2. On that track's channel strip, open the **MIDI FX** slot ▸ **Audio Units ▸ Musa ▸ Musa: Processor**.
3. Point it at the project and choose the piece.
4. Choose the **reading** — *score* for the piece as written, *performance* for the piece as played.
5. Choose the **timeline** — *piece* to place positions on the piece's own exact schedule, *host* to place them in
   quarter notes on the host's musical timeline so its tempo track moves the piece.
6. Put an instrument below it in the strip. `Musa: Instrument` works; so does any other.

Transport, tempo, cycle, and position stay the host's. The processor follows them and owns none of them. Seeking starts
where you seek to rather than replaying what you skipped, and notes that were sounding across the seek point are
re-entered.

**Do not run the processor and import the same piece's MIDI at once.** They are two projections of one piece and would
duplicate every message. Import when you want the piece permanently in the session; run the processor when you want it
played from the source.

**GarageBand is not supported for the processor.** Not measured and found wanting — not observed at all. Use the
Instrument there, or import a bundle.

## Move a session to another machine

The plug-in stores where your project lives and the identities of the music and the assets — never the bytes. Carry the
`.musa` project with the session. On the other machine, open the plug-in window and point it at the project again; the
identities in the document tell you at once whether it is the same music you left.

## What it will not do

- write `.musa` source, ever — the source is the master record;
- read the host's session document;
- guess a tempo when the host supplies none;
- flatten a polytempo piece onto the host's one quarter-note grid. It refuses that combination by name; put it on the
  *piece* timeline instead, where every part plays at its own speed.
