# Your first captured phrase

This lesson plays a phrase on a MIDI keyboard and ends with it written into a piece, in ordinary Musa source you could
have typed yourself. It takes about ten minutes and needs the desktop app and a keyboard plugged in.

You will not learn a note-entry mode here, because there is not one. The keyboard is an instrument. Turning what you
played into notation is a separate, visible decision, and the score is only ever the source you accepted.

## Before you start

Run the app and open a piece:

```bash
make desktop
```

Open any `.musa` piece — [Getting started](getting-started.md)'s `first.musa` will do — and look at the top margin of
Compose. If a keyboard is connected, its name is there beside the part it will sound as. If it says **No MIDI
keyboard**, choose one from the list beside it.

## 1. Play something

Play. You should hear the selected part's own instrument, and the score should not change at all: no notes, no
selection, no undo step, no asterisk on the filename.

That is the whole of the first state. The keyboard sounds the piece's instrument whenever it is safely connected, and
listening writes nothing.

## 2. Capture a phrase

Click in the score where the phrase should go, so there is a caret. Then press **Capture**.

The button becomes **Finish**, and beside it the app counts the take: *Capturing 3.4 s*. Play four or five notes in
time, then press **Finish**.

Nothing has been written yet. Capture keeps what you played — key presses, releases, velocities, the pedal — and not one
guess about how it should be notated.

## 3. Read the proposal

Press **Review**.

The take is engraved in place as a proposal, with the piece still legible behind it. Along the top: what the phrase is,
where it would go, and the verbs. Down the page, each note of the proposal, with a bracket under anything the app is not
sure of.

Try the two audition buttons:

- **Played** sounds the take with its own performed timing.
- **Written** sounds the notation as proposed.

Switching between them decides nothing. Hearing the difference is the point: it is how you tell a phrase you rushed from
a phrase that is genuinely in triplets.

## 4. Answer the questions

If a bracket is marked, press `n` to walk to it. The app asks one short musical question — *These presses may be a
rolled chord or two voices* — and offers two or three readings you could actually mean.

Choose one. The proposal is replaced by a new one, and the notes you did not ask about stay exactly as they were. Press
`u` (or **Take back**) if you change your mind.

You never have to answer a question you do not care about. Anything the app is confident of stays quiet, and a phrase
with no ambiguities opens with nothing marked at all.

## 5. Accept, then keep

When the reading is the one you want, press **Accept** (or `k`).

Accepting settles the notation. The margin then asks where the phrase goes: one name per line the phrase writes,
pre-filled with the voices your part already has. A name the part does not have will be added as a new voice, and the
margin says so before you commit to it.

Press **Keep**. Now, and only now, the source changes: one revision, formatted the way the formatter would have
formatted it if you had typed it, at the caret you chose. Open the source column and read it.

## 6. Undo it

Press `⌘Z`.

The piece is exactly what it was before you kept the phrase — one keystroke, not one per note. The phrase you accepted
was one edit, because keeping it was one decision.

## What you did, and what you did not

You went **Listen → Capture → Review → Accept → Keep**. Four states, one source boundary, and the boundary is the last
step rather than the first.

You did not select a duration and then press a key to place a note. That workflow does not exist in Musa: for exact
authorship the source is faster and clearer than any grid of buttons, and it is right there in the source column.

## Next

- Give the take a clock it can trust: [Capture against a tempo](../how-to/capture-with-a-clock.md).
- Play rubato and supply the pulse yourself: [Capture without a tempo](../how-to/capture-without-a-clock.md).
- Decide after playing instead of before: [Keep what you just played](../how-to/keep-what-you-just-played.md).
- Understand what is actually being converted:
  [Performed time and written time](../concepts/performed-and-written-time.md).
