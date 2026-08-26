---
id: 201
slug: keyboard-composition-contract
status: pending
depends_on: [193, 200]
phase: 2
---

# Compose by Playing or by Writing, Not by Splitting One Gesture Between Them

## Task

Amend the desktop interaction specification and roadmap before replacing MIDI step entry. Establish one coherent
keyboard-composition workflow: a connected keyboard always auditions the selected instrument; **Capture** records a
finite expressive MIDI take; **Keep that** recovers a bounded recent phrase; **Review** turns either take into an
inspectable notation proposal; exact notation is written directly in Musa source. Group score edits remain a separate,
bounded way to revise existing music.

## Read

- `docs/rules/README.md`'s amendment procedure; constitution §§1, 4, 6, and 9; obligations §§1, 4, 6, and 9;
  `docs/rules/desktop/00-thesis.md`, `03-interaction.md`, `04-provenance.md`, `05-states.md`, and `06-frame-budgets.md`;
  roadmap §§2, 12.5, 14.5–14.8, and 17.
- Prompts 23, 25, 27, 33, 53, 57, 61, 64, 74–75, 77, 189–193, and 200; current selection, semantic-edit, MIDI-input,
  transport, prepared-audio, source-workspace, autosave, and generated-edit paths.
- Open Music Theory chapters `009-notating-rhythm.md`, `010-simple-meter-and-time-signatures.md`,
  `011-compound-meter-and-time-signatures.md`, `012-other-rhythmic-essentials.md`,
  `098-twentieth-century-rhythmic-techniques.md`, `117-hypermeter.md`, and `118-metrical-dissonance.md`. They establish
  why readable notation is a hierarchical metrical interpretation, not independent nearest-grid rounding.
- Cemgil, Kappen, Desain, and Honing,
  [“Rhythm Quantization for Transcription”](https://doi.org/10.1162/014892600559218); Nakamura, Itoyama, and Yoshii,
  [polyphonic MIDI rhythm transcription](https://eita-nakamura.github.io/articles/Nakamura_etal_RhythmTranscriptionOfPolyphonicMIDIPerformances_SMC2016.pdf);
  Madsen and Widmer, [“Separating Voices in MIDI”](https://www.cp.jku.at/research/papers/Madsen_Widmer_ISMIR_2006.pdf);
  and the [ASAP aligned score/performance dataset paper](https://apmcleod.github.io/pdf/ismir-asap.pdf). These inform
  the trial in prompt 203; they do not govern Musa or justify importing one repertoire's priors as universal defaults.

## Design

Remove the false equation “MIDI keyboard = step entry.” Step entry divides one statement among pitch on the instrument,
duration/modifiers on the computer, and position on the page; Musa source already expresses exact notation more
directly. The final interface has no persistent entry mode, active-duration state, chord-window insertion, or letters
that change meaning because `N` was pressed.

Fix four states and their authority:

1. **Listen** is the default whenever a keyboard and a prepared instrument are available. Playing is immediate and never
   edits source.
2. **Capture** is explicitly armed and visibly active. It retains finite MIDI key, velocity, controller, pedal, and
   timestamp evidence, not audio.
3. **Review** displays an immutable project-produced transcription candidate plus alternatives and constraints. It is
   not a second editable score model; every adjustment asks the project for a new candidate and source preview.
4. **Accepted** applies one transactional semantic source edit. Discarding changes nothing; one undo reverses
   acceptance.

**Keep that** freezes the visible, bounded, memory-only recent-event buffer. The app states that recent MIDI is being
remembered, allows it to be disabled/cleared, and never writes it to disk or telemetry. This is finite symbolic capture,
not microphone recording, waveform editing, or a DAW timeline.

The primary accurate path is capture against the piece's known tempo/meter or a count-in. Free capture may propose
tempo, meter, and phase, but ambiguity is shown. A musician can tap pulse and downbeat during review; the system never
hides an uncertain metrical guess behind “confidence.” Performed key-release time, pedal-extended sounding time, and
notated duration remain three different facts.

Extend the selection contract only as needed for group transformations. An explicit nonempty set of `EventId`s may span
voices when a rectangle, candidate phrase, or project command supplies it; the frontend may collect rendered identities
by geometry but may not infer pitch, rhythm, voice, provenance, or applicability. Commands—not persistent modes—set all
selected durations, scale durations, transpose by a spelled interval, move diatonically, or respell accidentals. Every
operation previews exact affected/unchanged counts and generated-source consequences before one transaction.

Declarable transcription policies and named structural profiles belong in ordinary standard-library Musa source. The
host owns device timestamps, bounded event buffers, clock calibration, prepared audition state, finite search, and the
provenance-preserving proposal/edit boundary. No Rust enum becomes a second catalogue of meters, tuplets, voices, or
styles.

Record the rejected step-entry workflow and the argument above in a research note. Prompt 209 removes the implementation
only after prompts 202–208 prove the replacement end to end; until then, old entry remains a migration bridge rather
than the intended design.

## Target

- Deliberately amended desktop interaction/state rules and roadmap, including states, keyboard map, privacy, selection,
  review, source authority, and the finite-MIDI/not-audio-recording boundary.
- A research note satisfying the amendment procedure and a code-map plan for prompts 202–209.
- An explicit interaction state diagram and accessibility language for Listen, Capture, Keep that, Review, Accept,
  Discard, device loss, stale source, and undo.
- Prompt 33 retained as historical implementation evidence but linked to prompt 209's planned retirement.

## Check

```sh
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
make docs-check
python3 scripts/renumber-prompts.py audit
```

Commit as `Define keyboard capture and transcription workflow`.

## Stop

- No Rust, TypeScript, standard-library, UI, MIDI, audio, transcription, or edit implementation.
- No audio recording, pitch detection, FFT, waveform editor, DAW timeline, learned model, or cloud service.
- Do not delete step entry until prompt 209 proves that Capture, Keep that, Review, and exact source entry replace it.
