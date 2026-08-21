---
id: 127dcfaa
slug: list-fold-direction
status: done
depends_on: [127dcfa]
phase: 3
---

# Say Which End a List Fold Runs From

## Task

`nat_fold`, `option_fold`, and every fold generated for a `data` declaration are catamorphisms: the step case receives
what the eliminator already made of the substructure. `list_fold` is not. Its equation
`list_fold(z,s,x::xs) → list_fold(s(x,z),s,xs)` threads an accumulator, and the prose above the block calls all three
folds "deterministic left folds in source order", which is true of exactly one of them. Lists are therefore the one
inductive type in this language whose eliminator is not its eliminator, and the direction a fold runs — the one fact
that separates the two readings — is stated by neither the type nor the name.

This prompt gives `list` two eliminators whose names state their direction, `list_fold_from_start` and
`list_fold_from_end`, retires the bare `list_fold` with a diagnostic that names the replacement, and adds to §5.6 the
paragraph saying why `list` has earned both directions while other types retain one canonical eliminator until a second
primitive has evidence.

## Read

- `docs/rules/language/02-core-calculus.md` §1 (the eliminator signatures and the argument that `nat` is in the language
  to be the inductive numeric type), §5.6 (the equations, the typing rules, and the strong-normalization measure), and
  §5.8's four-family list, whose structural-eliminator row names them. This is the governing page the prompt amends, and
  it is amended in this prompt's commit, before any code changes — `docs/rules/README.md` calls that the ordinary way to
  change a page below `constitution.md` and `obligations.md`.
- `docs/rules/language/01-surface.md`, the paragraph beginning "Structural folds do not add syntax", which names the
  three folds a source author writes.
- `crates/musa-compiler/src/core/mod.rs`: the `Eliminator` enum and its doc comment ("The seven structural
  eliminators"), `Eliminator::instantiate`, `Eliminator::arity`, the `BUILTIN_OWNERSHIP` array, the `Builtin::ListFold`
  arm of `eval`, and the law asserting the eliminator list by name.
- `crates/musa-compiler/src/data.rs`, `fold_name`'s doc comment: "The shape is `nat_fold`, `list_fold`, `option_fold` —
  the eliminators the language already had — because a generated fold *is* one of those." A generated fold is one of
  those in shape but not in direction, which is the sentence this prompt makes true.
- `crates/musa-compiler/src/core_budget.rs`: `Reduction` and `CostTable`.
- `stdlib/src/adapters/staff.musa`, `from_the_end`, and its comment explaining that the sequence is assembled from its
  end because `list_fold` runs left to right. That helper is the closure chain this prompt deletes.
- Prompt [127dcec](127dcec-construction-charges.md): its **Design** measured the closure chain at 104,016 constructed
  nodes against 100,009 for a plain left fold of identical per-step work on the same region, and its **Stop** recorded
  this inconsistency as real and out of that prompt's scope.
- Peyton Jones ch. 4 §4.1.2.1: `list * ::= NIL | CONS * (list *)` — "lists are just an instance of a general structured
  type", with `[x,y,z]` translating to `CONS x (CONS y (CONS z NIL))`. §4.3.3 gives a sum-constructor pattern its
  semantics as one case per constructor over that declaration. A `list` is a `data` declaration in everything but
  spelling, so its eliminator is fixed by the same rule that fixes a user declaration's.
- Peyton Jones ch. 2 §2.1.1: "the lambda calculus allows a function to return a function as its result". The closure
  chain that turns one fold direction into the other is that and nothing more, which is why this is a legibility and
  cost decision rather than an expressiveness one. Ch. 3 §3.2's enriched calculus is the standing precedent for the
  answer — when an author needs a construct, enrich the core rather than make them encode it.
- Root `AGENTS.md`, "No sublanguage by subtraction": a convenience the core drops is paid by every author who writes in
  it, rather than once by us.

## Design

**Only `list` has earned two compiler-owned answers.** This is an admission claim, not a theorem that other finite
structures cannot have an observable traversal order.

- For the current `nat_fold`, catamorphism and upward accumulation coincide. The equation
  `nat_fold(z,s,n+1) → s(n, nat_fold(z,s,n))` expands to `s(n−1, … s(1, s(0, z)))`, and an accumulator fold that visits
  the indices from `0` upward builds the same term: the successor structure numbers itself, so its outermost constructor
  carries its largest index. The implementation already exploits this — it iterates `0..n` with an accumulator and
  satisfies the catamorphic equation. A reverse visit from `n−1` downward would be observable with `s(i,a) = i`, but no
  reviewed program needs it, so it has not earned a primitive.
- For `option`, there is no sequence to have a direction; `option_fold` consumes its sole constructor.
- A generated `data` fold is the declaration's canonical catamorphism: a case sees its recursive fields already folded,
  one constructor layer at a time. A particular declaration may encode an ordered structure and later earn another
  traversal; generation does not add one without a caller.
- For `list`, the outermost `CONS` holds the *first* element. Folding from the outside in therefore reaches the last
  element first, while accumulating from the start reaches it last. The projection `s(x,a) = x` distinguishes them;
  associativity and commutativity with a common unit are sufficient for agreement but are not an exact classification of
  every operation or input. Both directions have current consumers and neither is derivable at zero cost, so both names
  are admitted.

**The missing direction is the one the language most needs.** Prompt 127ac made right-nested `data` first class, and a
catamorphism-shaped value is what the staff package's `StaffItem` is: `Sounded(anchor, event, after)` nests to the
right, so an adapter reading a region left to right must build the chain from its end. With only the accumulator fold
available, the only route is the standard closure chain — fold to a `Pending -> Pending` and apply it — which is what
`stdlib/src/adapters/staff.musa`'s `from_the_end` does today. It is correct, it is the one place in that file a reader
stops, and prompt 127dcec measured it at roughly 4,000 constructed nodes on the smallest region the adapter accepts,
because it allocates one closure and one extra application per element. The language that generates right-nested data
should be able to read a list into it directly.

**The decision, and the two alternatives refused.**

*Refused: flip `list_fold` to the catamorphism and keep the name.* It buys the uniformity, and it pays for it by
silently changing what every existing call means. Eight `.musa` call sites and twelve more inside Rust test fixtures
would keep compiling and start answering differently wherever the step is not symmetric — `first_refusal` would become
last-refusal, `readable` would find the last covering value, `stated_numbers` would keep the last two numbers instead of
the first two. A break this repo can afford is a break it can see; a rename cannot be missed and a direction change can.
It also only moves the stumble: the accumulator idiom, which several of those sites genuinely want, would then need the
closure chain.

*Refused: leave it and explain lists as the exception.* The explanation would have to say that the exception is the
eliminator being the wrong one, which is a defect described rather than a design stated. It also leaves `from_the_end`
in the standard library as a permanent worked example of a thing the core should have done.

*Taken: two eliminators, each named for the end it starts from, and no bare `list_fold`.* `list_fold_from_start` is
today's accumulator fold with today's meaning; `list_fold_from_end` is the catamorphism. Both have the identical type —
which is the point, and the reason the name has to carry the direction:

```text
list_fold_from_start : A → (X → A → A) → list X → A
list_fold_from_end   : A → (X → A → A) → list X → A
```

The member stays the step's first argument in both, so a call migrates by changing one name and nothing else, and a
reader comparing the two compares only the word that differs. The bare `list_fold` resolves to nothing and is rejected
with a diagnostic that names both replacements and says which one preserves the old meaning; it is not an alias and not
a deprecation, per [`../clean-break-ledger.md`](../clean-break-ledger.md)'s standing rule that a replaced name is
deleted rather than kept working.

**The equations.** §5.6's block becomes:

```text
nat_fold(z,s,0)                     → z
nat_fold(z,s,n+1)                   → s(n, nat_fold(z,s,n))
list_fold_from_start(z,s,[])        → z
list_fold_from_start(z,s,x::xs)     → list_fold_from_start(s(x,z),s,xs)
list_fold_from_end(z,s,[])          → z
list_fold_from_end(z,s,x::xs)       → s(x, list_fold_from_end(z,s,xs))
option_fold(z,s,none)               → z
option_fold(z,s,some(x))            → s(x)
```

The sentence above them stops claiming a direction they do not share. It says instead that each equation is
deterministic, that `nat_fold`, `option_fold`, `list_fold_from_end`, and every generated fold are the catamorphisms of
their types, and that `list_fold_from_start` is the accumulator fold `list` also needs; and it states the coincidence
above, so a reader who asks why `nat` has one name and `list` has two finds the answer where the question occurs.

**Both are primitive, and the calculus says why rather than pretending they are independent.** §5.6 gains a remark
recording that either derives from the other by a closure chain —

```text
list_fold_from_end(z,s,xs) ≡ list_fold_from_start(λa.a, λ(x,g).λa.g(s(x,a)), xs)(z)
```

— so nothing here extends what the language can express. What it changes is what the language can say plainly and what
the meter charges: the derivation costs one closure and one application per element, which prompt 127dcec measured, and
it puts a higher-order term at a call site whose subject is a list of note-heads. Ousterhout ch. 8's rule decides it:
pay once here rather than at every author.

**The metatheory delta is one measure case, not a new argument.** The `ListFold` typing rule is duplicated under both
names with the same premises. The reducibility candidate for `list` is unchanged. In the lexicographic measure
`(constructor count, reduction height of arguments)`, `list_fold_from_end` decreases list length by one exactly as
`list_fold_from_start` does; its step is applied to a reducible member and to the reducible result of the fold over the
shorter list, so the candidate closure argument is the one already written. Preservation, progress, determinism, and
strong normalization extend by that case. The implementation iterates the list in reverse rather than building the
recursive term, exactly as the other folds iterate rather than build.

**No cost-table version bump.** `CostTable`'s weights are per metric — reduction, node, byte, instance, occurrence — not
per reduction kind, and this prompt adds no weight and changes none. It adds two `Reduction` names so a rejection prints
the fold the project actually wrote. A program that names `list_fold_from_end` is rejected by a `V2` compiler at
resolution, because the builtin is not there to resolve, so there is no version at which two compilers disagree about
its cost.

**Migration.** Every existing call keeps its meaning under `list_fold_from_start`; exactly one call becomes the new
eliminator and deletes its wrapper.

| Site | Becomes |
| --- | --- |
| `stdlib/src/transformational.musa` `chain` | `list_fold_from_start` — a chain applies its steps in the order written |
| `stdlib/src/notation/staff.musa` `readable` | `list_fold_from_start` — the first covering value wins |
| `stdlib/src/adapters/doubled.musa` group case | `list_fold_from_start` — the first `;` in the region is the refusal |
| `stdlib/src/adapters/staff.musa` `first_voice`, `stated_numbers`, `stated_exact` | `list_fold_from_start` |
| `stdlib/src/adapters/staff.musa` `from_the_end` | deleted; its one caller folds with `list_fold_from_end` over `taken_piece` directly, and the comment explaining the closure chain goes with it |
| `tests/fixtures/core-pressure.musa`, the fixture sources in `crates/musa-compiler/tests/suite/` | `list_fold_from_start` |

## Target

- `docs/rules/language/02-core-calculus.md` — §1's signature block gains the second list eliminator and loses the bare
  one; the sentence at §1 explaining `nat`'s inductive role names the folds correctly; §5.6's term grammar, `ListFold`
  typing rule, equation block, prose above it, derivation remark, and normalization measure as **Design** states them;
  §5.8's structural-eliminator row becomes eight names.
- `docs/rules/language/01-surface.md` — the "Structural folds do not add syntax" paragraph names four folds and says the
  direction is in the name.
- `crates/musa-compiler/src/core/mod.rs` — `Builtin` and `Eliminator` gain `ListFoldFromStart` and `ListFoldFromEnd` and
  lose `ListFold`; `arity` and `instantiate` cover both with the one type; `BUILTIN_OWNERSHIP` carries both entries with
  their hidden information; `eval` iterates forward for one and in reverse for the other; the "seven structural
  eliminators" doc comment and the law that asserts the eliminator names by hand say eight.
- `crates/musa-compiler/src/core/mod.rs` resolution — the bare `list_fold` is an unresolved name carrying an applicable
  fix that names `list_fold_from_start` as the one preserving the old meaning, in the shape prompt 109 used for
  `use std::…`.
- `crates/musa-compiler/src/data.rs` — `fold_name`'s doc comment says a generated fold is a catamorphism and names the
  three it shares that shape with.
- `crates/musa-compiler/src/core_budget.rs` — `Reduction::ListFoldFromStart` and `Reduction::ListFoldFromEnd` with their
  printed spellings; `CostTable::V2` unchanged, and its doc comment states that a new reduction *kind* is not a table
  change.
- `stdlib/`, `tests/fixtures/core-pressure.musa`, and the fixture sources under `crates/musa-compiler/tests/suite/` —
  the migration table above.
- Tests in `crates/musa-compiler`: the two folds agree for addition and disagree for the projection `s(x,a) = x`;
  `list_fold_from_end` over a right-nested `data` declaration builds the same value as the closure chain it replaces,
  and costs strictly fewer nodes; the bare `list_fold` is rejected with the fix naming `list_fold_from_start`; both
  folds are total on the empty list; and the existing generated-fold law compares the declaration's catamorphism with
  the two list directions through terms evaluated by the one production evaluator. There is no separate substitution
  evaluator to extend.
- `docs/notes/research/core-calculus/20-the-direction-a-list-fold-runs.md` and its entry in that directory's `README.md`
  — the decision, the coincidence that makes `nat` need one name, and the two refused alternatives above, so the
  argument stays visible after the equations stop showing it.

The plan bookkeeping is already done and is not this prompt's work: `docs/plan/prompts/README.md` carries the
sequence-overview row and names this prompt in its 127d paragraph, and `127dcfb-staff-edit-print.md` depends on it
transitively through [127dcfag](127dcfag-staff-retrial.md), so the printer is written against the eliminator the
expansion ends up using.

## Check

```sh
cargo nextest run -p musa-compiler -p musa-project -p musa-lsp
cargo clippy --all-targets -p musa-compiler -p musa-project -p musa-lsp -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
```

Commit as `Say which end a list fold runs from`.

## Stop

- No change to `nat_fold` or `option_fold`, and no second name for either. `Option` has no ordered member sequence, and
  no reviewed program needs a reverse natural-number traversal. A later prompt may reopen either only with evidence;
  this prompt does not turn the absence of current evidence into an impossibility theorem.
- No change to generated `data` folds. They are already catamorphisms; only the doc comment claiming kinship with
  `list_fold` moves.
- No `map`, `filter`, `range`, or `repeat` change. Direction is unobservable in all four.
- No cons expression, no list reversal, no indexing. That a list can only be built by a literal, `range`, `repeat`,
  `map`, and `filter` is a separate question about the language's list vocabulary, and adding a constructor to make a
  fold expressible would be answering this one twice.
- No alias, no deprecation period, and no compatibility shim for `list_fold`. The name is deleted and the diagnostic is
  the migration.
- No cost-table version bump and no budget-limit change.
- No adapter behavior change. `from_the_end` is deleted and its caller folds directly; the staff expansion produces the
  same items, and its fixtures are expected to be byte-identical.
