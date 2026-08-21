# Simplifying the core and the elaborator: a removal inventory

**Status: records a design direction; governs nothing.** Written on the user's directive: simplicity and performance
first; prefer removal from the core over addition unless an improvement bar is met; dependent types stay — for terseness
and what it buys, not proof-assistant ambitions; the macro system (quotation and adapters) is the center of gravity, in
a lisp-like role without the parens. This note inventories what the core and elaborator carry, names what each piece
earns, and ranks the removals. Where a removal has no owner in the prompt stack, it says so; where one needs an
amendment, it says that instead of doing it.

## What the references say

**Peyton Jones.** The whole book is the argument that a compiler is a *small core* plus a *translation*: chapter 3
enriches the lambda calculus and then translates every enrichment away, so the evaluator below never grows. Two
consequences for us, both already half-heeded. First, the complexity budget belongs in the translation (`lower/`,
`elab.rs`), never in the term language — every former added to `Term` is paid by evaluation, unification, conversion,
quotation, and every pass that walks terms. Second, his evaluator is exactly as simple as the terms it runs: template
instantiation works because a supercombinator body is inert data. Our `Term::Def` unfolding with the meter beside it is
that machine; anything that makes evaluation cleverer than the terms is a sign the terms are wrong.

**Idris2.** The division is `TTImp` (surface) against `TT` (a core of fifteen constructors), with **one elaborator
between them and one evaluator below**. Fresh names live in `UST.nextName` inside the `Core` monad — elaboration-time
state; `Core.Normalise` mints nothing. Idris2's primitives are registered functions with evaluators — the same shape as
our δ rules, and the same discipline: the core's type for a rule forbids captured state, so primitives stay honest
functions of their arguments. The lesson Idris2 carries for a small team is not any feature; it is that there is **one**
of each stage. Every place we run two — two checkers, two unifiers, two evaluators for the same operation — is not
redundancy, it is a second language to keep honest.

**Open Music Theory.** The feature floor, and the standing falsifier for removals: a simplification that makes a musical
distinction unsayable is not a simplification. Its concrete contributions are already load-bearing in the spec — written
pitch versus sounding pitch (`016-intervals.md`, via `02-core-calculus.md` §1.3's refusal of a bare `Int`), meter as
felt grouping rather than time signature decoration, the tie as a relation between two noteheads. Nothing in this note
proposes touching those; they are what the language is *for*.

## The measured inventory

Numbers at `a8bb8ac`, `wc -l` on `src/`, comments included — size is being used as a proxy for concept count, and both
are what a new reader pays:

| Body | Lines | What it is |
| --- | ---: | --- |
| `musa-calculus` total | 20,869 | the core calculus: `elab` 2,666 · `family` 1,820 · `case` 1,396 · `dictionary` 1,253 · `base` 1,136 · `raw` 1,068 · `eval` 985 · `refuse` 951 · `unify` 935 · `program` 826 · `term` 729 · `declare` 704 · `rec` 647 · `quote` 579 |
| `musa-compiler` total | 50,364 | of which **`core.rs` alone: 15,413** — the old checker, still live for one caller |
| the new path | 13,775 | `lower/` 8,262 + `registry/` 5,513 — the translation and the δ rules |
| the `Term` language | 18 formers | `Var`, `Const`, `Def`, `Numeral`, `Base`, `Lit`, `Builtin`, `Universe`, `Pi`, `Lam`, `App`, `RecordType/Record/Project`, `Id/Refl/J`, `Meta`, `Let` |

The term language is already close to the floor the references point at. `Id/Refl/J` is the constitution's typed
derivations, and K is *not* admitted as an axiom (§1.4's decision, settled by the trial: no program unifies an index).
`Meta` is what bidirectional elaboration with real inference costs; `Numeral` is a representation, not a feature. There
is no former here that fails the musical falsifier or the terseness test, and **this note proposes removing none of
them**. The sprawl is not in the calculus. It is in the compiler running two of everything.

## The removals, ranked

**R1 — The old checker, which was supposed to be gone.** The code-map says of the registry row "prompt 142 … deletes the
old checker and evaluator," and 142's own Target names "`check_material` calling 141g's and 141k's lowering and
`musa_calculus::check`, with the old checking path deleted." Neither happened for the library path: `elaborate_material`
(elaborate.rs:1099) still calls `core::check_material` → `check_and_evaluate`, and `core.rs` still carries `Type` (line
318), `ExprKind` (697), `Value` (3311), `Checker` (5428), and the whole checking pass — an estimated 10–11K lines of the
file's 15.4K (estimate: the `Type`→`Checker` span plus the check/eval machinery after it, minus the ownership tables and
adapter drivers that stay; to be measured properly at removal). One caller keeps it alive: standalone `library { … }`
documents. Every other path — pieces, the registry's own stdlib construction, adapter modules — already elaborates
through musa-calculus. This is the single largest simplification available in the repository: one checker, one unifier,
one evaluator, the Idris2 shape. **It has no owner prompt.** Proposal: a new prompt between 145 and 143, scoped to
moving `elaborate_material` onto the document path and deleting what dies — the library document is already a material
reader over a resolved module tree, so the port is a lowering plus a call, and the staff-class suite is the regression
gate that says the new reading agrees with the old. Whether 142's Target sentence is repaired to record the remainder,
or the removal lands as its belated delivery, is the same work either way.

**R2 — The phase-operation retirements already specified.** `11-quotation.md` §5 retires seven of fourteen phase
operations; the macro surface that remains is `recurse_syntax`, `run_syntax_step`, `syntax_fold_from_leaves`,
`syntax_at`, `syntax_anchor`, `syntax_number`, `as_expression` — read, fold, locate, number, and the checked parse: the
lisp reader-macro set with types. 145 takes the anchor's third argument (note 48). The four construction builders still
have one caller, `stdlib/src/adapters/doubled.musa`, the flat fixture; the binding three have none. Each deletion lands
when its last caller moves — the stack's own rule — so the inventory item is: migrate or retire `doubled.musa` (147 owns
the adapter freeze), then let 139's deletions fire. Every retired operation is a registry row, a rule, two evaluator
arms, and a doc comment that stop existing.

**R3 — The builtin-table collapse (prompt 143, already planned).** 92 δ entries plus 14 builders behind the trait
vocabulary — `Add`, `Eq`, `Iterable` — so `nat_add`/`ratio_add` is one concept. This is the user's directive in its
purest form: dependent types paying for *terseness*, and the table shrinking rather than growing. It also deletes the
second unification surface the traits currently re-derive. Owned; nothing to add.

**R4 — `Nat`'s unary spine, behind a measurement.** `prelude.rs` declares `Nat` as `zero | succ` and `rules.rs`'s own
module doc names the cost: "`nat_add` writes its answer one `Succ` at a time, and a large sum is a large term." The core
already has the escape hatch — `Term::Numeral` is a count standing for the tower, definitionally equal to it
(`family.rs`'s counting families). The removal-shaped fix is to make the *prelude's* `Nat` the counting representation
rather than growing a second numeric type; §5.10 already argued numerals are a conservative extension, so the calculus
does not move. The bar, stated in advance per the directive: the staff-page step count after the 145 rewrite, measured
against the 200,000-step budget; if `Nat` arithmetic is not in the hot profile there, R4 waits for a profile that shows
it. 144's cost table owns the verdict.

**R5 — The anti-list: what simplification must not touch.** `Id/Refl/J` (typed derivations are the constitution, §7);
`Meta` and pattern unification (inference is the terseness); records (the trial's `Pending` gate); quote patterns (the
reading half of the macro system); the meter and the termination measure (totality is the language's one promise); the
refusal of `Int` (OMT's written/sounding distinction, §1.3). A proposal that removes from this list is an amendment, and
this note is not one.

## The bar for anything new

Stated once, so future prompts can cite it. An addition to the core or the elaborator must clear all four:

1. **A named musical case needs it** — one of the five the closure was judged on, or the macro system itself.
2. **No existing mechanism already says it** — the second-path audit 142 ran is the standing rule, applied at design
   time rather than after the fact.
3. **It deletes at least as many concepts as it adds** — the trial's gate-table shape: every row has a "now" and a
   "required", and additions without a deletion column do not land.
4. **A measurement, if it touches evaluation or representation** — workload, baseline, delta, per the performance
   workflow; intuition is not a measurement.

The macro system is the standing exception that proves the rule: quotation, splicing, adapters, and the anchor's
provenance are *why* the rest can be small — user syntax absorbs pressure that would otherwise become language features.
Keeping it central is the simplification strategy, not a competing one.

## What this note does not do

It does not amend `02-core-calculus.md` (R4's representation question would, if the profile calls for it — the amendment
belongs to the prompt that lands it, with the measurement attached). It does not reorder the stack: R1's owner prompt is
a proposal for the user to ratify, since inserting a prompt between 145 and 143 is a plan change. And it does not touch
the audio half, which has its own closure (prompts 16x–17x) and its own performance law — the real-time callback's rules
are already stricter than anything here.
