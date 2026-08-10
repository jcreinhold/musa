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

## `std::pcset`

- `pc(number: nat) -> pc12` — The pitch class a number names, reduced modulo twelve. `pc(13)` and `pc(1)` are one pitch class, because they are one residue.
- `pcs(numbers: list[nat]) -> list[pc12]` — The pitch classes a list of numbers names, each reduced modulo twelve. A row or a set is written as its numbers, because that is what this domain has instead of letters.
- `number_of(member: pc12) -> nat` — The canonical representative, zero through eleven.
- `forget_spelling(spelled: pitchclass) -> pc12` — Forget a spelling. This is the only total map from the spelled domain into this one; it is not injective, and it has no inverse without a policy.
- `spelled_in(member: pc12, collection: scale) -> option[pitchclass]` — Spell a pitch class inside one collection — the explicit policy that `forget_spelling` has no inverse without. Absent when the collection holds no note of this pitch class.
- `transposed_by(index: nat, member: pc12) -> pc12` — T_n: transposition by n semitones, `x + n` modulo twelve. The index comes first so that `transposed_by(3)` is the transposition itself, a `pc12 -> pc12` that `map_pc` can carry across a list.
- `inverted_about(index: nat, member: pc12) -> pc12` — I_n: inversion about n, `n - x` modulo twelve. I_0 is the plain mirror through zero. The twelve transpositions and the twelve inversions are together the whole 24-element affine group on `pc12` — and 24 is the number, whatever a row's four form labels might suggest.
- `map_pc(function: pc12 -> pc12, members: list[pc12]) -> list[pc12]` — Apply one pitch-class function to every member of a finite list.
- `pcset(members: list[pc12]) -> pcset12` — The set of everything listed, however often it was listed. A set cannot hold a duplicate, so this cannot fail: a repetition is a mistake only where order matters, which is `std::serial`.
- `members_of(set: pcset12) -> list[pc12]` — The members, ascending from zero. This is the set's own order and not its normal order.
- `set_transposed(set: pcset12, index: nat) -> pcset12` — T_n applied to every member.
- `set_inverted(set: pcset12, index: nat) -> pcset12` — I_n applied to every member.
- `normal_order(set: pcset12) -> list[pc12]` — Normal order: the rotation of the ascending members packed most tightly to the left. Ties break inward — first to last, then first to the one before last, and so on — and finally by the lowest starting pitch class.
- `prime_form(set: pcset12) -> pcset12` — Prime form: the set class this set belongs to. The normal orders of the set and of its inversion are each transposed to begin on zero, and whichever reads lower is the answer.
- `interval_class_vector(set: pcset12) -> list[nat]` — The interval-class vector: six counts, for interval classes one through six. Six and not twelve, because interval class seven is interval class five heard the other way round.

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

## `std::serial`

- `row(pcs: list[pc12]) -> option[row12]` — The row a sequence spells, or nothing when the sequence is not one.
- `repeated_positions(pcs: list[pc12]) -> list[nat]` — The order positions whose pitch class already appeared earlier — the exact reason a sequence failed to be a row. The first occurrence is not among them, because that is where the pitch class belongs.
- `missing_classes(pcs: list[pc12]) -> list[pc12]` — The pitch classes a sequence never names, ascending — the other exact reason. A sequence of the right length has one of these lists empty exactly when it has the other empty.
- `pcs_of(series: row12) -> list[pc12]` — The row's pitch classes, in order position order.
- `transposed(series: row12, index: nat) -> row12` — P: transposition by n semitones, order positions untouched.
- `inverted(series: row12, index: nat) -> row12` — I: inversion about n, order positions untouched.
- `retrograde_of(series: row12) -> row12` — R: the order positions reversed, pitch classes untouched. An involution, and it commutes with P and I because it acts on the other side of the row.
- `retrograde_inversion_of(series: row12, index: nat) -> row12` — RI: the retrograde of the inversion, which is also the inversion of the retrograde. Writing it both ways and getting one row is what "commutes" means here.
- `matrix(series: row12) -> list[row12]` — The twelve-tone matrix, as twelve rows. Row zero is the row as written; row i is the transposition beginning on the ith pitch class of the inversion about the row's own head, so every column read downward is an inversion. The construction fixes no naming convention, because the rows are rows and not labels: which transposition is called P0 is the question the two functions below answer, differently and by name.
- `fixed_zero_index(series: row12) -> nat` — The transposition index under the fixed-zero convention: P0 is the form beginning on pitch class zero, so a row's index is simply the number of the pitch class it begins on.
- `moveable_zero_index(reference: row12, form: row12) -> nat` — The transposition index under the moveable-zero convention: P0 is the row as written, so an index is only meaningful relative to a stated reference row. `moveable_zero_index(reference, form)` is how far the form stands above the reference.
- `distinct_forms(series: row12) -> nat` — How many *distinct* rows the 48 labelled forms produce. Forty-eight for a generic row; fewer for a row some labelled operation fixes.
- `symmetries(series: row12) -> nat` — The order of the row's stabilizer: how many of the 48 labelled operations send the row to itself. This times `distinct_forms` is always 48, which is the orbit-stabilizer accounting the four labels are so often asked to do on their own.
- `row_spelled_in(series: row12, collection: scale) -> list[option[pitchclass]]` — Spell one row inside a collection, position by position. A pitch class the collection cannot spell is absent, and the row keeps its length, so a projection that lost notes is visible as the gaps it left.
- `spelling_in(collection: scale, member: pc12) -> option[pitchclass]` — The spelling policy of one collection, as a function a row can be carried across.
- `first_pc(series: row12) -> pc12` — The pitch class a row begins on. Order position zero always exists, because a row has twelve of them.

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
