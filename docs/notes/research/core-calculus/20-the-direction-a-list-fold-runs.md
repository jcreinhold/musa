# The direction a list fold runs

**Status: research. These notes do not set Musa's rules.**

Prompt 127dcfaa split `list_fold` into `list_fold_from_start` and `list_fold_from_end` and deleted the bare name. This
note records why that primitive convenience is earned, why `nat` and `option` do not currently get the same treatment,
and what was refused on the way.

The short version: **Musa has demonstrated uses for both directional folds over `List`, and their types are identical,
so their names say which direction they use.** That is an admission argument about the current language, not a theorem
that no other finite structure can have an observable traversal order.

## 1. What was wrong

`docs/rules/language/02-core-calculus.md` §5.6 offered three folds and one sentence describing all three as
deterministic left folds in source order. That sentence was true of exactly one of them.

`nat_fold` and `option_fold` are catamorphisms: the step case receives what the eliminator already made of the
substructure. So is every fold generated for a `data` declaration — a case sees its recursive fields already folded,
which is what "replaces one constructor layer" means. `list_fold` was not. Its equation

```text
list_fold(z,s,x::xs) → list_fold(s(x,z),s,xs)
```

threads an accumulator. `list` was therefore the one inductive type in the language whose eliminator was not its
eliminator, and the single fact that separates the two readings — which end the traversal starts from — was stated by
neither the type nor the name. Both folds have the *identical* type, which is exactly why the name has to carry it.

## 2. Why `list` has two names today

The earlier version of this note said that direction was observable only when constructor nesting and element order
disagreed, and therefore only for `list`. That was too strong. It confused the canonical catamorphism of a type with all
the additional traversals that could be defined over values of that type.

**For the current `nat_fold`, catamorphism and upward accumulation coincide.** The equation
`nat_fold(z,s,n+1) → s(n, nat_fold(z,s,n))` expands to `s(n−1, … s(1, s(0, z)))`, and an accumulator fold visiting the
indices from `0` upward builds the same term. The successor structure numbers itself, so its outermost constructor
carries its *largest* index — the opposite of a list, whose outermost `CONS` carries its *first* element. The
implementation exploits that coincidence by iterating `0..n`. But a fold visiting `n−1` downward is still observable:
with `s(i,a) = i`, the current fold answers `n−1` and the reverse visit answers `0`. Musa has one `nat_fold` because no
reviewed program needs the second traversal, not because only one answer exists.

**For `option` there is no sequence to have a direction.** `option_fold` consumes its sole constructor.

**A generated `data` fold is the declaration's canonical catamorphism.** A case sees its recursive fields already
folded, one constructor layer at a time. A particular declaration may still describe an ordered structure and later earn
another traversal; the generator does not manufacture one without a caller.

**For `list` the two disagree.** Folding from the outside in reaches the last element first; accumulating from the start
reaches it last. The projection `s(x,a) = x` makes the disagreement immediate. Associativity and commutativity with a
common unit are sufficient conditions for agreement, not a necessary-and-sufficient classification of every operation
and input. Both readings are wanted: `chain` applies its steps in the order written, `readable` takes the first covering
value, and a staff adapter building `Sounded(anchor, event, after)` needs the other end.

A reader who asks "why does `nat` have one name and `list` two?" now finds the evidence answer in §5.6: `list` has two
current consumers, while a second natural-number traversal has none.

## 3. The missing direction is the one the language most needs

Prompt 127ac made right-nested `data` first class, and a catamorphism-shaped value is what the staff package's
`StaffItem` is. An adapter reading a region left to right must therefore build its chain from the end. With only the
accumulator fold reachable, the only route was the standard closure chain — fold to a `Pending -> Pending` and apply it
— which is what `stdlib/src/adapters/staff.musa`'s `from_the_end` did:

```text
list_fold(
    fn (later: Pending) -> Pending { later },
    fn (one_piece: Read, sofar: Pending -> Pending) -> Pending -> Pending {
        fn (later: Pending) -> Pending { sofar(taken_piece(region, one_piece, later)) }
    },
    pieces,
)(start)
```

It is correct, it is the one place in that file a reader stops, and prompt 127dcec measured it at 104,016 constructed
nodes against 100,009 for a plain left fold of identical per-step work on the same region — one closure and one extra
application per element. A language that generates right-nested data should be able to read a list into it directly.

## 4. Refused: flip `list_fold` to the catamorphism and keep the name

This buys the uniformity, and pays for it by silently changing what every existing call means. Eight `.musa` call sites
and twelve more inside Rust fixtures would keep compiling and start answering differently wherever the step is not
symmetric: `first_refusal` would become last-refusal, `readable` would find the last covering value, `stated_numbers`
would keep the last two numbers instead of the first two.

A break this repository can afford is a break it can *see*. A rename cannot be missed; a direction change can. The flip
also only moves the stumble — the accumulator idiom, which several of those sites genuinely want, would then need the
closure chain.

## 5. Refused: leave it and explain lists as the exception

The explanation would have to say that the exception is the eliminator being the wrong one, which is a defect described
rather than a design stated. It also leaves `from_the_end` in the standard library as a permanent worked example of
something the core should have done — and root `AGENTS.md`'s rule against a sublanguage by subtraction cuts the same
way: a convenience the core drops is paid by every author who writes in it rather than once by us.

## 6. What was taken

Two eliminators, each named for the end it starts from, with the identical type:

```text
list_fold_from_start : A → (X → A → A) → list X → A
list_fold_from_end   : A → (X → A → A) → list X → A
```

The member stays the step's first argument in both, so a call migrates by changing one name and nothing else, and a
reader comparing the two compares only the word that differs. The bare `list_fold` resolves to nothing and is rejected
with a diagnostic naming both replacements and saying which preserves the old meaning — not an alias and not a
deprecation, per `docs/plan/clean-break-ledger.md`'s standing rule that a replaced name is deleted rather than kept
working.

**Both are primitive, and §5.6 says why rather than pretending they are independent.** Either derives from the other by
a closure chain:

```text
list_fold_from_end(z,s,xs) ≡ list_fold_from_start(λa.a, λ(x,g).λa.g(s(x,a)), xs)(z)
```

Nothing here extends what the language can express. What it changes is what the language can say plainly and what the
meter charges. Ousterhout ch. 8 decides it: pay once here rather than at every author.

**The metatheory delta is one measure case, not a new argument.** The `ListFold` typing rule is duplicated under both
names with the same premises; the reducibility candidate for `list` is unchanged. In the lexicographic measure
`(constructor count, reduction height of arguments)`, `list_fold_from_end` decreases list length by one exactly as
`list_fold_from_start` does, and its step is applied to a reducible member and to the reducible result of the fold over
the shorter list. Preservation, progress, determinism, and strong normalization extend by that case.

## 7. What was built

- **`crates/musa-compiler/src/phase/mod.rs`** — `Builtin` and `Eliminator` carry `ListFoldFromStart` and
  `ListFoldFromEnd` and no `ListFold`; `eval` iterates forward for one and in reverse for the other, building neither
  the recursive term nor a closure per element. The law asserting the structural eliminators by hand says eight.
- **The retirement diagnostic** — `list_fold` is an unresolved name carrying an applicable fix that names
  `list_fold_from_start`, in the shape prompt 109 used for `use std::…`.
- **Five tests** — the two folds agree on a step that is associative and commutative and disagree on one that keeps the
  member it saw last; both are total on the empty list; `list_fold_from_end` over a right-nested `data` declaration
  builds the same value as the closure chain it replaces and charges strictly fewer nodes; the bare name is rejected
  with its fix; and a generated `data` fold answers what `list_fold_from_end` answers over the same members and step,
  while `list_fold_from_start` does not.
- **`stdlib/src/adapters/staff.musa`** — `from_the_end` is deleted and `read_body` folds with `list_fold_from_end` over
  `taken_piece` directly. The staff expansion produces the same items and its fixtures are byte-identical.

## 8. What would reverse this

A third reading of `list` would. Nothing in the current vocabulary offers one: a list can be built only by a literal,
`range`, `repeat`, `map`, and `filter`, and none of those is direction-sensitive. If the language later gains a cons
expression, a reversal, or indexing, the question of what `list`'s vocabulary should be is worth asking again as its own
question — but adding a constructor in order to make a fold expressible would have been answering this one twice, which
is why prompt 127dcfaa stopped short of all three.

A demonstrated use for a reverse natural-number traversal could earn a second name later. It would not reverse the list
decision; it would show only that the earlier uniqueness claim was wrong, which this revision already records. `Option`
still has no ordered sequence of members, though a future convenience must meet the same evidence rule rather than be
ruled out by analogy.
