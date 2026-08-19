---
id: 141u
slug: glued-evaluation
status: done
depends_on: [141n, 141t]
phase: 3
---

# A Definition Stays Folded Until Something Needs It Open

## Task

Prompt 141n gave the core a top-level definition scope, and prompt 142 pointed the standard library at it. Nothing holds
a definition folded, so every use of every library definition is that definition's normal form, rebuilt at the use site.
Measured on the corpus 142 is migrating:

| workload | reduction steps | peak nesting |
| --- | --- | --- |
| [`examples/staff-page.musa`](../../../examples/staff-page.musa), 77 lines | **1,605,182,361** | 483 |
| the largest single `staff_expansion_laws` region | 54,994,216 | 373 |
| every other example, `stdlib/`, every fixture | ≤ 262,488 | ≤ 141 |

The same adapter expanded the same regions for under 200,000 steps on the evaluator 142 replaces — that was
`Budget::LANGUAGE`'s limit and those tests were green. The adapter did not grow by four orders of magnitude; the
evaluator stopped sharing. Where the 1.6 billion goes says the same thing: `evaluation` 1,108,265,060, `field
projection` 272,441,477, `function application` 213,344,524, `builtin reduction` 11,023,338, `structural reduction`
107,962 — a normalizer unfolding everything it meets, against an adapter whose `Pending` is an eight-field product read
field by field.

[Note 44](../../notes/research/language-design-closure/44-audit-against-smalltt-and-peyton-jones.md) §6 predicted this
exactly — "It becomes wrong at **prompt 142**, which points the whole standard library at this core" — and deferred the
fix to prompt 144 because until 142 there was nothing foldable. There is now. Build Finding C: a definition scope whose
uses stay folded, `Head::Def` as the head that carries both forms, and conversion that tries the folded comparison
first.

**What the same measurement says this prompt is _not_ the lever for**, taken by tallying `eval` entries by term shape on
the same run:

| shape | entries |
| --- | --- |
| record, projection, `let`, `J` | 383,002,707 |
| `Var` | 291,535,753 |
| `App` | 196,516,743 |
| `Lam` | 87,772,277 |
| `Const` | 51,676,883 |
| `Def` | **18,922,391** |

`Shape::Def` already hands back a value computed once at the declaration — `eval`'s arm clones an `Arc`-backed value
rather than re-evaluating one — so the 1.6 billion is not one definition rebuilt at many use sites. It is **18.9 million
distinct calls**, each costing some eighty-five evaluation entries of β through the library. Nineteen million calls to
read a 77-line page is the adapter's own reading algorithm, and
[`staff.musa`](../../../stdlib/src/adapters/staff.musa)'s header says why in its own words: a list cannot be built, so
the reading runs backwards. Prompt 141 closed that language gap and **prompt 145 is the prompt that rewrites the adapter
on it**. This prompt does not make nineteen million calls cheaper by two orders of magnitude and must not claim to.

## Read

- [Note 44](../../notes/research/language-design-closure/44-audit-against-smalltt-and-peyton-jones.md) **§6 in full** —
  Finding C, its three-part correct design, and its statement that Findings D and E wait on it. Also §7's and §8's
  closing paragraphs, which name the remainders this prompt does _not_ take.
- [Prompt 144](144-diagnostics-and-performance.md), **Design**, "The six items prompt 136b left here" — the list this
  prompt takes the first two entries from, and the reason each was waiting. 144 keeps the other four.
- `~/Code/smalltt`'s README on **glued evaluation** and on the three quotation modes. The `G` pair, the flexible-rigid
  head, and the speculative conversion are that implementation's, stated by it; take the design and not the Haskell.
- [`02-core-calculus.md`](../../rules/language/02-core-calculus.md) §3 — definitional equality by NbE and its
  decidability obligation, which is what fixes the _order_ of unfolding below rather than leaving it to whichever branch
  ran first — and §4, the meter this prompt is measured on and does not move.
- [Prompt 141n](141n-top-level-program.md) — the top-level program, which is what made a definition scope exist and so
  made Finding C real rather than hypothetical.
- [`context.rs`](../../../crates/musa-core/src/context.rs)'s `Cx::defined`, `Cx::env`, and `Cx::assumed`, and
  [`scope.rs`](../../../crates/musa-core/src/scope.rs)'s `define` — where a definition's _evaluated_ value is pushed
  into `Env = List<Value>` today, and why a use of it is already its normal form before anything asks.
- [`value.rs`](../../../crates/musa-core/src/value.rs)'s `Head` and `Neutral` — head plus `Vec<Elim>` since prompt
  136b's Finding F, which is what makes adding one head a variant rather than a restructuring.
- [`eval.rs`](../../../crates/musa-core/src/eval.rs)'s `apply`, [`quote.rs`](../../../crates/musa-core/src/quote.rs),
  and [`unify.rs`](../../../crates/musa-core/src/unify.rs) — the three places that ask "is this canonical yet", which
  are the three places that will have to force δ.
- Peyton Jones **ch. 12 §12.1** — tree reduction against graph reduction, which is this same observation one machine
  down: an expression evaluated once and shared costs a sum where an expression rebuilt at each use costs a product.
  Take the analysis; musa has no graph reducer and this prompt does not build one.

## Design

### What is folded today: nothing, and the reason is structural

`Cx::defined(ty, value)` pushes the definition's _already-evaluated_ value into `Env`, and `eval` of `Term::Var(i)`
returns `env[i]`. A definition is therefore indistinguishable from a value that happened to be written out in full at
every one of its uses. `staff.musa` is some two hundred definitions calling one another; each call site rebuilds its
callee, and the callee's callees, and the cost is the product where it should be the sum.

### `Head::Def`: one new head, flexible-rigid

```rust
/// A definition held folded: what it is known as, its type, and its unfolded value.
Def(DefHead, Arc<Value>, Arc<Value>),
```

*Repaired against the code, which falsified the sketched `Def(DbLevel, Arc<Value>)` in three places.*

- **The identity is one enum with two disjoint constructors**, not a bare `DbLevel`. A `let` or context definition is
  its binder's level — a definition is already a telescope entry with a stable level, so this needs no new table and no
  interning. A top-level definition is the `Def` itself, whose name is its equality. The sketch's single `DbLevel` does
  not survive contact: the diagnostic law below needs _program_ definitions folded (a `let`'s name never reaches the
  value domain, so "prints the names" is only testable on top-level ones), and a program position and a binder level are
  two numberings that both start at zero — one program definition plus one `let` is a constructible wrong-`true`
  conversion if they share a constructor. Two constructors of one enum cannot disagree with each other; two uses of one
  numbering could.
- **The type travels beside the unfolded value.** `neutral_type` and `head_type` answer a head's type with no context to
  ask, which is why `Head::Var` carries one; a folded definition is the same question. The unfolded value travels for
  the sketch's reason: δ stays a _local_ rule — `eval` takes `&Env` and not a `Cx`, and a head that had to consult a
  context to unfold would make δ a lookup the evaluator cannot perform where it needs to.
- **Flexible-rigid.** It never becomes a metavariable, so unification treats it as rigid; it computes on demand, so
  conversion treats it as reducible. That pairing is why it is one head rather than a second `Form`.

### δ becomes demand-driven, and the demand has exactly four sources

`eval` at a definition builds the folded neutral in constant time: `Shape::Def` from the declaration it names, and
`Cx::defined` from the binder's level. Applying, projecting, or `J`-eliminating a `Def`-headed neutral extends the spine
and stays folded. Unfolding is forced at five kinds of site and nowhere else, and each of them is a place that already
asks whether it is looking at a canonical form:

1. **conversion**, when the folded comparison disagrees;
2. **ι**, when a recursor's target must become a constructor — and `stepped`, when a step constructor's argument must
   become a numeral;
3. **δ-builtin reduction**, when an argument must become a literal;
4. **quotation in the opening mode**, below;
5. **the elaborator's existing force-before-match sites**, which already ask exactly this of metavariables.

The fifth kind is a repair, not an extension: the sketch listed four, and the fourth's absence from the elaborator moves
acceptance, which this prompt's own Target forbids. A `let T = … in` whose uses stay folded makes every
`force`-then-match in `elab.rs`, `case.rs`, `dictionary.rs`, and `recheck.rs` read a type synonym as `Neutral` rather
than as the `Π` it names, and a program that applies a function through one is refused where it was accepted. Those
sites already call `force` before matching because a solved metavariable hides the same forms; they now see through a
folded definition at the same moment, through the same shared operation.

So the change is one `unfold` — a `Def` head replaced by its carried value with the spine replayed over it — and one
fixed point of `force`-then-`unfold` built on it, rather than a new pass over anything.

**The replay carries no bookkeeping charge.** The sketch charged the unfold a step and let the replay charge each
elimination again, and the corpus answered: `neo-riemannian.musa`, green at the base at 200,000 steps, exhausted at
200,001 — the spine-building `apply` had already charged every elimination, so charging the replay counts every
application of a definition twice. Neither the unfold nor its replay charges; the elimination was charged when it
entered the spine, and the replay's real work — a β body, an ι step, a builtin's rule — carries its own charges either
way. Termination needs no meter on the chain: a definition's value names only what was declared before it (a local's
carried heads sit at strictly smaller levels, a global's at earlier declarations, a recursive definition's
self-reference under a λ), so the chain is finite by the ordering argument below. Note the asymmetry this sets: the
*metavariable* replay in `force` keeps its charges, because a spine behind a metavariable waited behind something and
its replay is where the waiting work runs; a spine behind a definition waited behind nothing. That distinction is what
keeps the calibrated budgets exact.

**A definition's body is still paid for at its declaration.** δ deferred is δ *lazy*: with the use staying folded,
`let values = range(500)` never runs the recursion when nothing reads `values`, and the corpus answered that too —
`an_aggregate_past_the_budget_is_refused_and_publishes_nothing` lost its refusal. The budget laws were calibrated
against evaluation that ran a definition's body when the declaration was elaborated, and "acceptance does not move"
covers the laws as much as the programs. So the boundary keeps the old strictness: `Cx::defined` for a `let` and
`program::elaborate` for a top-level definition each open the body's value to weak-head form — one `opened` at the
definition, before the folded reference is what the environment stores — while every *use* stays folded. The sharing and
the conversion win are untouched: the body is still evaluated once, and a use still compares by name. What is genuinely
lazier is a definition's use inside another value — an application behind a constructor — which runs at the first unfold
site that reaches it; that is Finding C working as designed, and no law in the corpus is calibrated against it.

The unfold order is fixed by **the one that can mention the other goes first**: a local before a global (a `let`'s value
may name a program definition and a program definition is closed to locals), the larger level before the smaller, and
between two globals the lexicographically larger name — arbitrary, and fixed, which is all §3 asks: a conversion whose
answer depended on which branch ran first would not be a decision procedure. Unfold chains terminate because a local's
stored value was evaluated before its binder was pushed, so the `Def` heads inside it carry strictly smaller levels, and
a global's carry only earlier definitions — a recursive global's self-reference stands under a λ the recursor plan
built, never at the head.

### Conversion tries folded first, and unfolds in a fixed order

Two `Def` heads with the same level and convertible spines answer `true` without unfolding either side. That is the
whole of the win on the conversion path and it is smalltt's speculation.

On disagreement, **one** side unfolds and the comparison is retried, and the side is chosen by the order the section
above fixes — the one that can mention the other first: a local before a global, then the larger level, then the larger
name. Fixing the order matters for §3 rather than for speed: a conversion whose answer depended on which branch ran
first would not be the decision procedure §3 requires. Where the identities are equal the heads are the same definition,
and a spine disagreement there unfolds either side, since both unfold to the same value.

### Quotation gains two modes, and they are two because there are two callers

`quote` takes a mode with exactly two values:

- **`Keep`** — stop at a `Def` head and write the definition's name. The diagnostic path takes this. Where that shows up
  is worth stating precisely, because the sketch's "a mismatch names `pitch_of`" is true one level *around* the mismatch
  pair rather than in it: the pair itself is found after unfolding (a folded disagreement is retried open, for
  completeness), but everything quoted *about* the pair — the type a refusal reports, the spine failure a same-head
  retry also fails with, the readbacks the elaborator hands on — keeps the author's names. Two rules make that concrete:
  a refusal that would quote a type it just opened quotes the value *before* opening instead, so applying a value of an
  aliased type is refused at the alias's name; and when a same-head folded comparison fails and the unfolded retry also
  fails, the folded failure is the one reported — the unfolded comparison decides the question, and the folded one says
  it better. That is half of what [144](144-diagnostics-and-performance.md)'s conversion-error work asks for, arriving
  as a consequence rather than as work.
- **`Open`** — unfold `Def` heads. Metavariable solutions take this, because a solution mentioning a definition that
  escapes its scope is unsound; so does the canonical readback in
  [`lower/values.rs`](../../../crates/musa-compiler/src/lower/values.rs), because a musical value must be a value and
  not a name for one.

Two callers, two modes, and no third. Keeping folded heads in _meta solutions_ is smalltt's `flexQuote` and is Finding
E's remainder; it stays at 144.

### What this prompt is not

It does not make conversion approximate (Finding D's remainder), does not add the per-metavariable occurs cache (Finding
E's remainder), does not touch the notation fold's quadratic follow spine, and **does not move one budget constant**.
Those are 144's, and the cost table is the prompt after this one — which is the point of the ordering: a table set
against a checker that rebuilds the standard library at every call site would be a number measured on a defect.

### The gate

The gate is on the path this prompt actually changes, and it is stated as a property rather than as a number the corpus
happens to produce, because the corpus's number is the adapter's and belongs to 145.

**Conversion.** Two uses of the same definition at the same arguments are convertible in a spend that does not grow with
the size of the definition's normal form, stated over a definition whose normal form is deliberately large so that a
regression fails rather than merely slows.

**Sharing.** A term using one definition _n_ times costs its evaluation once, within a constant.

**The corpus, recorded rather than gated.** `examples/staff-page.musa`'s spend is re-measured and written into
`budget.rs`'s doc comment beside `Budget::LANGUAGE` with the command that produced it, in the shape
[`core_budget.rs`](../../../crates/musa-compiler/src/core_budget.rs)'s `FRAME_CEILING` already uses — against
1,605,182,361 before. Whatever it becomes is the number the cost-table prompt and prompt 145 both argue from. No
prediction is offered, because the tally above says the residual is the adapter's call count and this prompt does not
change it.

## Target

- `Head::Def(DefHead, Arc<Value>, Arc<Value>)` — identity, type, unfolded value — with `Cx::defined` and `eval`'s
  `Shape::Def` arm building the folded neutral and `Cx::assumed` unchanged.
- δ forced at exactly the five kinds of site named above, each through one shared `unfold` rather than five spellings of
  it.
- Conversion answering same-level `Def` heads with convertible spines without unfolding, and unfolding the larger level
  first on disagreement.
- `quote`'s two modes, with the diagnostic path on `Keep` and metavariable solutions and the canonical readback on
  `Open`.
- **The sharing law**: a definition used _n_ times costs its own evaluation once, stated as a test that checks a term
  using one definition ten times and asserts the spend is within a constant of using it once — the property, not a
  recorded number.
- **The folded-comparison law**: two uses of the same definition at the same arguments are convertible with a spend that
  does not depend on the size of the definition's normal form, stated over a definition whose normal form is
  deliberately large.
- **The unchanged-acceptance law**: the corpus that checks before checks after, to the same values and the same normal
  forms. This is a representation change; if it moves acceptance, the design is wrong.
- **The diagnostic law**: a refusal quotes the type the author wrote — a definition's name, not the unfolding the
  elaborator matched against — tested at a `NotAFunction` refusal through a type alias, plus the folded-failure rule for
  same-head comparisons above.
- `convertible_metered`, the `Spend`-reading twin the folded-comparison law is stated through, in the pattern of the
  other `*_metered` facades.
- The re-measurement of `examples/staff-page.musa`, recorded in `budget.rs` with its command, and the residual against
  `Budget::LANGUAGE` stated plainly whichever way it falls — including, if it is still far past it, that the remainder
  is prompt 145's.
- A closing line in [note 44](../../notes/research/language-design-closure/44-audit-against-smalltt-and-peyton-jones.md)
  §6 for Finding C and for `Head::Def`, so the audit's first two remaining items end here rather than being inherited
  again.
- `docs/plan/code-map/` rows for the changed files.

## Check

```sh
cargo build --workspace
cargo nextest run -p musa-core
cargo nextest run -p musa-compiler -E 'test(staff_expansion_laws) or test(staff_writing_laws) or test(conversion_laws)'
cargo clippy --all-targets -p musa-core -- -D warnings
cargo fmt --check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
python3 scripts/renumber-prompts.py audit
```

One truth-table line moves with this prompt and is not a stop: `every_example_elaborates`'s expected budget walls for
the staff page drop from two to one, because the wall the page hits first is a different one when δ is lazy. The example
is still refused at the same budget; prompt 145's rewrite still owns the class. `musa-compiler`'s full suite and its
clippy run are not gates here, for the reason [141s](141s-numeral-representation.md)'s and
[141t](141t-nested-occurrences.md)'s **Check** sections state and measure: this prompt lands inside 142's migration,
that suite is red for migration reasons this prompt neither causes nor can fix, and `-D warnings` is red on `core.rs`'s
superseded checking paths that 142's own Target deletes. The staff tests are named because they are what this prompt is
_for_; the whole corpus answers at 142's Check.

Commit as `A definition stays folded until something needs it open`.

## Stop

- **No budget constant moves.** Not the step limit, not the nesting limit, not the frame ceiling, not `CostTable`. This
  prompt produces the number; the next one sets the table.
- **No approximate conversion, no flexible quotation for meta solutions, no per-metavariable occurs cache.** Note 44's
  Findings D and E remainders stay at 144, where they are measured against a checker this prompt has already made worth
  measuring.
- **No second `Value` threaded through evaluation.** If the single folded value proves insufficient and smalltt's full
  `G` pair is genuinely needed, that is evidence and a repair, not a quiet widening of `Value`.
- **No change to what is accepted or refused**, and none to any normal form or rendered output.
- **No corpus change.** Not one `.musa` file, not one snapshot. If a fixture has to move, it is 142's.
- **No adapter rewrite.** `staff.musa` is prompt 145's, and this prompt exists so that 145 is measured against a checker
  rather than against a normalizer.
- **No work on the notation fold's follow spine.** It is quadratic, it is recorded, and it is 144's.
