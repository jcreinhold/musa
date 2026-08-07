---
id: 46
slug: kernel-term-spec
status: pending
depends_on: [43]
phase: 3
---

# Specify the Kernel Term Calculus

## Task

Specify — in `docs/kernel/`, before any code — the small term language whose meanings are the timelines the kernel
already has: constructors, `let` for sharing, an evaluation relation into `(d, E)`, and the soundness theorems that make
evaluation and normalization the same thing. This turns the kernel from an algebra of values into a calculus with a
syntax, which is what a second implementation, an interchange file, and sharing-without-expansion all require.

Specification only. Prompt 47 implements it; prompt 48 gives it a text form; prompt 49 makes elaboration produce it.
Marked candidate until prompt 48 graduates it, exactly as prompts 08–12 handled the kernel itself.

## Read

- `docs/kernel/01-grammar.md` — it already sketches an interchange grammar with un-normalized expressions and named
  compositions. This prompt makes it precise and adds what it lacks; it is a repair of that document plus a new one, not
  a fresh design.
- `docs/kernel/03-denotational-semantics.md` (D0–D7 — the semantic domain the terms denote),
  `05-normalization.md` (N1–N6 — normalization, equality, serialization, hash),
  `08-open-questions.md` **Q6** (no parser until a second producer/consumer exists — prompt 48 is that consumer, so this
  block is the trigger firing, not an end-run around it) and **Q1/Q5** (patterns and recursion stay out).
- Course correction §5 (named references for sharing are part of the initial basis), §16 (no monadic `join`), §18 (the
  kernel is finite), §32 (do not prematurely decide), §34 (semantic necessity).
- `docs/kernel/00-purpose.md`: "Not a general-purpose programming language (no recursion, no general computation)". The
  calculus specified here must not violate that line; see the scope rule below.

## Design

### The scope rule, stated first because it is what keeps this honest

A calculus is worth adding **only** for what values cannot express: *sharing* (naming a timeline used more than once),
*deferred observation* (asking about a window without building the whole), and *interchange* (a syntax a second
implementation can read). It is not an invitation to binders-in-general, functions, recursion, or computation. Terms are
finite, closed under the existing operations, and every term has a denotation that already exists in `03`. If a proposed
form does not serve one of those three purposes, it does not go in.

### The terms

```text
t ::= timeline d { (s, e, a)* }     % literal
    | seq t₁ … tₙ                   % D2
    | over t₁ … tₙ                  % D3
    | shift d t                     % translation by d ≥ 0 — see below
    | scale r t                     % D5, r ∈ ℚ>0
    | restrict [i, j) t             % D6
    | map f t                       % D7 — see below
    | let x = t in u                % sharing; x scopes over u
    | x                             % reference
```

Two of these need justification, and the specification must give it or drop them:

- **`shift`** is not one of D1–D7. It is derivable — `shift d t = seq (timeline d {}) t` — so under §34 it is a
  convenience, not a primitive. Specify it as **derived sugar with a stated expansion**, because a canon's delay is the
  single most common thing an interchange file will express and writing it as a sequence with an empty timeline obscures
  intent. Sugar with an expansion costs nothing semantically; a new primitive would.
- **`map f`** cannot be a term unless `f` is nameable, and naming functions is the door to general computation. Specify
  it as **not a term**: payload mapping happens above the calculus, during elaboration, exactly as it does today.
  Terms carry already-mapped payloads. Record this as the deliberate asymmetry it is — the calculus is one of *temporal
  structure*, and payload transformation is not temporal structure.

### The semantics

Give the evaluation relation `ρ ⊢ t ⇓ (d, E)` with an environment `ρ` mapping names to values, one rule per form. Then
state the theorems that make this a calculus rather than a serialization format:

- **T1 — homomorphism.** `⟦seq t u⟧ = ⟦t⟧ ; ⟦u⟧`, and likewise for `over`, `scale`, `restrict`. The kernel's L1–L18 are
  then laws about terms too, by transport.
- **T2 — `let` is transparent.** `⟦let x = t in u⟧ = ⟦u[t/x]⟧`. Sharing changes cost, never meaning.
- **T3 — evaluation is normalization.** Evaluating a term and normalizing the result (N1) yields the canonical form;
  two terms are semantically equal exactly when their evaluations have equal canonical forms (N4). This is the theorem
  that stops the term language from becoming a second notion of equality.
- **T4 — totality.** Every well-formed closed term evaluates; there is no diverging term, because there is no recursion
  and every constructor is finite. State the well-formedness rules (`02-static-semantics.md` gains a section: scoping,
  `restrict` windows within extent, `scale` positive, no free variables) so "well-formed" is checkable.
- **T5 — observation commutes with sharing.** `restrict I (let x = t in u) = let x = t in restrict I u`. This is the
  law that makes deferred observation *sound*, and therefore the one prompt 50 depends on; if it cannot be stated
  cleanly, prompt 50 has no foundation and should be struck.

Each theorem names the property test that will implement it at prompt 47, in the style `04-algebraic-laws.md` already
uses.

### What is deliberately absent

Record, with reasons, in the new document: no functions or application; no recursion or fixpoints (Q5); no `Pattern`
type (Q1); no `join` (§16); no conditionals; no arithmetic on terms. And no *reverse* — prompt 34 established that
retrograde needs no primitive, and a calculus is not a reason to revisit that.

## Target

- `docs/kernel/10-term-calculus.md` (new): scope rule, grammar, evaluation relation, T1–T5, the absent list, status
  banner **candidate**.
- `docs/kernel/01-grammar.md`: repaired to be the surface syntax *of these terms*, with `let`, `shift`, and the removal
  of anything it promises that the calculus does not have; the split between "grammar" (prompt 48's text form) and
  "calculus" (this document) stated in both.
- `docs/kernel/02-static-semantics.md`: well-formedness for terms.
- `docs/kernel/08-open-questions.md`: Q6 restated — the trigger has fired, and here is what fires it.
- `docs/kernel/00-purpose.md`: one paragraph placing the calculus under the existing "not a general-purpose programming
  language" line, so the two documents cannot be read as disagreeing.

## Check

```sh
grep -n "Status: candidate" docs/kernel/10-term-calculus.md
# every theorem names its future test; every form has a denotation:
grep -c "Test:" docs/kernel/10-term-calculus.md    # ≥ 5
cargo fmt --check
```

No code changes; the workspace build is untouched.

Commit as `Specify the kernel term calculus`.

## Stop

- No implementation, no `Term` type, no parser. Prompts 47–48.
- No new *semantic* operation. Every term denotes something D1–D7 already define; if one does not, it is out of scope
  and the specification says so.
- Do not settle Q1, Q2, or Q5 in passing. A calculus makes patterns look tractable; they are still open. Q4 is prompt
  45's, and its answer is a payload *value* — cite it if it has landed, do not give the calculus a curve form.
- Do not specify a binary format, a version negotiation scheme, or a schema. One text form, prompt 48.
