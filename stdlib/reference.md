# Musa standard library 1

This reference is generated from the source comments in the bundled `.musa` modules. Standard definitions are ordinary
Musa definitions; importing a module is explicit and never searches the filesystem.

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
