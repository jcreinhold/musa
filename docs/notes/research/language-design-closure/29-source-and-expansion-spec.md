# The source and expansion language

## Purpose

This document defines the small language selected in note 26. Its job is to make three claims precise:

1. ordinary Musa code has principal inferred types;
2. every accepted source expression finishes; and
3. package syntax adapters cannot evade parsing, name resolution, type checking, or source tracking.

The next note defines `Music` and the later stages. Neither note governs the repository until the proof gate passes.

## 1. Read one example first

```musa
pub type Shape<A>:
  private Leaf(A)
  private Group(List<Shape<A>>)

pub fn count(shape):
  fold_Shape(
    shape,
    fn(item): 1,
    fn(children): fold_list(children, 0, fn(child, total): nat_add(child, total)),
  )

let answer = count(Group([Leaf("a"), Leaf("b")]))
```

Inside this file, the compiler infers:

```text
count: Shape<A> -> Nat
answer: Nat
```

Outside the file, `Shape` and `count` are public but `Leaf` and `Group` are not. A client can hold a `Shape<Text>` and
pass it to `count`; it cannot forge or inspect a constructor. The generated `fold_Shape` is public only if the module
marks it public.

The example shows the whole design in miniature: nominal data, hidden constructors, rank-1 inference, a finite fold, and
no recursive source function.

## 2. What a build fixes

A build starts with one finite, already resolved package graph. Package resolution itself is outside this calculus. The
graph supplies exact source files and edges; it contains no registry lookup, version search, compiled identity, or
artifact cache.

For this build, the resolver assigns fresh ids to:

```text
ModuleId, TypeId, ConstructorId, FieldId, ValueId, AdapterId
```

An id is unique only inside this build. Two declarations have the same nominal identity exactly when they have the same
assigned id. Equal source text in two modules does not make their type ids equal, and this specification promises no id
stability across builds.

Write `Σ` for the resulting finite declaration table. It contains base types, nominal types, constructors, record
fields, imported value schemes, compiler operations, and visibility. Write `Γ` for local value types.

The static judgments are:

```text
Σ well formed
Σ ; Γ ⊢ e : τ
Σ ; Γ ⊢ p : τ ⇒ Γp
Σ ⊢ module M : Interface
```

The pattern judgment says that pattern `p` accepts values of type `τ` and binds the variables in `Γp`.

## 3. Types and their exact meanings

### 3.1 Base values

The base types mean:

- `Unit` has one value, `()`.
- `Bool` has `true` and `false`.
- `Nat` is the set of nonnegative integers. Implementations use arbitrary-precision integers.
- `Ratio` is a reduced pair `z/n`, where `z` is an integer, `n` is a positive integer, and `gcd(|z|, n) = 1`.
- `Text` is a finite sequence of Unicode scalar values after escape decoding. Equality compares that sequence exactly;
  the language performs no implicit Unicode normalization.

Natural and rational arithmetic is exact and total. Resource limits bound compiler work; integer overflow does not
change a program's value. A conversion to a bounded device or file format returns `Result` and names the failed bound.

### 3.2 Type grammar

Monotypes and rank-1 schemes are:

```text
τ ::= α
    | Unit | Bool | Nat | Ratio | Text
    | τ₁ × ... × τₙ
    | τ₁ -> τ₂
    | N<τ₁, ..., τₖ>

σ ::= ∀ α₁ ... αₙ. τ
```

`N` is a nominal type constructor from `Σ`. `List`, `Option`, and `Result` are ordinary public nominal types in the
prelude:

```text
List<A>   = Nil | Cons(A, List<A>)
Option<A> = None | Some(A)
Result<A, E> = Ok(A) | Err(E)
```

Their constructors are public. Products include the empty product `Unit`; a one-field product is identified with its
field type.

A function has one domain. Source syntax `fn f(x, y): body` uses the product domain `type(x) × type(y)`. The call
`f(a, b)` supplies that whole product. It never returns a function merely because one argument was omitted.

### 3.3 Type variables and substitutions

`ftv(τ)` is the set of free type variables in `τ`. A substitution `S` maps finitely many type variables to monotypes and
acts homomorphically on types, schemes, and environments. It does not replace variables bound by `∀`.

Generalization and fresh instantiation are:

```text
gen(Γ, τ) = ∀(ftv(τ) - ftv(Γ)). τ
inst(∀α₁ ... αₙ. τ) = τ[β₁/α₁, ..., βₙ/αₙ]
```

where each `βᵢ` is fresh.

## 4. Which data declarations are allowed?

### 4.1 Ordinary declarations follow a dependency order

Each nominal data or record declaration may mention base types, its parameters, and types declared earlier in the same
module or imported from dependencies. The declaration graph must be acyclic, with one exception: a variant type may
refer to itself under the finite forms listed below.

The initial language does not accept mutually recursive nominal declarations. The five programs do not need them, and
rejecting them removes a proof and implementation burden. A single self-recursive type can express trees, syntax, and
musical form.

### 4.2 Self-reference must be polynomial

Let `Self` stand for the type being declared. A constructor field may contain `Self` only through this grammar:

```text
P ::= Self
    | α
    | base type
    | earlier_nominal<Q₁, ..., Qₙ>
    | P₁ × ... × Pₙ
    | List<P>
    | Option<P>
    | Result<P, P>
```

Each `Qᵢ` is free of `Self`; only products and the three listed containers may wrap a recursive occurrence. `Self` may
not occur anywhere under a function arrow. Values cannot contain pointers back to an ancestor, mutation, or lazy fields,
so every constructed value is a finite tree.

Accepted:

```musa
type Form<A>:
  Leaf(A)
  Group(List<Form<A>>)
```

Rejected:

```musa
type Bad:
  Call(Bad -> Nat)
```

### 4.3 Every recursive declaration gets one fold

For each accepted `D<A₁, ..., Aₙ>`, the compiler generates `fold_D`. Replace every recursive `D<A₁, ..., Aₙ>` inside a
constructor field by a fresh result type `R`, preserving products, `List`, `Option`, and `Result`. The constructor's
algebra receives those replaced fields and returns `R`.

For `Form<A>`, the generated scheme is:

```text
fold_Form:
  Form<A>
  × (A -> R)
  × (List<R> -> R)
  -> R
```

The fold evaluates child folds before the parent algebra, from left to right. It therefore visits each finite
constructor occurrence once and terminates. Source code has no `let rec`, recursive function declaration, fixed-point
operator, or general recursive pattern binding.

## 5. Modules and names

### 5.1 One file is one module

A file has this fixed order:

```text
module header
ordinary imports
syntax imports
type and record declarations
value and function declarations
```

Imports are explicit and qualified. `import theory.tonal as tonal` makes `tonal.build` visible; it does not copy every
export into the local namespace. A syntax import uses `import syntax ...` and is visible only to the expansion phase.

Top-level value dependencies must form an acyclic graph. Functions may refer to earlier values and imported public
values, but not to themselves or a later cycle. The compiler checks the whole module graph, not just adjacent
declarations.

### 5.2 Public interfaces are inferred

A module interface contains:

- each public nominal type id and arity;
- each visible constructor and record field;
- the principal scheme of each public value;
- each public adapter descriptor; and
- no private declaration body.

A public type may hide all or some constructors. A public value scheme may mention a private type only if that type name
is also exported as an abstract public type. Otherwise interface construction fails with a source diagnostic.

No source signature or structure language is needed. The inferred interface is the signature. The source file is the
module implementation.

### 5.3 Record fields resolve before type inference

Record construction names its type:

```musa
let point = Point:
  x = 2
  y = 3
```

Each declaration creates a selector id such as `geometry.Point.x`. The short projection `point.x` is accepted only when
exactly one visible selector is spelled `x`. If two are visible, the source must write:

```musa
point.(geometry.Point.x)
```

The resolver therefore chooses one `FieldId` before inference. Types never choose among overloaded field names. This
keeps field access inside ordinary Hindley–Milner inference.

### 5.4 Resolution is finite

The resolver walks a finite syntax tree and finite symbol tables. Every unqualified namespace lookup must have exactly
one result. Every qualified path follows one resolved import edge. It either returns one id or a finite diagnostic.

## 6. Terms, values, and patterns

After syntax expansion, ordinary source expressions are:

```text
e ::= x | () | true | false | n | r | text
    | (e₁, ..., eₙ)
    | C(e₁, ..., eₙ)
    | R { l₁ = e₁, ..., lₙ = eₙ }
    | e.l
    | fn (x₁, ..., xₙ) => e
    | e₀(e₁, ..., eₙ)
    | let x = e₁ in e₂
    | if e₀ then e₁ else e₂
    | match e with p₁ -> e₁ | ... | pₙ -> eₙ
    | fold_D(e, a₁, ..., aₖ)
    | op(e₁, ..., eₙ)
    | (e : τ)
```

`C`, `R`, `l`, `fold_D`, and `op` are resolved ids. An `op` is a saturated compiler bridge operation. It receives all
arguments at once and never exists as a source function value.

Values are:

```text
v ::= () | true | false | n | r | text
    | (v₁, ..., vₙ)
    | C(v₁, ..., vₙ)
    | R { l₁ = v₁, ..., lₙ = vₙ }
    | fn (x₁, ..., xₙ) => e
```

Patterns are:

```text
p ::= _ | x
    | true | false
    | (p₁, ..., pₙ)
    | C(p₁, ..., pₙ)
    | R { l₁ = p₁, ..., lₙ = pₙ }
```

`true`, `false`, `Nil`, `Cons`, `None`, `Some`, `Ok`, and `Err` are constructor patterns. The initial language has no
guards, or-patterns, view patterns, numeric ranges, or arbitrary literal patterns.

Surface list syntax, record layout, `if`, and declaration bodies are fixed parser sugar for these forms. The expanded
ordinary expression tree records the desugaring.

## 7. Inference rules

### 7.1 The rule readers use most

For a non-recursive `let`, infer the right side, generalize every variable not fixed by the environment, then infer the
body:

```text
Σ ; Γ ⊢ e₁ : τ₁        σ = gen(Γ, τ₁)        Σ ; Γ, x : σ ⊢ e₂ : τ₂
──────────────────────────────────────────────────────────────────── Let
Σ ; Γ ⊢ let x = e₁ in e₂ : τ₂
```

The language is pure, so it needs no value restriction. `let identity = fn(x): x` may be used at both `Nat` and `Text`
in the same body.

### 7.2 Variables, literals, and functions

```text
x : σ ∈ Γ        τ = inst(σ)
──────────────────────────── Var
Σ ; Γ ⊢ x : τ

Σ ; Γ, x₁ : α₁, ..., xₙ : αₙ ⊢ e : τ
──────────────────────────────────────── Fn
Σ ; Γ ⊢ fn(x₁, ..., xₙ): e : (α₁ × ... × αₙ) -> τ

Σ ; Γ ⊢ f : (τ₁ × ... × τₙ) -> τ
Σ ; Γ ⊢ e₁ : τ₁   ...   Σ ; Γ ⊢ eₙ : τₙ
────────────────────────────────────────── Call
Σ ; Γ ⊢ f(e₁, ..., eₙ) : τ
```

`Fn` starts each parameter at one fresh monotype. It does not generalize parameters. `Call` constrains the function's
whole domain product. A call with the wrong number of fields does not typecheck, and no rule turns missing fields into a
closure.

Literal rules give `Unit`, `Bool`, `Nat`, `Ratio`, and `Text` their stated types.

### 7.3 Products, constructors, records, and fields

```text
Σ ; Γ ⊢ eᵢ : τᵢ for every i
──────────────────────────── Product
Σ ; Γ ⊢ (e₁, ..., eₙ) : τ₁ × ... × τₙ

Σ(C) = ∀ᾱ. τ₁ × ... × τₙ -> N<ᾱ>
Σ ; Γ ⊢ eᵢ : Sτᵢ for every i, with fresh instantiation S
────────────────────────────────────────────────────── Constructor
Σ ; Γ ⊢ C(e₁, ..., eₙ) : S(N<ᾱ>)

Σ(R) = ∀ᾱ. { l₁ : τ₁, ..., lₙ : τₙ }
the literal names each field exactly once
Σ ; Γ ⊢ eᵢ : Sτᵢ for every i, with fresh instantiation S
────────────────────────────────────────────────────── Record
Σ ; Γ ⊢ R { l₁ = e₁, ..., lₙ = eₙ } : S(R<ᾱ>)

Σ(l) = ∀ᾱ. R<ᾱ> -> τ        Σ ; Γ ⊢ e : S(R<ᾱ>)
────────────────────────────────────────────────────── Field
Σ ; Γ ⊢ e.l : Sτ
```

Constructor and record names have already passed visibility checks. A client cannot type a hidden constructor because
that constructor id is absent from its `Σ`.

### 7.4 Conditions, annotations, and compiler operations

```text
Σ ; Γ ⊢ e₀ : Bool        Σ ; Γ ⊢ e₁ : τ        Σ ; Γ ⊢ e₂ : τ
──────────────────────────────────────────────────────────── If
Σ ; Γ ⊢ if e₀ then e₁ else e₂ : τ

Σ ; Γ ⊢ e : τ        τ unifies with the written monotype A
──────────────────────────────────────────────────────────── Annotation
Σ ; Γ ⊢ (e : A) : A

Σ(op) = τ₁ × ... × τₙ ⇒ τ        Σ ; Γ ⊢ eᵢ : τᵢ for every i
──────────────────────────────────────────────────────────── Op
Σ ; Γ ⊢ op(e₁, ..., eₙ) : τ
```

An annotation checks one monotype. Explicit `∀` is not source syntax. The operation arrow `⇒` marks a first-order,
saturated compiler operation; it is not a source function type and cannot be stored or partially called.

Every operation descriptor states one total deterministic function on well-typed values. A recoverable domain failure is
represented by a `Result` in its return type. A compiler resource-limit error is outside the source value.

### 7.5 Patterns and matches

A pattern is linear: one variable name may occur at most once. Pattern typing follows constructor and record field
types. The important constructor rule is:

```text
Σ(C) = ∀ᾱ. τ₁ × ... × τₙ -> N<ᾱ>
S(N<ᾱ>) = τ
Σ ; Γ ⊢ pᵢ : Sτᵢ ⇒ Γᵢ for every i
the Γᵢ domains are disjoint
──────────────────────────────────────── Pattern-Constructor
Σ ; Γ ⊢ C(p₁, ..., pₙ) : τ ⇒ Γ₁, ..., Γₙ
```

Wildcard binds nothing. A variable pattern binds the scrutinee type. Product and record patterns combine disjoint
bindings component by component.

The match rule is:

```text
Σ ; Γ ⊢ e : τ
for every i: Σ ; Γ ⊢ pᵢ : τ ⇒ Γᵢ and Σ ; Γ, Γᵢ ⊢ eᵢ : υ
the ordered pattern matrix p₁ ... pₙ is exhaustive for τ
──────────────────────────────────────────────────────────── Match
Σ ; Γ ⊢ match e with p₁ -> e₁ | ... | pₙ -> eₙ : υ
```

Coverage uses the finite constructor table in `Σ`. For an abstract type with no visible constructors, a variable or
wildcard is required. The compiler may warn that a later arm is unreachable, but overlap is legal because source order
has meaning.

Coverage is decidable by the standard pattern-matrix specialization algorithm. Each specialization either removes a
constructor column or descends into a finite constructor field. Since patterns and constructor tables are finite, the
algorithm finishes.

### 7.6 Generated folds

Each `fold_D` is entered in `Σ` with the scheme described in §4.3. Its ordinary application is checked by `Call`; it has
no special inference rule. The positivity check, not the type checker, justifies its terminating implementation.

### 7.7 Algorithm W

Inference runs a syntax-directed form of Algorithm W:

1. give each unknown a fresh type variable;
2. collect equalities from the rules above from left to right;
3. unify with an occurs check and nominal-head equality;
4. generalize only at non-recursive `let` and module value boundaries; and
5. apply the final substitution to every typed-body node.

Unification treats `TypeId` as a rigid head. `N<τ̄>` unifies with `N<ῡ>` only when the two `TypeId`s are equal and the
arguments unify componentwise. It never identifies two nominal types because their constructors look alike.

The field resolver has already selected a `FieldId`, and compiler operations have fixed schemes. Algorithm W therefore
contains no ad hoc overloading, subtyping, implicit conversion, or expected-type macro call.

An annotation can reject an inferred type but cannot create a more general one. Inference either returns a principal
scheme or a finite unification diagnostic.

## 8. The fixed syntax-expansion boundary

### 8.1 Compiler order

Every source file follows this order:

1. lex and group the whole file with fixed compiler rules;
2. parse the fixed module header;
3. resolve explicit syntax imports in the already resolved package graph;
4. expand named expression and layout-block regions;
5. parse each result as one ordinary expression;
6. resolve ordinary names and fields;
7. infer and check types; and
8. lower the typed body to the evaluation core.

An adapter cannot change an earlier phase. Its output cannot add an import, module, type, value declaration, field,
constructor, or adapter.

### 8.2 Fixed tokens and groups

The lexer recognizes identifiers, natural and rational numbers, quoted text, comments, whitespace, the fixed core marks,
and every other non-alphanumeric code point as a symbol token. Packages cannot add a token rule.

The grouper matches `()`, `[]`, and `{}`. A line ending in `:` opens one layout group when the next significant line is
indented farther. No other token opens a group. Malformed delimiters or indentation fail before package code runs.

The lossless tree retains comments, whitespace, exact bytes, and source ranges.

### 8.3 Public syntax data

```text
SourceInfo = Original(SourceRange)
           | Generated(ExpansionId, ChildNumber)

Syntax = Missing(SourceInfo)
       | Token(SourceInfo, TokenKind, Text)
       | Identifier(SourceInfo, Name, Scopes)
       | Group(SourceInfo, Delimiter, List<Syntax>)
```

`Scopes` is an opaque compiler value. Package code can preserve and compare it; only the compiler can make a raw scope
id. `BlockSyntax` and `ExprSyntax` are hidden checked wrappers around `Syntax`. Package code cannot forge them from
arbitrary text.

The compiler exposes `fold_syntax`:

```text
fold_syntax:
  Syntax
  × (SourceInfo -> A)
  × (SourceInfo × TokenKind × Text -> A)
  × (SourceInfo × Name × Scopes -> A)
  × (SourceInfo × Delimiter × List<A> -> A)
  -> A
```

It is the generated fold for the finite `Syntax` tree.

### 8.4 Adapter descriptors and ranks

An adapter descriptor contains:

```text
AdapterDescriptor = {
  id,
  exact defining module in this build,
  expand function,
  adapters that its output may call,
  edit command type and function, if any,
  print function, if any,
  semantic equality for its produced value,
}
```

The declared adapter-call graph must be finite and acyclic. Define `rank(A)` as one plus the greatest rank of an adapter
that `A` may call; an adapter that calls none has rank zero. The compiler derives ranks and rejects a cycle.

The expansion function has the one public type:

```text
expand_A: BlockSyntax -> Result<ExprSyntax, SyntaxError_A>
```

It is an already checked total Musa function. It can use finite data, folds, syntax quotation, antiquotation, and fresh
hygienic names. It cannot read an inferred type, expected type, ordinary importing-module value, file, network, clock,
random source, project service, audio service, or mutable compiler state.

### 8.5 Expansion step and termination measure

One expansion step replaces a named region for adapter `A` with the finite syntax returned by `expand_A`. Any adapter
regions in that result must name adapters in the descriptor's lower-rank list.

For a syntax tree `s`, let `μ(s)` be the finite multiset of ranks of its unexpanded adapter regions. Order finite
multisets by the strict multiset extension of `<` on natural numbers. Replacing one rank by finitely many smaller ranks
strictly decreases `μ`.

Expansion repeatedly chooses the leftmost outermost unexpanded region. It stops at the first error or when `μ` is empty.
The choice rule is fixed, although confluence is unnecessary because the result is already deterministic.

### 8.6 Hygiene

Syntax quotation distinguishes three identifier origins:

1. a quoted identifier receives the adapter definition scope;
2. an antiquoted identifier keeps its use-site scopes; and
3. `fresh_name(hint)` receives a new scope that no source identifier has.

Package code cannot remove or forge these scope ids. Ordinary resolution compares both the name and scopes. Therefore a
quoted helper cannot capture a use-site name, an antiquoted name still refers from its use site, and a fresh name cannot
collide accidentally.

### 8.7 Source attribution

Each successful adapter call creates:

```text
ExpansionRecord = {
  expansion id,
  adapter id and defining module,
  use-site range,
  exact input syntax,
  exact output syntax,
  parent expansion, if any,
}
```

An antiquoted node retains its original `SourceInfo`. Every new quoted node receives
`Generated(expansion id, child number)`. Child numbers are unique within the record. Following `Generated` links reaches
one finite chain of records and ends at an original use site and adapter definition.

This record explains source expansion. It is not a musical derivation step. Musical derivation begins only when the
checked value crosses a musical stage boundary.

### 8.8 Editing and printing

An editable adapter also defines:

```text
edit_A:
  BlockSyntax × Edit_A
  -> Result<List<TextEdit>, EditError_A>

apply_A:
  Value_A × Edit_A
  -> Result<Value_A, EditError_A>
```

Returned edits must be sorted, disjoint, and wholly inside the adapter block. Applying them leaves every byte outside
their ranges unchanged.

Let `meaning_A(s)` mean: expand `s`, parse the one expression, resolve it, infer it, evaluate it, and obtain a value of
the adapter descriptor's stated `Value_A`. The edit law is:

```text
edit_A(s, c) = Ok(p)
meaning_A(s) = Ok(v)
apply_A(v, c) = Ok(v')
────────────────────────────────────────────
meaning_A(apply_text_edits(s, p)) = Ok(v')
```

Equality on `Value_A` is the exact semantic equality named by the adapter descriptor. The law does not claim equal
comments, whitespace, or source anchors after an edit.

An optional printer has type:

```text
print_A: Value_A -> Result<BlockSyntax, PrintLoss_A>
```

When it succeeds, `meaning_A(print_A(v))` must equal `v`. Printing creates new source; it is not the edit algorithm for
existing source.

## 9. The four compiler forms

### 9.1 Lossless grouped syntax

This form retains every source byte, parse error, group, and range. Editors, formatters, and adapter edits use it.
Expansion records are attached to it but do not replace it with a second editable tree.

### 9.2 Resolved program

This source-shaped form contains expanded ordinary expressions. Every module, type, constructor, field, value, and
operation occurrence holds one build-local id. Every node retains an original or generated source path. It contains no
unresolved name, though it may still fail type inference.

### 9.3 Typed body

This form adds:

- the inferred monotype of every expression;
- the principal scheme of every generalized `let` and public value;
- the chosen instantiation at each use;
- record and constructor visibility evidence;
- data positivity and fold-generation evidence; and
- match coverage and reachability information.

It preserves source shape for diagnostics and tools.

### 9.4 Evaluation core

The core makes these facts explicit:

- function captures and product domains;
- left-to-right evaluation order;
- resolved constructors and fields;
- generated fold algebras;
- saturated compiler operations;
- pattern decision trees; and
- local join points that share ordered fallbacks.

Polymorphic schemes and source patterns are gone. Instantiated monotypes remain as proof and validation data but are
erased from ordinary values.

The core adds two control forms:

```text
join j(x₁ : τ₁, ..., xₙ : τₙ) : τ = body in continuation
jump j(v₁, ..., vₙ)
```

A join name is not a value. It cannot be returned, stored, passed to a function, or called from outside its lexical
body. `jump` is allowed only in tail position.

The typing rules are:

```text
Σ ; Γ, x₁ : τ₁, ..., xₙ : τₙ ; J ⊢ body : τ
Σ ; Γ ; J, j : (τ₁ × ... × τₙ ⇒ τ) ⊢ continuation : τ
────────────────────────────────────────────────────────── Join
Σ ; Γ ; J ⊢ join j(x̄) : τ = body in continuation : τ

j : (τ₁ × ... × τₙ ⇒ τ) ∈ J
Σ ; Γ ; J ⊢ vᵢ : τᵢ for every i
──────────────────────────────────── Jump
Σ ; Γ ; J ⊢ jump j(v₁, ..., vₙ) : τ
```

The `⇒` here marks local control transfer, not a function type.

### 9.5 Ordered match lowering

The coverage checker gives the compiler an ordered, exhaustive pattern matrix. The decision-tree compiler specializes
one constructor column at a time. When a row may fail after some tests, it binds the remaining rows once:

```text
join next(bound values) = compile(remaining rows) in
compile_pattern(current row, on_failure = jump next(bound values))
```

Specialization narrows the remaining constructor set. At each leaf, the coverage witness says at least one row matches.
The final specialized row is irrefutable for that residual set. The generated tree therefore contains no `match_fail`,
exception, or impossible default value.

For the example:

```musa
match symbol:
  I -> tonic
  V -> dominant
  FlatII ->
    match next:
      Some(V) -> predominant
      _ -> other
  _ -> other
```

the compiler binds `other` once as a join and jumps to it from both fallback sites. It does not copy an effectful or
expensive expression. Source code is pure, but sharing still preserves exact evaluation charges and diagnostics.

## 10. Call-by-value evaluation

### 10.1 Evaluation order

Evaluation is strict and left to right. Contexts are:

```text
E ::= []
    | let x = E in e
    | E(e₁, ..., eₙ)
    | v₀(v₁, ..., vᵢ₋₁, E, eᵢ₊₁, ..., eₙ)
    | (v₁, ..., vᵢ₋₁, E, eᵢ₊₁, ..., eₙ)
    | C(v₁, ..., vᵢ₋₁, E, eᵢ₊₁, ..., eₙ)
    | R { ..., lᵢ = E, ... }
    | E.l
    | if E then e₁ else e₂
    | match E with arms
    | fold_D(E, a₁, ..., aₖ)
    | fold_D(v, v₁, ..., vᵢ₋₁, E, aᵢ₊₁, ..., aₖ)
    | op(v₁, ..., vᵢ₋₁, E, eᵢ₊₁, ..., eₙ)
    | (E : τ)
```

The annotation context is explicit. A well-typed annotated non-value cannot become stuck merely because its annotation
is present.

If `e -> e'`, then `E[e] -> E[e']`.

### 10.2 Reduction rules

The main rules are:

```text
(fn(x₁, ..., xₙ): e)(v₁, ..., vₙ)
  -> e[v₁/x₁, ..., vₙ/xₙ]                         Beta

let x = v in e -> e[v/x]                           Let-Value

if true then e₁ else e₂ -> e₁                      If-True
if false then e₁ else e₂ -> e₂                     If-False

(R { ..., l = v, ... }).l -> v                     Field

(v : τ) -> v                                       Annotation-Erase
```

Define `matches(p, v)` as the unique variable substitution obtained by structural pattern matching, or `None` when the
pattern does not match. Linearity makes the substitution unique. A match chooses the first successful arm:

```text
matches(p₁, v) = None ... matches(pᵢ₋₁, v) = None
matches(pᵢ, v) = Some(θ)
────────────────────────────────────────────────── Match-First
match v with p₁ -> e₁ | ... | pₙ -> eₙ -> θ(eᵢ)
```

For a closed value of the scrutinee type, exhaustiveness guarantees such an `i`.

### 10.3 Fold reduction

Suppose `C` is a constructor of recursive type `D`, and its algebra is `a_C`. The generated fold replaces each recursive
child in the constructor fields by its folded result, traversing products and standard containers from left to right.
Write that finite transformed field list as `lift_D(fold_D, fields)`.

```text
fold_D(C(fields), algebras)
  -> a_C(lift_D(fold_D, fields))                    Fold
```

The right side contains folds only on proper constructor subvalues. It never folds the original value again.

### 10.4 Compiler operations

Each operation descriptor supplies a mathematical function `δ_op` on well-typed values:

```text
δ_op(v₁, ..., vₙ) = v
──────────────────────── Op
op(v₁, ..., vₙ) -> v
```

`δ_op` is total, deterministic, and first-order. It cannot inspect a source closure. Its implementation must finish and
must obey the phase resource contract. File access, devices, clocks, randomness, audio stepping, and package resolution
are not compiler operations in source evaluation.

### 10.5 Join evaluation

The core evaluator carries a finite lexical join table `J`. Entering a `join` adds its parameter list, body, and current
value environment to `J`, then evaluates the continuation. A tail `jump` looks up exactly one join, binds its already
evaluated arguments, and evaluates the stored body. Joins cannot be recursive and cannot refer to a later join cycle.

This rule gives the same result as substituting the shared fallback at each jump, but evaluates only the selected path.

### 10.6 Deterministic logical charges

Every reduction has a rule tag. Its logical cost is:

```text
1 + semantic_size(inputs read by the rule) + semantic_size(value produced)
```

`semantic_size` counts constructor and syntax nodes, Unicode scalar values, and the bit lengths of naturals and reduced
rational numerators and denominators. A closure counts its core body nodes plus its fixed ordered capture vector and the
semantic size of each captured value. Each compiler-owned opaque value type supplies a versioned structural size
function in its operation descriptor. The count follows the value tree even when an implementation shares memory. It
does not depend on an allocator, CPU instruction, pointer address, hash table order, or cache warmth.

The compiler keeps separate counters for lexing, expansion, resolution, inference, source evaluation, temporal
evaluation, and preparation. A pure step computes its candidate result and logical charge, then commits the result only
when the new counter is within the phase limit. Exceeding a limit returns a compiler diagnostic with the phase and
source path; it does not become a source value.

A cache hit, where a later implementation permits one, must replay the exact logical charge certificate of the work it
replaces. This specification does not design or prove a persistent compiled-artifact cache.

## 11. Static source errors and mechanical migrations

The parser or type checker reports these old call forms directly:

```text
missing argument       -> write `fn(x): f(fixed, x)`
named call argument    -> put configuration in a named record value
default declaration    -> provide two named complete functions
partial compiler op    -> write a named or explicit `fn` wrapper
```

The staff adapter reports the old spaced exact duration `c4 3/8` with the mechanical replacement `c4(3/8)`. Existing
common staff forms such as `c4/4`, `c4/4.`, `[c4 e4 g4]/2`, and `rest/8` remain unchanged.

## 12. What this specification does not claim

It does not define package resolution, stable compiled identities, a package registry, a persistent cache, audio graph
steps, engraving, or any theory of pitch, metre, chords, form, tuning, or performance. It defines the total language in
which packages can state those theories and the bounded adapter path by which packages can give their data readable
source syntax.

The next note states the boundaries from evaluated values to exact temporal terms, notation, gestures, processes, and
sound.
