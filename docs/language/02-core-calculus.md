# Total elaboration core

The core is a monomorphic, pure, call-by-value simply typed lambda calculus with finite data and structural
eliminators. Surface conveniences elaborate into this core before evaluation. It is deliberately more expressive than
the kernel and deliberately less expressive than a general-purpose programming language.

## 1. Syntax

Let base types `b` include the finite and exact musical domains named in `01-surface.md`. Core types are:

```text
τ ::= b | unit | bool | nat | ratio | τ × τ | option τ | list τ | τ → τ | music
```

`declaration κ`, `module`, `piece`, `part`, `voice`, `Term[A]`, `Timeline[A]`, `Signal`, and DSP nodes are not value
types. `music` is abstract: user code has constructors and controlled transforms but no representation eliminator.

Terms are variables, literals, products/projections, constructors, lambdas, application, non-recursive `let`,
conditionals, finite primitive operations, and these eliminators:

```text
nat_fold  : A → (nat → A → A) → nat → A
list_fold : A → (X → A → A) → list X → A
option_fold : A → (X → A) → option X → A
```

The surface provides `map`, `filter`, bounded `range`, `repeat`, and row/chord traversals only as typed definitions or
compiler primitives reducible to these eliminators. A primitive must be total on its declared domain. Partial musical
operations return `option` or a diagnostic-bearing assertion witness.

There is no `fix`, recursive binding, while loop, exception, mutation, I/O, reflection, syntax value, dynamic cast, or
effect handler. Functions may be higher-order. User definitions are monomorphic and every parameter/result type is
written; there is no user generic-parameter syntax in this candidate. Compiler-owned `fold`/`map` primitives have
finite type schemes instantiated to monomorphic core operations during checking. Runtime/System-F polymorphism and
implicit let-generalization are absent; adding user parametric polymorphism requires a later specification.

## 2. Static semantics

The standard rules for products, sums, arrows, and immutable bindings apply. Representative rules are:

```text
Γ,x:σ ⊢ e : τ
────────────────────────────── Lam
Γ ⊢ (fn (x:σ) -> τ = e) : σ→τ

Γ ⊢ f : σ→τ    Γ ⊢ a : σ
────────────────────────────── App
Γ ⊢ f(a) : τ

Γ ⊢ z : A    Γ ⊢ s : nat→A→A    Γ ⊢ n : nat
──────────────────────────────────────────── FoldNat
Γ ⊢ nat_fold(z,s,n) : A
```

Literal constructors enforce refinements such as nonnegative `duration`, finite scale members, and row bijectivity.
These are constructor judgments returning a value or a located diagnostic; they do not introduce dependent types.
Named predicates used by `assert` return finite evidence that the assertion layer can report.

Only compiler-owned primitives may construct `music`. `map_note_pitches` accepts a total `pitch → pitch` and visits a
documented subset of musical payload positions; it does not reveal them as a list. A kernel quote has a dedicated
typing rule and cannot be encoded by string operations.

## 3. Dynamic semantics

Evaluation is deterministic call by value, left-to-right for arguments and source-order for finite folds. A closure is
`⟨λx.e, ρ⟩` with an immutable finite environment. Function application evaluates the function, then its argument, then
the body under the captured environment extended by the value. Music constructors produce contextual `Music` values;
they do not call `instantiate` during ordinary evaluation.

Exact values remain integers or reduced rationals. There is no floating-point base type in the core. Ordering of maps,
declarations, diagnostic witnesses, and provenance steps is source-stable, never hash-iteration order.

## 4. Resource acceptance

Strong normalization does not bound a terminating program to useful project size. Before evaluation, Musa therefore
computes or conservatively bounds:

- monomorphized definition count and closure environment size;
- natural/list fold work, including products induced by nested folds;
- generated music occurrence count and kernel-term binding count;
- template/module instantiation count and static dependency depth;
- quotation size after typed substitution.

A project whose proven upper bound exceeds the configured deterministic budget is rejected before partial output. If a
bound depends on a project literal, the diagnostic names that literal and the multiplicative path. Budgets are compile
options included in cache keys, never timeouts. Interactive cancellation is an external compiler operation, not a
language effect. Prompt 96 owns the bound analysis; prompts 118 and 135 benchmark the accepted envelope.

## 5. Metatheory

The implementation must mechanize as property tests and documented inductive arguments the following claims.

**Theorem 1 — preservation.** If `Γ ⊢ e : τ`, `ρ` realizes `Γ`, and `⟨e,ρ⟩ → ⟨e′,ρ′⟩`, then the residual
configuration has type `τ`.

*Argument.* Induct on the reduction rule. The only nonstandard cases are primitives, folds, music constructors, and
quotation. Each primitive supplies a typed total implementation; fold induction uses the accumulator premise; music
constructors return the abstract `music` type; the quotation checker supplies a typed `Term[ScoreFact]` hole.

**Theorem 2 — progress.** A closed well-typed core term is a value or takes one deterministic step.

*Argument.* The canonical-forms lemma for each type handles the ordinary cases. Refined constructors either already
produced a value or were rejected during checking. No partial primitive is admitted.

**Theorem 3 — strong normalization.** Every well-typed core term evaluates to a value.

*Argument.* STLC is strongly normalizing. Products/options/lists preserve the reducibility argument. `nat_fold` and
the other eliminators reduce on a structurally smaller finite constructor at every recursive semantic step. There is no
term former for general recursion. Monomorphization is a finite static graph checked before term evaluation.

**Corollary — deterministic total evaluation.** Progress, preservation, normalization, and syntax-directed evaluation
give one value of the declared type for every accepted closed expression.

**Theorem 4 — contextual closure.** If `m : music`, `ρ` is a well-formed `ElabEnv`, `p` is a valid exact placement,
and resource checking accepts the instance, then `instantiate(m,ρ,p)` returns a finite well-typed fragment and `close`
returns a closed `Term[ScoreFact]`.

This theorem is not inherited from STLC: prompts 97–98 must prove it by induction over the private music constructors,
including compatible binding-environment union, placement checks, quotation substitution, and provenance attachment.

## 6. Implementation boundary

`Type`, `Value`, `Closure`, `Music`, evaluator environments, theory representations, monomorphization tables, and
resource proofs remain private to `musa-compiler`. Passes may expose narrow internal queries, but no single-implementor
public trait or pass-through facade is added. The stable public result remains the compilation/snapshot contract.
