---
id: 47
slug: kernel-terms
status: pending
depends_on: [46]
phase: 3
---

# Implement the Term Calculus and Prove It Sound

## Task

Implement prompt 46's calculus inside `musa-kernel`: the `Term` type, the evaluator, well-formedness checking, and
property tests for T1–T5. The public surface added here is small and has exactly two callers, both named: prompt 48's
text form and prompt 49's elaboration. Nothing in the compiler changes.

## Read

- `docs/kernel/10-term-calculus.md` (grammar, evaluation relation, theorems T1–T5, the absent list),
  `docs/kernel/02-static-semantics.md` (well-formedness).
- `crates/musa-kernel/src/timeline.rs` and `tests/laws.rs` — the value algebra the evaluator produces and the style the
  new property tests follow.
- Course correction §34 and `docs/kernel/00-purpose.md` — the calculus is structure, not computation.
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
- `docs/kernel/10-term-calculus.md`: each theorem's `Test:` line pointed at the real test name.
- `docs/kernel/09-performance.md`: no row — nothing on the measured path changed.

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
