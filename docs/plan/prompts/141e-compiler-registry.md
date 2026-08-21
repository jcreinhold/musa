---
id: 141e
slug: compiler-registry
status: done
depends_on: [141b, 141c, 141d]
phase: 3
---

# Say What the Compiler Owns, in the Core's Own Terms

## Task

Prompts 141b, 141c, and 141d built the mechanism `02-core-calculus.md` §5.8 describes: base types, δ-builtins over
finite data, and structural eliminators. Nothing fills it. `crates/musa-compiler/src/phase/mod.rs`'s `BUILTIN_OWNERSHIP`
is still a table of 117 entries written against the *old* checker's `Type`, and its 900-line evaluator is written
against the old checker's `Value`.

Fill the mechanism's δ half. Declare the families the compiler's own signatures mention, register the inert musical
domains as base types, and re-express every δ entry of both tables — 92 of `BUILTIN_OWNERSHIP`'s 117 and 14 of
`SYNTAX_OWNERSHIP`'s 17 — as `musa_calculus::Builtin`s with rules over `Datum`. The other 28 are named, counted, and
left, each for a reason the Design gives. Nothing calls the result: this prompt is the table, proved by its own suite,
exactly as 141b–141d were the mechanism proved by theirs. Prompt 142 is still the one cutover.

## Read

- [`141b`](141b-base-types-and-builtins.md), [`141c`](141c-structural-eliminators.md), and
  [`141d`](141d-finite-constructor-builtins.md) — the three halves of the mechanism this prompt fills. `Base` and its
  `Payload`, `Builtin::new` with a `Rule` over `Datum`, `Builtin::structural` with a `Rewrite` over `Term`, and
  `Registry::new`'s registration checks.
- `crates/musa-compiler/src/phase/mod.rs`: `Base`, `Shape`, `Family`, `Eliminator`, `SyntaxOp`, `PhaseFamily`, the
  `BUILTIN_OWNERSHIP` and `SYNTAX_OWNERSHIP` tables, and `eval_builtin` — the 117 signatures and the 900 lines of
  reduction that have to be said again in the core's terms. The signatures are already declarative and are reused rather
  than retyped; only the value plumbing changes.
- `crates/musa-compiler/src/prelude.rs` — the compiler's own `data` declarations and the context they live in, which is
  what a δ signature mentioning `Option` or `Result` is written against.
- [`../../rules/language/02-core-calculus.md`](../../rules/language/02-core-calculus.md) §1's type list and §5.8's four
  families. §1 is the authority on which musical types are declared families and which are base types, and on why `Nat`
  is inductive while `Ratio` is inert: `zero | succ` is well founded and ℚ has no least element to descend to.
- `crates/musa-calculus/tests/suite/base_laws.rs` — the shape a worked registry and its laws take, and the fixture
  pattern where the families are declared before the builtins that answer them.
- `crates/musa-calculus/src/base.rs`'s `check_finite_data` doc comment, which reserves the literal-index case in so many
  words, and `Registry::new`'s, which says why a structural target must be a base type. Both are why this prompt's scope
  is what it is rather than what it was.
- Peyton Jones ch. 2 §2.5.3 and §2.6: built-in functions are δ-conversion, and "constants and built-in functions, each
  of which require individual treatment". Ninety-two rules written out one at a time is what that sentence looks like in
  a real table; a scheme clever enough to generate them would be a second signature language beside `Shape`.

## Design

**The signature table is reused, not retyped.** `Shape` is already a declarative signature language with no arrow, and
`delta()` already checks storability where the table is written. What this prompt adds is one translation from `Shape`
to a core `Term`, read against the prelude's context. Retyping 117 signatures by hand would be 117 chances to write a
different type from the one the old checker enforced, and the compatibility oracle is what would find out.

**`Shape::Product` goes, and the one entry that used it names its domain.** `row12_of` answers
`Result<Row12, (List Nat, List Pc12)>`, and 141d's `Datum` has a literal arm and a constructor arm and no record arm —
so an anonymous pair is the one signature in the table a rule cannot write. The answer is not a third `Datum` arm; it is
that the pair was always a domain concept wearing a tuple. *Open Music Theory* `108-basics-of-twelve-tone-theory.md`
says what it is — the order positions whose pitch class already appeared, and the pitch classes the sequence never names
— so it is declared, named, and the shape constructor that only it used is deleted. That is `Shape` getting smaller
because a type got a name, which is the direction the collapse in prompt 164 goes as well.

**`Bool` and `Nat` are declared, and the musical domains are registered.** A base type is inert: it contributes no
ι-rule, so two closed values of it are convertible exactly when the host says the payloads agree. That is right for a
`Pitch` and wrong for anything source pattern-matches on. §1 decides both by name — `Nat` is in the language *to be* the
inductive numeric type, and `Bool` is the same argument one constructor shorter — so `nat_fold` has something to fold
and `if` has something to split. `Ratio` stays inert for §1's own reason: no least element, so no recursor, so
arithmetic on it is δ.

**A rule is a `fn` pointer, so every type it answers at must be buildable from nothing.** That is D3 working as designed
— a rule cannot capture the context — and it decides the shape of the three indexed base types. `duration_of` answers a
`Duration` and takes only a `Ratio`, so it cannot borrow the type from an argument and cannot look one up; it has to
*write* `Duration ⟨written⟩` itself. `Base::new` and `Literal::new` are public and need no context, and a declared
family's constructor is neither. So `Coordinate` and `Cat` are **base types whose values are literals**, not declared
families: an index is then a literal, a rule builds it with the same two calls it builds a `Pitch` with, and nothing in
the compiler needs a context to say what it answers. They lose nothing by it — neither is pattern-matched by source,
which is §1's own test for inertness.

**The one check this changes is 141d's, in the case 141d reserved.** `check_finite_data` refuses a literal in an index
position today, and says why: "a check that admits what nothing writes is a check nobody has read; the day something
wants one, it arrives with a caller and a law." This is that day. `Duration ⟨written⟩`, `Position ⟨written⟩`, and
`Syntax ⟨token-tree⟩` are the callers — the first two on 40 signatures, the third on 14 — and the law is that a literal
index is admitted only *under a registered base head*, so a declared family applied to a literal is still refused and
`Vec Nat 3` is still not a δ signature. That is a widening of one positive check inside the crate, not a widened API,
and it arrives with the callers 141d asked for.

**Twenty-eight entries are named and left, and that is not a shortfall.** `BUILTIN_OWNERSHIP` has 92 δ entries; the
other 25 belong to §5.8's other three families and none of them is registrable *here*. The eight collection eliminators
are 141c's own argued exclusion — `Nat`, `List`, and `Option` are declared, so `declare` already generated their
recursors, and `Registry::new` refuses a structural target that is not a base type precisely so that nothing registers a
second ι-rule for a type that has one. The eight track builtins and nine machine builtins need `EventTrack` and
`Machine`, and both are types prompt 142 reshapes when it absorbs 127e and deletes contextual `Music`; registering them
against the representation that is about to go would be writing the migration twice. Of `SYNTAX_OWNERSHIP`'s 17, the 14
builders are δ and are registered; the three folds are prompt 141f's, for the finding that prompt states.

**The finding the folds turn on, recorded here because this is where it was found.** `Rewrite` is
`fn(&Builtin, &Literal) -> Option<Term>`, so a rewrite names the other arguments by de Bruijn index and can build
nothing else. All three folds have a group branch typed `NodePath → Delimiter → List A → A`, so firing at a group node
means *building a list term*, which means naming `List.Cons` — and `Constant`'s fields are `pub(crate)`, so a rewrite
cannot. Two encodings were tried and both are worse than the problem: a λ standing where a `SyntaxStep` value belongs is
ill-typed at a base type, and a payload holding a de Bruijn index is meaningless the moment it leaves the spine it was
minted in. 141c's own Design proposed `fn(&[&Term]) -> Option<Term>`, which is the half that would have made this
writable; the narrowing happened in its implementation. 141f is where that is settled, with the three folds as its
callers.

**One `Payload` implementation, not eighteen.** Every inert domain is a Rust value that is `PartialEq` for `same`,
`Display` for `shown`, and `'static` for `as_any`. A single generic wrapper satisfies all three, so adding a domain is
adding a row rather than an impl block. `Key` and `Frame` gain the `Display` the others already have, because a
diagnostic that cannot print a value is a diagnostic with a hole in it.

**A rule reads a `Datum` and answers a `Datum`, and the music does not move.** The bodies in `eval_builtin` are already
thin: they unwrap a `Value`, call into `crate::pitch`, `crate::scale`, `crate::chord`, `crate::pc12`, and wrap the
answer. Only the unwrapping and the wrapping change. A rule that found itself reimplementing a domain operation would be
evidence that the operation was in the evaluator rather than in its module, and that is a finding to record rather than
a thing to write twice.

**Nothing calls this, and the two consequences are stated rather than worked around.** `musa-compiler` keeps its old
checker, its old evaluator, and its old table for one more prompt. Two paths existing side by side for the length of one
prompt is the price of a cutover that can be reviewed; two paths existing after prompt 142 is the defect the second-path
audit looks for, and 142 is where the old one is deleted. But "nothing calls this" is a claim the compiler checks:

- **The registry is unreachable in a non-test build**, so `dead_code` fires on the whole subtree. The right statement is
  `#[cfg_attr(not(test), expect(dead_code, …))]` on the two module declarations, and not an `allow`. An `expect` is
  itself checked — the day prompt 142 wires the elaborator, the expectation goes unfulfilled and the compiler says so,
  which is the opposite of an allow-list that silently outlives its reason.
- **The laws are unit tests, not integration tests.** `eval_builtin` is private to `crate::phase`, and the whole point
  of the agreement law is to run a rule and the old arm on the same input and compare. A test in
  `crates/musa-compiler/tests/suite/` links against the crate's public facade — `parse`, `compile`, `render_notation` —
  and can reach neither the old evaluator nor `owned()`. Making either public to test it would widen the facade for a
  test's convenience, which is the failure the crate's own narrow-facade rule exists to prevent. So the laws live beside
  what they are about, in `src/registry/laws.rs` under `#[cfg(test)]`, and prompt 142 moves whatever survives the
  cutover into the suite once there is a public path to it.

## Target

- `crates/musa-compiler/src/prelude.rs`: the `data` declarations the registry's signatures mention — `Bool`, `Nat`,
  `Option`, `List`, `Result`, and the row fault — with the context that holds them.
- `crates/musa-calculus/src/base.rs`: `check_finite_data` admits a literal index under a registered base head, with the
  refusal unchanged everywhere else, and `crates/musa-calculus/tests/suite/base_laws.rs` states both halves.
- `crates/musa-compiler/src/registry.rs`: the generic `Payload`, the base types with their kinds, the `Shape`-to-`Term`
  translation, the 92 δ entries of `BUILTIN_OWNERSHIP` and the 14 δ builders of `SYNTAX_OWNERSHIP` with their rules, and
  the assembled `musa_calculus::Registry`, with the rules themselves in `src/registry/rules.rs`. The two tables stay
  two, because §5.9 makes the phase registry separate, and the two walks over them stay two for the same reason.
- `crates/musa-compiler/src/lib.rs`: `mod prelude` and `mod registry` carrying
  `#[cfg_attr(not(test), expect(dead_code, …))]` with the reason naming prompt 142, so that "nothing calls this" is a
  claim the compiler retires rather than a comment.
- `Shape::Product` deleted, and `Shape`'s remaining constructors each still used.
- `Display` for `Key` and `Frame`, in their own modules.
- Laws in `crates/musa-compiler/src/registry/laws.rs`, under `#[cfg(test)]` because the agreement law needs the old
  evaluator and the old evaluator is private: the registry builds; every δ spelling of both tables is registered exactly
  once and no name is registered twice; the 28 entries that are *not* registered are exactly the eight collection
  eliminators, the eight track builtins, the nine machine builtins, and the three phase folds, counted from the tables
  themselves so the accounting cannot drift; every δ signature is finite data, checked by `Registry::new` itself; each
  rule agrees with the corresponding arm of the old evaluator on sampled inputs, which is what makes this a translation
  rather than a rewrite; and a rule that answers `Option` or `Result` answers a value the re-checker accepts at the
  signature's own result type.
- `docs/plan/code-map/` rows for `musa-compiler`.

## Check

```sh
cargo build --workspace
cargo nextest run -p musa-calculus -p musa-compiler
cargo clippy --all-targets -p musa-calculus -p musa-compiler -- -D warnings
cargo fmt --check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
python3 scripts/renumber-prompts.py audit
```

Commit as `Say what the compiler owns, in the core's own terms`.

## Stop

- No elaboration through `musa-calculus`, no surface change, no `.musa` file touched, and no old checking path deleted.
  Prompt 142 owns the cutover and owns it whole.
- No builtin added, removed, renamed, or merged. The tables say the same 117 and 17 things they said before; prompt 164
  is where they get smaller.
- No musical type in `musa-calculus`. The core stays a leaf, and every name this prompt writes is registered from the
  compiler side.
- No second `Datum` arm, and no change to `Rule`, `Rewrite`, `Builtin`, or `Registry`'s public surface. The one core
  change is the positive check 141d reserved, and it arrives with its callers. A signature this prompt still cannot
  express is evidence about the signature, and 141f is where the three that cannot are settled.
- No structural eliminator registered. The eight collection folds are 141c's argued exclusion and the three phase folds
  are 141f's; registering a fourth family's worth of stubs to make a count come out is the opposite of what the count is
  for.
- No performance work. A unary `Nat` is what §1 asks for; prompt 165 measures the finished checker.
