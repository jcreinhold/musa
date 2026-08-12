# Repair after the first proof review

## Purpose

This note records the one permitted repair. It answers one question:

> What changed after the counterexamples in note 34?

The repair changes definitions, not just proofs. Notes 26–33 now contain the repaired rules. This note is the audit
trail.

## 1. Adapters now have a defined phase language

The first draft said adapters could quote code, insert source syntax, make fresh names, and make anchors. It never put
those acts in a grammar or gave them typing and evaluation rules.

The repair removes general quotation. An adapter receives an explicit `ExpansionContext` and uses six builder forms:

```text
generated_token
definition_name
local_name
generated_group
checked_expression
anchor_at
```

Each form has fixed input and output types. It is a pure total operation on finite values. Existing input syntax may be
placed in a generated group as an ordinary value; it keeps its source and use-site scopes.

Generated identity is structural. An expansion id is the exact path to the adapter call. Each output node has a finite
node path. A local binder has a finite binding path. The same context and paths produce the same identities. Different
binders must have different binding paths. `checked_expression` rejects duplicate node paths and duplicate binder
declarations.

This fixes both adapter failures from note 34. The language no longer relies on an undefined quote form, and it no
longer asks a pure function `Text -> Identifier` to return a different result on identical calls.

Adapter definitions contain no adapter regions. This is the bootstrap rule. The rank graph orders only adapter calls
that generated output may contain. It is no longer used for two different dependency relations.

## 2. Joins are not executable terms

The first draft typed `join` and `jump` but did not define how they step. The repair makes them temporary match-lowering
forms.

The compiler introduces acyclic joins to share repeated fallback trees, then erases them:

```text
join j(x): A = body in rest  ->  let j = fn(x): body in rest
jump j(value)                ->  j(value)
```

Only the erased ordinary core is evaluated. The progress, preservation, determinism, and termination theorems contain no
join case. Join erasure is proved first by induction on the finite join dependency order.

This choice is smaller than adding a second abstract machine. It also leaves one exact executable core for resource
accounting.

## 3. Charges apply to expressions, not imagined values

The first draft charged for “the value produced” by a step, although beta reduction and pattern matching often produce
another expression.

Unique decomposition now selects one redex `r` and one reduct `r'`. The step costs:

```text
1 + semantic_size(r) + semantic_size(r')
```

`semantic_size` is defined for every executable expression and value. The limited evaluator computes this one candidate
transition and either commits it or reports the first limit error. There is no charge for temporary joins because they
are erased before evaluation.

## 4. Payload maps finish before temporal closing

The first draft placed `MapPayload` inside `ScoreRecipe`, then appealed to a temporal-term map that the governing kernel
does not have. That also failed on a mapped use of an otherwise shared definition.

`ScoreRecipe` no longer contains `MapPayload`. `music_map` is a finite compiler operation:

1. validate its argument;
2. unfold its lexical `Share` and `Use` references;
3. apply the admitted map to each fact;
4. add the map anchor to each fact's origin; and
5. return a finite, share-free recipe.

The rewrite touches only its argument. Mapping one use cannot alter an unmapped use elsewhere. It may increase size, so
its logical charge covers the full result and its `Result` type can report a build or resource error.

Every music operation now promises only a finite recipe or a stated error. `ValidMusic` is a separate whole-recipe
check. Thus a bare free `Use` is finite but invalid, exactly as closing needs.

## 5. Pass composition covers every output anchor

The first draft composed an unspecified selection of paths. Selecting none made its theorem vacuous.

A valid pass result now covers every addressable target anchor. Composition processes every path that covers the second
pass's output. For each exact intermediate source anchor used by that path, it prepends every covering path from the
first result. A missing exact anchor or version is an error. The intermediate presentation is retained, and only exact
duplicate paths are removed.

This proves origin coverage for the composite output. It does not merely prove that any paths which happen to be kept
are locally valid.

## 6. The paper-program audit is literal at its stated boundary

The unused `before` argument was removed from the tonal analysis, so its displayed principal type is now the type that
Algorithm W infers. The ensemble and live examples now import and qualify their gesture constructors.

The staff and studio trials specify complete source blocks and complete expanded expressions. Their finite parsing
algorithms and builder interface are specified in note 27; they are not claimed to compile in the current
implementation. Theorem 12.1 starts after replacing each adapter block by its displayed expansion, which is the exact
boundary that theorem states.

## 7. What did not change

The repair did not add dependent types, effects, partial calls, general recursion, a larger temporal kernel, a package
identity calculus, or a cache proof. Rank-1 inference, finite folds, complete calls, file modules, hidden constructors,
unequal-length overlay, finite process preparation, and stepwise audio remain as before.

The repaired proof target is notes 26–33 together with this repair record. It is ready for the final permitted review.
