---
id: 133
slug: core-crate
status: done
depends_on: [132]
phase: 3
---

# Build the Dependent Core as a Leaf Crate

## Task

Create `crates/musa-calculus`: the core term language of `docs/rules/language/02-core-calculus.md` — universes, Π,
dependent records, an identity type — with contexts, evaluation, quotation, and conversion by
normalization-by-evaluation, behind a facade that exposes neither semantic values nor the evaluator. No surface, no
musical types, no elaboration. The old checker keeps working; nothing calls this crate yet except its own tests.

## Read

- `docs/rules/language/02-core-calculus.md` §1 (syntax), §3 (dynamic semantics and conversion), and §5's obligation
  matrix — the rows this prompt discharges and the rows it must not claim.
- `docs/notes/research/language-design-closure/43-*.md` (prompt 132's trial) — specifically what it deleted from the
  specification. A mechanism the trial marked for deletion is not built here.
- `crates/musa-events/src/lib.rs` — the leaf crate this one is shaped after: a narrow facade, no dependency on anything
  above it, and a public surface small enough to read in one sitting.
- `crates/musa-compiler/src/phase/mod.rs`'s `Value`, `Term`, and evaluator, and `core_budget.rs` — 15,017 lines that
  already contain a total evaluator with a cost budget. What survives is the *budget discipline*, not the
  representation, and this prompt should be explicit about which of the two it is borrowing.
- `docs/plan/roadmap.md` §15 as amended by prompt 128 — the crate's declared place and its dependency list. A dependency
  not listed there is a repair, not a `cargo add`.
- Peyton Jones ch. 8–9 for the shape of a checker's data structures, and the `module-design` skill's rule that no public
  item exists without a caller.

## Design

**The facade is two operations and a context.** `normalize(&Cx, &Term) -> Term` and `convertible(&Cx, &Term, &Term) ->
bool`, over a `Cx` that carries the typing context and the evaluation budget. `Value`, the closure representation, the
environment, and the quotation machinery are all private. That is the whole point of NbE living behind a boundary: a
caller asks whether two terms are the same, and the crate answers without teaching the caller what a semantic value is.

The obvious objection is that prompt 134's elaborator wants to check against a *value* type rather than re-normalizing
at every step, and exposing `Value` would let it. The answer is that the elaborator then belongs in this crate, not that
`Value` belongs in the facade: 134 adds check/infer *inside* `musa-calculus` over a surface-independent raw term, and
`musa-compiler` keeps only the translation from Musa syntax to that raw term. Exposing `Value` now would be a public
item with no caller in this prompt and a caller in the next one who is on the wrong side of the boundary. Write that
argument into the crate's module documentation, because the pressure to leak it will come back.

**Terms are de Bruijn-indexed; values close over levels.** The standard NbE arrangement, and the standard reason:
α-equivalence becomes structural equality on terms, and quotation needs a level to name a fresh variable without
renaming anything. Say which is which in the doc comments, because mixing them up is the classic bug in this
construction and the type system will not catch it.

**Universes carry levels, and levels have their own arithmetic.** `Level` with `zero`, `succ`, and `max`, and a
comparison that decides `l ≤ l'` on the syntactic forms 129 allows. No cumulativity — a `Type l` is not a `Type (l+1)` —
so there is no subtyping in conversion and nothing here needs to know about coercions.

**Records are primitive with η.** Conversion at a record type compares fields, not projections, and two terms are
convertible when their projections are, including when one side is a variable. Getting record η right is what makes
prompt 137's dictionaries compare cheaply, so it belongs at the bottom rather than being papered over higher up.

**The budget survives the representation change.** `core_budget.rs`'s discipline — evaluation runs under a cost budget,
exhaustion is a distinct outcome, and a completed evaluation is budget-independent — carries into this crate as 129 §4's
three-outcome law: `normalize` and `convertible` return an exhaustion outcome that is neither success nor a negative
answer. A `bool` return for `convertible` therefore is not enough; the signature above is the sketch, and the real one
carries the third case. Say so in the doc comment rather than discovering it when the first deep term arrives.

**Laws, as tests, in `crates/musa-calculus/tests/suite/`.** Conversion is reflexive, symmetric, and transitive;
conversion agrees with normalization (`convertible(a, b)` exactly when `normalize(a) == normalize(b)`); normalization is
stable (`normalize(normalize(t)) == normalize(t)`); evaluation is deterministic; α-equivalent inputs have identical
terms; β, η at Π and at records, δ, and ι each hold as stated equations; and exhaustion never turns into a wrong answer.
Each law names the §5 obligation it partially discharges and says what prompt 148 still owes.

## Target

- `crates/musa-calculus` in the workspace, with the facade above, `Value` and the evaluator private, and a module doc
  comment that states the invariants before the implementation does.
- `crates/musa-calculus/tests/suite/{main.rs, conversion_laws.rs, normalization_laws.rs, budget_laws.rs}` — one test
  binary, one module per file, per root `AGENTS.md`.
- `docs/plan/code-map/`: `musa-calculus`'s row, marked for what is implemented here and what is absent.
- No change to `musa-compiler`, `musa-syntax`, or any shell. The crate is a leaf with no callers yet, and it stays that
  way until prompt 142.

## Check

```sh
cargo build --workspace
cargo nextest run -p musa-calculus
cargo clippy --all-targets -p musa-calculus -- -D warnings
cargo fmt --check
cargo deny check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
```

`cargo nextest run --workspace` must also still pass unchanged: this prompt adds a crate and changes no behaviour, so a
moved snapshot or a moved oracle entry anywhere else is a defect in this commit.

Commit as `Build the dependent core as a leaf crate`.

## Stop

- No elaboration, no metavariables, no unification. Prompt 134.
- No `data` declarations, no inductive families, no dependent match, no coverage, no termination checker. Prompt 135.
- No records at the surface, no traits, no operators, no `Syntax`, no collections, no musical type of any kind. This
  crate does not know what a pitch is and must not learn.
- No public `Value`, no public evaluator, no public environment, and no `pub(crate)` widened to `pub` "for tests" —
  tests that need internals belong beside the code as unit tests.
- No deletion from `crates/musa-compiler`. The old checker stays working until prompt 142's single cutover; deleting
  either checker early leaves the tree red across a prompt boundary.
- No new external dependency beyond roadmap §15's list for this crate.
