---
id: 141e
slug: compiler-registry
status: pending
depends_on: [141b, 141c, 141d]
phase: 3
---

# Say What the Compiler Owns, in the Core's Own Terms

## Task

Prompts 141b, 141c, and 141d built the mechanism `02-core-calculus.md` §5.8 describes: base types, δ-builtins over
finite data, and structural eliminators. Nothing fills it. `crates/musa-compiler/src/core.rs`'s `BUILTIN_OWNERSHIP` is
still a table of 117 entries written against the *old* checker's `Type`, and its 900-line evaluator is written against
the old checker's `Value`.

Fill the mechanism. Declare the families the compiler's own signatures mention, register the inert musical domains as
base types, and re-express all 117 entries as `musa_core::Builtin`s with rules over `Datum`. Nothing calls the result:
this prompt is the table, proved by its own suite, exactly as 141b–141d were the mechanism proved by theirs. Prompt 142
is still the one cutover.

## Read

- [`141b`](141b-base-types-and-builtins.md), [`141c`](141c-structural-eliminators.md), and
  [`141d`](141d-finite-constructor-builtins.md) — the three halves of the mechanism this prompt fills. `Base` and its
  `Payload`, `Builtin::new` with a `Rule` over `Datum`, `Builtin::structural` with a `Rewrite` over `Term`, and
  `Registry::new`'s registration checks.
- `crates/musa-compiler/src/core.rs`: `Base`, `Shape`, `Family`, `Eliminator`, `SyntaxOp`, `PhaseFamily`, the
  `BUILTIN_OWNERSHIP` and `SYNTAX_OWNERSHIP` tables, and `eval_builtin` — the 117 signatures and the 900 lines of
  reduction that have to be said again in the core's terms. The signatures are already declarative and are reused rather
  than retyped; only the value plumbing changes.
- `crates/musa-compiler/src/prelude.rs` — the compiler's own `data` declarations and the context they live in, which is
  what a δ signature mentioning `Option` or `Result` is written against.
- [`../../rules/language/02-core-calculus.md`](../../rules/language/02-core-calculus.md) §1's type list and §5.8's four
  families. §1 is the authority on which musical types are declared families and which are base types, and on why `Nat`
  is inductive while `Ratio` is inert: `zero | succ` is well founded and ℚ has no least element to descend to.
- `crates/musa-core/tests/suite/base_laws.rs` — the shape a worked registry and its laws take, and the fixture pattern
  where the families are declared before the builtins that answer them.

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
because a type got a name, which is the direction the collapse in prompt 143 goes as well.

**`Bool` and `Nat` are declared, and the musical domains are registered.** A base type is inert: it contributes no
ι-rule, so two closed values of it are convertible exactly when the host says the payloads agree. That is right for a
`Pitch` and wrong for anything source pattern-matches on. §1 decides both by name — `Nat` is in the language *to be* the
inductive numeric type, and `Bool` is the same argument one constructor shorter — so `nat_fold` has something to fold
and `if` has something to split. `Ratio` stays inert for §1's own reason: no least element, so no recursor, so
arithmetic on it is δ.

**One `Payload` implementation, not eighteen.** Every inert domain is a Rust value that is `PartialEq` for `same`,
`Display` for `shown`, and `'static` for `as_any`. A single generic wrapper satisfies all three, so adding a domain is
adding a row rather than an impl block. `Key` and `Frame` gain the `Display` the others already have, because a
diagnostic that cannot print a value is a diagnostic with a hole in it.

**A rule reads a `Datum` and answers a `Datum`, and the music does not move.** The bodies in `eval_builtin` are already
thin: they unwrap a `Value`, call into `crate::pitch`, `crate::scale`, `crate::chord`, `crate::pc12`, and wrap the
answer. Only the unwrapping and the wrapping change. A rule that found itself reimplementing a domain operation would be
evidence that the operation was in the evaluator rather than in its module, and that is a finding to record rather than
a thing to write twice.

**Nothing calls this.** `musa-compiler` keeps its old checker, its old evaluator, and its old table for one more prompt.
Two paths existing side by side for the length of one prompt is the price of a cutover that can be reviewed; two paths
existing after prompt 142 is the defect the second-path audit looks for, and 142 is where the old one is deleted.

## Target

- `crates/musa-compiler/src/prelude.rs`: the `data` declarations the registry's signatures mention — `Bool`, `Nat`,
  `Option`, `List`, `Result`, `Coordinate`, `Cat`, and the row fault — with the context that holds them.
- `crates/musa-compiler/src/registry.rs`: the generic `Payload`, the base types with their kinds, the `Shape`-to-`Term`
  translation, all 117 builtins with their rules, and the assembled `musa_core::Registry`.
- `Shape::Product` deleted, and `Shape`'s remaining constructors each still used.
- `Display` for `Key` and `Frame`, in their own modules.
- Laws in `crates/musa-compiler/tests/suite/`: the registry builds; every `BUILTIN_OWNERSHIP` spelling is registered
  exactly once and no name is registered twice; every δ signature is finite data, checked by `Registry::new` itself;
  each rule agrees with the corresponding arm of the old evaluator on sampled inputs, which is what makes this a
  translation rather than a rewrite; and a rule that answers `Option` or `Result` answers a value the re-checker accepts
  at the signature's own result type.
- `docs/plan/code-map/` rows for `musa-compiler`.

## Check

```sh
cargo build --workspace
cargo nextest run -p musa-core -p musa-compiler
cargo clippy --all-targets -p musa-core -p musa-compiler -- -D warnings
cargo fmt --check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
python3 scripts/renumber-prompts.py audit
```

Commit as `Say what the compiler owns, in the core's own terms`.

## Stop

- No elaboration through `musa-core`, no surface change, no `.musa` file touched, and no old checking path deleted.
  Prompt 142 owns the cutover and owns it whole.
- No builtin added, removed, renamed, or merged. The table says the same 117 things it said before; prompt 143 is where
  it gets smaller.
- No musical type in `musa-core`. The core stays a leaf, and every name this prompt writes is registered from the
  compiler side.
- No second `Datum` arm and no widened core API. A signature this prompt cannot express is evidence about the signature.
- No performance work. A unary `Nat` is what §1 asks for; prompt 144 measures the finished checker.
