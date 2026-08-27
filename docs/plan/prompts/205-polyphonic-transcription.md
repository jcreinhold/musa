---
id: 205
slug: polyphonic-transcription
status: pending
depends_on: [204b]
phase: 2
---

# Turn Rhythm Candidates into Honest Polyphonic Notation Proposals

## Task

Complete transcription from exact rhythm candidates to pitches, chords, voices, rests, ties, and a checked Musa source
proposal. Preserve ambiguity and performance evidence rather than forcing every keyboard gesture into one pianistic
notation convention.

## Read

- Prompt 203's measured chord/voice/pedal decisions in `docs/notes/research/90-midi-transcription-trial.md`, prompt
  204b's candidate/constraint facade, prompt 202's complete event evidence, and prompts 27, 39–40, 63–65, 70–75,
  100–103, and 176b–180.
- Open Music Theory chapters `001`–`007` (written spelling), `009`–`012` (rhythm), `022`–`031` (voice leading as one
  named practice), `083-rhythm-and-meter-in-pop-music.md`, `098-twentieth-century-rhythmic-techniques.md`, and
  `118-metrical-dissonance.md`.
- Madsen and Widmer's voice-separation paper and Nakamura et al.'s multiple-voice rhythm model cited by prompt 201.
  Their pitch-proximity/continuity/crossing preferences are candidate costs, not laws of all polyphony.

## Design

First construct physical key intervals independent of sounding intervals: pair note-ons/offs per channel and pitch,
retain attack/release velocity, and retain pedal/controller spans separately. Sustain or sostenuto may inform audition,
phrasing alternatives, and explicit pedal-mark suggestions; it never simply lengthens every written note to pedal-up.
Overlapping retriggers remain distinct notes.

Cluster near onsets using prompt 203's measured starting proposal: one twelfth of the local beat, clamped to 18–70 ms.
It is a source-policy-weighted candidate cost, not a hard grouping rule. The 40 ms corpus attack at 120 BPM must remain
one available group; the fixed 35 ms baseline incorrectly split it. A compact spread may be a chord; an ordered spread
may be an arpeggio/rolled chord; a fast scale is neither. Keep competing groupings where cost separation is small and
let the musician constrain any group. A fixed global millisecond window is forbidden. “Small” means the top-five
retained structural candidates disagree locally, not an uncalibrated confidence percentage.

Assign events to at most four candidate voices jointly with rhythm by extending prompt 204a's shared candidate DAG,
without reparsing or rebuilding its phrase lattice. Costs may use pitch proximity, temporal continuity, hand/register
hints, overlap, repeated patterns, and crossings; crossings and large leaps are penalties, never invalidity. The
selected destination voice count and an explicit one-/two-/N-voice constraint outrank inferred preference. Do not invent
a new staff, voice, or hand assignment without surfacing it in Review.

Spell pitches from the key/collection and scope at the proposed insertion point while preserving MIDI pitch identity.
Chromatic, atonal, microtonal, pitch-bend, and ambiguous enharmonic evidence produce alternatives or explicit loss;
there is no “C major if unknown” semantic default. Chord members retain individual spellings. A later selection command
can respell without retranscribing timing.

Infer written ends jointly from key intervals, subsequent attacks, articulation patterns, rhythm vocabulary, voice
continuity, and bar structure. Short key release is weak evidence for staccato; velocity is weak evidence for metrical
accent; neither creates dynamics, articulations, slurs, pedal marks, or performance controls without a specific review
choice. Grace/ornament candidates are shown as such rather than silently converted to tiny ordinary notes.

Produce an immutable `NotationProposal`: rhythm candidate identity, voices of exact score facts, chord/rest/tie/tuplet
structure, local alternatives/constraints, complete derivation to take events, declared losses, and a canonical source
preview built with the existing syntax/formatter. Parse and compile that preview under the destination project context;
an uncheckable proposal is an internal error, never something Review asks the musician to repair.

Preserve prompt 203's bounds: at most five proposals, 96 retained states per layer, 128 completed notes, four proposed
voices, 128 KiB candidate/back-pointer storage, and the 50 ms reference-host target for a 128-note phrase. Corpus
admission requires at least 59/63 top-one voice-label matches after label permutation and 176/176 grouping-pair matches.
The four crossing-texture corrections are not waived: the intended crossing must occur in top five or be recoverable by
one local crossing/voice constraint that leaves unaffected candidate bytes identical. Key-release duration accuracy must
not regress below 47/58, and pedal-extended sound is never scored as a written end.

The ASAP adapter's 5,530 anchor-normalized matched notes are clock evidence only, not a voice/spelling ground truth.
Keep the adapter exact-pinned and optional; no CC BY-NC-SA event or learned parameter enters the repository/build.

## Target

- Private bounded polyphonic completion over prompt 204a candidates and one narrow project proposal/report operation.
- Exact source previews and derivations for monophonic melody, block/rolled chords, two-hand piano texture, crossing
  voices, repeated notes, pedal, tuplets, syncopation, chromatic/atonal spelling, grace-like gestures, and mixed rests.
- Grouping/voice/spelling/duration alternative constraints with locality laws: constraining one marked ambiguity leaves
  unaffected candidate regions byte-identical.
- Corpus thresholds for chord/voice/pitch/note-end accuracy and review-correction count, plus deterministic and bounded
  adversarial tests.

## Check

```sh
cargo nextest run -p musa-syntax -p musa-compiler -p musa-project
cargo clippy --all-targets -p musa-syntax -p musa-compiler -p musa-project -- -D warnings
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
cargo insta test --workspace --unreferenced=reject
cargo bench -p musa-project --bench transcription_trial
```

Commit as `Complete polyphonic MIDI transcription proposals`.

## Stop

- No Review UI, acceptance/source mutation, batch editing, dynamics/articulation/pedal insertion by default, or
  step-entry deletion.
- No tonal, SATB, piano-hand, no-crossing, equal-temperament, or common-practice assumption disguised as a universal
  law.
- No mutable proposal AST, unbounded voice count, learned model, or frontend musical inference.
