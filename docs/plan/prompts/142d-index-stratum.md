---
id: 142d
slug: index-stratum
status: done
depends_on: [142c]
phase: 3
---

# Implement the Index Stratum: a Separate Language, a Separate Decider

> **Reversed by [prompt 143](143-one-theory-amendment.md), *Commit to One Theory, and Amend the Constitution to Say
> So*.** What this prompt built is deleted by [prompt 151](151-delete-the-index-stratum.md): the index expression
> language, its decision procedure, and the conversion checker's hook into it all go, and an indexed type becomes an
> ordinary applied type constructor. The one hook is the reason — a solver the checker consults rather than implements
> is an acceptance rule read-back cannot see, and 143 refuses that class outright.
>
> This prompt was executed and its work is in the tree until 151 removes it. It stays because the ledger links to it and
> because 151's Read section cites it for what has to come out.

## Task

Build what 142c specified: an index expression language over `Nat`, exact `Ratio`, and finite literal enums; a decision
procedure for equality and entailment in it; indexed declared and base types; and one hook in the conversion checker
that hands an index question to the solver. Erasure is total, so nothing downstream of elaboration changes.

## Read

- `docs/rules/language/02-core-calculus.md` §1.5 as 142c wrote it. Where this prompt and that section differ, repair the
  section first and commit the repair.
- [`51-the-terseness-audit.md`](../../notes/research/language-design-closure/51-the-terseness-audit.md) §4, especially
  the four properties: separate index language, erased, solver not unifier, term core unchanged.
- `crates/musa-calculus/src/base.rs`, `Base`'s existing parameter mechanism — `Syntax` is already indexed by a category
  literal and `EventTrack` by its time. That is this feature in special-cased form, and the general version should
  subsume both rather than sit beside them.
- `crates/musa-calculus/src/convert.rs` after 142b, the `rigid` descent — the one place an index comparison enters.
- `crates/musa-calculus/src/budget.rs`. The solver is metered like everything else; an index question that costs more
  than the budget allows is a refusal, not a hang.

## Design

**A new module, `index.rs`, and it does not import `value.rs`.** That independence is the design, not an accident: an
index expression is not a term, cannot mention a term variable, and is not evaluated by `eval`. The module holds the
expression type, substitution over index variables, normalization to a linear form, and `decide(a, b) -> Answer`. If it
ever needs a `Value`, the stratification has been broken and the prompt is wrong.

**Presburger, but the small fragment first.** Equality of two normalized linear forms over ℕ/ℚ with variables settles
every program in note 51 §3's table. Implement that; refuse anything outside it by naming the expression. A general
Presburger procedure is admitted later by a prompt that names the program needing it — the audit rule of note 51 §2,
applied to this prompt's own machinery.

**Indices are erased at quotation, not at code generation.** `quote` drops them, so a read-back term carries none and
`43-semantic-identity.md`'s digests are unchanged by construction rather than by a test. The suite proves it: every
existing snapshot and the pinned digests in `musa-events` move through this prompt byte-identically.

**A refinement constructor carries its index out.** `fn row(pcs: List<Pc(n)>) -> Result<Row(n), RowFault>` — `n` is read
off the argument type, and the constructor's runtime check is the one that already exists. This is §2.2's mechanism
gaining a type, not a new one.

**Diagnostics name the index, never the solver.** "a `Row(12)` where a `Row(24)` was expected" and never "linear
constraint unsatisfiable". The solver's internal form does not appear in a message.

## Target

- `crates/musa-calculus/src/index.rs`: the expression, normalization, `decide`, metered. Private to the crate except for
  the type an indexed declaration carries.
- `crates/musa-calculus/src/base.rs`: `Syntax<Cat>` and `EventTrack<time>` reworked onto the general mechanism, or a
  recorded reason they cannot be.
- `crates/musa-calculus/src/declare.rs`, `raw.rs`, `elab/`: index arguments on a declared type, checked at use.
- `crates/musa-calculus/src/convert.rs`: the one hook.
- `crates/musa-compiler/src/lower/types.rs`: surface `T(i)` lowered to an indexed `Raw` type.
- Law suites: `decide` on the fragment, including refusals; erasure — a term's read-back carries no index; conversion
  refusing `Row(12)` against `Row(24)`; a refinement constructor carrying its index; the budget refusing a pathological
  index expression rather than hanging.

## Check

```sh
cargo nextest run --workspace
cargo nextest run --run-ignored all
cargo clippy --workspace --all-targets -- -D warnings
make fmt-check && make docs-check
```

Byte-identity is the load-bearing check: every `insta` snapshot, `tests/fixtures/elaboration-compatibility.txt`, the
`musa-events` pinned digests, and `apps/musa-desktop/ui/fixtures/` are unchanged. An index that reaches a stored file is
a defect in erasure.

## Stop

- No index-level functions, no existentials, no proof obligations discharged by search, no `Vec`.
- No inductive-family indices. `family/` is not touched.
- No solver in the audio, notation, or project crates. `musa-calculus` owns it and nothing re-derives it.
- Do not use the index to re-admit anything note 51 §7 keeps deleted.

Commit as `Build the index stratum: a solver beside the checker, and erased before it is stored`.
