# Musa standard library 1

This reference is generated from what the compiler records about each bundled declaration — the same record an
editor shows on hover. Standard definitions are ordinary Musa definitions; importing a module is explicit and never
searches the filesystem. Under a `structure`, only the members its signature exports are listed, because the rest are
private to it.

## `std::collections`

- `fn major_on(tonic: NoteName) -> Scale` — The major collection, W–W–H–W–W–W–H, rooted on tonic.
- `fn dorian_on(tonic: NoteName) -> Scale` — The Dorian rotation of the diatonic collection.
- `fn phrygian_on(tonic: NoteName) -> Scale` — The Phrygian rotation.
- `fn lydian_on(tonic: NoteName) -> Scale` — The Lydian rotation.
- `fn mixolydian_on(tonic: NoteName) -> Scale` — The Mixolydian rotation.
- `fn natural_minor_on(tonic: NoteName) -> Scale` — The natural minor collection, which is also the Aeolian rotation.
- `fn locrian_on(tonic: NoteName) -> Scale` — The Locrian rotation.
- `fn harmonic_minor_on(tonic: NoteName) -> Scale` — Harmonic minor: natural minor with a raised seventh degree. A distinct collection, not a spelling of the natural one.
- `fn melodic_minor_on(tonic: NoteName) -> Scale` — Ascending melodic minor: raised sixth and seventh degrees.
- `fn descending_melodic_minor_on(tonic: NoteName) -> Scale` — Descending melodic minor, whose offsets are the natural minor's and whose identity is not.
- `fn major_pentatonic_on(tonic: NoteName) -> Scale` — The five-note major pentatonic collection.
- `fn minor_pentatonic_on(tonic: NoteName) -> Scale` — The five-note minor pentatonic collection.
- `fn whole_tone_on(tonic: NoteName) -> Scale` — The six-note whole-tone collection.
- `fn octatonic_half_whole_on(tonic: NoteName) -> Scale` — The eight-note octatonic collection beginning with a half step.
- `fn octatonic_whole_half_on(tonic: NoteName) -> Scale` — The eight-note octatonic collection beginning with a whole step.
- `fn hexatonic_on(tonic: NoteName) -> Scale` — The six-note hexatonic collection alternating minor thirds and half steps.
- `fn acoustic_on(tonic: NoteName) -> Scale` — The acoustic collection: raised fourth and lowered seventh.

## `std::context`

- `signature TonalContext` — A tonal context is the small bundle of facts that always travel together: what key a passage is in, which collection it steps through, how a numbered degree is spelled in register, and how a chord class is voiced. Passing them one at a time is how they drift apart, which is the whole reason a signature exists — Open Music Theory `105-diatonic-modes.md` names the collection, `017-triads.md` the chords, and neither is meaningful without the other.
  - `let TonalContext.tonic: Key` — The key the passage is written in. A key is a signature and a tonic, never a scale.
  - `let TonalContext.collection: Scale` — The collection stepwise motion reads. A default, not a claim: a passage may still name another collection where it wants one.
  - `let TonalContext.spell: Nat -> Option<Pitch>` — The written pitch a numbered degree names, in this context's own register. Absent when the context has no register to spell in.
  - `let TonalContext.voicing_for: ChordClass -> Option<Voicing>` — How this context voices a chord class. Absent when the class cannot be voiced from the register it chose.
- `structure CMajor: TonalContext` — C major, spelled from middle C.
  - `let CMajor.tonic: Key` — The key the passage is written in. A key is a signature and a tonic, never a scale.
  - `let CMajor.collection: Scale` — The collection stepwise motion reads. A default, not a claim: a passage may still name another collection where it wants one.
  - `fn CMajor.spell(ordinal: Nat) -> Option<Pitch>` — The written pitch a numbered degree names, in this context's own register. Absent when the context has no register to spell in.
  - `fn CMajor.voicing_for(content: ChordClass) -> Option<Voicing>` — How this context voices a chord class. Absent when the class cannot be voiced from the register it chose.
- `structure ANaturalMinor: TonalContext` — A natural minor, spelled from the A below middle C. The same four members, answered differently — which is what makes the two structures interchangeable everywhere `TonalContext` is asked for.
  - `let ANaturalMinor.tonic: Key` — The key the passage is written in. A key is a signature and a tonic, never a scale.
  - `let ANaturalMinor.collection: Scale` — The collection stepwise motion reads. A default, not a claim: a passage may still name another collection where it wants one.
  - `fn ANaturalMinor.spell(ordinal: Nat) -> Option<Pitch>` — The written pitch a numbered degree names, in this context's own register. Absent when the context has no register to spell in.
  - `fn ANaturalMinor.voicing_for(content: ChordClass) -> Option<Voicing>` — How this context voices a chord class. Absent when the class cannot be voiced from the register it chose.

## `std::core`

- `fn identity_ratio(value: Ratio) -> Ratio` — Return an exact rational unchanged. This is useful when a public API wants to say explicitly that it preserves a proportion.
- `fn identity_nat(value: Nat) -> Nat` — Return a natural number unchanged.
- `fn compose_music(first: Music -> Music, second: Music -> Music, value: Music) -> Music` — Apply the second musical transformation, then the first.
- `fn compose_pitch(first: Pitch -> Pitch, second: Pitch -> Pitch, value: Pitch) -> Pitch` — Apply the second pitch function, then the first.

## `std::harmony`

- `fn chord_rooted_on(content: ChordClass, root: NoteName) -> ChordClass` — Re-root a chord class, keeping its type. `chord c major7` on `eb` is an E-flat major seventh, spelled from E-flat.
- `fn root_of(content: ChordClass) -> NoteName` — The pitch class a chord class is rooted on. This is the root, which is not the bass: a designated bass is asked for separately.
- `fn bass_of(content: ChordClass) -> Option<NoteName>` — The bass a chord class designates, when it designates one. Absent means no bass was chosen — it does not mean the root.
- `fn members_of(content: ChordClass) -> List<Interval>` — The spelled intervals above the root, lowest first, beginning at the unison. Spelled: a major third is a third, never a diminished fourth.
- `fn inversion(content: ChordClass, position: Nat) -> Option<ChordClass>` — A true inversion: the numbered member becomes the designated bass. Positions are counted from zero, so position one is first inversion. Absent when the class has no such member.
- `fn slash_bass(content: ChordClass, bass: NoteName) -> ChordClass` — A slash bass: a designated bass that need not be a member at all. `chord_over(chord c major, d)` is C over D, and the D is not a chord tone. This is a different construction from an inversion, and stays one.
- `fn as_triad(content: ChordClass) -> Option<Triad>` — The triad refinement, when the content really is a major or minor triad. This is the domain a neo-Riemannian transformation acts on, and the proof that it applies is this `option` being present.
- `fn triad_content(refined: Triad) -> ChordClass` — Forget the refinement: every triad is a chord class.
- `fn is_triad(content: ChordClass) -> Bool` — Whether a chord class is a major or minor triad.
- `fn triad_is_present(refined: Triad) -> Bool` — The present case of `is_triad`: a refinement that exists is a triad, whichever of the two it turned out to be.
- `fn is_major(refined: Triad) -> Bool` — Which of the two a triad is. Total, and a `bool` rather than a partial answer, because the refinement admitted exactly two chord classes: not major is minor here, and only here. Every transformation in `std::transformational` branches on this, since which way a voice moves is the whole content of the transformation.

## `std::list`

- `fn naturals(count: Nat) -> List<Nat>` — The natural numbers from zero up to, but not including, count.
- `fn map_pitches(function: Pitch -> Pitch, values: List<Pitch>) -> List<Pitch>` — Apply one pitch function to every member of a finite pitch list.
- `fn filter_pitches(predicate: Pitch -> Bool, values: List<Pitch>) -> List<Pitch>` — Keep the pitches for which predicate returns true.
- `fn repeat_music(value: Music, count: Nat) -> List<Music>` — Repeat one contextual music value count times as finite data.

## `std::option`

- `fn pitch_or_else(fallback: Pitch, present: Pitch -> Pitch, value: Option<Pitch>) -> Pitch` — Read an optional pitch, using fallback when it is absent and present when it is available.
- `fn nat_or_else(fallback: Nat, present: Nat -> Nat, value: Option<Nat>) -> Nat` — Read an optional natural number under the same explicit policy.

## `std::pitch`

- `let unison: Interval` — Open Music Theory `016-intervals.md` supplies the conventional generic/specific interval names; `005-half-steps-whole-steps-and-accidentals.md` supplies the spelling distinction retained by these values. The written unison has no staff displacement and no chromatic displacement.
- `let minor_second: Interval` — The chromatic semitone spelled as a minor second.
- `let major_second: Interval` — The diatonic tone spelled as a major second.
- `let perfect_fourth: Interval` — The perfect fourth.
- `let perfect_fifth: Interval` — The perfect fifth.
- `let octave: Interval` — The written octave, which changes both coordinates by (7, 12).
- `fn compose_intervals(first: Interval, second: Interval) -> Interval` — Apply two spelling-preserving written intervals in sequence.
- `fn inverse_interval(value: Interval) -> Interval` — Reverse the direction of a written interval.
- `fn written_pitch_class(value: Pitch) -> NoteName` — Forget octave while retaining the written letter and accidental.

## `std::post_tonal::pcset`

- `fn pc(number: Nat) -> Pc12` — The pitch class a number names, reduced modulo twelve. `pc(13)` and `pc(1)` are one pitch class, because they are one residue.
- `fn pcs(numbers: List<Nat>) -> List<Pc12>` — The pitch classes a list of numbers names, each reduced modulo twelve. A row or a set is written as its numbers, because that is what this domain has instead of letters.
- `fn number_of(member: Pc12) -> Nat` — The canonical representative, zero through eleven.
- `fn forget_spelling(spelled: NoteName) -> Pc12` — Forget a spelling. This is the only total map from the spelled domain into this one; it is not injective, and it has no inverse without a policy.
- `fn spelled_in(member: Pc12, collection: Scale) -> Option<NoteName>` — Spell a pitch class inside one collection — the explicit policy that `forget_spelling` has no inverse without. Absent when the collection holds no note of this pitch class.
- `fn transposed_by(index: Nat, member: Pc12) -> Pc12` — T_n: transposition by n semitones, `x + n` modulo twelve. The index comes first so that `transposed_by(3)` is the transposition itself, a `pc12 -> pc12` that `map_pc` can carry across a list.
- `fn inverted_about(index: Nat, member: Pc12) -> Pc12` — I_n: inversion about n, `n - x` modulo twelve. I_0 is the plain mirror through zero. The twelve transpositions and the twelve inversions are together the whole 24-element affine group on `pc12` — and 24 is the number, whatever a row's four form labels might suggest.
- `fn map_pc(function: Pc12 -> Pc12, members: List<Pc12>) -> List<Pc12>` — Apply one pitch-class function to every member of a finite list.
- `fn pcset(members: List<Pc12>) -> PcSet12` — The set of everything listed, however often it was listed. A set cannot hold a duplicate, so this cannot fail: a repetition is a mistake only where order matters, which is `std::post_tonal::serial`.
- `fn set_members(set: PcSet12) -> List<Pc12>` — The members, ascending from zero. This is the set's own order and not its normal order. Named for the set rather than `members_of`, because `std::harmony` already reads the members of a chord class and a piece that reasons about both must be able to import both.
- `fn set_transposed(set: PcSet12, index: Nat) -> PcSet12` — T_n applied to every member.
- `fn set_inverted(set: PcSet12, index: Nat) -> PcSet12` — I_n applied to every member.
- `fn normal_order(set: PcSet12) -> List<Pc12>` — Normal order: the rotation of the ascending members packed most tightly to the left. Ties break inward — first to last, then first to the one before last, and so on — and finally by the lowest starting pitch class.
- `fn prime_form(set: PcSet12) -> PcSet12` — Prime form: the set class this set belongs to. The normal orders of the set and of its inversion are each transposed to begin on zero, and whichever reads lower is the answer.
- `fn interval_class_vector(set: PcSet12) -> List<Nat>` — The interval-class vector: six counts, for interval classes one through six. Six and not twelve, because interval class seven is interval class five heard the other way round.

## `std::post_tonal::serial`

- `fn row(pcs: List<Pc12>) -> Option<Row12>` — The row a sequence spells, or nothing when the sequence is not one.
- `fn repeated_positions(pcs: List<Pc12>) -> List<Nat>` — The order positions whose pitch class already appeared earlier — the exact reason a sequence failed to be a row. The first occurrence is not among them, because that is where the pitch class belongs.
- `fn missing_classes(pcs: List<Pc12>) -> List<Pc12>` — The pitch classes a sequence never names, ascending — the other exact reason. A sequence of the right length has one of these lists empty exactly when it has the other empty.
- `fn pcs_of(series: Row12) -> List<Pc12>` — The row's pitch classes, in order position order.
- `fn transposed(series: Row12, index: Nat) -> Row12` — P: transposition by n semitones, order positions untouched.
- `fn inverted(series: Row12, index: Nat) -> Row12` — I: inversion about n, order positions untouched.
- `fn retrograde_of(series: Row12) -> Row12` — R: the order positions reversed, pitch classes untouched. An involution, and it commutes with P and I because it acts on the other side of the row.
- `fn retrograde_inversion_of(series: Row12, index: Nat) -> Row12` — RI: the retrograde of the inversion, which is also the inversion of the retrograde. Writing it both ways and getting one row is what "commutes" means here.
- `fn matrix(series: Row12) -> List<Row12>` — The twelve-tone matrix, as twelve rows. Row zero is the row as written; row i is the transposition beginning on the ith pitch class of the inversion about the row's own head, so every column read downward is an inversion. The construction fixes no naming convention, because the rows are rows and not labels: which transposition is called P0 is the question the two functions below answer, differently and by name.
- `fn fixed_zero_index(series: Row12) -> Nat` — The transposition index under the fixed-zero convention: P0 is the form beginning on pitch class zero, so a row's index is simply the number of the pitch class it begins on.
- `fn moveable_zero_index(reference: Row12, form: Row12) -> Nat` — The transposition index under the moveable-zero convention: P0 is the row as written, so an index is only meaningful relative to a stated reference row. `moveable_zero_index(reference, form)` is how far the form stands above the reference.
- `fn distinct_forms(series: Row12) -> Nat` — How many *distinct* rows the 48 labelled forms produce. Forty-eight for a generic row; fewer for a row some labelled operation fixes.
- `fn symmetries(series: Row12) -> Nat` — The order of the row's stabilizer: how many of the 48 labelled operations send the row to itself. This times `distinct_forms` is always 48, which is the orbit-stabilizer accounting the four labels are so often asked to do on their own.
- `fn row_spelled_in(series: Row12, collection: Scale) -> List<Option<NoteName>>` — Spell one row inside a collection, position by position. A pitch class the collection cannot spell is absent, and the row keeps its length, so a projection that lost notes is visible as the gaps it left.
- `fn spelling_in(collection: Scale, member: Pc12) -> Option<NoteName>` — The spelling policy of one collection, as a function a row can be carried across.
- `fn first_pc(series: Row12) -> Pc12` — The pitch class a row begins on. Order position zero always exists, because a row has twelve of them.

## `std::scale`

- `let tonic_degree: Degree` — The first degree of any scale. Degrees are written from one, as musicians write them.
- `fn key_scale(written: Key) -> Scale` — The scale a key's signature suggests for stepwise motion. It is a default, not a claim: `key c minor` fixes three flats, and a passage may still ask for the harmonic or melodic collection by name.
- `fn scale_root(collection: Scale) -> NoteName` — The tonic pitch class a scale is rooted on.
- `fn scale_degrees(collection: Scale) -> Nat` — How many degrees one period of a scale holds.
- `fn degree_in(collection: Scale, written: Pitch) -> Option<Degree>` — The degree a written pitch occupies, when it occupies one. Membership is spelled: `eb5` and `d#5` answer differently.
- `fn belongs_to(collection: Scale, written: Pitch) -> Bool` — Whether a written pitch belongs to a scale at all.
- `fn degree_is_present(located: Degree) -> Bool` — The present case of `belongs_to`: a located degree means the pitch is a member, whichever degree it turned out to be.
- `fn frame_on(collection: Scale, root: Pitch) -> Option<Frame>` — The register frame a scale takes on one absolute tonic pitch. It is absent when that pitch is not the scale's tonic class.
- `fn frame_degree(register: Frame, ordinal: Nat) -> Pitch` — The written pitch a numbered degree names in one register frame.
- `fn frame_triad(register: Frame) -> List<Pitch>` — The tonic, third, and fifth degrees of a frame, in register.
- `fn degree_class(collection: Scale, ordinal: Nat) -> Option<NoteName>` — The pitch class a numbered degree names, with no register at all. `frame_degree` asks the same thing of a scale that has been given an absolute tonic, and it has to be given one, because a written pitch has an octave and something must choose it. A Roman numeral has no octave to choose — `V` in C major is the class `g`, and which `g` sounds is the voicing's business. Absent only for an ordinal no score can write.
- `fn altered_class(collection: Scale, altered: Degree) -> Option<NoteName>` — The same for a degree that already carries an alteration, so that a `raise` or `lower` composes into the spelling rather than being lost. This is how a borrowed or Neapolitan degree is spelled without a frame.
- `fn degree_chord(collection: Scale, written: Degree, members: Nat) -> Option<ChordClass>` — The chord the collection stacks in thirds from a degree. `members` counts the notes, so three is a triad and four a seventh chord. The quality is the collection's and not the caller's: `ii` is minor in major and `II` is major in Dorian because those are the notes there, which is the whole content of the word "diatonic". A degree rather than a number because the degree is where this language does ordinal arithmetic, so a succession walked by `up_steps` can be harmonized where a walked number could not. Absent for an altered degree, which is not asking for the collection's own chord, and absent when the collection stacks to a sonority the chord vocabulary cannot name.
- `fn degree_triad(collection: Scale, written: Degree) -> Option<ChordClass>` — The diatonic triad on a degree.
- `fn degree_seventh(collection: Scale, written: Degree) -> Option<ChordClass>` — The diatonic seventh chord on a degree.
- `fn up_steps(from: Degree, steps: Nat) -> Degree` — Move a degree up by a whole number of scale steps.
- `fn down_steps(from: Degree, steps: Nat) -> Degree` — Move a degree down by a whole number of scale steps.
- `fn raise(from: Degree) -> Degree` — Raise a degree chromatically without moving its coordinate.
- `fn lower(from: Degree) -> Degree` — Lower a degree chromatically without moving its coordinate.

## `std::tonal::harmony`

- `fn numeral(ordinal: Nat, members: Nat, position: Nat) -> Option<Roman>` — The numeral three numbers describe, when they describe one. Absent when the ordinal is outside `I`–`vii`, when the stack is smaller than a triad or larger than a thirteenth, or when the bass position names a member the stack does not have — a third inversion of a triad is not a numeral that is hard to realize, it is not a numeral. OMT 020 and 021.
- `fn triad_numeral(ordinal: Nat) -> Option<Roman>` — The root-position triad on a degree, which is what OMT 020 writes with a bare numeral and no figures.
- `fn seventh_numeral(ordinal: Nat) -> Option<Roman>` — The root-position seventh chord on a degree: OMT 021's `7`.
- `fn numeral_step(written: Roman) -> Nat` — Which degree the numeral is built on, counted from one.
- `fn numeral_size(written: Roman) -> Nat` — How many members the numeral stacks: three is a triad, four a seventh.
- `fn numeral_bass(written: Roman) -> Nat` — Which member the numeral puts in the bass, counted from zero, so zero is root position and one is the `6` of figured bass. A position in the stack and not a pitch: which note actually sounds lowest is a voicing's business, and `std::voicing` is where that is decided.
- `fn numeral_chord(collection: Scale, written: Roman) -> Option<ChordClass>` — The chord a numeral names in a collection, spelled by that collection. The quality is never supplied: `ii` is minor in major and `II` is major in Dorian because those are the notes there, which is the whole content of the word "diatonic". Absent when the collection has no such degree, or when it stacks to a sonority the chord vocabulary cannot name — a harmonic-minor `III7` is an augmented major seventh, and reporting the absence is more honest than rounding it to a chord with other notes in it. OMT 020 and 021.
- `fn borrowed(home: Scale, mode: Scale, written: Roman) -> Option<ChordClass>` — Modal mixture: the numeral realized against a borrowed collection on the home tonic. Borrowing needs no altered degrees and no alteration field, because that is what the word means — the flat six of C major is the sixth of C minor, and spelling it as a lowered degree describes the result rather than the operation. Which collection is borrowed from is the caller's, since parallel minor, harmonic minor, and Phrygian all lend chords and none of them is the default. OMT 061.
- `fn secondary(home: Scale, target: Degree, mode: Scale, written: Roman) -> Option<ChordClass>` — An applied chord: a numeral read in the collection that tonicizes a target degree of the home collection. `V/V` in C major is `secondary(home, degree_of(5), major_on(c), five)` — the numeral is read in G, which is why it is D major and not the D minor that C major stacks. The target degree is written out, so a tonicized lowered sixth is as sayable as a tonicized fifth, and the tonicizing collection is written out, so nothing here decides that an applied chord implies major. Tonicization is a local relationship between two chords; whether a passage has modulated is a claim about the passage and is not decided here. OMT 050.
- `fn quality_on_degree(collection: Scale, written: Degree, quality: ChordClass) -> Option<ChordClass>` — A named chord quality rooted on a named degree of a collection. This is how every chromatic sonority below is built: the degree carries the alteration, the quality carries the members, and neither is guessed. A Neapolitan in C major names a lowered second and in C minor names the plain second, and that difference is the caller's to write, because the minor collection was the caller's to choose. Absent when the collection has no such degree.
- `fn neapolitan(collection: Scale, lowered_second: Degree) -> Option<ChordClass>` — The Neapolitan: a major triad on the lowered second degree. The `6` in its usual name is a first inversion, which is a position and is taken with `inversion` rather than baked in here — the chord is a major triad in any position, and OMT 062 says so before it says the sixth is idiomatic.
- `fn italian_sixth(collection: Scale, lowered_sixth: Degree) -> Option<ChordClass>` — The Italian sixth: a lowered sixth, the tonic, and a raised fourth, with no fifth. Its augmented sixth is spelled as an augmented sixth, which is the whole reason it is a chord of its own and not a seventh. OMT 063.
- `fn french_sixth(collection: Scale, lowered_sixth: Degree) -> Option<ChordClass>` — The French sixth: the Italian sixth with the second degree added, which spells as an augmented fourth above the root. OMT 063.
- `fn german_sixth(collection: Scale, lowered_sixth: Degree) -> Option<ChordClass>` — The German sixth: the Italian sixth with the lowered third added, which spells as a perfect fifth above the root. It sounds like a dominant seventh and is not one — the top note is an augmented sixth, written from a different letter, and re-rooting `dominant7` here would spell the wrong note. OMT 063.
- `fn altered_dominant(collection: Scale, quality: ChordClass) -> Option<ChordClass>` — An altered or extended dominant: a named quality on the fifth degree. Which alteration is present is the caller's, written as the chord type — there is no universal set of alterations, and a function that picked one would be asserting a style rather than constructing a chord. `chord c dom7b9`, `chord c dom7s5`, and the plain extensions all pass here. OMT 071.

## `std::tonal::schemas`

- `fn degrees_of(ordinals: List<Nat>) -> List<Degree>` — The degrees of a written line, from the ordinals OMT prints. Written out rather than folded because a schema *is* its table: a reader should see the same numbers here that they see in OMT's row.
- `fn as_degree(ordinal: Nat) -> Degree` — One ordinal as a degree. Written out because `degree_of` is a compiler-owned operation and only a source function can be passed to a fold.
- `fn romanesca_bass() -> List<Degree>` — Romanesca, OMT 34 §Romanesca. Four stages, strong-weak-strong-weak, figures 5-6-5-6. The bass falls *do-ti-la-mi*: three steps down and then a leap, which is what distinguishes it from the stepwise openings.  Direction: descending, then a leap down to the third.
- `fn romanesca_melody() -> List<Degree>` — The Romanesca melody, *do-sol-do-do*.
- `fn romanesca_roots() -> List<Degree>` — The roots the Romanesca's figures name: I-V-vi-I. The 6 on stage two means the chord over *ti* is rooted on *sol*, and the 6 on stage four means the chord over *mi* is rooted on *do*. The figure is read here so that a caller reads roots, and the bass is kept separately so that the caller can still voice from it.
- `fn romanesca_strong() -> List<Bool>` — The Romanesca's metrical shape. True is OMT's S.
- `fn do_re_mi_bass() -> List<Degree>` — Do-Re-Mi, OMT 34 §Do–Re–Mi. Three stages, the shortest opening in the summary table, and the one whose two halves OMT itself calls schemas: the Do-Re question and the Re-Mi answer. Figures 5-6-5, numerals I-V-I.
- `fn do_re_mi_melody() -> List<Degree>` — The Do-Re-Mi melody, which is the stepwise ascent the schema is named for.
- `fn do_re_mi_roots() -> List<Degree>` — The Do-Re-Mi roots: I-V-I, the 6 on stage two rooting the chord over *ti* on *sol*.
- `fn do_re_mi_strong() -> List<Bool>` — The Do-Re-Mi's metrical shape, strong-weak-strong: the shortest of the openings leans on its outer stages.
- `fn prinner_bass() -> List<Degree>` — Prinner, OMT 34 §Prinner. Four stages with a stepwise falling bass *fa-mi-re-do* under a falling *la-sol-fa-mi*: parallel tenths, which is why it answers an opening so readily. Figures 5-6-7-6-5, numerals IV-I-vii-I.
- `fn prinner_melody() -> List<Degree>` — The Prinner melody, *la-sol-fa-mi*. It falls a tenth above the falling bass at every stage, which is the parallel motion the schema is heard by.
- `fn prinner_roots() -> List<Degree>` — The Prinner's roots. Stage two's 6 roots the chord over *mi* on *do*, and stage four's 6 does the same over *do*, which is why the last two roots are not the bass.
- `fn prinner_strong() -> List<Bool>` — The Prinner's metrical shape, strong-weak-strong-weak.
- `fn prinner_with_dominant_bass() -> List<Degree>` — The five-stage Prinner OMT 34 gives as "a slight variant on this": a root-position dominant is inserted before the final stage. A separate function rather than a flag, because a variant with a different number of stages is a different table and a caller reading four lists in parallel should not have one of them silently change length.
- `fn prinner_with_dominant_melody() -> List<Degree>` — The five-stage Prinner's melody: the four-stage line with *fa* held across the inserted dominant rather than falling through it.
- `fn prinner_with_dominant_roots() -> List<Degree>` — The five-stage Prinner's roots. The four-stage roots with the inserted stage rooted on the fifth degree, which is what makes it a dominant.
- `fn prinner_with_dominant_strong() -> List<Bool>` — The five-stage Prinner's metrical shape. One stage longer than the four-stage Prinner's, and strong at the end rather than weak.
- `fn fonte_bass() -> List<Degree>` — Fonte, OMT 34 §Fonte. Two tonicizations a step apart, and the first of them needs a bass note the collection does not contain: OMT writes it *di*, a raised *do*. This is the schema that proves an alteration has to be carried explicitly — `raise` moves the spelling without moving the coordinate, so the degree is still the first and is still spelled sharp.
- `fn fonte_melody() -> List<Degree>` — The Fonte melody, *sol-fa-fa-mi*: the fourth degree is held across the step down, so only the bass and the harmony move between the two tonicizations.
- `fn fonte_roots() -> List<Degree>` — The Fonte's roots: V/ii-ii-V-I. The applied dominant is rooted on the raised first degree because that is the note OMT's figure sits over, and the raising travels with the root.
- `fn fonte_strong() -> List<Bool>` — The Fonte's metrical shape, weak-strong-weak-strong: each of the two tonicizations leans on its second stage.
- `fn monte_bass() -> List<Degree>` — Monte, OMT 34 §Monte. The Fonte's counterpart, rising rather than falling, and altered in the other direction: OMT writes *fi*, a raised *fa*, tonicizing the dominant. Figures 6/5-5-6/5-5, numerals V/IV-V-V/V-V.
- `fn monte_melody() -> List<Degree>` — The Monte melody, *ti-la-do-ti*, rising to the octave over the second tonicization where the Fonte's line fell.
- `fn monte_roots() -> List<Degree>` — The Monte's roots, the numerals above read as degrees. The third stage is rooted on a raised degree because its chord is an applied dominant, and the raising travels with the root as it does in the Fonte.
- `fn monte_strong() -> List<Bool>` — The Monte's metrical shape, weak-strong-weak-strong: the same lean as the Fonte's, which is what makes the two a rising and a falling pair.
- `fn fenaroli_bass() -> List<Degree>` — Fenaroli, OMT 34 §Fenaroli. The one representative here whose bass *rises* stepwise, *ti-do-re-mi*, which is why it is in this file beside the falling ones: a schema is not its intervals reversed.
- `fn fenaroli_melody() -> List<Degree>` — The Fenaroli melody, *fa-mi-ti-do*: a falling step, and then the leading tone resolving upward to the tonic.
- `fn fenaroli_roots() -> List<Degree>` — The Fenaroli's roots: V-I-V-I, with the 6 on the last stage rooting the chord over *mi* on *do*.
- `fn fenaroli_strong() -> List<Bool>` — The Fenaroli's metrical shape, strong-weak-strong-weak.
- `fn cadenza_semplice_bass() -> List<Degree>` — Cadenza semplice, OMT 34 §Cadenza Semplice. The plain cadence: bass *mi-fa-sol-do*, figures 6-6/5-5-5, numerals I-ii-V-I.
- `fn cadenza_semplice_melody() -> List<Degree>` — The plain cadence's melody, *do-re-re-do*: the second degree held across the pre-dominant and the dominant, and falling to the tonic at the end.
- `fn cadenza_semplice_roots() -> List<Degree>` — The plain cadence's roots, I-ii-V-I. The 6 on stage one roots the chord over *mi* on *do*, and the 6/5 on stage two roots the chord over *fa* on *re*, so neither of the first two roots is its bass.
- `fn cadenza_semplice_strong() -> List<Bool>` — The plain cadence's metrical shape, weak-strong-weak-strong, so the cadence itself lands on a strong stage.
- `fn quiescenza_bass() -> List<Degree>` — Quiescenza, OMT 34 §Quiescenza. Post-cadential, and the one schema here whose bass does not move at all: four stages on *do*. Its melody needs the other alteration — OMT writes *te*, a lowered *ti* — which makes the first chord a dominant of the subdominant over a tonic pedal.
- `fn quiescenza_melody() -> List<Degree>` — The Quiescenza melody, *te-la-ti-do*. The lowered seventh is what makes the first chord a dominant of the subdominant; the rest is the ordinary ascent to the tonic over the held bass.
- `fn quiescenza_roots() -> List<Degree>` — The Quiescenza's roots, I-V-V-I, sounding above the tonic its bass holds through all four stages.
- `fn quiescenza_strong() -> List<Bool>` — The Quiescenza's metrical shape, weak-strong-weak-strong.
- `fn schema_triads(collection: Scale, roots: List<Degree>) -> List<Option<ChordClass>>` — The chord classes a schema's roots name in one collection, as triads. Absent where the collection does not stack to a nameable sonority, and absent on an altered root, because an applied dominant is not the collection's own chord and this function only knows the collection's. That absence is the honest answer: `fonte_roots` and `monte_roots` carry raised degrees on purpose, and a caller who wants those chords supplies them, from `std::tonal::harmony`, where applied chords live.
- `fn schema_triad(collection: Scale, root: Degree) -> Option<ChordClass>` — One root, harmonized as the collection's triad.
- `fn schema_sevenths(collection: Scale, roots: List<Degree>) -> List<Option<ChordClass>>` — The same as seventh chords, for the figures OMT prints with a 7 or a 6/5 in them.
- `fn schema_seventh(collection: Scale, root: Degree) -> Option<ChordClass>` — One root, harmonized as the collection's seventh chord.
- `fn sixth_over(collection: Scale, bass: Degree) -> Option<ChordClass>` — Step 1. A 6/3 over a bass degree is the triad a third below it, in first inversion. This is the whole of the parallel-sixths harmonization, and it is the same function `std::tonal::sequences` uses for the parallel 6/3 passage, seen from the bass rather than from the root.
- `fn fifth_over(collection: Scale, bass: Degree) -> Option<ChordClass>` — Step 2 and step 3 share a shape: a 5/3 is the triad rooted on the bass itself, root position, no designation.
- `fn six_five_over(collection: Scale, bass: Degree) -> Option<ChordClass>` — Step 4. A seventh chord over a bass degree, rooted a third below it so that the bass is still the third — OMT's 6/5 — which is the figure the recipe's seventh chords carry.
- `fn inverted_first(content: Option<ChordClass>) -> Option<ChordClass>` — Designate the third as bass, keeping absence absent.
- `fn rule_ascending_chord(collection: Scale, bass: Nat) -> Option<ChordClass>` — The Rule ascending, one bass degree at a time. Ordinals are written from one and read modulo the collection's period, so eight is the octave and is the tonic again.  Where each answer comes from: 1 and 5 are the recipe's 5/3s; 8 is the closing 5/3, which is the same chord as 1 and is written separately because OMT's "first and last" is two places. 4 precedes the dominant and 7 precedes the octave, so both take the recipe's sevenths. The rest are the parallel 6/3s the recipe starts from.  The chromatic alteration OMT mentions once — "In one case, this also involves a chromatic alteration for a stronger sense of tonicizing the dominant" — is *not* applied here, and its absence is the point: OMT names it without printing which chord takes it, and a library that guessed would be asserting a version rather than following one. A caller who wants it writes it, and `raise` is how.
- `fn rule_descending_chord(collection: Scale, bass: Nat) -> Option<ChordClass>` — The Rule descending, one bass degree at a time. The 5/3s stand where they stood — a tonic is a tonic whichever way the bass is walking — and the sevenths move, because 6 is what precedes the dominant coming down and 2 is what precedes the tonic.
- `fn rule_ascending_basses() -> List<Nat>` — The bass scale the Rule is harmonized over, ascending. Eight ordinals and not seven: OMT prints the octave, so the last chord is a tonic again, and the closing 5/3 has somewhere to stand. Written as a literal because the Rule of the Octave is over the octave — there is no count here to vary, which is exactly what distinguishes it from the sequences in `std::tonal::sequences`, where the count is the whole point.
- `fn rule_descending_basses() -> List<Nat>` — The same descending. Written out rather than reversed, because a descent is its own line and OMT gives it as one.
- `fn rule_of_the_octave_ascending(collection: Scale) -> List<Option<ChordClass>>` — The whole ascending Rule: eight chords over the ascending bass scale.
- `fn rule_of_the_octave_descending(collection: Scale) -> List<Option<ChordClass>>` — The whole descending Rule.

## `std::tonal::sequences`

- `fn rising_degree(start: Degree, steps: Nat, index: Nat) -> Degree` — The degree reached after `index` applications of a rise of `steps` scale steps. Index zero is the start, which is what makes a count of one mean "the pattern, stated once".
- `fn risen_by(steps: Nat, index: Nat, from: Degree) -> Degree` — The rise itself, as the `nat_fold` step it is applied by. The index is ignored on purpose: a diatonic sequence moves by the same interval every time, and a pattern that did not would be a different pattern.
- `fn falling_degree(start: Degree, steps: Nat, index: Nat) -> Degree` — The degree reached after `index` applications of a fall of `steps` scale steps. Falling is its own function rather than a negative rise, because a `nat` has no sign and a direction that could be forgotten is a direction that will be.
- `fn fallen_by(steps: Nat, index: Nat, from: Degree) -> Degree` — The fall, as the `nat_fold` step it is applied by.
- `fn rising_degrees(start: Degree, steps: Nat, count: Nat) -> List<Degree>` — The whole finite walk upward: `count` degrees, beginning at `start`. A count of zero is the empty walk and a count of one is the start alone, which are the ordinary meanings and are asserted as laws.
- `fn falling_degrees(start: Degree, steps: Nat, count: Nat) -> List<Degree>` — The whole finite walk downward.
- `fn stacked_on(collection: Scale, members: Nat, written: Degree) -> Option<ChordClass>` — Harmonize one degree of a walk with the collection's own stack. Absent when the collection stacks to a sonority the chord vocabulary cannot name, which is how a pentatonic or whole-tone collection reports that it does not harmonize in thirds.
- `fn harmonized(collection: Scale, members: Nat, walk: List<Degree>) -> List<Option<ChordClass>>` — Harmonize a whole walk. The quality of each chord is the collection's, so a sequence in minor is a different succession of qualities from the same sequence in major without either being written twice.
- `fn descending_fifths_degree(start: Degree, index: Nat) -> Degree` — The descending-fifths sequence, one index at a time: roots fall four scale steps each time, which is a fifth down inside the collection. OMT 049.
- `fn descending_fifths_chord(collection: Scale, start: Degree, members: Nat, index: Nat) -> Option<ChordClass>` — The chord the descending-fifths sequence reaches at one index.
- `fn descending_fifths(collection: Scale, start: Degree, count: Nat) -> List<Option<ChordClass>>` — The descending-fifths sequence as triads. In a major collection this is the succession OMT writes `I–IV–viiº–iii–vi–ii–V–I`, and the diminished triad in it is the collection's doing rather than an exception. OMT 049.
- `fn descending_fifths_sevenths(collection: Scale, start: Degree, count: Nat) -> List<Option<ChordClass>>` — The same sequence as seventh chords, which is how OMT 049 most often presents it because the sevenths chain the resolutions together.
- `fn ascending_fifths_degree(start: Degree, index: Nat) -> Degree` — The ascending-fifths sequence: roots rise four scale steps each time. A different pattern from the descending one and not its retrograde, because the collection is not symmetrical. OMT 049.
- `fn ascending_fifths(collection: Scale, start: Degree, count: Nat) -> List<Option<ChordClass>>` — The ascending-fifths sequence as triads.
- `fn descending_thirds_degree(start: Degree, index: Nat) -> Degree` — The descending-thirds sequence, the skeleton under the descending 5–6 pattern: roots fall two scale steps each time. OMT 049.
- `fn descending_thirds(collection: Scale, start: Degree, count: Nat) -> List<Option<ChordClass>>` — The descending-thirds sequence as triads.
- `fn ascending_seconds_degree(start: Degree, index: Nat) -> Degree` — The ascending-seconds sequence, the skeleton under the ascending 5–6 pattern and under parallel first-inversion chords: roots rise one scale step each time. OMT 049.
- `fn ascending_seconds(collection: Scale, start: Degree, count: Nat) -> List<Option<ChordClass>>` — The ascending-seconds sequence as triads, in root position.
- `fn parallel_sixths(collection: Scale, start: Degree, count: Nat) -> List<Option<ChordClass>>` — The same walk with every chord in first inversion: the parallel `6/3` passage OMT 049 describes. The inversion is a designation on the chord class and not yet a bass note — which note actually sounds lowest is still the voicing's decision.
- `fn first_inversion(content: Option<ChordClass>) -> Option<ChordClass>` — Designate the third as bass, keeping absence absent. Written out because an `option` of an `option` is not a chord and this language composes the two by hand.
- `fn stage_music(policy: ChordClass -> Music, content: Option<ChordClass>) -> Music` — Sound one stage of a skeleton under a caller's voicing policy. The policy is a function because the library has no opinion: a stage that cannot be voiced from the bass the caller named is silence here, and the caller can see that it was.

## `std::transformational`

- `fn triad_root(refined: Triad) -> NoteName` — The root of a triad, spelled.
- `fn triad_third(refined: Triad) -> NoteName` — The third, which is the tone that says which triad this is: a major third above the root for the major triad, a minor third for the minor.
- `fn triad_fifth(refined: Triad) -> NoteName` — The fifth, which is perfect in both triads and so needs no question.
- `fn major_triad_on(root: NoteName, fallback: Triad) -> Triad` — The major triad rooted on a pitch class. `fallback` is returned only if a major triad were not a triad, which is why it is the argument the caller already had: an impossible answer is still a triad and still spelled from somewhere real, rather than a silence that would have to be explained. Every transformation below is total because of this function and the next.
- `fn minor_triad_on(root: NoteName, fallback: Triad) -> Triad` — The minor triad rooted on a pitch class, on the same terms.
- `fn itself(refined: Triad) -> Triad` — The present case of the two constructors above.
- `fn parallel(refined: Triad) -> Triad` — P, parallel: keep the root and the fifth, move the third by a chromatic semitone. The C major triad and the C minor triad, which share a letter and a key signature's worth of difference.
- `fn leading_tone(refined: Triad) -> Triad` — L, Leittonwechsel: keep the third and the fifth, and move the remaining tone by a diatonic semitone. From a major triad that is its own third made a root — C major goes to E minor, the root `c` falling to `b`; from a minor triad the fifth rises, E minor back to C major.
- `fn relative(refined: Triad) -> Triad` — R, relative: keep the root and the third, move the remaining tone by a whole step. C major goes to A minor and A minor back to C major, which is the relative pair every key signature already names.
- `fn slide(refined: Triad) -> Triad` — S, slide: keep the third, move the root and the fifth by a chromatic semitone. C major goes to C sharp minor. Written as `L P R` because that is what it is — OMT 072 introduces S, N, and H as the named compositions worth having, not as further generators.
- `fn nebenverwandt(refined: Triad) -> Triad` — N, Nebenverwandt: a major triad and its minor subdominant, C major to F minor. `R L P`.
- `fn hexatonic_pole(refined: Triad) -> Triad` — H, the hexatonic pole: the triad sharing no tone at all with its argument, C major to G sharp minor. `L P L`.
- `fn then(first: Triad -> Triad, second: Triad -> Triad, refined: Triad) -> Triad` — Two transformations, applied in the order written.
- `fn chain(steps: List<Triad -> Triad>, start: Triad) -> Triad` — A chain of transformations, applied left to right from a starting triad. Composition is what a chain is — no transformation here is a keyword or a special form, so a list of them is ordinary data and this is an ordinary fold.
- `fn applied(operation: Triad -> Triad, carried: Triad) -> Triad` — One step of `chain`.
- `fn unspelled(spelled: NoteName) -> Pc12` — Forget one spelling. Named for what it does to a pitch class rather than for the domain it lands in, so that a piece may import this and `std::post_tonal::pcset` together.
- `fn triad_tones(refined: Triad) -> List<NoteName>` — The three tones of a triad, spelled, from the root upward.
- `fn triad_classes(refined: Triad) -> PcSet12` — The projection into the chromatic quotient: the triad as a set of three unspelled pitch classes. This is where a finite group claim becomes sayable. `dbb` major and `c` major are two triads and one set, so a cycle that fails to close in spelling closes here, and the failure and the closing are both facts an author can hold at once rather than one hiding the other.

## `std::voicing`

- `fn close_position(content: ChordClass, bass: Pitch) -> Option<Voicing>` — Stack the class upward from an absolute bass, one member per octave position, taking each next member at the first pitch above the last. This is the one policy `stack c4 major7/2` desugars to.
- `fn drop_position(content: ChordClass, bass: Pitch, from_top: Nat) -> Option<Voicing>` — Close position with one upper note dropped an octave, counted from the top: `drop_position(content, bass, 2)` is the drop-2 voicing.
- `fn voiced_as(content: ChordClass, pitches: List<Pitch>) -> Option<Voicing>` — The voicing an explicit list of written pitches spells, when those pitches really do voice the class: ascending, distinct, and every one a member. This is how a writer voices by hand.
- `fn pitches_of(chosen: Voicing) -> List<Pitch>` — Every sounding pitch of a voicing, lowest first.
- `fn lowest_of(chosen: Voicing) -> Pitch` — The lowest sounding pitch. A voicing always has one, which is why this is not an `option`.
- `fn chord_of(chosen: Voicing) -> ChordClass` — The chord class this voicing voices.
- `fn inversion_of(chosen: Voicing) -> Option<Nat>` — Which member is in the bass, counted from zero, when the bass is a member at all. Absent for a slash bass, which is not an inversion.
- `fn omitting(chosen: Voicing, position: Nat) -> Option<Voicing>` — Drop one numbered member from a voicing, keeping the class it voices. The chord class is unchanged: an omission is a choice about what sounds, not a claim that the chord is a different chord.
- `fn rootless(chosen: Voicing) -> Option<Voicing>` — The rootless voicing a pianist plays under a bass player: the root, in position zero, is the note removed, and it is named here rather than left implicit.
- `fn sound_for(chosen: Voicing, held: Duration) -> Music` — Sound a chosen voicing for a written length. This is the only way a chord class becomes notes.
