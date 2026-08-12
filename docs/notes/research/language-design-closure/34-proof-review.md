# Proof review of the inferred language and stage safety proof

## Verdict

**Incorrect.** The ordinary rank-1 value calculus is a promising small core, but the theorem proved in note 33 is not a
theorem of the rules displayed in notes 29 and 30. Four High-severity gaps independently block promotion:

1. the adapter transformer language has no formal quotation, antiquotation, or syntax-construction terms;
2. hygienic fresh-name generation contradicts the pure deterministic operation contract;
3. typed `join` and `jump` terms have no reduction relation; and
4. the `Music` proof uses a term-level payload map that the governing temporal calculus deliberately does not have.

The first two gaps mean that the adapter examples are not programs in the proved language. The third supplies a closed,
well-typed stuck core term. The fourth gives a valid shared recipe on which the stated closing proof has no correct
case. There are also Medium-severity gaps in adapter ranking, logical charges, recipe validity, pass-result composition,
and the literal program audit.

This verdict is about the frozen paper design at commit `3eaa13123ec140a328958a88a77e6bfdb801c03b`. The notes correctly
say that the candidate is not the current implementation. Missing implementation is not counted as a proof defect.

## Findings

### High: the adapter functions are not terms of the source calculus

Note 29 section 6 says that its expression grammar is exhaustive. It has variables, data, functions, complete calls,
matches, folds, compiler operations, and annotations. It has no syntax quotation, antiquotation, syntax splice, quoted
identifier, or fresh-binder form. The typing rules, reduction rules, Algorithm W proof, substitution proof, and
reducibility proof have no cases for any of them.

Nevertheless, note 29 section 8.4 says that an ordinary checked function

```text
BlockSyntax -> Result<ExprSyntax, SyntaxError>
```

constructs its result with quotation, antiquotation, and fresh names. This cannot be treated as an omitted library
definition. `ExprSyntax` is an authenticated wrapper whose constructors package code cannot call. Note 27 names

```text
quote_expr: QuotedExpression -> ExprSyntax
```

but never defines `QuotedExpression` as a type or defines a term that constructs one. The paper adapters also need to
turn source locations into the `Anchor` values placed in their expansions, but the displayed common API has
`source_of: Syntax -> SourceInfo` and no checked `SourceInfo -> Anchor` operation.

This makes the appeal to the ordinary source theorem circular. Lemma 9.1 says that a rank-zero adapter's source is
handled directly by the source theorems, while Theorem 9.6 performs induction on “quoted syntax.” There is no such
source derivation or inductive grammar in the frozen calculus. In particular, the staff and studio notes display desired
expansion outputs and prose folding algorithms, not complete transformer terms that inhabit the displayed language.

The smallest honest repair is to choose one of two designs and prove it:

- define a separate, finite transformer calculus with exact quotation, splicing, binder, anchor, typing, evaluation,
  size, hygiene, and source-map rules; or
- extend the ordinary source grammar with those forms and add every missing W, substitution, progress, determinism,
  charge, and reducibility case.

The first design is cleaner. Quotation is compiler-phase machinery, and forcing it into the ordinary value calculus is
what currently makes the proof pretend that a load-bearing language is a library.

### High: `fresh_name` cannot satisfy both freshness and the operation semantics

The contradiction remains even if quotation is added. Note 27 gives

```text
fresh_name: Text -> Identifier
```

and note 29 says it receives a new opaque scope. But note 29 also says that every compiler operation is a total
deterministic mathematical function of its value arguments, and that adapters cannot read or mutate compiler state.
Consider one adapter evaluation containing:

```text
let a = fresh_name("temporary") in
let b = fresh_name("temporary") in
(a, b)
```

As a deterministic function `Text -> Identifier`, both calls return the same identifier. As a freshness operation, the
second call must return an identifier distinct from the first. A hidden global counter can make them distinct, but then
the result is not a function of the displayed argument and the adapter reads mutable compiler state.

The same issue affects Theorem 9.4's exact equality claim. Freshness alone does not imply repeatability. Two conforming
runs may choose different fresh `ExpansionId` and scope ids, and expanded-syntax equality in note 30 observes exact
source information and scopes. The theorem does not pass a name supply, require a canonical allocation function, or
quotient results by renaming.

A repair should not hide a state monad in `op`. A natural interface would make freshness a property of quotation
binders: the compiler derives a scope from a stable expansion coordinate and a local quotation-node coordinate, and
references to the quoted binder use that same authenticated coordinate. Alternatively, pass and return an explicit
linear supply and include it in the operational and charge semantics. Then state whether expansion equality is exact or
only up to a proved renaming.

### High: a closed, well-typed `join` term is stuck

Note 29 gives typing rules for `join` and `jump`, but its evaluation-context grammar contains neither form and its
reduction rules contain no transition for either form. Section 10.5 describes an evaluator carrying a lexical table `J`,
but the formal relation used throughout notes 32 and 33 remains `e -> e'`; it is not a relation on configurations
containing `J` and a captured environment.

The exact counterexample is:

```text
join j(x : Unit) : Nat = 0 in jump j(())
```

The displayed `Join` and `Jump` rules type this closed term as `Nat`. It is not a value. No displayed evaluation context
or reduction rule steps it. Therefore Theorem 5.2 (progress), Lemma 5.3 (unique decomposition), Theorem 5.4 (determinism
as proved), and the direct join case of the normalization proof do not hold for the stated relation.

Lemma 6.3 does not close the gap. It proves a result-only erasure fact later in the dependency order; it does not define
`e -> e'`, and its proof presupposes the missing claim that a jump and a complete call perform the same evaluation.

There are two small coherent repairs:

- make join erasure part of lowering and define the evaluation core to contain no joins; prove erasure preserves types,
  results, and logical charges before invoking source safety; or
- define configurations such as `J ; e -> J' ; e'`, including lexical entry, captured environments, scope exit, jump,
  contexts, and charging, then prove preservation and unique decomposition for configurations.

The first is simpler and matches the proof's existing strategy.

### High: `MapPayload` has no semantics compatible with selective shared use

Lemma 10.3 closes a `MapPayload` child and then invokes “the governed term payload map.” The governed term calculus
explicitly has no `map` term. It says payload transformation occurs above the kernel. The current Rust host helper
`Term::map_payloads` is not the missing semantics: it rewrites literal payloads and deliberately does nothing at a
`Var`.

This valid recipe isolates the failure:

```text
Share(
  x,
  Fact([0, 1], a, source),
  MapPayload(f, Use(x, use_site), map_site),
)
```

For an admitted map `f`, closing should share the definition and produce `f(a)` at the use. Closing the child of
`MapPayload` first produces a reference to `x`. A literal-payload walk sees no payload at that reference, so it leaves
`a` unchanged. Mapping the whole enclosing `let` instead maps the shared definition and therefore also changes uses
outside this `MapPayload`; that is a different error.

The conclusion of Theorem 10.4 is likely recoverable, but not from the stated case. Define an exact recipe-to-term
translation that carries the active composition of score maps. At a `Fact`, apply it before constructing the final
`ScoreFact`; at a `Use`, encode the active map and its origin step in the marked instantiation, or deliberately inline a
mapped use. Then prove that nested map order, sharing, marked provenance, and admitted payload equality agree. Do not
call that operation a governed kernel term map.

### Medium: adapter rank orders emitted calls, not definition-time expansion

The descriptor graph in note 29 records adapters that an adapter's *output may call*. Lemma 9.1 silently uses the same
rank to claim that expanding and checking the adapter's *definition* invokes only lower ranks. Those are different
dependency relations.

For example, let adapter `A` emit no adapter region, so its displayed rank is zero, while the source defining `A` uses a
syntax block for adapter `B` to construct a helper expression. Nothing in the descriptor rule forbids this. Lemma 9.1
then calls `A` rank zero and says its definition needs no adapter theorem, although checking it requires `B`.

Record definition-time syntax dependencies separately and require their graph to be well founded, or require adapter
definitions to be written in an adapter-free bootstrap language. If the two dependency graphs are intentionally one, the
descriptor and acceptance rule must say so rather than switching meanings in the proof.

### Medium: the logical charge is undefined for most reductions

Note 29 defines one-step cost as

```text
1 + semantic_size(inputs read by the rule) + semantic_size(value produced)
```

but Beta, `Let-Value`, `Match-First`, and `Fold` generally produce expressions, not values. For example,

```text
(fn(x): nat_add(x, 1))(2) -> nat_add(2, 1)
```

has no “value produced” by that step. The definition describes sizes of values, closures, syntax values, and opaque
values, but does not define the size of this reduct as the produced value. Join entry and jump have no formal step at
all, so they have no rule tag or charge.

This invalidates Corollary 7.7's exact charge argument and the “same diagnostic and charge trace” part of Theorem 9.4.
It does not invalidate unmetered strong normalization. Repair it by defining costs on exact machine transitions, with a
structural size for every expression/configuration component the transition reads and writes. Then prove that the
limited machine is a prefix of the unmetered machine.

### Medium: the construction-time `ValidMusic` contract is false

Note 30 states that well-typed `Music` arguments imply that every compiler-owned operation returns `ValidMusic`. But

```text
music_use("not_bound", anchor)
```

has well-typed arguments and returns a finite `Music` value whose `Use` is free, which the same section says is not
valid. The proof in note 33 partly retreats to the correct statement: `music_use` creates a finite leaf whose binding is
checked only when an enclosing recipe is validated. The two documents therefore assert different invariants.

Separate the properties cleanly:

- every operation returns a finite recipe of the right representation;
- `ValidMusic` is a decidable whole-recipe predicate; and
- `close_music` first validates and then closes, returning a stated error otherwise.

That weaker design is sufficient for total closing. If validity by construction is desired, `Use` needs a scoped builder
capability or a typed name token, not a bare `RecipeName`.

### Medium: pass-result composition is weaker than the claimed composite result

The path-level theorem survives: in one valid registry, two paths concatenate associatively only at the exact same
presentation version and anchor. That is the governing theorem, and note 33 does not refute it.

The lift from paths to `PassResult` is not defined. Theorem 11.2 says “for every selected path pair” and then declares a
valid composite result, but never defines selection or requires coverage of the `Q` result. Select no pairs from two
nonempty pass results. Every selected pair meets at an exact anchor vacuously, yet the purported composite contains the
`C` output and no path explaining it from `A`.

If `PassResult` validity requires complete ancestry for its output anchors, this is a counterexample. If it requires
only that paths which happen to be present are locally valid, the theorem is true but too weak to establish the
constitution's provenance goal. Define the composite path set, state coverage for every retained target anchor
(including generated and combined steps), require compatible pass descriptors, retain the intermediate presentation, and
then prove the result valid. Exact anchor equality should remain unchanged.

### Medium: the “complete paper programs” audit is not literal

The ordinary data-and-function portions are useful pressure tests, and most are straightforward instances of the
candidate core. Theorem 12.1 nevertheless overstates what was checked.

- The staff and studio transformer implementations are prose algorithms, not displayed source terms. Their only complete
  displayed objects are expansion outputs. The missing quotation and anchor operations above are essential, not
  syntactic ellipses.
- Several client or package functions are neither declared nor imported in the displayed modules, including
  `paired_frequency_gesture`, `silence_gesture`, `named_drum_gesture`, and `release_all_gesture`. The module rule says
  imports are explicit and qualified. An unstated implicit prelude could repair this, but no exact prelude interface is
  given for these names.
- The claimed inferred type of `trial.tonal.analyze` is too specific. Its `before` parameter is never used, so ordinary
  Algorithm W infers a fresh unconstrained parameter:

  ```text
  forall A. Key × A × Chord × Option<ChordSymbol>
            -> Result<FunctionClaim, TonalError>
  ```

  not the displayed type with `Option<ChordSymbol>` in the second position.

The last point does not make the client ill typed; it shows that the literal inference audit was not performed. Add the
missing imported interfaces, supply actual transformer terms in the chosen transformer calculus, and either use
`before`, remove it, or add a monomorphic annotation.

## Claims that survived the attack

The failed main theorem should not obscure the parts worth retaining.

- For the ordinary adapter-free source fragment, rigid nominal heads, pre-resolved fields, complete product calls, and
  non-recursive `let` leave ordinary rank-1 Algorithm W. I found no counterexample to decidability or principality under
  the stated finite-table assumptions.
- The permitted recursive data are finite polynomial trees. Given immutable acyclic values and reducibility-preserving
  primitive operations, the generated-fold termination argument is sound in outline. The recursive calls are on proper
  constructor subvalues, and the `List`, `Option`, `Result`, and product lift is strictly positive.
- Ordered, exhaustive source matches have a deterministic first-match meaning. Pattern-matrix coverage and the
  result-preserving idea of sharing equal fallback subtrees are reasonable. The defect is the missing join machine, not
  the first-match policy.
- The direct hidden-constructor rule is sound for one build: a client cannot name a private `ConstructorId` absent from
  its visible interface. Treating an owner-exported adapter as an owner-defined eliminator or constructor is also a
  coherent abstraction policy once quotation and scoped resolution are formalized.
- The multiset-of-ranks proof is valid for expansion of an already checked adapter when every emitted adapter call is
  strictly lower ranked. It does not by itself compile adapter definitions.
- Finite `ScoreRecipe` values, whole-recipe validity checking, unequal-length overlay, and structural closing are a
  natural route to the governing finite temporal kernel. `Music` need not store a source closure. The selective map case
  needs repair, not abandonment of the recipe.
- Governing temporal normalization, unequal-length overlay, marked-reference support preservation, and exact
  path-at-anchor concatenation remain valid imported results.
- The separation between finite source/temporal normalization, finite prepared process graphs, and possibly unbounded
  audio histories is consistent with the constitution and across-stage rules.
- The research notes are honest that the candidate is unimplemented and that present compiler behavior is not evidence
  for the paper theorem.

## Recommended repair order

Do not patch the final proof first. Repair the definitions in dependency order:

1. Decide whether adapters use a separate total transformer calculus. Define its syntax construction and generative
   binding semantics, including pure deterministic scope and expansion identifiers.
2. Separate definition-time adapter dependencies from emitted-call ranks and prove both orders well founded.
3. Erase joins before evaluation, or define the complete join-machine relation and its costs.
4. Define logical charges on that exact evaluator, not on an informal mixture of expressions and values.
5. Give `ScoreRecipe` a precise denotation into terms, with a selective `MapPayload` rule tested against shared and
   unshared uses. State finiteness separately from `ValidMusic`.
6. Define total pass-result composition with target-anchor coverage.
7. Only then re-run W, safety, normalization, adapter, Music, and paper-program audits. The ordinary HM and fold proofs
   can largely be reused.

This keeps the intended language small. None of these repairs requires dependent types, effects in ordinary source,
general recursion, partial application, or a larger musical kernel.

## What I verified

- The reviewed checkout was exactly commit `3eaa13123ec140a328958a88a77e6bfdb801c03b` when the review began.
- I read notes 26 through 33 in order and checked the theorem dependencies against the displayed grammars and rules.
- I read the governing constitution, obligations, all across-stage rules, and the governing kernel purpose, grammar,
  typing, denotation, laws, normalization, term-calculus, realization, and payload-admission documents relevant to the
  imported results.
- I inspected `crates/musa-kernel/src/term.rs`. The current host payload walker traverses literals and bindings and has
  an explicit no-op `Var` case, as used in the shared-map counterexample.
- I inspected the current compiler's private `Music` representation. It is the older implementation, not the proposed
  `ScoreRecipe`; the research notes do not confuse the two.
- I ran the focused current-kernel term, mark, and payload-map tests. All 17 selected tests passed. They support the
  governing kernel facts; they do not implement or prove the candidate source language.

## What I judged

- The counterexamples above are judgments about the exact formal objects displayed in the frozen notes.
- I judged the separate transformer calculus to be the smallest natural repair because quotation and generative hygiene
  are phase operations, not ordinary pure value operations.
- I judged the five musical examples adequate as domain pressure for nominal data, hidden constructors, folds, package
  conversions, and distinct notation/performance routes. That does not establish their cultural or analytical adequacy
  as music theories.

## What I did not check

- I did not test an implementation of the candidate because none exists, by design.
- I did not prove or property-test the adapter-specific edit and print laws; the main theorem explicitly leaves future
  adapter laws open.
- I did not audit package resolution, stable cross-build identity, persistent caches, exporter fidelity, or unbounded
  audio execution. The candidate explicitly excludes those results.
- I did not run the full Rust, TypeScript, desktop, or slow test suites. They cannot decide the frozen paper theorem.

## Check record

The focused command was:

```text
cargo nextest run -p musa-kernel -E 'test(/term|marked|map_payload/)'
```

It ran 17 tests and all passed. Markdown and whitespace checks were run after writing this report.
