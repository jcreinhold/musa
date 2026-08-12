# Proof outline

**Purpose:** fix the exact statements and the order of the final proof.

This outline removes the loose claims found during the prototype. Every theorem below names the assumptions it uses. The
final proof may shorten the prose, but it may not strengthen a conclusion without adding a proof.

## 1. Objects and judgments

Let:

- `G` be one finite resolved import graph;
- `S` be its finite source declarations;
- `N` be the fresh build-local names assigned to user data;
- `Delta` map each private nominal name to its constructors and field types;
- `Sigma` be the public module environment after sealing;
- `Gamma` map value names and variables to types; and
- `P` be the finite registry of compiler operations.

The proof assumes:

1. `G` has one node for each resolved package instance and repeated imports of that instance share the node.
2. Nominal names are distinct within the build. Consistent renaming of all fresh names preserves lookup.
3. Data dependencies and value dependencies are finite and acyclic.
4. Every ordinary operation in `P` is first order, pure, deterministic, total on closed well-typed arguments, type
   preserving, finite, and charged by a fixed rule. All seven structural operations use their language rules. The sole
   admitted higher-order music operation stores its function in a finite recipe; the later adapter traversal satisfies
   the finite occurrence-bound rule stated in `04-source-calculus.md`.
5. The accepted temporal kernel and cross-stage specifications satisfy their governing theorems.

The main source judgments are:

```text
G |- source resolves to (N, Delta, Sigma, core)

Delta; Sigma; Gamma |- e => A
Delta; Sigma; Gamma |- e <= A

Delta |- e --> e'
Delta |- e -->* v
```

The reduction relation is the unmetered left-to-right call-by-value relation. Resource charging is proved as a separate
refinement.

## 2. Definitions needed by the proofs

### 2.1 Well-formed types

A type is well formed when every nominal name is present in `Delta` or abstractly declared in `Sigma`, and each type
former has well-formed arguments. A field type is well formed only when it contains no function, `Music`, timeline, or
process type and every nominal name in it precedes the enclosing data declaration.

### 2.2 Substitution

`e[v/x]` is capture-avoiding substitution of the closed value `v` for the free variable `x` in `e`. Build-local nominal
names are constants, not variables, and substitution does not change them.

### 2.3 Public and private terms

A client term is public when all its names resolve in `Sigma` and the public value environment. A compiled structure
body may also use constructors in its retained slice of `Delta`. Sealing checks the body before hiding those
constructors.

### 2.4 Canonical forms

A canonical-forms lemma states what a closed value of each type can be. Examples:

- a `Bool` value is `true` or `false`;
- an `Option<A>` value is `None` or `Some(v)`;
- a `Result<A, E>` value is `Ok(v)` or `Err(w)`;
- a value of nominal type `mu` is one constructor declared for `mu`; and
- a function value is a closed lambda or named closure.

### 2.5 Rank and reducibility

Use the lexicographic measure from the prototype:

```text
m(A) = (greatest nominal rank in A, written type size of A)
```

Leaf nominals have rank 1; rank 0 means that no nominal occurs. Define good values and reducible terms by induction on
this measure. The function clause says that a good function sends every good argument to a reducible result.

## 3. Lemma order

The final proof follows this order:

1. finite graph algorithms terminate;
2. weakening and lookup stability;
3. substitution;
4. canonical forms;
5. preservation;
6. progress;
7. unique evaluation-context decomposition;
8. determinism;
9. well-foundedness of reducibility;
10. fundamental reducibility lemma;
11. source termination;
12. sealing and unforgeability;
13. metered evaluation refinement;
14. retained-fragment translation and open-term comparison;
15. `Music` closure under adapter contracts; and
16. typed stage composition under the governing derivation contracts.

This order prevents termination from being used to prove ordinary type safety and keeps stage assumptions outside the
source calculus.

## 4. Exact theorem statements

### Theorem 1. Decidable resolution and checking

Given finite `G`, finite source `S`, and a finite operation registry `P`, the resolver and bidirectional checker return
either one typed core result or one finite list of errors in finite time.

The theorem covers duplicate checks, graph cycles, ranks, structural type equality, and flat-pattern coverage. It does
not cover finding or version-solving packages; `G` is already resolved.

### Lemma 2. Substitution

If:

```text
Delta; Sigma; Gamma, x:A |- e : B
Delta; Sigma; Gamma |- v : A
```

and `v` is a value, then:

```text
Delta; Sigma; Gamma |- e[v/x] : B
```

The single colon abbreviates either a completed synthesis judgment or a checking judgment with its stated type.

### Theorem 3. Preservation

If `Delta; Sigma; empty |- e : A` and `Delta |- e --> e'`, then `Delta; Sigma; empty |- e' : A`.

### Theorem 4. Progress

If `Delta; Sigma; empty |- e : A`, then either `e` is a value or there is a unique `e'` such that `Delta |- e --> e'`.

The uniqueness part may be split into the determinism theorem, but the final proof must not imply that exhaustiveness
alone gives a unique step. Evaluation order also matters.

### Theorem 5. Determinism

If `Delta |- e --> e_1` and `Delta |- e --> e_2`, then `e_1 = e_2`.

This theorem relies on deterministic compiler operations. It is about the unmetered semantics.

### Theorem 6. Source termination

If `Delta; Sigma; empty |- e : A`, then there is a value `v` such that `Delta |- e -->* v`.

This theorem applies only to the source core. It does not say that a process graph or live run reaches a final value.

### Theorem 7. Sealed constructors cannot be forged or inspected

Let structure `M` define nominal type `M.T` with constructors hidden by signature sealing. If client source resolves and
checks using `Sigma`, no client-originated core node names a private constructor, constructs `M.T` with one, or uses a
constructor pattern to inspect it. A wildcard or binder match remains legal because it reveals no representation.

Imported compiled bodies may construct or match `M.T` using the retained private environment. The theorem does not claim
that an untyped byte string cannot be maliciously passed to an unsafe runtime decoder; no such decoder is part of this
calculus.

### Theorem 8. Retained expressions keep their results

Let `Gamma |- e_old translates_to e_new : A` have a derivation under the rules in `04a-formal-rules.md` §12. If old and
new environments bind each name in `Gamma` to related values, then successful old evaluation ends at `v_old`, new
evaluation ends at `v_new`, and the two values are related at `A`. Therefore, for a closed translated expression:

1. `e_new` has the same type as `e_old`;
2. if old evaluation succeeds, new evaluation reaches a related final value.

The translation accepts complete compiler calls. It accepts an ordinary partial call only when the supplied arguments
form a parameter prefix. It rejects compiler-operation values and non-prefix ordinary partial calls. Named curried
wrappers retain both dynamic operation parameters and deliberately reordered ordinary parameters. The theorem does not
promise that future surface syntax reserves no new keywords. It also does not cover the later task of moving built-in
musical concepts into packages.

The result does not compare individual old and new reduction steps or resource charges. Those details belong to two
different language versions. In particular, the new proof does not recreate the old evaluator's private partial
`BuiltinValue` state.

### Theorem 9. Closing `Music`

First prove the private recipe invariant for every atom, composition, controlled transform, mapped-pitch recipe, checked
quote, sounded voicing, and acyclic reference. Then prove that instantiation returns a stated error or a finite
well-formed fragment and that closing returns a stated error or a closed, well-typed `Term<ScoreFact>`.

Then for every closed, well-typed source expression `music: Music` and well-formed explicit context `c`, evaluation,
application, and closing finish with either a stated error or a finite, closed, well-typed temporal term.

The conclusion remains conditional only on the finite atom and transform contracts listed by the recipe invariant; their
checked table is the implementation witness. Source typing alone cannot prove an unlisted foreign adapter safe.

### Theorem 10. Typed stage passes compose

Assume two adjacent successful pass results satisfy the accepted cross-stage formation rules and the second consumes the
exact stored output anchor of the first, including `PresentationRef` and local anchor id. Then exact-anchor path
concatenation produces a well-formed composite path. Ordered concatenation of the two already valid loss lists remains a
valid list; no normalization law is assumed.

This theorem applies repeatedly along any finite source-to-preparation path. For an unbounded audio run it applies to
each finite prefix; it does not create a final infinite value.

## 5. Metered evaluation

The implementation judgment is:

```text
<e, budget, log> ==>* <outcome, budget', log'>
```

Each step charges a fixed event before taking the corresponding unmetered reduction. The proof needs two statements:

1. **repeatability:** equal checked terms, budgets, logs, operation versions, and registries give equal outcomes and
   final states; and
2. **successful erasure:** if metered evaluation succeeds with `v`, unmetered evaluation reaches the same `v`.

A small budget may reject a term that a larger budget accepts. No theorem should compare those different initial states.
No cache premise appears.

## 6. What the proof does not claim

The final proof will not claim:

- complete type inference;
- principal types;
- stable nominal identity across builds;
- compiled-artifact or execution-cache correctness;
- correctness of a package resolver;
- adequacy of any culture-specific theory package;
- equality of arbitrary functions or `Music` values;
- equivalence between notation and performance; or
- normalization of an unbounded audio history.

These limits are part of the result, not loose ends hidden by notation.
