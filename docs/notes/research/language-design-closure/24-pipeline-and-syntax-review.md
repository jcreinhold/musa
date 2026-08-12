# Design review: pipeline, surface syntax, and expansion

## Scope

This is a design review of [20-compiler-pipeline.md](20-compiler-pipeline.md),
[21-surface-syntax.md](21-surface-syntax.md), [22-syntax-extension.md](22-syntax-extension.md), and
[23-values-not-types.md](23-values-not-types.md), read against *Open Music Theory* and Peyton Jones's *The
Implementation of Functional Programming Languages*.

It is not one of the two proof reviews the plan permits. There is no proof to attack yet. The question here is whether
these four notes are consistent enough with each other, with the governing documents, and with the music to be worth
committing as the target for the five paper programs.

## Verdict

The direction is right and the breaking changes are worth making. Four problems must be settled **before** the paper
programs are written, because each one changes what those programs may contain:

1. expansion and name resolution are mutually dependent, and note 20 says they are not (H1);
2. the transformer language has no stated way to take a syntax tree apart (H2);
3. the extension boundary — lexing or grouping — is never stated, and it decides whether the flagship notation is even
   expressible (H3);
4. note 20's claim that the core needs no pattern-failure value is false (H5).

H1–H3 are all consequences of note 22. Note 22 is the most valuable of the four and also the least finished: it adds a
second language to the design in nine pages and leaves its elimination form, its resolution order, and its token
boundary open. Note 23 is the strongest of the four and needs only one correction.

## Findings

### High

**H1. Module-body expansion and name resolution are mutually dependent.**

- **Location:** 20 §"The whole path" and §Form 2 ("Name resolution happens once"); 22 §"The two phases" and §"The
  transformer language".
- **Type:** false statement / missing protocol.
- **Problem:** note 22's transformer kinds include definitions ("An expression transformer cannot silently emit an
  import or a package declaration" implies a definition transformer that can emit a definition), and its table assigns
  `record` declarations, `motif` and `fragment` declarations, and the score adapter to expansion. If a transformer can
  emit a declaration, the set of declarations in a module is unknown until expansion finishes. But finding the
  transformer bound to `staff` requires module resolution first. Note 20's pipeline runs expansion to completion, then
  resolves names once.
- **Why it matters:** this is the ordering that Racket solves with partial expansion of module bodies, and it is not a
  detail. It decides whether `musa-compiler` has one resolution pass or an interleaved expand-and-declare loop, and it
  decides whether `resolve(module_id)` can be a single query at all.
- **Smallest repair:** pick one. Either (a) specify partial expansion: expand each module-body form far enough to learn
  what it declares, collect bindings, then finish expansion — and say so in note 20's Form 2; or (b) forbid
  package-supplied definition transformers, leaving packages with expression and block adapters only. Option (b) is
  smaller and costs `record` and `motif` as package-ownable forms; those stay compiler-owned derived forms, which is
  where note 21 already had them.

**H2. The transformer language has no stated eliminator for `Syntax`.**

- **Location:** 22 §"The transformer language"; 19 §"Keep inference predictable" (no general recursion; user data is
  consumed through generated folds).
- **Type:** missing rule.
- **Problem:** a `staff` adapter must walk a block of unknown depth and length. Note 22 says transformers "may use Text,
  Nat, products, lists, options, results, and finite folds" and that quotation builds templates and antiquotation
  inserts matched syntax. It never says what `Syntax` *is* as a type, nor how a transformer takes one apart. Musa has no
  general recursion, so traversal must go through a generated fold, which means `Syntax` must be a declared strictly
  positive nominal type with a fold — or a set of compiler primitives.
- **Why it matters:** until this is stated, it is unknown whether any adapter can be written in the transformer language
  at all, which makes the central claim of note 22 unverified. It also decides whether the expansion proof obligations
  ("every accepted expansion finishes") are about the rank argument alone or also about a fold over syntax.
- **Smallest repair:** declare the syntax datatype and its eliminator explicitly: the grouped-syntax type, its
  constructors (atom, group, block, sequence), its generated fold, and the pattern language used by templates. Then
  write the `staff` adapter against it — see the acceptance test below.

**H3. The extension boundary is never stated: does a package extend the lexer or only the grouper?**

- **Location:** 22 §"Implementation shape" ("after layout grouping and before ordinary resolution"); 21 §"A written
  pitch is an expression"; 20 Form 1.
- **Type:** missing rule with a concrete consequence.
- **Problem:** note 22 places expansion after layout grouping, which implies one universal lexer and one universal
  grouping pass; a package rearranges core tokens and cannot add new ones. That is the right answer, but it is never
  said, and the flagship example may not survive it. `c#4/4` must lex under the same rules that make `1/4` a `Ratio`
  literal and `c4` an identifier. `tempo 1/4 = 104` and `gap = 1/2` in note 21 show the ratio literal in the general
  language. A dotted duration (`c#4/4.`), a double sharp (OMT 005 shows `D𝄪` as ordinary spelling), and a tie or slur
  marker all have to fall out of the core token set too.
- **Why it matters:** if `#` or `𝄪` must be a token only inside a staff block, the boundary is at lexing, not grouping,
  and note 22's "no arbitrary reader replacement" has been given up quietly. That is a much larger system than the note
  describes, and it breaks the lossless tree's single grammar.
- **Smallest repair:** state the rule as a hard constraint — *packages arrange core tokens; they never add tokens or
  change lexing* — and test it against three notations before the paper programs: staff with double sharps, dots, and
  ties; a rhythmic counting syntax (`1 e & a`, OMT 009/012); and one non-Western syllable notation. If any of the three
  needs a new token, reopen the boundary rather than special-casing staff.

**H4. Package-owned score grammar collides with constitution §1 and the desktop specification.**

- **Location:** 22 §"Revised recommendation"; `docs/rules/constitution.md` §1; `docs/rules/desktop/03-interaction.md`;
  `editors/tree-sitter-musa/README.md` (the drift law).
- **Type:** unexamined cost against a governing document.
- **Problem:** the constitution says moving a note in the score editor changes the source. If a package owns the
  notation grammar, the editor can only perform that edit for grammars it understands. Nothing in note 21 or 22 mentions
  the structured editor, the formatter, or the tree-sitter grammar, all three of which must operate on text whose
  meaning is now supplied by a package.
- **Why it matters:** this is not a reason to reject expansion, but it is a scope decision the notes silently make.
  Today `make fmt-check` formats every `.musa` file and the drift law holds the tree-sitter grammar to the real lexer;
  both assume one grammar.
- **Smallest repair:** state the supported-editing rule. The honest version is narrow: only the standard library's staff
  and score adapters get structured score editing, syntax-aware formatting, and tree-sitter coverage; other adapters
  produce engravable music but are edited as text. Record that as a cost of the decision rather than discovering it in
  prompt 26's successor.

**H5. Exhaustive, ordered matches do not remove the pattern-failure value; uniformity does.**

- **Location:** 20 §Form 4 ("The compiler never carries an internal 'pattern failed, try later' value because source
  matches are exhaustive and ordered before lowering").
- **Type:** false statement.
- **Problem:** Peyton Jones §5.4.1 shows the exact case. Compiling an exhaustive, ordered, overlapping definition by the
  constructor rule alone duplicates right-hand sides; the alternative is `[]` and `FAIL`. His `unwieldy` example is
  exhaustive and still needs one or the other. §5.5 identifies the property that actually removes them — *uniformity*,
  meaning the mixture rule is never needed — and notes that uniformity is strictly stronger than non-overlap. In Musa
  surface terms:

  ```musa
  match (left, right):
    (Some(x), _) -> first(x)
    (_, Some(y)) -> second(y)
    _ -> neither
  ```

  This is exhaustive and ordered. It is not uniform. Lowering it either duplicates `neither`'s body across two
  constructor arms or introduces a join point that the second arm can fall through to.

- **Why it matters:** the core's grammar in note 20 lists "decision trees over constructor tags" and nothing else, so
  the core as written cannot express the lowering of this match. It also affects the deterministic cost model: body
  duplication changes logical cost, and a warm-cache-independent budget has to charge the duplicated form.
- **Smallest repair:** choose one and write it down. Either (a) add join points to the evaluation core and prove they
  cannot escape a match; (b) restrict source matches to uniform ones, which is a real and unusual source restriction
  that must be stated in note 21, not discovered during lowering; or (c) permit duplication and bound it. Option (a) is
  the ordinary answer and costs one core form.

### Medium

**M1. Note 20 says there are two retained record kinds; note 22 adds a third.**

20 §"Source anchors and derivation records" says "Every lowering keeps two related records" (source map, derivation
record). 22 §"Hygiene and source ownership" introduces the expansion record and correctly distinguishes it from
derivation. Diagnostics need it at inference time and later, so it is retained, not transient. Add it to note 20's
section and say which forms carry it.

**M2. Three of the eleven queries are keyed on runtime values.**

`close_music(root_value_id, musical_context)`, `gesture_timeline(intent_id, performance_context)`, and
`prepared_execution(gestures_id, bindings, seed, options)` key a memo table on evaluated values. That requires equality
and hashing over arbitrary typed values including closures, which the language does not provide and should not. Key them
on `(definition_id, revision, context)` instead — the value is then a result, not part of the key.

**M3. Pass law 5 ranges over a form with no semantics.**

"Evaluating a typed body and its evaluation core gives related values" presumes an evaluator for the typed body. Only
the core has an operational semantics in these notes. This is precisely the failure note 20 warns about two paragraphs
later ("a theorem that ranges over three undocumented internal representations"). Either give the typed body a reference
semantics or restate law 5 as: lowering is type-preserving, and the core's value is the program's value by definition.

**M4. Note 21 still states its superseded decisions normatively.**

The update banner points at 22, but §"Recommendation" still says "Do not add public macros", §"Why build-time evaluation
does not require macros" argues at length for a position the next note reverses, and §"Decision for the next draft" ends
with "Defer public macros and full homoiconicity". Its examples (`let phrase = music: c4/4`) also use staff pitch
literals in a module that selects no adapter. A reader following the stated reading order gets two answers. Rewrite 21's
recommendation and decision sections, or merge 21 and 22 into one note with one decision.

**M5. Core-language rules and standard-library adapter rules are not separated.**

"Every entry in a music or voice block must have that type, and the block sequences its entries" (21) reads as a
language rule. After 22 it is a rule of a package-supplied adapter. The same goes for bar lines, `part`, `voice`, and
`|`. This matters for governance: `docs/rules/language/` cannot bind a package's grammar, and a package cannot be held
to a rule written in a language specification. Label each rule with its owner — core form, compiler-owned derived form,
or standard-library adapter.

**M6. The staff adapter's stated scope does not cover ordinary notation.**

"The block sequences its entries" and "durations add" fail for at least four constructs that OMT treats as basic:

| Construct | Source | Why sequencing fails |
| --- | --- | --- |
| tie across a barline | 009 §"Dots and ties" | one sounding note spans two written events in two bars |
| slur | 007 §"Articulations" | a span over events, not an event |
| triplet or duplet | 012 §"Tuplets" | written durations do not sum to the notated span |
| chord, divisi | 008 | simultaneity inside one voice |

Grace notes are a fifth. None of these is exotic; a first score example needs at least ties and chords. Require the
paper programs to include a phrase with a tie across a barline, a triplet, a chord, and a slur, and show what the
adapter expands each one to.

**M7. Note 23's own example begs the question it should settle.**

`up_octave : SpelledPitch -> Result(SpelledPitch, PitchError)`, and `answer` passes a closure of that type to
`map_pitches`. For the example to typecheck, `map_pitches` must thread the `Result`, so every pointwise musical
transformation becomes monadic. Note 19 forbids overloading and type classes; note 21 leaves `try` as an open question
to be answered *after* the paper programs. That ordering is backwards: this is the ergonomic decision the whole theory
library depends on.

There is also a musical error underneath it. OMT 005 treats double sharps and flats as ordinary spelling, and nothing in
the notation system stops a representation from carrying an arbitrary accidental count and register. Spelled
transposition can be **total**; what fails is *rendering* an extreme spelling, which is a notation-target concern — the
position note 23 itself takes in its last paragraph. Make `transpose_spelled` total, push representability failure to
the notation adapter, and settle `Result` chaining before the five programs rather than after.

**M8. The default score adapter is one sentence and load-bearing.**

22 says "A piece may choose one adapter as its local score default". Note 21's Twinkle example depends entirely on this
and never invokes it. Unspecified: how the default is declared, what scopes it (piece, file, package), what happens when
a `score` block and a `voice` block select different ones, and what distinguishes a piece-scoped default adapter from
the "arbitrary reader replacement for a whole file" that 22 rejects two pages later. Specify it or drop bare notation
from the examples.

### Low

**L1. "Each eligible non-recursive `let`" (20 Form 3) leaves "eligible" undefined.** Note 19 is explicit that the
language is pure, needs no value restriction, and may generalize every non-recursive `let`. Say that, or say what makes
one ineligible.

**L2. Two budgets, no stated relation.** 22 charges "a deterministic expansion budget"; 20 charges a "deterministic
compiler resource limit" on evaluation. State whether they are one budget or two, otherwise a macro-heavy build can
spend arbitrarily under one while the other stays green.

**L3. No owner for gesture production.** 20's ownership table covers syntax, resolution, typing, evaluation, kernel
terms, notation plans, prepared execution, live stepping, and sessions. Performance profiles producing
`Timeline<Gesture>` (musical target 3) have no row.

**L4. No `expand` query.** The query list has no expansion entry, so an edit to a macro package has no stated
invalidation edge to its use sites. Add `expand(module_id)` or fold expansion into `resolve` once H1 is decided.

**L5. Adapter version is part of what a score means.** The expansion record retains the adapter definition, which is
right, but nothing connects the adapter's package version to the document's versioned exact identity (obligations §on
identity). One sentence.

## What is right and should not be reopened

- **Four front-end forms, one question each, and no CFG-based MIR.** The argument that Musa's targets do not converge on
  a common low-level instruction language is correct and worth keeping verbatim.
- **Stopping the common pipeline at typed values.** This is the decision that keeps the notation/performance/sound
  distinctions the constitution exists to protect.
- **Resource limits that ignore cache warmth.** A cache hit charging the same logical work as a miss is an unusual and
  correct commitment. It is what makes acceptance a property of the program rather than of the session.
- **Note 23 entirely, apart from M7.** The reopening gate — two independent cases, all three conditions — is the right
  shape for a decision like this, and the invariant table (`Row12`, `Phrase`, `Voicing`, `PreparedExecution`) shows the
  opaque-type answer actually covers the pressure tests.
- **Expansion ranks as the termination argument, and keeping expansion out of inference.** Both are the small versions
  of the right ideas. The rank rule should say that ranks are derived from the syntax dependency graph, not written by
  package authors, so adding a macro call cannot force renumbering across packages.

## Additions to the acceptance test

Notes 20 and 21 each propose a five-program test; they should be one test, and it should check the pipeline claims and
not only surface metrics. Add:

1. **Write the standard library's `staff` adapter in the transformer language, with no compiler privilege.** This is the
   single highest-value experiment in the whole set. If it cannot be written — because of H2, or H3, or because it needs
   inferred types — then note 22's mechanism is justified by an example the compiler will implement in Rust anyway, and
   the public part of it should be deferred.
2. **Expand a staff phrase containing a tie across a barline, a triplet, a chord, a slur, and a grace note** (M6).
3. **Count near-duplicate names per concept.** With no overloading and no dependent types, `transpose_spelled`,
   `transpose_pc`, `transpose_music`, `transpose_chord`, `transpose_row` all coexist. That is an acceptable price, but
   it should be measured on real programs before it is paid, because OMT's breadth guarantees the pattern repeats for
   `map`, `invert`, `retrograde`, and `interval_between`.
4. **Show one `Result`-chaining passage in the tonal package written with plain `match`** (M7), and judge the `try`
   question from that page rather than in the abstract.
5. **Show a lowered non-uniform match** (H5) so the evaluation core's grammar is fixed by an example.

## Recommendation

Commit the direction. Do not commit these four notes as the paper-program target until H1, H2, H3, and H5 have answers,
and note 21's superseded sections are repaired (M4).

The pattern that ended the last round was a proof over representations that had not been pinned down. Notes 20 and 22
reintroduce exactly that risk in a new place: an expansion phase whose data type, resolution order, and token boundary
are all still open, with six proof obligations already written against it. Pin the syntax datatype and the expansion
protocol first. The proof obligations in 22 are good ones and will be provable once there is something to quantify over.
