---
id: 142i
slug: core-re-checker
status: pending
depends_on: [142h]
phase: 3
---

# Build the Re-Checker the Crate Already Cites

## Task

Prompt [134](134-bidirectional-elaboration.md) specified an independent re-checker for elaborated terms, called it _"the
single most valuable invariant in the whole crate"_, and listed it as a deliverable. It does not exist. Four sites in
`musa-calculus` speak of it as though it does:

```sh
$ grep -rn 'recheck' crates/musa-calculus/src/ | sed 's/:.*\(recheck[a-z_:]*\)/ -> \1/'
crates/musa-calculus/src/case.rs:1049 -> recheck::universe_of
crates/musa-calculus/src/program.rs:84 -> recheck
crates/musa-calculus/src/dictionary.rs:926 -> recheck
crates/musa-calculus/src/base.rs:436 -> recheck
$ git log --oneline -- crates/musa-calculus/src/recheck.rs
$
```

`case.rs:1049` is an intra-doc link to a module that has never existed in this repository's history.
`operator_laws.rs:154` cites a `trait_laws.rs` test named `the_re_checker_accepts_what_dictionary_elaboration_produced`
that is not in that file. The tests that are named `..._re_checks_in_the_core` call `musa_calculus::check` — the
elaborator checking its own work, which is not evidence about anything.

So there is one typing judgment in the crate, the elaborator's, and nothing validates the terms it emits. Prompt
[148](148-core-conformance.md) discharges an obligation matrix that assumes the re-checker exists. Build it, at the path
the comments already name.

## Read

- Prompt [134](134-bidirectional-elaboration.md)'s sections on the re-checker — the specification this prompt
  discharges, unchanged. Read it for _why_ it was called the most valuable invariant, not only for what it asks.
- Prompt [148](148-core-conformance.md), which already writes "the elaborator's conversion and the re-checker's" among
  the pairs that must answer the same question.
- 142h's `kernel/`–`elaboration/` boundary and its law suite. The re-checker's whole value is that it lives on the
  kernel side and _cannot_ call the elaborator; before 142h nothing enforced that and a re-checker would have been
  theatre.
- [`crates/musa-calculus/src/lib.rs`](../../../crates/musa-calculus/src/lib.rs)'s **"One evaluator"** invariant, which
  fixes the one thing the re-checker must _share_ rather than duplicate. See _What is duplicated and what is not_.
- `docs/rules/language/02-core-calculus.md` §§1–3 — the typing rules being implemented a second time. The re-checker is
  written from the specification, not from `elab/`, or it inherits whatever `elab/` gets wrong.
- Peyton Jones ch. 8 and ch. 9 on a type checker written out. The re-checker is the small, direct, syntax-driven checker
  of that shape; the elaborator is not, and the difference is the point.

## Design

**It types a `Term`, not a `Raw`.** `kernel::recheck::type_of(cx, term) -> Result<Term, CoreError>` and its companion
`universe_of` for types — the path `case.rs:1049` already links to. It sees a finished core term: de Bruijn indices,
recursors, record projections, `let`. There are no patterns, no coverage, no traits, no holes, and no indices, because
elaboration compiled all of them away and 142f made the erasure structural. A re-checker that needed to know what a
`match` was would be proof that `match` had not been compiled away.

**What is duplicated and what is not.** The typing rules are implemented a second time, from `02-core-calculus.md`
rather than from `elab/` — that is the entire evidentiary value, and copying the elaborator's traversal would destroy
it. Conversion is **shared**, deliberately: `lib.rs`'s "One evaluator" invariant says a second decider would be a second
semantics obliged to agree with the first by a law nobody could state. So the re-checker asks `convert` the same
question the elaborator asks, and differs everywhere else.

**A negative control, or the check proves nothing.** A re-checker that accepts everything passes every test in the
suite. The prompt therefore delivers hand-built ill-typed terms — a projection of a field from a function, an
application at the wrong domain, a recursor at the wrong motive — and the law is that each is _rejected_, with the
`Malformed` variant naming what did not fit. `lib.rs` already says a malformed term is a caller defect reported rather
than panicked on; this is what reports it.

**Where it runs.** Every law suite in `crates/musa-calculus/tests/suite/` that elaborates a term re-checks the result,
through one shared helper rather than a call written out per test. The tests already _named_ for it —
`a_compiled_match_re_checks_in_the_core`, `a_call_of_a_method_re_checks_in_the_core` — start doing what their names say,
and `trait_laws.rs` gains the dictionary law `operator_laws.rs:154` cites.

**The dangling links are repaired by being made true.** `case.rs:1049`, `program.rs:84`, `dictionary.rs:926`, and
`base.rs:436` keep their prose; the module they name now exists.

## Target

- `crates/musa-calculus/src/kernel/recheck.rs`: `type_of` and `universe_of`, written from `02-core-calculus.md`, with
  the doc comment stating what it deliberately does not know.
- `crates/musa-calculus/tests/suite/` — a shared re-check helper, used by every suite that elaborates; the
  `..._re_checks_in_the_core` tests re-checking rather than re-elaborating; and
  `trait_laws.rs::the_re_checker_accepts_what_dictionary_elaboration_produced`, which `operator_laws.rs` has been
  citing.
- `crates/musa-calculus/tests/suite/recheck_laws.rs`: the negative controls — at least the three ill-typed shapes named
  above, each rejected with the `Malformed` case that names the mismatch.
- The four doc comments' intra-doc links resolving.

## Check

```sh
cargo nextest run --workspace
cargo nextest run -p musa-calculus
cargo clippy --workspace --all-targets -- -D warnings
make fmt-check && make docs-check
```

And the two properties that make it evidence rather than decoration:

- **Every term the calculus law suites elaborate re-checks**, including every compiled `match`, every dictionary, every
  recursor application, and every term in `tests/fixtures/elaboration-compatibility.txt`.
- **Each negative control is rejected.** A run with the re-checker's body replaced by `Ok(ty.clone())` must fail
  `recheck_laws.rs`; a check that cannot fail is not one.

## Stop

- **No new typing rule, and no rule relaxed.** If the re-checker rejects something the elaborator accepts, that is a
  finding to report — it is exactly what the prompt was built to find — and not a licence to widen either side.
- **No second conversion relation.** Shared, for the reason `lib.rs` gives.
- **No change to any verdict the elaborator reaches today**, and no change to any diagnostic.
- **No repair of 148.** Its obligation matrix stays as written; this prompt makes it satisfiable.
- Not a public facade item. The re-checker serves the law suites and 148; `lib.rs` gains nothing until a caller outside
  the crate needs it.
- No re-checking inside `check`/`infer` on the normal path. A compiler that type-checks everything twice in production
  has bought its confidence with the author's time.
- No index re-checking. Indices are erased before a term reaches here, which 142f settled.

Commit as `Build the re-checker the crate already cites`.
