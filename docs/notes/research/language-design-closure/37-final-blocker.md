# Why the language design was not promoted

## Decision

The bounded language-design effort stops here. The second and final proof review found four High-severity gaps. The plan
allowed one repair and no third review. Musa's governing language rules, architecture, and implementation prompts
therefore remain unchanged.

This is not a verdict against the whole source-language direction. It is a verdict against claiming that the whole
design has been defined and proved.

## The four blockers

### 1. Adapters cannot make the paths their builders require

The repaired builder API takes `NodePath` and `BindingPath`, but these types are abstract and have no public
constructors or traversal operations. A staff adapter cannot obtain the path of an input node or derive a child path.
The displayed staff and studio expansions are therefore examples of desired output, not executable transformer programs.

A future repair needs a small, pure path API or a path-aware syntax fold. It must show that every generated node has a
unique path and that every use of one generated binder receives the same binding path.

### 2. Lowered matches have no executable meaning

The repair correctly erases temporary joins. But the remaining decision-tree forms—`Leaf`, `Bind`, and `Switch`—have no
term grammar or execution rules. The proved evaluator still runs source `match`; the proposed compiler says patterns are
gone before evaluation. No theorem connects those two claims.

A future repair should choose one target. Either evaluate the already proved source match directly, or define the full
decision-tree target and prove that lowering preserves types, results, and charges.

### 3. Mapping shared music loses each reuse site

`music_map` unfolds `Share` and `Use`, then keeps only the original fact anchor and map steps. If one phrase is used at
two sites, the two mapped copies have the same origin. Their distinct generation sites are lost.

A future `FactOrigin` needs an ordered derivation trace that includes reuse as well as transformation. Unfolding a use
must append both its shared root and its generation site before appending the map step.

### 4. A list-shaped path cannot express combined ancestry

One result may combine two or more inputs. A linear path can be prefixed by one earlier path, but not by two independent
paths at once. The proposed composition theorem therefore does not cover `Combined` steps or a generated step whose root
and generation site have separate histories.

The likely repair is a finite acyclic derivation graph or proof tree. Composition would graft an earlier derivation at
each source leaf. That object needs its own coverage and associativity proof; list concatenation is not enough.

## What survived

The following choices remain strong candidates:

- a strict, pure, total source language;
- rank-1 Hindley–Milner inference with principal types;
- complete positional calls and explicit functions;
- finite nominal data, private constructors, exhaustive matches, and generated folds;
- file modules with qualified imports and build-local nominal identities;
- a fixed lexer and grouper plus restricted package-owned adapters;
- the staff spellings `c4/4`, `c4/4.`, `[c4 e4 g4]/2`, `rest/8`, and exact `c4(3/8)`;
- a finite private `Music` recipe with all theory and performance choices supplied before closing;
- exact rational temporal terms, unequal-length overlay, finite gestures, prepared process graphs, and stepwise audio;
  and
- the refusal to add dependent types, call-by-push-value, effects, partial calls, general recursion, or a larger kernel
  without a musical need.

The ordinary adapter-free source calculus survived both reviews. Its unresolved problems lie at four interfaces:
transformer paths, match lowering, music-origin preservation, and multi-source derivations.

## What was deliberately not changed

No compiler code changed. No governing rule changed. No architecture or implementation prompt changed. Package
registries, persistent compiled identities, and compiled-value caches remain outside this effort.

Those omissions follow the gate. Promoting or scheduling an unproved design would turn research uncertainty into
repository policy.

## Recommendation

If this work is reopened, start with the four missing objects, not another surface-language comparison:

1. a path-aware finite transformer calculus;
2. one executable match target;
3. a derivation trace for each `Music` fact; and
4. a finite derivation graph for cross-stage origin.

Freeze those definitions, write the four small proofs, and only then reconnect them to the otherwise sound source
calculus. The musical cases do not call for a larger type system. They call for clearer interfaces.
