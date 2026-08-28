# Keep what you just played

You do not always know a phrase is worth keeping until after you have played it. **Keep that** is for that case: it
takes the phrase you just played, without you having pressed Capture first.

## Turn it on

Recent phrase memory is on by default once a MIDI input has been used. The top margin says **Recent phrase on** and how
much is held: *Recent phrase on · 4.2 s*.

To turn it off, press **Clear** beside it, or use **Remember recent MIDI for Keep that** in Settings. Turning it off
clears the buffer immediately.

## Keep it

Play. Then press **Keep that**.

The most recent complete phrase is frozen and opened for Review exactly as a Capture take would be. From there the
workflow is identical: read it, answer what it asks, Accept, Keep.

## What is held, and for how long

A rolling suffix: at most 4,096 events or 30 seconds, whichever comes first. When either bound is reached the oldest
complete phrase is dropped, so the buffer never grows without limit and what remains is always a whole phrase rather
than a fragment starting mid-chord.

The buffer lives in memory only. It is never written to source, autosave, recovery, preferences, logs, crash reports, or
temporary files, and it holds no audio. It is cleared when the project closes, when the input device changes, when the
app suspends, and when you press **Clear**.

## When it refuses

| The margin says | What to do |
| --- | --- |
| **Nothing recent to keep** | Play something, or turn Recent phrase back on. |
| **Recent suffix only · 12.0 s** | Only the tail survived the bound. Keep the tail, or press Capture and play it again. |

A refusal names the duration that actually survived. Keep that never trims the opening note of your phrase because a
phrase model found it inconvenient, and never implies the whole phrase is there when it is not.
