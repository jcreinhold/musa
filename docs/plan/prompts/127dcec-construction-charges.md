---
id: 127dcec
slug: construction-charges
status: done
depends_on: [127dceb]
phase: 3
---

# Charge a Value Where It Is Constructed, Not Where It Is Named

## Task

The work meter charges a value's whole shape at every expression the value passes through, and every capture of it into
a closure. Naming a value constructs nothing, and a constructor holding a value it was handed constructs one node, not
that value again. Make the node and byte counters charge construction, bump the cost table, and say in the governing
calculus where a value is charged.

## Read

- `docs/rules/language/02-core-calculus.md` §3 (the versioned cost table, and the budget-independence law) and §4 (the
  meter's six bullets and the prompt-96 defaults). §4 already says the meter covers "constructed machine node count and
  wiring depth" and "instantiated definition count and closure environment size". This prompt is the implementation
  catching up to those words and then pinning them so they cannot drift again — it does not widen what the meter is
  *about*.
- Prompt 96, which introduced the counters, and its `preflight_construct` rule that a finite aggregate operation charges
  its result shape before allocating it. That rule stays.
- `crates/musa-compiler/src/phase/mod.rs`: `eval`'s tail, which runs `value_shape` for every `ExprKind` including
  `ExprKind::Name`; `value_shape` and `aggregate_shape`; the `Value::Closure` arm, which deep-counts captured values.
- `crates/musa-compiler/src/phase_budget.rs`: `CostTable::V1`, `Budget::LANGUAGE`, `WorkMeter::construct`,
  `WorkMeter::preflight_construct`, and the `a_narrowed_budget_*` tests that state budget independence.
- Prompt 127dcfa, the trial that found this. Measurements are in **Design**.
- Peyton Jones ch. 10 §10.3: each node of the graph is one cell, and a cell's field holds the *address* of another cell.
  Constructing `CONS E₁ E₂` allocates one cell with two pointers; it does not allocate E₁ and E₂ again. Ch. 12 is why
  the tree becomes a graph at all — reduction shares subexpressions rather than copying them, and a cost model that
  counts a value once per mention is charging for copies the machine does not make.
- Root `AGENTS.md`, "Hand a consumer what we already computed": the same principle one level down. A stage that has
  already paid for a value should not pay for it again because a later expression mentions it.

## Design

**The defect, measured.** Prompt 127dcfa's staff adapter reads a region and threads a reader state through it. On the
smallest region that adapter accepts — six header lines and one bar — expansion is stopped at 104,016 constructed value
nodes against a limit of 100,000. Replacing its one closure-chain fold with a plain left fold of identical per-step work
still costs 100,009. Charging `ExprKind::Name`, a `match` arm's result, an application's result, and a fold case's
result nothing, and counting a closure's captures rather than deep-counting them, brings the same region in with room
and lets the adapter answer with its own sentence. One root cause, two places.

**The rule.** A value is charged where it is constructed, not where it is named. Three cases:

- *Selection.* An expression that yields a value that already existed constructs nothing and is charged nothing:
  `ExprKind::Name`, `ExprKind::Match` (the arm's result), `ExprKind::Apply` (the body's result), and `ExprKind::Fold` (a
  case's result). Each of these still charges its reduction step, which is what bounds a program's *length*; what it
  must not charge is the *size* of something it did not build.
- *Wiring.* An expression that builds one aggregate out of parts already evaluated is charged that aggregate's own node
  and its immediate fields, not its fields' contents: products, lists, options, injections, `data` construction, and a
  lambda's closure, whose charge is its own node plus one per captured name. This is §4's "wiring depth" read literally
  and ch. 10's cell with pointer fields.
- *Fabrication.* An expression whose value is new all the way down is charged all the way down, exactly as today:
  literals, pitch actions and scale steps, music and event track quotations, and builtin results. `range` and `repeat`
  fabricate, and keep their preflight charge.

The byte counter follows the node counter, case for case, and for the same reason: naming a value creates no bytes
either. A `Text` literal still charges its length, because it is a fabrication.

**What this cannot do is change an accepted program.** The budget can stop an evaluation and cannot change one, so
charging less can only turn `failed` into `done`. That is the existing budget-independence law and this prompt does not
touch it; it adds the narrower statement that the new charges are pointwise no larger than the old ones, so nothing that
compiles today stops compiling.

**Acceptance moved, so the cost table moves with it.** `CostTable::V2`, with the weights unchanged and the reason
stated: version 1 charged a value once per mention, which made a project's cost the product of its data size and its
program size rather than the count of what it built. A rejection cites the table it was rejected under, so the version
is what tells two compilers they do not agree.

**The governing calculus gains one sentence, not a new idea.** §4's bullet list already names constructed node count and
closure environment size. It gains, after the "finite aggregate operation" sentence, the locus those bullets left open:
a value is charged once, where it is constructed; an expression that names, selects, or returns a value already built
charges no nodes and no bytes for it, and a constructor is charged its own node and its fields' wiring rather than its
fields. That sentence is what makes the implementation checkable against the document rather than the other way round.

## Target

- `crates/musa-compiler/src/phase/mod.rs` — `eval` charges by the three cases above; `value_shape`'s `Value::Closure`
  arm counts captures instead of deep-counting them. `value_shape` itself stays a deep measure, because fabrication and
  `preflight_construct` still need it.
- `crates/musa-compiler/src/phase_budget.rs` — `CostTable::V2`, weights unchanged, reason in its doc comment.
- `docs/rules/language/02-core-calculus.md` §4 — the charging-locus sentence, and the cost-table version where §3 names
  it.
- Tests in `crates/musa-compiler`: naming a large value repeatedly costs no more than building it once; a `data` value
  holding a large field costs one node more than the field; a chain of `n` such constructions is linear in `n` and not
  quadratic; a closure capturing a large value costs its captures and not their contents; and the existing
  budget-independence and resource-limit tests still state what they stated.
- `docs/book/src/reference/` — wherever the budget is described to a reader, the cost table version it names.

## Check

```sh
cargo nextest run -p musa-compiler -p musa-project -p musa-lsp
cargo clippy --all-targets -p musa-compiler -p musa-project -p musa-lsp -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
```

Commit as `Charge a value where it is constructed, not where it is named`.

## Stop

- No change to the budget's limits. `Budget::LANGUAGE`'s five numbers are prompt 96's and stay; prompts 124 and 188 own
  measuring them.
- No change to the cost table's weights. They stay uniform at one, for prompt 96's stated reason.
- No new eliminator and no change to `list_fold`'s direction. That `list_fold` is a left fold while `nat_fold` and every
  generated `data` fold are catamorphisms is a real inconsistency and a separate one; it is recorded, not fixed here.
- No interrupt, no partial result, and no caller-visible budget knob. `under_budget` stays test-only.
- No adapter and no stdlib change; prompt 127dcfa resumes after this one.
- No change to `docs/rules/` beyond §3's cost-table version and §4's charging-locus sentence.
