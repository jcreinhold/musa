---
id: 136b
slug: core-divergence-repair
status: in-progress
depends_on: [136]
phase: 3
---

# Remove the Ad Hoc Divergences from the Dependent Core

## Task

Note 44 audited `musa-core` against smalltt and Peyton Jones ch. 3–6 and found seven divergences, five of them ad hoc —
departures that were never argued, or were argued against a different question than the one they answer. One is measured
today at 2.2× per matched column in both term size and elaboration time. Remove every ad hoc divergence that can be
removed without what prompt 142 supplies — a top-level definition scope, and real code to price a change against — and
hand the ones that cannot to prompt 144 with the argument written down rather than left to be rediscovered.

## Read

- [`docs/notes/research/language-design-closure/44-audit-against-smalltt-and-peyton-jones.md`](../../notes/research/language-design-closure/44-audit-against-smalltt-and-peyton-jones.md)
  in full: the seven findings, their classification, the measurement and the second shape that bounds it, and the stated
  long-term design for each. It governs nothing — it is the evidence this prompt implements.
- Peyton Jones ch. 5 §5.4.1 (`unwieldy`: only the constructor rule duplicates right-hand sides), §5.5 (uniformity, and
  what it implies about order-independence), and ch. 6's let-bound right-hand side. The fat bar's _second_ job is what
  this prompt supplies; its first job stays declined, for the reason [`case.rs`](../../../crates/musa-core/src/case.rs)
  already gives.
- Notes [24 §H5](../../notes/research/language-design-closure/24-pipeline-and-syntax-review.md),
  [26 §2.4](../../notes/research/language-design-closure/26-language-design-decision.md), and
  [29](../../notes/research/language-design-closure/29-source-and-expansion-spec.md) — this repo found Finding A twice
  before, chose join points in writing, and then lost the decision when note 42 superseded that design line for an
  unrelated reason. What this prompt builds is that decision, spelled in a calculus with no labels.
- `~/Code/smalltt`'s README on approximate conversion, the three quotation modes, approximate occurs checking, and
  head-plus-vector spines — and on glued evaluation, which is the one this prompt does _not_ build.
- `crates/musa-core/src/{value.rs, eval.rs, quote.rs, unify.rs, case.rs, lib.rs}` — the six files that change, and the
  header arguments in each that are being amended rather than ignored.
- `docs/rules/language/02-core-calculus.md` §3 (definitional equality, η at Π and at records) and §6.2 (the case
  compiler). §6.2's sentence about the fat bar is **not** repaired here; see **Stop**.
- Prompt [144](144-diagnostics-and-performance.md), whose Design this prompt repairs to own Finding C.

## Design

Five repairs, in the order they must land. Each is internal to `musa-core`: `Value` never leaves the crate, so nothing
below is visible to any other crate, and the whole prompt changes no program's meaning.

**F first, because D and E are written against it.** Today a neutral is a left-nested chain of `Arc<Neutral>`: finding
the head of `f x y z` is three hops, `flexible_head` does it on both sides of every unification step, `force` does it
again before that, and an `n`-argument application is `n` separate allocations with `n` refcounts. Replace it with
smalltt's head-plus-vector:

```rust
struct Neutral { origin: Origin, head: Head, spine: Vec<Elim> }
enum Head { Var(DbLevel, Arc<Value>), Const(Constant), Meta(Meta) }
enum Elim { App { origin: Origin, argument: Arc<Value> }, Project { origin: Origin, field: Name }, J { … } }
```

Head access becomes O(1), a spine length mismatch is one comparison before any element is compared, and an application
is one allocation. §7's per-node origins are preserved by the `origin` on each `Elim`: `value.rs`'s header argues them
from quotation writing three nodes for `f x y`, which is a fact about quotation, not about how many allocations the
value needs. Finding H's cost — every neutral variable carrying an `Arc<Value>` type it usually does not need — is
mostly paid off by the same change, because the type now lives once on the `Head` instead of on every node of a chain.
Doing F first is not sequencing for its own sake: D adds arms that read spines and E rewrites the traversal that walks
them, and building either against the linked list means writing it twice.

**D: conversion gets structural and η arms, and reading back becomes the diagnostic path.** `Unifier::rigid` has no
`Lam`/`Lam` arm and no `Record`/`Record` arm, and falls through to `by_reading_back`, which normalizes _both_ sides
completely and allocates two whole normal forms before comparing one node. The comment defending it says a structural
walk "would gain nothing", which is a claim about the answer where the objection is about the cost, and it rests on a
premise — that a lambda or record literal "stands here only as an eliminated argument" — that prompt 137 falsifies:
traits elaborate to dictionaries, dictionaries are records, and comparing two dictionaries will be among the most
frequent conversions the checker performs. Add:

- `Lam`/`Lam`: open both closures at one fresh variable and recurse at the codomain.
- `Record`/`Record`: walk fields in telescope order, failing at the first field that differs.
- η by dispatching on the type: at `At::Term(ty)` with `ty` a Π, open both sides under a fresh variable whatever their
  forms; with `ty` a record type, compare field by field. This is what makes `f` and `λx. f x` agree without either
  being quoted.

`by_reading_back` then keeps exactly one job: once conversion has already failed, build the two normal forms the message
prints. That is a demotion, not a deletion — the message is §3's own statement of what disagreed.

**E: scope restriction is a quotation mode, not a second traversal.** `Unifier::attempt` quotes the solution and then
calls `restrict` over the resulting `Term` to perform the occurs check and the scope check, walking the same structure
twice and building the intermediate term even when the check fails on its first node. Fuse them: quotation takes the
metavariable and the permitted scope, and refuses in place.

The three quotation modes (`rigidQuote`/`flexQuote`/`fullCheck`) are **not** built here. They exist to keep folded heads
folded, and there is nothing foldable in this core yet; see the handoff below.

**And neither is smalltt's per-metavariable occurs cache**, which this prompt asked for before the fusion was written
out. The cache answers "does `?α` occur in this value" by lookup instead of by traversal — and once the check rides on
the walk that _writes the solution_, there is no traversal to skip: quotation visits every node of the term it is
building whether or not the answer is already known. smalltt's cache pays off because its occurs check is a separate
approximate pass that stops at folded definitions; musa-core has no definition scope, `force` unfolds a solved
metavariable before quotation matches on it, and so the solution's every node is written out anyway. The cache becomes
able to save work at exactly the moment glued evaluation lands, and for exactly the same reason — so it goes to prompt
144 with Finding C rather than being built here as a lookup that can never hit.

**A and B: only a necessary column is split, and that is enough.** These were written as one change because B decides
how many leaves A has to serve; B turned out to decide the whole question.

_B is the smaller half and the larger win on the measured program._ `testable` scans **every** row for a constructor
pattern and splits the leftmost column any of them tests. But a `match` is ordered: if the _first_ row of a matrix has a
variable in every remaining column, that row matches whatever the subjects are, and no later row at that node can be
reached — so there is nothing to test and the leaf is the answer. Today the search finds a later row's constructor and
splits anyway, and note 44's program is exactly that shape, which is why its tree is exponential. The repair is one
sentence of code and it is Maranget's necessity condition specialized to an ordered match: **the first row decides
whether to test at all, and the column is the leftmost the first row tests.** Coverage is unaffected — a row that
matches everything covers everything — and `unselected` already reports arms that no leaf selects, which is the correct
verdict for the rows this rule steps over.

_A does not land here, and B is why._ The plan was ch. 6's let-bound right-hand side — note 26 §2.4's join point spelled
without labels:

```
let armᵢ = λ (x⃗ : T⃗). body in  <case tree, whose leaves are `armᵢ v⃗`>
```

hoisted wherever the arm's body is **uniform** across the leaves it reaches, with note 44's test for uniformity: an arm
is hoistable iff the types of the variables it binds, and the goal its body answers, are all expressible at the match's
_own_ depth. That condition is **wrong**, and this prompt's falsifier clause is what caught it. The condition looks at
the abstraction's interface; what breaks is the body. A leaf binds its pattern variables with `define`, so `b` at a leaf
is not a `Nat` — it is `Succ j`, and the body may rely on that reducing. A λ binder is an assumption, so the hoisted
body is checked in a strictly weaker context than any leaf's. `coverage_laws.rs`'s
`a_pattern_binder_is_a_definition_at_every_leaf_it_reaches` is the witness: every binder type in it is `Nat` and its
goal is `Nat`, so the condition admits it, and `λ (a b : Nat). f b refl` does not typecheck because opaque `b` is not
`Succ (pred b)`.

The exact condition is "the abstraction typechecks", which is not decided by inspection but by attempting it, so the
sound shape is speculative: build the abstraction, elaborate the body against it once, fall back to per-leaf elaboration
where it is refused, and report only the per-leaf refusal. Two further costs come with that shape. An arm's binder
_types_ are not uniform across leaves either once a family is indexed — splitting refines `Vec A n` to `Vec A (Succ k)`
— so a plan pass has to establish telescope uniformity before an abstraction can even be formed. And a refused hoist
costs `1 + N` elaborations where today's costs `N`.

None of that is unbuildable; it is a larger and more delicate change than a prompt about removing ad hoc divergences
should carry, and B has removed the reason it was urgent. Note 44 measured 2.2× per column — 3 columns at 55,942
characters, 10 at 14,928,160 — and argued A had to land before prompt 142 pointed the standard library at this compiler.
After the first-row rule that program is polynomial (3 columns at 34,594 characters, 6 at 73,361, ~1,384·k² + 22,138),
each arm on it is elaborated exactly once, and every measured column is under 3 ms. What is left of A is a linear
constant factor on the leaves a non-first-row arm reaches, whose worth can only be judged against real code. That is
prompt 142's output and prompt 144's question, which is where Finding C already went for the same reason.

**G: one conversion procedure.** `convertible` normalizes both sides completely and compares — the most expensive
decision available, and a second implementation of a question the unifier already answers value-directed with early
exit. It is latent today (no caller outside the law suites) and prompt 142 supplies the first real one. Make the facade
a thin call into the unifier's conversion with metavariable solving disabled, and move the normalize-and-compare version
into the law suites as a **test-local oracle**, where its naivety is the point: the specification checked against the
implementation. The same applies to `convertible_types`.

**What is deliberately left to prompt 144, and why.** Finding C — no glued evaluation — is not ad hoc: musa-core has no
top-level definition scope, so there is nothing that _could_ be held folded, and every smalltt technique that is missing
is a technique for deciding when not to unfold. It becomes wrong at prompt 142, which points the standard library at
this core. Four items ride on it and cannot be built before it: `Spine::Def` and the `G` pair, D's approximate
rigid/flex/full conversion, E's flexible quotation mode, and E's per-metavariable occurs cache. **Finding A joins
them**, for a different reason argued above — not that it cannot be built, but that what remains of it after B is a
constant factor no one can price until real code goes through this compiler. This prompt's documents half is to repair
144's **Read**, **Design**, and **Target** so 144 owns all six by name and by citation, rather than leaving a future
reader to rediscover the audit. That is a repair of a pending prompt, which the prompt README's §6 makes ordinary.

**What would falsify this prompt.** The hoisting clause fired, and above is what it produced: the condition admitted an
arm whose abstraction does not typecheck, so A was repaired out of this prompt rather than patched with a special case,
and the program that caught it stays behind as a law. The other clause still stands: if B's first-row rule changes which
programs are accepted, the rule is wrong — it may change the _shape_ of a tree and it may change which arms are reported
unreachable, but never whether a `match` is exhaustive.

## Target

- `crates/musa-core/src/value.rs` and its 50 use sites: `Neutral { head, spine: Vec<Elim> }`, with `Head` and `Elim` as
  above and per-`Elim` origins.
- `crates/musa-core/src/unify.rs`: structural `Lam`/`Lam` and `Record`/`Record` arms, η dispatch on the type at Π and at
  record types, and `by_reading_back` reachable only on the failure path.
- `crates/musa-core/src/quote.rs` and `unify.rs`: scope restriction and the occurs check fused into quotation, and
  `restrict` deleted rather than kept beside it.
- `crates/musa-core/src/case.rs`: first-row column selection.
- `crates/musa-core/src/lib.rs`: `convertible` and `convertible_types` as one procedure with the unifier.
- Laws in `crates/musa-core/tests/suite/`:
    - note 44's interleaved-column program elaborates with a term size that grows **polynomially** in the number of
      columns, stated as a ratio law so it is not a pinned byte count;
    - an arm reaching two leaves gets each leaf's _definition_ for the variables it binds — the program that falsified
      A's hoisting condition, kept behind as the bound on what prompt 144 may do to `case.rs`;
    - an arm no leaf applies is still `Refusal::UnreachableBranch`, at the same origin;
    - conversion answers what it answered before on the whole `fixtures::corpus`, including the η samples, with
      `by_reading_back` never reached on a success;
    - a metavariable solution that would capture a variable is still refused, and the fused check refuses at the same
      programs the two-pass one did;
    - `convertible` and the test-local oracle agree on every sample — the second-path audit stated as a test rather than
      as a prohibition.
- `docs/plan/prompts/144-diagnostics-and-performance.md`: **Read**, **Design**, and **Target** naming Finding C, the
  `Def` head and the `G` pair, approximate conversion, the flexible quotation mode, the per-metavariable occurs cache,
  and Finding A with the falsifier this prompt found and the speculative shape that answers it, each cited to note 44.
- `docs/plan/code-map/` rows updated for what changed.
- `docs/notes/research/language-design-closure/44-…md`: a short closing section recording which findings this prompt
  discharged and what the measurement became. The note is evidence; leaving it claiming a cost that is gone would make
  it wrong.

## Check

```sh
cargo build --workspace
cargo nextest run -p musa-core
cargo nextest run --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
python3 scripts/renumber-prompts.py audit
```

Commit as `Remove the ad hoc divergences from the dependent core`.

## Stop

- **No amendment of `docs/rules/`.** §6.2's sentence about the fat bar states a true reason for declining one of its two
  jobs as a reason for declining both, and note 44 §2 says what the minimal honest repair is — but that is a governing
  document and the amendment procedure in `docs/rules/README.md` is the user's call, not a step in a batch run. The
  implementation repair needs no amendment: the hoisted tree is convertible with the duplicated one, which is the law
  `coverage_laws.rs` already states.
- No glued evaluation, no `Spine::Def`, no definition scope, no approximate conversion, no flexible quotation mode, and
  **no hoisting of arm bodies** — no plan/emit split of `case.rs` either, since its only caller was the hoist. Prompt
  144, which this prompt repairs to say so.
- No behaviour change. The same programs elaborate, to convertible terms, with the same refusals at the same origins.
  The one permitted difference is which arm a `Refusal::UnreachableBranch` names when B's rule steps over rows an
  exhaustive earlier arm already covered — and that difference is a law here, not a side effect.
- No `musa-compiler` change, no `stdlib/` change, no `examples/` change. Prompt 142.
- No new `Refusal` variant and no new `musa explain` code: nothing here is a new way for a program to be wrong.
- No optimization outside the five findings, and none without a measurement. The `rust-performance` rule holds: a change
  that does not move a number is reverted, not kept because it seemed principled.
