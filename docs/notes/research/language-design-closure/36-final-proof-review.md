# Final proof review of the repaired language closure

## Verdict

**Incorrect.** The repair closes several findings from review 34, and the ordinary source calculus remains a credible
small total language. Four High-severity gaps nevertheless remain in the exact repaired definitions. Any one blocks
promotion.

This is the final bounded review of commit `1a32664aff1973b1c47019d98214c6dafe839f77`. Missing implementation is not a
proof defect here; the findings concern the paper rules themselves.

## Findings

### High: a transformer cannot construct its required structural paths

`NodePath` and `BindingPath` are abstract nominal types. The transformer language exposes operations that *consume*
them, but no constant or operation constructs a root path, obtains the path of the current input node, or extends a path
with a fixed role. `fold_syntax` supplies `SourceInfo` and folded children, not an input-tree path. Consequently there
is no displayed transformer term that can call even

```text
generated_token(ctx, node_path, kind, text)
```

for an arbitrary input, because it cannot produce `node_path`. Prose saying that a transformer “derives” paths from
input-tree paths does not introduce a term or typing rule. The staff and studio algorithms therefore remain desired
expansions rather than inhabitants of the stated transformer calculus.

This also leaves Theorems 9.4, 9.6, and 9.7 without their required arbitrary-finite-input premise. The deterministic
identity idea is sound once paths exist; the displayed calculus does not make them exist.

The smallest repair would expose authenticated pure operations such as `root_path`, `input_path`, and
`child_path(path, role)`, or make the syntax fold explicitly path-indexed. Their laws must prove that distinct output
nodes and binders receive distinct coordinates while repeated binder uses receive the same coordinate.

### High: match lowering has no defined executable target

The repair correctly makes `join` and `jump` temporary and erases them before evaluation. That closes the old stuck join
counterexample. It does not define the rest of the lowered match language.

The decision tree contains `Leaf`, `Bind`, and `Switch`, and note 29 says source patterns are gone from the evaluation
core. No grammar, typing rules, evaluation contexts, or reductions are given for those three forms. Conversely, the only
formal evaluator still contains source `match` and `Match-First`. Thus Theorem 6.4 cannot transport source safety to the
claimed lowered executable program, and Theorem 9.8 cannot invoke preservation, progress, determinism, or the charge
theorem for that program.

For example, lowering an exhaustive `match true with true -> 0 | false -> 1` is said to produce a `Switch`, but no
displayed reduction takes that closed typed `Switch` to `0`. Treating the tree only as a compiler witness would be
coherent, but then an exact translation from the witness to the already defined ordinary core and a result-preserving
simulation are still required.

The natural repair is either to evaluate the proved source-match calculus directly, or to define one complete lowered
target calculus and prove typing, unique decomposition, result preservation, and charge preservation for the lowering.

### High: `music_map` erases the generation sites of mapped shared uses

The repaired `music_map` removes `Share` and `Use` by unfolding them before temporal closing. But a `FactOrigin` stores
only the original anchor and map steps. It has no reuse step. Consider the valid finite recipe

```text
Share(
  x,
  Fact(span, payload, origin),
  Overlay(Use(x, site_1), Use(x, site_2)),
)
```

Applying `music_map(f, recipe, map_site)` returns two facts whose recorded origins are both just `origin` followed by
`(f, map_site)`. Because the result is share-free, `close_music` never sees either `Use` and cannot recover `site_1` or
`site_2`. The two generated occurrences have become provenance-indistinguishable.

This contradicts note 30's own boundary contract that generated reuse records both root and use site, as well as the
governing origin obligation. Lemma 10.3 proves finiteness and payload admission, but not preservation of the required
origin. Theorem 10.5 therefore establishes a typed term, not the promised correctly attributed one.

The repair should make `FactOrigin` a finite typed derivation trace containing reuse steps as well as map steps. When a
`Use` is unfolded, every copied fact must acquire that use's root and generation site before the map step is appended.
Alternatively the translation may retain a marked reference with an exact composition of reuse and map provenance.

### High: linear origin paths cannot compose a multi-source step

The coverage repair removes the old empty-selection loophole, but it does not make multi-source composition a
finite-list operation. Let `P` contain paths

```text
a_1 -> b_1
a_2 -> b_2
```

and let a path of `Q` begin with

```text
Combined([b_1, b_2], c, evidence).
```

An ordinary origin path has one current endpoint. There is no list concatenation that prepends both `P` paths to this
one `Q` path. Producing two linear paths attaches only one ancestry to each copy; neither path contains the complete
two-source derivation asserted by `Combined`. The same issue applies to a `Generated` step when its root and generation
site have independent incoming histories.

Therefore Theorem 11.1's category of unary paths does not prove Theorem 11.2 for the stated `Combined` and `Generated`
steps. Target-anchor coverage alone does not repair missing input ancestry.

The natural object is a finite acyclic derivation graph or proof tree. Composition grafts the appropriate `P` derivation
at every source leaf of a `Q` step. A second acceptable design would decompose multi-source steps into explicit
predecessor edges tied by one derivation id. Either design needs its own associativity and coverage proof; ordinary list
concatenation is insufficient.

## Repairs that survived the attack

- Adapter definitions are now bootstrap expressions with no adapter regions. The emitted-call rank graph is no longer
  confused with definition-time compilation, and its multiset termination argument is sound for already executable
  transformers.
- Structural expansion identities replace the contradictory `fresh_name: Text -> Identifier` contract. The remaining
  problem is the absent path-construction interface, not determinism of identity from a supplied context and path.
- Erasing acyclic joins to non-recursive functions is a coherent repair of the earlier stuck-join term. The separate
  decision-tree target is what remains undefined.
- The expression charge `1 + semantic_size(redex) + semantic_size(reduct)` is defined for expression-producing steps,
  and the limited evaluator is a deterministic prefix of the mathematical evaluator for the displayed executable
  relation.
- `FiniteMusic` is now correctly separated from whole-recipe `ValidMusic`. A bare free `Use` may be finite and invalid,
  and `close_music` can reject it without contradicting construction.
- Mapping is now performed before temporal closing and terminates on a valid finite acyclic reference graph. It no
  longer appeals to a nonexistent kernel term map. The defect is loss of reuse provenance during unfolding.
- The ordinary adapter-free rank-1 source fragment retains decidable resolution and inference, principal types under the
  stated finite-table contracts, preservation, progress, determinism, and strong normalization under the explicit total
  primitive-operation premise.
- Strictly positive finite recursive data and generated folds remain a sound way to express the program examples without
  general recursion.
- Hidden-constructor control survives for one checked build, subject to the stated authenticated-name assumptions.
- The repaired five examples use the intended ordinary data, match, fold, module, and complete-call forms after their
  displayed adapter blocks are replaced by displayed expansions. This syntactic audit does not supply the missing
  transformer terms or lowered-match semantics.

## What I verified

- The checkout was exactly commit `1a32664aff1973b1c47019d98214c6dafe839f77`, and notes 26 through 35 were unchanged
  from that commit during the review.
- I read the repaired notes 26 through 33, the first review 34, and repair record 35, and checked the dependency order
  of their stated lemmas and theorems.
- I checked the relevant governing source, temporal-kernel, marked-reference, and across-stage origin contracts.
- I inspected the current kernel's marked-reference evaluator only to confirm the imported distinction between literal
  payload rewriting and per-instantiation provenance. The candidate source and transformer languages are not current
  implementations.

## What I judged

- The four counterexamples above concern the exact displayed mathematical objects, not implementation absence.
- I judged each High because it breaks a load-bearing conclusion of the main theorem: executable adapters, lowered
  evaluation safety, provenance-preserving `Music` closure, or typed stage composition.
- I judged the ordinary source calculus worth retaining. None of the repairs requires dependent types, general
  recursion, effects in ordinary source, partial calls, or a larger temporal kernel.

## What I did not check

- I did not test an implementation of the candidate because none exists.
- I did not mechanize Algorithm W, reducibility, transformer execution, recipe closing, or provenance composition.
- I did not re-audit package resolution, persistent identities, caches, exporter fidelity, or unbounded audio execution;
  the frozen theorem excludes them.
- I did not run the full Rust or TypeScript suites. They cannot decide these paper counterexamples.

## Final recommendation

Do not promote notes 26–33 into the governing specification. Preserve the small source calculus, but treat transformers,
lowered matches, `Music` derivations, and cross-stage provenance as four unfinished formal interfaces. The next design
work should define the missing objects directly rather than add another proof layer over the current prose.
