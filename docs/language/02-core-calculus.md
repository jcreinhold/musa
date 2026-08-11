# Total elaboration core

The core is a monomorphic, pure, call-by-value simply typed lambda calculus with finite data and structural eliminators.
Surface conveniences elaborate into this core before evaluation. It is deliberately more expressive than the kernel and
deliberately less expressive than a general-purpose programming language.

## 1. Syntax

Let base types `b` include the finite and exact musical domains named in `01-surface.md`. Each musical domain beyond the
prompt-95 fragment enters through §5.8's conservative-extension theorem and its checked discipline, not by assumption.
Core types are:

```text
τ ::= b | unit | bool | nat | ratio | τ × τ | option τ | list τ | τ → τ | music
```

`declaration κ`, `structure`, `piece`, `part`, `voice`, `Term[A]`, `Timeline[A]`, `Signal`, and DSP nodes are not value
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
written; there is no user generic-parameter syntax in this candidate. Compiler-owned `fold`/`map` primitives have finite
type schemes instantiated to monomorphic core operations during checking. Runtime/System-F polymorphism and implicit
let-generalization are absent; adding user parametric polymorphism requires a later specification.

## 2. Static semantics

The standard rules for products, sums, arrows, and immutable bindings apply. Representative rules are:

```text
Γ,x:σ ⊢ e : τ
────────────────────────────── Lam
Γ ⊢ (fn (x:σ) -> τ { e }) : σ→τ

Γ ⊢ f : σ→τ    Γ ⊢ a : σ
────────────────────────────── App
Γ ⊢ f(a) : τ

Γ ⊢ z : A    Γ ⊢ s : nat→A→A    Γ ⊢ n : nat
──────────────────────────────────────────── FoldNat
Γ ⊢ nat_fold(z,s,n) : A
```

Literal constructors enforce refinements such as nonnegative `duration`, finite scale members, and row bijectivity.
These are constructor judgments returning a value or a located diagnostic; they do not introduce dependent types. Named
predicates used by `assert` return finite evidence that the assertion layer can report.

Only compiler-owned primitives may construct `music`. `map_note_pitches` accepts a total `pitch → pitch` and visits a
documented subset of musical payload positions; it does not reveal them as a list. A kernel quote has a dedicated typing
rule and cannot be encoded by string operations.

## 3. Dynamic semantics

Evaluation is deterministic call by value, left-to-right for arguments and source-order for finite folds. A closure is
`⟨λx.e, ρ⟩` with an immutable finite environment. Function application evaluates the function, then its argument, then
the body under the captured environment extended by the value. Music constructors produce contextual `Music` values;
they do not call `instantiate` during ordinary evaluation.

Exact values remain integers or reduced rationals. There is no floating-point base type in the core. Ordering of maps,
declarations, diagnostic witnesses, and provenance steps is source-stable, never hash-iteration order.

## 4. Resource acceptance

Strong normalization does not bound a terminating program to useful project size. Musa therefore maintains one
deterministic meter over checking and evaluation. A finite aggregate operation charges its known count and logical
result shape before entering its loop or allocating its result; nested work is charged when its enclosing operation is
reached. All values remain private until the whole declaration graph succeeds, so exhaustion publishes neither a partial
value nor a partial score. The meter covers:

- monomorphized definition count and closure environment size;
- natural/list fold work, including products induced by nested folds;
- generated music occurrence count and kernel-term binding count;
- template/module instantiation count and static dependency depth;
- quotation size after typed substitution.

A project whose next charge exceeds the deterministic budget is rejected at that operation. The diagnostic names the
operation, metric, attempted amount, and limit. The prompt-96 defaults are 200,000 reduction steps, 100,000 constructed
value nodes, 1,048,576 logical value bytes, 2,048 monomorphized prelude instances, and 1,000,000 estimated music
occurrences. The scalar fragment charges zero output occurrences; prompt 97 connects music constructors to the already
present output counter. These are language-version constants, not timeouts or machine-memory observations. Interactive
cancellation remains an external compiler operation, not a language effect. Prompts 123 and 140 benchmark and may
tighten the accepted envelope deliberately.

## 5. Prompt-95 fragment and metatheory

This section fixes the proof obligation already implemented by prompt 95. Options, lists, folds, primitive pitch
operations, and `music` extend the calculus later and require their own compatibility cases; they are not smuggled into
this theorem by an appeal to “standard STLC.” §5.6 discharges finite data, §5.7 discharges `music`, and §5.8 discharges
every musical base type and compiler-owned operation added after prompt 95. Let `b` range over `bool`, `nat`, `ratio`,
`duration`, `pitch`, and `interval`. The implemented fragment is:

```text
σ,τ ::= unit | b | (τ₁ × … × τₙ) | (τ₁,…,τₙ) → τ
e   ::= x | c | (e₁,…,eₙ) | λ(x₁:τ₁,…,xₙ:τₙ).e
      | e(e₁,…,eₙ) | let x:τ = e in e
v   ::= c | (v₁,…,vₙ) | λ(x₁:τ₁,…,xₙ:τₙ).e
```

There is no anonymous-function surface syntax. A checked named `fn` supplies the lambda, and an acyclic named `let`
graph supplies the lexical lets. A `fn` body is written `{ e }` (prompt 112), and the braces are a **derived form**
erased by the elaboration `⟦{ e }⟧ = ⟦e⟧`: a block holds exactly one expression, so `⟦·⟧` is defined on it by that one
equation and is total. The erasure is applied where the surface is read, before any core term exists, so the set of core
terms above is unchanged and every theorem in §§5.2–5.5 quantifies over exactly the same set it did before the form was
added. There is no new value form, no new reduction rule, and hence no new case in preservation, progress, determinism,
or strong normalization — not because a block resembles a parenthesis, but because after `⟦·⟧` there is no block left
for a proof to be about. Multi-argument arrows and applications are notation for the corresponding curried STLC terms. A
surface call with named arguments is permuted into parameter order; an omitted default is inserted in that order and may
refer only to earlier parameters. Thus defaults and argument names add no core reduction rule. Products currently have
introduction but no surface projection, which is a conservative sublanguage of the product calculus.

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

Top-level signatures are collected before bodies are checked, so a later declaration may be referenced. From each body
the checker records an edge to every free named declaration. Acceptance requires the resulting finite graph to be
acyclic. A topological ordering `d₁,…,dₖ` then denotes the closed term `let d₁=e₁ in … let dₖ=eₖ in (d₁,…,dₖ)`; an edge
can only point to an earlier binding in this ordering. This gives forward references lexical meaning without giving Musa
recursive binding.

### 5.2 Small-step and big-step semantics

For the proof, capture-avoiding simultaneous substitution is written `e[v̄/x̄]`. Evaluation contexts enforce left-to-right
call by value:

```text
E ::= [] | (v₁,…,vᵢ₋₁,E,eᵢ₊₁,…,eₙ)
       | E(e₁,…,eₙ) | v(v₁,…,vᵢ₋₁,E,eᵢ₊₁,…,eₙ)
       | let x:τ = E in e

(λ(x̄:τ̄).e)(v̄)       → e[v̄/x̄]                       βᵥ
let x:σ = v in e       → e[v/x]                        Letᵥ
e → e′                 ⇒ E[e] → E[e′]                 Context
```

The implementation uses environments rather than copying syntax. Write `ρ ⊨ Γ` when `dom(ρ)=dom(Γ)` and each `ρ(x)` is a
closed value of type `Γ(x)`. Its big-step judgment is:

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
the result, or another variable, where `Var` is unchanged. Products and applications follow componentwise. For a lambda
or let binder, alpha-rename it fresh, use weakening for `v` under the extended context, and apply the induction
hypothesis to the body. Simultaneous substitution follows by iterating this lemma over distinct parameters. ∎

**Environment substitution.** If `ρ ⊨ Γ` and `Γ ⊢ e : τ`, replacing every free `x` in `e` by `ρ(x)` yields a closed term
of type `τ`.

*Proof.* Repeated application of substitution in any order, since the substituted values are closed. ∎

**Canonical forms.** A closed value of base type is a constant of that base type; a closed value of product type is a
product of values of the component types; a closed value of arrow type is a lambda (equivalently, an environment closure
whose environment realizes the lambda’s free-variable context).

*Proof.* Inspect the value grammar, then invert its typing rule. `unit` is presently uninhabited by surface literals, so
its closed-value case is vacuous until a unit constructor is introduced. ∎

### 5.4 Safety and determinism

**Theorem 1 — preservation.** If `∅ ⊢ e : τ` and `e → e′`, then `∅ ⊢ e′ : τ`.

*Proof.* Induct on the reduction derivation. For `βᵥ`, invert `Lam` and `App` and apply simultaneous substitution. For
`Letᵥ`, invert `Let` and apply substitution. For `Context`, invert the typing rule at the context hole, apply the
induction hypothesis there, and rebuild the same derivation. Exact constants do not reduce. ∎

**Theorem 2 — progress.** If `∅ ⊢ e : τ`, then `e` is a value or some `e′` satisfies `e → e′`.

*Proof.* Induct on typing. Literals and lambdas are values. For a product, select the leftmost non-value premise or
conclude by the value grammar. For application, first advance the function, then the leftmost non-value argument; if all
are values, canonical forms makes the function a lambda and `βᵥ` applies. A let advances its bound expression or uses
`Letᵥ`. There are no stuck primitive operations in this fragment. ∎

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
component induction hypotheses and the product candidate. For a lambda, take arbitrary reducible arguments; `βᵥ` reduces
its application to the body under the extended reducible substitution, so the induction hypothesis and candidate closure
give the result. Application follows directly from the arrow candidate. `let` is the lambda case via
`let x=e₁ in e₂ ≡ (λx.e₂)e₁`. ∎

**Theorem 4 — strong normalization.** Every well-typed term in the prompt-95 fragment is strongly normalizing.

*Proof.* Apply the fundamental lemma with the empty substitution. An accepted declaration graph expands to a finite nest
of non-recursive lets because DFS rejects every back edge. The expansion is therefore a well-typed term of this fragment
and is strongly normalizing. ∎

**Corollary — deterministic total evaluation.** Every accepted closed prompt-95 expression evaluates to exactly one
value of its declared type. Existence follows from normalization plus progress; type preservation follows from Theorem
1; uniqueness follows from Theorem 3. The environment big-step evaluator returns that value by induction on its
derivation and the environment-substitution lemma.

The implementation checks the computational counterpart at every declaration boundary: evaluation returning no value or
a value whose reconstructed type differs from the checked type is reported as a compiler-invariant failure. The
generated-law tests additionally compare the production environment evaluator with a separate small substitution
evaluator and exercise products, lexical capture, higher-order functions, defaults, and rejected cycles.

### 5.6 Strictly positive finite data

Prompt 96 extends terms and values by:

```text
e ::= … | none_τ | some(e) | []_τ | e :: e | match e with arms
        | nat_fold(z,s,n) | list_fold(z,s,xs) | option_fold(z,s,o)
v ::= … | none_τ | some(v) | []_τ | v :: v
```

The surface list literal elaborates to finite conses. `[head, ..tail]` is an elimination pattern, not a general spread
operator. Constructor typing and the three eliminators are:

```text
Γ ⊢ e : τ                         Γ ⊢ h : τ   Γ ⊢ t : list τ
────────────── Some               ───────────────────────── Cons
Γ ⊢ some(e) : option τ            Γ ⊢ h::t : list τ

Γ ⊢ z : A   Γ ⊢ s : (nat,A)→A   Γ ⊢ n : nat
──────────────────────────────────────────────── NatFold
Γ ⊢ nat_fold(z,s,n) : A

Γ ⊢ z : A   Γ ⊢ s : (X,A)→A   Γ ⊢ xs : list X
──────────────────────────────────────────────── ListFold
Γ ⊢ list_fold(z,s,xs) : A

Γ ⊢ z : A   Γ ⊢ s : X→A   Γ ⊢ o : option X
────────────────────────────────────────────── OptionFold
Γ ⊢ option_fold(z,s,o) : A
```

A match checks every arm under the bindings introduced by its pattern and requires one result type. Boolean matches
cover `true` and `false`; option matches cover `none` and `some`; list matches cover `[]` and cons. A binding or `_`
covers any type. Literal matches over naturals, ratios, durations, pitches, and intervals therefore require a final
catch-all. Product binding patterns are irrefutable. An arm following complete coverage, or repeating a constructor or
literal already covered, is rejected as unreachable. These finite coverage facts extend canonical forms and make the
match case of progress immediate.

The fold equations are deterministic left folds in source order:

```text
nat_fold(z,s,0)       → z
nat_fold(z,s,n+1)     → s(n, nat_fold(z,s,n))
list_fold(z,s,[])     → z
list_fold(z,s,x::xs)  → list_fold(s(x,z),s,xs)
option_fold(z,s,none) → z
option_fold(z,s,some(x)) → s(x)
```

The implementation iterates rather than building these recursive terms; the equations specify the result. `map` and
`filter` are list folds, `range(n)` constructs `[0,…,n−1]`, and value `repeat(x,n)` constructs `n` copies of `x`. Their
rank-1 schemes are compiler-owned and instantiated once at a concrete type at each direct call. They cannot be stored as
polymorphic values, partially applied, or used to infer a polymorphic user definition.

Extend the reducibility candidates by:

```text
R_(option τ)(t) iff t ∈ SN and t →* none or t →* some(v) with R_τ(v)
R_(list τ)(t)   iff t ∈ SN and t →* [v₁,…,vₙ] with every R_τ(vᵢ)
```

Constructor compatibility follows from the induction hypotheses for members. For eliminators, use the lexicographic
measure `(constructor count, reduction height of arguments)`: `nat_fold` decreases the natural by one, `list_fold`,
`map`, and `filter` decrease list length by one, `option_fold` consumes its sole constructor, and `range`/value-repeat
decrease their compiler-owned natural counter. The step function is already reducible at the instantiated monomorphic
arrow type, so applying it preserves the accumulator candidate. Induction on that measure proves each eliminator maps
reducible arguments to a reducible result. These new cases extend the fundamental lemma and hence preservation,
progress, determinism, and strong normalization. Finite monomorphization does not alter the proof: each instance is an
ordinary monomorphic term, and the accepted instance graph is finite and acyclic.

### 5.7 Contextual-music safety (prompt 97)

`music` is not a stored `Timeline[ScoreFact]`. Its checked value is a finite, context-reading recipe whose scalar free
variables have already been captured and whose nested `use` expressions have already evaluated to checked `music`
values. Write a private fragment as `F = (t,B,d,n)`, where `t : Term[ScoreFact]`, `B` is a finite acyclic environment of
term bindings, `d ∈ ℚ≥0` is the exact extent, and `n ∈ ℕ` is the exact occurrence count after instantiation. Its
well-formedness judgment requires:

1. every free term name of `t` is in `dom(B)`;
2. binding edges in `B` point to earlier completed bindings;
3. every literal in `t` is a valid finite `Timeline[ScoreFact]` and every mark is a total payload map;
4. `d` and `n` agree structurally with `t` after its bindings are instantiated; and
5. all facts have the requested scope, exact nonnegative placement, and a complete `Origin`.

**Lemma 1 — compatible composition.** If `F₁,…,Fₖ` are well formed, their sequential and overlay compositions are well
formed. Sequence has extent `Σᵢdᵢ` and count `Σᵢnᵢ`; overlay has extent `maxᵢdᵢ` and the same count sum.

*Proof.* The kernel constructors validate nonempty finite operands and preserve literal well-formedness. Musa's shared
binding table interns equal contextual instances by a conservative key. A new binding is appended only after every
binding its body references has completed; a reused binding names that same completed body. Thus union is compatible,
finite, and acyclic. Sequence shifts the `i`th operand by `Σ_{j<i}dⱼ`, while overlay shifts none, giving the stated
extent equations. Neither operation deletes or duplicates an occurrence, giving the count equation. ∎

**Lemma 2 — marked reference.** Replacing a well-formed fragment by a reference to its completed binding, marked with a
call scope, call span, and finite Origin path, preserves its extent and occurrence count and yields valid facts when
evaluated.

*Proof.* Kernel marked evaluation instantiates the bound term and applies Musa's mark only to payloads. The mark changes
`Scope` and `Origin`; it does not change spans, multiplicity, ordering, or extent. Its operation is total because the
checker constructs the mark rather than accepting arbitrary mark text at this boundary. ∎

**Theorem 4 — contextual instantiation and closure.** If `Γ ⊢ m : music`, `ρ ⊨ Γ`, `p` is a valid exact placement, and
the deterministic resource meter accepts the required output, then `instantiate(m,ρ,p)` returns a finite well-formed
fragment. Closing that fragment returns a closed, checked `Term[ScoreFact]` whose evaluation has exactly the fragment's
`n` occurrences.

*Proof.* Use well-founded induction on the pair consisting of the declaration dependency rank and the finite syntax size
of `m`. A note, rest, point mark, or region constructs a finite literal; exact rational validation supplies its span,
and the checked scalar environment supplies any pitch or duration variable. A region overlays one valid fact on the
inductively obtained body. Sequence and voice overlay follow from Lemma 1. A `use` targets either a value already
evaluated in the acyclic declaration graph or a syntactically nested checked value of smaller size; apply the induction
hypothesis and Lemma 2. A finite repeat uses a checked natural count, and the existing transpose, stretch, inversion,
retrograde, tie, and specialization operations are total finite transformations already covered by their kernel and
compiler law suites. Context-changing statements have no case: checking rejects key, meter, tempo, and clef inside
reusable material, so contextual music reads placement but cannot mutate the ambient structural context.

For closure, traverse completed bindings in reverse completion order and wrap exactly those reachable from `t`. By
condition 2, when a binding is wrapped, every name it can expose is wrapped later and hence dominates it in the final
term. Condition 1 therefore leaves no free name. Literal validity and constructor preservation make `Term::check`
succeed. Structural occurrence accounting uses addition, multiplication by a finite repeat count, and one for each
region; the resource preflight occurs before occurrence-sized allocation, so accepted evaluation is finite and has
exactly `n` occurrences. ∎

Prompt 98 may add higher-order constructors only by proving that each maps well-formed finite fragments to well-formed
finite fragments; it does not reopen this closure argument.

### 5.8 Musical domains as a conservative extension

Prompts 100–107 add the base types `NoteName`, `Pc12`, `Scale`, `Key`, `Degree`, `Frame`, `ChordClass`, `Triad`,
`Voicing`, `PcSet12`, `Row12`, and `Roman`, and the compiler-owned operations over them. §5's warning applies to every
one of them: they are not covered by an appeal to standard STLC. They are covered instead by one parametric theorem
whose premises are mechanically checked, so that a later domain costs a registry entry rather than a new induction.
`docs/language-correction.md` §2 is the governing statement; this section is its calculus-side text.

Every compiler-owned primitive belongs to exactly one of three families, and that the families are disjoint and
exhaustive is a checked law:

- **δ-primitives** — every argument type and the result type is a base type or a finite constructor (`option`, `list`,
  product) over base types, with no arrow anywhere in the signature;
- **structural eliminators** — `nat_fold`, `list_fold`, `option_fold`, `map`, `filter`, `range`, `repeat`, proved in
  §5.6;
- **music primitives** — the constructors and controlled transforms of §5.7.

A δ-primitive must satisfy four conditions:

- **D1 inertness.** Its base types have no eliminator. A closed value of a musical base type is an opaque constant; no
  reduction rule inspects its structure, and the only pattern that may match it is a literal or a catch-all, which §5.6
  already requires to be followed by a catch-all arm.
- **D2 totality.** For every tuple of closed values of the declared argument types the primitive yields a closed value
  of the declared result type. Partiality is expressed *in the result type* as an `option` — never as a stuck term, a
  panic, or a diagnostic.
- **D3 purity.** The result is a function of the argument values alone: no ambient context, no evaluation-order
  dependence, no hash-iteration order, no diagnostic emission.
- **D4 finiteness.** The result's constructed-node count is bounded by a function of the argument sizes, charged to the
  §4 meter before construction begins.

**Theorem 5 — conservative extension.** Let `𝔅` be the base types of the proved fragment. Adding a base type `b ∉ 𝔅`
with no eliminator, together with any finite set of δ-primitives over `𝔅 ∪ {b}` satisfying D1–D4, preserves Theorems
1–4.

*Proof.* Take `R_b(t) ⟺ t : b ∧ t ∈ SN`, which is the clause §5.5 already assigns every base type, so candidate
properties (i)–(iii) hold by the existing induction with one additional leaf and no new case shape. *Preservation*: by
D2 the primitive's actual result type is its declared result type, so inverting its application rule goes through
unchanged. *Progress*: an application whose arguments are all values steps by D2, one with a non-value argument steps by
`Context`, and D1 removes the only other way a value of `b` could stand at a redex position; so no δ application is
stuck. *Determinism*: D3 makes the primitive a function, and the leftmost-context decomposition of §5.4 is unchanged
because no new context former is introduced. *Strong normalization*: by D2 a δ redex whose arguments are values
contracts to a value in one step, and values are normal, so the fundamental lemma's new case is immediate. The arrow,
product, option, list, and `music` cases of §5.5–§5.7 quantify over the base-type set without inspecting it and
therefore carry over verbatim. ∎

**Corollary.** A later musical domain needs no new proof — it needs a base type with no eliminator, primitive signatures
containing no arrow, and a discharge of D1–D4.

The theorem concerns the type system only. That a German sixth spells its top note as an augmented sixth, that `ii` is
minor in a major collection, and that a harmonic-minor `III7` is honestly absent rather than rounded to a named chord
are claims about music, checked by the law suites in `crates/musa-compiler/tests/` and defined in
`03-musical-domains.md`. Neither statement substitutes for the other.

The implementation carries the premises rather than trusting them. The primitive-ownership registry records each
operation's family and declared signature, and its law suite checks that every primitive is classified exactly once,
that no δ-primitive signature contains an arrow, that every base type reachable from a δ signature is inert and admits
no destructuring pattern, and that evaluating each δ-primitive over a finite sample of its argument domains — exhaustive
where the domain is finite, generated to a documented bound where it is not — returns a value of the declared type
without panicking, diagnosing, or reporting a Rust-level absence at a non-`option` result type.

A new base type is admissible only with a stated reason no existing domain can carry the distinction, a D1–D4 discharge
with its registry entries, and a row in `03-musical-domains.md` giving its definition, source, and a counterexample it
rules out.

## 6. Implementation boundary

`Type`, `Value`, `Closure`, `Music`, evaluator environments, theory representations, monomorphization tables, and
resource proofs remain private to `musa-compiler`. Passes may expose narrow internal queries, but no single-implementor
public trait or pass-through facade is added. The stable public result remains the compilation/snapshot contract.
