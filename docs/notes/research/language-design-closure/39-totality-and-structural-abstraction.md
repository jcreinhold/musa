# Keep totality; repair structural abstraction

**Status: research. Governs nothing.** This note recommends a direction. It does not amend
[`docs/rules/`](../../../rules/README.md), and it does not authorize implementation. The repository is private and
pre-release, so compatibility and migration effort are deliberately excluded from the decision.

## 1. Decision

Keep Musa a **pure, strict, total, rank-1 Hindley–Milner language**. Do not add dependent types, general recursion,
call-by-push-value, higher-kinded constructor variables, type classes, or implicit model search.

Repair the smaller foundations that the staff adapter actually falsified:

| Question from note 38 §10 | Recommendation |
| --- | --- |
| Record update, guards/`if`, `try`/`?`, `foldr` | Add record update, ordinary expression `if`, `Result`-specific `?`, and the already-planned `list_fold_from_end`; do not add guarded pattern equations. |
| Structural `Syntax` eliminator | Replace the primitive bottom-up catamorphism with a total, path-aware, inherited-context structural recursor over sealed immediate-child steps. Derive the old fold from it. |
| Container abstraction | Accept constructor-specific operations for now. Do not add higher-kinded variables or dependent types. Pointwise enumeration remains available when an actual summary operation needs it. |
| Source totality | Keep it. The domain audit supplies no operation that needs divergence, and the actual adapter obstacles all have total repairs. |
| Dependent types | Reject them for Musa's foundation. They remove no demonstrated musical side condition and close no safety boundary that opaque types, checked constructors, and `Result` leave open. |

This is not the smallest change to the current code. It is the smallest *foundation*: make finite programs pleasant and
give the one compiler-owned tree the right total recursion principle, without changing the meaning of types or accepted
evaluation.

No recommendation here requires amending `constitution.md` or `obligations.md`. The changes that are recommended are
ordinary amendments to the candidate language specification, after the adapter evidence is complete. Dropping totality
or adopting dependent types would require the constitutional amendment procedure; §11 states those alternative costs.

## 2. How the decision was made

The unit of evidence is an operation, not a feature name. For each proposal this review asked:

1. What phenomenon must a musician or package author express?
2. Which later stages consume the result?
3. What is the exact mathematical object, and what cheaper shadow is sufficient?
4. Does the proposal remove a real side condition in two materially different musical uses, or close a safety boundary?
5. Who writes the resulting annotations, models, or workarounds?

The representative examples were:

- the implemented staff adapter, including its degenerate `Missing` and leaf-token cases;
- the specified but not implemented studio adapter;
- tonal construction and analysis;
- flexible, written, perceived, and performed time;
- phrase-led intent and ensemble-led tuning; and
- a live finite-state protocol whose history is unbounded only in the host.

The method is the phenomenon/object/law/level procedure in `theory-design`, especially its `level-audit.md` and
`theory-decomposition-patterns.md` references. The sources actually inspected are listed in §13.

## 3. Domain audit

### 3.1 The musical distinctions remain value-level and plural

The local *Open Music Theory* chapters confirm the distinctions used in
[02-five-musical-cases.md](02-five-musical-cases.md):

- Written straight eighths and performed swing are deliberately different; the performed ratio varies with tempo and
  performer (`074-swing-rhythms.md:7–29,54–59`).
- Written meter, perceived meter, simultaneous meters, metric modulation, and seconds-based timeline notation are
  different presentations (`098-twentieth-century-rhythmic-techniques.md:7–19,94–132,164–240`).
- Form is a defeasible hierarchy: levels can collapse, and segmentation is an analytical decision
  (`052-foundational-concepts-for-phrase-level-forms.md:19–35,98–128`).
- Pitch and pitch class differ, and whether enharmonic equivalence is useful depends on the repertoire and analysis
  (`099-pitch-and-pitch-class.md:10–36,92–96`).
- Pitch-class segmentation can be mathematically factual and still musically unhelpful; rhythm, meter, timbre, texture,
  articulation, and register may all determine the analyst's grouping
  (`104-analyzing-with-set-theory-or-not.md:28–56,88–97`).
- Orchestration combines material vertically and horizontally, with context-dependent choices of register, doubling,
  omission, timbre, antiphony, and dovetailing (`114-core-principles-of-orchestration.md`).

These are arguments for theory-owned data, opaque representations, explicit conversions, and named analysis results.
They are not arguments for putting a musical value into a type. In fact, a load-bearing index for one analytical view
would work against constitution §8's plural, payload-opaque event track.

### 3.2 The five pressure tests still do not need dependent types

Changing the source foundation does not invalidate the judgments in
[02-five-musical-cases.md](02-five-musical-cases.md):

| Case | Relation to enforce | Smallest adequate mechanism |
| --- | --- | --- |
| Tonal construction | a chord recipe and voicing policy may fail in a context | opaque package types, checked constructors, `Result`, explicit evidence |
| Flexible time | written, perceived, gestural, and physical time must not be identified | distinct nominal types and recorded conversions |
| Phrase-led intent | a finite phrase must satisfy theory-owned formation rules | private constructor returning `Result<Phrase, Error>` |
| Ensemble tuning | ensemble, degree, register, and acoustic target jointly determine a pair | an ordinary total function returning `Result` |
| Live protocol | each input has one finite-state response; the number of host steps is unbounded | a total `step`, with repetition in the runtime rather than source evaluation |

The question from [23-values-not-types.md](23-values-not-types.md) remains decisive: does a caller need a relation
before evaluating, does an opaque result erase information needed later, or is a small decidable index equality the only
way to protect a boundary? None of these cases answers yes. A failed constructor is not a type error delayed too long;
it is a domain result at the point where all musical evidence is available.

### 3.3 Level audit of the new proposals

| Demand | Exact object | Cheaper sufficient level | Verdict |
| --- | --- | --- | --- |
| Read a finite syntax tree with context and selective descent | an initial algebra with controlled structural recursion | a phase-local inherited-context recursor with sealed child steps | use the exact recursion principle, but keep the representation opaque |
| Summarize elements in an arbitrary finite container | a natural transformation to the free monoid `List` | a pointwise `C -> List<X>` view | add only when a real operation needs it |
| Write one `map`/`traverse`/`bind` over many constructors | a variable of kind `Type -> Type` and an explicit model | no first-order shadow preserves the varying constructor | no demonstrated cross-constructor operation; defer |
| Ensure a constructed musical value satisfies a theory-owned invariant | a proposition indexed by that value | an opaque type with a checked constructor | use the value-level boundary |
| Keep compilation responsive | termination of accepted source evaluation plus deterministic resource limits | a budget alone only bounds a run | retain both totality and budgets |

The important separation is between **structural recursion**, **container protocols**, and **failure sequencing**. The
staff adapter contains all three, but none unfolds the others. A syntax recursor does not imply `Monad`; `?` does not
imply higher-kinded variables; and neither has anything to do with general recursion.

## 4. Ergonomics: add the four small facilities

### 4.1 Record update

Add immutable update of one or more named fields of a nominal record. The semantic contract should be:

- the subject is evaluated once;
- each right-hand side is evaluated once, left to right, against the original lexical environment;
- every named field belongs to the same nominal record type;
- duplicate field names are rejected; and
- the result is the same nominal type, with every unmentioned field unchanged.

This is constructor elaboration, not row polymorphism, width subtyping, lenses, mutation, or structural records. It
removes the eight `holding_*` copies in the staff adapter without changing what a record means.

The exact spelling is not foundational. A form such as `pending with { dots = more }` is locally explicit and leaves
`{ ... }` as a block delimiter; it should be tested against the formatter before being governed.

### 4.2 `if`, not pattern guards

Add ordinary expression `if condition { then } else { otherwise }`. The core-calculus term inventory already includes
conditionals ([02-core-calculus.md](../../../rules/language/02-core-calculus.md) §1), while the surface grammar does
not. This is specification/implementation drift, not a new type-theoretic choice.

Do **not** add guards to match arms or guarded function equations in the same change. Core-calculus §6.2 deliberately
keeps patterns depth one; Peyton Jones Chapters 4–6 show that guarded equations, fall-through, and nested patterns are a
pattern-compilation subsystem. An expression `if` handles `clef_named` and similar value decisions without taking on
that subsystem. A later pattern-guard proposal must justify itself separately.

### 4.3 `Result`-specific `?`, not general `do`

Add postfix `?` with exactly one meaning: in an expression whose enclosing function or adapter operation returns
`Result<B,E>`, `e?` evaluates `e : Result<A,E>` once, yields `A` for success, and returns the same error for failure.
Different error types still require an explicit mapping.

This is a surface elaboration to exhaustive sum elimination and continuation of the enclosing expression. It introduces
no exception value, handler, mutation, I/O, implicit model, or effect. It must not be generalized through a
`Try`/`Monad` type class. If a future constructor needs propagation, it earns its own explicit operation or supplies the
evidence for the container-abstraction reopening rule in §6.4.

This feature addresses the actual 120-line `document_read` staircase. General monad abstraction would solve a much
larger problem than the program presents.

### 4.4 `foldr`

Take the design already written in [prompt 127dcfaa](../../../plan/prompts/127dcfaa-list-fold-direction.md): replace
ambiguous `list_fold` with `list_fold_from_start` and `list_fold_from_end`. The latter is the list catamorphism

```text
fold_from_end(z, s, [])       = z
fold_from_end(z, s, x :: xs)  = s(x, fold_from_end(z, s, xs)).
```

It is total and removes the closure chain used to build right-nested staff values. It does not abstract over a container
and should not be made to wait for that question.

## 5. `Syntax`: use a total inherited-context recursor

### 5.1 The phenomenon

An adapter must inspect a node before deciding whether, in what order, and under what context to process its children.
It must retain the compiler-derived path of every node it reaches. The compiler must remain the only owner of scopes,
source ranges, paths, and the raw syntax representation.

The current primitive instead evaluates every child bottom-up and gives a group branch only `List<A>`. That makes the
staff reader encode context, lookahead, order, and failure in `Pending` and closures. Making `Syntax` public data would
restore recursion but would also expose information the adapter is specifically not allowed to forge.

### 5.2 The object and its interface

Use a phase-local, path-aware recursor with explicit inherited context and this schematic shape:

```text
recurse_syntax(
  missing,
  token,
  identifier,
  group,
  initial_context,
  subject,
) -> A

group : C
     -> NodePath
     -> Delimiter
     -> List<SyntaxStep<C,A>>
     -> A

run_syntax_step : C -> SyntaxStep<C,A> -> A
```

Every branch receives the current `C`. `SyntaxStep<C,A>` is an opaque, non-storable suspended recursive call minted for
exactly one immediate proper child. It seals together that child, the current algebra, and the operation that resumes
the recursor. It has no source constructor and no operation that yields scopes, a source range, a path, raw syntax, or
its hidden algebra. `run_syntax_step(next_context, step)` passes `next_context` to the sealed child's branch; that
branch receives the child's unique compiler-derived path separately.

The intrinsic characterization is the observationally unique function `R : (C, Syntax) -> A` satisfying these equations,
where `step_R(s)` is the sealed step for proper child `s` under the same `R`:

```text
R(c, Missing(path)) = missing(c, path)
R(c, Token(path, kind, text)) = token(c, path, kind, text)
R(c, Identifier(path, name)) = identifier(c, path, name)
R(c, Group(path, delimiter, [s1, ..., sn]))
  = group(c, path, delimiter, [step_R(s1), ..., step_R(sn)])

run_syntax_step(c, step_R(s)) = R(c, s)
```

Existence and uniqueness are by structural recursion on `Syntax`: constructing a step does not enter its child, and
running it enters exactly that strict subtree. Categorically, this is the syntax catamorphism at carrier `C -> A`, made
operationally selective by suspending each recursive child call. The suspension is the exact object the adapter needs; a
child handle separated from its runner is a lossy representation because it forgets which structural-decrease proof
belongs to which call.

This remains rank 1. `C` and `A` are quantified only in the outer builtin scheme, and `SyntaxStep<C,A>` is one nominal
phase-local type constructor over ordinary type arguments. It is excluded from `d` because its hidden representation
contains the current algebra. A step may be captured in an intermediate closure and invoked later in the same phase;
that preserves association because it cannot be re-associated with another traversal. The representation property is
sealed association rather than dynamic non-escape; source termination also needs the qualification below.

**Qualification to the sealed-step revision.** Sealing proves the association lemma; it is not by itself the whole
source-termination proof. `C` and `A` may be function types whose closures capture steps, and a branch may start a
nested recursor on the original subject before running a captured outer step. In such a trace, each step still enters
the proper child that minted it, but the size of the subject currently being evaluated need not decrease at every
reduction because the nested recursor may restart from a larger tree. The §5.5 extension must therefore use the
calculus's reducibility argument: a step over child `s` is reducible when `run_syntax_step(c, step)` is reducible for
every reducible `c`, proved by induction on `s`, and the fundamental lemma must cover higher-order contexts/results,
capture, duplication, delayed use, and nested recursors. The paper trial in §12.1 is a falsifier for that proof
obligation. This qualification does not exhibit a loop or reopen the sealed representation; it separates the local
decrease lemma from the global normalization theorem that still has to be established.

**Correction to the first draft of this note.** The earlier `SyntaxChild<C,A>` plus separately supplied `descend` did
not enforce its claimed non-escape law. Nested recursors can choose the same `C` and `A`, capture a child from an outer
group, and pass it to an inner group's descender. The types then agree even though the child is not a proper child of
the inner group. At a descendant callback the captured child can denote the node being processed, turning the purported
structural call into self-descent. The ordinary occurs check sees types, not values hidden in closure environments, and
the `d` exclusion protects only the phase boundary. A dynamic owner check would need an explicit failure result because
the operation otherwise has no `A` to return; a fresh rank-2 region could prevent cross-pairing but is unnecessary once
the child and its runner are sealed into one step.

The old catamorphism is derivable with `C = Unit`: the group case runs every step with `Unit` and passes the resulting
`List<A>` to the old group algebra. It should therefore cease to be the primitive. A derived `fold_syntax` may remain as
a convenience only if its name and equations make the strict bottom-up behavior explicit.

### 5.3 Laws

The recursor needs these laws, separately from its representation conveniences:

1. **Sealed formation.** Every step is compiler-minted for one immediate proper child and the algebra of the recursor
   that exposed it; source constructs neither a step nor a replacement algebra for one.
2. **Association.** Running a step always uses its sealed child and algebra. Passing it through a nested recursor cannot
   make that recursor interpret the step as one of its own children.
3. **Inherited context.** `run_syntax_step(c, step)` supplies exactly `c` to the sealed child's branch; no ambient state
   is read or changed.
4. **Path uniqueness.** Running a step supplies exactly the structural path already assigned to its child.
5. **Structural decrease and reducibility.** Every step application enters a strict subtree of the group that minted it,
   even when the step is captured, invoked later, or invoked from a nested traversal. Strong normalization follows only
   after the reducibility candidate for steps and the fundamental lemma discharge higher-order `C`/`A`, delayed and
   repeated use, and nested recursors; it is not inferred from a globally decreasing runtime tree-size trace.
6. **Repeatability.** A step may be omitted or run finitely many times under different contexts; every run has the same
   sealed subject and algebra.
7. **Determinism.** Equal subject, algebra, initial context, and budget produce equal completed results.
8. **Opacity.** A step reveals no `SourceInfo`, scopes, raw syntax, path, or hidden algebra. Branches receive a
   compiler-derived `NodePath`; no operation forges one or reveals a `BindingPath` from the input.
9. **Fold derivation.** Running every step in source order with `Unit` is observationally equal to the current
   `fold_syntax`.
10. **Budget accounting.** Minting and running a step are charged by versioned structural rules; capture and repeated
    use do not hide free work.
11. **Phase conservativity.** Ordinary source cannot name or obtain `Syntax` or `SyntaxStep`, the completed phase result
    is storable data, and the transformer uses the same rank-1 checker and evaluator as ordinary source.

### 5.4 Representative programs

- **Staff.** Take `C` to be the current meter/open-form state and `A` to be the reader's result. A layout group can
  choose right-to-left processing for ties and right-nested output, then pass the updated state explicitly to each
  selected child. No `Context -> Result` encoding is required by the traversal interface.
- **Studio.** A node declaration can inspect its header before selecting the parameter grammar for its body. The actual
  trial must establish whether selective descent helps or whether a plain derived fold remains clearer.
- **Source-preserving edit.** The adapter can stop after the anchored child it needs, while the path law still
  identifies the edit locus.
- **Degenerate leaf.** `Missing`, token, and identifier branches receive no child step. Their termination argument is
  immediate.

This is more general than the current irritation but no more general than structural recursion on the compiler's one
finite tree. It does not add a cursor with arbitrary parent/sibling navigation, general recursion, quotation, fresh
names, or a public syntax type.

## 6. Containers: accept monomorphic operations for now

### 6.1 Why higher kinds are not earned

The current evidence names three different demands:

- right-folding a `List`;
- sequencing `Result`; and
- controlling recursion over `Syntax`.

The recommendations above solve them with three direct mechanisms. No algorithm in the implemented staff adapter is
shown running unchanged over two different type constructors. The paper studio adapter also uses lists, `Result`, and a
syntax fold; a second use of the *same* `Result` propagation justifies `?`, not abstraction over `M : Type -> Type`.

Thus the reopening rule in constitution §9 and obligations §10 is not met for higher-kinded variables. Adopting them
because `Functor`, `Traversable`, and `Monad` are familiar would be the analogy-first move that the rule forbids.

### 6.2 What remains available without higher kinds

- Each strictly positive nominal data declaration keeps its generated catamorphism.
- `List` gets both directionally named folds, plus its existing `map` and `filter`.
- `Option` keeps its eliminator and explicit convenience functions where they have callers.
- `Result` gets ordinary constructors/elimination and the `?` surface elaboration.
- A real summary operation may receive a pointwise enumeration `C -> List<X>`. Because `List<X>` is the free monoid,
  this one view derives `length`, `any`, `sum`, and other monoidal summaries without pretending to be a general
  container calculus.

Do not generate `Listing`/`Building` records for every data declaration in advance. Generation is justified when a
consumer exists and when the declaration has a canonical element order. A syntax tree, a graph description, and a
pitch-class set do not all have the same canonical enumeration merely because they are finite.

### 6.3 Why dependent types are not the container answer

Dependent types make a constructor kind an ordinary Π-type, but that is incidental power. They also replace Algorithm W,
make conversion depend on term normalization, and allow values to enter types. Buying that foundation to quantify over
`F` is the Overconstrained Hypothesis anti-pattern: the container operation needs a constructor variable, not dependent
elimination.

### 6.4 Reopening rule

The leading reopening candidate is **higher-kinded constructor variables with explicit, pointwise model arguments and no
search**. Reopen only after recording:

1. one complete musical or adapter algorithm that is duplicated solely to run over two distinct constructors; and
2. a second materially different algorithm needing `map`, `traverse`, or `bind` across constructors.

At least one must involve a user-declared constructor; otherwise compiler-owned conveniences may still be the smaller
interface. The trial must compare source, diagnostics, inferred types, and laws under monomorphic and explicit-model
versions.

If reopened, remain in the constructor-pattern fragment: no type families, type-level computation, constructor lambdas,
or implicit instance search. In rank 1, model records must be **pointwise**, for example `Mapping<F,A,B>`, because a
single record field polymorphic in `A` and `B` would itself be rank 2. Laws remain explicit package/generated-law
obligations; internal parametricity and cohesion are much larger foundations than this demand.

### 6.5 Cost of this refusal

Musa will have several similarly shaped functions, and a library algorithm that genuinely wants constructor polymorphism
must currently be duplicated. That is an accepted cost, not a claim that duplication is good. The foundation remains
reopenable before release when the required programs exist.

The benefit is that musicians see no model arguments, package authors do not manufacture naturality evidence, and the
checker retains the principal rank-1 inference it already promises.

## 7. Source totality: keep it

### 7.1 Note 38's separation is correct

[Note 38](38-abstraction-totality-and-substitution.md) is right that source strong normalization, residual event-track
normalization T4, and one machine step M1 are three independent theorems. Finite musical values do not logically imply a
total language for computing them. A partial metalanguage with a budget could emit a finite closed track.

That observation removes a bad argument for totality; it does not supply an argument for general recursion.

### 7.2 The proposal that needs evidence is non-termination

Totality is the current constitutional invariant. The domain-first question is therefore not “which musical object needs
totality?” but “which required source operation cannot be expressed by structural or well-founded recursion and
therefore needs possible divergence?” No reviewed case provides one:

- staff expansion is structural over finite syntax;
- studio expansion and validation are structural over a finite declaration list and graph description;
- tonal, phrase, tuning, and analysis constructors consume finite values and return finite values or `Result`;
- flexible time preserves underdetermined *performance choices* as data rather than searching forever for one; and
- bomba's unbounded interaction is repeated total stepping by the host, not one diverging source evaluation.

The staff implementation is not counterevidence after record update, `if`, `?`, `fold_from_end`, and the syntax recursor
are separated from totality. Five of the six causes in note 38 were already unrelated to totality; the sixth is also
handled by a total structural recursor.

### 7.3 A budget is not a replacement theorem

A deterministic budget protects the compiler process, and it should remain. It does not make a partial declarative
evaluation relation total. With general recursion:

- some well-typed terms have no value;
- a function returning `Result<A,E>` may produce neither branch;
- normalization cannot justify equality or accepted evaluation; and
- “increase the budget” becomes a semantic debugging question for code that may never finish.

The existing budget-independence law says that two *completed* runs agree. Strong normalization additionally says every
well-typed closed source term has a completion in the unbounded semantics. These are different guarantees, and Musa has
no demonstrated use that pays for losing the latter.

### 7.4 The right future extension is still total

The current fold-only surface may eventually be too weak for finite graph algorithms, search with a decreasing measure,
or mutually recursive theory data. If two real programs demonstrate that, first test compiler-checked structural or
well-founded recursion. Total languages routinely express those algorithms. “More total recursion principles” and
“general recursion” are different decisions.

Live coding could be a genuine falsifier, but only if a written Musa construct itself must denote an open-ended history.
Constitution §4 already gives that event an explicit reopening rule. Until such a program exists, keep open-ended
execution in `Machine` and the host.

## 8. Dependent types: reject them

### 8.1 The old reasons were weak; the conclusion is still right

Note 38 correctly rejects two shortcuts in the recorded rationale:

- dependent checking is not “HM with worse inference”; it is a different bidirectional/checking discipline; and
- inline argument lambdas often receive enough expected type to avoid annotations.

Neither point is positive evidence for dependent types. The decisive reasons are domain fit and permanent cost:

1. No two materially different musical operations need a value in a type.
2. No safety boundary is currently unstated. Storable-data admission, machine ports, time coordinates, opaque package
   types, checked constructors, and explicit conversions already state the reviewed boundaries.
3. Musical classifications are often theory-, repertoire-, method-, and context-owned. Turning one into an index makes
   its choice load-bearing in every operation and proof.
4. Higher kinds, the only concrete foundational benefit proposed in note 38, can be added directly if earned.
5. Dependent types replace the checker, equality, evaluator support for conversion, diagnostics, and the metatheory.
   That is not a library feature that can later be ignored.

The annotation cost against `examples/` is not decision-grade evidence. That corpus is deliberately small, and adapter
entries such as `let expand = fn (region) { ... }` rely on contextual inference outside the measured examples. A real
trial would have to migrate `stdlib/`, adapter interfaces, public package signatures, and error snapshots, then count
new annotations and compare error locality. There is no reason to run that expensive trial until a domain use exists.

### 8.2 Do not import adjacent dependent foundations by analogy

- Musa's phase-local adapter environment is **not two-level type theory**. The inspected 2LTT source has two full type
  theories, inner and outer, with distinct type formers/equalities and a conversion from inner to outer. Musa has one
  rank-1 theory under two name environments.
- The `d` class is **not a grade or modality** in the cited graded-modal sense. Those grades are elements of a semiring
  or lattice assigned to variable use, erasure, information flow, or effects. Musa's `d` is a structural admissibility
  predicate on type shapes: no function at any depth plus a finite exact encoding.
- Internal parametricity would make naturality expressible, but the cited cubical/cohesive systems add modalities and a
  new metatheory. External generated-law tests are proportionate to an explicit container model; internal parametricity
  is not.

### 8.3 What Musa gives up

Musa cannot state `Vec<A,n>`, a proof that a transformation preserves `n`, a type of well-connected graphs indexed by
their ports, or a phrase type indexed by a proof of its formation rule. Those facts remain constructor results,
abstract-type invariants, or external laws. If a later consumer must manipulate such evidence *before evaluation* and
two musical uses survive the plurality audit, dependent types can be reopened with that evidence.

## 9. The fire triangle and the combinations actually available

### 9.1 Correction to the premise of this review

Pédrot and Tabareau do prove that **observable effects + substitution + dependent elimination** are inconsistent
(`fire-triangle.../text.md:51–110`). But their Definition 3 is specific, and the paper immediately says that it **does
not apply to non-termination**, printing, or unhandled exceptions because the type theory cannot reason on those effects
(`:117–119`).

Therefore this statement is false as an attribution to the paper:

> non-termination is an effect, so dropping totality plus dependent types is exactly the theorem's forbidden
> combination.

Non-termination is an effect in the broader programming-language sense. It is not an observable effect under that
paper's no-go hypothesis. This correction matters because the literature contains models of dependent types with
recursion rather than a theorem that they cannot coexist.

### 9.2 The design trilemma remains real

The corrected result does not make “partial dependent Musa” simple. The available combinations are:

| Combination | Available? | Price |
| --- | --- | --- |
| Pure total terms + substitution + full dependent elimination | Yes | a total dependent theory and its termination/conversion machinery |
| Effectful CBV + dependent elimination + substitution restricted to values | Yes | types depend on values; arbitrary computations cannot substitute into them |
| Effectful CBN + substitution + restricted dependent elimination | Yes | weak/non-dependent elimination or a storage/linearity discipline |
| Full dependent sequencing of effectful computations | Research systems exist | explicit value/computation stratification such as dCBPV/∂CBPV, dependent Kleisli extension, and effect-specific subject-reduction conditions |
| Observable effects + unrestricted substitution + full dependent elimination in a consistent logic | No | the fire-triangle theorem |
| General recursion + dependent programming, with logical consistency abandoned or a partial layer separated | Yes in existing languages/models | no strong normalization; type-level dependence and definitional equality must still be restricted or stratified |

Vákár's dCBPV work is especially precise about divergence. It says recursion/non-termination admits dependent Kleisli
extension in the studied model (`effectful-treatment.../text.md:241–252,394–404`), but leaves a type-checking algorithm
to future work and hopes for decidability **without recursion** (`:572–587`). The later thesis chapter is more explicit:
allowing types to depend on dynamic computations would make non-termination make type checking undecidable
(`in-search.../text.md`, §5.1).

So the user's practical trilemma is directionally right after correction:

- keep totality; or
- restrict what types may depend on and how dependent elimination/substitution works; or
- reopen a value/computation stratification such as CBPV.

There is also a fourth, unattractive choice: call the result a dependently typed programming language, permit general
fixpoints to destroy the logical reading, and give up complete normalization-based checking. A deterministic fuel budget
can make one implementation return, but it does not restore the lost theorem or determine a coherent dependent
equational theory.

For Musa the result is simple: keep totality, do not adopt dependency, and leave the CBPV rejection intact. There is no
fire triangle to resolve in the recommended language.

## 10. Corrections and qualifications to note 38

These are corrections to [38-abstraction-totality-and-substitution.md](38-abstraction-totality-and-substitution.md), not
silent changes to its findings.

### Correction A: no search still does not imply per-type eliminators

Note 38's Correction 1 ends with:

> totality plus no instance search implies per-type eliminators.

That is still false. Higher-kinded variables plus an **explicit model argument** have total structural operations and no
instance search. The accurate statement is:

> first-order type variables plus neither explicit protocol/model abstraction nor an associated-type-like projection
> imply constructor-specific operations.

Totality controls recursive calls. Search controls how a model is selected. Constructor abstraction controls whether the
model's type can be written. They are independent axes.

### Correction B: the table calls enumeration “fold”

In §3.4, the row named `fold` has type `C -> List<X>`. That is enumeration/to-list, not a fold. The subsequent universal
property is sound: an enumeration into the free monoid determines every monoidal summary. The operation should be named
correctly so it is not conflated with the catamorphism analyzed in §3.1.

### Correction C: a reusable explicit model can cost rank 2

The §4 sketch `Folding<F>` leaves the element and accumulator variables polymorphic inside a record field. In Musa's
rank-1 system that reusable dictionary is not directly expressible even after adding `F : Type -> Type`; the field would
be rank 2. A rank-1 design must pass pointwise models such as `Folding<F,X,A>` or generate a family of ordinary
polymorphic top-level values. This does not defeat explicit models, but it is part of their cost.

### Correction D: a budget does not decide dependent conversion

§6 says that if source totality is dropped, no termination checker is needed and the existing budget covers divergent
type-level computation. A budget can make an implementation stop. It does not provide a complete, budget-independent
decision procedure for definitional equality or type checking. The dCBPV source inspected here leaves type checking
future work and states its decidability hope only without recursion. Any partial dependent design must say whether
conversion is incomplete, effectful terms are excluded from types, or a total static fragment is introduced.

### Correction E: de Bruijn syntax is an implementation option, not a consequence

§7 calls de Bruijn indices in syntax and levels in semantics “the standard setup.” That is a common NbE design, and the
cited CBPV NbE paper uses intrinsically typed nameless syntax and presheaves. It is not forced. McBride and McKinna give
a mixed representation with names for free variables and indices for bound variables, specifically to keep systematic
construction out of raw index arithmetic. A dependent checker would need open neutrals, weakening/renaming, and quote;
it would not by that fact require Musa's source AST to become de Bruijn syntax.

### Correction F: the studio prompt is 127dcg

§10 says the studio adapter is prompt 127dd. The implementation trial is
[127dcg-studio-trial.md](../../../plan/prompts/127dcg-studio-trial.md). Prompt
[127dd-adapter-trials.md](../../../plan/prompts/127dd-adapter-trials.md) is the subsequent freeze, proof, and hostile
review.

### Qualification G: the annotation measurement is too narrow

§6 correctly says “measured against the corpus,” but its conclusion is based on `examples/` and a few inline lambdas.
That supports rejecting the claim that dependent types necessarily impose a large annotation tax. It does not establish
that the tax is approximately zero for the language Musa is about to ship. The stdlib, phase-local adapter signatures,
user-declared generic data, public package boundaries, and diagnostic snapshots were not migrated.

### Editorial correction

The introduction says note 38 corrects three analyses, while the note labels Corrections 1 through 4. Its README entry
already says four.

The fire-triangle correction in §9 is a correction to the premise supplied for this review, not to note 38, which did
not discuss that paper.

## 11. Governing, proof, and prompt costs

No file under `docs/rules/` should change from this research note alone. If the user accepts the direction, repair the
plan first, run the trials, then amend the candidate language pages before implementation.

| Recommendation | Governing documents that would change | Proof work | Prompt impact |
| --- | --- | --- | --- |
| Record update, expression `if`, `Result ?` | `language/01-surface.md`; the elaboration and source-map portions of `language/02-core-calculus.md` and `language/05-verification.md` | typing/elaboration preservation; single evaluation and source-map laws; no new core reduction for desugared forms | add one ergonomics prompt before the remaining adapter trials; repair grammar/formatter/tree-sitter/tooling evidence from 80 and 122; rerun staff evidence before 127dcfb |
| Directional list folds | `language/01-surface.md` and `language/02-core-calculus.md` as prompt 127dcfaa already states | one list-decrease case in preservation/progress/determinism/normalization | 127dcfaa remains valid and should run independently of container abstraction |
| Structural `Syntax` recursor | add the phase-local rules to `language/00-semantics.md` and `language/02-core-calculus.md`, plus adapter laws in `language/05-verification.md`; constitution §9 remains satisfied | typing and canonical forms for non-storable `SyntaxStep`; sealed-association and local structural-decrease lemmas; the reducibility candidate and fundamental-lemma cases for higher-order `C`/`A`, capture, duplication, delayed use, and nested traversal; determinism; path uniqueness; fold derivation; budget law; phase conservativity; differential evaluator coverage | 127da's “fold is the only way in” design is superseded and needs a repair prompt; 127dcfa must be rewritten/retrialed; 127dcfb and 127dcg should use the repaired API; 127dd, 127e, 153, and 146 must wait for the repaired evidence |
| Constructor-specific container operations | no governing change | no proof restatement | no prompt invalidation; do not add generated `Listing`/`Building` without a caller |
| Keep source totality | no governing change | retain §5.5 strong normalization independently of T4 and M1; extend it for the new recursor/fold only | 153 and 146 keep their source-termination rows |
| Reject dependent types | no governing change | no checker or metatheory replacement | 127aa, 127b, 153, and 146 remain rank-1/principal-inference prompts |

The historical prompt 127ac remains correct for ordinary user-declared data: one generated catamorphism is the general
eliminator. `Syntax` is exceptional because it is an opaque compiler-owned phase value whose proper-child relation and
paths are hidden. The repair must say this explicitly rather than silently turning every data declaration into a visitor
API.

### 11.1 What would change if totality were dropped

This is not recommended. It would require the amendment procedure in [`docs/rules/README.md`](../../../rules/README.md):

- replace constitution §9's **Total** rule with a rule saying that pure, deterministic source evaluation may diverge,
  while every compiler run is stopped by a versioned resource budget and completed runs remain budget-independent;
- amend obligations §10 with the two concrete operations that require possible divergence and their smallest failing
  total terms;
- remove source strong normalization from `language/02-core-calculus.md` §5.5 and from prompts 153/146, without touching
  T4 or M1; and
- restate every API currently promising a value or `Result` so that divergence/resource exhaustion is an additional
  outcome.

A budget limit is then part of acceptance, even if not part of completed-value equality. That semantic cost must be
stated, not described merely as an implementation safeguard.

### 11.2 What would change if dependent types were adopted

This is not recommended. It would require:

- replacing constitution §9's rank-1 HM rule with an exact bidirectional dependent-checking rule and deleting the
  explicit refusal of dependent/refinement types;
- choosing whether types may depend only on values, whether dependent elimination is weak, or whether CBPV
  stratification is adopted; adopting the latter also removes the explicit CBPV refusal;
- amending obligations §10 with two musical uses or one otherwise unstated safety boundary;
- replacing Algorithm W, principal types, and the closed-value environment proof with conversion, open neutrals,
  reification, weakening/renaming, and the chosen normalization/partiality theorem; and
- repairing prompts 127a, 127aa–127b, 153, and 146, plus every language/tooling prompt whose diagnostics assume
  principal inferred types.

Adopting dependent types *and* general recursion would additionally have to state which row of §9.2 Musa takes. “The
budget handles it” is not a type-system design.

## 12. What remains undecided

### 12.1 The exact recursor surface

The sealed-step equations and structural-decrease principle are the recommendation. The exact names, argument order,
whether a step is invoked by `run_syntax_step` or callable syntax, whether child paths appear only at the resumed
branch, and whether the derived bottom-up fold remains public should be settled by rewriting the staff adapter and
implementing the studio adapter on paper before governance.

The ordinary adapter falsifiers remain: if either adapter must recover raw `Syntax`, forge a path, or encode the same
`Pending`/closure machine after the recursor is available, the interface is wrong.

Add one hostile nested-traversal trial before governance. An outer group captures one of its steps; an inner recursor
uses the same `C` and `A`, restarts on the original or an ancestor subject, and runs that outer step from its group
branch. The run must use the step's sealed outer algebra and proper child and never reinterpret the step as an inner
child. Exercise function-valued `C` and `A` whose closures capture a step, delayed invocation after the group callback,
and repeated invocation under two contexts. Then discharge the reducibility argument rather than claiming that a single
runtime tree-size measure decreases across the fresh nested recursor. If the fundamental lemma cannot cover those terms
without a dynamic owner mismatch, a failure result, rank-2 regions, affinity, or general recursion, the interface is
wrong.

### 12.2 Higher-kinded constructor variables

They are rejected for the current foundation, not proved useless forever. Evidence that would reopen them is the pair of
cross-constructor programs in §6.4, with a pointwise explicit-model prototype and no search. Mere repetition of
`Result ?` in both adapters is not that evidence.

### 12.3 More total recursion

Finite graph search, mutually recursive musical grammars, or an analysis whose termination measure is not constructor
size may justify checked well-founded recursion. Record the smallest failing fold program before choosing syntax or a
termination checker. This evidence would not by itself justify general recursion.

### 12.4 Is the studio trial worth waiting for?

**Yes for freezing the adapter API; no for the totality/dependent-type decision.** Prompt 127dcg is deliberately the
second materially different adapter and must run against the repaired recursor before prompt 127dd freezes anything. It
can falsify the recursor's shape and expose missing everyday ergonomics.

It is unlikely to change totality or dependent types: its specified product is a finite graph description and its
validation is a total function over finite declarations. If implementation discovers a genuinely non-structural
operation, record that smallest term rather than attributing the difficulty to the whole adapter.

The studio trial alone also cannot earn higher-kinded abstraction unless it demonstrates one algorithm reused over a
different constructor. Add a targeted container trial only if duplication actually appears.

## 13. Sources inspected

### Repository

- [`AGENTS.md`](../../../../AGENTS.md) and [`docs/README.md`](../../../README.md).
- [`constitution.md`](../../../rules/constitution.md) §§4, 7–9; [`obligations.md`](../../../rules/obligations.md) §10;
  and the amendment procedure in [`rules/README.md`](../../../rules/README.md).
- [`language/02-core-calculus.md`](../../../rules/language/02-core-calculus.md) §§1, 1.1, 5, 5.5–5.8, 6.1–6.2.
- [02-five-musical-cases.md](02-five-musical-cases.md),
  [19-inference-course-correction.md](19-inference-course-correction.md),
  [22-syntax-extension.md](22-syntax-extension.md), [23-values-not-types.md](23-values-not-types.md),
  [24-pipeline-and-syntax-review.md](24-pipeline-and-syntax-review.md),
  [26-language-design-decision.md](26-language-design-decision.md), [27-adapter-trials.md](27-adapter-trials.md),
  [33-metatheory.md](33-metatheory.md), and
  [38-abstraction-totality-and-substitution.md](38-abstraction-totality-and-substitution.md).
- The implemented `Syntax` fold, evaluator, and closure representation in `crates/musa-compiler/src/core/mod.rs`; the
  rank-1 occurs and storable-data checks in `crates/musa-compiler/src/infer.rs`; the staff reader in
  `stdlib/src/adapters/staff.musa`; and prompts 127a, 127aa–127b, 127da, 127dcfa–127dd, 127e, 153, and 146.

### Music theory

The local files actually read under `~/Code/papers/music-theory/open-music-theory/` were:

- `008-texture.md`, `009-notating-rhythm.md`, `052-foundational-concepts-for-phrase-level-forms.md`,
  `074-swing-rhythms.md`, `083-rhythm-and-meter-in-pop-music.md`, `098-twentieth-century-rhythmic-techniques.md`,
  `099-pitch-and-pitch-class.md`, `101-pitch-class-sets-normal-order-and-transformations.md`,
  `104-analyzing-with-set-theory-or-not.md`, and `114-core-principles-of-orchestration.md`.

They were used for distinctions and counterexamples, not as a universal ontology. The Karnatak, gamelan, and bomba
judgments remain limited pressure tests and retain note 02's specialist-review warning.

### Type theory

The local `text.md` files actually inspected under `~/Code/papers/logic-and-computation/type-theory/` were:

- `fire-triangle-how-to-mix-substitution-dependent-elimination-and-effects`;
- `call-by-push-value-decomposing-call-by-value-and-call-by-name`;
- `effectful-treatment-of-dependent-types` and `in-search-of-effectful-dependent-types-ch-5`;
- `normalization-by-evaluation-for-call-by-push-value-and-polarized-lambda-calculus`;
- `functional-pearl-i-am-not-a-number-i-am-a-free-variable`;
- `two-level-type-theory`;
- `graded-modal-dependent-type-theory-with-a-universe-and-erasure-formalized` and
  `quantitative-program-reasoning-with-graded-modal-types`;
- `logical-relations-as-types`; and
- `internal-parametricity-for-cubical-type-theory` and `parametricity-via-cohesion`.

The fire-triangle and dCBPV claims in §9 are restricted to what those sources state. In particular, the no-go theorem is
not cited for non-termination.

### Functional-language and module design

The local sources inspected were Peyton Jones, *The Implementation of Functional Programming Languages*, Chapters 3, 5,
6, 8, and 9; Ousterhout, *A Philosophy of Software Design*, Chapters 4, 7, and 8; and Hickey, *Simple Made Easy*. They
support enriching the one language with direct constructs, keeping the pattern compiler separate from expression `if`,
and pulling traversal complexity behind the narrow phase-owned recursor. They do not decide the musical domain.
