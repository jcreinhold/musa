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

## `std::core`

- `identity_ratio(value: ratio) -> ratio` — Return an exact rational unchanged. This is useful when a public API wants to say explicitly that it preserves a proportion.
- `identity_nat(value: nat) -> nat` — Return a natural number unchanged.
- `compose_music(first: music -> music, second: music -> music, value: music) -> music` — Apply the second musical transformation, then the first.
- `compose_pitch(first: pitch -> pitch, second: pitch -> pitch, value: pitch) -> pitch` — Apply the second pitch function, then the first.

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
