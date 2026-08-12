# Complete source rules

**Purpose:** give the full static and operational rules used by the proof.

`04-source-calculus.md` explains the design. This file is the compact reference. Surface punctuation remains open to
implementation work; the core forms and judgments do not.

## 1. Names and tables

The rules use four finite tables:

```text
Delta   private nominal types, constructors, and fields
Sigma   public modules, abstract types, values, and public constructors
Gamma   local and resolved value names with types
P       admitted compiler operations with types and contracts
```

A fresh nominal name is written `mu`. It belongs to one resolved package node, logical module path, and source
declaration during this build. Equality compares the fresh name. A consistent one-to-one renaming of all fresh names
does not change a checked program.

## 2. Types

```text
base ::= Unit | Bool | Nat | Ratio | Text | Duration | Music

type ::= base
       | bridge
       | mu
       | (type_1, ..., type_n)
       | type -> type
       | Option<type>
       | List<type>
       | Result<type, type>
```

`bridge` ranges over the finite compiler-owned notation types admitted for this language version.

The well-formedness judgment is `Delta; Sigma |- A type`. It checks that every nominal name appears in `Delta` or as an
abstract type in `Sigma` and recursively checks every component. Type equality is exact structural equality, with exact
equality of fresh nominal names. There is no subtyping or implicit coercion.

## 3. Data declarations

A constructor field uses this smaller grammar:

```text
field ::= Unit | Bool | Nat | Ratio | Text | Duration | bridge
        | earlier-mu
        | (field_1, ..., field_n)
        | Option<field>
        | List<field>
        | Result<field, field>
```

It excludes `Music`, functions, timelines, processes, and the type currently being defined. Looking through every
structural form, add `A -> B` to the nominal dependency graph when a field of `B` contains nominal `A`. Accept the data
table only when this graph is acyclic.

In dependency-first order, a declaration:

```text
data T {
  C_1(fields_1);
  C_2(fields_2);
}
```

adds fresh `mu_T` and constructor schemes:

```text
C_i : fields_i -> mu_T
```

to `Delta`. A top-level public data declaration also exposes its constructors in `Sigma`. A data declaration used to
implement an abstract signature member does not.

The compiler rejects duplicate type names, constructor names, and field names in one constructor. A constructor list
must be non-empty.

## 4. Terms and values

```text
e ::= x
    | literal
    | (e_1, ..., e_n)
    | lambda(x:A) => e
    | e_1(e_2)
    | let x = e_1; e_2
    | C(e_1, ..., e_n)
    | None | Some(e)
    | [] | e_1 :: e_2
    | Ok(e) | Err(e)
    | match e { p_1 => e_1; ...; p_n => e_n; }
    | nat_fold(e_0, e_step, e_nat)
    | list_fold(e_0, e_step, e_list)
    | option_fold(e_0, e_step, e_option)
    | primitive(e_1, ..., e_n)
    | (e : A)
```

The repeated dots in this grammar mean a finite sequence. A program must contain the actual terms.

Values are:

```text
v ::= literal
    | (v_1, ..., v_n)
    | closed-function
    | C(v_1, ..., v_n)
    | None | Some(v)
    | [] | v_1 :: v_2
    | Ok(v) | Err(v)
    | compiler-value
```

A `compiler-value` is a closed canonical bridge or `Music` value admitted by its operation contract.

Surface multi-argument functions become nested one-argument functions. A source function declaration provides all
parameter and result types. List literals become `::` and `[]`. `if condition then yes else no` becomes a `Bool` match.
No derived form adds a reduction rule.

## 5. Patterns

```text
p ::= _
    | x
    | literal
    | (x_1, ..., x_n)
    | C(x_1, ..., x_n)
    | None | Some(x)
    | [] | head :: tail
    | Ok(x) | Err(x)
```

Every bound name in one pattern is distinct. A constructor, product, `Some`, cons, `Ok`, or `Err` pattern contains only
binders, not another pattern. There are no guards.

The pattern judgment:

```text
Delta; Sigma |- p : A gives Gamma_p
```

checks as follows:

- `_` gives the empty table.
- A binder `x` gives `x:A`.
- A literal must have type `A` and gives the empty table.
- A product binder pattern gives each binder the matching component type.
- `C(x_1, ..., x_n)` requires an accessible constructor `C:A_1 * ... * A_n -> A` and gives each `x_i:A_i`.
- `None` requires `A = Option<B>` and binds nothing; `Some(x)` binds `x:B`.
- `[]` requires `A = List<B>`; `head :: tail` binds `head:B` and `tail:List<B>`.
- `Ok(x)` requires `A = Result<B,E>` and binds `x:B`; `Err(x)` binds `x:E`.

Inside the structure that defines `A`, constructor access may use its retained part of `Delta`. Client patterns use only
`Sigma`.

Coverage is a finite function `covers(A, patterns)`:

- `_`, a bare binder, or a product binder pattern covers its whole type;
- `true` plus `false` covers `Bool`;
- `None` plus `Some` covers `Option`;
- `[]` plus cons covers `List`;
- `Ok` plus `Err` covers `Result`;
- every accessible constructor covers a transparent nominal type; and
- every other type needs `_` or a bare binder.

An abstract nominal type has no accessible constructor set and therefore cannot be matched directly. The first matching
arm in source order is chosen. A compiler may warn about an unreachable arm, but that warning is not a typing premise.

## 6. Bidirectional checking

The synthesis judgment `Gamma |- e => A` finds `A`. The checking judgment `Gamma |- e <= A` uses the stated `A`. Both
judgments also carry `Delta` and `Sigma`; they are omitted below to keep the rules readable.

### 6.1 General rules

```text
Gamma |- e => A    A = B
------------------------- CheckFromSynth
Gamma |- e <= B

Gamma |- e <= A
----------------- Annotation
Gamma |- (e:A) => A
```

The equality in `CheckFromSynth` is exact type equality, not conversion.

### 6.2 Names and literals

```text
Gamma(x) = A
------------ Name
Gamma |- x => A

literal has base type A
----------------------- Literal
Gamma |- literal => A
```

Literal formation checks bounds. An invalid ratio denominator or out-of-range literal is a source error, not a term that
can step.

### 6.3 Products and functions

```text
Gamma |- e_i => A_i for every i
-------------------------------- ProductSynth
Gamma |- (e_1, ..., e_n) => (A_1, ..., A_n)

Gamma |- e_i <= A_i for every i
-------------------------------- ProductCheck
Gamma |- (e_1, ..., e_n) <= (A_1, ..., A_n)

Gamma, x:A |- body => B
----------------------------- LambdaSynth
Gamma |- lambda(x:A)=>body => A -> B

Gamma, x:A |- body <= B
-------------------------------- LambdaCheck
Gamma |- lambda(x:A)=>body <= A -> B

Gamma |- function => A -> B    Gamma |- argument <= A
----------------------------------------------------- Application
Gamma |- function(argument) => B
```

### 6.4 Let

```text
Gamma |- bound => A    Gamma, x:A |- body => B
------------------------------------------------ LetSynth
Gamma |- let x=bound; body => B

Gamma |- bound => A    Gamma, x:A |- body <= B
------------------------------------------------ LetCheck
Gamma |- let x=bound; body <= B
```

A surface declaration with a stated type checks its bound expression against that type before the body is checked.

### 6.5 Nominal and structural values

```text
C : A_1 * ... * A_n -> mu    Gamma |- e_i <= A_i for every i
---------------------------------------------------------------- Constructor
Gamma |- C(e_1, ..., e_n) => mu

Gamma |- e => A
---------------------------- SomeSynth
Gamma |- Some(e) => Option<A>

Gamma |- e <= A
---------------------------- SomeCheck
Gamma |- Some(e) <= Option<A>

Gamma |- head => A    Gamma |- tail <= List<A>
------------------------------------------------ ConsSynth
Gamma |- head :: tail => List<A>

Gamma |- head <= A    Gamma |- tail <= List<A>
------------------------------------------------ ConsCheck
Gamma |- head :: tail <= List<A>
```

`None` and `[]` only check:

```text
Gamma |- None <= Option<A>
Gamma |- [] <= List<A>
```

`Ok` and `Err` also only check:

```text
Gamma |- e <= A
------------------------------- OkCheck
Gamma |- Ok(e) <= Result<A,E>

Gamma |- e <= E
-------------------------------- ErrCheck
Gamma |- Err(e) <= Result<A,E>
```

An annotation can turn any checked form into a synthesizing term.

### 6.6 Matches

First synthesize the subject type `A`. For every arm, pattern checking gives `Gamma_i`. Require coverage.

```text
Gamma |- subject => A
Delta; Sigma |- p_i : A gives Gamma_i
covers(A, [p_1, ..., p_n])
Gamma, Gamma_1 |- arm_1 => B
Gamma, Gamma_i |- arm_i <= B for i > 1
------------------------------------------------ MatchSynth
Gamma |- match subject { arms } => B
```

When a surrounding declaration supplies the result type:

```text
Gamma |- subject => A
Delta; Sigma |- p_i : A gives Gamma_i
covers(A, [p_1, ..., p_n])
Gamma, Gamma_i |- arm_i <= B for every i
------------------------------------------------ MatchCheck
Gamma |- match subject { arms } <= B
```

The checking rule is what lets every arm begin with `Ok` or `Err`.

### 6.7 Finite folds

```text
Gamma |- initial => A
Gamma |- step <= Nat -> A -> A
Gamma |- count <= Nat
-------------------------------- NatFold
Gamma |- nat_fold(initial, step, count) => A

Gamma |- initial => A
Gamma |- step <= X -> A -> A
Gamma |- items => List<X>
-------------------------------- ListFold
Gamma |- list_fold(initial, step, items) => A

Gamma |- initial => A
Gamma |- step <= X -> A
Gamma |- item => Option<X>
-------------------------------- OptionFold
Gamma |- option_fold(initial, step, item) => A
```

Surface argument order may differ, but elaboration produces these core forms.

### 6.8 Compiler operations

```text
P(name) = A_1 * ... * A_n -> B
Gamma |- e_i <= A_i for every i
-------------------------------- Primitive
Gamma |- name(e_1, ..., e_n) => B
```

Ordinary entries satisfy the first-order contract. Structural folds and the named bounded music traversal use their
separate rules and cannot enter through this general rule.

## 7. Declarations and modules

Add an edge `x -> y` when the body of value definition `y` refers to definition `x`. The graph includes the entire
resolved build. Reject a cycle. In dependency-first order, check:

```text
Gamma |- body <= DeclaredType
```

then add the checked name to `Gamma`.

A signature contains only:

```text
type T;
let x: A;
```

A matching structure must:

1. define exactly one finite `data T` for each abstract type member;
2. define every required value at its substituted type;
3. contain no extra data member; and
4. contain only private extra value helpers.

Structure checking first adds its fresh private nominals and constructors to `Delta`. It checks all bodies there. It
then builds `Sigma` with the abstract type names and required value types but without the private constructors and
helper values. Compiled bodies retain the private `Delta` slice.

Modules are static. They are neither terms nor values. The first language has no type aliases, manifest type equations,
first-class modules, or nested structures.

## 8. Evaluation contexts

An evaluation context marks the next subterm. Its key forms are:

```text
E ::= hole
    | (v_1, ..., E, e_after)
    | E(e) | v(E)
    | let x=E; e
    | C(v_before, E, e_after)
    | Some(E)
    | E :: e | v :: E
    | Ok(E) | Err(E)
    | match E { arms }
    | fold(E, e, e) | fold(v, E, e) | fold(v, v, E)
    | primitive(v_before, E, e_after)
```

`v_before` contains only values to the left of the hole; `e_after` contains the untouched terms to its right. This
grammar fixes left-to-right evaluation.

If `r --> r'`, then:

```text
E[r] --> E[r']
```

## 9. Basic reductions

Function and `let` rules are:

```text
(lambda(x:A)=>body)(v) --> body[v/x]
let x=v; body          --> body[v/x]
(v:A)                  --> v
```

A match selects the first source-order pattern that matches its canonical subject and substitutes the selected fields
for the pattern binders:

```text
match C_i(v_1, ..., v_n) { arms } --> selected_body[v_1/x_1, ..., v_n/x_n]
```

The same rule covers products, literals, `Option`, lists, and `Result` with their field shapes.

The structural equations are:

```text
nat_fold(z, step, 0)       --> z
nat_fold(z, step, n+1)     --> step(n, nat_fold(z, step, n))

list_fold(z, step, [])     --> z
list_fold(z, step, x::xs)  --> list_fold(step(x, z), step, xs)

option_fold(z, step, None)    --> z
option_fold(z, step, Some(x)) --> step(x)
```

These equations use finite canonical naturals and lists. The evaluation contexts reduce the new calls in their stated
order.

An ordinary compiler operation with value arguments takes one abstract delta step to the unique value required by its
contract. The bounded music traversal has its own finite internal traversal and callback steps.

There is no reduction for resource exhaustion. The unmetered relation describes language meaning; the metered evaluator
in Section 10 may stop before a reduction and report a compile error.

## 10. Metered evaluation

Let `event(e)` name the next checking or evaluation action and let:

```text
charge(version, event, exact input sizes) = natural number
```

be a total fixed function. Relevant events include definition checking, closure creation, one fold step, value-node
construction, compiler-operation work, music-occurrence construction, and kernel quotation size. Aggregate operations
charge their known output bound before allocating or looping. Nested work is charged when reached.

A metered step is:

```text
budget >= charge
-----------------------------------------------
<e, budget, log> ==> <e', budget-charge, log'>
```

where `e --> e'` and `log'` appends the fixed diagnostics for that event. If the budget is smaller, evaluation returns
one resource-exhaustion diagnostic naming the event, attempted charge, and limit. It publishes no value.

The same term may fail under a smaller budget and succeed under a larger one. Repeatability compares equal initial
budgets, logs, language versions, and operation tables.

## 11. Denotation

Use the set interpretation in `04-source-calculus.md`. A nominal declaration denotes the tagged union of its constructor
field products. The dependency order makes that definition finite. A sealed abstract type keeps this same private set;
clients know only its exported operations.

An environment assigns each free variable a value in its type's set. Every typing rule above has the matching total set
operation. `Music` denotes a total function from an explicit `NotationContext` to a finite fragment or a stated error,
but source terms cannot invoke that function. The stage boundary invokes it.

The final metatheory proves that operational evaluation reaches the value given by this denotation.

