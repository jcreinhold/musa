# Capture and transcription

What a take contains, what reads it, what it will refuse, and where it stops. The workflow itself is taught in [Your
first captured phrase](../tutorials/first-captured-phrase.md).

## Scope

Musa captures **MIDI performance evidence** and turns it into notation you accept.

It does not transcribe audio. There is no microphone input, no waveform, no pitch detection from sound. It is also not a
recorder: there are no recording sessions, no take lanes, no comping, no overdub timeline, no punch-in. A take exists to
become notation, and is released once it has.

Preserving performances as documents would be a different feature, with its own source syntax and its own archive. It
does not exist today, and the app does not pretend otherwise by keeping takes alive in hidden state.

## Supported MIDI evidence

A take retains:

|  |  |
| --- | --- |
| Times | device and callback timestamps, and the calibrated physical time derived from them |
| Notes | note-on and note-off, channel, note number, velocity |
| Pedals | sustain, sostenuto, and soft pedal transitions |
| Expression | pitch bend, and the pressure and control changes the source declares |

Three durations stay distinct: **key-held** (down to up), **pedal-extended** (how long it sounded), and **written**
(what the notation says). See [Performed time and written time](../concepts/performed-and-written-time.md).

Order is preserved exactly enough to explain repeated notes and same-time controls. Buffer overflow, unmatched releases,
clock discontinuity, and device loss are recorded as facts about the take, not repaired silently.

Input the piece does not declare is reported in the device details. It is never remapped into a different musical
meaning.

## Transcription policies

A policy is an ordinary value in the standard library — `stdlib/src/transcription/mod.musa` — not a host default. Two
ship today.

### `standard`

| Admits | Values |
| --- | --- |
| Binary subdivisions of the beat | whole beat, half, quarter, eighth (through the thirty-second note) |
| Irregular divisions | triplets at the eighth and sixteenth level (`1/3` and `1/6` of a beat) |
| Onset-group window | one twelfth of the local beat, clamped to 18–70 ms |

The declared denominators have least common multiple 24, which is exactly the measured quarter-note grid the search runs
on.

### `unmeasured`

No subdivision and no tuplet. A take declared unmeasured has no honest measured notation, so the policy refuses to write
one rather than inventing a grid.

### Cost fields

Readings are ranked over nine published fields, in this order:

`onset-displacement`, `duration-displacement`, `tempo-smoothness`, `notation-complexity`, `rests`, `ties`, `tuplets`,
`syncopation-preservation`, `user-constraints`.

The order and the artifact are versioned (`cost_version = 1`). No probability, model name, or confidence score is part
of a reading, because none of those is a fact about your intent.

## Bounds

| Bound | Value |
| --- | --- |
| One capture take | 65,536 events or 10 minutes |
| Recent phrase memory | 4,096 events or 30 seconds, under 1 MiB |
| Notes searched per take | 128 |
| Search states retained per layer | 96 |
| Search storage | 131,072 bytes |
| Alternatives offered | top 5 |
| Lines a proposal writes | 4 |

Reaching a bound is an actionable refusal that names the retained duration or count. It is never a silent truncation.

Measured against these ceilings on an Apple M4 Pro, a 128-note take completes in about 2.7 ms and uses 96 states and
2,090 bytes — see
[`docs/notes/research/91-keyboard-composition-closure.md`](../../../notes/research/91-keyboard-composition-closure.md).

## What Review can ask

Seven kinds of question, each marked on the notes it concerns:

| Kind | The question |
| --- | --- |
| `pulse` | How fast is this, when the piece did not say? |
| `phase` | Where does the beat fall? |
| `placement` | Where does this onset land — swing, or triplet? |
| `end` | Where does this note end — key release, or pedal? |
| `group` | Is this cluster a chord, a rolled chord, or separate onsets? |
| `voice` | Which note continues which line? |
| `spelling` | Which correct spelling of this chromatic note, in this key? |

Each offers two or three musically distinct readings. Choosing one settles that mark and leaves every other mark
standing; nothing you did not ask about changes.

## Review commands

| Key | What it does |
| --- | --- |
| `←` `→` | Walk the notes of the proposal |
| `n` `p` | Walk to the next or previous question |
| `↑` `↓` | Move the selected notes on the staff (`⇧` moves the accidental instead) |
| `1` `2` `4` `8` `6` `3` | Write that duration on the selection |
| `t` | Transpose the selection by a typed interval |
| `u` | Take back the last decision |
| `k` | Accept the reading, then keep the phrase |
| `Esc` | Back out one layer: the open question, the selection, then the review |

The **Played** and **Written** buttons audition the take and the proposal against the same instrument. Auditioning
decides nothing.

## Declared losses

A proposal states what it could not write, in the musician's terms:

- *a key went down and never came up; it is not written*
- *a key came up that never went down; it is not written*
- *n notes were still sounding when the take ended*
- *note n is a length this notation cannot write exactly, so no source was written*

A loss is disclosed, never repaired by inventing a note or a duration.

## Refusals

| Situation | The answer |
| --- | --- |
| Nothing recent to keep | Play something, or turn Recent phrase on |
| Recent memory overflowed | The retained suffix is named; Capture is offered |
| A device disappeared with unexplainable state | The take is kept for inspection; transcription is refused |
| Source does not compile | Audition continues on the last valid instrument; transcription waits |
| The piece changed since the take | *Review this phrase against the current score*; the take is kept |
| The destination part no longer exists | Refused, and the take survives it |
| A reading still has questions open | It cannot be kept until they are answered |
| A reading has no exact written form | It cannot be kept |
| A phrase writes two lines and they are unnamed | It cannot be kept until they are named |
| Two lines named for one voice | Refused |
| A voice name the language cannot write | Refused before the parser sees it |
| A phrase already kept | Cannot be kept twice |
| A fifth line | Refused, with that reason |

## What accepting writes

One revision, one undo. The insertion is the smallest formatter-produced source at the destination you chose, compiled
before it commits. Existing music, meter, key, tempo, part structure, and generated definitions are never replaced as a
side effect.

Accepted notes originate at their new source spans exactly as typed notes do. Timestamps, velocities, review decisions,
and rejected alternatives enter neither the musical identity nor any hidden metadata.

## Privacy

Recent phrase memory is in memory only. It is never written to source, autosave, recovery, preferences, logs, telemetry,
crash reports, project metadata, or temporary files, and it contains no audio. It is cleared on project close, input
change, app suspension, and explicit **Clear**, and turning the setting off clears it immediately.
