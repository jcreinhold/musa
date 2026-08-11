# Musa standard library 1

This reference is generated from the source comments in the bundled `.musa` modules. Standard definitions are ordinary
Musa definitions; importing a module is explicit and never searches the filesystem.

## `std::collections`

- `major_on(tonic: NoteName) -> Scale { scale_on(scale c major, tonic) }` — The major collection, W–W–H–W–W–W–H, rooted
  on tonic.
- `dorian_on(tonic: NoteName) -> Scale { scale_on(scale c dorian, tonic) }` — The Dorian rotation of the diatonic
  collection.
- `phrygian_on(tonic: NoteName) -> Scale { scale_on(scale c phrygian, tonic) }` — The Phrygian rotation.
- `lydian_on(tonic: NoteName) -> Scale { scale_on(scale c lydian, tonic) }` — The Lydian rotation.
- `mixolydian_on(tonic: NoteName) -> Scale { scale_on(scale c mixolydian, tonic) }` — The Mixolydian rotation.
- `natural_minor_on(tonic: NoteName) -> Scale { scale_on(scale c natural_minor, tonic) }` — The natural minor
  collection, which is also the Aeolian rotation.
- `locrian_on(tonic: NoteName) -> Scale { scale_on(scale c locrian, tonic) }` — The Locrian rotation.
- `harmonic_minor_on(tonic: NoteName) -> Scale { scale_on(scale c harmonic_minor, tonic) }` — Harmonic minor: natural
  minor with a raised seventh degree. A distinct collection, not a spelling of the natural one.
- `melodic_minor_on(tonic: NoteName) -> Scale { scale_on(scale c melodic_minor, tonic) }` — Ascending melodic minor:
  raised sixth and seventh degrees.
- `descending_melodic_minor_on(tonic: NoteName) -> Scale {` — Descending melodic minor, whose offsets are the natural
  minor's and whose identity is not.
- `major_pentatonic_on(tonic: NoteName) -> Scale {` — The five-note major pentatonic collection.
- `minor_pentatonic_on(tonic: NoteName) -> Scale {` — The five-note minor pentatonic collection.
- `whole_tone_on(tonic: NoteName) -> Scale { scale_on(scale c whole_tone, tonic) }` — The six-note whole-tone
  collection.
- `octatonic_half_whole_on(tonic: NoteName) -> Scale {` — The eight-note octatonic collection beginning with a half
  step.
- `octatonic_whole_half_on(tonic: NoteName) -> Scale {` — The eight-note octatonic collection beginning with a whole
  step.
- `hexatonic_on(tonic: NoteName) -> Scale { scale_on(scale c hexatonic, tonic) }` — The six-note hexatonic collection
  alternating minor thirds and half steps.
- `acoustic_on(tonic: NoteName) -> Scale { scale_on(scale c acoustic, tonic) }` — The acoustic collection: raised fourth
  and lowered seventh.

## `std::context`

- `signature TonalContext` — A tonal context is the small bundle of facts that always travel together: what key a
  passage is in, which collection it steps through, how a numbered degree is spelled in register, and how a chord class
  is voiced. Passing them one at a time is how they drift apart, which is the whole reason a signature exists — Open
  Music Theory `020-diatonic-modes.md` names the collection, `026-triads.md` the chords, and neither is meaningful
  without the other.
  - `TonalContext.tonic: Key` — The key the passage is written in. A key is a signature and a tonic, never a scale.
  - `TonalContext.collection: Scale` — The collection stepwise motion reads. A default, not a claim: a passage may still
    name another collection where it wants one.
  - `TonalContext.spell: Nat -> Option[Pitch]` — The written pitch a numbered degree names, in this context's own
    register. Absent when the context has no register to spell in.
  - `TonalContext.voicing_for: ChordClass -> Option[Voicing]` — How this context voices a chord class. Absent when the
    class cannot be voiced from the register it chose.
- `structure CMajor: TonalContext` — C major, spelled from middle C.
- `voicing_for(content: ChordClass) -> Option[Voicing] { close_position(content, c4) }` — 
- `structure ANaturalMinor: TonalContext` — A natural minor, spelled from the A below middle C. The same four members,
  answered differently — which is what makes the two structures interchangeable everywhere `TonalContext` is asked for.
- `voicing_for(content: ChordClass) -> Option[Voicing] { close_position(content, a3) }` — 

## `std::core`

- `identity_ratio(value: Ratio) -> Ratio { value }` — Return an exact rational unchanged. This is useful when a public
  API wants to say explicitly that it preserves a proportion.
- `identity_nat(value: Nat) -> Nat { value }` — Return a natural number unchanged.
- `compose_music(first: Music -> Music, second: Music -> Music, value: Music) -> Music {` — Apply the second musical
  transformation, then the first.
- `compose_pitch(first: Pitch -> Pitch, second: Pitch -> Pitch, value: Pitch) -> Pitch {` — Apply the second pitch
  function, then the first.

## `std::harmony`

- `chord_rooted_on(content: ChordClass, root: NoteName) -> ChordClass {` — Re-root a chord class, keeping its type.
  `chord c major7` on `eb` is an E-flat major seventh, spelled from E-flat.
- `root_of(content: ChordClass) -> NoteName { chord_root(content) }` — The pitch class a chord class is rooted on. This
  is the root, which is not the bass: a designated bass is asked for separately.
- `bass_of(content: ChordClass) -> Option[NoteName] { chord_bass(content) }` — The bass a chord class designates, when
  it designates one. Absent means no bass was chosen — it does not mean the root.
- `members_of(content: ChordClass) -> List[Interval] { chord_members(content) }` — The spelled intervals above the root,
  lowest first, beginning at the unison. Spelled: a major third is a third, never a diminished fourth.
- `inversion(content: ChordClass, position: Nat) -> Option[ChordClass] {` — A true inversion: the numbered member
  becomes the designated bass. Positions are counted from zero, so position one is first inversion. Absent when the
  class has no such member.
- `slash_bass(content: ChordClass, bass: NoteName) -> ChordClass {` — A slash bass: a designated bass that need not be a
  member at all. `chord_over(chord c major, d)` is C over D, and the D is not a chord tone. This is a different
  construction from an inversion, and stays one.
- `as_triad(content: ChordClass) -> Option[Triad] { chord_triad(content) }` — The triad refinement, when the content
  really is a major or minor triad. This is the domain a neo-Riemannian transformation acts on, and the proof that it
  applies is this `option` being present.
- `triad_content(refined: Triad) -> ChordClass { triad_chord(refined) }` — Forget the refinement: every triad is a chord
  class.
- `is_triad(content: ChordClass) -> Bool {` — Whether a chord class is a major or minor triad.
- `triad_is_present(refined: Triad) -> Bool { true }` — The present case of `is_triad`: a refinement that exists is a
  triad, whichever of the two it turned out to be.
- `is_major(refined: Triad) -> Bool { triad_major(refined) }` — Which of the two a triad is. Total, and a `bool` rather
  than a partial answer, because the refinement admitted exactly two chord classes: not major is minor here, and only
  here. Every transformation in `std::transformational` branches on this, since which way a voice moves is the whole
  content of the transformation.

## `std::list`

- `naturals(count: Nat) -> List[Nat] { range(count) }` — The natural numbers from zero up to, but not including, count.
- `map_pitches(function: Pitch -> Pitch, values: List[Pitch]) -> List[Pitch] {` — Apply one pitch function to every
  member of a finite pitch list.
- `filter_pitches(predicate: Pitch -> Bool, values: List[Pitch]) -> List[Pitch] {` — Keep the pitches for which
  predicate returns true.
- `repeat_music(value: Music, count: Nat) -> List[Music] { repeat(value, count) }` — Repeat one contextual music value
  count times as finite data.

## `std::option`

- `pitch_or_else(fallback: Pitch, present: Pitch -> Pitch, value: Option[Pitch]) -> Pitch {` — Read an optional pitch,
  using fallback when it is absent and present when it is available.
- `nat_or_else(fallback: Nat, present: Nat -> Nat, value: Option[Nat]) -> Nat {` — Read an optional natural number under
  the same explicit policy.

## `std::pitch`

- `unison: Interval` — Open Music Theory `016-intervals.md` supplies the conventional generic/specific interval names;
  `005-half-steps-whole-steps-and-accidentals.md` supplies the spelling distinction retained by these values. The
  written unison has no staff displacement and no chromatic displacement.
- `minor_second: Interval` — The chromatic semitone spelled as a minor second.
- `major_second: Interval` — The diatonic tone spelled as a major second.
- `perfect_fourth: Interval` — The perfect fourth.
- `perfect_fifth: Interval` — The perfect fifth.
- `octave: Interval` — The written octave, which changes both coordinates by (7, 12).
- `compose_intervals(first: Interval, second: Interval) -> Interval {` — Apply two spelling-preserving written intervals
  in sequence.
- `inverse_interval(value: Interval) -> Interval { interval_inverse(value) }` — Reverse the direction of a written
  interval.
- `written_pitch_class(value: Pitch) -> NoteName { pitchclass_of(value) }` — Forget octave while retaining the written
  letter and accidental.

## `std::post_tonal::pcset`

- `pc(number: Nat) -> Pc12 { pc12_of(number) }` — The pitch class a number names, reduced modulo twelve. `pc(13)` and
  `pc(1)` are one pitch class, because they are one residue.
- `pcs(numbers: List[Nat]) -> List[Pc12] { map(pc, numbers) }` — The pitch classes a list of numbers names, each reduced
  modulo twelve. A row or a set is written as its numbers, because that is what this domain has instead of letters.
- `number_of(member: Pc12) -> Nat { pc12_number(member) }` — The canonical representative, zero through eleven.
- `forget_spelling(spelled: NoteName) -> Pc12 { pc12_forget(spelled) }` — Forget a spelling. This is the only total map
  from the spelled domain into this one; it is not injective, and it has no inverse without a policy.
- `spelled_in(member: Pc12, collection: Scale) -> Option[NoteName] {` — Spell a pitch class inside one collection — the
  explicit policy that `forget_spelling` has no inverse without. Absent when the collection holds no note of this pitch
  class.
- `transposed_by(index: Nat, member: Pc12) -> Pc12 { pc12_transposed(member, index) }` — T_n: transposition by n
  semitones, `x + n` modulo twelve. The index comes first so that `transposed_by(3)` is the transposition itself, a
  `pc12 -> pc12` that `map_pc` can carry across a list.
- `inverted_about(index: Nat, member: Pc12) -> Pc12 { pc12_inverted(member, index) }` — I_n: inversion about n, `n - x`
  modulo twelve. I_0 is the plain mirror through zero. The twelve transpositions and the twelve inversions are together
  the whole 24-element affine group on `pc12` — and 24 is the number, whatever a row's four form labels might suggest.
- `map_pc(function: Pc12 -> Pc12, members: List[Pc12]) -> List[Pc12] {` — Apply one pitch-class function to every member
  of a finite list.
- `pcset(members: List[Pc12]) -> PcSet12 { pcset12_of(members) }` — The set of everything listed, however often it was
  listed. A set cannot hold a duplicate, so this cannot fail: a repetition is a mistake only where order matters, which
  is `std::post_tonal::serial`.
- `set_members(set: PcSet12) -> List[Pc12] { pcset12_members(set) }` — The members, ascending from zero. This is the
  set's own order and not its normal order. Named for the set rather than `members_of`, because `std::harmony` already
  reads the members of a chord class and a piece that reasons about both must be able to import both.
- `set_transposed(set: PcSet12, index: Nat) -> PcSet12 {` — T_n applied to every member.
- `set_inverted(set: PcSet12, index: Nat) -> PcSet12 {` — I_n applied to every member.
- `normal_order(set: PcSet12) -> List[Pc12] { pcset12_normal(set) }` — Normal order: the rotation of the ascending
  members packed most tightly to the left. Ties break inward — first to last, then first to the one before last, and so
  on — and finally by the lowest starting pitch class.
- `prime_form(set: PcSet12) -> PcSet12 { pcset12_prime(set) }` — Prime form: the set class this set belongs to. The
  normal orders of the set and of its inversion are each transposed to begin on zero, and whichever reads lower is the
  answer.
- `interval_class_vector(set: PcSet12) -> List[Nat] { pcset12_vector(set) }` — The interval-class vector: six counts,
  for interval classes one through six. Six and not twelve, because interval class seven is interval class five heard
  the other way round.

## `std::post_tonal::serial`

- `row(pcs: List[Pc12]) -> Option[Row12] { row12_of(pcs) }` — The row a sequence spells, or nothing when the sequence is
  not one.
- `repeated_positions(pcs: List[Pc12]) -> List[Nat] { row12_repeats(pcs) }` — The order positions whose pitch class
  already appeared earlier — the exact reason a sequence failed to be a row. The first occurrence is not among them,
  because that is where the pitch class belongs.
- `missing_classes(pcs: List[Pc12]) -> List[Pc12] { row12_missing(pcs) }` — The pitch classes a sequence never names,
  ascending — the other exact reason. A sequence of the right length has one of these lists empty exactly when it has
  the other empty.
- `pcs_of(series: Row12) -> List[Pc12] { row12_pcs(series) }` — The row's pitch classes, in order position order.
- `transposed(series: Row12, index: Nat) -> Row12 { row12_transposed(series, index) }` — P: transposition by n
  semitones, order positions untouched.
- `inverted(series: Row12, index: Nat) -> Row12 { row12_inverted(series, index) }` — I: inversion about n, order
  positions untouched.
- `retrograde_of(series: Row12) -> Row12 { row12_retrograde(series) }` — R: the order positions reversed, pitch classes
  untouched. An involution, and it commutes with P and I because it acts on the other side of the row.
- `retrograde_inversion_of(series: Row12, index: Nat) -> Row12 {` — RI: the retrograde of the inversion, which is also
  the inversion of the retrograde. Writing it both ways and getting one row is what "commutes" means here.
- `matrix(series: Row12) -> List[Row12] { row12_matrix(series) }` — The twelve-tone matrix, as twelve rows. Row zero is
  the row as written; row i is the transposition beginning on the ith pitch class of the inversion about the row's own
  head, so every column read downward is an inversion. The construction fixes no naming convention, because the rows are
  rows and not labels: which transposition is called P0 is the question the two functions below answer, differently and
  by name.
- `fixed_zero_index(series: Row12) -> Nat { pc12_number(first_pc(series)) }` — The transposition index under the
  fixed-zero convention: P0 is the form beginning on pitch class zero, so a row's index is simply the number of the
  pitch class it begins on.
- `moveable_zero_index(reference: Row12, form: Row12) -> Nat {` — The transposition index under the moveable-zero
  convention: P0 is the row as written, so an index is only meaningful relative to a stated reference row.
  `moveable_zero_index(reference, form)` is how far the form stands above the reference.
- `distinct_forms(series: Row12) -> Nat { row12_forms(series) }` — How many *distinct* rows the 48 labelled forms
  produce. Forty-eight for a generic row; fewer for a row some labelled operation fixes.
- `symmetries(series: Row12) -> Nat { row12_symmetries(series) }` — The order of the row's stabilizer: how many of the
  48 labelled operations send the row to itself. This times `distinct_forms` is always 48, which is the orbit-stabilizer
  accounting the four labels are so often asked to do on their own.
- `row_spelled_in(series: Row12, collection: Scale) -> List[Option[NoteName]] {` — Spell one row inside a collection,
  position by position. A pitch class the collection cannot spell is absent, and the row keeps its length, so a
  projection that lost notes is visible as the gaps it left.
- `spelling_in(collection: Scale, member: Pc12) -> Option[NoteName] {` — The spelling policy of one collection, as a
  function a row can be carried across.
- `first_pc(series: Row12) -> Pc12 { row12_head(series) }` — The pitch class a row begins on. Order position zero always
  exists, because a row has twelve of them.

## `std::scale`

- `tonic_degree: Degree` — The first degree of any scale. Degrees are written from one, as musicians write them.
- `key_scale(written: Key) -> Scale { signature_scale(written) }` — The scale a key's signature suggests for stepwise
  motion. It is a default, not a claim: `key c minor` fixes three flats, and a passage may still ask for the harmonic or
  melodic collection by name.
- `scale_root(collection: Scale) -> NoteName { scale_tonic(collection) }` — The tonic pitch class a scale is rooted on.
- `scale_degrees(collection: Scale) -> Nat { scale_size(collection) }` — How many degrees one period of a scale holds.
- `degree_in(collection: Scale, written: Pitch) -> Option[Degree] {` — The degree a written pitch occupies, when it
  occupies one. Membership is spelled: `eb5` and `d#5` answer differently.
- `belongs_to(collection: Scale, written: Pitch) -> Bool {` — Whether a written pitch belongs to a scale at all.
- `degree_is_present(located: Degree) -> Bool { true }` — The present case of `belongs_to`: a located degree means the
  pitch is a member, whichever degree it turned out to be.
- `frame_on(collection: Scale, root: Pitch) -> Option[Frame] {` — The register frame a scale takes on one absolute tonic
  pitch. It is absent when that pitch is not the scale's tonic class.
- `frame_degree(register: Frame, ordinal: Nat) -> Pitch {` — The written pitch a numbered degree names in one register
  frame.
- `frame_triad(register: Frame) -> List[Pitch] {` — The tonic, third, and fifth degrees of a frame, in register.
- `degree_class(collection: Scale, ordinal: Nat) -> Option[NoteName] {` — The pitch class a numbered degree names, with
  no register at all. `frame_degree` asks the same thing of a scale that has been given an absolute tonic, and it has to
  be given one, because a written pitch has an octave and something must choose it. A Roman numeral has no octave to
  choose — `V` in C major is the class `g`, and which `g` sounds is the voicing's business. Absent only for an ordinal
  no score can write.
- `altered_class(collection: Scale, altered: Degree) -> Option[NoteName] {` — The same for a degree that already carries
  an alteration, so that a `raise` or `lower` composes into the spelling rather than being lost. This is how a borrowed
  or Neapolitan degree is spelled without a frame.
- `degree_chord(collection: Scale, written: Degree, members: Nat) -> Option[ChordClass] {` — The chord the collection
  stacks in thirds from a degree. `members` counts the notes, so three is a triad and four a seventh chord. The quality
  is the collection's and not the caller's: `ii` is minor in major and `II` is major in Dorian because those are the
  notes there, which is the whole content of the word "diatonic". A degree rather than a number because the degree is
  where this language does ordinal arithmetic, so a succession walked by `up_steps` can be harmonized where a walked
  number could not. Absent for an altered degree, which is not asking for the collection's own chord, and absent when
  the collection stacks to a sonority the chord vocabulary cannot name.
- `degree_triad(collection: Scale, written: Degree) -> Option[ChordClass] {` — The diatonic triad on a degree.
- `degree_seventh(collection: Scale, written: Degree) -> Option[ChordClass] {` — The diatonic seventh chord on a degree.
- `up_steps(from: Degree, steps: Nat) -> Degree { degree_step_up(from, steps) }` — Move a degree up by a whole number of
  scale steps.
- `down_steps(from: Degree, steps: Nat) -> Degree { degree_step_down(from, steps) }` — Move a degree down by a whole
  number of scale steps.
- `raise(from: Degree) -> Degree { degree_raised(from) }` — Raise a degree chromatically without moving its coordinate.
- `lower(from: Degree) -> Degree { degree_lowered(from) }` — Lower a degree chromatically without moving its coordinate.

## `std::tonal::harmony`

- `numeral(ordinal: Nat, members: Nat, position: Nat) -> Option[Roman] {` — The numeral three numbers describe, when
  they describe one. Absent when the ordinal is outside `I`–`vii`, when the stack is smaller than a triad or larger than
  a thirteenth, or when the bass position names a member the stack does not have — a third inversion of a triad is not a
  numeral that is hard to realize, it is not a numeral. OMT 020 and 021.
- `triad_numeral(ordinal: Nat) -> Option[Roman] { roman_of(ordinal, 3, 0) }` — The root-position triad on a degree,
  which is what OMT 020 writes with a bare numeral and no figures.
- `seventh_numeral(ordinal: Nat) -> Option[Roman] { roman_of(ordinal, 4, 0) }` — The root-position seventh chord on a
  degree: OMT 021's `7`.
- `numeral_step(written: Roman) -> Nat { roman_ordinal(written) }` — Which degree the numeral is built on, counted from
  one.
- `numeral_size(written: Roman) -> Nat { roman_size(written) }` — How many members the numeral stacks: three is a triad,
  four a seventh.
- `numeral_bass(written: Roman) -> Nat { roman_inversion(written) }` — Which member the numeral puts in the bass,
  counted from zero, so zero is root position and one is the `6` of figured bass. A position in the stack and not a
  pitch: which note actually sounds lowest is a voicing's business, and `std::voicing` is where that is decided.
- `numeral_chord(collection: Scale, written: Roman) -> Option[ChordClass] {` — The chord a numeral names in a
  collection, spelled by that collection. The quality is never supplied: `ii` is minor in major and `II` is major in
  Dorian because those are the notes there, which is the whole content of the word "diatonic". Absent when the
  collection has no such degree, or when it stacks to a sonority the chord vocabulary cannot name — a harmonic-minor
  `III7` is an augmented major seventh, and reporting the absence is more honest than rounding it to a chord with other
  notes in it. OMT 020 and 021.
- `borrowed(home: Scale, mode: Scale, written: Roman) -> Option[ChordClass] {` — Modal mixture: the numeral realized
  against a borrowed collection on the home tonic. Borrowing needs no altered degrees and no alteration field, because
  that is what the word means — the flat six of C major is the sixth of C minor, and spelling it as a lowered degree
  describes the result rather than the operation. Which collection is borrowed from is the caller's, since parallel
  minor, harmonic minor, and Phrygian all lend chords and none of them is the default. OMT 061.
- `secondary(home: Scale, target: Degree, mode: Scale, written: Roman) -> Option[ChordClass] {` — An applied chord: a
  numeral read in the collection that tonicizes a target degree of the home collection. `V/V` in C major is
  `secondary(home, degree_of(5), major_on(c), five)` — the numeral is read in G, which is why it is D major and not the
  D minor that C major stacks. The target degree is written out, so a tonicized lowered sixth is as sayable as a
  tonicized fifth, and the tonicizing collection is written out, so nothing here decides that an applied chord implies
  major. Tonicization is a local relationship between two chords; whether a passage has modulated is a claim about the
  passage and is not decided here. OMT 050.
- `quality_on_degree(collection: Scale, written: Degree, quality: ChordClass) -> Option[ChordClass] {` — A named chord
  quality rooted on a named degree of a collection. This is how every chromatic sonority below is built: the degree
  carries the alteration, the quality carries the members, and neither is guessed. A Neapolitan in C major names a
  lowered second and in C minor names the plain second, and that difference is the caller's to write, because the minor
  collection was the caller's to choose. Absent when the collection has no such degree.
- `neapolitan(collection: Scale, lowered_second: Degree) -> Option[ChordClass] {` — The Neapolitan: a major triad on the
  lowered second degree. The `6` in its usual name is a first inversion, which is a position and is taken with
  `inversion` rather than baked in here — the chord is a major triad in any position, and OMT 062 says so before it says
  the sixth is idiomatic.
- `italian_sixth(collection: Scale, lowered_sixth: Degree) -> Option[ChordClass] {` — The Italian sixth: a lowered
  sixth, the tonic, and a raised fourth, with no fifth. Its augmented sixth is spelled as an augmented sixth, which is
  the whole reason it is a chord of its own and not a seventh. OMT 063.
- `french_sixth(collection: Scale, lowered_sixth: Degree) -> Option[ChordClass] {` — The French sixth: the Italian sixth
  with the second degree added, which spells as an augmented fourth above the root. OMT 063.
- `german_sixth(collection: Scale, lowered_sixth: Degree) -> Option[ChordClass] {` — The German sixth: the Italian sixth
  with the lowered third added, which spells as a perfect fifth above the root. It sounds like a dominant seventh and is
  not one — the top note is an augmented sixth, written from a different letter, and re-rooting `dominant7` here would
  spell the wrong note. OMT 063.
- `altered_dominant(collection: Scale, quality: ChordClass) -> Option[ChordClass] {` — An altered or extended dominant:
  a named quality on the fifth degree. Which alteration is present is the caller's, written as the chord type — there is
  no universal set of alterations, and a function that picked one would be asserting a style rather than constructing a
  chord. `chord c dom7b9`, `chord c dom7s5`, and the plain extensions all pass here. OMT 071.

## `std::tonal::sequences`

- `rising_degree(start: Degree, steps: Nat, index: Nat) -> Degree {` — The degree reached after `index` applications of
  a rise of `steps` scale steps. Index zero is the start, which is what makes a count of one mean "the pattern, stated
  once".
- `risen_by(steps: Nat, index: Nat, from: Degree) -> Degree { up_steps(from, steps) }` — The rise itself, as the
  `nat_fold` step it is applied by. The index is ignored on purpose: a diatonic sequence moves by the same interval
  every time, and a pattern that did not would be a different pattern.
- `falling_degree(start: Degree, steps: Nat, index: Nat) -> Degree {` — The degree reached after `index` applications of
  a fall of `steps` scale steps. Falling is its own function rather than a negative rise, because a `nat` has no sign
  and a direction that could be forgotten is a direction that will be.
- `fallen_by(steps: Nat, index: Nat, from: Degree) -> Degree { down_steps(from, steps) }` — The fall, as the `nat_fold`
  step it is applied by.
- `rising_degrees(start: Degree, steps: Nat, count: Nat) -> List[Degree] {` — The whole finite walk upward: `count`
  degrees, beginning at `start`. A count of zero is the empty walk and a count of one is the start alone, which are the
  ordinary meanings and are asserted as laws.
- `falling_degrees(start: Degree, steps: Nat, count: Nat) -> List[Degree] {` — The whole finite walk downward.
- `stacked_on(collection: Scale, members: Nat, written: Degree) -> Option[ChordClass] {` — Harmonize one degree of a
  walk with the collection's own stack. Absent when the collection stacks to a sonority the chord vocabulary cannot
  name, which is how a pentatonic or whole-tone collection reports that it does not harmonize in thirds.
- `harmonized(collection: Scale, members: Nat, walk: List[Degree]) -> List[Option[ChordClass]] {` — Harmonize a whole
  walk. The quality of each chord is the collection's, so a sequence in minor is a different succession of qualities
  from the same sequence in major without either being written twice.
- `descending_fifths_degree(start: Degree, index: Nat) -> Degree {` — The descending-fifths sequence, one index at a
  time: roots fall four scale steps each time, which is a fifth down inside the collection. OMT 049.
- `descending_fifths_chord(collection: Scale, start: Degree, members: Nat, index: Nat) -> Option[ChordClass] {` — The
  chord the descending-fifths sequence reaches at one index.
- `descending_fifths(collection: Scale, start: Degree, count: Nat) -> List[Option[ChordClass]] {` — The
  descending-fifths sequence as triads. In a major collection this is the succession OMT writes
  `I–IV–viiº–iii–vi–ii–V–I`, and the diminished triad in it is the collection's doing rather than an exception. OMT 049.
- `descending_fifths_sevenths(collection: Scale, start: Degree, count: Nat) -> List[Option[ChordClass]] {` — The same
  sequence as seventh chords, which is how OMT 049 most often presents it because the sevenths chain the resolutions
  together.
- `ascending_fifths_degree(start: Degree, index: Nat) -> Degree {` — The ascending-fifths sequence: roots rise four
  scale steps each time. A different pattern from the descending one and not its retrograde, because the collection is
  not symmetrical. OMT 049.
- `ascending_fifths(collection: Scale, start: Degree, count: Nat) -> List[Option[ChordClass]] {` — The ascending-fifths
  sequence as triads.
- `descending_thirds_degree(start: Degree, index: Nat) -> Degree {` — The descending-thirds sequence, the skeleton under
  the descending 5–6 pattern: roots fall two scale steps each time. OMT 049.
- `descending_thirds(collection: Scale, start: Degree, count: Nat) -> List[Option[ChordClass]] {` — The
  descending-thirds sequence as triads.
- `ascending_seconds_degree(start: Degree, index: Nat) -> Degree {` — The ascending-seconds sequence, the skeleton under
  the ascending 5–6 pattern and under parallel first-inversion chords: roots rise one scale step each time. OMT 049.
- `ascending_seconds(collection: Scale, start: Degree, count: Nat) -> List[Option[ChordClass]] {` — The
  ascending-seconds sequence as triads, in root position.
- `parallel_sixths(collection: Scale, start: Degree, count: Nat) -> List[Option[ChordClass]] {` — The same walk with
  every chord in first inversion: the parallel `6/3` passage OMT 049 describes. The inversion is a designation on the
  chord class and not yet a bass note — which note actually sounds lowest is still the voicing's decision.
- `first_inversion(content: Option[ChordClass]) -> Option[ChordClass] {` — Designate the third as bass, keeping absence
  absent. Written out because an `option` of an `option` is not a chord and this language composes the two by hand.
- `stage_music(policy: ChordClass -> Music, content: Option[ChordClass]) -> Music {` — Sound one stage of a skeleton
  under a caller's voicing policy. The policy is a function because the library has no opinion: a stage that cannot be
  voiced from the bass the caller named is silence here, and the caller can see that it was.

## `std::transformational`

- `triad_root(refined: Triad) -> NoteName { chord_root(triad_chord(refined)) }` — The root of a triad, spelled.
- `triad_third(refined: Triad) -> NoteName {` — The third, which is the tone that says which triad this is: a major
  third above the root for the major triad, a minor third for the minor.
- `triad_fifth(refined: Triad) -> NoteName { triad_root(refined) up P5 }` — The fifth, which is perfect in both triads
  and so needs no question.
- `major_triad_on(root: NoteName, fallback: Triad) -> Triad {` — The major triad rooted on a pitch class. `fallback` is
  returned only if a major triad were not a triad, which is why it is the argument the caller already had: an impossible
  answer is still a triad and still spelled from somewhere real, rather than a silence that would have to be explained.
  Every transformation below is total because of this function and the next.
- `minor_triad_on(root: NoteName, fallback: Triad) -> Triad {` — The minor triad rooted on a pitch class, on the same
  terms.
- `itself(refined: Triad) -> Triad { refined }` — The present case of the two constructors above.
- `parallel(refined: Triad) -> Triad {` — P, parallel: keep the root and the fifth, move the third by a chromatic
  semitone. The C major triad and the C minor triad, which share a letter and a key signature's worth of difference.
- `leading_tone(refined: Triad) -> Triad {` — L, Leittonwechsel: keep the third and the fifth, and move the remaining
  tone by a diatonic semitone. From a major triad that is its own third made a root — C major goes to E minor, the root
  `c` falling to `b`; from a minor triad the fifth rises, E minor back to C major.
- `relative(refined: Triad) -> Triad {` — R, relative: keep the root and the third, move the remaining tone by a whole
  step. C major goes to A minor and A minor back to C major, which is the relative pair every key signature already
  names.
- `slide(refined: Triad) -> Triad { leading_tone(parallel(relative(refined))) }` — S, slide: keep the third, move the
  root and the fifth by a chromatic semitone. C major goes to C sharp minor. Written as `L P R` because that is what it
  is — OMT 072 introduces S, N, and H as the named compositions worth having, not as further generators.
- `nebenverwandt(refined: Triad) -> Triad { relative(leading_tone(parallel(refined))) }` — N, Nebenverwandt: a major
  triad and its minor subdominant, C major to F minor. `R L P`.
- `hexatonic_pole(refined: Triad) -> Triad { leading_tone(parallel(leading_tone(refined))) }` — H, the hexatonic pole:
  the triad sharing no tone at all with its argument, C major to G sharp minor. `L P L`.
- `then(first: Triad -> Triad, second: Triad -> Triad, refined: Triad) -> Triad {` — Two transformations, applied in the
  order written.
- `chain(steps: List[Triad -> Triad], start: Triad) -> Triad {` — A chain of transformations, applied left to right from
  a starting triad. Composition is what a chain is — no transformation here is a keyword or a special form, so a list of
  them is ordinary data and this is an ordinary fold.
- `applied(operation: Triad -> Triad, carried: Triad) -> Triad { operation(carried) }` — One step of `chain`.
- `unspelled(spelled: NoteName) -> Pc12 { pc12_forget(spelled) }` — Forget one spelling. Named for what it does to a
  pitch class rather than for the domain it lands in, so that a piece may import this and `std::post_tonal::pcset`
  together.
- `triad_tones(refined: Triad) -> List[NoteName] {` — The three tones of a triad, spelled, from the root upward.
- `triad_classes(refined: Triad) -> PcSet12 {` — The projection into the chromatic quotient: the triad as a set of three
  unspelled pitch classes. This is where a finite group claim becomes sayable. `dbb` major and `c` major are two triads
  and one set, so a cycle that fails to close in spelling closes here, and the failure and the closing are both facts an
  author can hold at once rather than one hiding the other.

## `std::voicing`

- `close_position(content: ChordClass, bass: Pitch) -> Option[Voicing] {` — Stack the class upward from an absolute
  bass, one member per octave position, taking each next member at the first pitch above the last. This is the one
  policy `stack c4 major7/2` desugars to.
- `drop_position(content: ChordClass, bass: Pitch, from_top: Nat) -> Option[Voicing] {` — Close position with one upper
  note dropped an octave, counted from the top: `drop_position(content, bass, 2)` is the drop-2 voicing.
- `voiced_as(content: ChordClass, pitches: List[Pitch]) -> Option[Voicing] {` — The voicing an explicit list of written
  pitches spells, when those pitches really do voice the class: ascending, distinct, and every one a member. This is how
  a writer voices by hand.
- `pitches_of(chosen: Voicing) -> List[Pitch] { voicing_pitches(chosen) }` — Every sounding pitch of a voicing, lowest
  first.
- `lowest_of(chosen: Voicing) -> Pitch { voicing_bass(chosen) }` — The lowest sounding pitch. A voicing always has one,
  which is why this is not an `option`.
- `chord_of(chosen: Voicing) -> ChordClass { voicing_chord(chosen) }` — The chord class this voicing voices.
- `inversion_of(chosen: Voicing) -> Option[Nat] { voicing_position(chosen) }` — Which member is in the bass, counted
  from zero, when the bass is a member at all. Absent for a slash bass, which is not an inversion.
- `omitting(chosen: Voicing, position: Nat) -> Option[Voicing] {` — Drop one numbered member from a voicing, keeping the
  class it voices. The chord class is unchanged: an omission is a choice about what sounds, not a claim that the chord
  is a different chord.
- `rootless(chosen: Voicing) -> Option[Voicing] { omit_voicing(chosen, 0) }` — The rootless voicing a pianist plays
  under a bass player: the root, in position zero, is the note removed, and it is named here rather than left implicit.
- `sound_for(chosen: Voicing, held: Duration) -> Music { play(chosen, held) }` — Sound a chosen voicing for a written
  length. This is the only way a chord class becomes notes.
