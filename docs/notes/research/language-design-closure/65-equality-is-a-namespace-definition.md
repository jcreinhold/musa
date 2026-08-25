# Equality is a namespace definition

Prompt 166a asked whether the finite types introduced by the rewritten standard library should receive equality by
generation, by a written definition, or not at all. The question had first been phrased in terms of `Eq` instances;
prompts 143 and 146 removed that mechanism. In the language that exists, `left == right` is method syntax for a
definition named `equal` in the receiver's namespace.

## The count

Taken on the tree after prompt 166 with:

```sh
rg -n '^\s*(data|record)\b' stdlib/src --glob '*.musa'
rg -n '^\s*(private )?fn (same|equal)[A-Za-z_]*\b' stdlib/src --glob '*.musa'
git grep -En '\b(same_place|same_operation|same_class|same_set|same_row)\b' HEAD -- stdlib examples crates
```

The library declares **54** data or record types. Five public predicates compared declared values, but only two had
runtime callers:

| Type | Representation depth | Predicate callers | What equality observes |
| --- | --- | ---: | --- |
| `Cyclic(n)` | recursive spine, at most `n` constructors | 0 | `number_of`; removed |
| `Ti` | one constructor carrying one `Nat` | 0 | constructor tag and field; removed |
| `Pc(n)` | one wrapper over `Cyclic(n)` | 0 | `class_number`; removed |
| `PcSet(n)` | one private wrapper over recursive `List<Nat>` | 14 | ascending, duplicate-free members |
| `ToneRow(n)` | one private wrapper over recursive `List<Nat>` | 3 | the checked row order |

The 17 calls are not five independent demands for structural equality. They are two domain comparisons repeatedly passed
to `orbit` and `stabilizer` or asserted in the post-tonal fixtures. The other three predicates had a generated reference
entry and explanatory comments but no program called them.

## Decision: written

`PcSet` and `ToneRow` each receive one written namespace definition:

```musa
impl PcSet {
    fn equal<{n : Nat}>(left: PcSet(n), right: PcSet(n)) -> Bool { ... }
}
```

The inferred value parameter is important. Indices are retained ordinary arguments in the current calculus, not the
erased stratum prompt 143 removed; the operands determine `n`, so `left == right` remains a complete two-operand call.

Generation would serve 2 of 54 declarations, **3.7%**, and neither comparison is best described as a blind constructor
walk. Both types have private constructors and canonical representations established by their smart constructors. Their
equality deliberately reads those invariants. The written route retains four small bodies — the two namespace
definitions and the two private list folds — and removes the three unused predicates. A generator would remove those
four bodies only by acquiring rules for indexed families, private constructors, and recursive `List`, then running for
52 types with no caller.

This agrees with the useful part of Idris 2's `Decidable.Equality`: structural decisions are ordinary written programs
when the corpus is small. The Haskell 2010 Report §11.1 gives a coherent structural derivation rule, but the count does
not justify importing its mechanism here.

There is no fallback. `Pc(n)` remains finite and has no `equal`; `pc(...) == pc(...)` is therefore the ordinary
`no-method-for-type` diagnostic. That refusal is the check that “written” has not quietly become “generated when
missing.” The compiler-owned seven namespace definitions are unchanged.
