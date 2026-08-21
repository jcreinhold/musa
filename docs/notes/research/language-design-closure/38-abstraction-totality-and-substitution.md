# Container abstraction, totality, and the substitution model

**Status: research. Governs nothing.** This note records a review that began from implementation evidence rather than
from a proof gate. It reopens questions notes 19, 22, 24, 26, and 33 settled, states what those notes actually gave as
reasons, corrects three analyses that were wrong, and lays out the alternatives with their costs. **It reaches no
conclusion about direction.** Nothing here is a recommendation, and nothing here proposes a change to `docs/rules/`.

## Why this note exists

Prompt 127dcfa landed [`stdlib/src/adapters/staff.musa`](../../../../stdlib/src/adapters/staff.musa) — the staff
adapter, the first of note 26 §9's two required trials. It is 2142 lines. Its actual logic is perhaps 400. Reading it
raised four separate questions that turned out to be independent:

1. Why does Musa have `list_fold`, `option_fold`, `nat_fold` rather than one abstraction over containers?
2. Why is the language total?
3. Would dependent types help, and were they rejected for a good reason?
4. What is the compiler's model of substitution?

Each is recorded below with what the repository says, what is actually true, and what the alternatives cost.

## 1. The evidence: what the staff adapter looks like, and why

The adapter is a parser written in a language where a parser cannot be written the usual way. The compiler hands an
adapter exactly one primitive — `syntax_fold` ([staff.musa:2106](../../../../stdlib/src/adapters/staff.musa)) — a
**bottom-up fold** over the region's delimiter-grouped token tree. No cursor, no lookahead, no recursion over the input.

Its pipeline, in execution order (the file reads bottom-up, definitions before uses):

| Stage        | Where                | What it does                                       |
| ------------ | -------------------- | -------------------------------------------------- |
| 1. tag       | `expand` :2105       | `syntax_fold` → every node becomes a `Read`         |
| 2. per-brace | `read_body` :1941    | seeds a `Pending`, folds one group's pieces         |
| 3. the fold  | `from_the_end` :1871 | runs **right to left**                              |
| 4. dispatch  | `taken_piece` :1843  | → `token_read` / `group_read` → `word_read` :1690   |
| 5. assemble  | `document_read` :1988| head must be complete → emit `Document(…)`          |

The central data structure is `Pending` (:274): one field holding the accumulated result, and **seven slots of arguments
seen but not yet claimed by a word**. Because the fold runs right to left and every form in the notation is
`word (numbers) { body }`, the reader collects the arguments and *then* meets the word that claims them. `holding_*`
functions fill slots; `word_read` empties them; `leftover` (:1882) reports whatever was never claimed.

Reading right to left is forced by two things, both stated in the file: the emitted data is right-nested, so the tail is
needed before the cons cell can be built (:1865); and a tie reaches *forward*, so reading backwards means the target is
already computed when the tie is met (:144).

### Why it is 2142 lines

Sorted by cause:

| Cause | Consequence of totality? |
| --- | --- |
| No record-update syntax (eight near-identical `holding_*`) | No |
| No `if` / guards (`clef_named` :1155 is a 4-deep `match`) | No |
| No `?` / `do` (`document_read`'s 120-line staircase) | No |
| No variadics (`call1`…`call7`, :311–:428) | No |
| `list_fold` is left-only, no `foldr` | No — `foldr` is total |
| `syntax_fold` is bottom-up only, so no recursive descent | **See below** |

The last item is the structural cause of the whole `Pending`/CPS idiom. It is **not** a consequence of totality either.
It is a consequence of `Syntax` being an abstract compiler-owned type whose only eliminator is a catamorphism. Agda and
Lean write recursive-descent tree traversals routinely and are total; structural recursion on an inductive tree
terminates. If `Syntax` were an ordinary `data` declaration with a generated fold, an adapter could match a node and
recurse on selected children, and `from_the_end` would not be needed.

The `?`/`do` gap is the same one note 24 already flagged: *"transformation becomes monadic. Note 19 forbids overloading
and type classes; note 21 leaves `try` as an open question"*
([24-pipeline-and-syntax-review.md:193](24-pipeline-and-syntax-review.md)).

## 2. Eliminators: what Musa has and the recorded reasons

`docs/rules/language/02-core-calculus.md` §1 gives three named eliminators, plus the generated fold of every nominal
`data` declaration:

```text
nat_fold    : A → (nat → A → A) → nat → A
list_fold   : A → (X → A → A) → list X → A
option_fold : A → (X → A) → option X → A
```

Inference is rank-1 Hindley–Milner with two classes of type variable — ordinary `a` over any value type, and `d` over
storable data only. The section is explicit that this is *"not subtyping, overloading, or a source-visible type class"*
(:160), and that *"Rank-1 is a boundary, not an accident: rank-2 and above make inference undecidable in general, and
nothing musical has asked for it."*

The absence of type classes is recorded twice:

- [19-inference-course-correction.md:213](19-inference-course-correction.md), under the heading **"Keep inference
  predictable"**: no subtyping; no implicit numeric conversions; **no overloading or type classes**; no higher-rank
  polymorphism; no polymorphic recursion; no dependent or refinement types; no general recursion.
- [26-language-design-decision.md:557](26-language-design-decision.md) §8, in the deliberately-omitted list, with a
  reopening rule at :559: *"Reopen one only when two materially different musical cases need it, or when an existing
  safety or stage theorem cannot be stated without it."*

### Correction 1: totality does not force per-type eliminators

An earlier framing in this review argued that in a total language the eliminator *is* the type's induction principle,
therefore folds must be per-type. That is wrong. Lean 4 is total by default (structural and well-founded recursion,
`partial` is opt-out) and Agda's termination checker enforces the same, and **both abstract over the container** — Lean
through `Foldable`/`ForIn`, Agda through `Foldable` records and instance arguments.

Totality forces eliminators to be *structural*. It does not force them to be *per-type*. The accurate statement is:

> totality **plus no instance search** ⇒ per-type eliminators.

What actually blocks a container-generic fold in Musa is that type variables range over types, not type constructors.
The model's type cannot be written:

```musa
data Folding<F> { Folding(run: (A, (X, A) -> A, F<X>) -> A) }   // needs F : Type -> Type
```

## 3. The categorical analysis

### 3.1 What a fold is

Musa's three eliminators are two different universal properties under one name:

| Operation     | Signature functor            | Universal property        |
| ------------- | ---------------------------- | ------------------------- |
| `nat_fold`    | `G A = 1 + A`                | initial `G`-algebra       |
| `list_fold`   | `F_X A = 1 + X × A`          | initial `F_X`-algebra     |
| `option_fold` | `H A = 1 + X` (constant in A)| coproduct copairing       |

`option_fold` is not recursive at all. Uniformly: for a polynomial endofunctor `F`, the initial algebra `(μF, in_F)`
gives, for every `F`-algebra `(A, α)`, a unique `cata α : μF → A` with `cata α ∘ in_F = α ∘ F(cata α)`. Every fold in
Musa, named or generated, is `cata` at some `F`. Reading "abstract over the container" as "quantify over `F`" is exactly
where kinds and higher-order unification come from: you must solve `?F ?X ≟ list nat`.

### 3.2 `Foldable` is not `cata`

The standard categorical reading of `Foldable f` is a **natural transformation `η : f ⇒ List`**, plus operations derived
from it. It never touches `f`'s recursion principle. Neither do `sum`, `length`, `any`, `toList`, or `traverse`. They
need only the enumeration of elements.

That is a decomposition in the sense of the `theory-design` skill's reference — two concerns, neither unfolds the other:

|  | **Elimination** | **Enumeration** |
| --- | --- | --- |
| Categorical content | initial `F`-algebra | natural transformation `f ⇒ List` |
| Needs `F` as a variable? | yes | **no** |
| Who provides it | generated per `data` | a value you pass |
| Musa today | present | absent |

Rust confirms this independently and without HKT: `IntoIterator` is `η : f ⇒ List` (lazily), and `Iterator::fold` is
written once against the protocol. Rust did not work around the absence of kinds; it observed that kinds were never
needed for this question. Rust's mechanism is an **associated-type projection** (`Self::Item`) in place of a constructor
application (`F<X>`) — `Self` has kind `*`, so nothing higher-kinded appears.

### 3.3 Level audit

Running `level-audit.md` on the demand for a generic `fold`:

- **Exact object:** the signature functor `F` and its initial algebra.
- **Shadow:** the enumeration `C → list X`.
- **What the shadow forgets:** `C`'s shape — nesting, labels, positions, how to rebuild.
- **Verdict:** for *elimination*, the question is substantive only at the exact level, and is already answered there per
  type by generation. For *summary operations*, the question is substantive at the shadow, and demanding `F` is the
  Overconstrained Hypothesis anti-pattern — requiring a functor variable when the argument only uses element order.

### 3.4 The defunctionalization criterion

> An operation defunctionalizes to first-order **iff its type mentions the container constructor only in fully-applied
> positions with fixed arguments.**

| Operation | Type | `C` appears | First-order? |
| --- | --- | --- | --- |
| fold | `C → list X` | once, fully applied | yes |
| `map` | `C X → C Y` | twice, at different arguments | no |
| `traverse` | `(X → F Y) → C X → F (C Y)` | two constructors, nested | no |
| `>>=` | `M X → (X → M Y) → M Y` | twice, different arguments | no |

`Foldable` defunctionalizes because `List` is the free monoid — a canonical target, so the abstraction has somewhere to
land. `Functor` has no canonical target: `map`'s conclusion mentions `C` applied to a type that varies.
Defunctionalization fixes the argument; here the argument varies in the conclusion. This is a wall, not a gap.

### 3.5 The first-order construction (and what it does not cover)

Passing the natural transformation *pointwise* removes the constructor variable:

```musa
data Listing<C, X> { Listing(elements: C -> list X) }

fn listing_fold(view: Listing<C, X>, seed: A, step: (X, A) -> A, container: C) -> A {
    match view {
        Listing(elements) -> list_fold(seed, step, elements(container)),
    }
}
```

`C`, `X`, `A` all have kind `*`. Unifying `Listing<?C, ?X>` against `Listing<list nat, nat>` is first-order:
`?C := list nat`, `?X := nat`. No higher-order unification, and no search, because the model is an argument.

**Why `list` is the right codomain.** `list X` is the free monoid on `X`, so an enumeration `C → list X` determines, for
every monoid `(M, e, ⊕)` and every `f : X → M`, a unique map `C → M`. That covers `length`, `any`, `sum`, `concat_map` —
the whole `Foldable` family — from one universal property rather than an ad hoc list. Rust must use a lazy `Iterator`
because its containers may be infinite and fusion is load-bearing; Musa's containers are finite by constitution §7, so
the free monoid is a real finite value and the intermediate list is the free object rather than a wart. Against a
metered cost table it is a budget line item, not a runtime cost.

**The dual.** `map`/`filter` back *into* `C` need `C`'s construction, so the algebra is a separate datum:

```musa
data Building<C, X> { Building(assemble: list X -> C) }
```

A pair with `assemble ∘ elements = id` exhibits `C` as a retract of `list X` — Rust's `IntoIterator` + `FromIterator`.

**Where the models come from.** Not hand-written: each `data` declaration whose shape admits it generates `C::listing`
and `C::building` as ordinary named constants. No registry, no orphan rule, no coherence question.

**What is given up.** Naturality is not statable. `Listing<C,X>` is a *component*, so nothing internal asserts that
`elements` commutes with `map`, and nothing prevents a `Listing<list nat, nat>` that reverses. Haskell's `Foldable` laws
are documentation too, so this is not a regression against the typeclass alternative — but it is a law that would need a
stated obligation on the generator in either design.

### Correction 2: this covers `Foldable` only

The `Listing` construction was initially presented as *the* answer to container abstraction. It is not. By §3.4 it
solves exactly the fold slice. `Functor`, `Traversable`, and monadic sequencing are all on the far side of the wall —
and the single worst part of the staff adapter, `document_read`'s 120-line staircase, is a **monad** problem, not a
`Foldable` problem. Any first-order-only design still needs an answer for the second group.

## 4. The alternatives, with costs

| Candidate | Categorical content | Cost |
| --- | --- | --- |
| **Typeclass / trait `Foldable`** | `f ⇒ List` + global instance search | Search, coherence/orphan rules, HKT. Rejected twice on record (note 19, note 26 §8). |
| **Explicit model record `Folding<F>`** | same, passed as a value | Kills search and coherence; keeps HKT. |
| **Church / Böhm–Berarducci encoding** | the initial algebra *as* a type | Needs **rank-2**. Forbidden by constitution §9 and core-calculus §1; reintroduces undecidable inference. |
| **Universe of functor codes** (`data Code {…}` + `⟦_⟧`) | datatype-generic programming, **zero HKT** | Needs a type-level interpretation `Code → Type → Type`, i.e. dependent types. |
| **Row / structural polymorphism** | records, not containers | Does not apply. |
| **Pointwise `Listing`/`Building` + generation** | components of `f ⇒ List` and its section | First-order, no search. Covers `Foldable` only (§3.5). |

### Correction 3: HKT does not require higher-order unification

Higher-order unification arises when a constructor metavariable appears in an *arbitrary* application. Haskell has had
HKT for thirty years without HOU because it restricts to the **Miller pattern** fragment — constructor variables applied
to distinct bound variables — where matching is decidable, unitary, and syntactic: `f a ~ List Nat` decomposes to
`f := List, a := Nat`.

Everything `Functor`/`Applicative`/`Monad`/`Traversable` need lives in that fragment. So "kinds ⇒ HOU" is false. A
design can have a kind system (`*`, `* → *`), explicit model passing (no search, no coherence, no orphans), and
pattern-fragment unification only.

Separately: **HOU comes from implicit arguments, not from dependency.** A dependent system with explicit or
deterministically-elided arguments needs no HOU either.

## 5. Totality

### What the constitution says

[`docs/rules/constitution.md`](../../../rules/constitution.md) §9, "One total source language builds both core values",
lists five load-bearing properties. The first (:193):

> **Total.** There is no general recursion and no partial call. Every accepted program finishes. A resource budget may
> stop an evaluation, but it may not change the value an accepted program produces.

The other four properties in §9 carry an argument. This one is stated as a bare refusal.

### The budget already exists

`02-core-calculus.md` :212–:217 gives `run(budget, e) ⇓ done(v) | failed(ResourceError)` and a budget-independence law:
if `run(b₁,e) ⇓ done(v₁)` and `run(b₂,e) ⇓ done(v₂)` then `v₁ = v₂`. So the operational guarantee — the compiler and the
editor cannot hang on a diverging adapter — is already provided independently of the type system, the way Rust's proc
macros and Lean's `partial` elaborator provide it.

### Totality is proved twice, and the two are independent

`02-core-calculus.md:683`:

> **Totality is proved twice, separately.** §5.5's strong normalization is about *this* language; T4 is about the term
> calculus. Neither implies the other, and a change to either one leaves the other's proof intact. A machine's M1 is a
> third, independent totality claim — one *step*, not one evaluation — and it shares no lemma with either.

- **T4** — the residual `Term[ScoreFact]` calculus normalizes. Load-bearing and musically motivated: constitution §7
  makes an event track a *finite* multiset of occurrences, §4 makes a machine a finite description, and :681 explains
  that `.musa.kernel` can be an interchange format *"because a residual program in a language with no functions is
  checkable, normalizable, and hashable by a consumer that knows nothing about this calculus."*
- **§5.5** — the *source language* strongly normalizes. This is the claim that shapes what adapter authors write.

The constitutional commitment in §7 is that musical *values* are finite. That the *language computing them* must be
total is a strictly stronger claim, and §9 does not derive it from anything musical. A metalanguage with a fuel budget
can emit a finite normalizing score term.

## 6. Dependent types

### The recorded rejection

- [19-inference-course-correction.md:213](19-inference-course-correction.md) lists "no dependent or refinement types"
  under **"Keep inference predictable"**, alongside subtyping and type classes.
- [26-language-design-decision.md](26-language-design-decision.md) §8 lists them among deliberately omitted features.
- [02-five-musical-cases.md](02-five-musical-cases.md) §3, the Karnatak case: *"The language needs theory-owned finite
  data, abstract types, `Result`, and explicit loss records. It does not need a universal `Pitch`, `Scale`, or `Raga`
  primitive. It also does not need dependent types: constructors can validate phrase rules and return a stated error."*
- [23-values-not-types.md](23-values-not-types.md) covers the same ground for pitch literals.

Two observations about that rationale, recorded without a verdict:

1. Grouping dependent types under "keep inference predictable" conflates two different regimes. Dependent types do not
   weaken principal types; they abandon HM inference for bidirectional checking, which is a different predictability
   story rather than an obviously worse one.
2. The five-cases argument is YAGNI. YAGNI is sound for libraries and asymmetric for foundations: a library can be added
   later, a foundation cannot, and its absence is paid by every author rather than once by the implementers. `AGENTS.md`
   states that asymmetry for sublanguages already.

### Do dependent types require annotations?

| Language | Requirement |
| --- | --- |
| **Agda** | Full signature for every top-level (and most local) definition. `foo x = x + 1` alone is an error. |
| **Idris 2** | Full signature at top level. |
| **Lean 4** | **Parameter** types required, **return** type inferred: `def foo (x : Nat) := x + 1` works. |
| **Coq** | Same as Lean. |

Agda's extra strictness is partly syntactic: clause-based definitions are written before the checker knows the type.

**Why none do HM inference.** HM works because types are first-order terms, where unification is decidable and *unitary*
— a most general unifier exists, which is what "principal type" means. With dependent types:

1. Type equality becomes term equality (conversion), decided by normalization — hence NbE.
2. Unification becomes non-unitary: `?f 3 ≟ 9` is solved by `λx. 9`, `λx. x * 3`, `λx. x + 6`, … with no most general
   solution. No MGU, no principal type.
3. Even `foo x = x` is ambiguous — `Nat → Nat`, `(A : Type) → A → A`, `(n : Nat) → Vec A n → Vec A n` all check, and how
   many implicit binders to insert in what order is not determined by the body.

Bidirectional checking splits terms into **synthesis** (variables, literals, constructor applications, fully-applied
calls, ascribed terms — type flows out) and **checking** (lambdas and other introduction forms — type flows in). A
lambda in argument position is in checking mode and needs no annotation.

### Correction 4: the annotation cost was measured wrong

This review initially claimed that dependent typing would cost musicians annotation-free inference. The corpus says
otherwise. Every top-level binding in `examples/` is already annotated:

```musa
let subject: Music = music { … }
fn raised(by: Interval, line: Music) -> Music { … }
let counted: List<Nat> = map(fn (index) { identity_nat(index) }, range(4));
let octave_answer: Music -> Music = fn (line: Music) -> Music { transpose(P8, line) };
```

The *only* places Musa currently omits annotations are inline lambdas passed to a fold — `staff.musa:2107`, `:2120`,
`doubled.musa:86`, `anonymous-functions.musa:30`. Those are argument-position lambdas, exactly the case bidirectional
checking handles for free. The two sets are disjoint in the direction that defeats the argument: where Musa relies on
inference, bidirectional needs no annotation; where bidirectional would demand one, Musa already writes it (and
`canon-functions.musa:14` writes it twice).

Measured against the corpus, the additional annotation burden is approximately zero.

### What dependent types would and would not cost here

Cheaper than usually assumed:

- **HKT becomes a non-question** — `(A : Type) → Type` is an ordinary Π. No kind system, no separate class mechanism.
- **No HOU** if arguments are explicit or deterministically elided.
- **No termination checker** if source-language totality is not required: the result is a dependently-typed programming
  language, not a proof assistant. Type-level computation could then diverge, but the existing budget already covers
  that.
- **T4 is untouched.** `Term[ScoreFact]` is "a language with no functions" and sits downstream of every source-language
  typing decision, so the interchange-format guarantee is independent of the source type system.

Genuine remaining costs:

- **Metatheory restatement.** §5.5's strong-normalization proof, the D1–D4 builtin conditions, and the §D corollary ("a
  later musical domain needs no new proof — it needs a base type with no eliminator") are all written for a rank-1
  system.
- **Checker rewrite.** Prompts 1–5 of the current stack built the kinded unifier and unification-driven checker.
- **Diagnostics.** Dependent type errors are notoriously poor, and this repository invests heavily in plain-language
  refusals pointed at the composer's own text.

### The constraint that makes it all-or-nothing

`AGENTS.md`, Standards: **"No sublanguage by subtraction."** A richer type system for adapter authors and a simpler one
for musicians is already forbidden. Adapters are written in Musa; whatever adapters need, the language has.

## 7. The substitution model

**There is no substitution operation anywhere in the compiler, and no de Bruijn indices.**

What exists instead is an environment machine with explicit capture lists:

```rust
fn eval(expression: &Expr, environment: &IndexMap<String, Value>, meter: &mut WorkMeter) -> Option<Value>
```

[`core.rs:7801`](../../../../crates/musa-compiler/src/phase/mod.rs). Variables are source-level strings
(`ExprKind::Name(name) => environment.get(name).cloned()`). A `Closure` (:3538) holds `parameters`, `result`, `body`,
and `captures: IndexMap<String, Value>`. The capture set is computed during elaboration and resolved against the
enclosing environment when the lambda is evaluated (:7818). Application extends rather than substitutes (:8188):

```rust
let mut local = closure.captures.clone();
for (parameter, value) in closure.parameters.iter().zip(provided) {
    local.insert(parameter.name.clone(), value);
}
eval(&closure.body, &local, meter)
```

This is capture-safe without α-conversion because the capture problem is a property of substitution into open terms; an
environment machine never rewrites a term. A parameter shadowing a captured name is handled by `insert` overwriting,
which is correct — inside the body that name lexically refers to the parameter.

The **type** side has no binders at all: `Type::Var` (:485) is a *unification* variable, and rank-1 HM quantifies only
at the outside of a scheme, so instantiation is the unifier's job. `Term[ScoreFact]` has no functions, hence no binders.

### This is not ad hoc — the spec states the arrangement

`02-core-calculus.md` defines the semantics by capture-avoiding simultaneous substitution `e[v̄/x̄]` over small-step rules
with evaluation contexts (:314), gives a big-step environment semantics `ρ ⊢ e ⇓ v` in §5.2, and states the relationship
at :352:

> Closure environments contain precisely the free named dependencies. This is an implementation optimization: by the
> environment-substitution lemma below, it has the same meaning as the small-step rules.

The bridge is proved in the standard order — Weakening → Substitution → **Environment substitution** (:370): *"If
`ρ ⊨ Γ` and `Γ ⊢ e : τ`, replacing every free `x` in `e` by `ρ(x)` yields a closed term of type `τ`. Proof. Repeated
application of substitution in any order, since the substituted values are closed."* The corollary at :439 discharges
the Rust: *"The environment big-step evaluator returns that value by induction on its derivation and the
environment-substitution lemma."*

Definition is substitution, implementation is an environment machine, equivalence is a stated and proved lemma. That is
Landin's arrangement and what every ML implementation does.

### What NbE would disturb

The environment-substitution lemma's proof turns on one premise: *"since the substituted values are closed."* NbE
evaluates **under binders**, so conversion checking works with open terms and neutrals, and the premise fails. A
dependent checker would need `Value::Neutral(level, spine)` — absent from the current `Value` enum (:3079) — and a
`quote : Value → Expr`. The standard setup is de Bruijn indices in the syntax and **levels** in the semantic domain,
which makes quoting capture-free by construction.

The environment machine already *is* the evaluation half of NbE; what is missing is neutrals and quote. NbE exists
precisely so that capture-avoiding substitution never has to be written.

One further interaction: `meter.step(Reduction::Application, 1, span)` charges every β-step. Under NbE, conversion
checking performs reductions a substitution-based checker would not, so the budget would begin counting type-level work.
The budget-independence law would need restating to cover it — the *value* stays budget-independent, but *acceptance*
would depend on conversion checking completing within the budget.

## 8. Convergence with an external design note

The user supplied a design note for a separate cubical/linear language that rejects typeclasses in favour of explicit
model passing. Two of its positions match Musa's adapter design, arrived at independently:

- §9.1's `using ring_syntax(IntegerRing) { x * x + 2 * x + 1 }` and Musa's `syntax staff { c5/4 d5/4. }` are the same
  construct: a lexically delimited region, a named and visible selection of meaning at the region head, hygienic macro
  expansion, no type-directed dispatch.
- §12.5's *"Importing a package must not silently change how unrelated files parse"* is
  [22-syntax-extension.md](22-syntax-extension.md)'s "no grammar takeover" rule.
- §24's three-way split — information *forced* by the program (may be inferred), information *generated mechanically*
  from declarations (may be macro-generated), information that *determines meaning* (must be named) — is the rule Musa's
  inference already follows, unstated.

Its §11 ("derivation generates values, not instances") is where the generated `C::listing` / `C::building` idea in §3.5
comes from.

## 9. Two findings that need resolution

Recorded because `AGENTS.md` forbids silent drift between code and governing documents. Neither is resolved here.

1. **A documented differential test does not exist.** `02-core-calculus.md:443` states: *"The generated-law tests
   additionally compare the production environment evaluator with a separate small substitution evaluator and exercise
   products, lexical capture, higher-order functions, and rejected cycles."* The four features are exercised —
   `crates/musa-compiler/tests/suite/core_laws.rs` has exactly those four tests — but there is no separate substitution
   evaluator in the crate, and the tests are not differential; they assert `!has_errors()` and `snapshot().is_some()`
   against the production path only. Either the reference evaluator should be written or the sentence repaired. It is
   worth noting that such a test is the cheapest available guard for any evaluator migration, and is worth much less
   written after the fact than before.

2. **The formatter can produce unparseable source.** A multi-line `//` comment written in argument position is joined
   into one line, swallowing the following argument. Repro: place such a comment before the last argument of
   `after_item(...)` inside `fn nesting` in `stdlib/src/adapters/staff.musa` and run
   `cargo run -q -p musa -- format stdlib/src/adapters/staff.musa`. Worked around in that file by moving the comment to
   the function's doc comment.

Two smaller inconsistencies noticed in passing: `AGENTS.md` says "currently 170 prompts through rank 153" while
`scripts/renumber-prompts.py audit` reports 186; and `docs/rules/language/01-surface.md:85` still claims the candidate
"deliberately has no anonymous-lambda surface" while `stdlib/src/adapters/staff.musa` uses inline `fn (…) { … }`
throughout.

## 10. What is not decided here

No direction is chosen. The live questions, and the evidence that would bear on each:

- **Ergonomics** (record update, guards or `if`, `try`/`?`, `foldr`). Needed under every option; forecloses nothing.
- **A structural eliminator for `Syntax`** in place of the bottom-up-only catamorphism. Adapter-API scope.
- **Container abstraction** beyond `Foldable`. Three shapes remain: kinds + explicit models in the Miller fragment;
  dependent types where kinds are ordinary Π; or accepting monomorphic duplication. §3.4 says which operations each
  shape can reach.
- **Source-language totality.** Amending `constitution.md:193` requires the procedure in `docs/rules/README.md`. §5
  records that T4 and §5.5 are separable, so the finiteness guarantee §7 needs does not depend on the answer.
- **Dependent types.** §6 records that the two recorded reasons for rejection do not survive scrutiny, and that the
  annotation cost is approximately zero against the current corpus. It does not follow that they should be adopted; the
  metatheory restatement and diagnostics risks are unpriced.

Note 26 §9 requires two unprivileged adapter trials. Only the staff adapter exists; the studio adapter is prompt 127dd.
A second adapter of a materially different kind is the natural experiment for several of the questions above, and note
26 §8's reopening rule — *"two materially different musical cases"* — is stated in exactly those terms.
