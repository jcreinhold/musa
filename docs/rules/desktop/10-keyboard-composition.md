# 10 — Keyboard Composition

Status: **governing** (amended at prompt 201; implemented by prompts 202–209).

A MIDI keyboard is first an instrument. Playing it is immediate and harmless; keeping what was played is deliberate;
turning a performance into notation is inspectable; and only accepted Musa source is the score. This page fixes the
workflow that preserves all four statements at once.

## 1. Four states, one authority

```text
          Listen ───── R / Capture ─────► Capture
             │                              │
             │ Shift-R / Keep that          │ R / Finish
             │                              │
             └──────────────────────────► Review
             ▲                              │
             ├──── Esc / Discard ───────────┤
             └──── Accept / one source edit ┘
```

- **Listen** is the default whenever an input device and prepared instrument are available. MIDI controls an ephemeral
  audition voice and changes no project source, score identity, selection, undo history, or saved state.
- **Capture** is explicit, finite, and visibly active. It keeps MIDI performance evidence, never audio.
- **Review** presents an immutable project-produced notation candidate, local alternatives, constraints, an exact source
  preview, and audition. It is neither a mutable frontend score nor a second project document.
- **Accepted** is the result of one preflighted project transaction. The inserted notes have ordinary source spans and
  no special authority. One undo restores the exact source before acceptance.

Discard changes nothing. No stopped take, candidate, or frontend adjustment is canonical. A device gesture never writes
source merely because a caret exists.

## 2. Listen: the keyboard is always playable

The selected part's source-declared instrument is the audition instrument. If selection has no part, the active caret's
part is used; a new piece uses its source-declared default. The interface says the instrument and input port in musical
terms: *Piano — Concert grand — KeyLab 61*. Choosing a different input device is application state, not project source.
Choosing a different instrument is the ordinary source edit from `09-sound-and-mix.md`.

Playing must not require transport playback, a valid current edit, Capture, an engraving round trip, or transcription.
While source is invalid, audition uses the last valid prepared instrument and the existing stale-revision treatment
makes that fact visible. Device arrival and loss do not steal focus, move selection, start Capture, or stop ordinary
audio playback.

MIDI note numbers are not written pitches. Audition may realize them at the device edge; notation spelling waits for the
checked key, scale, staff, and insertion context in Review. Velocity, pressure, bend, and pedals shape audition only
through supported source-declared controls. Unsupported input is reported in the device details, not silently remapped
into a different musical meaning.

## 3. Recent phrase memory is finite and visible

**Keep that** uses a bounded rolling suffix of recent MIDI evidence so a musician may decide after playing. While this
memory is enabled, the top margin says **Recent phrase on** and offers **Clear**. Settings offers **Remember recent MIDI
for Keep that**, enabled by default when a MIDI input is first used; disabling it clears the buffer immediately. The
buffer is memory-only and is cleared on project close, input-device change, app suspension, or explicit Clear.

Recent MIDI is never written to source, recovery, autosave, preferences, logs, telemetry, crash reports, project
metadata, or temporary files. It contains no microphone samples. Its time and event bounds are published and measured by
prompt 202; reaching either drops the oldest complete phrase boundary rather than allocating without limit. If no safe
boundary remains, Keep that refuses with the exact retained duration and a direct suggestion to use Capture.

Keep that freezes the suffix before transcription begins. Its initial boundary is the latest supported silence/pedal
boundary under the selected policy; Review lets the musician move that boundary while hearing the result. It never
deletes an opening note merely because an inferred phrase model finds it inconvenient.

## 4. Capture keeps performance evidence, not notation guesses

Starting Capture records the current project revision, destination caret/range, input device, audition instrument, clock
context, and chosen transcription policy. A count-in is offered when the piece supplies a clock and is not forced for
free capture. The visible state reads **Capturing — press R to review** and is also announced. `Esc` cancels without
producing a candidate.

The take retains device/callback timestamps, calibrated physical times, note-on and note-off, channel, note number,
velocity, sustain/sostenuto/soft pedal transitions, pitch bend, and supported pressure/control changes. It distinguishes
key-held duration, pedal-extended sounding duration, and eventual written duration. Events remain ordered exactly enough
to explain repeated notes and same-time controls; overflow, unmatched releases, clock discontinuity, and device loss are
facts, not repaired silently.

Capture is finite. The app gives elapsed time and a stop control, publishes event/time/polyphony bounds, and returns an
actionable refusal if a take exceeds them. It is not audio recording, a waveform, an overdub lane, comping, or a DAW
timeline.

## 5. Review asks musical questions locally

Review keeps the Compose page primary. The candidate is engraved in place as a proposal, with the existing score still
legible and source unchanged. The margin contains the short path through the work:

```text
Phrase  ·  Pulse and bars  ·  Voices  ·  Written notes  ·  Source
```

These are progressive disclosures, not mandatory wizard pages. The candidate opens at the first unresolved musical
choice. Unambiguous material stays quiet. An ambiguity is marked on the affected beat or notes, has a plain sentence
such as *These presses may be a rolled chord or two voices*, and offers two or three musically distinct readings. It
does not show a probability, generic settings wall, model name, cost vector, or “AI confidence”. The explanation may
disclose timing fit and notation complexity when asked.

Known tempo/meter and count-in are the primary clock. For free capture, Review may propose tempo, meter, and phase, but
it marks uncertainty. `T` taps pulse and `Shift-T` marks a downbeat; each tap constrains the same take and produces a
new immutable candidate. Taps are evidence supplied by the musician, not edits to the piece's tempo or meter. A proposed
context change must be written and accepted explicitly or the phrase must fit the existing context.

Review distinguishes:

- simultaneous chord, intentionally rolled chord/arpeggio, and separate onsets;
- notated duration, articulation, key release, and pedal-held sound;
- one voice, several voices, and a voice crossing that is costly but legal;
- rest, tie, dot, tuplet, syncopation, and bar-boundary spellings with equal performed time; and
- MIDI note number from written spelling in the destination's checked context.

The musician can audition the raw performance timing or the written candidate against the same selected instrument and
switch between them without changing source. Every alternative and constraint asks the project for another candidate;
the frontend never edits note objects or reimplements quantization.

## 6. Revision uses selections and commands, not entry state

Review and the ordinary score share one selection model. A lasso or candidate phrase may select an explicit set of
events across voices; keyboard extension stays contiguous within one voice. Selection is always visible and announced.
With a nonempty selection:

- pressing `8`, for example, sets each applicable note to an eighth note;
- **Scale durations** multiplies each selected written duration by an exact ratio;
- `Option-Up`/`Option-Down` moves every selected note by one diatonic step;
- **Transpose selection** applies a spelled interval; and
- **Respell accidentals** changes spelling without pretending it is the same operation as transposition.

Each command has one project-owned preflight that reports exact changed, unchanged, refused, and generated-source
counts; previews the resulting score and source; and applies one transaction. Mixed notes/rests, tuplets, ties,
cross-voice selections, and generated occurrences are disclosed rather than partly changed. With no selection, duration
and pitch-edit keys do nothing: there is no invisible “next note” state.

## 7. Accept is the only source boundary

Accept shows the smallest formatter-produced source insertion or replacement at the destination chosen in Review. A new
voice is added only after an explicit reviewed choice and name. Existing music, meter, key, tempo, part structure, or
generated definition is never replaced as a side effect.

Acceptance preflights against the current project revision. An unchanged stable destination may rebase; otherwise Review
says **Piece changed—review this phrase against the current score** and keeps the take. A successful accept compiles
before commit, writes one revision, follows ordinary autosave/recovery, selects the inserted phrase, and returns to
Listen. A failure leaves source, undo, saved state, and installed audio unchanged.

Accepted notes originate at their new source spans exactly as typed notes do. Raw timestamps, expressive controls,
calibration, model costs, alternatives, and review decisions enter neither musical identity nor hidden project metadata,
and are released after acceptance. Preserving a take would require an explicit later source/archive feature.

## 8. State and failure language

| Situation | Interface answer |
| --- | --- |
| No MIDI input | **No MIDI keyboard — choose an input device.** Source editing and playback remain available. |
| Input arrives | Name it without a toast; audition becomes available. |
| Input disappears while listening | End held audition notes safely; **Keyboard disconnected — choose an input device.** |
| Input disappears during Capture | Finish a partial take only when note/pedal state is explainable; otherwise keep it for inspection and refuse transcription. |
| Recent phrase disabled/empty | **Nothing recent to keep — turn on Recent phrase or start Capture.** |
| Recent phrase overflowed | State the retained suffix and offer Capture; never imply the whole phrase survived. |
| No usable onset | Keep Review open on the raw boundary and say what to trim or replay. |
| Ambiguous meter/voice/spelling | Mark only the affected passage and ask the smallest musical question. |
| Source becomes stale | Preserve the take and candidate; require re-review against the current score before Accept. |
| Invalid current source | Audition the last valid instrument; Capture may continue, but transcription/Accept waits for valid context. |
| Discard | Return to Listen; source and undo are unchanged; release the take. |
| Undo after Accept | Restore the exact pre-accept source and score through ordinary project undo. |

## 9. Accessibility and restraint

Capture and Review are fully usable without holding a key or using a pointer. State, elapsed time, device loss,
candidate replacement, ambiguity movement, and acceptance are announced at useful transitions, never per MIDI event.
Engraved ambiguities use a bracket or underline as well as color. Alternative readings have musical accessible names;
raw and written audition controls state which is sounding. Focus moves into Review at its first ambiguity and returns to
the selected inserted phrase or prior caret on Accept/Discard. Reduced motion changes no information.

Review adds no piano roll, waveform, permanent take lane, dense transport ruler, draggable timing points, or score-wide
warning wash. It asks only questions that can change the proposed notation. Exact detailed authorship remains better in
Musa source, and the interface says so rather than rebuilding a notation editor one gesture at a time.

## 10. Source owns policy; the host owns bounded mechanics

Named transcription policies and structural profiles are ordinary values in the standard-library Musa package. They may
state admissible metric subdivisions, complexity costs, chord-spread/voice preferences, spelling context, and review
thresholds. They do not contain device handles, timestamps, buffers, mutable model state, or host callbacks.

The host owns timestamp calibration, input/device state, bounded buffers, finite candidate search, prepared audition,
revision-safe proposals, and source transactions. Rust may carry an opaque checked projection of a source policy and
must differential-test it against the source value; it does not own a parallel enum of meters, tuplets, voices, or
styles. A repertoire-specific profile is named and selected, never smuggled into a universal default.

## 11. Transition from step entry

Closed. Prompt 33's `N` mode, active duration/octave/accidental state, MIDI-to-written-pitch heuristic, and fixed chord
window were temporary implementation evidence while prompts 202–208 built and proved this workflow; prompt 209 removed
them once audition, Capture, Keep that, Review, group revision, Accept/Discard, device loss, stale source, and undo
passed their checks. There is no second supported design and no preference that restores one. What the old path was and
why it was rejected is recorded in prompt 33 and note 88, which are history rather than an alternative.
