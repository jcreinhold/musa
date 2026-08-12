# Repair after the first proof review

**Purpose:** record the one permitted repair without hiding the failed proof beside it.

The independent review in `10-proof-review.md` judged the frozen draft at commit `967f632` incorrect. This repair edits
the research specification and proof. The review remains unchanged.

## 1. The stuck annotation

The term:

```text
((let x = true; x) : Bool)
```

was well typed but could not step. The evaluation contexts now include `(E:A)`. Preservation, progress, unique
decomposition, and reducibility now treat an annotation as evaluating its subject before erasing the value annotation.

## 2. The real old language

The old-fragment theorem had silently dropped `map`, `filter`, `range`, `repeat`, `map_note_pitches`, and checked kernel
quotation. `04a-formal-rules.md` now gives rules for all of them and an exhaustive table covering every current checked
expression form, all seven structural operations, all eight controlled music operations, and quotation.

The theorem no longer says one old step is the same one new step. Old simultaneous function application may take several
curried target steps. The repaired theorem proves a finite forward simulation and relates returned function values by
how they act on arguments.

The current musical base types remain as migration bridge types until the separate theory-package prompt moves their
owners. That temporary inclusion is what makes a literal core embedding possible. Written pitch and interval use
mathematical integer coordinates in the compatibility meaning, so movement is total; every successful current bounded
movement keeps the same result.

## 3. Higher-order operations

`map` and `filter` now have private evaluator states. They call the source function once per list member, in source
order, and shorten a finite remaining list at each step. `range` and `repeat` construct exact finite lists after
preflight.

`map_note_pitches` no longer pretends to call its function during source evaluation. It constructs one finite `Music`
recipe node. The recipe adapter later applies the total pitch function once per documented pitch-bearing fact in a
finite fragment. This separates source termination from score instantiation and fixes the missing small-step case.

## 4. Sealing

The strong claim that a client cannot match an abstract value was false. A wildcard match reveals nothing and is useful.
The repaired theorem says what abstraction needs: a client cannot name a private constructor, construct a value with
one, or inspect a value with a constructor pattern. Wildcards and whole-value binders remain legal.

## 5. `Music` closure

The old client theorem assumed that a successful `close` already returned a closed typed term. The repair defines the
finite recipe graph and proves three separate facts:

1. source construction preserves the recipe invariant;
2. instantiation returns an error or a finite well-formed fragment; and
3. closing returns an error or a closed well-typed kernel term.

The proof still relies on a checked finite table for atomic score requests and transforms. It no longer assumes the
whole conclusion as one adapter premise.

## 6. Stage composition

Paths now join only at the exact same full anchor, including the stored presentation reference and local anchor id.
Semantic equality is not enough. Loss lists combine by ordered concatenation. The proof no longer cites a loss
normalization law that the governing specification does not contain.

## 7. Complete examples

The phrase example now handles `Result<Phrase,Error>` before calling `perform`. The score-and-studio trace handles
`Result<Music,Error>` before instantiation and displays its musical context, performance profile, performance context,
realization seed, and separate preparation seed. The paper-program prelude names the exact surface-to-core elaborations
for multi-argument calls, list literals, and folds.

The elaboration table also distinguishes user calls from saturated compiler calls and spells out named functions and
nullary constructors. The examples use `Text` as the notation adapter's error type, so every type they mention is
defined.

## 8. Gate

This is the only repair round allowed by the closure plan. The repaired proof must now receive a second independent
review. If that review finds a fatal, high, or medium problem, the design stays in research and the blocker is recorded.
