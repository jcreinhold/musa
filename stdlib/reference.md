# Musa standard library 1

This reference is generated from what the compiler records about each bundled declaration — the same record an
editor shows on hover. Standard definitions are ordinary Musa definitions; importing a module is explicit and never
searches the filesystem.

## `std::algebra`

- `record Group(G: Type)` — A set with an associative composition, a unit, and inverses.  Laws: `compose(unit(g), g)` and `compose(g, unit(g))` are `g`; composition is associative; `compose(g, inverse(g))` is `unit(g)`.  Not every set of movers is one. A duration composes and has a zero and has no inverse, because a duration is a length and not a displacement, and `Action` below is stated so that such a mover is still admitted.  `unit` takes an element, which looks redundant and is not: the element is read for its *type* and never for its value, so `P5.unit()` is `P1` and so is `m2.unit()`. Written as a field it is what a caller of the record supplies; written as `Interval.unit` it is what `P5.unit()` finds.
- `record Action(X: Type, G: Type)` — A carrier moved by a set of movers.  Laws: `act(x, unit(g))` is `x`, and `act(x, compose(g, h))` is `act(act(x, h), g)`. Both are stated over whatever unit and composition the mover has, so a monoid acting is an action here and not an approximation of one; where the mover is also a `Group`, the action is invertible and the second law reads backwards as well as forwards.  The second law is what makes a transformation an object rather than a call: if composing two movers and acting once agrees with acting twice, a chain can be built before anything is applied.  **The carrier comes first** (`docs/rules/style-guide.md` §6). One mover moves several carriers — an interval moves both a pitch and a spelled pitch class — and the carrier is what a reader is asking about, so it is what the parameter list says first. It is also where `x.act(g)` looks: the receiver decides which `act` runs.
- `record Torsor(P: Type, V: Type)` — A carrier on which the movers act *simply transitively*: every ordered pair of points is joined by exactly one mover.  That uniqueness is the whole content, so `difference` is the whole record. The movement itself is an `Action`'s `act` and is not declared twice; the law joining them is `act(a, difference(a, b))` is `b`, for every `a` and `b`.  **Two points never add.** `a + b` for two pitches or two positions is the single arithmetic error the point/mover split exists to catch, so there is no field here that could spell it.
- `fn already({X: Type}, same: X -> X -> Bool, built: List(X), value: X) -> Bool` — Whether `built` already holds a member equal to `value`.
- `fn adding({X: Type}, same: X -> X -> Bool, built: List(X), value: X) -> List(X)` — `built`, with `value` at its front where it is not already there.  Prepended, and the fold's answer reversed once at the end. Appending instead would rebuild the whole list at every new member, which is a second quadratic on top of the comparison's — and at a division of twelve the row group has forty-eight members, where a spare quadratic is the step budget. The order a caller sees is unchanged: the group's own, first occurrence winning, which is what makes a canonical representative a question about the answer rather than about this function.
- `fn orbit({X: Type}, {G: Type}, action: Action(X, G), same: X -> X -> Bool, group: List(G), point: X) -> List(X)` — The distinct images of `point` under every operation in `group`.  Order is the group's own, first occurrence winning, and duplicates are dropped — which is what makes the *length* of this the count that `row12_forms` was: how many distinct rows the labelled forms produce.
- `fn stabilizer({X: Type}, {G: Type}, action: Action(X, G), same: X -> X -> Bool, group: List(G), point: X) -> List(G)` — The operations in `group` that leave `point` where it is.  Its length times the orbit's is the size of the group, which is the orbit-stabilizer accounting `110-row-properties.md` asks the four form labels to do on their own.

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

- `record TonalContext: Type` — A tonal context is the small bundle of facts that always travel together: what key a passage is in, which collection it steps through, how a numbered degree is spelled in register, and how a chord class is voiced. Passing them one at a time is how they drift apart — Open Music Theory `105-diatonic-modes.md` names the collection, `017-triads.md` the chords, and neither is meaningful without the other.  A record, because a record is all a `signature` ever was: the fields are the members, a value of the type is the structure, and a function that takes one is the functor.
- `let c_major_home: Option(Frame)` — The register C major's degrees are spelled in. Private: a context promises spelled pitches, not the frame it spells them from, so moving this one changes nothing anyone outside can name. That is what sealing was, written with the visibility the language already had.
- `fn c_major_spell(ordinal: Nat) -> Option(Pitch)` — C major's degree spelling, read out of the register above. Private for the same reason the register is: what a context promises is the function's answers, not which function it happens to be.
- `fn c_major_voicing(content: ChordClass) -> Option(Voicing)` — C major's close-position voicing, from middle C upward.
- `let c_major: TonalContext` — C major, spelled from middle C.
- `let a_minor_home: Option(Frame)` — The register A natural minor's degrees are spelled in, an octave and a third below C major's.
- `fn a_minor_spell(ordinal: Nat) -> Option(Pitch)` — A natural minor's degree spelling, read out of its own register.
- `fn a_minor_voicing(content: ChordClass) -> Option(Voicing)` — A natural minor's close-position voicing, from the A below middle C.
- `let a_natural_minor: TonalContext` — A natural minor, spelled from the A below middle C. The same four fields, answered differently — which is what makes the two values interchangeable everywhere a `TonalContext` is asked for.

## `std::core`

- `fn identity_ratio(value: Ratio) -> Ratio` — Return an exact rational unchanged. This is useful when a public API wants to say explicitly that it preserves a proportion.
- `fn identity_nat(value: Nat) -> Nat` — Return a natural number unchanged.
- `fn compose({A: Type}, {B: Type}, {C: Type}, after: B -> C, before: A -> B) -> A -> C` — Apply `before`, then `after`. The composition is the answer, not a value it was applied to: `compose(f, g)` names the function, and `compose(f, g)(x)` runs it.
- `fn compose_music(first: EventTrack(WrittenTime) -> EventTrack(WrittenTime), second: EventTrack(WrittenTime) -> EventTrack(WrittenTime)) -> EventTrack(WrittenTime) -> EventTrack(WrittenTime)` — Apply the second musical transformation, then the first.
- `fn compose_pitch(first: Pitch -> Pitch, second: Pitch -> Pitch) -> Pitch -> Pitch` — Apply the second pitch function, then the first.

## `std::cyclic`

- `let chromatic: Cycle(12)` — The chromatic cycle: twelve equal divisions of the octave.  Here rather than in `std::post_tonal` because twelve is not a fact about pitch. `068-equal-divisions-of-the-octave.md` divides the octave into n parts and asks which n admit which symmetries; twelve is the n common practice chose, and the two below are the two the corpus asks for next.
- `let quartertone: Cycle(24)` — The quarter-tone cycle, which quarter-tone practice and most modelling of Turkish and Arabic theory use.
- `fn cycle_of(n: Nat, place: Cyclic(n)) -> Cycle(n)` — The cycle one position belongs to.  Total and free: a member's index *is* its cycle's size, so this reads the witness back out of a value that already carried it.
- `fn size_of(n: Nat, cycle: Cycle(n)) -> Nat` — How many positions a cycle has.
- `fn number_of(n: Nat, place: Cyclic(n)) -> Nat` — The number a position names: zero through `n - 1`.
- `fn embed(besides: Nat, j: Nat) -> Cyclic(Succ(besides))` — The position numbered `j`, in a cycle of `Succ(besides)` positions.  Private because it is only correct where `j` is at most `besides`, which is what `onward` and `backward` guarantee of everything handed to it. A `j` past the end folds back to the origin rather than failing, so the function is total for the checker as well as for the caller.
- `fn without(value: Nat, amount: Nat) -> Nat` — `value - amount`, and zero where that would run below zero.  `Nat.sub` answers an `Option` because a whole number has no negatives to land in, which is the right refusal at the operator and the wrong shape here: every subtraction below has already been guarded by the comparison beside it, so the absent case is unreachable and zero is the only thing it could mean.
- `fn folded(size: Nat, fuel: Nat, value: Nat) -> Nat` — `value`, with `size` taken off it as often as it will go.  Subtracting rather than counting up. An earlier reading of this module walked the cycle one position at a time, on the argument that the language has no division on whole numbers — true, and it does not need one, but a walk costs a reduction step per position and a group orbit asks for the residue of every member under every operation. At a division of twelve that walk is the whole step budget, so the residue is taken by subtraction instead: a number already standing in the cycle costs one comparison.  `fuel` is what makes the descent structural. Each turn takes at least one off `value`, because `size` is `Succ(besides)` and never zero, so counting the turns down from `value` itself can only run out after the answer is found — and the totality checker sees a `Nat` getting smaller rather than a subtraction it would have to reason about.
- `fn reduced_in(size: Nat, value: Nat) -> Nat` — `value` reduced into a cycle of `size` positions.
- `fn differing(size: Nat, left: Nat, right: Nat) -> Nat` — `left - right`, reduced into a cycle of `size` positions.
- `fn place_in(n: Nat, cycle: Cycle(n), number: Nat) -> Cyclic(n)` — The position `number` names, reduced into the cycle.  `place_in(chromatic, 13)` and `place_in(chromatic, 1)` are one position, because they are one residue. Total: the cycle value is the witness that there is a position to answer with.
- `fn transposed(n: Nat, place: Cyclic(n), steps: Nat) -> Cyclic(n)` — T_i: `place` moved `steps` positions forward.
- `fn inverted(n: Nat, place: Cyclic(n), about: Nat) -> Cyclic(n)` — I_j: `place` reflected through the position numbered `about`.  `about - place`, walked backward, which is the same reflection `101-pitch-class-sets-normal-order-and-transformations.md` writes as `n - x` for I_0.
- `fn moved(n: Nat, place: Cyclic(n), by: Ti) -> Cyclic(n)` — One operation of the T/I group, applied.
- `fn rotation(n: Nat) -> Action(Cyclic(n), Ti)` — The action of the T/I group on the cycle, as the value `std::algebra` names.  A record and not a class: prompt 143 made a structure a value, so this can be passed to `orbit` and `stabilizer` beside any other action.
- `fn positions(n: Nat, cycle: Cycle(n)) -> List(Cyclic(n))` — Every position of the cycle, ascending from zero.
- `fn transpositions(n: Nat, cycle: Cycle(n)) -> List(Ti)` — The n transpositions of the cycle.
- `fn inversions(n: Nat, cycle: Cycle(n)) -> List(Ti)` — The n inversions of the cycle.
- `fn class_between(n: Nat, cycle: Cycle(n), left: Nat, right: Nat) -> Nat` — The smaller of the two ways round the cycle from `left` to `right`.  What `103-interval-class-vectors.md` calls an interval class: five up and seven down are one class at twelve, because the chapter's own reason is that an interval is the same interval heard the other way round. At an even `n` the half-cycle is its own opposite, which is why the tritone counts once.
- `fn class_count(n: Nat, cycle: Cycle(n)) -> Nat` — How many interval classes a cycle of `n` has: `n / 2`, rounded down.  Six at twelve, and `03-musical-domains.md` §5's derived count rather than §4's literal. Written by counting down two at a time, because the language has no division on whole numbers and needs none for this.
- `fn halved(count: Nat) -> Nat` — `count / 2`, rounded down.
- `fn ti_operations(n: Nat, cycle: Cycle(n)) -> List(Ti)` — The whole T/I group, enumerated: 2n operations, and 2n whatever n is.  Twenty-four at twelve, forty-eight at twenty-four. The number is derived and not written down, which is `03-musical-domains.md` §5's "every count that was a literal in §4 is now derived".  The list and the `Group(Ti)` below are two different things and both are wanted: `orbit` and `stabilizer` walk the elements, and composition is what makes a chain of transformations an object before anything is applied.
- `fn plus_in(n: Nat, cycle: Cycle(n), left: Nat, right: Nat) -> Nat` — `left + right`, reduced into the cycle.  Both arguments are reduced on the way in, so a caller may hand over a label written past the end — `Transpose(15)` at twelve — and get the operation it names.
- `fn minus_in(n: Nat, cycle: Cycle(n), left: Nat, right: Nat) -> Nat` — `left - right`, reduced into the cycle, wrapping at zero.
- `fn number_moved(n: Nat, cycle: Cycle(n), number: Nat, by: Ti) -> Nat` — The number a T/I operation sends `number` to.  T_i adds and I_j reflects, which is the whole of the dihedral action on a cycle. Written on numbers rather than on `Cyclic(n)` because a set, a row, and a bell pattern each move a whole list at once, and each of them holds its members as the numbers they reduce to.
- `fn ti_compose(n: Nat, cycle: Cycle(n), later: Ti, earlier: Ti) -> Ti` — Apply `earlier`, then `later`.  The four cases are the dihedral multiplication table, and each is one line of arithmetic on the labels: T_a T_b is T_(a+b), T_a I_b is I_(a+b), I_a T_b is I_(a-b), and I_a I_b is T_(a-b). The last is the one that makes this a group rather than two unrelated families — two reflections compose to a rotation — and it is why `Ti` is one `data` with two cases.
- `fn ti_inverse(n: Nat, cycle: Cycle(n), by: Ti) -> Ti` — The operation that undoes this one.  A reflection is its own inverse, which is why the second case only reduces the label; a rotation's inverse is the rotation the other way.
- `fn ti_group(n: Nat, cycle: Cycle(n)) -> Group(Ti)` — The T/I group of a cycle, as the value `std::algebra` names.  `unit` takes an element and reads it for its type alone, which is the record's own convention: every operation of a cycle has the same unit, and it is the rotation by nothing.

## `std::harmony`

- `fn chord_rooted_on(content: ChordClass, root: NoteName) -> ChordClass` — Re-root a chord class, keeping its type. `chord c major7` on `eb` is an E-flat major seventh, spelled from E-flat.
- `fn root_of(content: ChordClass) -> NoteName` — The pitch class a chord class is rooted on. This is the root, which is not the bass: a designated bass is asked for separately.
- `fn bass_of(content: ChordClass) -> Option(NoteName)` — The bass a chord class designates, when it designates one. Absent means no bass was chosen — it does not mean the root.
- `fn members_of(content: ChordClass) -> List(Interval)` — The spelled intervals above the root, lowest first, beginning at the unison. Spelled: a major third is a third, never a diminished fourth.
- `fn inversion(content: ChordClass, position: Nat) -> Option(ChordClass)` — A true inversion: the numbered member becomes the designated bass. Positions are counted from zero, so position one is first inversion. Absent when the class has no such member.
- `fn slash_bass(content: ChordClass, bass: NoteName) -> ChordClass` — A slash bass: a designated bass that need not be a member at all. `chord_over(chord c major, d)` is C over D, and the D is not a chord tone. This is a different construction from an inversion, and stays one.
- `fn as_triad(content: ChordClass) -> Option(Triad)` — The triad refinement, when the content really is a major or minor triad. This is the domain a neo-Riemannian transformation acts on, and the proof that it applies is this `option` being present.
- `fn triad_content(refined: Triad) -> ChordClass` — Forget the refinement: every triad is a chord class.
- `fn is_triad(content: ChordClass) -> Bool` — Whether a chord class is a major or minor triad.
- `fn triad_is_present(refined: Triad) -> Bool` — The present case of `is_triad`: a refinement that exists is a triad, whichever of the two it turned out to be.
- `fn is_major(refined: Triad) -> Bool` — Which of the two a triad is. Total, and a `bool` rather than a partial answer, because the refinement admitted exactly two chord classes: not major is minor here, and only here. Every transformation in `std::transformational` branches on this, since which way a voice moves is the whole content of the transformation.

## `std::indexed`

- `fn row_top({A: Type}, size: Nat, held: Row(A, Succ(size))) -> A` — The first element of a row that has one.  **Total, and with one arm.** `Empty` stands at `Zero` and this row stands at `Succ(size)`, so index unification rules that constructor out and there is no case to write for it — which is the difference between this and the `Option(A)` the same function returns today.
- `fn row_later({A: Type}, size: Nat, held: Row(A, Succ(size))) -> Row(A, size)` — Everything after the first element, one shorter.

## `std::list`

- `fn counting_from(count: Nat, first: Nat) -> List(Nat)` — Finite lists: how one is built, and the two ways one is read.  Every function here is written over `List`'s own constructors and nothing else. That is the point of the module rather than an implementation detail: `List` is a declared family with a generated recursor, so a fold over it is ordinary source, and a compiler builtin for one would be a second reduction rule for a type that already has one. `02-core-calculus.md` §5.8 admits a builtin that hides something; a traversal of a list whose constructors the language writes hides nothing.  # Where the folds went  A list is read by `xs.fold_from_start(seed, combine)` and `xs.fold_from_end(seed, combine)`, which are `List`'s own two definitions and not functions of this module. One spelling per direction, and every container that can be walked answers to the same two words in its own namespace — which is what a per-type `list_fold_from_start` beside an `option_fold` beside a `nat_fold` could never be.  What stays here is what a fold does not say as well. `map` and `filter` below are written over `List`'s own constructors and cost the length; building an answer by folding and appending costs its square, because appending to the end of a list walks it. The natural numbers from `first`, `count` of them, ascending.  The accumulator is `first` and it comes after the argument the recursion descends on, because §2.4's measure holds everything written *before* that argument fixed: a walk that both descends and accumulates has to descend first and accumulate later. `Succ` rather than `nat_add`: the successor constructor *is* "one more", so counting needs no arithmetic and no failure case.
- `fn range(count: Nat) -> List(Nat)` — The natural numbers from zero up to, but not including, `count`.
- `fn repeated({A: Type}, value: A, count: Nat) -> List(A)` — One value, `count` times, as finite data.  Not `repeat`: that word opens a repeated passage in a score, so it is a statement keyword and cannot also be a function name. The past participle is what the list *is* rather than what a player does.
- `fn map({A: Type}, {B: Type}, function: A -> B, values: List(A)) -> List(B)` — Apply `function` to every member, in order.
- `fn filter({A: Type}, predicate: A -> Bool, values: List(A)) -> List(A)` — Keep the members for which `predicate` answers true, in order.
- `fn concat({A: Type}, first: List(A), second: List(A)) -> List(A)` — The members of `first`, then the members of `second`.  Costs the length of `first` and nothing for `second`, which is why the module docs say an answer built by folding and appending costs the square: this is the append, and folding it over n members walks n prefixes.
- `fn length({A: Type}, values: List(A)) -> Nat` — How many members a list has.
- `fn reversing({A: Type}, values: List(A), built: List(A)) -> List(A)` — The members in the opposite order.  Written over an accumulator rather than over `concat`, because appending one member at a time to the end costs the square of the length and this costs the length. The accumulator comes after the argument the recursion descends on, for §2.4's reason.
- `fn reverse({A: Type}, values: List(A)) -> List(A)` — The members in the opposite order.
- `fn take({A: Type}, count: Nat, values: List(A)) -> List(A)` — The first `count` members, or all of them where there are fewer.
- `fn drop({A: Type}, count: Nat, values: List(A)) -> List(A)` — Everything but the first `count` members, or nothing where there are fewer.
- `fn rotated({A: Type}, count: Nat, values: List(A)) -> List(A)` — The members from `count` onward, then the members before it.  A rotation and not a shift: nothing is lost, so rotating by the length gives the list back. `102-set-class-and-prime-form.md`'s normal order is chosen from among these.
- `fn rotations({A: Type}, values: List(A)) -> List(List(A))` — Every rotation of the list, starting with the list itself.
- `fn any({A: Type}, predicate: A -> Bool, values: List(A)) -> Bool` — Whether any member answers true.
- `fn all({A: Type}, predicate: A -> Bool, values: List(A)) -> Bool` — Whether every member answers true.
- `fn naturals(count: Nat) -> List(Nat)` — The natural numbers from zero up to, but not including, count.
- `fn map_pitches(function: Pitch -> Pitch, values: List(Pitch)) -> List(Pitch)` — Apply one pitch function to every member of a finite pitch list.
- `fn filter_pitches(predicate: Pitch -> Bool, values: List(Pitch)) -> List(Pitch)` — Keep the pitches for which predicate returns true.
- `fn repeat_music(value: EventTrack(WrittenTime), count: Nat) -> List(EventTrack(WrittenTime))` — Repeat one contextual music value count times as finite data.

## `std::notation::staff`

- `fn staff_item_fold({A: Type}, nothing: A, sounded: Nat -> StaffEvent -> A -> A, barred: Nat -> Meter -> A -> A -> A, slurred: Nat -> A -> A -> A, grouped: Nat -> Nat -> Nat -> A -> A -> A, looped: Nat -> Nat -> A -> A -> A, volta: Nat -> Nat -> A -> A -> A, items: StaffItem) -> A` — Read a written sequence from the end, one case per constructor.  A nested form hands its case two answers, its body's and its rest's, which is what lets a caller decide independently what a bar does to what is inside it and what follows it. The sequence is the last argument because that is how the corpus already reads; the recursion descends there and every argument before it is fixed, which is what §2.4's measure asks.  None of the parameters is named for the constructor it answers: `repeat` and `bar` are keywords, and a fold that spelled half its cases one way and half another would read worse than one that spells all seven as what the case *did*.
- `fn folded_spans({A: Type}, spans: WrittenSpans, empty: A, combine: Nat -> Position(WrittenTime) -> Duration(WrittenTime) -> Tie -> A -> A) -> A` — The catamorphism's equation, written as it reads: `combine` is handed what the spans after this one produced.  `combine` and not `step`, which is a keyword — the scale step is the word the language already spent.  The spans come first here and last in the public spelling, for the reason `std::list` states at length: the termination measure holds the arguments before the recursive position fixed, so the argument that descends is the first one.
- `fn written_spans_fold({A: Type}, empty: A, combine: Nat -> Position(WrittenTime) -> Duration(WrittenTime) -> Tie -> A -> A, spans: WrittenSpans) -> A` — Read a sequence of spans from the end: `combine` sees each span together with what the spans after it produced, seeded with `empty`.  A fold over a declared family is ordinary source. The core names a family's recursor after the family and compiles `match` to it, so this is written once, here, beside the family it reads.
- `record Realization: Type` — What a traversal reached: the spans it found, and where it stopped.
- `fn folded_spelling({A: Type}, spelled: Spelled, empty: A, combine: Nat -> Position(WrittenTime) -> WrittenDuration -> A -> A) -> A` — The catamorphism over spelled spans, its list first for the same reason [`folded_spans`] gives.
- `fn spelled_fold({A: Type}, empty: A, combine: Nat -> Position(WrittenTime) -> WrittenDuration -> A -> A, spelled: Spelled) -> A` — Read spelled spans from the end: `combine` sees each span's chosen written value together with what the spans after it produced.
- `record Dotted: Type` — A dotted value in the making: what it covers so far, and what the next dot would add.
- `fn ratio_of(count: Nat) -> Ratio` — The exact rational a whole number names.  A note value is a division of the whole note, so the package needs one; the language has no coercion from `Nat` to `Ratio`, deliberately, so the conversion is written rather than assumed. `1/1` and not `1` for the same reason one step down: a literal infers, and the whole-number token is a `Nat` wherever it stands.
- `fn exact_quotient(amount: Ratio, parts: Nat, because: Text) -> Result(Ratio, Text)` — `amount` shared out among `parts` of them, or `because` when there are none of them.  The zero is caught here rather than left to `ratio_div`. A refusal is the core's own sentence and is not data a package can answer with, and this partiality belongs to the reader: `NoteValue(0, 0)` and `Tuplet(_, 0, 2, …)` are both things someone wrote down, so the package says what is wrong with each in a staff's words and hands back a `Result` its callers thread.  The count is taken apart rather than compared against zero, and the difference is not stylistic. A whole number is `Zero` or a `Succ`, and only the second is a division; a comparison would answer the same question, but the division sits in an arm that binds nothing and an arm that binds nothing is an ordinary expression, evaluated where it stands. Written this way the division lives under a binder and is reached only when the count is one the reader can divide by.
- `fn division_span(division: Nat) -> Result(Ratio, Text)` — What one undotted note value covers: the whole note divided.
- `fn dotted_span(base: Ratio, dots: Nat) -> Ratio` — What a value covers once its dots are added: each dot adds half of what the dot before it added.  The half is named because it is wanted twice — added on, and kept as what the next dot halves — and naming it is all `dotted_by` ever was.
- `fn written_span(value: WrittenDuration) -> Result(Duration(WrittenTime), Text)` — The exact span a written value covers, or the sentence saying why none does.
- `fn meter_span(beats: Meter) -> Result(Duration(WrittenTime), Text)` — The bar a meter measures.
- `fn joined_spans(first: WrittenSpans, second: WrittenSpans) -> WrittenSpans` — One sequence of spans after another, in the order they were written.
- `fn stopped(here: Position(WrittenTime)) -> Result(Realization, Text)` — A traversal that has run out of items reaches nothing and stays where it is.
- `fn nested(body: (Position(WrittenTime) -> Result(Realization, Text)), after: (Position(WrittenTime) -> Result(Realization, Text)), here: Position(WrittenTime)) -> Result(Realization, Text)` — A nested form — a bar, a slur, a tuplet's siblings, a repeat, an ending — is its body followed by what comes after it.
- `fn joined_ties(spans: WrittenSpans) -> Result(WrittenSpans, Text)` — A tie joins what is written to whatever sounds next, wherever that is.  This is a pass over the flat spans rather than a case of the traversal, because the traversal is inside a bar when it meets the tie and what the tie reaches is usually in the next one. Joining last is what lets a tie cross a barline, a slur, or a repeat without any of the three knowing about ties.
- `fn placed(anchor: Nat, held: WrittenDuration, tied: Tie, here: Position(WrittenTime), after: (Position(WrittenTime) -> Result(Realization, Text))) -> Result(Realization, Text)` — One sounding item: its span starts here, and what follows starts after it.  The span is measured once and named, because it is wanted twice — once to say where what follows begins, once to record what was written. Naming it is all `placed_span` ever was: a parameter used to be how this language named a value used twice, and a `let` is how it is now.
- `fn sounded_span(anchor: Nat, event: StaffEvent, here: Position(WrittenTime), after: (Position(WrittenTime) -> Result(Realization, Text))) -> Result(Realization, Text)` — What one written event covers.  A grace note refuses. How long a grace takes, and what it takes it from, is a performance profile's choice, and a package that guessed here would be answering a performance question with a notation answer.
- `fn rescaled(factor: Ratio, here: Position(WrittenTime), point: Position(WrittenTime)) -> Position(WrittenTime)` — A point moved by a factor of its distance from the origin, then placed against `here`. This is how a tuplet's inside becomes its outside.
- `fn rescaled_spans(factor: Ratio, here: Position(WrittenTime), spans: WrittenSpans) -> WrittenSpans` — Every span of a tuplet's body, moved and shortened by the tuplet's factor.
- `fn tuplet_factor(played: Nat, against: Nat) -> Result(Ratio, Text)` — Three in the time of two is a factor of two thirds: the written values stay what they are, and the time they take does not.  A tuplet that plays nothing is refused for the reason `division_span` gives: nothing in the time of two is not a tuplet, and the reader who wrote it should hear that rather than the core's division rule.
- `fn tupleted(played: Nat, against: Nat, body: (Position(WrittenTime) -> Result(Realization, Text)), after: (Position(WrittenTime) -> Result(Realization, Text)), here: Position(WrittenTime)) -> Result(Realization, Text)` — A tuplet realizes its body against its own origin and then places the result, so that the factor multiplies distances rather than positions.
- `fn walked(items: StaffItem) -> (Position(WrittenTime) -> Result(Realization, Text))` — The traversal itself: one case per constructor, answering with what the sequence covers once someone says where it starts.  The fold's answer is a function because the fold is bottom-up and time runs the other way: what a suffix covers is known before where it begins, so each case answers "given a starting point, this is what I reach" rather than a value that would have needed the point already.
- `fn document_items(page: StaffDocument) -> StaffItem` — What is written on the staff.  `StaffDocument` is a family with one constructor and not a record, because the adapter builds one positionally from syntax it assembled. A family is not projected at a field, so the two fields this module reads are read by a case, once each, rather than at every use.
- `fn document_spelling(page: StaffDocument) -> Spelling` — Which written value `engrave` is to choose for this document's spans.
- `fn realize(document: StaffDocument) -> Result(Realization, Text)` — What a document covers, in exact written time.  A repeat and an ending are traversed once, because written time counts the page and not the performance: how many times a repeat sounds is a reading of the score, and this answers what the score says.  Ties are joined afterwards, over the flat spans, so that a tie reaches whatever sounds next however deeply either of them is nested.
- `fn written_extent(document: StaffDocument) -> Result(Duration(WrittenTime), Text)` — How much written time a document covers altogether.
- `let readable_spellings: List((WrittenDuration, Ratio))` — The note values a reader is expected to read at sight, each beside the span it covers, shortest spelling first: no dots before one dot, and one before two.  The spans are written down rather than derived. `written_span` answers the same rationals, but it answers them by counting: `ratio_of` walks a `Nat` one unit at a time, because the language has no coercion from a whole number to a rational and this module would rather write the conversion than assume it. Asking that question about a sixty-fourth costs sixty-four additions, and `first_covering` would ask it again at every step of a twenty-one-value search, nested inside whatever depth the caller had already reached. A table the module already knows is a table the module should write down.
- `let readable_values: List(WrittenDuration)` — The note values a reader is expected to read at sight, shortest spelling first: no dots before one dot, and one before two.
- `fn first_covering(spellings: List((WrittenDuration, Ratio)), held: Ratio) -> Option( WrittenDuration, )` — The first of `spellings` whose span is `held` exactly, if one is.  A search and not a fold: a fold would have to be seeded with a bare `None`, and a constructor is read against the family it is expected at, which a fold's answer type does not say until the fold is done. Here both answers are checked against the type this signature states. It also stops at the first match, which the fold could not.
- `fn readable(held: Duration(WrittenTime)) -> Option(WrittenDuration)` — The first readable value that covers a span exactly, if one does.
- `fn spelled_as(policy: Spelling, held: Duration(WrittenTime)) -> Result(WrittenDuration, Text)` — Which written value a realized span is printed as.  This is the choice expansion deliberately left open, and it is made here so that a span nothing readable spells is a complaint about the document's spelling policy rather than a note quietly rounded.
- `fn engrave(document: StaffDocument) -> Result(Spelled, Text)` — Every realized span with the written value chosen for it.
- `fn spelled_spans(policy: Spelling, spans: WrittenSpans) -> Result(Spelled, Text)` — The spelling choice, made once per span and refused once for all of them.

## `std::performance`

- `fn some_control({kind: ControlKind}, control_key: ControlKey(kind), value: ControlValue(kind)) -> SomeControl` — `kind` is intentionally omitted at call sites. It is solved only by unifying the two indexed arguments; a disagreement is a normal type error and an unresolved kind is never defaulted.
- `let expression: ControlKey(Normalized)` — Sustained relative-intensity intention, independent of gain or velocity.
- `let emphasis: ControlKey(Normalized)` — Per-note attack-salience intention.
- `let separation: ControlKey(Normalized)` — Per-transition perceptual-detachment intention.
- `let brightness: ControlKey(Normalized)` — Continuous relative spectral-brightness intention.
- `let sustain: ControlKey(Normalized)` — Continuous continuation-after-release intention.
- `let phrase_relation: ControlKey(PhraseConnection)` — Typed grouping relation among the gestures of a phrase.
- `record GestureId: Type` — Stable identity shared by a gesture and its separate host lineage entry.
- `record TechniqueRequest: Type` — A request remains symbolic even when an instrument also supplies a numeric fallback. Namespacing permits libraries to extend the vocabulary without a closed Rust enum.
- `record HairpinView: Type` — A notation view contains every musical input visible to an ordinary profile call. It deliberately contains no host provenance, frame, primitive, or DSP address. The bridge supplies one value per written occurrence.
- `record NotationView: Type` — Complete musical notation input presented to one profile call.
- `record DynamicRule: Type` — One source-declared reading of a written dynamic marking.
- `record MarkRule: Type` — One source-declared reading of a written articulation mark.
- `record LegacyMarkTiming: Type` — Legacy surface timing declarations transcribed without host interpretation.
- `record PerformanceProfile: Type` — A finite, storable collection of source interpretation rules.
- `fn timing_rule(timing: LegacyMarkTiming) -> MarkRule` — Translate one compatibility timing declaration into source-owned intent.
- `fn timing_rules(timings: List(LegacyMarkTiming)) -> List(MarkRule)` — Translate compatibility timing declarations in their written order.
- `fn profile_from_legacy(dynamics: List(DynamicRule), timings: List(LegacyMarkTiming)) -> PerformanceProfile` — Build the compatibility surface profile as ordinary source policy.
- `record ProfileResult: Type` — Exact musical intent returned by one finite profile evaluation.
- `record MarkTiming` — Temporary temporal compatibility fields kept outside gesture identity.
- `record InterpretationRequest: Type` — One finite request presented to the source interpreter by the host bridge.
- `record PerformanceInterpretationArtifact: Type` — Versioned checked results returned to the provenance/track bridge.
- `fn dynamic_level(rules: List(DynamicRule), sought: Text) -> Ratio` — Find a declared dynamic level, returning neutral expression when absent.
- `fn marked_controls(rules: List(MarkRule), marks: List(Text)) -> List(SomeControl)` — Collect the exact controls declared for the marks on one notation view.
- `fn marked_techniques(rules: List(MarkRule), marks: List(Text)) -> List( TechniqueRequest, )` — Collect symbolic techniques without collapsing them into numeric controls.
- `fn mark_timing(rules: List(MarkRule), marks: List(Text), gate: Ratio, attack: Ratio, hold: Ratio) -> MarkTiming` — Fold the legacy temporal projections declared by matching mark rules.
- `fn member_text(sought: Text, values: List(Text)) -> Bool` — Whether one exact spelling occurs in a finite list.
- `fn level_of(policy: PerformanceProfile, view: NotationView) -> Ratio` — The prevailing expression level a profile gives one notation view.
- `fn connection_controls(policy: PerformanceProfile, view: NotationView) -> List( SomeControl, )` — Realize an explicit or profile-default phrase relation as one typed control.
- `fn hairpin_expression(from: Ratio, target_level: Ratio, reached: Ratio) -> Ratio` — Exact affine interpolation for a hairpin sample. `reached` is the result of applying its source `Progress` at normalized local time; no sampling density or physical mapping is chosen here.
- `fn interpret(policy: PerformanceProfile, view: NotationView) -> ProfileResult` — Interpret one complete notation view as exact controls and techniques.
- `fn interpret_all(requests: List(InterpretationRequest)) -> List(ProfileResult)` — Interpret a finite request list in source order.
- `let neutral: PerformanceProfile` — The edition-pinned neutral policy is ordinary data. Its defaults retain symbolic intent: staccato requests separation, accent requests emphasis, tenuto requests sustain, and fermata remains a named technique rather than a host duration multiplier.
- `record PerformanceVocabularyArtifact: Type` — Versioned finite root frozen by the generic checked-source boundary.
- `let performance_vocabulary: PerformanceVocabularyArtifact` — Edition-pinned standard controls and neutral profile for host/tooling consumers.

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
- `let interval_group: Group( Interval, )` — What `std::algebra` declares, said here about this carrier. Each is the same operations the `impl` blocks above already name, gathered into the record that says *which structure* they are — so a function that takes a group takes this one, and `P5.compose(M3)` and `interval_group.compose` are one definition rather than two spellings of one operation.  Here and not in `algebra.musa` because this file imports that one, and a module tree with no cycle in it has to say a structure's name in one place and its inhabitants in another. That is also the ordinary direction: `algebra` says what a group is, and this file says that the written intervals are one.  Nothing consumed these before they could be written. A `record` with a parameter was a declaration nothing could construct, so `Group(Interval)` was a claim about a value that did not exist.
- `let pitch_action: Action(Pitch, Interval)` — The action, at the carrier that keeps its octave. Not at `NoteName`, whose action is real and whose record is prompt 164's: the quotient has an action and no torsor, and saying so needs both here.
- `let pitch_torsor: Torsor(Pitch, Interval)` — And simply transitively at `Pitch`: exactly one written interval carries any pitch to any other, which is the lemma `Pitch.difference` computes.

## `std::post_tonal::pcset`

- `fn pc(n: Nat, cycle: Cycle(n), number: Nat) -> Pc(n)` — The pitch class a number names, reduced into the division.  `pc(12, chromatic, 13)` and `pc(12, chromatic, 1)` are one pitch class, because they are one residue.
- `fn pcs(n: Nat, cycle: Cycle(n), numbers: List(Nat)) -> List(Pc(n))` — The pitch classes a list of numbers names, each reduced.  A row or a set is written as its numbers, because that is what this domain has instead of letters.
- `fn class_number(n: Nat, member: Pc(n)) -> Nat` — The canonical representative: zero through `n - 1`.
- `fn class_moved(n: Nat, cycle: Cycle(n), member: Pc(n), by: Ti) -> Pc(n)` — One T/I operation, applied to one pitch class.
- `fn transposed_by(n: Nat, cycle: Cycle(n), index: Nat, member: Pc(n)) -> Pc(n)` — T_i: transposition by `index`, `x + i` in the division.  The index comes before the member because it is what names the operation: T_3 of a member, read in that order.
- `fn inverted_about(n: Nat, cycle: Cycle(n), index: Nat, member: Pc(n)) -> Pc(n)` — I_j: inversion about `index`, `j - x` in the division. I_0 is the plain mirror through zero.
- `fn class_action(n: Nat, cycle: Cycle(n)) -> Action(Pc(n), Ti)` — The T/I group acting on pitch classes, as the value `std::algebra` names.  The whole point of it being a value: `orbit` and `stabilizer` take it beside any other action, so a set class and a row's symmetries are the same two functions asked at different carriers.
- `fn map_pc(n: Nat, function: Pc(n) -> Pc(n), members: List(Pc(n))) -> List(Pc(n))` — Apply one pitch-class function to every member of a finite list.
- `fn forget_spelling(spelled: NoteName) -> Pc(12)` — Forget a spelling.  The only total map from the spelled domain into this one; it is not injective, and it has no inverse without a policy. At twelve, because a spelling is a common-practice object and the letters name that division.
- `fn spelled_in(member: Pc(12), collection: Scale) -> Option(NoteName)` — Spell a pitch class inside one collection — the explicit policy that `forget_spelling` has no inverse without. Absent when the collection holds no note of this pitch class.
- `fn inserted(value: Nat, sorted: List(Nat)) -> List(Nat)` — `sorted`, with `value` in its place — and unchanged where it is already there.
- `fn gathered(numbers: List(Nat)) -> List(Nat)` — The numbers, ascending and each once.
- `fn pcset(n: Nat, cycle: Cycle(n), members: List(Pc(n))) -> PcSet(n)` — The set of everything listed, however often it was listed.  A set cannot hold a duplicate, so this cannot fail: a repetition is a mistake only where order matters, which is `std::post_tonal::serial`.
- `fn numbers_of(n: Nat, set: PcSet(n)) -> List(Nat)` — The members' numbers, ascending. The set's own order and not its normal order.
- `fn set_members(n: Nat, cycle: Cycle(n), set: PcSet(n)) -> List(Pc(n))` — The members, ascending from zero.  Named for the set rather than `members_of`, because `std::harmony` already reads the members of a chord class and a piece that reasons about both must be able to import both.
- `fn set_size(n: Nat, set: PcSet(n)) -> Nat` — How many members the set has.
- `fn set_moved(n: Nat, cycle: Cycle(n), set: PcSet(n), by: Ti) -> PcSet(n)` — One T/I operation, applied to the whole set.  A set operation and not a map: the answer is a set, so the images are gathered again, and an operation that collapsed two members would give a smaller set rather than a list with a repeat in it.
- `fn set_transposed(n: Nat, cycle: Cycle(n), set: PcSet(n), index: Nat) -> PcSet(n)` — T_i applied to every member.
- `fn set_inverted(n: Nat, cycle: Cycle(n), set: PcSet(n), index: Nat) -> PcSet(n)` — I_j applied to every member.
- `fn same_numbers(left: List(Nat), right: List(Nat)) -> Bool` — Whether two ascending lists are the same list.
- `fn set_action(n: Nat, cycle: Cycle(n)) -> Action(PcSet(n), Ti)` — The T/I group acting on sets.
- `fn compactness(n: Nat, cycle: Cycle(n), rotation: List(Nat)) -> List(Nat)` — How tightly one rotation packs, as a list to compare.  The spans from the first member outward to the last, the second-to-last, and so on inward, and then the first member itself as the final tie break. Comparing two of these left to right is exactly the convention `101-pitch-class-sets-normal-order-and-transformations.md` states in prose.
- `fn reads_lower(left: List(Nat), right: List(Nat)) -> Bool` — Whether `left` reads lower than `right`, left to right.  Both lists are the same length wherever this is asked — two rotations of one set, or two zeroed normal orders of one set class — so running out is not a case that decides anything and answers false.
- `fn tighter(n: Nat, cycle: Cycle(n), best: List(Nat), candidate: List(Nat)) -> List( Nat, )` — Whichever of the two packs more tightly, the earlier winning a tie.
- `fn normal_numbers(n: Nat, cycle: Cycle(n), set: PcSet(n)) -> List(Nat)` — Normal order, as the numbers.
- `fn normal_order(n: Nat, cycle: Cycle(n), set: PcSet(n)) -> List(Pc(n))` — Normal order: the rotation of the ascending members packed most tightly to the left.  Ties break inward — first to last, then first to the one before last, and so on — and finally by the lowest starting pitch class.
- `fn zeroed(n: Nat, cycle: Cycle(n), set: PcSet(n)) -> List(Nat)` — The normal order transposed to begin on zero.
- `fn prime_form(n: Nat, cycle: Cycle(n), set: PcSet(n)) -> PcSet(n)` — Prime form: the set class this set belongs to.  The normal orders of the set and of its inversion are each transposed to begin on zero, and whichever reads lower is the answer. That is `102-set-class-and-prime-form.md`'s construction, and it agrees with the orbit reading below: a set class is the orbit of the set under the T/I group, and this is its canonical representative.
- `fn set_class(n: Nat, cycle: Cycle(n), set: PcSet(n)) -> List(PcSet(n))` — The set class itself: every set the T/I group reaches from this one.  `52-the-musical-algebra.md` §2.3's observation, executable — a set class *is* an orbit, and `prime_form` names it. The length of this times the length of `set_symmetries` is `2n`.
- `fn set_symmetries(n: Nat, cycle: Cycle(n), set: PcSet(n)) -> List(Ti)` — The T/I operations that send the set to itself.
- `fn transposition_symmetries(n: Nat, cycle: Cycle(n), set: PcSet(n)) -> List(Ti)` — The transpositions alone that send the set to itself.  A collection is one of Messiaen's modes of limited transposition exactly when more than one of these fixes it (`106-collections.md`): the whole-tone collection is fixed by every even transposition, the octatonic by every third. That is the chapter, and it is a stabilizer and nothing else.
- `fn pairs_of_class(n: Nat, cycle: Cycle(n), sought: Nat, members: List(Nat)) -> Nat` — How many pairs among `members` realize `sought` as their interval class.
- `fn against(n: Nat, cycle: Cycle(n), sought: Nat, member: Nat, others: List(Nat)) -> Nat` — How many members of `others` stand `sought` away from `member`.
- `fn interval_class_vector(n: Nat, cycle: Cycle(n), set: PcSet(n)) -> List(Nat)` — The interval-class vector: one count per interval class, ascending.  `n / 2` entries and not six, for the reason `103-interval-class-vectors.md` already gives — interval class `n - k` is interval class `k` heard the other way round — so the count is derived from the division rather than written down.

## `std::post_tonal::serial`

- `fn found_in(value: Nat, among: List(Nat)) -> Bool` — Whether `value` is somewhere in `among`.
- `fn repeats_from(position: Nat, seen: List(Nat), numbers: List(Nat)) -> List(Nat)` — The order positions of `numbers`, from `position` on, whose pitch class stood in `seen` already.
- `fn repeated_positions(numbers: List(Nat)) -> List(Nat)` — The order positions whose pitch class already appeared earlier.  The first occurrence is not among them, because that is where the pitch class belongs.
- `fn missing_classes(n: Nat, cycle: Cycle(n), numbers: List(Nat)) -> List(Nat)` — The pitch classes a sequence never names, ascending.  A sequence of the right length has one of these lists empty exactly when it has the other empty.
- `fn row(n: Nat, cycle: Cycle(n), numbers: List(Nat)) -> Result(ToneRow(n), RowFault)` — The row a sequence spells, or both exact reasons it is not one.  The numbers are reduced into the division first, so `12` and `0` are the same pitch class here as everywhere else and a sequence is not refused for spelling one of its classes the long way round.
- `fn row_numbers(n: Nat, series: ToneRow(n)) -> List(Nat)` — The row's pitch classes as numbers, in order-position order.
- `fn row_pcs(n: Nat, cycle: Cycle(n), series: ToneRow(n)) -> List(Pc(n))` — The row's pitch classes, in order-position order.
- `fn first_number(n: Nat, series: ToneRow(n)) -> Nat` — The pitch class a row begins on, as a number. Order position zero always exists, because a row has one position per pitch class and there is at least one.
- `fn row_moved(n: Nat, cycle: Cycle(n), series: ToneRow(n), by: RowOp) -> ToneRow(n)` — One labelled operation, applied to the row.  Total by construction: relabelling the pitch classes by a bijection and permuting the order positions each send a row to a row.
- `fn order_of(reversed: Bool, numbers: List(Nat)) -> List(Nat)` — The order positions, reversed or not.
- `fn row_transposed(n: Nat, cycle: Cycle(n), series: ToneRow(n), index: Nat) -> ToneRow(n)` — P: transposition by `index`, order positions untouched.
- `fn row_inverted(n: Nat, cycle: Cycle(n), series: ToneRow(n), index: Nat) -> ToneRow(n)` — I: inversion about `index`, order positions untouched.
- `fn row_retrograde(n: Nat, cycle: Cycle(n), series: ToneRow(n)) -> ToneRow(n)` — R: the order positions reversed, pitch classes untouched.  An involution, and it commutes with P and I because it acts on the other side of the row — which is exactly what `RowOp`'s two fields say.
- `fn row_retrograde_inversion(n: Nat, cycle: Cycle(n), series: ToneRow(n), index: Nat) -> ToneRow( n, )` — RI: the retrograde of the inversion, which is also the inversion of the retrograde. Writing it both ways and getting one row is what "commutes" means here.
- `fn same_order(left: List(Nat), right: List(Nat)) -> Bool` — Whether two sequences agree position by position.
- `fn row_action(n: Nat, cycle: Cycle(n)) -> Action(ToneRow(n), RowOp)` — The labelled group acting on rows.
- `fn row_operations(n: Nat, cycle: Cycle(n)) -> List(RowOp)` — Every labelled form there is: `4n` of them, each T/I read forward and backward. How many *rows* that is depends on the row, which is what `distinct_forms` counts.
- `fn row_forms(n: Nat, cycle: Cycle(n), series: ToneRow(n)) -> List(ToneRow(n))` — The distinct rows the labelled forms produce.  An orbit, and `52-the-musical-algebra.md` §2.3's point: this was a builtin that counted, and it is the length of a list the library can now build.
- `fn distinct_forms(n: Nat, cycle: Cycle(n), series: ToneRow(n)) -> Nat` — How many *distinct* rows the labelled forms produce. `4n` for a generic row; fewer for a row some labelled operation fixes.
- `fn row_symmetries(n: Nat, cycle: Cycle(n), series: ToneRow(n)) -> List(RowOp)` — The labelled operations that send the row to itself.  Its length times `distinct_forms` is `4n`, which is the orbit-stabilizer accounting the four labels are so often asked to do on their own.
- `fn starting_on(n: Nat, cycle: Cycle(n), series: ToneRow(n), start: Nat) -> ToneRow(n)` — The row transposed to begin on `start`.
- `fn matrix(n: Nat, cycle: Cycle(n), series: ToneRow(n)) -> List(ToneRow(n))` — The row matrix, as one row per order position.  Row zero is the row as written; row i is the transposition beginning on the ith pitch class of the inversion about the row's own head, so every column read downward is an inversion. The construction fixes no naming convention, because the rows are rows and not labels: which transposition is called P0 is the question the two functions below answer, differently and by name.
- `fn fixed_zero_index(n: Nat, series: ToneRow(n)) -> Nat` — The transposition index under the fixed-zero convention: P0 is the form beginning on pitch class zero, so a row's index is simply the number of the pitch class it begins on.
- `fn moveable_zero_index(n: Nat, cycle: Cycle(n), reference: ToneRow(n), form: ToneRow(n)) -> Nat` — The transposition index under the moveable-zero convention: P0 is the row as written, so an index is only meaningful relative to a stated reference row. `moveable_zero_index(reference, form)` is how far the form stands above the reference.
- `fn row_spelled_in(series: ToneRow(12), collection: Scale) -> List(Option(NoteName))` — Spell one row inside a collection, position by position.  A pitch class the collection cannot spell is absent, and the row keeps its length, so a projection that lost notes is visible as the gaps it left. At twelve, because a spelling is.

## `std::scale`

- `let tonic_degree: Degree` — The first degree of any scale. Degrees are written from one, as musicians write them.
- `fn key_scale(written: Key) -> Scale` — The scale a key's signature suggests for stepwise motion. It is a default, not a claim: `key c minor` fixes three flats, and a passage may still ask for the harmonic or melodic collection by name.
- `fn scale_root(collection: Scale) -> NoteName` — The tonic pitch class a scale is rooted on.
- `fn scale_degrees(collection: Scale) -> Nat` — How many degrees one period of a scale holds.
- `fn degree_in(collection: Scale, written: Pitch) -> Option(Degree)` — The degree a written pitch occupies, when it occupies one. Membership is spelled: `eb5` and `d#5` answer differently.
- `fn belongs_to(collection: Scale, written: Pitch) -> Bool` — Whether a written pitch belongs to a scale at all.
- `fn degree_is_present(located: Degree) -> Bool` — The present case of `belongs_to`: a located degree means the pitch is a member, whichever degree it turned out to be.
- `fn frame_on(collection: Scale, root: Pitch) -> Option(Frame)` — The register frame a scale takes on one absolute tonic pitch. It is absent when that pitch is not the scale's tonic class.
- `fn frame_degree(register: Frame, ordinal: Nat) -> Pitch` — The written pitch a numbered degree names in one register frame.
- `fn frame_triad(register: Frame) -> List(Pitch)` — The tonic, third, and fifth degrees of a frame, in register.
- `fn degree_class(collection: Scale, ordinal: Nat) -> Option(NoteName)` — The pitch class a numbered degree names, with no register at all. `frame_degree` asks the same thing of a scale that has been given an absolute tonic, and it has to be given one, because a written pitch has an octave and something must choose it. A Roman numeral has no octave to choose — `V` in C major is the class `g`, and which `g` sounds is the voicing's business. Absent only for an ordinal no score can write.
- `fn altered_class(collection: Scale, altered: Degree) -> Option(NoteName)` — The same for a degree that already carries an alteration, so that a `raise` or `lower` composes into the spelling rather than being lost. This is how a borrowed or Neapolitan degree is spelled without a frame.
- `fn degree_chord(collection: Scale, written: Degree, members: Nat) -> Option(ChordClass)` — The chord the collection stacks in thirds from a degree. `members` counts the notes, so three is a triad and four a seventh chord. The quality is the collection's and not the caller's: `ii` is minor in major and `II` is major in Dorian because those are the notes there, which is the whole content of the word "diatonic". A degree rather than a number because the degree is where this language does ordinal arithmetic, so a succession walked by `up_steps` can be harmonized where a walked number could not. Absent for an altered degree, which is not asking for the collection's own chord, and absent when the collection stacks to a sonority the chord vocabulary cannot name.
- `fn degree_triad(collection: Scale, written: Degree) -> Option(ChordClass)` — The diatonic triad on a degree.
- `fn degree_seventh(collection: Scale, written: Degree) -> Option(ChordClass)` — The diatonic seventh chord on a degree.
- `fn up_steps(from: Degree, steps: Nat) -> Degree` — Move a degree up by a whole number of scale steps.
- `fn down_steps(from: Degree, steps: Nat) -> Degree` — Move a degree down by a whole number of scale steps.
- `fn raise(from: Degree) -> Degree` — Raise a degree chromatically without moving its coordinate.
- `fn lower(from: Degree) -> Degree` — Lower a degree chromatically without moving its coordinate.

## `std::sound::catalogue`

- `record PortContract: Type` — One valid first-order input and result shape for a processor.
- `record PrimitiveRequirement: Type` — Stable host capability required by a future executable source wrapper. This names no state layout or work figure: those remain private host facts.
- `record ProcessorContract: Type` — One complete source-facing standard processor declaration.
- `record StudioTermContract: Type` — Documentation and written shape for one standard studio term.
- `record StudioVocabularyArtifact: Type` — The edition-pinned finite studio vocabulary crossing the checked boundary.
- `fn frequency_value(value: Ratio) -> SoundQuantity(Frequency)` — Construct an exact frequency quantity for a catalogue declaration.
- `fn linear_value(value: Ratio) -> SoundQuantity(LinearAmplitude)` — Construct an exact linear-amplitude quantity for a catalogue declaration.
- `fn level_value(value: Ratio) -> SoundQuantity(Level)` — Construct an exact level quantity for a catalogue declaration.
- `fn time_value(value: Ratio) -> SoundQuantity(Time)` — Construct an exact time quantity for a catalogue declaration.
- `fn frequency_parameter(name: Text, summary: Text, default: Ratio, minimum: Ratio, maximum: Ratio) -> SomeParameterContract` — Package a frequency parameter while retaining its checked index.
- `fn linear_parameter(name: Text, summary: Text, default: Ratio, minimum: Ratio, maximum: Ratio) -> SomeParameterContract` — Package a linear-amplitude parameter while retaining its checked index.
- `fn level_parameter(name: Text, summary: Text, default: Ratio, minimum: Ratio, maximum: Ratio) -> SomeParameterContract` — Package a level parameter while retaining its checked index.
- `fn time_parameter(name: Text, summary: Text, default: Ratio, minimum: Ratio, maximum: Ratio) -> SomeParameterContract` — Package a time parameter while retaining its checked index.
- `fn primitive(id: Text) -> PrimitiveRequirement` — Name one version-one host capability without exposing its implementation.
- `let audio_processor_ports: List(PortContract)` — The public shape shared by unary audio processors.
- `let control_processor_ports: List(PortContract)` — The public shape shared by unary control processors.
- `let oscillator_ports: List(PortContract)` — The two context-selected public shapes of an oscillator.
- `let studio_vocabulary: StudioVocabularyArtifact` — The complete source-owned studio vocabulary for standard-library edition 1.

## `std::sound::graph`

- `record PortPath: Type` — One node and one port on it, kept as two names rather than reparsed text.
- `record PortInfo` — One resolved descriptor port, private to graph validation.
- `record Parameter: Type` — One written setting, including the anchor of its value.
- `record StudioDescription: Type` — A complete finite description, still independent of any running processor.
- `record CheckedStudio: Type` — A description whose names, settings, ports, bindings, and cycles were checked.
- `record Named` — One declaration name and its anchor, private to duplicate checking.
- `fn make_description(declarations: List(StudioDecl)) -> StudioDescription` — Build a description from declarations already in written order.
- `fn description_of(checked: CheckedStudio) -> StudioDescription` — Recover the finite written description from a checked one.
- `fn between_zero_and_one(value: Ratio) -> Bool` — Whether an exact value fits an inclusive normalized parameter range.
- `fn at_most_ten_seconds(value: Ratio) -> Bool` — Whether a time is nonnegative and within the trial descriptors' bound.
- `fn linear_magnitude(quantity: ExactQuantity) -> Option(Ratio)` — Read a normalized value only when its source index proves the requested dimension. There is no host-side unit coercion or fallback.
- `fn seconds_magnitude(quantity: ExactQuantity) -> Option(Ratio)` — Read seconds only when the source index and unit both prove time.
- `fn count_in_voice_range(value: Nat) -> Bool` — Whether a polyphonic voice count fits the finite descriptor range.
- `fn descriptor_known(descriptor: Text) -> Bool` — Whether the trial package declares this processor descriptor.
- `fn parameter_checked(descriptor: Text, parameter: Parameter) -> Option(StudioError)` — Check one parameter against its descriptor's name, unit, and range.
- `fn parameters_checked(descriptor: Text, parameters: List(Parameter)) -> Option( StudioError, )` — Return the first invalid parameter in written order, if there is one.
- `fn descriptor_port(descriptor: Text, port: Text) -> Option(PortInfo)` — Resolve one port declared by a known processor descriptor.
- `fn port_of(description: StudioDescription, path: PortPath) -> Option(PortInfo)` — Resolve an external or processor port from a complete description.
- `fn direction_is_output(direction: PortDirection) -> Bool` — Whether a resolved port may stand at the source of a connection.
- `fn direction_is_input(direction: PortDirection) -> Bool` — Whether a resolved port may stand at the target of a connection.
- `fn connection_checked(description: StudioDescription, anchor: Nat, source_path: PortPath, target_path: PortPath) -> Option(StudioError)` — Check both ends, their direction, and their exact port-kind agreement.
- `fn named(declaration: StudioDecl) -> Option(Named)` — Project the namespace-bearing declarations and ignore edges and bindings.
- `fn duplicate_checked(description: StudioDescription, declaration: StudioDecl) -> Option( StudioError, )` — Find a second declaration carrying the same graph-level name.
- `fn node_exists(description: StudioDescription, sought: Text) -> Bool` — Whether a binding target names a declared processor node.
- `fn connection_path(remaining: Nat, declarations: List(StudioDecl), current: Text, target: Text) -> Option(List(Nat))` — Find one path from `current` to `target`. The first argument decreases on every recursive call, so malformed cyclic input cannot make validation run forever.
- `fn cycle_checked(description: StudioDescription, declaration: StudioDecl) -> Option( StudioError, )` — Return every connection anchor in the first cycle closed by this edge.
- `fn declaration_checked(description: StudioDescription, declaration: StudioDecl) -> Option(StudioError)` — Check one declaration against the complete graph around it.
- `fn validate(description: StudioDescription) -> Result(CheckedStudio, StudioError)` — Validate every declaration and return the first complaint in written order.

## `std::sound::instrument`

- `record TechniqueSupport: Type` — One namespaced technique accepted by an instrument signature.
- `fn accepts_control({kind: ControlKind}, control_key: ControlKey(kind), default_value: ControlValue(kind)) -> SomeControlRequirement` — Package one requirement after the general pattern unifier settles its kind.
- `record InstrumentSignature: Type` — The complete public behavioral contract of one source instrument.
- `record ParameterTarget` — A target is private graph structure, never a public control address.
- `record ConnectionTransfer` — Exact private parameter values selected by each phrase relation.
- `fn maps_connection(control_key: ControlKey(PhraseConnection), node: Text, parameter: Text, transfer: ConnectionTransfer) -> SomeControlMapping` — Bind phrase connection to a private parameter through an exact source table.
- `fn maps_exact_ratio(control_key: ControlKey(ExactRatio), node: Text, parameter: Text) -> SomeControlMapping` — Bind one concrete dimensionless ratio directly to a private parameter.
- `let basic_sine_partial_ratio: ControlKey(ExactRatio)` — A physical oscillator-bank control deliberately specific to this instrument.
- `fn maps_normalized(control_key: ControlKey(Normalized), node: Text, parameter: Text, transfer: NormalizedTransfer) -> SomeControlMapping` — Bind one normalized musical control to a private primitive parameter.
- `record NativeInstrumentBody` — The implementation type and every binding of it are private. The public `Instrument` below carries only a stable declaration identity and signature.
- `record InstrumentImplementationContract` — Canonical private intent crosses the checked boundary; the machine itself crosses through the distinct machine projection. Including the mappings here makes any implementation-policy change part of exact preparation identity.
- `fn implementation_contract(body: NativeInstrumentBody) -> InstrumentImplementationContract` — Retain the exact private mapping policy beside its stable declaration.
- `record Instrument: Type` — A source instrument value exposes its contract, never graph-local paths.
- `fn scale_frame(factor: Ratio) -> Machine(AudioFrameStep, Ratio, Ratio)` — Registered primitive wrappers are ordinary functions. The primitive call is the only host-owned leaf; composition and configuration remain source.
- `fn mix_frames(left: Ratio, right: Ratio) -> Machine( AudioFrameStep, (Ratio, Ratio), Ratio, )` — Mix two exact reference channels with explicit gains.
- `let basic_sine_machine: Machine(AudioFrameStep, Ratio, Ratio)` — A machine remains its own checked projection rather than being smuggled through canonical record data. Keeping the binding at module scope lets the compiler recognize its Machine type without exposing it to importers.
- `let basic_sine_body: NativeInstrumentBody` — The edition-one basic instrument's private machine and control mappings.
- `let note_instrument: InstrumentSignature` — Edition-one note instrument contract shared by the basic native preset.
- `let basic_sine: Instrument` — Stable edition-one zero-setup instrument declaration.
- `let basic_sine_demo_profile: PerformanceProfile` — A source-only profile demonstrating standard and instrument-specific controls.
- `record InstrumentExecutionArtifact: Type` — This root is the only route from a private body to host preparation. It is produced and checked as one source value; no Rust instrument schema can independently construct or amend it.
- `let standard_instruments: InstrumentExecutionArtifact` — Versioned checked standard instrument declarations and private machines.

## `std::sound::media`

- `record MediaPlayback: Type` — Exact playback settings shared by clips and fixed-media cues.
- `let neutral_media_playback: MediaPlayback` — The neutral playback policy used when a cue writes no settings.

## `std::sound::production`

- `record StudioNode: Type` — One resolved processor node in a patch, bus, or control signal.
- `record StudioGraph: Type` — One named finite processor graph and its designated output node.
- `record StudioAssignment: Type` — One part-to-instrument assignment.
- `record StudioSend: Type` — One exact-decibel send.
- `record StudioRoute: Type` — One explicit route to a bus or the master output.
- `record StudioModulation: Type` — One control-signal binding to a resolved private stage parameter.
- `record StudioExecutionArtifact: Type` — The complete exact production studio crossing the checked-source boundary.
- `fn frequency(value: Ratio) -> SoundQuantity(Frequency)` — Lift an exact ratio into the frequency dimension.
- `fn level(value: Ratio) -> SoundQuantity(Level)` — Lift an exact ratio into the level dimension.
- `fn time(value: Ratio) -> SoundQuantity(Time)` — Lift an exact ratio into the time dimension.
- `fn frequency_or(value: Option(SoundQuantity(Frequency)), fallback: SoundQuantity(Frequency)) -> SoundQuantity(Frequency)` — Select a supplied frequency or its source-owned fallback.
- `fn linear_or(value: Option(SoundQuantity(LinearAmplitude)), fallback: SoundQuantity(LinearAmplitude)) -> SoundQuantity(LinearAmplitude)` — Select a supplied linear amplitude or its source-owned fallback.
- `fn level_or(value: Option(SoundQuantity(Level)), fallback: SoundQuantity(Level)) -> SoundQuantity(Level)` — Select a supplied level or its source-owned fallback.
- `fn time_or(value: Option(SoundQuantity(Time)), fallback: SoundQuantity(Time)) -> SoundQuantity(Time)` — Select a supplied time or its source-owned fallback.
- `fn studio_oscillator(frequency_value: Option(SoundQuantity(Frequency)), ratio: Option(SoundQuantity(LinearAmplitude))) -> StudioProcessor` — Construct an oscillator, applying edition-one source defaults.
- `fn studio_gain(gain: Option(SoundQuantity(Level))) -> StudioProcessor` — Construct a gain stage, applying its source default.
- `fn studio_mix() -> StudioProcessor` — Construct the signal mixer.
- `fn studio_envelope(attack: Option(SoundQuantity(Time)), decay: Option(SoundQuantity(Time)), sustain: Option(SoundQuantity(LinearAmplitude)), release: Option(SoundQuantity(Time))) -> StudioProcessor` — Construct a voice envelope, applying edition-one source defaults.
- `fn default_resonance() -> SoundQuantity(LinearAmplitude)` — The exact Butterworth resonance used by both filter constructors.
- `fn studio_lowpass(cutoff: Option(SoundQuantity(Frequency)), resonance: Option(SoundQuantity(LinearAmplitude))) -> StudioProcessor` — Construct a low-pass stage, applying source defaults.
- `fn studio_highpass(cutoff: Option(SoundQuantity(Frequency)), resonance: Option(SoundQuantity(LinearAmplitude))) -> StudioProcessor` — Construct a high-pass stage, applying source defaults.
- `fn studio_reverb(room: Option(SoundQuantity(LinearAmplitude)), damping: Option(SoundQuantity(LinearAmplitude)), mix: Option(SoundQuantity(LinearAmplitude))) -> StudioProcessor` — Construct a reverb stage, applying source defaults.
- `fn studio_delay(delay_time: Option(SoundQuantity(Time)), feedback: Option(SoundQuantity(LinearAmplitude)), mix: Option(SoundQuantity(LinearAmplitude))) -> StudioProcessor` — Construct a delay stage, applying source defaults.
- `fn studio_chorus(rate: Option(SoundQuantity(Frequency)), depth: Option(SoundQuantity(Time)), mix: Option(SoundQuantity(LinearAmplitude))) -> StudioProcessor` — Construct a chorus stage, applying source defaults.
- `fn studio_scale(factor: Option(SoundQuantity(Frequency))) -> StudioProcessor` — Construct a control scale stage, applying its source default.
- `fn studio_bias(offset: Option(SoundQuantity(Frequency))) -> StudioProcessor` — Construct a control bias stage, applying its source default.
- `fn studio_clamp(minimum: Option(SoundQuantity(Frequency)), maximum: Option(SoundQuantity(Frequency))) -> StudioProcessor` — Construct a control clamp stage, applying source defaults.
- `fn studio_smoothing(smoothing_time: Option(SoundQuantity(Time))) -> StudioProcessor` — Construct a smoothing stage, applying its source default.
- `let empty_studio: StudioExecutionArtifact` — The source-owned zero-setup studio value.

## `std::sound::quantity`

- `record ExactQuantityArtifact: Type` — The versioned checked-artifact root consumed at the DSP boundary.
- `fn hertz(magnitude: Ratio) -> ExactQuantity` — An exact frequency in hertz.
- `fn linear(magnitude: Ratio) -> ExactQuantity` — An exact dimensionless linear amplitude or factor.
- `fn decibels(magnitude: Ratio) -> ExactQuantity` — An exact logarithmic amplitude level in decibels.
- `fn seconds(magnitude: Ratio) -> ExactQuantity` — An exact physical duration in seconds.
- `fn milliseconds(magnitude: Ratio) -> ExactQuantity` — Milliseconds normalize exactly into the source base unit. The CST retains the written `ms` token for diagnostics and token-scoped edits.
- `fn quantity_artifact(quantity: ExactQuantity) -> ExactQuantityArtifact` — Wrap a quantity in the versioned value consumed by a host boundary.

## `std::sound::sample`

- `record SampleModulation: Type` — One exact typed note-on modulation; target units determine `amount` units.
- `record SampleFilter: Type` — The SoundFont-compatible resonant low-pass at its unmodulated setting.
- `record SampleEnvelope: Type` — Exact source envelope values and linear amplitude.
- `record SampleRegion: Type` — One immutable audio region and every predicate or playback rule it owns.
- `record SampleMap: Type` — A complete source-declared sample instrument implementation.
- `record SampleInstrument: Type` — One ordinary source instrument contract paired with its sample-map body. The body is private implementation policy to consumers of the instrument; adapters construct this same value before native normalization.
- `fn sample_instrument(contract: Instrument, implementation: SampleMap) -> SampleInstrument` — Pair a public instrument contract with a source-declared sample map.
- `record SampleMapArtifact: Type` — The versioned checked boundary consumed by native preparation and adapters.

## `std::tonal::harmony`

- `fn numeral(ordinal: Nat, members: Nat, position: Nat) -> Option(Roman)` — The numeral three numbers describe, when they describe one. Absent when the ordinal is outside `I`–`vii`, when the stack is smaller than a triad or larger than a thirteenth, or when the bass position names a member the stack does not have — a third inversion of a triad is not a numeral that is hard to realize, it is not a numeral. OMT 020 and 021.
- `fn triad_numeral(ordinal: Nat) -> Option(Roman)` — The root-position triad on a degree, which is what OMT 020 writes with a bare numeral and no figures.
- `fn seventh_numeral(ordinal: Nat) -> Option(Roman)` — The root-position seventh chord on a degree: OMT 021's `7`.
- `fn numeral_step(written: Roman) -> Nat` — Which degree the numeral is built on, counted from one.
- `fn numeral_size(written: Roman) -> Nat` — How many members the numeral stacks: three is a triad, four a seventh.
- `fn numeral_bass(written: Roman) -> Nat` — Which member the numeral puts in the bass, counted from zero, so zero is root position and one is the `6` of figured bass. A position in the stack and not a pitch: which note actually sounds lowest is a voicing's business, and `std::voicing` is where that is decided.
- `fn numeral_chord(collection: Scale, written: Roman) -> Option(ChordClass)` — The chord a numeral names in a collection, spelled by that collection. The quality is never supplied: `ii` is minor in major and `II` is major in Dorian because those are the notes there, which is the whole content of the word "diatonic". Absent when the collection has no such degree, or when it stacks to a sonority the chord vocabulary cannot name — a harmonic-minor `III7` is an augmented major seventh, and reporting the absence is more honest than rounding it to a chord with other notes in it. OMT 020 and 021.
- `fn borrowed(home: Scale, mode: Scale, written: Roman) -> Option(ChordClass)` — Modal mixture: the numeral realized against a borrowed collection on the home tonic. Borrowing needs no altered degrees and no alteration field, because that is what the word means — the flat six of C major is the sixth of C minor, and spelling it as a lowered degree describes the result rather than the operation. Which collection is borrowed from is the caller's, since parallel minor, harmonic minor, and Phrygian all lend chords and none of them is the default. OMT 061.
- `fn secondary(home: Scale, target: Degree, mode: Scale, written: Roman) -> Option(ChordClass)` — An applied chord: a numeral read in the collection that tonicizes a target degree of the home collection. `V/V` in C major is `secondary(home, degree_of(5), major_on(c), five)` — the numeral is read in G, which is why it is D major and not the D minor that C major stacks. The target degree is written out, so a tonicized lowered sixth is as sayable as a tonicized fifth, and the tonicizing collection is written out, so nothing here decides that an applied chord implies major. Tonicization is a local relationship between two chords; whether a passage has modulated is a claim about the passage and is not decided here. OMT 050.
- `fn quality_on_degree(collection: Scale, written: Degree, quality: ChordClass) -> Option( ChordClass, )` — A named chord quality rooted on a named degree of a collection. This is how every chromatic sonority below is built: the degree carries the alteration, the quality carries the members, and neither is guessed. A Neapolitan in C major names a lowered second and in C minor names the plain second, and that difference is the caller's to write, because the minor collection was the caller's to choose. Absent when the collection has no such degree.
- `fn neapolitan(collection: Scale, lowered_second: Degree) -> Option(ChordClass)` — The Neapolitan: a major triad on the lowered second degree. The `6` in its usual name is a first inversion, which is a position and is taken with `inversion` rather than baked in here — the chord is a major triad in any position, and OMT 062 says so before it says the sixth is idiomatic.
- `fn italian_sixth(collection: Scale, lowered_sixth: Degree) -> Option(ChordClass)` — The Italian sixth: a lowered sixth, the tonic, and a raised fourth, with no fifth. Its augmented sixth is spelled as an augmented sixth, which is the whole reason it is a chord of its own and not a seventh. OMT 063.
- `fn french_sixth(collection: Scale, lowered_sixth: Degree) -> Option(ChordClass)` — The French sixth: the Italian sixth with the second degree added, which spells as an augmented fourth above the root. OMT 063.
- `fn german_sixth(collection: Scale, lowered_sixth: Degree) -> Option(ChordClass)` — The German sixth: the Italian sixth with the lowered third added, which spells as a perfect fifth above the root. It sounds like a dominant seventh and is not one — the top note is an augmented sixth, written from a different letter, and re-rooting `dominant7` here would spell the wrong note. OMT 063.
- `fn altered_dominant(collection: Scale, quality: ChordClass) -> Option(ChordClass)` — An altered or extended dominant: a named quality on the fifth degree. Which alteration is present is the caller's, written as the chord type — there is no universal set of alterations, and a function that picked one would be asserting a style rather than constructing a chord. `chord c dom7b9`, `chord c dom7s5`, and the plain extensions all pass here. OMT 071.

## `std::tonal::schemas`

- `fn degrees_of(ordinals: List(Nat)) -> List(Degree)` — The degrees of a written line, from the ordinals OMT prints. Written out rather than folded because a schema *is* its table: a reader should see the same numbers here that they see in OMT's row.
- `fn as_degree(ordinal: Nat) -> Degree` — One ordinal as a degree. Written out because `degree_of` is a compiler-owned operation and only a source function can be passed to a fold.
- `fn romanesca_bass() -> List(Degree)` — Romanesca, OMT 34 §Romanesca. Four stages, strong-weak-strong-weak, figures 5-6-5-6. The bass falls *do-ti-la-mi*: three steps down and then a leap, which is what distinguishes it from the stepwise openings.  Direction: descending, then a leap down to the third.
- `fn romanesca_melody() -> List(Degree)` — The Romanesca melody, *do-sol-do-do*.
- `fn romanesca_roots() -> List(Degree)` — The roots the Romanesca's figures name: I-V-vi-I. The 6 on stage two means the chord over *ti* is rooted on *sol*, and the 6 on stage four means the chord over *mi* is rooted on *do*. The figure is read here so that a caller reads roots, and the bass is kept separately so that the caller can still voice from it.
- `fn romanesca_strong() -> List(Bool)` — The Romanesca's metrical shape. True is OMT's S.
- `fn do_re_mi_bass() -> List(Degree)` — Do-Re-Mi, OMT 34 §Do–Re–Mi. Three stages, the shortest opening in the summary table, and the one whose two halves OMT itself calls schemas: the Do-Re question and the Re-Mi answer. Figures 5-6-5, numerals I-V-I.
- `fn do_re_mi_melody() -> List(Degree)` — The Do-Re-Mi melody, which is the stepwise ascent the schema is named for.
- `fn do_re_mi_roots() -> List(Degree)` — The Do-Re-Mi roots: I-V-I, the 6 on stage two rooting the chord over *ti* on *sol*.
- `fn do_re_mi_strong() -> List(Bool)` — The Do-Re-Mi's metrical shape, strong-weak-strong: the shortest of the openings leans on its outer stages.
- `fn prinner_bass() -> List(Degree)` — Prinner, OMT 34 §Prinner. Four stages with a stepwise falling bass *fa-mi-re-do* under a falling *la-sol-fa-mi*: parallel tenths, which is why it answers an opening so readily. Figures 5-6-7-6-5, numerals IV-I-vii-I.
- `fn prinner_melody() -> List(Degree)` — The Prinner melody, *la-sol-fa-mi*. It falls a tenth above the falling bass at every stage, which is the parallel motion the schema is heard by.
- `fn prinner_roots() -> List(Degree)` — The Prinner's roots. Stage two's 6 roots the chord over *mi* on *do*, and stage four's 6 does the same over *do*, which is why the last two roots are not the bass.
- `fn prinner_strong() -> List(Bool)` — The Prinner's metrical shape, strong-weak-strong-weak.
- `fn prinner_with_dominant_bass() -> List(Degree)` — The five-stage Prinner OMT 34 gives as "a slight variant on this": a root-position dominant is inserted before the final stage. A separate function rather than a flag, because a variant with a different number of stages is a different table and a caller reading four lists in parallel should not have one of them silently change length.
- `fn prinner_with_dominant_melody() -> List(Degree)` — The five-stage Prinner's melody: the four-stage line with *fa* held across the inserted dominant rather than falling through it.
- `fn prinner_with_dominant_roots() -> List(Degree)` — The five-stage Prinner's roots. The four-stage roots with the inserted stage rooted on the fifth degree, which is what makes it a dominant.
- `fn prinner_with_dominant_strong() -> List(Bool)` — The five-stage Prinner's metrical shape. One stage longer than the four-stage Prinner's, and strong at the end rather than weak.
- `fn fonte_bass() -> List(Degree)` — Fonte, OMT 34 §Fonte. Two tonicizations a step apart, and the first of them needs a bass note the collection does not contain: OMT writes it *di*, a raised *do*. This is the schema that proves an alteration has to be carried explicitly — `raise` moves the spelling without moving the coordinate, so the degree is still the first and is still spelled sharp.
- `fn fonte_melody() -> List(Degree)` — The Fonte melody, *sol-fa-fa-mi*: the fourth degree is held across the step down, so only the bass and the harmony move between the two tonicizations.
- `fn fonte_roots() -> List(Degree)` — The Fonte's roots: V/ii-ii-V-I. The applied dominant is rooted on the raised first degree because that is the note OMT's figure sits over, and the raising travels with the root.
- `fn fonte_strong() -> List(Bool)` — The Fonte's metrical shape, weak-strong-weak-strong: each of the two tonicizations leans on its second stage.
- `fn monte_bass() -> List(Degree)` — Monte, OMT 34 §Monte. The Fonte's counterpart, rising rather than falling, and altered in the other direction: OMT writes *fi*, a raised *fa*, tonicizing the dominant. Figures 6/5-5-6/5-5, numerals V/IV-V-V/V-V.
- `fn monte_melody() -> List(Degree)` — The Monte melody, *ti-la-do-ti*, rising to the octave over the second tonicization where the Fonte's line fell.
- `fn monte_roots() -> List(Degree)` — The Monte's roots, the numerals above read as degrees. The third stage is rooted on a raised degree because its chord is an applied dominant, and the raising travels with the root as it does in the Fonte.
- `fn monte_strong() -> List(Bool)` — The Monte's metrical shape, weak-strong-weak-strong: the same lean as the Fonte's, which is what makes the two a rising and a falling pair.
- `fn fenaroli_bass() -> List(Degree)` — Fenaroli, OMT 34 §Fenaroli. The one representative here whose bass *rises* stepwise, *ti-do-re-mi*, which is why it is in this file beside the falling ones: a schema is not its intervals reversed.
- `fn fenaroli_melody() -> List(Degree)` — The Fenaroli melody, *fa-mi-ti-do*: a falling step, and then the leading tone resolving upward to the tonic.
- `fn fenaroli_roots() -> List(Degree)` — The Fenaroli's roots: V-I-V-I, with the 6 on the last stage rooting the chord over *mi* on *do*.
- `fn fenaroli_strong() -> List(Bool)` — The Fenaroli's metrical shape, strong-weak-strong-weak.
- `fn cadenza_semplice_bass() -> List(Degree)` — Cadenza semplice, OMT 34 §Cadenza Semplice. The plain cadence: bass *mi-fa-sol-do*, figures 6-6/5-5-5, numerals I-ii-V-I.
- `fn cadenza_semplice_melody() -> List(Degree)` — The plain cadence's melody, *do-re-re-do*: the second degree held across the pre-dominant and the dominant, and falling to the tonic at the end.
- `fn cadenza_semplice_roots() -> List(Degree)` — The plain cadence's roots, I-ii-V-I. The 6 on stage one roots the chord over *mi* on *do*, and the 6/5 on stage two roots the chord over *fa* on *re*, so neither of the first two roots is its bass.
- `fn cadenza_semplice_strong() -> List(Bool)` — The plain cadence's metrical shape, weak-strong-weak-strong, so the cadence itself lands on a strong stage.
- `fn quiescenza_bass() -> List(Degree)` — Quiescenza, OMT 34 §Quiescenza. Post-cadential, and the one schema here whose bass does not move at all: four stages on *do*. Its melody needs the other alteration — OMT writes *te*, a lowered *ti* — which makes the first chord a dominant of the subdominant over a tonic pedal.
- `fn quiescenza_melody() -> List(Degree)` — The Quiescenza melody, *te-la-ti-do*. The lowered seventh is what makes the first chord a dominant of the subdominant; the rest is the ordinary ascent to the tonic over the held bass.
- `fn quiescenza_roots() -> List(Degree)` — The Quiescenza's roots, I-V-V-I, sounding above the tonic its bass holds through all four stages.
- `fn quiescenza_strong() -> List(Bool)` — The Quiescenza's metrical shape, weak-strong-weak-strong.
- `fn schema_triads(collection: Scale, roots: List(Degree)) -> List(Option(ChordClass))` — The chord classes a schema's roots name in one collection, as triads. Absent where the collection does not stack to a nameable sonority, and absent on an altered root, because an applied dominant is not the collection's own chord and this function only knows the collection's. That absence is the honest answer: `fonte_roots` and `monte_roots` carry raised degrees on purpose, and a caller who wants those chords supplies them, from `std::tonal::harmony`, where applied chords live.
- `fn schema_triad(collection: Scale, root: Degree) -> Option(ChordClass)` — One root, harmonized as the collection's triad.
- `fn schema_sevenths(collection: Scale, roots: List(Degree)) -> List(Option(ChordClass))` — The same as seventh chords, for the figures OMT prints with a 7 or a 6/5 in them.
- `fn schema_seventh(collection: Scale, root: Degree) -> Option(ChordClass)` — One root, harmonized as the collection's seventh chord.
- `fn sixth_over(collection: Scale, bass: Degree) -> Option(ChordClass)` — Step 1. A 6/3 over a bass degree is the triad a third below it, in first inversion. This is the whole of the parallel-sixths harmonization, and it is the same function `std::tonal::sequences` uses for the parallel 6/3 passage, seen from the bass rather than from the root.
- `fn fifth_over(collection: Scale, bass: Degree) -> Option(ChordClass)` — Step 2 and step 3 share a shape: a 5/3 is the triad rooted on the bass itself, root position, no designation.
- `fn six_five_over(collection: Scale, bass: Degree) -> Option(ChordClass)` — Step 4. A seventh chord over a bass degree, rooted a third below it so that the bass is still the third — OMT's 6/5 — which is the figure the recipe's seventh chords carry.
- `fn inverted_first(content: Option(ChordClass)) -> Option(ChordClass)` — Designate the third as bass, keeping absence absent.
- `fn rule_ascending_chord(collection: Scale, bass: Nat) -> Option(ChordClass)` — The Rule ascending, one bass degree at a time. Ordinals are written from one and read modulo the collection's period, so eight is the octave and is the tonic again.  Where each answer comes from: 1 and 5 are the recipe's 5/3s; 8 is the closing 5/3, which is the same chord as 1 and is written separately because OMT's "first and last" is two places. 4 precedes the dominant and 7 precedes the octave, so both take the recipe's sevenths. The rest are the parallel 6/3s the recipe starts from.  The chromatic alteration OMT mentions once — "In one case, this also involves a chromatic alteration for a stronger sense of tonicizing the dominant" — is *not* applied here, and its absence is the point: OMT names it without printing which chord takes it, and a library that guessed would be asserting a version rather than following one. A caller who wants it writes it, and `raise` is how.
- `fn rule_descending_chord(collection: Scale, bass: Nat) -> Option(ChordClass)` — The Rule descending, one bass degree at a time. The 5/3s stand where they stood — a tonic is a tonic whichever way the bass is walking — and the sevenths move, because 6 is what precedes the dominant coming down and 2 is what precedes the tonic.
- `fn rule_ascending_basses() -> List(Nat)` — The bass scale the Rule is harmonized over, ascending. Eight ordinals and not seven: OMT prints the octave, so the last chord is a tonic again, and the closing 5/3 has somewhere to stand. Written as a literal because the Rule of the Octave is over the octave — there is no count here to vary, which is exactly what distinguishes it from the sequences in `std::tonal::sequences`, where the count is the whole point.
- `fn rule_descending_basses() -> List(Nat)` — The same descending. Written out rather than reversed, because a descent is its own line and OMT gives it as one.
- `fn rule_of_the_octave_ascending(collection: Scale) -> List(Option(ChordClass))` — The whole ascending Rule: eight chords over the ascending bass scale.
- `fn rule_of_the_octave_descending(collection: Scale) -> List(Option(ChordClass))` — The whole descending Rule.

## `std::tonal::sequences`

- `fn rising_degree(start: Degree, steps: Nat, index: Nat) -> Degree` — The degree reached after `index` applications of a rise of `steps` scale steps. Index zero is the start, which is what makes a count of one mean "the pattern, stated once".
- `fn risen_by(steps: Nat, index: Nat, from: Degree) -> Degree` — The rise itself, as the fold step it is applied by. The index is ignored on purpose: a diatonic sequence moves by the same interval every time, and a pattern that did not would be a different pattern.
- `fn falling_degree(start: Degree, steps: Nat, index: Nat) -> Degree` — The degree reached after `index` applications of a fall of `steps` scale steps. Falling is its own function rather than a negative rise, because a `nat` has no sign and a direction that could be forgotten is a direction that will be.
- `fn fallen_by(steps: Nat, index: Nat, from: Degree) -> Degree` — The fall, as the fold step it is applied by.
- `fn rising_degrees(start: Degree, steps: Nat, count: Nat) -> List(Degree)` — The whole finite walk upward: `count` degrees, beginning at `start`. A count of zero is the empty walk and a count of one is the start alone, which are the ordinary meanings and are asserted as laws.
- `fn falling_degrees(start: Degree, steps: Nat, count: Nat) -> List(Degree)` — The whole finite walk downward.
- `fn stacked_on(collection: Scale, members: Nat, written: Degree) -> Option(ChordClass)` — Harmonize one degree of a walk with the collection's own stack. Absent when the collection stacks to a sonority the chord vocabulary cannot name, which is how a pentatonic or whole-tone collection reports that it does not harmonize in thirds.
- `fn harmonized(collection: Scale, members: Nat, walk: List(Degree)) -> List(Option(ChordClass))` — Harmonize a whole walk. The quality of each chord is the collection's, so a sequence in minor is a different succession of qualities from the same sequence in major without either being written twice.
- `fn descending_fifths_degree(start: Degree, index: Nat) -> Degree` — The descending-fifths sequence, one index at a time: roots fall four scale steps each time, which is a fifth down inside the collection. OMT 049.
- `fn descending_fifths_chord(collection: Scale, start: Degree, members: Nat, index: Nat) -> Option(ChordClass)` — The chord the descending-fifths sequence reaches at one index.
- `fn descending_fifths(collection: Scale, start: Degree, count: Nat) -> List(Option(ChordClass))` — The descending-fifths sequence as triads. In a major collection this is the succession OMT writes `I–IV–viiº–iii–vi–ii–V–I`, and the diminished triad in it is the collection's doing rather than an exception. OMT 049.
- `fn descending_fifths_sevenths(collection: Scale, start: Degree, count: Nat) -> List( Option(ChordClass), )` — The same sequence as seventh chords, which is how OMT 049 most often presents it because the sevenths chain the resolutions together.
- `fn ascending_fifths_degree(start: Degree, index: Nat) -> Degree` — The ascending-fifths sequence: roots rise four scale steps each time. A different pattern from the descending one and not its retrograde, because the collection is not symmetrical. OMT 049.
- `fn ascending_fifths(collection: Scale, start: Degree, count: Nat) -> List(Option(ChordClass))` — The ascending-fifths sequence as triads.
- `fn descending_thirds_degree(start: Degree, index: Nat) -> Degree` — The descending-thirds sequence, the skeleton under the descending 5–6 pattern: roots fall two scale steps each time. OMT 049.
- `fn descending_thirds(collection: Scale, start: Degree, count: Nat) -> List(Option(ChordClass))` — The descending-thirds sequence as triads.
- `fn ascending_seconds_degree(start: Degree, index: Nat) -> Degree` — The ascending-seconds sequence, the skeleton under the ascending 5–6 pattern and under parallel first-inversion chords: roots rise one scale step each time. OMT 049.
- `fn ascending_seconds(collection: Scale, start: Degree, count: Nat) -> List(Option(ChordClass))` — The ascending-seconds sequence as triads, in root position.
- `fn parallel_sixths(collection: Scale, start: Degree, count: Nat) -> List(Option(ChordClass))` — The same walk with every chord in first inversion: the parallel `6/3` passage OMT 049 describes. The inversion is a designation on the chord class and not yet a bass note — which note actually sounds lowest is still the voicing's decision.
- `fn first_inversion(content: Option(ChordClass)) -> Option(ChordClass)` — Designate the third as bass, keeping absence absent. Written out because an `option` of an `option` is not a chord and this language composes the two by hand.
- `fn stage_music(policy: ChordClass -> EventTrack(WrittenTime), content: Option(ChordClass)) -> EventTrack(WrittenTime)` — Sound one stage of a skeleton under a caller's voicing policy. The policy is a function because the library has no opinion: a stage that cannot be voiced from the bass the caller named is silence here, and the caller can see that it was.

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
- `fn chain(steps: List(Triad -> Triad), start: Triad) -> Triad` — A chain of transformations, applied left to right from a starting triad. Composition is what a chain is — no transformation here is a keyword or a special form, so a list of them is ordinary data and this is an ordinary fold.
- `fn applied(operation: Triad -> Triad, carried: Triad) -> Triad` — One step of `chain`.
- `fn unspelled(spelled: NoteName) -> Pc(12)` — Forget one spelling. Named for what it does to a pitch class rather than for the domain it lands in, so that a piece may import this and `std::post_tonal::pcset` together.
- `fn triad_tones(refined: Triad) -> List(NoteName)` — The three tones of a triad, spelled, from the root upward.
- `fn triad_classes(refined: Triad) -> PcSet(12)` — The projection into the chromatic quotient: the triad as a set of three unspelled pitch classes. This is where a finite group claim becomes sayable. `dbb` major and `c` major are two triads and one set, so a cycle that fails to close in spelling closes here, and the failure and the closing are both facts an author can hold at once rather than one hiding the other.

## `std::voicing`

- `fn close_position(content: ChordClass, bass: Pitch) -> Option(Voicing)` — Stack the class upward from an absolute bass, one member per octave position, taking each next member at the first pitch above the last. This is the one policy `stack c4 major7/2` desugars to.
- `fn drop_position(content: ChordClass, bass: Pitch, from_top: Nat) -> Option(Voicing)` — Close position with one upper note dropped an octave, counted from the top: `drop_position(content, bass, 2)` is the drop-2 voicing.
- `fn voiced_as(content: ChordClass, pitches: List(Pitch)) -> Option(Voicing)` — The voicing an explicit list of written pitches spells, when those pitches really do voice the class: ascending, distinct, and every one a member. This is how a writer voices by hand.
- `fn pitches_of(chosen: Voicing) -> List(Pitch)` — Every sounding pitch of a voicing, lowest first.
- `fn lowest_of(chosen: Voicing) -> Pitch` — The lowest sounding pitch. A voicing always has one, which is why this is not an `option`.
- `fn chord_of(chosen: Voicing) -> ChordClass` — The chord class this voicing voices.
- `fn inversion_of(chosen: Voicing) -> Option(Nat)` — Which member is in the bass, counted from zero, when the bass is a member at all. Absent for a slash bass, which is not an inversion.
- `fn omitting(chosen: Voicing, position: Nat) -> Option(Voicing)` — Drop one numbered member from a voicing, keeping the class it voices. The chord class is unchanged: an omission is a choice about what sounds, not a claim that the chord is a different chord.
- `fn rootless(chosen: Voicing) -> Option(Voicing)` — The rootless voicing a pianist plays under a bass player: the root, in position zero, is the note removed, and it is named here rather than left implicit.
- `fn sound_for(chosen: Voicing, held: Duration(WrittenTime)) -> EventTrack(WrittenTime)` — Sound a chosen voicing for a written length. This is the only way a chord class becomes notes.
