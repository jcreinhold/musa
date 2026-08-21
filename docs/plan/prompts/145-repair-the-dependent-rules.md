---
id: 145
slug: repair-the-dependent-rules
status: pending
depends_on: [144]
phase: 3
---

# Repair Every Rule That Read the Old Core, and Retire the Trait Specification

## Task

Three governing documents were written against the core prompt 144 replaced, and one of them describes a subsystem that
is about to stop existing. Repair `11-quotation.md` and `01-surface.md`; **retire `10-traits.md` outright**, with no
successor document, because after prompt 146 a structure is an ordinary record and needs no specification of its own.
**It changes no code.**

## Read

- `docs/rules/language/11-quotation.md` §1 — the paragraph that refuses to make `Syntax` a family, and its reason.
- `docs/rules/language/01-surface.md` — every section that names `record`, `enum`, `signature`, `structure`, or an
  implicit binder.
- `docs/rules/language/10-traits.md` in full, one last time.
- `stdlib/src/algebra.musa` — the three taxes the trait system charges, stated in that file's own comments.

## Design

**`11-quotation.md` §1's reason expires.** It reads: "A family would make `Cat` an index a `match` could unify, and
would pull the indexed-family machine into the core for one type." After prompt 155 the machine is in the core for the
music domains anyway, so the cost is zero and the benefit is real: matching a syntax value refines its category, and
`as_expression` becomes an ordinary function rather than a compiler builtin.

**The forgetting rule loses its special status.** §1's "a position of category `TokenTree` accepts a value of any
category" is subtyping, and 143 refused it. It becomes a total function `forget : Syntax c -> Syntax TokenTree`, written
at the splice site. Measured cost of writing it: four internal call sites in `elab/check.rs`. The specification says the
word is required and why — an acceptance rule conversion cannot see is the thing the whole amendment removed.

**`01-surface.md` gains three things and loses one.** Gains: index syntax in `data`, writable implicit binders, and the
statement that `record` and `enum` are sugar. Loses: `signature`/`structure`/`template structure`/`make`, which prompt
161 deletes — marked *deprecated, owned by 161* here rather than removed, so the corpus still validates against this
document until that prompt runs.

**`10-traits.md` is retired, not succeeded.** The counter-argument to answer is that retiring it loses the coherence
statement. It does not: coherence was a property *of the trait system*, and with no instances there is nothing for two
of them to disagree about. What the document held that is worth keeping is one sentence — that a structure's carrier
comes first — and that becomes a style-guide line about record field order, not a specification.

**Record η loses its only client.** `10-traits.md` stated coherence as "unique up to conversion… decided by η at
`quote`". That sentence is the sole reason record η is required by anything. Note its removal explicitly, because prompt
156 depends on it and a reader who does not know this will think 156 is unsound.

## Target

- `docs/rules/language/11-quotation.md`: §1 rewritten around `Syntax : Cat -> Type`; the forgetting rule restated as a
  function; the refusal paragraph removed with a pointer to 143.
- `docs/rules/language/01-surface.md`: indices, implicit binders, `record`/`enum` as sugar, module layer marked
  deprecated-and-owned-by-161.
- `docs/rules/language/10-traits.md`: replaced by a short banner naming 143 and 146, kept so the links resolve.
- `docs/rules/language/README.md` and `docs/README.md`: the precedence ladder and the document list, minus traits.
- `docs/rules/style-guide.md`: the carrier-first line, as a naming rule.

## Check

```sh
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
! grep -rn '10-traits' docs/rules --include=*.md | grep -v '10-traits.md:'
git diff --stat -- crates/ stdlib/ examples/    # must be empty
```

Commit as `Repair the rules that read the old core and retire the trait specification`.

## Stop

- No code, and in particular no deletion of `class.rs` or `dictionary.rs`. That is 146.
- No deletion of the module-layer *sections* — deprecating them is the whole of this prompt's job there.
