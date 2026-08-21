# 50. The course correction: audit and plan

A decision record. Governs nothing; records why the language implementation is about to become much smaller.

## The directive

The implementation of prompts 128–166 sprawled past the language Musa needs. The correction, in the owner's words:

> the smallest practical language that gives Musa excellent metaprogramming, ordinary ergonomic programming, useful
> lightweight dependency, and exact resource checking — not the smallest proof assistant capable of expressing all of
> those features.

Macros are the primary extensibility mechanism. Dependency stays in the programming sense — a later type may mention an
earlier value, definitional equality may evaluate total pure terms — and must not import proof-assistant machinery. The
audit rule: **every mechanism names the committed Musa program that requires it.** "Standard in Lean/Agda" and "useful
later" are not evidence. This note is the audit and the plan; the implementation follows it phase by phase.

## What the corpus actually uses

Greps over `stdlib/`, `examples/`, and `tests/fixtures/` (every committed `.musa` file), taken at `8db2135`:

| Feature | Committed uses |
| --- | --- |
| `Id`, `refl`, `J` (propositional equality) | **0** |
| Universe ascriptions (`Type l`) in user code | **0** |
| `where` clauses | **0** (word occurs only in prose comments) |
| `implicit` binders | **0** (word occurs only in prose) |
| User-declared indexed or parameterized `data`/`enum` | **0** (`Option`/`List`/`Result` are prelude-owned) |
| User traits | **1**: `Transposable<A>` in `stdlib/src/pitch.musa`, with 2 concrete impls (`Pitch`, `NoteName`) |
| Self-recursive functions (brace-matched bodies, comments stripped) | **13**, all structural on `Nat`/`List`/`StaffItem`: `list.musa` 4, `notation/staff.musa` 7, `pitch.musa` 1, one example fold |
| Explicit termination measures | **0** — the 13 are all the structural case |
| Generated recursors | **0 by name** — `staff_item_fold` is written by hand in `stdlib/src/notation/staff.musa`; `match` (compiled through `case.rs`) is the only eliminator any program names |

The compiler's own trait corpus is exactly as flat: `Eq` has five concrete instances (`Text`, `Ratio`,
`Duration⟨written⟩`, `Pitch`, `Interval`); `Buildable`/`Iterable` have three concrete ones; every head is concrete; no
instance anywhere carries a `where` clause. `Storable` has no source-level use at all: its only demand sites are
compiler-owned machine-port signatures (`registry/machine.rs:646`).

`Syntax<C>` — the one indexed-looking type the macro system needs — is already **not** a family: it is a parameterized
base type in `base.rs:99`, indexed by compiler-owned category literals. The general indexed-family machine in
`family.rs` backs nothing.

The stated justification of the pass being corrected: prompt 128 amended the constitution on the evidence of
`stdlib/src/adapters/staff.musa`, and prompt 129 then specified "universes, Π, dependent records, inductive families, an
identity type, … metavariables, and a checked well-founded termination rule" — the standard checklist, not the least
machinery the evidence demanded. The staff adapter needs records, enums, match, structural recursion, first-order
generics, flat traits, and quotation. It needs nothing else. That gap is what this correction closes.

## The audit table

musa-calculus is 20,869 lines / 928 KB / 30 files. The language side of musa-compiler adds `core.rs` 15,422 (the old
checker, still the live path for `library { }` via `elaborate.rs:1099`), `elaborate.rs` 1,557, `infer.rs` 822, `lower/`
8,262, `registry/` 5,513, `expand.rs` 2,485, `syntax.rs` 2,076.

| Mechanism | Files (LOC) | Program requiring it | Verdict | Replacement |
| --- | --- | --- | --- | --- |
| `Id`/`refl`/`J`/`K` | formers in `term.rs`; rules in `eval.rs`, `quote.rs`, `elab.rs`, `case.rs`, `refuse.rs` (≈600 spread) | none | **delete** | definitional conversion; `==` for programs |
| Universe levels + level metas | `level.rs` 556 + arms across `elab`/`eval`/`term`/`declare`/`family` (≈300) | none | **delete** | fixed two-step stratification (`Type 0`, `Type 1`); no level arithmetic |
| General term metas | `meta.rs` 225 + plumbing in `elab`/`unify` | omitted type arguments at polymorphic calls (`map`, folds, phase traversals) | **shrink** | first-order holes solved once, locally, by matching explicit argument types; unsolved is an error naming the parameter |
| Pattern unification + postponement | `unify.rs` 935 | same calls | **replace** | first-order matcher + conversion (≈200 lines) |
| Postponed constraint machinery | `elab.rs` (`postponed` list, retry loop), `dictionary.rs`, `unify.rs` | none beyond the above | **delete** | immediate error |
| General indexed inductive families | `family.rs` 1,820 + parts of `declare.rs`, `case.rs` | none | **delete indices** | plain parameterized enums: parameters fixed per declaration, no indices |
| Dependent motive inference | `case.rs` (part) | none — every match returns an ordinary type | **delete** | `match` checks against the expected type |
| Generated dependent recursors | `family.rs` (part) | none by name | **shrink** | internal non-dependent eliminators for plain enums, existing only to compile `match` |
| Indexed strict positivity | `declare.rs` (part) | plain enums still need a negativity check | **shrink** | the plain-enum check |
| General termination measures | `rec.rs` 647 | the 13 structural functions | **shrink** | structural recursion only: a recursive call's argument must be a pattern-bound subterm |
| `recheck.rs` independent checker | 501 | test suites only | **delete** | the suites keep their eval/conversion laws |
| Traits as telescoped classes + constraint contexts + super-constraints | `class.rs` 522, `dictionary.rs` 1,253 | the flat corpus above | **replace** | trait = record type, impl = record value in a `(trait, head)` table; resolution below |
| `Storable` as trait | `storable.rs` 500 + arms in `class`/`context`/`dictionary`/`elab` | machine-port signatures (compiler-owned) | **replace** | one structural Rust predicate evaluated at machine registration |
| Implicit argument insertion | `elab.rs` insertion sites, `raw.rs` plicity | the prelude's own `{Context}` binders on phase functions | **delete** | explicit type parameters, omitted at call sites under the one first-order rule |
| Glued evaluation (141u) | `eval.rs` (part) | piece-compile performance, measured | **keep** | — |
| NbE over the small core | `eval.rs` 985, `value.rs` 306, `quote.rs` 579 | conversion for dependent signatures (`Duration⟨C⟩`, `Syntax⟨C⟩`) | **keep**, shrinks as formers die | — |
| Base types + builtins | `base.rs` 1,136 | everything | **keep** | — |
| Exact budget metering | `budget.rs` 421 | the constitution | **keep** | — |
| Case-tree compilation | `case.rs` 1,396 | `match` everywhere | **keep**, minus motive inference | — |
| Declaration graph | `program.rs` 826 | top-level order, recursion admission | **keep** | — |
| `raw.rs` | 1,068 | elaborator input | **shrink**: plicity and `Id` forms die | — |
| Context/scope/refuse/show/error/list/origin/room/visibility/lib | ≈3,900 | infrastructure | **keep**, pruned | — |

And the compiler side:

| Mechanism | Files (LOC) | Verdict |
| --- | --- | --- |
| The old checker/evaluator, live for `library { }` | `core.rs` 15,422, `elaborate.rs` 1,557, `infer.rs` 822 | **delete** (note 49's R1, subsumed): adapter modules check and evaluate in the one surviving core |
| The phase-op registry machine | `registry/` 5,513 | **keep**, retargeted: rules answer over the surviving core's values instead of old-path `Datum`s |
| Expansion boundary, syntax values, anchors | `expand.rs`, `syntax.rs` | **keep** |
| `lower/` (CST → Raw) | 8,262 | **keep**, shrinks as `Raw` shrinks |

Expected net: musa-calculus 20,869 → ≈11–12K lines; the compiler loses ≈18K more. Total deletion ≈25,000 lines.

## The surviving language

**Type checking, complete algorithm.** Bidirectional and explicit. Introduction forms (λ, record literals, `quote`,
`let`, match arms) check against an expected type; elimination forms (application spines, projections, qualified names)
infer. The switch rule is the only conversion site. Conversion is α-equality of η-long normal forms computed by NbE over
the small core (β, δ, ι for plain enums, η at Π and records). A type parameter omitted at a call site is solved by
first-order matching of the explicit arguments' inferred types against the parameter types — one pass, in order, nothing
postponed; a parameter that matching does not determine is an error at the call naming the parameter, and the author
writes the argument. Two universe levels, fixed, no metavariables of level sort. Totality: structural recursion is the
only user recursion — a self-call's recursive argument must be a pattern-bound subterm, checked at the `match` that
binds it; no measures, no `partial`; the budget remains the constitution's backstop.

**Trait resolution, complete algorithm.** A trait elaborates to a record type; an impl to a record value filed under
`(trait, head constructor)`. Operators lower to qualified calls (`x == y` → `Eq.equal(x, y)`). To resolve `C<τ>` at a
use site: (1) if `τ` is a bound type variable and an enclosing `where C<τ>` dictionary is in scope, project the method
from that dictionary; (2) else if `τ`'s head is a concrete constructor, look up `(C, head)` in the program's instance
table — found is done, absent is an error naming trait and type; (3) anything else is an error. No recursion, because no
instance carries constraints; no postponement, because generic code reads only local dictionaries.

**Macros.** Unchanged in surface, cheaper underneath: hygienic `quote at p { … }` with `$x` / `${ e }` / `$..xs`
splices, the category certificate checked at splice boundaries, automatic provenance (`Derived`), anchors, and the
traversal operations — all as builtins over a compiler-owned parameterized base type `Syntax<C>`. No indexed family is
consumed by any of it, and macros expand before ordinary type checking and never participate in resolution or
unification.

## The plan

Each phase is one commit with the workspace gates green (the carved red classes of prompt 142 travel with us unchanged;
they die in phase 5, not before).

0. **This note.** Supersede prompts 164–170 on the record: 143 (builtin collapse) is subsumed by phase 3, 144's budgets
   are re-derived after the excision, 145's staff rewrite becomes phase 5's benchmark, 167–170 close a pass this
   correction replaces.
1. **Documents first** (the repo's repair discipline): rewrite `02-core-calculus.md` as the surviving calculus; rewrite
   `10-traits.md` as the flat model; amend `01-surface.md` §1.4 and `11-quotation.md` §1 (which already describes a base
   type in everything but name).
2. **musa-calculus excision.** Delete `Id`/`refl`/`J`, `level.rs`, `recheck.rs`, `storable.rs`; replace `unify.rs` with
   first-order matching; replace `dictionary.rs`/`class.rs` with the flat table; strip plicity, indices, motive
   inference, and measures from `raw`/`elab`/`family`/`declare`/`case`/`rec`. Prune the suites that tested deleted
   machinery.
3. **One checker.** `library { }` documents move to the surviving core; `core.rs`, `elaborate.rs`, `infer.rs` die; the
   registry machine's rules retarget onto the core's values. This is the largest single move and is sequenced after the
   core has settled so it ports into a stable target.
4. **stdlib and fixtures migrate.** Expected to be small by construction: no committed program uses a deleted feature.
   The prelude's implicit-binder spellings become ordinary type parameters.
5. **The staff rewrite, as the benchmark.** `stdlib/src/adapters/staff.musa` rewritten for clarity and size on the
   simplified language (145's five eliminations carried over), with the budget and step numbers re-derived and recorded.
   The carved staff class goes green here or its failure is a finding.
6. **Book, gates, report.** `docs/book` examples migrated, `make docs-check` green, and the final report: before/after
   LOC and file counts, deleted and surviving mechanisms, both algorithms in full, staff.musa size, staff-page.musa
   compile cost, and anything that could not be removed with the committed program that proves it necessary.

## Risks, named now

- **Phase 3's retarget** is the deepest cut: the registry's `Rule = fn(&[Datum]) -> Option<Answer>` becomes builtins
  over core values, and the four-arg traversal operations stay higher-order builtins. Mechanical, but wide.
- **Suite parity through the excision.** The red ledger (27 staff + 2 tonal + 3 pressure) must move byte-identically
  through phases 2–4; any new red or missing red is a finding, exactly the 142 discipline.
- **`case.rs` without `family.rs`'s indices** rewrites the middle of the match pipeline; the plain-enum eliminator it
  generates must still give `rec.rs`'s structural check its hypotheses.
- **The constitution** is not amended by this note: exact time, source authority, and the budget stand untouched. Prompt
  128's amendment is narrowed, not reversed — lightweight dependency survives — and this record says why, which is what
  `docs/rules/README.md`'s sixth requirement asks of an overturning argument.
