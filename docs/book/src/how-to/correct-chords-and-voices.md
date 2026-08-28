# Correct chords and voices in a proposal

Two things a transcription cannot decide from timing alone: whether notes struck close together are one chord or several
onsets, and which note continues which line. Review asks about both, and the answers are musical rather than numeric.

## Walk to the question

In Review, `n` moves to the next mark and `p` to the previous. Marks are underlined and bracketed as well as coloured,
and each carries a spoken name, so the walk works with a screen reader and without a pointer.

Notes the app is confident about carry no mark. If nothing is marked, there is nothing to correct.

## Is this a chord?

At a cluster the app asks *What this cluster is* and offers the readings that are musically distinct:

- **one chord** — the notes are written stacked, struck together;
- **a rolled chord** — one chord with an arpeggiation sign, not five separate rhythms;
- **separate onsets** — genuinely successive notes, written as the rhythm you played.

Audition **Played** and **Written** before choosing. A spread you meant as a roll and a spread you meant as a melody
sound the same on paper and different in the ear.

## How long does this note last?

Under *Where this note ends* the proposal separates three things that a MIDI take reports as one:

- when you lifted the key;
- how long the note kept sounding because the sustain pedal was down; and
- what the notation should say.

A note held only by pedal is not automatically written long. Choose the reading you mean; the explanation says which
evidence each is based on. [Performed time and written time](../concepts/performed-and-written-time.md) explains why
these are three questions rather than one.

## Which line is this?

Where lines cross, the app asks under *Assign a line* rather than guessing by proximity. A crossing is legal and
sometimes exactly what you played; it is just more expensive to write than the reading where the lines do not cross.

A keyboard proposal writes at most four lines. Asking for a fifth is refused with that reason, not silently merged.

## Change your mind

**Take back** (or `u`) undoes the last decision and restores the previous proposal. Every decision is local: choosing a
reading at one mark settles that mark and leaves the others exactly as they were.

`Esc` backs out one layer at a time — the open question, then the selection, then the review itself. Discarding changes
nothing: the source and the undo history are as they were before you played.

## Name the lines before you keep

Once you press Accept, the margin asks for one name per line the phrase writes. It offers the voices your part already
has; a name it does not have is added as a new voice, and the margin says *2 bars into a new line* before you commit.

Two lines pointed at one voice is a refusal, not a merge.
