---
id: 47
slug: kernel-terms
status: done
depends_on: [45, 46]
phase: 3
---

# Implement the Term Calculus and Prove It Sound

## Task

Implement prompt 46's calculus inside `musa-kernel`: the `Term` type, the evaluator, well-formedness checking, and
property tests for T1–T5. The public surface added here is small and has exactly two callers, both named: prompt 48's
text form and prompt 49's elaboration. Nothing in the compiler changes.

## Read

- `docs/rules/kernel/10-term-calculus.md` (grammar, evaluation relation, theorems T1–T5, the absent list),
  `docs/rules/kernel/02-static-semantics.md` (well-formedness).
- `crates/musa-kernel/src/timeline.rs` and `tests/laws.rs` — the value algebra the evaluator produces and the style the
  new property tests follow.
- `docs/rules/kernel/00-purpose.md` and `docs/rules/kernel/00-purpose.md` — the calculus is structure, not computation.
- PoSD ch. 10 (define errors out of existence) for the well-formedness question below; the module-design rule "do not
  add an error case unless a caller can recover from it".

## Design

### Make ill-formed terms unrepresentable where that is cheap

`02-static-semantics.md` lists several ways a term can be ill-formed. Sort them before writing a checker:

- **Unrepresentable by construction** — `scale` takes a positive rational because its constructor rejects anything else
  (the existing `NonPositiveScale` pattern); a literal timeline's occurrences are bounds-checked by `timeline` as they
  are today. These need no check because there is no way to build the bad term.
- **A real error a caller can act on** — a free variable, or a `restrict` window outside the term's extent. These belong
  in one `check` that returns the first violation with enough detail to point at it, since prompt 48's parser is the
  caller that must report them against source text.
- **Not an error at all** — a `restrict` window that merely exceeds the extent is clamped by intersection (prompt 37
  made observation total). Do not invent a diagnostic for it.

Evaluation therefore takes a checked term and cannot fail. `evaluate(&Term<A>) -> Timeline<A>` with no `Result` is the
goal; if the design cannot reach it, say which case forced a `Result` and why it could not be pushed into construction.

### Sharing is real, not sugar

`let` must not be implemented by substitution-then-evaluate, or the whole point — that `repeat 200 { … }` is one
evaluated body — is lost. The environment binds a name to an **evaluated value**, evaluated once, and references clone
or share it. Since a `Timeline<A>` is a `Vec` of occurrences, "share" means the evaluator must decide between `Rc` and
cloning. Decide on evidence: prompt 49 will produce terms with heavy reuse, so measure both there. For this prompt,
clone, and leave a comment naming prompt 49 as the measurement that may change it. Do not build an `Rc`-threaded
evaluator speculatively.

T2 (`let` is transparent) is what licenses either choice; it is the first property test to write.

### The surface

```rust
pub enum Term<A> { /* literal, seq, over, shift, scale, restrict, let, var */ }

impl<A> Term<A> {
    /// The first well-formedness violation, or `Ok(())` (02-static-semantics).
    pub fn check(&self) -> Result<(), KernelError>;
}

/// Evaluate a checked term to its denotation (10-term-calculus, T3–T4).
pub fn evaluate<A: Clone>(term: &Term<A>) -> Timeline<A>;
```

Payloads containing a `Progress` (prompt 45) need **no** term-level treatment: they ride inside `a`, the evaluator never
inspects them, and prompt 45's span-alone theorem is what makes that safe under `scale` and `shift`. If the evaluator
ever needs to look at a payload, something has gone wrong — say what and stop.

`Term` is public because prompts 48 and 49 build and consume it across crate boundaries. Nothing else becomes public:
the environment, the evaluation stack, and any internal sharing representation stay private. If `Term`'s variants must
be public for the parser to build them, they are; if a constructor function set is enough, prefer that — a public enum
is a public layout.

### The tests

One property test per theorem, in `crates/musa-kernel/tests/terms.rs`, using generators that build *terms* (a recursive
`proptest` strategy with a depth bound — terms are finite and so are the generators):

- T1 homomorphism, for `seq`, `over`, `scale`, `restrict`;
- T2 `let`-transparency, against a substituting reference evaluator written in the test file for exactly this purpose;
- T3 evaluation-is-normalization, and the equality correspondence;
- T4 totality — every generated well-formed term evaluates (an evaluation that panics or loops fails the test);
- T5 observation commutes with sharing.

Plus the transported laws: generate two terms, assert L1/L4/L5/L18 hold at the term level through evaluation. If any
fails, the calculus and the algebra disagree and the *specification* is wrong — repair prompt 46 first, then implement.

## Target

- `crates/musa-kernel/src/term.rs` (new), `lib.rs` facade updated with `Term`, `evaluate`.
- `crates/musa-kernel/src/error.rs`: whatever well-formedness cases survived the sorting above, and no more.
- `crates/musa-kernel/tests/terms.rs`: T1–T5 and the transported laws.
- `docs/rules/kernel/10-term-calculus.md`: each theorem's `Test:` line pointed at the real test name.
- `docs/rules/kernel/09-performance.md`: no row — nothing on the measured path changed.

## Repairs made while implementing

**`Term` is an opaque struct, not a public enum, and that is what made `evaluate` infallible.** The prompt offered the
choice and preferred constructors if they sufficed; they do. With `Term::seq`/`over`/`shift`/`scale` returning `Result`,
four of `02-static-semantics.md` K7's rules become unrepresentable — an empty composition, a non-positive factor, a
backwards delay, a disordered window — and payload uniformity is the type parameter. `check` is left with exactly the
two rules that are **not local to one node**, and both are about names.

**The one case that could not be pushed into construction is a free name**, because a reference is built before the
binder that encloses it: `Term::var("x")` exists before `Term::bind("x", …)` wraps it. `evaluate` still returns a
`Timeline` rather than a `Result` — an unchecked free name denotes the empty timeline `(0, ∅)`, and a debug build
asserts that `check` passed. Panicking in a library on a caller's mistake is worse than the total answer, and returning
`Result` from `evaluate` would put an error case on the path of every well-formed term to describe a term no checked
caller can hold. This is the case the prompt asked to be named.

**Shadowing is rejected, and that shaped the term generator.** K7 forbids it so substitution stays textual and T2 needs
no capture-avoidance apparatus. The recursive `proptest` strategy therefore names each binding after its depth (`x2`,
`x1`, …); the first version reused `x` at every level and every nested term was ill-formed — which is the rule catching
a real mistake on its first use rather than a test being bent to fit.

**T2's reference is the substituted term, written out, not a substituting evaluator.** The prompt asked for a reference
evaluator that expands `let` by substitution. `Term` is opaque, so substitution is not expressible from a test file —
but it does not need to be: the test builds `let x = v in seq(x, body, x)` and `seq(v, body, v)` directly and compares
their denotations, which is exactly `⟦let x = t in u⟧ = ⟦u[t/x]⟧` with the substitution done by hand. Same reference,
fewer moving parts, and no second evaluator to keep correct.

**`restrict` needed D6 materialized as a value, and it is private.** `Timeline::restrict` returns an `Observation`,
which is a view; E-Restrict must yield a timeline. The evaluator materializes it — visible occurrences, **whole** spans
preserved (§17), the observed extent kept so L16 still makes full-extent restriction the identity — in a private helper.
It is not a new public operation, because it is not a new meaning and nothing outside the evaluator wants one. The T1
test writes the same materialization independently so the `restrict` case compares two implementations rather than one
against itself.

**`shift` is expanded by the evaluator, not denoted.** `Form::Shift` exists so a parser and a printer can round-trip the
sugar, but E-Shift is not a rule: evaluation builds `seq (timeline d {}) t` and hands off. A dedicated test
(`shift_denotes_its_stated_expansion`) pins that, so the sugar cannot drift into a primitive.

**References clone; the comment names prompt 49.** As instructed. `Rc` was not introduced, and the place the decision
would be revisited is stated in `evaluate`'s doc comment rather than left to be rediscovered.

**Two documents were repaired.** `02-static-semantics.md` K7 now says which of its rules are checked and which are
unrepresentable — a list of rules that does not say where each is enforced invites a checker that re-validates what the
type system already guarantees. `10-term-calculus.md`'s theorem preamble moved from future tense to present, and gained
a short section naming the three tests beyond T1–T5 (the transported algebra, the sugar's expansion, the negative
cases).

## Check

```sh
cargo nextest run -p musa-kernel
cargo clippy --all-targets -p musa-kernel -- -D warnings
cargo fmt --check
cargo build --workspace   # the kernel's new surface breaks nothing
```

Commit as `Implement the kernel term calculus`.

## Stop

- No text syntax, no parser, no printer — prompt 48.
- No compiler changes — prompt 49.
- No `Rc`, arena, or interning without prompt 49's measurement.
- No `map`, no functions, no recursion, no `Pattern`. The absent list in `10-term-calculus.md` is normative.
- If a theorem will not go through, stop and repair the specification. Do not weaken a test to make it pass.
