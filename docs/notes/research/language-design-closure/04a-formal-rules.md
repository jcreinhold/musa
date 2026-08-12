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
Keeping this set lets retained current expressions translate before that separate source migration occurs.

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
| a complete direct call `f(a, b)` | fresh ordered argument `let`s followed by `(f(a_value))(b_value)` |
| a prefix direct call `f(a)` when `f` has later parameters | `f(a)` with the curried type of the remaining parameters |
| an indirect function call | unary application `function(argument)` |
| a bare nullary constructor `C` | `C()` |
| a saturated constructor call `C(a, b)` | `C(a, b)` |
| `[a, b]` | `a :: b :: []` |
| `fold_list(items, initial, step)` | `list_fold(initial, step, items)` |
| `fold_nat(count, initial, step)` | `nat_fold(initial, step, count)` |
| `fold_option(item, initial, step)` | `option_fold(initial, step, item)` |
| `if condition then yes else no` | `match condition { true => yes; false => no; }` |
| a complete ordinary compiler call `p(a_1, ..., a_n)` | `primitive(p, a_1, ..., a_n)` |
| a complete controlled music call other than `map_note_pitches` | `music_operation(name, a_1, ..., a_n)` |
| `map_note_pitches(function, music)` | the core form of the same name |

A source function declaration provides all parameter and result types. Resolution tells a constructor, ordinary
function, and compiler operation apart, so these rows do not rely on capitalization or spelling. An operation name is
not a term and is not entered in `Gamma`. A compiler operation must receive exactly its declared arguments in one call.
It cannot be returned, passed, or partly applied. A musician can expose the same behavior as a value by writing an
ordinary source function whose body makes the complete call. No derived form adds a reduction rule.

A complete direct call may name arguments and omit declared defaults. Resolution puts the arguments in parameter order,
inserts each default in its declaration scope, and binds each resulting expression once before applying the curried
function. A direct partial call is valid only when it supplies exactly the first `k` parameters and no later one. It
inserts no default after the first missing parameter. Supplying a later parameter while skipping an earlier one is an
error. Section 12 gives the expansion with fresh names.

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
P(name) = op(A_1, ..., A_n) => B
Gamma |- e_i <= A_i for every i
-------------------------------- Primitive
Gamma |- primitive(name, e_1, ..., e_n) => B
```

Ordinary entries satisfy the first-order contract. Structural folds and the named bounded music traversal use their
separate rules and cannot enter through this general rule.

The current controlled music operations use this finite table:

| Name | Type |
| --- | --- |
| `transpose` | `op(Interval, Music) => Music` |
| `stretch` | `op(Ratio, Music) => Music` |
| `retrograde` | `op(Music) => Music` |
| `invert` | `op(Pitch, Music) => Music` |
| `shift` | `op(Duration, Music) => Music` |
| `overlay` | `op(Music, Music) => Music` |
| `map_note_pitches` | `op(Pitch -> Pitch, Music) => Music` |
| `play` | `op(Voicing, Duration) => Music` |

The `op` notation is not a source function type. It describes one complete compiler call. The source operation always
returns a finite recipe. An invalid stretch factor or held duration becomes a stated error when that recipe is
instantiated. Existing failing diagnostics remain an implementation migration obligation, not a new source value.

Every controlled operation other than `map_note_pitches` uses this rule:

```text
MusicOps(name) = op(A_1, ..., A_n) => Music
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
nat_fold(v_z, v_step, 0)       --> v_z
nat_fold(v_z, v_step, n+1)     --> (v_step(n))(nat_fold(v_z, v_step, n))

list_fold(v_z, v_step, [])          --> v_z
list_fold(v_z, v_step, v_x::v_xs)  --> list_fold((v_step(v_x))(v_z), v_step, v_xs)

option_fold(v_z, v_step, None)      --> v_z
option_fold(v_z, v_step, Some(v_x)) --> v_step(v_x)
```

These equations use finite canonical naturals and lists. The `v_` prefixes require the initial value, step function, and
structural input to be values before the equation applies. The evaluation contexts reduce them in their stated order.

The remaining structural operations use private states. A metavariable beginning with `v` ranges over values. `v_done`
is a canonical list kept in source order:

```text
map(v_function, v_items) --> map_state(v_function, v_items, [])

map_state(v_function, [], v_done)
  --> v_done

map_state(v_function, v_x::v_xs, v_done)
  --> map_wait(v_function, v_xs, v_done, v_function(v_x))

map_wait(v_function, v_xs, v_done, v_y)
  --> map_state(v_function, v_xs, v_done ++ [v_y])

filter(v_predicate, v_items)
  --> filter_state(v_predicate, v_items, [])

filter_state(v_predicate, [], v_done)
  --> v_done

filter_state(v_predicate, v_x::v_xs, v_done)
  --> filter_wait(v_predicate, v_x, v_xs, v_done, v_predicate(v_x))

filter_wait(v_predicate, v_x, v_xs, v_done, true)
  --> filter_state(v_predicate, v_xs, v_done ++ [v_x])

filter_wait(v_predicate, v_x, v_xs, v_done, false)
  --> filter_state(v_predicate, v_xs, v_done)

range(n) --> [0, 1, ..., n-1]
repeat(v, n) --> [v, v, ..., v] with exactly n copies
```

The entry rules apply only after their arguments are values. The wait rule for `map` applies only after its callback
result is a value. Until then, the evaluation context steps the pending expression. The two `filter_wait` rules apply
only to the canonical Boolean values. These premises prevent an administrative rule from competing with a step inside an
argument or callback.

The last two right sides are meta-notation for one finite canonical list value. Their aggregate charge and result-node
count are checked before construction. `map` and `filter` precharge their input count, then ordinary function
application charges each callback in source order.

Their exact administrative typing rules are:

```text
v_f:X->Y    v_remaining:List<X>    v_done:List<Y>
-------------------------------------------------
map_state(v_f, v_remaining, v_done):List<Y>

v_f:X->Y    v_remaining:List<X>    v_done:List<Y>    pending:Y
----------------------------------------------------------------
map_wait(v_f, v_remaining, v_done, pending):List<Y>

v_p:X->Bool    v_remaining:List<X>    v_done:List<X>
---------------------------------------------------
filter_state(v_p, v_remaining, v_done):List<X>

v_p:X->Bool    v_current:X    v_remaining:List<X>    v_done:List<X>    pending:Bool
-----------------------------------------------------------------------------
filter_wait(v_p, v_current, v_remaining, v_done, pending):List<X>
```

The `v_` premises are part of state formation, not comments about an implementation. These private forms may occur only
in evaluator states produced by the structural rules. The source checker never accepts their spellings.

After all arguments are values, a controlled music operation reduces to one finite `Music` recipe node. In particular:

```text
map_note_pitches(v_function, v_music)
  --> MusicMapRecipe(v_function, v_music)

kernel_quote(q, [(h_i, locus_i, v_music_i)]_i)
  --> MusicQuoteRecipe(q, [(h_i, locus_i, v_music_i)]_i)
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

## 12. Retained fragment and breaking changes

The retained fragment is defined by the syntax-directed judgment:

```text
Gamma |- e_old translates_to e_new : A
```

The judgment says that current checking gives `e_old` type `A` and the displayed rule produces `e_new`. A term belongs
to `Old_retained` exactly when this judgment has a derivation. Thus the domain contains no accepted old term for which
the translation is undefined.

`Gamma` uses resolved binding identities, not just written names. For a direct named call it includes the called
function and every free binding used by one of that function's defaults. A caller's local spelling therefore cannot
capture a name from the declaration scope.

For a direct call to a named ordinary function with parameters `x_1:A_1, ..., x_n:A_n`, resolve argument names to
parameter slots before translation. A call has one of two retained shapes.

1. A complete call supplies every required slot. Put supplied arguments in parameter order and fill omitted defaults. A
   default for `x_i` may refer only to `x_1, ..., x_(i-1)`. Fresh `let` bindings evaluate each supplied argument or
   default once, in that order, before nested unary application.
2. A prefix call supplies exactly slots 1 through `k`, where `k < n`, and no later slot. Apply the translated nested
   unary function to those `k` arguments. Do not insert a default after the first missing slot.

A call that supplies a later slot while skipping an earlier one has no translation rule. A compiler operation has only
the complete-call rule. These are deliberate source errors in the refined language.

Here is the exact expansion. `g_new` is the nested unary function made from the named declaration. For a complete call,
let `b_i` be the supplied argument for slot `i`, or that parameter's checked default when the slot is omitted. Translate
a supplied `b_i` in the caller's environment. Translate a default in its declaration environment plus the earlier
parameters, then replace those parameters with `y_1, ..., y_(i-1)`. The expansion is:

```text
let y_1 = b_1_new;
...
let y_n = b_n_new;
g_new(y_1)...(y_n)
```

All `y_i` are fresh. For a prefix call with supplied arguments `a_1, ..., a_k`, the expansion is just:

```text
g_new(a_1_new)...(a_k_new)
```

Both expansions are defined only when the named function declaration, every supplied argument, and every used default
have translation derivations. The declaration translation keeps the lexical environment in which each default was
checked. Thus `expand_complete` cannot hide an untranslated subterm or evaluate a default in the caller's scope.

The translation judgment has the following two call rules, where `expand_complete` and `expand_prefix` are the
deterministic expansions just defined:

```text
current_check(g(args)) = A    every required slot is supplied or has a default
expand_complete(g(args)) = e_new
------------------------------------------------------------------------ TranslateCompleteCall
Gamma |- g(args) translates_to e_new : A

current_check(g(args)) = A    supplied slots are exactly 1, ..., k    k < n
expand_prefix(g(args)) = e_new
-------------------------------------------------------------------------- TranslatePrefixCall
Gamma |- g(args) translates_to e_new : A
```

An indirect function value uses the unary core application rule and has no argument names or defaults. Translation for
the other term forms is homomorphic as listed below: each immediate old term must have a translation premise, and the
new form is rebuilt from those translated terms. A compiler call uses a third rule whose checking premise requires all
declared slots, with no inserted default, and whose result is one complete `primitive`, `music_operation`, or
`map_note_pitches` node.

The remaining translation rules are:

| Current checked source form | New core form |
| --- | --- |
| literal, name, product, `Option`, list | the same corresponding form |
| simultaneous multi-argument closure | nested unary closures in parameter order |
| complete named/default call of an ordinary source function | fresh ordered `let` bindings for supplied arguments and defaults, then nested unary application |
| prefix call of an ordinary source function | nested unary application of exactly the supplied prefix |
| named complete compiler call | arguments reordered, then one `primitive` or `music_operation` node |
| `PitchAction` | the total compatibility `pitch_move` entry selected by its checked `Pitch` or `NoteName` type |
| `Step` | a checked deferred-pitch field inside the same `Music` atom; current checking already forbids it elsewhere |
| first-order delta primitive | the same entry in `P` |
| `nat_fold`, `list_fold`, `option_fold` | the same structural form |
| `map`, `filter`, `range`, `repeat` | the explicit structural form in Sections 6.7 and 9 |
| match with old patterns | the same match and pattern |
| checked contextual `Music` expression | the same finite recipe form under the invariant in `09-metatheory.md` §11 |
| complete calls of `transpose`, `stretch`, `retrograde`, `invert`, `shift`, `overlay`, `play` | the matching controlled music operation |
| `map_note_pitches` | the explicit higher-order recipe constructor in Sections 6.8 and 9 |
| checked kernel quotation | `kernel_quote` with the same checked term, holes, loci, and embedded hole expressions |

The judgment covers every retained current form, every one of its seven structural eliminators, every complete
controlled music call, and checked quotation. It deliberately omits partial compiler calls and non-prefix partial calls
of ordinary functions. The migration bridge set covers every base type in the retained fragment. The compatibility
meaning of written pitch and interval uses mathematical integer coordinates, so pitch movement is total. Every
successful current machine-bounded movement embeds with the same result; a movement that currently overflows is outside
the successful-program premise. New `Text`, `Result`, nominal data, and sealing forms have no old preimage.

The comparison is between accepted source programs and their final values. It does not relate each private step of the
current evaluator. Even a complete current call may pass through a private partial `BuiltinValue`; preserving that state
would defeat the reason for this repair. The theorem therefore proves equal checked types and related successful
results, not a lockstep or finite-step simulation of current evaluator states.

The repository audit found four fixed rejected operation shapes: `transpose(fixed_interval)`, `stretch(fixed_ratio)`,
bare `retrograde`, and `invert(fixed_pitch)`. Each use can be replaced by a named source function with the same declared
arrow type. For example:

```musa
fn up_octave(subject: Music) -> Music {
    transpose(P8, subject)
}
```

The governing semantics also writes `transpose(i): Music -> Music` for an arbitrary `i`. An ordinary named function
keeps that behavior without making the operation itself a value:

```musa
fn transposer(interval: Interval, subject: Music) -> Music {
    transpose(interval, subject)
}

let answer: Music -> Music = transposer(runtime_interval);
```

The prefix call captures `runtime_interval` in an ordinary source closure. The refined language also rejects a current
ordinary call such as `choose(second: true)` when it skips the first parameter. A wrapper can put the captured
parameters first. The implementation migration gate must find both rejected call classes from the resolved syntax tree;
this research audit is evidence, not a universal migration theorem.
