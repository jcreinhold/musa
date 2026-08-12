# Proof prototype

**Purpose:** try the main proof routes quickly, keep the failed route visible, and expose missing assumptions before the
formal proof is frozen.

## 1. What needs proof

The proposed language adds finite data, exhaustive matches, `Text`, `Result`, and sealed structures to Musa's current
terminating expression core. The risky claim is termination. Preservation and progress are standard once constructor
types, match coverage, and private constructor metadata line up.

Two proof routes are plausible:

1. translate the new data into the old core; or
2. prove termination directly by assigning a meaning to each type.

The direct proof is shorter. The translation attempt below explains why.

## 2. Translation attempt

One might translate a two-constructor type into `Result`:

```text
data Choice {
  Left(value: A);
  Right(value: B);
}
```

could become `Result<A, B>`. A three-constructor type could become `Result<A, Result<B, C>>`.

This does not translate into the *already proved* core because `Result` is one of the new forms. Encoding a sum with the
old `Bool`, products, and `Option` types needs a tuple such as:

```text
(tag: Bool, left: Option<A>, right: Option<B>)
```

That tuple admits bad states: both fields may be `None`, or both may be `Some`. Every translated match then needs a case
that source typing had ruled out. Filling that case requires a made-up default value or a target error that the source
did not have.

The translation can be repaired by first extending the target with a proved binary sum. At that point the translation
proof and the direct proof have the same hard case. The translation adds tags and nested cases but removes no proof
obligation.

**Decision:** keep the direct proof. Do not claim a clean translation into the old core.

## 3. The direct termination measure

Each user-defined type has a rank. A leaf data type has rank 1. A type that contains an earlier nominal type has one
plus the greatest rank it contains. Cycles are rejected.

For any type `A`, define:

- `r(A)`: the greatest nominal rank appearing in `A`, or 0 if none appears;
- `s(A)`: the number of nodes in the written type; and
- `m(A) = (r(A), s(A))`, ordered lexicographically.

This measure falls in both recursive parts of the type interpretation:

- the element type of `List<A>`, `Option<A>`, or `Result<A, E>` has the same or lower rank and smaller type size;
- every field of a nominal type `mu` has nominal rank strictly below `rank(mu)`, even when the field has a larger
  written shape such as `List<Option<Earlier>>`.

The leaf rank must be 1. If a leaf nominal had rank 0, its field `List<Text>` could have the same nominal rank and a
larger type size, so the lexicographic argument would point the wrong way.

## 4. Reducible values

Say a closed term terminates when call-by-value evaluation reaches a value in finitely many steps. Write this as
`e downarrow v`.

Define `R_A(v)`, meaning “`v` is a good value of type `A`,” by induction on `m(A)`:

- a base, bridge, or `Music` value is good when it is a well-typed canonical value;
- a product is good when every field is good;
- `None` is good, and `Some(v)` is good when `v` is good;
- a finite list is good when every element is good;
- `Ok(v)` and `Err(e)` are good when their contents are good;
- a nominal value `C(v_1, ..., v_n)` is good when every constructor field is good; and
- a function value is good when applying it to any good argument terminates at a good result.

A closed term `e: A` is reducible when `e downarrow v` and `R_A(v)`.

This definition is well founded by Section 3. The function clause refers to smaller component types, as in the usual
proof for the simply typed lambda calculus. The nominal clause refers only to lower-ranked nominal contents.

## 5. Prototype of the fundamental lemma

**Claim.** If `Gamma |- e: A` and a closing substitution gives every variable in `Gamma` a good value of its declared
type, then the closed instance of `e` is reducible at `A`.

**Proof sketch.** Induct on the typing derivation.

- A variable is good by the substitution premise.
- A literal is already a good value.
- A product, constructor, or structural value is good after applying the induction hypotheses to its fields.
- A function is good because extending the substitution with a good argument lets the induction hypothesis apply to its
  body.
- An application terminates because the function induction hypothesis maps the good argument to a good result.
- A `let` uses the result for its bound expression in the induction hypothesis for its body.
- A match first reduces its subject to a canonical value. Exhaustiveness selects one arm. Substituting the good fields
  into that arm gives a good result by its induction hypothesis.
- A list fold terminates by induction on the finite list. Each step uses the good step function and accumulator.
- An ordinary compiler operation terminates and returns a good result by its first-order totality contract. The admitted
  music transform constructs one finite recipe node; its later instantiation traverses the finite occurrence bound.

The empty substitution gives termination for every closed, well-typed term.

## 6. Fast attacks on the claim

### 6.1 A higher-order foreign loop

Suppose the compiler could register:

```text
bad: (Unit -> Unit) -> Unit
```

and implement `bad(f)` by calling `f(())` forever. Its type alone would not expose the loop. The termination theorem
would be false.

**Repair adopted:** ordinary compiler operations are first order and satisfy a checked totality contract. All seven
structural operations stay inside the language semantics. The one existing higher-order music transform,
`map_note_pitches`, constructs a finite source recipe without calling its function. The recipe adapter later visits a
finite occurrence bound and applies the total function once per documented pitch position. The registry is not open to
arbitrary higher-order operations.

### 6.2 A nominal cycle through a list

This declaration looks finite at the outer constructor but permits values of unbounded depth:

```text
data Tree {
  Node(children: List<Tree>);
}
```

It breaks the rank definition.

**Repair already present:** nominal dependency search looks through products, `Option`, `List`, and `Result`; it rejects
this self-edge.

### 6.3 A cross-module value cycle

Checking only siblings misses:

```text
A.x = B.y
B.y = A.x
```

**Repair already present:** the value dependency graph covers the whole resolved build.

### 6.4 A missing `Result` type

`Ok(3)` does not determine its error type. A synthesizing rule would make checking non-functional or invent a type.

**Repair already present:** `Ok` and `Err` need an expected `Result` type or an annotation.

### 6.5 Sealing that erases too much

If sealing deletes all constructor information, the compiled body of `Box.size` can no longer match `Packed`.

**Repair already present:** clients receive a public environment without constructors; compiled bodies retain a private
data environment.

### 6.6 A hidden musical context

If `Music` reads a global key or tuning during evaluation, the same source term can change meaning without a changed
input. Evaluation determinism and stage records would both lie.

**Repair already present:** theory context is an ordinary source value. The later musical context is an explicit input
to `instantiate`.

### 6.7 A source stream

Adding a stream constructor or effectful input operation would make the source termination theorem false or force it to
exclude common programs.

**Repair already present:** a source program defines one finite protocol step. The host owns unbounded repetition.

## 7. Other proof obligations

The remaining claims need less machinery:

- **Decidable checking:** every syntax tree and resolved graph is finite; name lookup, cycle tests, rank calculation,
  structural type equality, and finite coverage tests all decide an answer.
- **Preservation:** substitution, constructor inversion, and primitive soundness cover every reduction rule.
- **Progress:** canonical forms plus exhaustive matching cover every closed term. Clients may use a wildcard or
  whole-value binder for an abstract value, but only compiled bodies with the private constructor table may use a
  constructor pattern.
- **Determinism:** evaluation contexts choose one leftmost call-by-value redex; each redex has one rule; compiler
  operations are deterministic.
- **Unforgeability:** a client cannot resolve a private constructor, so no accepted client core term can contain that
  constructor except inside an imported compiled body.
- **Old-fragment preservation:** the exhaustive embedding table covers every old core form. Old simultaneous calls may
  take several curried target steps, so the proof uses a finite forward simulation rather than identical reductions.
- **`Music` closure:** source construction must first preserve the finite private recipe invariant. Instantiation and
  closing then prove the error-or-closed-term result from the smaller atom and transform contracts.
- **Stage composition:** this follows only for passes whose derivation records satisfy the accepted cross-stage rules.

## 8. Prototype verdict

The language appears provable with one direct logical-relations argument and ordinary safety lemmas. The proof needs no
dependent indices, CBPV, worlds, links, stable package identity, or cache theorem.

The final proof must keep every contract above explicit. Hiding one as “the compiler does the right thing” would turn a
conditional theorem into a false unconditional claim.
