# Where Musa stops and a workstation starts

Musa is not a workstation, and a workstation is not a bad Musa. They are two different things that a piece passes
between, and almost every confusing question about "DAW support" gets easy once you know which side of that line you are
standing on.

The line is this: **the `.musa` source is the work.** Everything a workstation receives — a MIDI file, a WAV, a plug-in
window, a stream of live notes — is a *reading* of the work, produced in one direction. Nothing on the far side of the
line can edit the near side. There is no import, no round trip, and no second place your piece secretly lives.

That is one decision, and the rest follows from it.

## Three crossings, not one integration

"Works with Logic" is not a feature; it is three different features with different rules.

| Crossing | What crosses | Who owns time |
| --- | --- | --- |
| **A bundle** | a folder of open files, written once | Musa: every file states its own alignment |
| **Live MIDI** | notes, as they come due | one declared authority — Musa or the workstation, never both |
| **An Audio Unit** | Musa runs *inside* the workstation | the workstation, entirely |

They are separate because their failure modes are separate. A bundle can be examined at leisure and re-exported; a live
stream cannot be un-sent; and an Audio Unit is a guest in someone else's process with a deadline measured in
microseconds. A design that treated all three as "MIDI support" would get two of them wrong.

## Nothing is flattened quietly

Every crossing loses something. A Standard MIDI File has one tempo map, so a piece whose parts move at their own speeds
does not fit in it. MIDI has sixteen channels. Neither MIDI nor WAV records what A was tuned to. MusicXML has no
Musa-shaped notion of a realization seed.

The rule is that a loss is **refused or recorded**, and never absorbed. If the reading cannot be produced honestly, you
get a refusal that says why:

> cannot prepare audio: this piece is polytempo: its parts play at their own speeds, so there is no one quarter-note
> grid to lay on the host's.

If it can be produced but something did not survive, the loss is written down where the artifact is — in the bundle's
manifest, in the export report, in the plug-in's own window. [Losses](../reference/losses.md) is the list of what can be
lost and what each one means.

This is why a bundle carries a manifest at all. A folder of files is what a workstation reads; the manifest is what
makes the folder *honest* about what it is.

## Identity survives, so a rendering can be traced back

Every crossing carries the exact identity of the source it came from and, where the reading has notes in it, a record of
which source event produced which note. Six months later, holding a WAV and a Logic project, you can still say which
`.musa` file, at which content, at which realization seed, produced it.

You cannot go the other way — a WAV is not a piece — and Musa does not pretend otherwise. What it promises is that the
question "what was this?" always has an exact answer.

## A plug-in parameter is a source-declared control

When Musa runs as an Audio Unit, the parameters the workstation shows are exactly the controls the source declared as
exposed. Not the DSP graph's internals, not a fixed catalogue chosen by whoever wrote the plug-in, and not a coerced
float for something that was never a number. A control whose kind has no value domain becomes a named loss instead of a
knob that lies.

The consequence is pleasant: adding an exposed control to your source adds a parameter to the plug-in, and nothing in
the plug-in has to be updated for that to happen. The consequence is also strict: automation you write in the host is
*host* automation. It shapes the performance for that host session. It does not write back into your source, because
nothing does.

## What Musa declines, on purpose

- **Hosting other people's plug-ins.** Musa's studio is the work's room, not a rack.
- **Being an audio effect.** Same reason.
- **Reading or writing `.logicx` and `.band`.** They are proprietary documents belonging to their applications.
- **A round trip.** No proved reverse conversion from any of these readings back to source exists, so claiming one would
  be claiming something false.

A workstation is where a person finishes a record, with fingers and ears and other people's plug-ins. Musa's claim is
narrower and, we think, more useful: the piece you carried in is exactly the piece the source says it is, and everything
that could not come with it is written down.

The governing statement of all of this is
[`docs/rules/across-stages/06-daw-boundary.md`](../../../rules/across-stages/06-daw-boundary.md); what was measured
against it is the audit note in `docs/notes/research/` beside it.
