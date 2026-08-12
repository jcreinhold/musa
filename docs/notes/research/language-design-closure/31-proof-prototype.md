# Proof prototype: try to break the rules first

## Purpose

This note tries to falsify the claims in notes 29 and 30 before a polished proof hides a bad definition. It tests the
main failure modes from earlier reviews, compares two termination proofs, and fixes the assumptions for the final proof.

The candidate survives these attacks. Several broader alternatives do not.

## 1. What must be proved

The proof gate asks for ten results:

1. expansion terminates and is deterministic;
2. expansion preserves hygiene and source attribution;
3. expansion does not depend on inferred types;
4. resolution and inference terminate, and successful inference returns a principal type;
5. evaluation preserves types, makes progress, is deterministic, and terminates;
6. hidden constructors remain under their owner's control;
7. checked adapter output receives the same safety guarantees as handwritten ordinary code;
8. closing `Music` returns an error or a finite well-typed temporal term;
9. typed stage records compose only at exact equal anchors; and
10. the five paper programs use only rules covered by the proof.

The proof does not cover package resolution, persistent identity, or cache correctness.

## 2. First attack: can a closed typed term get stuck?

### 2.1 Annotations

An earlier draft admitted this term but omitted the evaluation context around its annotation:

```text
((let x = true in x) : Bool)
```

The term was not a value and no rule could step it. Note 29 now includes `(E : τ)` and reduces the inner `let` before
erasing the annotation:

```text
((let x = true in x) : Bool)
-> (true : Bool)
-> true
```

This counterexample is closed.

### 2.2 Ordered matches

A runtime `match_fail` value would refute progress for any type that did not include it. Copying the fallback branch can
also change logical charges.

The new lowering does neither. Coverage proves that the residual final row is irrefutable. Local joins share a fallback
without becoming values. Join erasure maps a join to an ordinary non-recursive function and a tail jump to one complete
call, so a typed join cannot invent a stuck state.

### 2.3 Compiler operations

If a compiler operation could return a function and wait for more arguments, the value grammar and termination proof
would need a private partial-application case. It would also reintroduce the surface shortcut that caused the earlier
proof loop.

The operation judgment accepts one complete product and returns one first-order value. A source function can wrap a
complete operation explicitly:

```musa
let up_octave = fn(music): transpose(music, P8)
```

No private partial value is needed.

## 3. Second attack: can inference lose principal types?

### 3.1 Type-directed fields

If `x.name` could mean any visible field spelled `name`, inference would have to choose a record type while solving the
same constraints that choice creates. Ad hoc field overloading does not have the ordinary Hindley–Milner principal-type
property.

Note 29 resolves a unique `FieldId` before inference. Ambiguous source writes the qualified selector. The inferred type
then follows from one fixed scheme.

### 3.2 Type-directed syntax

If an adapter received an expected type, expansion could change the expression that creates the expected type. Expansion
and inference would become mutually recursive, and the usual Algorithm W proof would no longer apply.

The adapter API has no expected-type argument and runs before inference. Two later type environments cannot change the
same expansion input. Principal inference therefore sees one fixed ordinary expression.

### 3.3 Named, default, and partial calls

These forms are not unsafe by themselves, but their old rules made the accepted call shape depend on parameter names,
declaration defaults, and which arguments remained. Intervening defaults produced the exact counterexample that ended
the previous proof.

The new function type has one product domain. The call either supplies it or fails. Configuration records and explicit
closures cover the useful cases without a second call calculus.

### 3.4 Polymorphic recursion

Let-bound polymorphism with recursion can make inference undecidable. The language has no recursive value binding, so
the problem does not arise. A generated data fold has one fixed rank-1 scheme and recursively visits values at the same
data parameters.

## 4. Third attack: can source evaluation loop?

### 4.1 Hidden recursion in data

Strict positivity alone is not enough if an implementation admits infinite or cyclic values. Musa values are immutable
finite constructor trees, and data contains no lazy field or pointer-building operation.

The broader declaration:

```musa
type Bad:
  Bad(Bad -> Nat)
```

is rejected because `Self` occurs under an arrow. Mutually recursive nominal declarations are also rejected. The
accepted recursive positions are products, `List`, `Option`, and `Result`, so each generated fold makes recursive calls
only on proper subvalues.

### 4.2 Higher-order compiler operations

The assumption “every primitive terminates on terminating arguments” is too weak if a primitive can call a source
closure arbitrarily. A primitive could implement a loop by repeatedly invoking its function argument.

Compiler operations are first-order. Higher-order work is expressed through source `fold` and ordinary functions, both
covered by the calculus. This closes the old higher-order primitive loophole.

### 4.3 Join cycles

A recursive local join would be a hidden fixed point. The join table is lexical and acyclic; a body may jump only to an
earlier surrounding join, and the lowering algorithm itself emits no backward cycle. Join erasure therefore produces
only non-recursive function bindings.

### 4.4 Resource limits

A finite term may require more work than a chosen limit. That is not nontermination. The evaluator computes one next
result and charge, then either commits it or returns a deterministic phase diagnostic. For a fixed limit it therefore
finishes even sooner than the unbounded mathematical evaluation.

## 5. Fourth attack: can expansion loop or capture names?

### 5.1 Direct self-expansion

An adapter that emits itself would loop:

```text
A(block) -> A(block)
```

The adapter graph rejects that self-edge. More generally, every emitted adapter has lower rank. Replacing one rank by a
finite multiset of lower ranks strictly decreases the multiset measure.

### 5.2 Expansion inside adapter definitions

The termination argument would be circular if proving adapter totality required the expansion theorem. Adapter
definitions therefore contain no adapter region. They use fixed Musa syntax plus the six transformer builders. Ranks
order only the adapter calls that expanded output may contain.

### 5.3 Capturing a use-site name

Suppose an adapter introduces `helper`, while the use site already binds `helper`. Text-only substitution can capture
one or the other.

`definition_name` gives `helper` the adapter's definition scope. Existing input syntax keeps its use-site scopes. A
local helper receives the scope determined by the explicit expansion context and binder slot. Resolution compares scopes
as well as text, and package code cannot forge or erase them. The three names remain distinct.

### 5.4 Lying about source

An adapter cannot give a new node an arbitrary original range. Builder nodes receive
`Generated(expansion id, node path)`. Existing input nodes retain their source info. The compiler, not the package,
creates checked wrappers, contexts, scopes, anchors, and generated ids.

Following parent expansion records terminates because there are finitely many expansion steps. Each generated node
therefore reaches an original use site.

## 6. Fifth attack: can `Music` hide a second language?

### 6.1 An open context dictionary

An earlier design let `Music` request arbitrary key, metre, tuning, notation, or package context. That is difficult to
type, hides dependencies, and lets every package extend a compiler-owned world.

The active design rejects that dictionary. Staff realization, phrase context, ensemble tuning, and timing choices are
ordinary package arguments supplied before `Music` is built. Closing receives only placement, score scope, and source
root.

### 6.2 Storing source closures

If `Music` stored arbitrary source functions, close termination and equality would inherit every closure detail. The
private recipe instead stores a fixed finite grammar. An admitted first-order `ScoreMapId` rewrites a finite recipe
before close, unfolding any references in the selected argument and recording the map on each rewritten fact. Source
functions can compute a recipe, but the recipe itself contains no source closure or pending payload map.

### 6.3 Free shared references

`Use(name)` outside its `Share` would make closing partial in an unstated way. `ValidMusic` rejects free, forward, and
cyclic names before close. Closing checks again and returns `MusicError` rather than getting stuck.

### 6.4 Running audio inside `Music`

A processor graph has state and may run forever. A `Music` recipe has finite exact musical extent. The types and stage
operations keep them apart. Only finite gestures and preparation inputs cross into the process stage.

## 7. Two termination routes

### 7.1 Route A: translate into a known total core

One can translate the source into a polymorphic lambda calculus with algebraic data and primitive recursors:

```text
source functions       -> product-domain lambdas
let generalization     -> type abstraction and application
nominal data           -> target inductive data with rigid names
generated fold         -> primitive recursor
ordered match          -> target case plus erased joins
compiler operation     -> total first-order constant
```

The target is strongly normalizing by the standard reducibility proof for System F with strictly positive inductive
types.

This route is sound, but it is not short. Musa's currently proved expression core does not contain the new nominal
recursive data, rank-1 type abstraction, or general recursors. A faithful translation would first have to define and
prove a new target calculus. That duplicates much of the direct proof.

### 7.2 Route B: prove reducibility directly

Define `R_τ(v)`, “value `v` is reducible at type `τ`”:

- base values are reducible;
- a product or record is reducible when each field is;
- a nominal constructor value is reducible when its non-recursive fields are reducible and every recursive child is
  reducible;
- a function is reducible when applying it to reducible arguments terminates in a reducible result; and
- a term is reducible when it terminates in a reducible value.

For recursive nominal data, the clause is an inductive definition over the finite constructor tree. The generated-fold
case is proved by induction on that tree. For type variables, the proof quantifies over an arbitrary candidate relation,
which gives the usual rank-1 polymorphic fundamental lemma.

Every term former has one local case. Joins are erased to non-recursive functions before the argument. Compiler
operations satisfy a stated reducibility contract. The fundamental lemma then says that substituting reducible values
for a well-typed term yields a reducible term. Closed well-typed terms terminate.

### 7.3 Choice

Use Route B. It has fewer translations and exposes the exact assumptions on recursive data and compiler operations. Use
one small join-erasure lemma inside it. Keep Route A as an independent explanation of why the result is unsurprising,
not as the formal dependency.

## 8. Exact assumptions for the proof

The final proof assumes:

1. the package graph and all declaration tables are finite and well formed;
2. source bytes and compiler options are finite;
3. build-local ids are fresh and table lookup is functional;
4. nominal declaration dependencies are acyclic except for one permitted polynomial self-reference;
5. source values are immutable finite trees and closures;
6. value definitions and lowering-only join dependencies are acyclic, and join erasure finishes before evaluation;
7. each compiler operation is first-order, total, deterministic, type preserving, and reducibility preserving;
8. each syntax wrapper, scope id, generated source id, and resolved declaration id is compiler-authenticated;
9. each adapter definition contains no adapter region and has passed the source and transformer rules;
10. each `ScoreMapId` is total, deterministic, preserves admitted fact templates, and its finite recipe rewrite obeys
    the explicit reference-unfolding rule;
11. temporal-kernel typing and normalization obey their governing specification; and
12. later stage passes return only records accepted by the governed derivation registry.

These are implementation contracts, not conclusions smuggled into the theorem. Assumption 10 gives one checked rewrite
step; it does not assert that an arbitrary recipe is valid or that close succeeds. The close proof still handles every
remaining recipe constructor and every stated error.

## 9. Prototype verdict

No counterexample in this note refutes the narrowed design. The proof should proceed directly, with explicit lemmas for:

- Algorithm W and unique field resolution;
- substitution and operation contracts;
- match compilation and join erasure;
- reducibility of generated folds;
- rank-decreasing expansion;
- constructor visibility under hygienic adapters;
- structural `Music` close; and
- exact-anchor path composition.

The most important negative result is also clear: broad macros, open `Music` context, partial compiler values, and
general recursive data would each add a real proof obligation. None helps the five programs, so none belongs in this
language.
