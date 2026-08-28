# Capture against a tempo

When the piece states a tempo and a meter, use them: a take against a known clock needs far fewer decisions than one
that has to have its pulse inferred.

## Give the piece a clock

The clock comes from the piece, not from the app:

```musa
tempo 1/4 = 104;
meter 4/4;
```

If the piece already declares both, there is nothing to do. If it does not, add them in the source column first — a
tempo you set in a dialog would be app state, and Musa keeps the clock where the score keeps it.

## Capture

1. Click where the phrase belongs, so there is a caret. The caret decides which part sounds and where the phrase would
   be written.
2. Press **Capture**. A count-in is offered, because the piece supplies a clock.
3. Play.
4. Press **Finish**, then **Review**.

## What the clock buys you

The proposal opens with the meter and tempo the piece stated, so onsets are placed against a grid you chose. Bar lines
appear in the proposal at the destination meter. The questions that remain are the musical ones — is this a rolled chord
or two voices, is this note held or pedalled — rather than "what speed was that".

## When you drift

A take that drifts against the click is still transcribed against the piece's clock: Musa does not quietly re-time the
piece to match your playing. If the reading looks displaced, audition **Played** against **Written** to hear which of
the two is wrong, then either play it again or move to [free capture](capture-without-a-clock.md) and supply the pulse
yourself.

Changing the piece's tempo or meter is an ordinary source edit, made deliberately, and never a side effect of accepting
a phrase.

## Bounds

One take holds up to 65,536 events or ten minutes, whichever comes first. Both are published in the margin, and reaching
one is an explicit refusal rather than a silent truncation — see
[Capture and transcription](../reference/capture-and-transcription.md).
