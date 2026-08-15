# Auditing the dependent core against smalltt and Peyton Jones

## Purpose

Phase B has built [`crates/musa-core`](../../../../crates/musa-core) across prompts 133–136: terms, values, NbE,
bidirectional elaboration with metavariables, inductive families, dependent `match`, and — uncommitted at the time of
writing — records and enums. Two references sit outside the repository that the design has been claiming kinship with:

- **`~/Code/smalltt`** — András Kovács' reference implementation of a small dependently typed language, whose README is
  an argument about _how to make elaboration fast_ rather than a specification of what to elaborate.
- **Peyton Jones, _The Implementation of Functional Programming Languages_** (1987) — chapters 3–6: the enriched lambda
  calculus, structured types, the semantics of pattern matching, and its efficient compilation. Several prompts already
  cite it by chapter, and [`case.rs`](../../../../crates/musa-core/src/case.rs)'s module header names ch. 5 as its
  source.

This note asks whether the implementation, and the prompts still to come, actually follow them. Every divergence is
classified: **backed** (the departure is argued, and the argument survives), or **ad hoc** (the departure was never
argued, or was argued against a different question than the one it answers). Each ad hoc divergence gets a stated
long-term design and a place in the prompt stack.

Nothing here governs. `docs/rules/` governs; this is evidence for a later repair.

## Verdict

**Seven divergences. Two are backed, five are ad hoc, and one of the five is measurable today.**

The measurable one is not a micro-optimization. `match` duplicates an arm's body once per case-tree leaf the arm
reaches, and elaboration re-typechecks each copy. Peyton Jones §5.4.1 names this exact failure — it is his `unwieldy`
example — and the fix he gives (the fat bar) is the one thing [`case.rs`](../../../../crates/musa-core/src/case.rs)'s
header explicitly declines. The decline is correct; the _replacement_ was never supplied. Measured below: elaborated
term size and elaboration time both grow by **2.2× per matched column**, reaching 14.9 MB and 45 ms at ten columns for a
program whose source is eleven lines.

The remaining four ad hoc divergences are all one design decision seen from four sides: **musa-core has no notion of a
top-level definition, so it has no unfolding control**, and every technique smalltt spends its README on — glued
evaluation, approximate conversion, the three quotation modes, approximate occurs checking — is a technique for deciding
_when not to unfold_. None of them can be implemented in a core that cannot fold anything. Prompt 142 hands that core
the standard library.

---

# Part 1 — Peyton Jones

## §1 What is followed, and correctly

[`case.rs`](../../../../crates/musa-core/src/case.rs) is a faithful ch. 5 pattern-matching compiler with two dependent
extensions, and both extensions are argued in its header rather than assumed.

| ch. 5 | musa-core | Status |
| --- | --- | --- |
| Variable rule | `solve`'s `None` arm and `leaf`'s `left` binder collection | followed |
| Constructor rule | `split` → `method` → `narrowed` | followed |
| Empty rule | the same `None` arm — the first row wins outright | followed |
| Mixture rule | `narrowed` expands a variable row into every constructor | **replaced**, argued |
| Fat bar / `FAIL` | not adopted | **declined**, argued |
| ch. 4's product types are irrefutable | `Test::Open`, which emits no term | followed |

Three of these deserve a note.

**The mixture rule's replacement is right.** ch. 5's mixture rule partitions equations into maximal all-variable and
all-constructor blocks and joins them with `[]`. That partition exists because `FAIL` exists. In a dependent core `FAIL`
is not merely unnecessary, it is **untypeable**: a term meaning "this branch did not match" would have to inhabit the
branch's refined goal type, and there is nothing to inhabit it with. Agda, Idris and Lean all reach the same conclusion
and all compile clauses to case trees without a failure continuation. Expanding a variable row into one row per
constructor is what remains, and it is what those implementations do.

**Opening a record is ch. 4's product rule, not ch. 5's constructor rule.** ch. 4 §4.3 separates sum types (which need a
case analysis) from product types (which need only selectors, and whose patterns are therefore irrefutable).
`Test::Open` emits no term at all: the column becomes one column per named field and the same matrix is solved again.
That is the correct reading, and — since prompt 136 makes records structural — it is also the only reading available,
because a structural record has no constructor to name.

**Opening only the _named_ fields is a real improvement on the book.** The code argues it: opening every declared field
would put a wildcard column in the matrix for each field the author declined to mention. ch. 5 has no equivalent because
its patterns are positional.

**`selects` matching a bare constructor name against the qualified one** is `01-surface.md` §1.3, and it is decided by
the column's type rather than by the spelling — the same discipline `parser.rs` keeps. Correct.

## §2 Finding A — right-hand sides are duplicated, and elaboration pays for each copy

**Ad hoc. Measured. The one finding with a cost today.**

`narrowed` copies each surviving row into every constructor branch, and `leaf` calls
`self.elaborator.check_open(&inner, &arm.body, goal)` on whichever row wins. An arm whose pattern in a split column is a
variable therefore survives into _every_ branch of that split, and its body is elaborated once per leaf it reaches — not
merely emitted twice, but **type-checked** twice, at a different (refined) goal each time.

Peyton Jones §5.4.1 opens on precisely this:

> If overlapping equations are allowed, then sometimes the pattern-matching compiler described above may transform a
> small set of equations into a case-expression that is much larger.

with the example

```
unwieldy [] [] = A
unwieldy xs ys = B xs ys
```

whose compilation contains `B xs ys` twice, and the observation that "only one rule can cause right-hand sides to be
duplicated, the constructor rule". His fix is to replace each duplicated `E` with `FAIL` and hang the single copy of `E`
off one `[]`.

[`case.rs`](../../../../crates/musa-core/src/case.rs)'s header declines the fat bar with this reason:

> The fat-bar exists so an equation can fail into the next one, and Musa's arms do not fall through

That sentence is true and it answers a different question. The fat bar has **two** jobs in ch. 5: it gives an equation a
way to fail into the next one (§5.2.5, the mixture rule), and it keeps a shared right-hand side from being copied
(§5.4.1). Musa needs only the first job removed. Declining the mechanism removed both jobs, and nothing replaced the
second.

ch. 5 §5.5 names the property that would make the second job unnecessary — a definition is **uniform** if it compiles
without the mixture rule at all — and proves that uniformity implies order-independence and non-overlap. Musa's `match`
is ordered and permits overlap by design, so it is not uniform, so the mixture rule is reached, so §5.4.1's duplication
is reached with it. There is no path where declining the fat bar and permitting overlapping arms both hold and nothing
is copied.

### This repo had already found it, twice, and chosen the answer

[`24-pipeline-and-syntax-review.md`](24-pipeline-and-syntax-review.md) §H5 is this finding, in 2024's design line, with
the same citation:

> Peyton Jones §5.4.1 shows the exact case. Compiling an exhaustive, ordered, overlapping definition by the constructor
> rule alone duplicates right-hand sides; the alternative is `[]` and `FAIL`. His `unwieldy` example is exhaustive and
> still needs one or the other.

It offered three repairs — (a) join points, (b) restrict source matches to uniform ones, (c) permit duplication and
bound it — and called (a) "the ordinary answer". [`26-language-design-decision.md`](26-language-design-decision.md) §2.4
then **took** (a), in writing, with a worked example, and said what it bought: "This preserves source order without
copying `other` or adding a runtime pattern-failure value."
[`29-source-and-expansion-spec.md`](29-source-and-expansion-spec.md) carried it forward as "local join points that share
ordered fallbacks".

Prompt 135 built the case compiler and took option (c) — permit duplication — without bounding it and without recording
the choice. The decision was not overturned; it was lost. Note 42 superseded that whole design line for a _different_
reason (the dependent core), and the join-point decision went with it unexamined even though nothing about a dependent
core argues against it.

That matters for the classification. This is not a cost the project accepted and can now weigh; it is a cost the project
twice decided not to pay, and then paid silently. And the answer §2 proposes below — a `let`-bound function per arm — is
note 26's join point, spelled in a calculus that has no labels: a join point that can only be entered by a tail jump
_is_ a let-bound function that is only ever applied in tail position, and in a dependent core it is the version that
typechecks.

### The measurement

A throwaway integration test elaborated this program at `Nat → … → Nat` for `k` from 1 to 10:

```
λ x₀ … x_{k-1}. match x₀, …, x_{k-1} {
  Zero, y, …, y  => 0,
  y, Zero, …, y  => 0,
  …                                  -- one arm per column
  y, …, y, Zero  => 0,
  z₀, …, z_{k-1} => 30,              -- the catch-all
}
```

Every arm is all-variables in every column but one, so every arm survives every split it does not itself decide. Size is
the character length of the elaborated `Term`'s `Debug` form — a proxy, but a monotone one; time is wall-clock for the
single `musa_core::check` call, debug build, warm.

| columns | elaborated term (chars) | elaboration (ms) | growth |
| ---: | ---: | ---: | ---: |
| 3 | 55,942 | 0.47 | — |
| 4 | 117,945 | 0.66 | 2.11× |
| 5 | 263,699 | 1.14 | 2.24× |
| 6 | 598,809 | 2.26 | 2.27× |
| 7 | 1,356,278 | 6.34 | 2.27× |
| 8 | 3,045,730 | 9.58 | 2.25× |
| 9 | 6,773,692 | 20.88 | 2.22× |
| 10 | 14,928,160 | 45.53 | 2.20× |

Exponential, base ≈ 2.2, in both size and time. Eleven lines of source produce fifteen megabytes of core term.

A second shape — one specific arm and one catch-all, `Zero…Zero` against `z…z` — grows **linearly** (20 KB → 143 KB from
1 to 6 columns). That is worth stating because it bounds the finding honestly: the blow-up needs several arms that each
constrain a _different_ column, which is what makes each split leave every other arm alive. A dispatch table on one
column — `staff.musa`'s fifteen notation keywords — is one split and duplicates nothing.

But the shape that does blow up is not exotic. It is "several special cases plus a default", which is what
`document_read` and the studio adapter's `validate` are, and note 43 predicts both are rewritten as multi-subject
`match`.

### The correct long-term design

**Elaborate each arm body exactly once, and let the leaves reference it.**

For each arm, bind its body as a function of the arm's own pattern variables, elaborate that function once, and have
every leaf the arm reaches _apply_ it rather than re-elaborate it:

```
let arm₂ = (λ (z₀ : A₀) … (z_{k-1} : A_{k-1}). body) in
  <case tree, whose leaves are `arm₂ v₀ … v_{k-1}`>
```

The dependent typing works out, and the reason is precise: an arm reaches more than one leaf **only** where its pattern
in the split column is a variable, and that variable's binder is exactly the abstraction that makes the body uniform
across the constructors the variable covers. Where the goal is `P x` and the arm binds `z := x`, the hoisted body has
type `Π (z : A). P z`; the `Succ` leaf applies it to `Succ k′` and gets `P (Succ k′)`, which is that leaf's refined goal
by construction. An arm whose pattern in the split column is a _constructor_ reaches exactly one leaf and needs no
hoisting — so hoisting the variable-pattern arms is both sufficient and always well-typed.

This is ch. 6's let-bound right-hand side, not an invention: it is what the fat bar was doing structurally, minus the
failure continuation musa cannot type. It is also how Lean's `matcher` auxiliaries and Agda's clause-indexed case trees
avoid the same blow-up.

Two consequences worth naming:

- `Refusal::UnreachableBranch` gets _easier_, not harder: an arm is unreachable exactly when its `let` is never applied,
  which is one check on the finished tree rather than the current `selected` flag set at each leaf.
- The `let` must be bound outside the recursor application, so it is a `Term::bind` at the top of `compile`'s result
  with the arm functions in arm order. Origins are unaffected — each `let` carries the arm's own origin, which is more
  faithful to §7 than today's N copies of one body all carrying the same one.

**Where it lands.** The _implementation_ repair needs no amendment: `02-core-calculus.md` §6.2 fixes the meaning of
`match`, and the meaning does not change — the hoisted tree is convertible with the duplicated one, which is exactly the
law [`coverage_laws.rs`](../../../../crates/musa-core/tests/suite/coverage_laws.rs) already states. It should be a new
prompt between 136 and 137, ahead of 142's cutover; folding it into prompt 144 (`diagnostics-and-performance`) is worse,
because 142 runs the whole standard library through this compiler first.

**One part of the repair is not mine to make.** `02-core-calculus.md` §6.2 states the decline in the same incomplete
form the code does —

> with the `FAIL`/fat-bar mechanism **not** adopted, because it exists to express failure between equations and Musa's
> arms do not fall through

— and that sentence is what a future reader will re-derive the duplication from. Correcting it is a change to a
governing document, which is the prompt README's amendment procedure and the user's call, not a step in a batch run. The
sentence is not _false_; it is a true reason for declining one of the fat bar's two jobs, presented as a reason for
declining both. The minimal honest repair is one added clause naming §5.4.1's second job and the let-bound arm as its
replacement.

## §3 Finding B — column selection is "leftmost tested", with no necessity check

**Ad hoc, but small.**

`testable` scans columns left to right and picks the first column any row tests. ch. 5 is more naive still — it always
processes column 1 — so this is already an improvement. But it can split a column no _relevant_ row needs: given rows
whose first is all-variables in the remaining columns, the search still finds a later row's constructor and splits.
Maranget's necessity heuristic (choose a column that every row reaching this node must test) is the standard answer and
would shrink the trees Finding A duplicates into.

**Correct long-term design:** score columns by necessity, break ties leftmost. It is a strictly local change to
`testable` and it does not alter what a `match` means, so it is a cheap follow-on to Finding A's prompt — but it is a
_heuristic_, and Finding A's fix makes the residual cost linear rather than exponential, so it is not urgent on its own.

## §4 ch. 3 and ch. 6 — the enriched calculus

**Backed, and already load-bearing.** [`raw.rs`](../../../../crates/musa-core/src/raw.rs)'s `RawShape` is ch. 3's
enriched lambda calculus almost item for item — `Let`, `Match`, `Rec`, constructors, and annotation — extended for
dependency (`Pi`, `Id`, `J`, `RecordType`, `Update`) and missing only `FatBar`/`FAIL`, whose absence §2 has already
argued. AGENTS.md's **"no sublanguage by subtraction"** standard is derived from ch. 3 and holds here: nothing
downstream gets a stripped-down `Raw`.

ch. 6's transformations (dependency analysis, let-floating, full laziness) mostly answer questions a total, finite,
call-by-value core does not ask. The one that transfers is the let-bound right-hand side, and §2 is now asking for it.

---

# Part 2 — smalltt

## §5 What is followed

Coquand's algorithm — elaboration into the semantic domain with NbE conversion, contextual metavariables, de Bruijn
indices for terms and levels for values, pattern-fragment higher-order unification — is followed exactly.
[`value.rs`](../../../../crates/musa-core/src/value.rs)'s header states the boundary argument, and `Value` genuinely
never leaves the crate. That is smalltt's central structural decision and musa-core keeps it.

## §6 Finding C — no glued evaluation, because there is nothing to glue

**A designed-in cliff, arriving at prompt 142.**

smalltt's README spends its longest section on **glued evaluation**: a top-level definition evaluates to a _pair_ of
values, one where the definition stays folded and one where it is unfolded, so conversion can try the folded one first
and unification can produce solutions that mention `Nat` rather than the 400-node normal form of `Nat`.

musa-core has no top-level definition scope at all. `Spine::Const` covers families, constructors and recursors — all
rigid — and everything else is either a context assumption or a `let`, and `let` is δ-transparent through
`Scope::define`. There is nothing that _could_ be held folded, so there is nothing glued evaluation would buy today. The
implementation is not wrong for its current inputs.

It becomes wrong at **prompt 142**, which points the whole standard library at this core. Every stdlib definition will
be a `let` or a context definition, every use will unfold, and conversion will compare unfolded normal forms of library
functions with no way to try the folded comparison first. The diagnostics inherit it: a conversion failure will print
normal forms rather than the names the author wrote.

**Correct long-term design.** Give the core a definition scope whose values are smalltt's `G`: a pair of the least- and
most-reduced value. Concretely:

1. `Spine::Def(DefId)` as a _flexible-rigid_ head — it never blocks like a variable and never computes like a meta, it
   unfolds on demand.
2. Conversion tries the folded comparison first (two `Def` heads with the same id and convertible spines answer `true`
   without unfolding either), and unfolds only on disagreement. This is smalltt's speculation, and it is Finding E's
   prerequisite.
3. Quotation for _diagnostics_ stops at folded heads; quotation for meta solutions does not.

This is squarely a prompt-144 concern and 144's own Design should name it, because 144 re-measures the P1/P2 budget
against exactly the workload that exposes it.

## §7 Finding D — conversion has no approximate mode, and no structural rule at Π or at record types for _terms_

**Ad hoc. The stated justification argues correctness, not cost, and its premise is wrong for the roadmap.**

`Unifier::rigid` has structural arms for `Universe`/`Universe`, `Pi`/`Pi`, `RecordType`/`RecordType`, `Id`/`Id`,
`Refl`/`Refl` and `Neutral`/`Neutral`. It has **no arm for `Lam`/`Lam` and none for `Record`/`Record`**, and the
fall-through is:

```rust
// Everything else — including a lambda or a record literal, which
// stand here only as an eliminated argument — is decided by reading
// both sides back. Quotation is η-long, so that is exactly the
// conversion §3 specifies, and a structural walk would gain nothing.
_ => Self::by_reading_back(meter, depth, at, left, right),
```

Three things are wrong with this.

1. **"A structural walk would gain nothing" is a claim about the answer, and the objection is about the cost.**
   `by_reading_back` fully normalizes _both_ sides and allocates two complete normal forms before comparing a single
   node. A structural walk opens both closures at one fresh variable and can fail on the first field — the standard
   fail-fast that every NbE conversion checker has. smalltt's `conv` has the `VLam`/`VLam` case for this reason, and it
   also has `VLam`/other with η-expansion so a lambda and a neutral never need quoting either.
2. **The premise — that a lambda or record "stands here only as an eliminated argument" — is false for musa's own
   roadmap.** `10-traits.md` elaborates dictionaries to records. After prompt 137, comparing two dictionaries is
   comparing two record _values_, and it will be one of the most frequent conversions the checker performs. Each one
   will normalize two dictionaries end to end.
3. **`step` never dispatches on the type**, so η at Π and at record types is performed only by quotation. Two functions
   compared at a function type, neither of them a `Lam` in `Form`, reach `by_reading_back` as well.

**Correct long-term design.** Two changes, in this order:

- **Structural arms.** `Lam`/`Lam` opens both closures at one fresh variable and recurses at the codomain. `Record`/
  `Record` walks fields in telescope order. Add the η arms: at `At::Term(ty)` with `ty` a Π, open both sides under a
  fresh variable whatever their forms; with `ty` a record type, compare field by field. `by_reading_back` then becomes
  what it should be — the _diagnostic_ path, run only once conversion has already failed, to build the message.
- **Approximate conversion**, smalltt's rigid/flex/full three-state speculation with one-shot backtracking, layered on
  Finding C's `Def` heads. This one is only worth building after there are definitions to fold, so it belongs to the
  same prompt as C.

The first change is worth doing on its own and does not wait for definitions.

## §8 Finding E — meta solutions are quoted β-normal and η-long, then walked a second time

**Ad hoc.**

`Unifier::attempt` solves a metavariable by quoting the right-hand side at the type and then calling `restrict` over the
resulting `Term` to perform the occurs check and scope restriction; `elab.rs`'s `zonk` splices solutions into the
output. Two costs follow.

- **Solutions are maximally unfolded.** Everything a solution mentions is already normalized, so a meta solved against a
  library value stores that value's normal form. smalltt's README calls this out directly and answers it with **three
  quotation modes** — `rigidQuote`, `flexQuote`, and a `fullCheck` fallback carrying an `Irrelevant` marker — so a
  solution keeps folded heads wherever the occurs check does not force unfolding. It also keeps **eta-short** solutions
  for the same reason.
- **The occurs check is a second traversal.** `restrict` re-walks the quoted term after quotation has already walked the
  value. smalltt fuses the two — quotation _is_ the occurs check — and caches an approximate occurs result per
  metavariable so a repeated check is a lookup.

**Correct long-term design.** Fuse scope restriction into quotation as a quotation _mode_, and keep the fully-unfolded
walk as the fallback taken only when the approximate one reports a possible occurrence. Add smalltt's per-meta cache.
This depends on Finding C — there is no point in a flexible quote mode until there is something foldable to keep folded
— so it is one prompt with C and D, not three.

Meta **freezing** (smalltt's other listed technique) has no analogue to build: freezing exists to stop a later
generalization from solving an earlier definition's meta, and musa has no generalization boundary. Not a divergence.

## §9 Finding F — neutral spines are a linked list, so the head is O(n) away

**Ad hoc, and the concrete answer to the `/rust-performance` concern already raised in this session.**

```rust
pub(crate) enum Spine {
    Var(DbLevel, Arc<Value>),
    Const(Constant),
    Meta(Meta),
    App { function: Arc<Neutral>, argument: Arc<Value> },
    Project { record: Arc<Neutral>, field: Name },
    J { …, proof: Arc<Neutral> },
}
```

The head — `Var`, `Const` or `Meta` — sits at the _deepest_ position of a left-nested chain. Consequences:

- Finding the head of `f x y z` is three `Arc<Neutral>` hops. `flexible_head` does this on both sides of **every**
  unification step, and `force`'s `head_is_solved` does it again before that.
- An `n`-argument application allocates `n` separate `Arc<Neutral>`s, each with its own `Origin` and refcount, scattered
  across the heap. Spine comparison in `neutrals` walks two chains in lockstep with no length check available up front.

smalltt represents a spine as head-plus-vector for exactly these reasons.

**Correct long-term design:**

```rust
struct Neutral { origin: Origin, head: Head, spine: Vec<Elim> }
enum Head { Var(DbLevel, Arc<Value>), Const(Constant), Meta(Meta), Def(DefId) }   // Def from Finding C
enum Elim { App(Arc<Value>), Project(Name), J { … } }
```

Head access is O(1), length mismatch is one comparison, an `n`-argument application is one allocation, and `Elim` is a
small enum in a contiguous buffer. §7's per-node origins are preserved by carrying an `Origin` in each `Elim` — the
header's argument for them ("the spine of `f x y` reads back as three terms") is about quotation writing three nodes,
not about the values being three allocations.

This is the single highest-value performance change in the crate and it is invisible outside it: `Value` never leaves,
so the whole change is internal. It belongs in prompt 144, measured against 144's own gate.

## §10 Finding G — `convertible` is a second path, and the naive one

**Ad hoc, currently latent.**

```rust
pub fn convertible(cx: &Cx, ty: &Term, left: &Term, right: &Term) -> Result<bool, CoreError> {
    Ok(normalize(cx, ty, left)? == normalize(cx, ty, right)?)
}
```

This normalizes both sides completely and compares normal forms — the most expensive decision procedure available, and a
_second_ implementation of a question `Unifier` already answers value-directed with early exit. The repo's own
second-path audits (prompt 133's, and prompt 148's to come) forbid exactly this shape.

It is latent: a search finds no caller outside musa-core's own law suites, and stating the laws through the naive
procedure is arguably the _right_ thing for a specification test — it is the definition, checked against the
implementation. The trap is the first external caller, which prompt 142 will supply.

**Correct long-term design.** Keep one procedure. `convertible` becomes a thin call into the unifier's conversion with
metavariable solving disabled, and the law suites keep the normalize-and-compare version as a **test-local oracle**,
where its naivety is the point. Prompt 148's second-path audit is the natural place, but the facade should be fixed
before 142 hands it a caller.

## §11 Finding H — neutral variables carry their types

**Backed, with a cost worth recording.**

[`value.rs`](../../../../crates/musa-core/src/value.rs)'s header argues it: quotation is type-directed because it
performs η at Π and at records, so quoting a blocked application's argument needs that argument's type, so a neutral
must be able to say its type. Without it `f g` and `f (λx. g x)` quote differently and conversion answers `false` for
two terms §3 calls equal. That argument is sound and the alternative — type-directed _conversion_ without quoting, as
Agda does — is a larger change than it looks, because §7's origin obligations are stated over quoted normal forms.

The cost is real and should be recorded rather than fixed: every neutral variable holds an `Arc<Value>` it usually does
not need, and `neutral_type`/`head_type` recompute a spine's type on demand. Finding F's representation change would let
the type live on the `Head` rather than on every node of the chain, which is most of the win at no design cost.

---

# Part 3 — Where each finding lands

| # | Divergence | Status | Correct long-term design | Where |
| --- | --- | --- | --- | --- |
| A | Arm bodies duplicated per leaf; each copy re-elaborated | **ad hoc**, measured at 2.2×/column | Hoist each arm into a `let`-bound function of its pattern variables; leaves apply it (PJ ch. 6) | new prompt before 142 |
| B | `testable` picks leftmost tested column, no necessity check | ad hoc, small | Maranget necessity heuristic, leftmost tie-break | with A |
| C | No glued evaluation; no definition scope to fold | designed-in cliff | `Spine::Def` head + smalltt's `G` pair; folded-first conversion | 144, named in its Design |
| D | No structural `Lam`/`Lam` or `Record`/`Record`; no η dispatch on the type | **ad hoc**, premise false after 137 | Structural + η arms; `by_reading_back` demoted to the diagnostic path; approximate conversion after C | arms now; approximation with C |
| E | Meta solutions β-normal/η-long; occurs check a second traversal | ad hoc | Three quotation modes; fuse restriction into quotation; per-meta occurs cache | with C |
| F | Neutral spines are left-nested `Arc` chains; head O(n) | **ad hoc** | `Neutral { head: Head, spine: Vec<Elim> }` | 144 |
| G | `convertible` normalizes both sides — a second path | ad hoc, latent | One procedure; the naive one becomes a test-local oracle | before 142; audited at 148 |
| H | Neutral variables carry their types | **backed** | Keep; move the type onto `Head` when F lands | recorded only |

## What this does _not_ find

Worth stating, because a thorough audit that finds only problems is not thorough.

- **No soundness divergence.** Every finding is about cost or about a second path. Nothing here says the core decides
  conversion wrongly, elaborates the wrong term, or accepts a program it should refuse.
- **The declined mechanisms are correctly declined.** The fat bar (untypeable in a dependent core), meta freezing (no
  generalization boundary), ch. 6's laziness transformations (no laziness), and ch. 10–24's runtime machinery (musa has
  no graph reduction) are all absent for reasons that survive.
- **The `Value`-never-leaves boundary is the right one** and is held. smalltt does not have this constraint, and
  musa-core keeping it while still following Coquand's algorithm is the audit's clearest success.

## The one thing that would change the verdict

Finding A's measurement was taken on a program written to expose it. The honest next step is to take the same
measurement on **real** input — which does not exist until prompt 142 puts the standard library through this compiler.
If 142's corpus contains no multi-subject `match` with several partially-constraining arms, Finding A is a latent hazard
rather than a present cost, and the hoisting prompt can wait for 144. Note 43's rewrite of `document_read` and of the
studio adapter's `validate` predicts otherwise, and those two programs are the cheapest available test.

## Closing: what prompt 136b discharged, and what the measurement became

Written after the fact, because an audit that keeps claiming a cost that is gone has stopped being evidence.

**Discharged.** Findings B, D, E, F, and G landed in prompt 136b. Neutrals are head-plus-vector, so the head is one
field access away and a spine-length disagreement is one comparison; conversion has structural `Lam`/`Lam` and
`Record`/`Record` arms and dispatches η on the type, with reading back demoted to the path that builds a mismatch's
message; scope restriction and the occurs check ride on the quotation that writes a solution rather than walking the
finished term again; `testable` consults only the first row; and `convertible` is one call into the unifier with solving
disabled, with normalize-and-compare kept in `conversion_laws.rs` as the oracle it is checked against. Finding H is
mostly paid off by F, because a neutral's type now lives once on the head rather than on every node of a chain.

**Two findings turned out to be one.** §2 and §3 were written as one change on the theory that B decides how many leaves
A has to serve. B decided the whole question. On the interleaved program §2 measured — 2.2× per column, 3 columns at
55,942 characters and 10 at 14,928,160 — the first-row rule leaves this:

| columns | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| term size (chars) | 20,264 | 26,279 | 34,594 | 45,211 | 58,132 | 73,361 | 90,893 | 110,729 |

That is `22,138 + 1,384·k²` to within a percent, every column elaborates in under 3 ms, and each arm on that program is
now elaborated exactly once. The shape §2's "second shape" bound — one specific arm plus a catch-all — is unaffected by
B and still grows linearly in the leaves the catch-all reaches (20,264 at one column to 203,517 at eight), which is what
is left of Finding A.

**§2's proposed condition for hoisting is wrong.** "An arm is hoistable iff the types of the variables it binds, and the
goal its body answers, are all expressible at the match's own depth" inspects the abstraction's interface, and what
breaks is the body. A leaf binds its pattern variables with `define`, so an arm body may rely on the binder _reducing_;
a λ binder is an assumption, and the hoisted body is checked in a strictly weaker context than any of its leaves.
`coverage_laws.rs`'s `a_pattern_binder_is_a_definition_at_every_leaf_it_reaches` is the counterexample and is now a
permanent law: every binder in it is a `Nat` and its goal is a `Nat`, so the condition admits it, and the abstraction it
would build does not typecheck. The sound shape is speculative — build the abstraction, elaborate once, fall back per
leaf, report only the per-leaf refusal — and it needs a plan pass first, because splitting an indexed family refines the
binder types too.

**So the answer to "the one thing that would change the verdict" is: it changed.** Finding A is a latent constant factor
rather than a present cost, and it goes to prompt 144 with Finding C, which is where this note's last section said it
should go if the measurement moved. Six items are waiting there, and 144's Design names each one.
