# Keyboard capture, not step entry

**Status:** decision record for prompt 201. This page governs nothing; the current rule is
[`docs/rules/desktop/10-keyboard-composition.md`](../../rules/desktop/10-keyboard-composition.md).

## The concrete failure

The finished prompt-33 path treats a MIDI keyboard as half of a typing mode. The musician presses `N`, chooses a
duration on the computer, plays one pitch or a cluster, and receives one source insertion after a fixed grouping window.
The pitch comes from the instrument, but duration, accidental policy, octave state, position, and mode come from the
screen. A chord is whatever arrived within 40 ms. Velocity is discarded; releases, pedals, pressure, bend, rubato, and
phrasing are absent. The player must decide notation before hearing the phrase as a phrase.

That interaction fails the workflow it claims to optimize. A musician who already knows the exact duration and spelling
can state them more directly in Musa source. A musician who is discovering the phrase needs to play continuously and
judge it afterward. Step entry occupies the uncomfortable middle: more divided attention than typing, less musical
evidence than performance.

The current examples make the failure concrete. A tied or syncopated phrase cannot be entered as performed because a key
press has one active duration. A triplet must be declared before its notes. `examples/hemiola.musa` cannot infer its two
groupings from one 40 ms chord window. `examples/cadenza.musa` has exact written durations but no bar grid, while a free
performance supplies neither those values nor a meter. Repeated notes under sustain cannot be reconstructed after
velocity, release, and pedal are discarded. These are not rare parser cases: they are the distinctions Open Music
Theory's chapters `009-notating-rhythm.md`, `010-simple-meter-and-time-signatures.md`,
`011-compound-meter-and-time-signatures.md`, `012-other-rhythmic-essentials.md`,
`098-twentieth-century-rhythmic-techniques.md`, `117-hypermeter.md`, and `118-metrical-dissonance.md` require a readable
rhythmic notation to express.

## The replacement in plain language

A connected keyboard always plays the selected instrument and changes nothing. **Capture** deliberately keeps one finite
expressive MIDI take. **Keep that** freezes a bounded, disclosed, memory-only recent phrase. **Review** proposes written
music, marks only real ambiguities, accepts taps and musical constraints, supports high-leverage selection edits, and
shows the exact source it would write. **Accept** applies one source transaction; **Discard** applies none. Exact
deliberate notation is written directly in Musa source.

This separation follows the problem rather than a familiar editor convention:

- Listen answers *what does this instrument do when I play?*
- Capture answers *which finite performance evidence should be considered?*
- Review answers *which written interpretation do I mean?*
- Accept answers *may this exact source edit become canonical?*

No state tries to answer two of those questions by hiding one.

## Why transcription is not nearest-grid DSP

MIDI has already performed onset detection and pitch sensing. It supplies discrete note numbers, key transitions, and
controllers—not written pitch, voice, metric position, or notated duration. Audio FFTs, spectrograms, source separation,
and waveform editing therefore solve the wrong problem.

The remaining problem is structured symbolic inference. Cemgil, Kappen, Desain, and Honing's
[rhythm-quantization model](https://doi.org/10.1162/014892600559218) balances timing fit against notation complexity;
Nakamura, Itoyama, and Yoshii's
[polyphonic MIDI transcription](https://eita-nakamura.github.io/articles/Nakamura_etal_RhythmTranscriptionOfPolyphonicMIDIPerformances_SMC2016.pdf)
models voices and rhythm jointly; Madsen and Widmer's
[voice-separation work](https://www.cp.jku.at/research/papers/Madsen_Widmer_ISMIR_2006.pdf) treats pitch proximity and
continuity as costs rather than prohibiting crossings. These sources do not choose Musa's algorithm. They refute three
unsafe defaults: rounding each event independently, deciding chords by one fixed time window, and treating the closest
voice as a law.

Prompt 203 therefore measures independent rounding, bounded dynamic programming, Bayesian/HMM-shaped models, and current
learned work before prompts 204–205 fix production structure. The intended path is deterministic bounded top-K search
with an exact cost explanation and musician-supplied constraints. A tap on pulse or downbeat is often a better interface
than another layer of inference: it supplies the missing fact directly and keeps uncertainty local.

## What changes and what does not

The amendment changes the roadmap's product path and note-entry section; desktop thesis, interaction, state, budget, and
new keyboard-composition rules; the language ownership boundary for transcription policy; and the descriptive code map.
The implementation cone is prompts 202–209.

The constitution does not change. Source remains canonical, MIDI remains an edge representation, written pitch remains
different from MIDI number, notated duration remains different from performed duration, and finite capture is not an
audio history. No stored `.musa`, package, lockfile, project, cache, recovery copy, or public semantic encoding
migrates.

The implementation migration is intentionally delayed. Prompt 33's `MidiInputEvent`, `MidiEntry`, `EntryBuffer`,
`NoteEntry`, `N` mode, active duration/octave/accidental state, and fixed chord window remain as temporary code while
the replacement is built. Prompt 209 deletes them only after the full path passes. New public APIs arrive only with
their callers in prompts 202–208; no compatibility wrapper preserves step entry afterward. Existing source files are
unchanged because step entry never had source syntax.

## The rejected rule

The rejected rule was: *score entry is keyboard-first and deterministic; choose duration on the computer and pitch on
the MIDI keyboard, grouping nearby presses into a chord.* It was chosen because notation editors commonly bind `N` to
entry and because one pitch plus one active duration maps cheaply to `InsertNote`.

Cheap implementation was not ergonomic evidence. The rule split one musical gesture across two devices, turned
expressive evidence into discarded input, and required a hidden persistent state so ordinary letters and numbers could
change meaning. Keeping it alongside Capture would leave two overlapping ways to turn a keyboard gesture into source,
with different spelling, chord, duration, and undo semantics. The amendment therefore replaces it rather than adding a
second workflow. The temporary coexistence ends at prompt 209 and is migration, not product plurality.
