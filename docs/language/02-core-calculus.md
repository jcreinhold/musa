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

## 5. Prompt-95 fragment and metatheory

This section fixes the proof obligation already implemented by prompt 95. Options, lists, folds, primitive pitch
operations, and `music` extend the calculus later and require their own compatibility cases; they are not smuggled into
this theorem by an appeal to “standard STLC.” Let `b` range over `bool`, `nat`, `ratio`, `duration`, `pitch`, and
`interval`. The implemented fragment is:

```text
σ,τ ::= unit | b | (τ₁ × … × τₙ) | (τ₁,…,τₙ) → τ
e   ::= x | c | (e₁,…,eₙ) | λ(x₁:τ₁,…,xₙ:τₙ).e
      | e(e₁,…,eₙ) | let x:τ = e in e
v   ::= c | (v₁,…,vₙ) | λ(x₁:τ₁,…,xₙ:τₙ).e
```

There is no anonymous-function surface syntax. A checked named `fn` supplies the lambda, and an acyclic named `let`
graph supplies the lexical lets. Multi-argument arrows and applications are notation for the corresponding curried
STLC terms. A surface call with named arguments is permuted into parameter order; an omitted default is inserted in
that order and may refer only to earlier parameters. Thus defaults and argument names add no core reduction rule.
Products currently have introduction but no surface projection, which is a conservative sublanguage of the product
calculus.

### 5.1 Static judgments

Literal checking is a partial *compiler* judgment `token ⇝ c : b`: malformed or out-of-range text produces a located
diagnostic, while accepted constants are mathematical booleans, bounded natural representations, reduced exact
rationals, or already-validated musical values. It is not a partial term operation. The core typing rules are:

```text
x:τ ∈ Γ                         Γ ⊢ eᵢ : τᵢ  (all i)
──────── Var                    ───────────────────── Prod
Γ ⊢ x : τ                       Γ ⊢ (e₁,…,eₙ) : τ₁×…×τₙ

Γ,x₁:τ₁,…,xₙ:τₙ ⊢ e : τ        Γ ⊢ f : (τ₁,…,τₙ)→τ   Γ ⊢ aᵢ : τᵢ (all i)
──────────────────────── Lam    ───────────────────────────────────────── App
Γ ⊢ λ(x₁:τ₁,…,xₙ:τₙ).e         Γ ⊢ f(a₁,…,aₙ) : τ
    : (τ₁,…,τₙ)→τ

Γ ⊢ e₁ : σ    Γ,x:σ ⊢ e₂ : τ
────────────────────────────── Let
Γ ⊢ let x:σ = e₁ in e₂ : τ
```

Top-level signatures are collected before bodies are checked, so a later declaration may be referenced. From each
body the checker records an edge to every free named declaration. Acceptance requires the resulting finite graph to be
acyclic. A topological ordering `d₁,…,dₖ` then denotes the closed term
`let d₁=e₁ in … let dₖ=eₖ in (d₁,…,dₖ)`; an edge can only point to an earlier binding in this ordering. This gives
forward references lexical meaning without giving Musa recursive binding.

### 5.2 Small-step and big-step semantics

For the proof, capture-avoiding simultaneous substitution is written `e[v̄/x̄]`. Evaluation contexts enforce
left-to-right call by value:

```text
E ::= [] | (v₁,…,vᵢ₋₁,E,eᵢ₊₁,…,eₙ)
       | E(e₁,…,eₙ) | v(v₁,…,vᵢ₋₁,E,eᵢ₊₁,…,eₙ)
       | let x:τ = E in e

(λ(x̄:τ̄).e)(v̄)       → e[v̄/x̄]                       βᵥ
let x:σ = v in e       → e[v/x]                        Letᵥ
e → e′                 ⇒ E[e] → E[e′]                 Context
```

The implementation uses environments rather than copying syntax. Write `ρ ⊨ Γ` when `dom(ρ)=dom(Γ)` and each
`ρ(x)` is a closed value of type `Γ(x)`. Its big-step judgment is:

```text
ρ(x)=v
──────────── Name       ─────────── Const
ρ ⊢ x ⇓ v               ρ ⊢ c ⇓ c

ρ ⊢ eᵢ ⇓ vᵢ (left to right)
──────────────────────────── Product
ρ ⊢ (e₁,…,eₙ) ⇓ (v₁,…,vₙ)

──────────────────────────────────────── Closure
ρ ⊢ λ(x̄:τ̄).e ⇓ ⟨λ(x̄:τ̄).e,ρ⟩

ρ ⊢ f ⇓ ⟨λ(x̄:τ̄).e,ρc⟩   ρ ⊢ aᵢ ⇓ vᵢ (left to right)
ρc[x₁↦v₁,…,xₙ↦vₙ] ⊢ e ⇓ v
────────────────────────────────────────────────────────────────── Apply
ρ ⊢ f(a₁,…,aₙ) ⇓ v

ρ ⊢ e₁ ⇓ v₁    ρ[x↦v₁] ⊢ e₂ ⇓ v₂
──────────────────────────────────── Let
ρ ⊢ let x=e₁ in e₂ ⇓ v₂
```

Closure environments contain precisely the free named dependencies plus preceding default parameters. This is an
implementation optimization: by the environment-substitution lemma below, it has the same meaning as the small-step
rules.

### 5.3 Structural lemmas

**Weakening.** If `Γ ⊢ e : τ`, `x ∉ dom(Γ)`, and `Γ ⊆ Γ′`, then `Γ′ ⊢ e : τ`.

*Proof.* Induction on the typing derivation. `Var` follows from inclusion; all constructor premises use the induction
hypothesis. In `Lam` and `Let`, alpha-rename the bound variable away from the added names and apply the hypothesis under
the extended context. ∎

**Substitution.** If `Γ,x:σ ⊢ e : τ` and `Γ ⊢ v : σ`, then `Γ ⊢ e[v/x] : τ`.

*Proof.* Induction on the derivation of `Γ,x:σ ⊢ e : τ`. The variable case is either `x`, where the second premise is
the result, or another variable, where `Var` is unchanged. Products and applications follow componentwise. For a
lambda or let binder, alpha-rename it fresh, use weakening for `v` under the extended context, and apply the induction
hypothesis to the body. Simultaneous substitution follows by iterating this lemma over distinct parameters. ∎

**Environment substitution.** If `ρ ⊨ Γ` and `Γ ⊢ e : τ`, replacing every free `x` in `e` by `ρ(x)` yields a closed
term of type `τ`.

*Proof.* Repeated application of substitution in any order, since the substituted values are closed. ∎

**Canonical forms.** A closed value of base type is a constant of that base type; a closed value of product type is a
product of values of the component types; a closed value of arrow type is a lambda (equivalently, an environment
closure whose environment realizes the lambda’s free-variable context).

*Proof.* Inspect the value grammar, then invert its typing rule. `unit` is presently uninhabited by surface literals,
so its closed-value case is vacuous until a unit constructor is introduced. ∎

### 5.4 Safety and determinism

**Theorem 1 — preservation.** If `∅ ⊢ e : τ` and `e → e′`, then `∅ ⊢ e′ : τ`.

*Proof.* Induct on the reduction derivation. For `βᵥ`, invert `Lam` and `App` and apply simultaneous substitution. For
`Letᵥ`, invert `Let` and apply substitution. For `Context`, invert the typing rule at the context hole, apply the
induction hypothesis there, and rebuild the same derivation. Exact constants do not reduce. ∎

**Theorem 2 — progress.** If `∅ ⊢ e : τ`, then `e` is a value or some `e′` satisfies `e → e′`.

*Proof.* Induct on typing. Literals and lambdas are values. For a product, select the leftmost non-value premise or
conclude by the value grammar. For application, first advance the function, then the leftmost non-value argument; if
all are values, canonical forms makes the function a lambda and `βᵥ` applies. A let advances its bound expression or
uses `Letᵥ`. There are no stuck primitive operations in this fragment. ∎

**Theorem 3 — determinism.** If `e → e₁` and `e → e₂`, then `e₁=e₂`.

*Proof.* Every non-value has a unique decomposition `E[r]` into the leftmost evaluation context and either a `βᵥ` or
`Letᵥ` redex. Each redex has one contractum because capture-avoiding substitution is unique up to alpha-equivalence.
Therefore both reductions choose the same context and result. The syntax-directed big-step judgment is likewise a
partial function. ∎

### 5.5 Strong normalization

Let `SN` be the terms with no infinite reduction sequence. Define reducibility predicates on closed typed terms:

```text
R_b(t)                 iff t : b and t ∈ SN
R_unit(t)              iff t : unit and t ∈ SN
R_(τ₁×…×τₙ)(t)         iff t : τ₁×…×τₙ, t ∈ SN, and whenever
                            t →* (v₁,…,vₙ), each R_τᵢ(vᵢ)
R_((τ₁,…,τₙ)→τ)(t)     iff t : (τ₁,…,τₙ)→τ, t ∈ SN, and for every
                            R_τᵢ(aᵢ), R_τ(t(a₁,…,aₙ))
```

These candidates satisfy: (i) membership implies `SN`; (ii) they are closed under reduction; and (iii) a neutral term
whose immediate reducts are in the candidate is in the candidate. All three properties follow simultaneously by
induction on the type; the arrow case applies an arbitrary reducible argument tuple, and the product case uses the
unique product normal form.

**Fundamental lemma.** If `Γ ⊢ e : τ` and a closing substitution `γ` maps each `x:σ` in `Γ` to a term in `R_σ`, then
`eγ ∈ R_τ`.

*Proof.* Induct on the typing derivation. Variables use the hypothesis; constants are normal forms. Products use the
component induction hypotheses and the product candidate. For a lambda, take arbitrary reducible arguments; `βᵥ`
reduces its application to the body under the extended reducible substitution, so the induction hypothesis and
candidate closure give the result. Application follows directly from the arrow candidate. `let` is the lambda case
via `let x=e₁ in e₂ ≡ (λx.e₂)e₁`. ∎

**Theorem 4 — strong normalization.** Every well-typed term in the prompt-95 fragment is strongly normalizing.

*Proof.* Apply the fundamental lemma with the empty substitution. An accepted declaration graph expands to a finite
nest of non-recursive lets because DFS rejects every back edge. The expansion is therefore a well-typed term of this
fragment and is strongly normalizing. ∎

**Corollary — deterministic total evaluation.** Every accepted closed prompt-95 expression evaluates to exactly one
value of its declared type. Existence follows from normalization plus progress; type preservation follows from
Theorem 1; uniqueness follows from Theorem 3. The environment big-step evaluator returns that value by induction on
its derivation and the environment-substitution lemma.

The implementation checks the computational counterpart at every declaration boundary: evaluation returning no value
or a value whose reconstructed type differs from the checked type is reported as a compiler-invariant failure. The
generated-law tests additionally compare the production environment evaluator with a separate small substitution
evaluator and exercise products, lexical capture, higher-order functions, defaults, and rejected cycles.

### 5.6 Obligations added later

Options, lists, and structural folds must extend the candidate proof with finite constructor and structurally smaller
eliminator cases (prompt 96). `music` is not justified by STLC alone. Prompts 97–98 must prove: if `m : music`, `ρ` is a
well-formed `ElabEnv`, `p` is a valid exact placement, and resource checking accepts the instance, then
`instantiate(m,ρ,p)` returns a finite well-typed fragment and `close` returns a closed `Term[ScoreFact]`. That proof is
by induction over the private music constructors and must cover compatible binding-environment union, placement,
quotation substitution, and provenance attachment.

## 6. Implementation boundary

`Type`, `Value`, `Closure`, `Music`, evaluator environments, theory representations, monomorphization tables, and
resource proofs remain private to `musa-compiler`. Passes may expose narrow internal queries, but no single-implementor
public trait or pass-through facade is added. The stable public result remains the compilation/snapshot contract.
