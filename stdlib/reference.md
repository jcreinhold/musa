# Musa standard library 1

This reference is generated from the source comments in the bundled `.musa` modules. Standard definitions are ordinary
Musa definitions; importing a module is explicit and never searches the filesystem.

## `std::collections`

- `major_on(tonic: pitchclass) -> scale` — The major collection, W–W–H–W–W–W–H, rooted on tonic.
- `dorian_on(tonic: pitchclass) -> scale` — The Dorian rotation of the diatonic collection.
- `phrygian_on(tonic: pitchclass) -> scale` — The Phrygian rotation.
- `lydian_on(tonic: pitchclass) -> scale` — The Lydian rotation.
- `mixolydian_on(tonic: pitchclass) -> scale` — The Mixolydian rotation.
- `natural_minor_on(tonic: pitchclass) -> scale` — The natural minor collection, which is also the Aeolian rotation.
- `locrian_on(tonic: pitchclass) -> scale` — The Locrian rotation.
- `harmonic_minor_on(tonic: pitchclass) -> scale` — Harmonic minor: natural minor with a raised seventh degree. A distinct collection, not a spelling of the natural one.
- `melodic_minor_on(tonic: pitchclass) -> scale` — Ascending melodic minor: raised sixth and seventh degrees.
- `descending_melodic_minor_on(tonic: pitchclass) -> scale` — Descending melodic minor, whose offsets are the natural minor's and whose identity is not.
- `major_pentatonic_on(tonic: pitchclass) -> scale` — The five-note major pentatonic collection.
- `minor_pentatonic_on(tonic: pitchclass) -> scale` — The five-note minor pentatonic collection.
- `whole_tone_on(tonic: pitchclass) -> scale` — The six-note whole-tone collection.
- `octatonic_half_whole_on(tonic: pitchclass) -> scale` — The eight-note octatonic collection beginning with a half step.
- `octatonic_whole_half_on(tonic: pitchclass) -> scale` — The eight-note octatonic collection beginning with a whole step.
- `hexatonic_on(tonic: pitchclass) -> scale` — The six-note hexatonic collection alternating minor thirds and half steps.
- `acoustic_on(tonic: pitchclass) -> scale` — The acoustic collection: raised fourth and lowered seventh.

## `std::context`

- `signature TonalContext` — A tonal context is the small bundle of facts that always travel together: what key a passage is in, which collection it steps through, how a numbered degree is spelled in register, and how a chord class is voiced. Passing them one at a time is how they drift apart, which is the whole reason a signature exists — Open Music Theory `020-diatonic-modes.md` names the collection, `026-triads.md` the chords, and neither is meaningful without the other.
  - `TonalContext.tonic: key` — The key the passage is written in. A key is a signature and a tonic, never a scale.
  - `TonalContext.collection: scale` — The collection stepwise motion reads. A default, not a claim: a passage may still name another collection where it wants one.
  - `TonalContext.spell: nat -> option[pitch]` — The written pitch a numbered degree names, in this context's own register. Absent when the context has no register to spell in.
  - `TonalContext.voicing_for: chord_class -> option[voicing]` — How this context voices a chord class. Absent when the class cannot be voiced from the register it chose.
- `module CMajor: TonalContext` — C major, spelled from middle C.
- `module ANaturalMinor: TonalContext` — A natural minor, spelled from the A below middle C. The same four members, answered differently — which is what makes the two modules interchangeable everywhere `TonalContext` is asked for.

## `std::core`

- `identity_ratio(value: ratio) -> ratio` — Return an exact rational unchanged. This is useful when a public API wants to say explicitly that it preserves a proportion.
- `identity_nat(value: nat) -> nat` — Return a natural number unchanged.
- `compose_music(first: music -> music, second: music -> music, value: music) -> music` — Apply the second musical transformation, then the first.
- `compose_pitch(first: pitch -> pitch, second: pitch -> pitch, value: pitch) -> pitch` — Apply the second pitch function, then the first.

## `std::harmony`

- `chord_rooted_on(content: chord_class, root: pitchclass) -> chord_class` — Re-root a chord class, keeping its type. `chord c major7` on `eb` is an E-flat major seventh, spelled from E-flat.
- `root_of(content: chord_class) -> pitchclass` — The pitch class a chord class is rooted on. This is the root, which is not the bass: a designated bass is asked for separately.
- `bass_of(content: chord_class) -> option[pitchclass]` — The bass a chord class designates, when it designates one. Absent means no bass was chosen — it does not mean the root.
- `members_of(content: chord_class) -> list[interval]` — The spelled intervals above the root, lowest first, beginning at the unison. Spelled: a major third is a third, never a diminished fourth.
- `inversion(content: chord_class, position: nat) -> option[chord_class]` — A true inversion: the numbered member becomes the designated bass. Positions are counted from zero, so position one is first inversion. Absent when the class has no such member.
- `slash_bass(content: chord_class, bass: pitchclass) -> chord_class` — A slash bass: a designated bass that need not be a member at all. `chord_over(chord c major, d)` is C over D, and the D is not a chord tone. This is a different construction from an inversion, and stays one.
- `as_triad(content: chord_class) -> option[triad]` — The triad refinement, when the content really is a major or minor triad. This is the domain a neo-Riemannian transformation acts on, and the proof that it applies is this `option` being present.
- `triad_content(refined: triad) -> chord_class` — Forget the refinement: every triad is a chord class.
- `is_triad(content: chord_class) -> bool` — Whether a chord class is a major or minor triad.
- `triad_is_present(refined: triad) -> bool` — The present case of `is_triad`: a refinement that exists is a triad, whichever of the two it turned out to be.

## `std::list`

- `naturals(count: nat) -> list[nat]` — The natural numbers from zero up to, but not including, count.
- `map_pitches(function: pitch -> pitch, values: list[pitch]) -> list[pitch]` — Apply one pitch function to every member of a finite pitch list.
- `filter_pitches(predicate: pitch -> bool, values: list[pitch]) -> list[pitch]` — Keep the pitches for which predicate returns true.
- `repeat_music(value: music, count: nat) -> list[music]` — Repeat one contextual music value count times as finite data.

## `std::option`

- `pitch_or_else(fallback: pitch, present: pitch -> pitch, value: option[pitch]) -> pitch` — Read an optional pitch, using fallback when it is absent and present when it is available.
- `nat_or_else(fallback: nat, present: nat -> nat, value: option[nat]) -> nat` — Read an optional natural number under the same explicit policy.

## `std::pitch`

- `unison: interval` — Open Music Theory `016-intervals.md` supplies the conventional generic/specific interval names; `005-half-steps-whole-steps-and-accidentals.md` supplies the spelling distinction retained by these values. The written unison has no staff displacement and no chromatic displacement.
- `minor_second: interval` — The chromatic semitone spelled as a minor second.
- `major_second: interval` — The diatonic tone spelled as a major second.
- `perfect_fourth: interval` — The perfect fourth.
- `perfect_fifth: interval` — The perfect fifth.
- `octave: interval` — The written octave, which changes both coordinates by (7, 12).
- `compose_intervals(first: interval, second: interval) -> interval` — Apply two spelling-preserving written intervals in sequence.
- `inverse_interval(value: interval) -> interval` — Reverse the direction of a written interval.
- `written_pitch_class(value: pitch) -> pitchclass` — Forget octave while retaining the written letter and accidental.

## `std::scale`

- `tonic_degree: degree` — The first degree of any scale. Degrees are written from one, as musicians write them.
- `key_scale(written: key) -> scale` — The scale a key's signature suggests for stepwise motion. It is a default, not a claim: `key c minor` fixes three flats, and a passage may still ask for the harmonic or melodic collection by name.
- `scale_root(collection: scale) -> pitchclass` — The tonic pitch class a scale is rooted on.
- `scale_degrees(collection: scale) -> nat` — How many degrees one period of a scale holds.
- `degree_in(collection: scale, written: pitch) -> option[degree]` — The degree a written pitch occupies, when it occupies one. Membership is spelled: `eb5` and `d#5` answer differently.
- `belongs_to(collection: scale, written: pitch) -> bool` — Whether a written pitch belongs to a scale at all.
- `degree_is_present(located: degree) -> bool` — The present case of `belongs_to`: a located degree means the pitch is a member, whichever degree it turned out to be.
- `frame_on(collection: scale, root: pitch) -> option[frame]` — The register frame a scale takes on one absolute tonic pitch. It is absent when that pitch is not the scale's tonic class.
- `frame_degree(register: frame, ordinal: nat) -> pitch` — The written pitch a numbered degree names in one register frame.
- `frame_triad(register: frame) -> list[pitch]` — The tonic, third, and fifth degrees of a frame, in register.
- `up_steps(from: degree, steps: nat) -> degree` — Move a degree up by a whole number of scale steps.
- `down_steps(from: degree, steps: nat) -> degree` — Move a degree down by a whole number of scale steps.
- `raise(from: degree) -> degree` — Raise a degree chromatically without moving its coordinate.
- `lower(from: degree) -> degree` — Lower a degree chromatically without moving its coordinate.

## `std::voicing`

- `close_position(content: chord_class, bass: pitch) -> option[voicing]` — Stack the class upward from an absolute bass, one member per octave position, taking each next member at the first pitch above the last. This is the one policy `stack c4 major7/2` desugars to.
- `drop_position(content: chord_class, bass: pitch, from_top: nat) -> option[voicing]` — Close position with one upper note dropped an octave, counted from the top: `drop_position(content, bass, 2)` is the drop-2 voicing.
- `voiced_as(content: chord_class, pitches: list[pitch]) -> option[voicing]` — The voicing an explicit list of written pitches spells, when those pitches really do voice the class: ascending, distinct, and every one a member. This is how a writer voices by hand.
- `pitches_of(chosen: voicing) -> list[pitch]` — Every sounding pitch of a voicing, lowest first.
- `lowest_of(chosen: voicing) -> pitch` — The lowest sounding pitch. A voicing always has one, which is why this is not an `option`.
- `chord_of(chosen: voicing) -> chord_class` — The chord class this voicing voices.
- `inversion_of(chosen: voicing) -> option[nat]` — Which member is in the bass, counted from zero, when the bass is a member at all. Absent for a slash bass, which is not an inversion.
- `omitting(chosen: voicing, position: nat) -> option[voicing]` — Drop one numbered member from a voicing, keeping the class it voices. The chord class is unchanged: an omission is a choice about what sounds, not a claim that the chord is a different chord.
- `rootless(chosen: voicing) -> option[voicing]` — The rootless voicing a pianist plays under a bass player: the root, in position zero, is the note removed, and it is named here rather than left implicit.
- `sound_for(chosen: voicing, held: duration) -> music` — Sound a chosen voicing for a written length. This is the only way a chord class becomes notes.
