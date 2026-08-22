---
id: 153
slug: metavariables-and-unification
status: done
depends_on: [152]
phase: 3
---

# Give the Core Real Metavariables and Pattern Unification

## Task

The calculus took Idris2's `checkRtoL` and dropped its fallback. Every construct that wants a real solution — an
implicit argument, index unification in `match`, a metavariable that outlives one call — is unavailable because the
mechanism underneath it is not there. Put it there: a `Meta` that carries the scope it may mention, a constraint queue
with postpone and retry, and pattern-fragment unification.

**Where the missing mechanism actually is, corrected against the code.** The first-order matcher is
`elaboration/convert.rs`'s `Conversion::assignment`: an *unapplied* unsolved meta takes the value it met, an *applied*
one is rigid, and there is no third answer. That is what this prompt replaces. `elab/spine.rs`'s two-pass walk — the
deferral of each checking-only argument behind a placeholder meta — is *not* the same mechanism, and it stays:
`01-surface.md` §1.5 requires it by name. `Design` below states the rule and the evidence.

**This is the step where dependent-type implementations go wrong, and it is the one to port rather than reinvent.**

## Read

- `/Users/jcreinhold/Code/Idris2/src/Core/Unify.idr` in full — `unifyD`, `postpone`, `retryGuess`, `patternEnv`, and the
  invertibility test. This prompt is that file, in Rust, minus quantities and laziness.
- `/Users/jcreinhold/Code/Idris2/tests/` — the unification corpus. Port it here, not at the end.
- `docs/rules/language/02-core-calculus.md` §2.1 — already written for this prompt, including the paragraph on what it
  replaces and why. This prompt implements a section that exists; it does not amend one.
- `crates/musa-calculus/src/elaboration/convert.rs` — `Conversion::assignment` is the first-order matcher, and
  `LevelEquation` is the one thing the file already postpones, with the finish point a queue needs already in place.
- `crates/musa-calculus/src/elaboration/elab/spine.rs` — `Waiting`, `Slot::Deferred` and the second pass, which this
  prompt keeps; `advance`, `given` and `metas` are untouched.
- `docs/rules/language/01-surface.md` §1.5 — the method rule, and the sentence that decides this: "`02-core-calculus.md`
  §2.1's two-pass spine is what now makes a receiver's type known in the cases that used to need it — an argument the
  walk defers is checked after the arguments that decide it, so `applied(fn (p) { p.act(P8) }, c4)` resolves `.act` at
  `Pitch`." A method resolves by exact receiver in one step, and a receiver at an unknown is refused rather than
  postponed. Read together with `elaboration/elab/check.rs`'s `carried`, the host's index-acceptance rule, which reads
  two *literal* indices and is the same shape.
- `crates/musa-compiler/tests/suite/argument_order_laws.rs` and `crates/musa-compiler/tests/suite/quotation_laws.rs` —
  the evidence. Deleting the deferral turns 57 of `musa-compiler`'s laws red, and these two name the mechanism:
  `an_un_annotated_lambda_is_typed_by_an_argument_written_after_it` fails as `MethodOnVariable`, and
  `a_spliced_value_arrives_where_the_splice_stood` fails as a `TokenTree`/`Expr` mismatch.
- `crates/musa-calculus/src/kernel/meta.rs` — `Meta` already carries a closed type and a write-once solution; it gains
  the scope, and `MetaSource` stays as the diagnostic's noun phrase.

## Design

**Miller pattern fragment, and the fallback is postponement.** A constraint `?m x₁ … xₙ ≟ t` is solved directly when the
`xᵢ` are distinct bound variables — the pattern fragment, where the solution is unique. Everything else is *postponed*,
not guessed and not refused: a queue entry retried when a metavariable it mentions is solved. Refusing outside the
fragment is what the current first-order matcher effectively does, and it is why implicits cannot work.

**A metavariable carries its context.** `Meta { source, ty, scope }` — the spine of variables it may depend on — so a
solution can be checked to mention nothing outside it. The occurs check and the scope check are the same walk.

**Solutions live in a table, not in the term.** A solved metavariable is not substituted through the program; `eval`
looks it up. This keeps the invariant the crate already has — **no substitution function anywhere** — and it is why
`zonk` exists as a separate pass rather than as a side effect of solving.

**Three outcomes, not two.** *Solved*, *definitely not unifiable*, and *blocked*. The third is a real answer and the
queue is where it lives; collapsing it into the second is the classic bug that makes elaboration order-dependent. Prompt
150 already assigned these variants to `CoreError`.

**The two-pass spine survives the queue, and a governing document says so.** §2.1 says a checking-only argument at an
undetermined slot "is checked after the rest of the spine has constrained its slot", and adds that with a real
constraint queue this "needs no special discipline" — the two-pass walk being "a consequence here rather than a
mechanism". The first sentence is normative and the deferral is exactly what satisfies it. The second is explanatory,
and it is true of everything a *comparison* can carry and false of the thing that actually breaks. `01-surface.md` §1.5
names the breakage in advance: a method resolves by exact receiver in one step, a receiver still undetermined after the
spine walk is *refused* rather than postponed, and "§2.1's two-pass spine is what now makes a receiver's type known in
the cases that used to need it". So the deferral is not an artefact of the missing queue — it is the mechanism §1.5
depends on, and deleting it would break a rule this prompt may not touch.

The measurement agrees. Deleting `Waiting`, `Slot::Deferred` and the second pass and postponing the arguments'
constraints instead turns **57** of `musa-compiler`'s laws red, and two of them name the reason exactly:
`argument_order_laws::an_un_annotated_lambda_is_typed_by_an_argument_written_after_it` fails as `MethodOnVariable` —
"`.act` needs a receiver whose type is a declared type" — and
`quotation_laws::a_spliced_value_arrives_where_the_splice_stood` fails as a `TokenTree`/`Expr` mismatch at the host's
index-acceptance rule. Neither is a constraint that a later solution could discharge: "no method" is a name that did not
resolve, and the index rule reads two literal indices or does not apply. What has to wait is the argument's
**elaboration**, not its constraints. Nothing in `docs/rules/language/` changes: §2.1's normative sentence is met, §1.5
is obeyed, and §2.1's explanatory sentence is a claim about mechanism that this prompt's evidence corrects here.

One wording in §1.5 goes stale and is deliberately left alone: "Nothing is postponed: postponement was deleted with the
rest of the constraint machinery" describes the state before this prompt, and this prompt restores postponement for
*comparisons*. Its normative content — a method call at an undetermined receiver is refused at the call — is exactly
what the code still does. Correcting the parenthetical is an amendment to a governing document and belongs to whoever
opens `01-surface.md` next, not to this prompt, whose **Stop** forbids it.

**What is deliberately not implemented.** No higher-order pattern unification beyond the Miller fragment. No constraint
solving across declaration boundaries. No `retryGuess` for `Delay`, which musa has no constructor for.

**The budget meets a new client, and §4 already answered which metric.** Conversion work is charged as evaluation steps
and quoted nodes; a retry re-runs the comparison and so charges again, *n* retries costing *n* times. So `budget.rs`
gains nothing and the deliverable is the law: a queue that retries forever exhausts `Steps` rather than hanging.

**Where the code goes, given the boundary 148 drew.** The kernel may not name `Refusal` or `ElabError`, and `convert.rs`
is where a mismatch acquires its path, its `whole`, and its span. So `kernel/unify.rs` owns what is genuinely
kernel-level and answers in `CoreError` — the pattern-fragment test, the occurs and scope checks, the abstraction that
turns `t` into `?m`'s solution, and the queue's entries — while `convert.rs` keeps the type-directed walk and the
diagnostics and becomes the queue's driver. Two files because they answer to two readers, which is prompt 150's own
test.

## Target

- `crates/musa-calculus/src/kernel/meta.rs`: `Meta` carrying the scope it may mention beside its type and solution.
- `crates/musa-calculus/src/kernel/unify.rs`: the pattern-fragment test, the occurs and scope checks, the abstraction
  that builds a solution, and the postponed-constraint queue — all of it in `CoreError`.
- `crates/musa-calculus/src/elaboration/convert.rs`: `assignment` replaced by the three-outcome call, and the retry loop
  that drains the queue when a solution arrives.
- `crates/musa-calculus/src/elaboration/elab/spine.rs`: unchanged except for the paragraph recording why the two-pass
  walk survives a real queue, citing `01-surface.md` §1.5 and the two laws above — written where the next reader of the
  walk will meet it.
- `crates/musa-calculus/src/elaboration/elab/zonk.rs`: solutions read back at the *meta's* scope depth rather than at
  the occurrence's, which is what carrying the scope makes possible.
- `crates/musa-calculus/tests/suite/unify_laws.rs`: the ported corpus, plus the three-outcome law and the budget law.

## Check

```sh
cargo nextest run --workspace
cargo nextest run --workspace --run-ignored all
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
! grep -rn 'fn subst' crates/musa-calculus/src | grep -v 'SortVar'
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
```

The `grep` is not decoration: a substitution function appearing here means solutions were pushed into terms, which
breaks the NbE presentation prompt 144 specified and makes `quote` no longer the only reader.

**Why it filters on `SortVar`, corrected against the code.** Prompt 152 landed `Sort::substitute`,
`Term::substitute_levels` and `substitute_fields` — level generalization, which replaces universe *level variables* in a
term's sorts and never replaces a variable by a term. It is not what this check protects: `eval` still looks a solution
up, and `quote` is still the only reader. The bare grep matches those three by name and would fail at prompt 152's own
commit, so it is narrowed to what it always meant. Every `fn subst…` in the crate must be a level substitution, which is
what naming `SortVar` in its signature says; a substitution of terms for variables would not, and would still fail
here.

**The re-checker's obligation for this prompt, and it is the important one.** `Checked::try_from` already rejects
unsolved metavariables — prompt 149 built the newtype for this line, so the caller it was waiting for is the queue's
drain, not a new rejection. What is owed here is the second, independent check: **a solution must mention nothing
outside its metavariable's scope, verified apart from the check made at solving time.** A negative control that solves a
metavariable with an out-of-scope variable is part of the deliverable. **Without this, this prompt's failure mode is
silent**, which is why 149 comes first.

Commit as `Give the core real metavariables and pattern unification`.

## Stop

- No *new* implicit insertion — that is 154, and separating them is what makes this prompt reviewable against
  `Unify.idr`. `advance`'s existing insertion for `Filling::Parameter` stays exactly as it is; 154 is where `Filling`
  gets its written and named forms.
- No change to `docs/rules/language/`. §2.1 specifies this prompt already; a repair that needed to edit it would be
  saying the specification is wrong, which is an amendment and not a step in this prompt.
- No case trees, no index unification in patterns. 155 and 156.
- No elaborator reflection, no `Elab` monad, no reification. Musa has one macro system and it is 160's.
