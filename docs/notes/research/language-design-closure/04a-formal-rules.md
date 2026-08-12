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

`bridge` ranges over the finite compiler-owned types admitted for this language version. The migration version includes
the current `Pitch`, `NoteName`, `Interval`, `Scale`, `Key`, `Degree`, `Frame`, `ChordClass`, `Triad`, `Roman`,
`Voicing`, `Pc12`, `PcSet12`, and `Row12` types. Only notation boundary types remain after the theory-package migration.
Keeping the migration set here lets the old core embed before that separate source migration occurs.

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
    | map(e_function, e_list)
    | filter(e_predicate, e_list)
    | range(e_nat)
    | repeat(e_value, e_nat)
    | music_operation(name, e_1, ..., e_n)
    | map_note_pitches(e_function, e_music)
    | kernel_quote(checked_term, [(hole_1, locus_1, e_1), ..., (hole_n, locus_n, e_n)])
    | primitive(name, e_1, ..., e_n)
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

Evaluation also uses two private states that source code cannot write:

```text
map_state(function, remaining_input, completed_output)
filter_state(predicate, remaining_input, completed_output)
map_wait(function, remaining_input, completed_output, expression)
filter_wait(predicate, current, remaining_input, completed_output, expression)
```

Each state evaluates the next callback through ordinary source-function application, then records its result before
moving to the next input. The exact transitions appear in Section 9.

Surface forms use this exact elaboration table:

| Surface form | Core form |
| --- | --- |
| `let f(x:A, y:B): C = body` | `f = lambda(x:A) => lambda(y:B) => body`, checked at `A -> B -> C` |
| an ordinary user call `f(a, b)` | `(f(a))(b)` |
| a bare nullary constructor `C` | `C()` |
| a saturated constructor call `C(a, b)` | `C(a, b)` |
| `[a, b]` | `a :: b :: []` |
| `fold_list(items, initial, step)` | `list_fold(initial, step, items)` |
| `fold_nat(count, initial, step)` | `nat_fold(initial, step, count)` |
| `fold_option(item, initial, step)` | `option_fold(initial, step, item)` |
| `if condition then yes else no` | `match condition { true => yes; false => no; }` |
| a saturated ordinary compiler call `p(a_1, ..., a_n)` | `primitive(p, a_1, ..., a_n)` |
| a saturated controlled music call other than `map_note_pitches` | `music_operation(name, a_1, ..., a_n)` |
| `map_note_pitches(function, music)` | the core form of the same name |

A source function declaration provides all parameter and result types. Resolution tells a constructor, ordinary
function, and compiler operation apart, so these rows do not rely on capitalization or spelling. A compiler operation
must receive every argument; it cannot be partially applied. No derived form adds a reduction rule.

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

Source syntax may omit the empty parentheses of a nullary constructor pattern. Thus `Tonic` is the surface spelling of
the core pattern `Tonic()`.

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

An abstract nominal type has no accessible constructor set. A client may still use `_` or a bare binder to ignore or
pass through the whole value, but it cannot use a constructor pattern to inspect it. The first matching arm in source
order is chosen. A compiler may warn about an unreachable arm, but that warning is not a typing premise.

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
Gamma |- step => Nat -> A -> A
Gamma |- initial <= A
Gamma |- count <= Nat
-------------------------------- NatFold
Gamma |- nat_fold(initial, step, count) => A

Gamma |- items => List<X>
Gamma |- step => X -> A -> A
Gamma |- initial <= A
-------------------------------- ListFold
Gamma |- list_fold(initial, step, items) => A

Gamma |- item => Option<X>
Gamma |- step => X -> A
Gamma |- initial <= A
-------------------------------- OptionFold
Gamma |- option_fold(initial, step, item) => A
```

Surface argument order may differ, but elaboration produces these core forms.

The other four structural operations have these monomorphic rules:

```text
Gamma |- function => X -> Y    Gamma |- items <= List<X>
-------------------------------------------------------- Map
Gamma |- map(function, items) => List<Y>

Gamma |- predicate => X -> Bool    Gamma |- items <= List<X>
------------------------------------------------------------ Filter
Gamma |- filter(predicate, items) => List<X>

Gamma |- count <= Nat
------------------------- Range
Gamma |- range(count) => List<Nat>

Gamma |- value => A    Gamma |- count <= Nat
-------------------------------------------- Repeat
Gamma |- repeat(value, count) => List<A>

Gamma |- value <= A    Gamma |- count <= Nat
-------------------------------------------- RepeatCheck
Gamma |- repeat(value, count) <= List<A>
```

The schemes are instantiated once at a concrete `X`, `Y`, or `A` at each direct call. They are not polymorphic values
and cannot be partially applied.

### 6.8 Compiler operations

```text
P(name) = A_1 * ... * A_n -> B
Gamma |- e_i <= A_i for every i
-------------------------------- Primitive
Gamma |- name(e_1, ..., e_n) => B
```

Ordinary entries satisfy the first-order contract. Structural folds and the named bounded music traversal use their
separate rules and cannot enter through this general rule.

The current controlled music operations use this finite table:

| Name | Type |
| --- | --- |
| `transpose` | `Interval -> Music -> Music` |
| `stretch` | `Ratio -> Music -> Music` |
| `retrograde` | `Music -> Music` |
| `invert` | `Pitch -> Music -> Music` |
| `shift` | `Duration -> Music -> Music` |
| `overlay` | `Music -> Music -> Music` |
| `map_note_pitches` | `(Pitch -> Pitch) -> Music -> Music` |
| `play` | `Voicing -> Duration -> Music` |

The source operation always returns a finite recipe. An invalid stretch factor or held duration becomes a stated error
when that recipe is instantiated. The compatibility theorem in Section 12 covers old programs whose old evaluation and
instantiation succeed; existing failing diagnostics remain an implementation migration obligation, not a new source
value.

Every operation except `map_note_pitches` checks by repeated use of `Application` against its table entry.
`map_note_pitches` has the explicit rule:

Equivalently, after surface applications have been collected, the compiler-owned recipe node checks by:

```text
MusicOps(name) = A_1 * ... * A_n -> Music
Gamma |- e_i <= A_i for every i
------------------------------------------ MusicOperation
Gamma |- music_operation(name, e_1, ..., e_n) => Music
```

`map_note_pitches` uses the stronger rule below instead of this first-order schema:

```text
Gamma |- function <= Pitch -> Pitch    Gamma |- music <= Music
------------------------------------------------------------- MapNotePitches
Gamma |- map_note_pitches(function, music) => Music
```

It constructs a finite recipe. It does not apply `function` during source evaluation.

A checked kernel quote has a finite kernel term `q` whose only free term names are the distinct names in its hole table.
Each name is expected at `Term<ScoreFact>`, each locus is a non-negative exact rational inside `q`'s extent, and every
hole expression has type `Music`:

```text
q checks as Term<ScoreFact> under holes h_1, ..., h_n
Gamma |- e_i <= Music for every i
------------------------------------------------------ KernelQuote
Gamma |- kernel_quote(q, [(h_i, locus_i, e_i)]_i) => Music
```

Malformed terms, duplicate or missing holes, extra free names, and invalid loci are checking errors.

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
    | map(E, e) | map(v, E)
    | filter(E, e) | filter(v, E)
    | range(E)
    | repeat(E, e) | repeat(v, E)
    | music_operation(name, v_before, E, e_after)
    | map_note_pitches(E, e) | map_note_pitches(v, E)
    | kernel_quote(q, holes_before, (h, locus, E), holes_after)
    | primitive(name, v_before, E, e_after)
    | (E : A)
    | map_wait(v, values, values, E)
    | filter_wait(v, v, values, values, E)
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

The remaining structural operations use private states. `completed` is kept in source order:

```text
map(function, items) --> map_state(function, items, [])

map_state(function, [], completed)
  --> completed

map_state(function, x::xs, completed)
  --> map_wait(function, xs, completed, function(x))

map_wait(function, xs, completed, y)
  --> map_state(function, xs, completed ++ [y])

filter(predicate, items)
  --> filter_state(predicate, items, [])

filter_state(predicate, [], completed)
  --> completed

filter_state(predicate, x::xs, completed)
  --> filter_wait(predicate, x, xs, completed, predicate(x))

filter_wait(predicate, x, xs, completed, true)
  --> filter_state(predicate, xs, completed ++ [x])

filter_wait(predicate, x, xs, completed, false)
  --> filter_state(predicate, xs, completed)

range(n) --> [0, 1, ..., n-1]
repeat(v, n) --> [v, v, ..., v] with exactly n copies
```

The last two right sides are meta-notation for one finite canonical list value. Their aggregate charge and result-node
count are checked before construction. `map` and `filter` precharge their input count, then ordinary function
application charges each callback in source order. The private states are well typed when the function, remaining input,
and completed output have the types in Section 6.7; the wait expression has the callback result type.

Their exact administrative typing rules are:

```text
f:X->Y    remaining:List<X>    completed:List<Y>
------------------------------------------------
map_state(f, remaining, completed):List<Y>

f:X->Y    remaining:List<X>    completed:List<Y>    pending:Y
----------------------------------------------------------------
map_wait(f, remaining, completed, pending):List<Y>

p:X->Bool    remaining:List<X>    completed:List<X>
---------------------------------------------------
filter_state(p, remaining, completed):List<X>

p:X->Bool    current:X    remaining:List<X>    completed:List<X>    pending:Bool
-----------------------------------------------------------------------------
filter_wait(p, current, remaining, completed, pending):List<X>
```

These private forms may occur only in evaluator states produced by the structural rules. The source checker never
accepts their spellings.

After all arguments are values, a controlled music operation reduces to one finite `Music` recipe node. In particular:

```text
map_note_pitches(function, music)
  --> MusicMapRecipe(function, music)

kernel_quote(q, [(h_i, locus_i, music_i)]_i)
  --> MusicQuoteRecipe(q, [(h_i, locus_i, music_i)]_i)
```

No pitch callback runs in either source reduction. The recipe adapter applies it during instantiation under the
invariant in `09-metatheory.md` §11.

An ordinary compiler operation with value arguments takes one abstract delta step to the unique value required by its
contract. The bounded music traversal has its own finite internal traversal and callback steps during recipe
instantiation, not source reduction.

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
operation. `Music` denotes a total function from an explicit `MusicalContext` to a finite fragment or a stated error,
but source terms cannot invoke that function. The stage boundary invokes it.

The final metatheory proves that operational evaluation reaches the value given by this denotation.

## 12. Exact embedding of the current core

The compatibility source `Old` is the current checked expression core, not an idealized subset. The embedding table is:

| Current checked form | New core form |
| --- | --- |
| literal, name, product, `Option`, list | the same corresponding form |
| simultaneous multi-argument closure | nested unary closures in parameter order |
| named/default argument application | arguments reordered and defaults inserted, then nested unary application |
| `PitchAction` | the total compatibility `pitch_move` entry selected by its checked `Pitch` or `NoteName` type |
| `Step` | a checked deferred-pitch field inside the same `Music` atom; current checking already forbids it elsewhere |
| first-order delta primitive | the same entry in `P` |
| `nat_fold`, `list_fold`, `option_fold` | the same structural form |
| `map`, `filter`, `range`, `repeat` | the explicit structural form in Sections 6.7 and 9 |
| match with old patterns | the same match and pattern |
| checked contextual `Music` expression | the same finite recipe form under the invariant in `09-metatheory.md` §11 |
| `transpose`, `stretch`, `retrograde`, `invert`, `shift`, `overlay`, `play` | the matching controlled music operation |
| `map_note_pitches` | the explicit higher-order recipe constructor in Sections 6.8 and 9 |
| checked kernel quotation | `kernel_quote` with the same checked term, holes, loci, and embedded hole expressions |

The table covers every variant of the current compiler's `ExprKind`, every one of its seven structural eliminators,
every current controlled music operation, and checked quotation. The migration bridge set covers every current base
type. The compatibility meaning of written pitch and interval uses mathematical integer coordinates, so pitch movement
is total. Every successful current machine-bounded movement embeds with the same result; a movement that currently
overflows is outside the successful-program premise below. New `Text`, `Result`, nominal data, and sealing forms have no
old preimage.

Because old multi-argument beta reduction substitutes several arguments at once while the target is curried, one old
step need not be one new step. The correct compatibility relation is a finite forward simulation:

```text
e -->Old e'  implies  embed(e) -->* embed(e')
```

The proof and its limits are in `09-metatheory.md` §10.
