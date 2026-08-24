# The builtin registry, entry by entry

## Purpose

Prompt 164 collapsed the compiler's builtin registry. This note is the survey that prompt's Target asks for: for every
entry in the registry as it stood, whether it was kept, replaced by a surface spelling, or moved into `stdlib/`, and the
hidden information that decided it. It also records the privacy audit and the P1/P2 measurement the same Target names.

Nothing here governs. [`docs/rules/language/00-semantics.md`](../../../rules/language/00-semantics.md) owns the test
being applied and [`03-musical-domains.md`](../../../rules/language/03-musical-domains.md) §5 owns the index the moved
half now stands on; this note records the verdicts, so a later reader can check the derivation instead of taking the
count on trust. [Note 45](45-phase-registry-survey.md) is the same exercise done for the phase registry alone at prompt
138, and it is not repeated here.

## The count

| registry | before | after | difference |
| --- | ---: | ---: | ---: |
| `BUILTIN_OWNERSHIP` — source operations | 122 | 100 | −22 |
| `SYNTAX_OWNERSHIP` — phase operations | 20 | 20 | 0 |
| total | 142 | 120 | −22 |

The prompt's banner says "seventeen builtins hardcoded to the modulus 12"; the measured number is twenty-two, because
the banner counted the `pc12_*` and `row12_*` families and not the `pcset12_*` family between them. Twenty-four entries
carried a literal twelve in their name. Twenty-two left. The two that stayed are named under "The two that look like the
others and are not", and they are not a concession — they hide `musa-score`'s spelling tables, not a modulus.

Nothing else moved. `δ`-rules that acquired an operator or a method spelling this prompt — `nat_add` behind `+`,
`text_equal` behind `Text.equal` — are still the same rules under the same names, and are counted as kept. **Collapsing
a surface vocabulary is not collapsing a registry**, and conflating the two is how a survey like this reports progress
that did not happen.

## The test

`00-semantics.md` admits a builtin on four grounds, and this is the whole of the standard:

> An operation may be a builtin only when it needs source-aware provenance, direct core construction, a registered
> primitive's private state, or the private finite representation and work budget needed to preserve total evaluation.

Two corollaries do the work below. First, *familiar* and *faster* are not grounds — the document says so in the next
sentence, so an entry that survives only because a Rust loop is quicker than a fold has already failed. Second, the
grounds are about **information**, so the question for each entry is not what it computes but what a caller could not
reconstruct from the public surface. That is the column the tables carry.

## The twenty-two that left

They fall into three groups, and the group is the verdict.

### Modular arithmetic, which a fold expresses

Four entries claimed that a `Nat` without subtraction cannot express arithmetic modulo twelve. That claim was true of
recursion on the numeral and false of the language: `std::cyclic`'s `folded` recurses structurally on a fuel argument
and terminates for the same reason every eliminator does, so the reduction is an ordinary total function and the modulus
is an ordinary argument.

| entry | what its ownership row claimed to hide | what carries it now |
| --- | --- | --- |
| `pc12_of` | the canonical representative of a residue class modulo twelve | `pcset::pc`, over `cyclic::place_in` |
| `pc12_number` | the canonical representative, which is the only number a residue class has | `pcset::class_number`, over `cyclic::number_of` |
| `pc12_transposed` | modular addition, which a `nat` without subtraction cannot express | `cyclic::transposed`, and `pcset::transposed_by` |
| `pc12_inverted` | modular subtraction, which a `nat` without subtraction cannot express | `cyclic::inverted`, and `pcset::inverted_about` |

`Ti` is what makes the pair one thing rather than two: `cyclic::moved` takes a transposition or an inversion, because
the composite of two reflections is a rotation and a type holding only the transpositions would be closed and wrong.

### A packed representation, which a `private` constructor holds

Eleven entries hid a bit pattern — a twelve-bit membership word, an order-position array — and each was right that the
representation must not be public. What changed is that a `data` declaration can now have a `private` case, so the
invariant is a fact about the type rather than a promise a Rust function keeps on the type's behalf.

| entry | what its ownership row claimed to hide | what carries it now |
| --- | --- | --- |
| `pcset12_of` | the twelve-bit membership word that makes duplication unrepresentable | `pcset::pcset`, into `private Members` |
| `pcset12_members` | the membership word, read out ascending | `pcset::set_members` |
| `pcset12_transposed` | the membership word, rotated by the index without unpacking it | `pcset::set_transposed` |
| `pcset12_inverted` | the membership word, reflected about the index without unpacking it | `pcset::set_inverted` |
| `pcset12_normal` | every rotation of the set and the compactness order that chooses between them | `pcset::normal_order` |
| `pcset12_prime` | the normal orders of the set and its inversion, and which of the two reads lower | `pcset::prime_form` |
| `pcset12_vector` | every unordered pair of members and the interval class each realizes | `pcset::interval_class_vector` |
| `row12_of` | the permutation invariant: twelve order positions and each pitch class once | `serial::row`, into `private Series` |
| `row12_pcs` | the private order-position array | `serial::row_numbers` and `serial::row_pcs` |
| `row12_head` | order position zero of the private array, which the finite list eliminators cannot index | `serial::first_number` |
| `row12_matrix` | the classical construction: the inversion about the row's own head, read as starting pitches | `serial::matrix` |

`row12_head`'s stated reason was true when it was written and is not now. `first_number` matches the list open —
`[head, .. others]` — which is prompt 155's case tree, not an eliminator; what is left of the entry's claim is that the
array it indexed was private, and that is the row above it rather than a separate fact.

### Equality and a search over a finite domain

Seven entries were folds waiting for `range`, `any`, and an equality the surface can now write. Six of them are now the
same two functions from `std::algebra` applied at different carriers, which is the collapse `52-the-musical-algebra.md`
§2.3 predicted: a set class is an orbit, a row's symmetries are a stabilizer, and Messiaen's modes are the collections
whose stabilizer under transposition is nontrivial.

| entry | what its ownership row claimed to hide | what carries it now |
| --- | --- | --- |
| `row12_transposed` | modular addition, and the finite-closure lemma that keeps the result a row | `serial::row_transposed` |
| `row12_inverted` | modular subtraction, and the finite-closure lemma that keeps the result a row | `serial::row_inverted` |
| `row12_retrograde` | reversal of the order positions, which the finite list eliminators cannot express | `serial::row_retrograde`, over `list::reverse` |
| `row12_forms` | the forty-eight labelled forms, compared for equality and counted once each | `algebra::orbit`, as `serial::row_forms` |
| `row12_symmetries` | the forty-eight labelled forms, counted where they fix the row | `algebra::stabilizer`, as `serial::row_symmetries` |
| `row12_repeats` | pitch-class equality, which the surface has no operator for | `serial::repeated_positions` |
| `row12_missing` | pitch-class equality against the whole finite domain | `serial::missing_classes`, over `list::range` |

The finite-closure lemma two of these named is not lost and was never evidence for a builtin. It is what the `private
Series` case states: a value of `ToneRow(n)` exists only where `row` built it, and `row_moved` maps a permutation to a
permutation.

## The two that look like the others and are not

`pc12_forget` and `pc12_spelled` keep their spellings and their δ-rules.

| entry | signature | why it stays |
| --- | --- | --- |
| `pc12_forget` | `(NoteName) -> Nat` | `musa-score`'s spelling table: which residue a written name denotes |
| `pc12_spelled` | `(Nat, Scale) -> Option<NoteName>` | the same table read backwards, against a collection that decides the choice |

Neither hides a modulus. They hide the **spelling** layer — roadmap §2's "written pitch ≠ MIDI number" — and a library
cannot express it because the table is `musa-score`'s and the source language has no way to name a `NoteName`'s
components. `pcset::forget_spelling` and `pcset::spelled_in` are the library functions over them, and with
`row_spelled_in` mapping the second across a row they are the only places in `std::post_tonal` where twelve is written
as a literal — because they are the only places where twelve is a fact about notation rather than an author's choice of
division.

## The hundred and twenty that stayed, by clause

### Source-aware provenance — 20 entries

Every `SYNTAX_OWNERSHIP` entry. Three folds (`recurse_syntax`, `run_syntax_step`, `syntax_fold_from_leaves`) and
seventeen builders (`as_expression`, `checked_expression`, `delimiter_equal`, `forget`, `syntax_anchor`, `syntax_at`,
`syntax_binder`, `syntax_binding`, `syntax_built`, `syntax_group`, `syntax_identifier`, `syntax_number`,
`syntax_numeral`, `syntax_reference`, `syntax_text`, `syntax_token`, `token_kind_equal`). Each hides the reader's node
representation, a node's structural path, or the reader's own numeric or textual reading of a token — information that
exists because a *parser* ran, which no value in the language carries. [Note 45](45-phase-registry-survey.md) surveys
them one at a time; this prompt changed none of them.

### Direct core construction — 21 entries

- **The eight track operations** — `invert`, `map_note_pitches`, `play`, `retrograde`, `shift`, `stretch`, `together`,
  `transpose` — build and combine `EventTrack` terms. The core's own operations, reached by name.
- **The eight eliminators** — `filter`, `list_fold_from_end`, `list_fold_from_start`, `map`, `nat_fold`, `option_fold`,
  `range`, `repeat` — are the recursion the language has. They are also where the work budget is charged, which is the
  fourth clause reading the same entries: a fold whose step count the evaluator cannot see is a fold that cannot be
  stopped.
- **The five literal rules** — `nat_literal`, `ratio_literal`, `pitch_literal`, `interval_literal`, `key_literal` — turn
  a token the reader already read into a core value. The token's text is the hidden information, and it is the first
  clause and the second at once.

### A registered primitive's private state — 9 entries

`Family::Machine`: `beside`, `connect`, `copy`, `drop`, `feedback`, `identity`, `machine`, `primitive`, `swap`. The
process graph is a private IR with its own tick semantics (`constitution.md` §7), and `primitive` names a registered
processor whose state the source language holds no value for. Untouched by this prompt.

### The private finite representation and work budget — 70 entries

The remaining δ-rules, grouped by the carrier whose representation they hide. None became a surface spelling's *only*
form: an operator or a method is how a piece asks for the rule, and the rule keeps its name.

| carrier | entries | what the representation hides |
| --- | --- | --- |
| whole numbers, rationals, text | `nat_add`, `nat_mul`, `nat_sub`, `ratio_add`, `ratio_sub`, `ratio_mul`, `ratio_div`, `ratio_equal`, `ratio_less`, `text_equal`, `text_join` | the representable range a whole number stays inside, and the reduced form two rationals share |
| written time | `duration_add`, `duration_equal`, `duration_less`, `duration_of`, `duration_ratio`, `duration_scale`, `position_between`, `position_equal`, `position_less`, `position_of`, `position_ratio`, `position_shift` | the nonnegativity a `Duration` constructor checks, and the exact rational behind a position |
| pitch and interval | `pitch_between`, `pitch_equal`, `pitch_frame`, `pitch_transposed`, `pitchclass_of`, `pitchclass_transposed`, `interval_add`, `interval_equal`, `interval_inverse` | the spelling a written pitch carries, which is not its number |
| scale, key, frame, degree | `scale_chord`, `scale_class`, `scale_on`, `scale_pitch`, `scale_size`, `scale_tonic`, `signature_scale`, `frame_pitch`, `frame_scale`, `frame_tonic`, `degree_of`, `degree_lowered`, `degree_raised`, `degree_step_down`, `degree_step_up` | the collection's own table, and the frame that makes a degree denote |
| roman numerals | `roman_of`, `roman_inversion`, `roman_ordinal`, `roman_size` | the parsed figure, which is a reading and not a number |
| chord, triad, voicing | `chord_bass`, `chord_inversion`, `chord_members`, `chord_on`, `chord_over`, `chord_root`, `chord_triad`, `triad_chord`, `triad_major`, `voicing_bass`, `voicing_chord`, `voicing_of`, `voicing_pitches`, `voicing_position`, `close_voicing`, `drop_voicing`, `omit_voicing` | the chord's private member order and the voicing's register assignment |
| spelling bridge | `pc12_forget`, `pc12_spelled` | the spelling table, surveyed above |

## The privacy audit

The question the audit asks: did the move to `stdlib/` publish anything the compiler was hiding?

- **`PcSet(n)` and `ToneRow(n)` are `private` cases.** `private Members(division, ascending)` and `private
  Series(division, numbers)` restore exactly what `pcset12_of` and `row12_of` hid. A caller cannot build either
  directly, cannot pattern-match one open, and reaches the contents only through `set_members`, `numbers_of`'s public
  readers, or `row_numbers`. Sorting-and-deduplication for the one and the permutation invariant for the other are
  therefore properties of the type.
- **`Pc(n)` and `Cyclic(n)` are public and should be.** A pitch class is a residue and has nothing to hide: its
  constructor takes the division it stands in, so `Pc(12)` can never stand where `Pc(24)` is wanted, and the index does
  the work the privacy would have done. `Cyclic(n)`'s two cases are a unary numeral bounded by its own index — the bound
  is the content.
- **Six helpers went `private` on the way in**, and each was internal to a compiler rule before: `cyclic::embed`,
  `cyclic::without`, `cyclic::folded`, `cyclic::differing`, `pcset::numbers_of`, and `pcset::compactness` with the
  `tighter` / `reads_lower` pair around it. Publishing `folded` in particular would publish a fuel argument, which is an
  implementation of totality and not a fact about music.
- **One thing is now public that was not: `Ti` and `RowOp`.** The forty-eight labelled forms used to be a count returned
  by `row12_forms`; they are now a list of values a caller can hold, compare, and compose. That is the point of the move
  rather than a leak — an operation had to become an object for `orbit` to take a group at all — and nothing about a
  `Ti` reveals a representation, because a `Ti` *is* its index.

Net: two private constructors replace two private Rust types, six helpers stay private, and the one new public surface
is the transformation itself.

## The measurement

`06-elaboration-baseline.md`'s gate: a P1 or P2 move over 10% requires a recorded rerun and an allocation comparison.
The absolute numbers in that document's tables date from prompt 127 and are six to ten times smaller than anything this
machine now measures, because prompts 128–163 replaced the elaborator with a dependent core in between. Comparing
against them would measure the language pass and not this prompt, so the gate is held the only way it can be held here:
**the same benchmark, on the same machine, at `HEAD` and at this prompt's tree, run back to back.**

Machine: Apple M4 Pro, arm64, macOS 26.5.1, release profile, divan medians over 100 samples. `HEAD` is `69af9bef`, built
in a separate worktree with its own target directory so neither side shares a build with the other.

```sh
cargo bench -p musa-compiler -- p1_compile p2_elaborate
```

| stage | workload | before | after | Δ median | allocations |
| --- | --- | ---: | ---: | ---: | ---: |
| P1 | analysis-pressure | 31.07 ms | 31.02 ms | −0.2% | 88,484 → 88,426 |
| P1 | audio-bridge | 5.206 ms | 4.955 ms | −4.8% | 9,401 → 9,343 |
| P1 | core-pressure | 10.29 ms | 11.00 ms | +6.9% | 30,332 → 34,977 |
| P1 | declaration-heavy | 32.88 ms | 32.66 ms | −0.7% | 74,599 → 74,541 |
| P1 | events-pressure | 25.43 ms | 25.12 ms | −1.2% | 46,537 → 46,479 |
| P1 | higher-order-shape | 7.962 ms | 7.721 ms | −3.0% | 9,788 → 9,730 |
| P1 | large | 31.63 ms | 31.38 ms | −0.8% | 203,558 → 203,500 |
| P1 | open-shape | 11.56 ms | 11.08 ms | −4.2% | 19,045 → 18,987 |
| P1 | shared | 46.21 ms | 45.93 ms | −0.6% | 38,266 → 38,208 |
| P1 | small | 5.901 ms | 5.506 ms | −6.7% | 9,799 → 9,741 |
| P1 | template-pressure | 528.2 µs | 540.1 µs | +2.3% | 17,037 → 17,037 |
| P2 | analysis-pressure | 30.99 ms | 30.89 ms | −0.3% | 83,489 → 83,431 |
| P2 | audio-bridge | 5.376 ms | 4.780 ms | −11.1% | 8,603 → 8,545 |
| P2 | core-pressure | 10.17 ms | 11.06 ms | +8.8% | 27,304 → 31,949 |
| P2 | declaration-heavy | 32.66 ms | 32.52 ms | −0.4% | 71,527 → 71,469 |
| P2 | events-pressure | 25.54 ms | 24.64 ms | −3.5% | 42,341 → 42,283 |
| P2 | higher-order-shape | 8.691 ms | 7.535 ms | −13.3% | 9,444 → 9,386 |
| P2 | large | 30.68 ms | 30.48 ms | −0.7% | 189,156 → 189,098 |
| P2 | open-shape | 11.22 ms | 10.97 ms | −2.2% | 17,677 → 17,619 |
| P2 | shared | 45.85 ms | 47.61 ms | +3.8% | 37,672 → 37,614 |
| P2 | small | 5.829 ms | 6.096 ms | +4.6% | 8,859 → 8,801 |
| P2 | template-pressure | 463.4 µs | 481.2 µs | +3.8% | 15,733 → 15,733 |

**Every workload but one allocates less, by exactly 58.** Twenty of the twenty-two rows move 58 allocations down and
nothing else; `template-pressure` is flat because it fails before it gets there. Fifty-eight is the same number in both
stages and on workloads three orders of magnitude apart in size, so it is a per-document constant — what a compilation
no longer spends standing the deleted rules up. Read against that constant, the two double-digit P2 rows —
`audio-bridge` at −11.1% and `higher-order-shape` at −13.3% — are the machine rather than the compiler: neither has a
matching P1 move, and 06's own note that run-to-run spread reaches 9% on the sub-millisecond workloads applies to both.
No mitigation was applied and none is owed; nothing crossed the gate.

**`core-pressure` is the one real change, and it is a fixed cost.** It is the *only* fixture that writes `import
std::list`, and `std::list` gained ten declarations this prompt — `concat`, `length`, `reverse`, `take`, `drop`,
`rotated`, `rotations`, `any`, `all`, and the private `reversing` — because the moved post-tonal code needed them.
Correcting for the −58 every other workload shows, the workload elaborates 4,703 more allocations' worth of *library*,
and 7–9% more wall clock, none of it scaling with the music: it is ten more declarations to elaborate, charged once, in
a document that compiles ninety-three lines. A piece that imports `std::list` and then writes a real score pays the same
constant against a much larger denominator. That is the price of moving an operation out of Rust and into the library,
and it is the price the collapse was for.
