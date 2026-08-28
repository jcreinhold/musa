# Performed time and written time

A MIDI take and a notated phrase are two different objects, and transcription is a translation between them rather than
a conversion of units. This page explains what the two sides actually contain, why the pedal makes a third question out
of what looks like one, and why Musa asks instead of guessing.

## What a performance contains

A take is a list of physical facts: a key went down at a measured time with a measured velocity, a key came up, a pedal
changed state, a bend moved. Every time in it is a real time, measured against a calibrated clock, and no two are
related by anything except the performer.

Nothing in that list is a duration in the notational sense. A key held for 431 ms is not "an eighth note played slightly
long" until someone decides what the beat is.

## What notation contains

A written phrase is a list of musical facts: this note is a dotted quarter in 6/8, it is tied across the bar, it is the
third member of a triplet. Every time in it is an exact rational fraction of a whole note (`009-notating-rhythm.md`,
`010-simple-meter-and-time-signatures.md`), and the relationships between them are the point — a triplet member means
nothing except relative to the beat it divides.

Musa keeps these exact, in rationals rather than floats, everywhere but the audio edge. [Exact time](exact-time.md)
explains why.

## Why the translation is a search, not a formula

Between the two sides there is no function. The same take supports many correct notations, and the same notation
supports infinitely many performances. Choosing between the readings is a cost question: how far each onset is
displaced, how complex the resulting notation is, how many rests and ties and tuplets it needs, how smooth the implied
tempo stays.

Musa runs that search under a *policy* — an ordinary value in the standard library, not a hidden host default — and
publishes the cost fields it weighs. Where two readings are close enough that the difference is musical rather than
numeric, Review asks you, in a sentence, and offers the readings that actually differ. It never shows a probability,
because a probability would be a claim about your intent that the app is in no position to make.

## The pedal makes three questions out of one

Press a key, lift it, but keep the sustain pedal down. The note keeps sounding. That single gesture produces three
different durations:

| Quantity | What it is | Where it lives |
| --- | --- | --- |
| Key-held duration | Down-time to up-time, as played | the take |
| Sounding duration | How long it was audible, pedal included | the take |
| Written duration | What the notation says | the accepted score |

They are not versions of each other. A pianist holding a chord with the pedal while the hands move on has written
quarter notes and sounding half notes, and writing the half notes would be a different piece. Equally, a phrase where
the pedal is the whole point — a note deliberately let ring — loses something if only the key-held time survives.

Musa keeps all three as separate facts, proposes a written duration, and marks the note where the choice is genuinely
open. That is [layer separation](layer-separation.md)'s general rule applied to one gesture: notated duration is not
performed duration, and one is never quietly substituted for the other.

## Once it is written, it is ordinary

Accepted notes have ordinary source spans and no special authority. Raw timestamps, velocities, the decisions you made
in Review, and the alternatives you rejected do not survive into the piece and are not stashed in hidden metadata: they
are released when the phrase is kept. What remains is source you could have typed, and [Origin view](provenance.md)
shows it as authored, because you authored it.

That is the deliberate cost of the design. A take is not a document, and Musa does not become a recorder to make it one
— see the scope statement in [Capture and transcription](../reference/capture-and-transcription.md).
