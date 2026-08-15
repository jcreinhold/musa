# 42 — The dependent core: the decision, its evidence, and what it overturns

Constitution §9 and obligations §10 are amended. The *Inferred* property becomes bidirectional elaboration with
metavariables; *Total* becomes well-founded rather than structural; the explicit refusal of dependent and refinement
types is deleted; the refusal of type-directed macros is narrowed to admit typed quotation; and obligations §10 gains a
second admission route for measured engineering evidence. This record discharges
[`docs/rules/README.md`](../../../rules/README.md)'s six requirements under six headings, answers note 39 §11.2's
five-item checklist point by point, and states the alternatives that were refused.

It is written for a reader who thinks this is a mistake. Notes 38 and 39 argued against exactly this change, and note 39
§8.1's five decisive reasons are not refuted below — they are shown to be answers to a different question.

---

## 1. The concrete reason

**Musa's surface language is not strong enough to write Musa in, and the standard library is the proof.**

The evidence is `stdlib/src/adapters/staff.musa` at revision `afc9caa`: **2,404 lines and 93,252 bytes** of Musa that
read staff notation and produce a `std::notation::staff` document. It is the largest program in the repository written
in the language the repository ships, it was written by us against a spelling we designed, and it has been rewritten
four times against four successive repairs to the phase interface. It is not a program that failed for want of care.

What it spends its size on, measured rather than described:

| Compensation | Count | What the language could not say |
| --- | --- | --- |
| `call1`–`call5`, `call7` argument builders (lines 365–476) | 6 definitions, 20 call sites | A call is assembled by hand as a nested `syntax_group`/`syntax_token` tree, because there is no way to *write* the call and splice its arguments in. |
| Hand-allocated role integers | 27 distinct values, 1–53 with 21 gaps, across 56 `syntax_built` calls | Provenance is an integer the author picks. Nothing checks that two nodes did not pick the same one, and the gaps are the record of values abandoned mid-edit. |
| `data Pending`'s eight named fields (line 316) | 16 construction and destructuring sites | Reading one field requires naming all eight. There is no record projection and no field update. |
| `text_equal` against token-kind and delimiter spellings | 21 sites, 13 distinct string literals | A token kind is `Text`. `"PitchLiteral"`, `"braces"`, `"Rational"` are the compiler's own vocabulary, retyped as strings and compared. |
| The reading algorithm runs backwards | the whole `the reader` section, lines 1551–2058 | A list cannot be constructed. The adapter's own header says so. |

The header's *"What this adapter wanted and did not get"* is five items long and was written by the author of the
adapter, not by a reviewer looking for evidence. Two of the five — "a number `expand` computed still cannot be written
back as a numeral" and "**a list cannot be built**" — are language limitations with no musical content whatsoever. A
notation reader that must run backwards because it cannot append to a list is not making a statement about notation.

Two things this evidence is **not**. It is not a claim that the adapter is badly written; note 41 measured the last
rewrite and found the ergonomic repairs did the shrinking and the recursor bought the one thing the fold could not
express. And it is not a claim that a *musical* operation needs a value in a type. Note 39 §8.1's first reason still
stands, and §9 below says so plainly.

---

## 2. Which current examples no longer work

Under the old rules, correctly and by design, the following are unwritable. Each is a program someone tried to write.

1. **`staff.musa`'s construction half, written as the notation it constructs.** `Sounded(anchor, event, items)` has to
   be assembled as a layout group holding an identifier, a parenthesis group, and two comma tokens, at three hand-chosen
   role integers. The program a reader would recognize — write the call, splice the three arguments — is not expressible
   because there is no quotation.
2. **A list the adapter builds rather than maps.** Note 41 §7 records this as a finding of its own rewrite: a group
   inside `[ … ]` has nowhere to put the several pitches it might hold, so it is not a pitch. The whole reading
   direction is shaped around the absence.
3. **Reading one field of `Pending`.** Every read destructures eight. This is not a workaround the author chose; it is
   the only elimination form the language has for a product.
4. **A token kind that cannot be misspelled.** `text_equal(kind, "PitchLiteral")` type-checks against any string. The
   compiler already knows the finite set of token kinds and hands the adapter a `Text`.
5. **`Vec A n` and anything like it.** Note 39 §8.3 names four: a length-indexed vector, a proof that a transformation
   preserves `n`, a type of well-connected graphs indexed by their ports, a phrase type indexed by a proof of its
   formation rule. The first is what `Syntax<Cat>` needs to stop being a single untyped node type.
6. **A generic operation that works for `Nat` and `Ratio` without being written twice.** The builtin registry carries
   117 source operations, and `nat_add`/`ratio_add`, `nat_equal`/`ratio_equal`/`text_equal` are the shape of most of
   them. Note 39 §6.5 accepted this duplication as a cost; the registry is the bill.

Item 6 is the one that matters for scope. Traits could be added without a dependent core, and were considered on their
own in §9.1. They were not enough.

---

## 3. The replacement rule, in plain language

**One small dependently typed core. One surface language elaborated into it. Nothing else changes.**

- A type may mention a value. `Vec A n`, `Syntax<Cat>`, and `Id a b` are ordinary declarations rather than things the
  language cannot say.
- Type checking is **bidirectional**: a term is checked against a type that is already known, or its type is inferred
  and the result flows outward. Metavariables and pattern-fragment unification fill in what the program determines.
  There are no principal types and there is no global inference. What a program means is fixed by what it says.
- Definitional equality is decided by **normalization by evaluation**. The checker is an evaluator.
- **Every accepted program still finishes**, and the checker can see why: every recursive definition decreases a
  well-founded measure the checker verifies. Constructor size is the common case and is not the rule. There is no
  `partial` keyword.
- **Traits are records the elaborator fills in**, coherently and without search: no overlapping instances, an orphan
  rule, no defaulting, ambiguity is an error, and explicit qualification is always available. `=` (propositional, proved
  by `refl`) and `==` (computational, from `PartialEq`) stay different words for different things.
- **Records are nominal**, with projection and `with` update including nested paths. Enums are typed and namespaced.
- **Quotation is typed.** `quote at here { … }` produces a `Syntax<Cat>` of a stated category; `$x` splices; provenance
  is *derived* — `Derived { origin, quotation, path }` — so a role integer is not something an author writes. The
  inverse is a syntax pattern, which destructures through the same case-tree compiler as every other pattern.

What is **not** admitted: general recursion, `partial`, `fix`, call-by-push-value stratification, first-class signals,
macros that dispatch on an inferred type, and worlds. Each remains a separate amendment with its own evidence.

---

## 4. The formal specification and the code map

The specification is rewritten by the prompts that follow this one, not by this record:

| Document | Prompt | What it becomes |
| --- | --- | --- |
| `docs/rules/language/02-core-calculus.md` | [129](../../../plan/prompts/129-dependent-core-spec.md) | Universes, Π, Σ and dependent records, inductive families with strict positivity, dependent match with coverage, the identity type, NbE conversion, bidirectional elaboration, metavariables, the well-founded termination rule, and §5's metatheory obligations restated against all of it. |
| `docs/rules/language/01-surface.md`, new `10-traits.md` | [130](../../../plan/prompts/130-trait-and-surface-spec.md) | Records, enums, traits and coherence, operators through traits, inherent methods with exact receiver lookup, collections. |
| new `docs/rules/language/11-quotation.md` | [131](../../../plan/prompts/131-quotation-spec.md) | `Syntax<Cat>`, quotation and splicing, derived provenance, syntax patterns, and what survives of the sealed-step recursor. |
| `docs/rules/language/06-performance.md` | [144](../../../plan/prompts/144-diagnostics-and-performance.md) | The P1/P2 baselines re-measured against a checker that normalizes during conversion, under the existing 10% gate. |
| `docs/plan/code-map/` | [149](../../../plan/prompts/149-language-pass-closure.md) | Every crate the pass touched, including `musa-core`. |

The code map cannot be updated in this commit for the honest reason that no code exists yet: `musa-core` is a crate
prompt 133 creates. What this record *does* fix is where it sits — `docs/plan/roadmap.md` §15.12, a leaf below
`musa-compiler` with `indexmap`, `thiserror`, and `tracing` as its whole dependency list, knowing nothing about pitch,
time, notation, or audio.

**Prompt 132 may falsify all of this before any of it is implemented.** It trials the specification on nine complete
programs on paper, and if one of them needs rank-2 polymorphism, a `partial` definition, search during trait resolution,
or a hand-written provenance path, the design is wrong and 129–131 are repaired rather than built.

---

## 5. How stored files and public APIs migrate

**One migration, at [prompt 142](../../../plan/prompts/142-surface-cutover.md), not two.** Phases B and C build
`musa-core` as a new leaf crate while the existing checker keeps working, which is why the compatibility oracle
(`existing_language_behavior_matches_the_migration_oracle`) can stay untouched through five prompts of new
implementation. 142 is the single prompt permitted to move it, and every moved entry is argued in
`crates/musa-compiler/tests/fixtures/elaboration-expected-changes.json`.

| What is stored | How it migrates |
| --- | --- |
| `.musa` source — `stdlib/`, `examples/`, book fixtures, desktop and LSP fixtures | Rewritten by 142 in one commit. Green at the end, not in the middle. |
| Semantic hashes, kernel digests, Origin paths | Held by the compatibility oracle. A moved hash is a behaviour change and is argued by slug in the expected-changes fixture. |
| MEI, LilyPond, MusicXML, MIDI corpora | Unchanged. A language that elaborates differently and exports differently has changed two things and can prove neither. |
| Diagnostic codes | Extended, not renumbered. `Code` is a named enum, so conversion, coverage, termination, unsolved-metavariable, and ambiguous-instance failures are new variants with new `musa explain` entries (prompt 144). |
| `ProjectSession`, `compile`, `render_notation`, `compile_graph` | Unchanged signatures. The core is below `musa-compiler`; nothing above it learns that the checker was replaced. |
| `crates/musa-compiler/src/infer.rs` | Deleted at 142, with every superseded checking path in `core.rs`. Not kept behind a flag — a second checker is a second semantics. |

`staff.musa` migrates at 142 with its structure intact — still backwards, still without quotation — and is *rewritten*
at 145. Splitting the two is what makes 145's measurement mean anything.

---

## 6. The old argument, kept visible

Requirement 6 exists so that overturning an argument does not erase it. The arguments this amendment overturns are:

**Note 26 §§9–10** set the admission conditions for a language feature and the nine proof obligations for the adapter
boundary. The admission conditions are amended by this record, not discarded: obligations §10 keeps the musical route
verbatim and gains a second one.

**Note 38 §10** reviewed the staff adapter as implementation evidence and concluded that per-type eliminators, totality,
and the environment evaluator were each correctly chosen. It was reading the adapter's *reading* half. The construction
half — `callN`, the role integers, the emitting section at lines 353–589 — was not in the evidence set, and it is where
the compensation concentrates.

**Note 39 §8** rejected dependent types on five reasons, of which the first is the load-bearing one: *no two materially
different musical operations need a value in a type.* **That reason is not refuted.** No musical operation in this
repository needs a value in a type today. What §8 did not ask is whether a *program we ship, written in Musa*, needs one
— and §8.3's own closing sentence anticipated the gap: "If a later consumer must manipulate such evidence before
evaluation … dependent types can be reopened with that evidence." `Syntax<Cat>` is exactly a consumer manipulating
evidence before evaluation. The reopening is on §8.3's own terms.

**Note 39 §6.5** accepted per-type duplication as a cost. The builtin registry's 117 source operations are what that
cost grew into. Prompt 143 collapses it and reports what shrank and what did not, which is the honest way to find out
whether §6.5 was wrong or merely early.

**Note 39 §11.1** described what dropping totality would require. This amendment does not drop totality; §8 below
explains why, and the answer is that §11.1's own reasoning holds and points the other way.

---

## 7. Note 39 §11.2, answered item by item

§11.2 listed five things that adopting dependent types would require. It was written by an argument recommending against
the adoption, which is what makes it the right checklist.

**7.1 — "replacing constitution §9's rank-1 HM rule with an exact bidirectional dependent-checking rule and deleting the
explicit refusal of dependent/refinement types."** Done in this commit. §9's *Inferred* property is now *Checked
bidirectionally*, and it states what replaces principal types: a term is checked against a known type or its type is
inferred, metavariables and pattern-fragment unification solve what the program determines, and a signature may mention
a value. The refusal paragraph loses dependent and refinement types and says, in the paragraph after it, exactly what
was narrowed and what stands.

**7.2 — "choosing whether types may depend only on values, whether dependent elimination is weak, or whether CBPV
stratification is adopted."** Answered: **types may depend on values, dependent elimination is strong, and CBPV is not
adopted.**

- *Types depend on values.* Musa is strict and pure and every value is finite, so there is no value/computation
  distinction to be careful about at the type level, and the two consumers that motivated this — `Syntax<Cat>` and
  `Vec A n` — index over ordinary first-order data.
- *Strong (dependent) elimination.* A weak eliminator cannot type `Syntax<Cat>`'s own destructuring, which is the
  program the feature is for. Prompt 135 owes the coverage proof.
- *CBPV is declined, and the reason is that nothing asked.* §11.2 notes that adopting it would also remove the explicit
  CBPV refusal — which is a reason to be careful, not a reason to take it. CBPV pays for itself when a language has
  effects to stratify or an evaluation order to make explicit; Musa is strict, pure, and total, so the stratification
  would be a distinction with no terms on either side of it. The refusal stays in §9 exactly as written.

**7.3 — "amending obligations §10 with two musical uses or one otherwise unstated safety boundary."** **This is the one
item the amendment does not satisfy on §11.2's terms, and it says so rather than stretching.** There are no two musical
uses. There is no unstated safety boundary. Obligations §10 is therefore amended to add a second admission route —
measured engineering evidence from a committed Musa program the language deformed — with four requirements attached: the
program named and committed at a stated revision, its size measured, its compensating constructs enumerated and counted,
and a later prompt that rewrites it and reports the new measurement. §1 above supplies the first three; prompt 145 owes
the fourth.

The route is deliberately narrow. "The compiler would be nicer" does not open it; a committed program that we wrote,
measured, and cannot write well does. And it is the harder route to fake: a musical argument can be made from an
analogy, whereas 2,404 lines either shrink or they do not.

**7.4 — "replacing Algorithm W, principal types, and the closed-value environment proof with conversion, open neutrals,
reification, weakening/renaming, and the chosen normalization/partiality theorem."** Accepted in full, and scheduled:
prompt 133 builds NbE (eval, quote, conversion) with conversion decidable and an equivalence; 134 replaces the checker;
135 adds coverage and the termination checker; 148 re-derives the metatheory matrix — NbE soundness and completeness,
conversion decidability, coverage, termination, canonicity, subject reduction — plus the storable-data and machine-port
boundaries and the privacy and second-path audits the old core carried. `infer.rs`'s unifier is deleted at 142, not
kept.

**7.5 — "repairing prompts 127a, 127aa–127b, 153, and 146, plus every language/tooling prompt whose diagnostics assume
principal inferred types."** Those ranks moved; the obligation did not. 127a and 127aa–127d are `done` and are not
edited — the amendment they made (event tracks and machines below a temporal kernel) is orthogonal to this one and
survives it untouched. The old 153 and 146 are now [150–153](../../../plan/prompts/150-machine-runtime.md), repaired to
depend on 149. Prompt 149 owns the sweep for diagnostics that assume principal inferred types, and it is a separate
prompt precisely because doing it inside the implementation would hide it.

---

## 8. Totality: kept, and widened rather than dropped

A reader will expect this half to have gone the other way, because dependent type theories usually arrive with a
termination checker that authors fight. It did not, and the reason is a fact about Musa rather than a preference.

**Musa is almost entirely a compile-time language.** Adapters run in the expansion phase. Analyses run inside
`musa check`. The desktop re-typesets on a keystroke, under a 120 ms budget after debounce. So divergence in Musa is not
a composer's infinite loop that a stop button ends — it is a compiler that does not return and an editor that stops
answering while the composer types.

**A dependent core makes that strictly worse.** Conversion is decided by normalization, so the checker *is* an
evaluator: under general recursion, a diverging term hangs type checking, not merely evaluation. The failure mode moves
from "this program loops when you run it" to "this program loops when you look at it".

**What is replaced is the reason a program finishes, not the guarantee.** Structural decrease on constructor size is
sufficient and too narrow: note 39 §12.3 names finite graph search, mutually recursive musical grammars, and analyses
whose measure is not constructor size as the cases it fails. The rule becomes a well-founded measure the checker
verifies. Everything the old rule accepted, the new one accepts.

**§12.3's condition is met by the well-founded direction and not by the partial direction.** §12.3 asks that the
smallest failing fold program be recorded before choosing syntax or a termination checker. The smallest failing program
is in `staff.musa`'s header and in note 41 §7: a reader that must run backwards because it cannot construct a list, and
a group inside `[ … ]` that cannot hold the pitches it was written to hold. That evidence justifies *checked*
well-founded recursion, which is what §12.3 says it justifies, and it justifies nothing about `partial`. There is no
`partial` keyword, no `fix`, and no escape hatch. Adding one later needs §12.3's evidence again and this procedure
again.

---

## 9. The refused alternatives

Four smaller changes were considered and each was refused for a stated reason.

**9.1 Traits and closed overloading alone, without a dependent core.** This is the cheapest option and it fixes item 6
of §2: `nat_add`/`ratio_add` collapse into `Add`, `text_equal` becomes `==`, and the builtin registry shrinks. It fixes
none of items 1–5. Quotation is what removes `callN` and the role integers, and quotation needs `Syntax<Cat>` to be
typed by its category, which needs an index. Doing traits now and indices later means elaborating traits twice — once
against the current checker and once against the one that replaces it — for a language that would still be unable to
write its own standard library in between.

**9.2 Quotation without a dependent core.** An untyped `Syntax` with a runtime category tag would remove `callN` and
derive provenance, which is most of the measured win. What it cannot do is fail at elaboration time: a quote spliced
where a pattern was expected becomes a phase-time error in a compiler pass, reported against generated syntax rather
than against the line the author wrote. Prompt 132 tests this by writing the staff dispatch table both ways; if the
untyped version reads as well, that is a finding and the index is dropped.

**9.3 Dropping totality.** Refused; §8 is the argument. Note 39 §11.1 already wrote down what dropping it would require,
and the requirement it could not meet is the same one now: two concrete operations that need possible divergence, with
their smallest failing total terms. None exist.

**9.4 Call-by-push-value.** Refused; §7.2 is the argument. §11.2 raises it as an available choice, and taking an
available choice with no term on either side of the distinction is how a core stops being small.

---

## 10. What this costs, stated now so it can be checked later

Twenty-two prompts, a new crate, a replaced type checker, and one migration of `stdlib/`, `examples/`, and every fixture
corpus. Three gates decide whether it was worth it, and all three are measurements rather than judgements:

1. **The staff benchmark** ([145](../../../plan/prompts/145-staff-rewrite.md)). The rewrite is measured against 2,404
   lines / 93,252 bytes, with zero `callN` helpers, zero hand-allocated role integers, zero string dispatch on token
   kinds, and `Pending` as a record. Prompt 132 predicts the number before any code exists. If it does not move
   dramatically, this amendment was wrong and 145 is a repair of Phase A rather than an implementation.
2. **The performance gate** ([144](../../../plan/prompts/144-diagnostics-and-performance.md)). P1 and P2 against the
   recorded baselines in `docs/rules/language/06-performance.md`, under the existing 10% relative gate. A checker that
   normalizes during conversion is exactly where this regresses silently.
3. **The generality claim** ([146](../../../plan/prompts/146-studio-rewrite.md)). The studio adapter was chosen before
   any of these mechanisms existed. If it gains nothing, that is a real result about adapters and it is recorded as
   asymmetry rather than averaged away.

Prompt [149](../../../plan/prompts/149-language-pass-closure.md) writes the closing note that puts the four measurements
together, and it is the honest place to record anything that did not work: a mechanism nobody used, a prediction that
was wrong, or a cost higher than note 39 §11.2 estimated. This record is a decision taken on evidence, not a proof that
the decision was right; the proof is 145's number.
