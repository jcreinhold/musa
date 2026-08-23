# 55. Why `Cat` stays a base type, and forgetting becomes a function anyway

**Status: governs nothing.** `../../../rules/language/11-quotation.md` §1 holds the decision; this page holds the
measurement behind its repair. Written at prompt 159, before a line of it was implemented.

## 1. What §1 said, and on what argument

Prompt 145 repaired §1 to read:

```text
enum Cat { Expr, TokenTree }

Syntax : Cat -> Type 0
```

The repair reversed an earlier refusal — "a family would pull the indexed-family machine into the core for one type" —
on the ground that prompt 156 put that machine in the core for the musical domains anyway, "so the cost of the family is
zero and its benefit is real".

The first clause is true and the second does not follow. **The cost was never the indexed-family machine.** It is D3,
and D3 is a different constraint that the repair did not weigh.

## 2. The measurement

Two mechanisms hold `Cat` where it is, and both are argued in the code by name.

**A δ-rule cannot write down a family's constructor.** `Rule = fn(&[Datum]) -> Option<Answer>` is a bare function
pointer, deliberately: `base.rs` states that "a function pointer cannot capture host state, so *the result is a function
of the argument values alone* is a property of the type rather than a promise a reader has to audit". Fourteen syntax
rules in `registry/rules.rs` answer

```rust
Datum::Lit(literal(syntax_type(cat), node))
```

and a `Literal` carries its own type as a `Term`. With `Cat` a family, `Cat.Expr` is a `family::Constant`, obtainable
only from an `Arc<Group>` that lives inside a `Cx`, which a `fn` pointer has no way to reach.

`Datum::Case` and `Datum::Count` already dodge exactly this, and how they dodge it is the shape of the escape: they name
their constructor or family by *string*, and `family::realize` supplies the type from the position the answer stands at.
`Datum::Lit` is the one arm that is not realized against its expected type. Freeing the rules would therefore mean a new
`Datum` arm — a payload with no type, typed from the signature — which is additive and coherent and is a change to the
core.

**A base type's kind is built with no context at all.** `registry::registered` is a `LazyLock<HashMap<Box<str>, Base>>`,
a process-global keyed by name, and `indexed("Syntax", "Cat")` writes the kind `Cat -> Type 0` out of it. For that kind
to mention a declared family, the whole static table would have to be re-plumbed through a `Cx`. The global is not an
accident either: `registry.rs` records that a base type compares **by name** precisely so that one host may build its
registry twice and two closed values stay convertible across the two.

So the family costs a core change and a registry rewrite, before the first line of §1's own content is written.

## 3. What the family was going to buy, checked one claim at a time

| §1's claim | Measured |
| --- | --- |
| `as_expression` becomes an ordinary function returning `Option (Syntax Expr)` | **Already true.** It is row `SyntaxOp::AsExpression` in `SYNTAX_OWNERSHIP`, at that exact type, and has been since prompt 142. |
| Forgetting becomes a written function rather than an acceptance rule | **True and independent.** `forget_category : Syntax Expr -> Syntax TokenTree` already exists as a δ-builtin with that signature; what makes it uncallable is that it is in neither ownership table, which is one row. |
| A `match` on a `Syntax c` value refines `c` | **Not implementable, and §1 says so itself** four paragraphs earlier: a syntax value is "the lossless token tree prompt 127da declared — `Missing`, `Token`, `Identifier`, `Group` over a `SourceInfo` **with no eliminator**". There is no `match` on a syntax value to refine anything, so the branch §1 describes cannot be written. |

Two of the three are had without the family and the third is had by nobody. That is the whole argument for the repair:
the family's price is a core change and a registry rewrite, and its measured benefit is zero.

## 4. What replaces it

`Cat` stays a base type with literal values, beside `Coordinate`, for the reason both were put there. And the *point* of
§1's repair survives intact, because the point was never the family — it was that **musa has no subtyping**. The
acceptance rule goes, and forgetting is written:

```text
forget : Syntax Expr -> Syntax TokenTree
```

Monomorphic rather than `(c : Cat) -> Syntax c -> Syntax TokenTree`, and with two categories the two are the same
function: forgetting at `TokenTree` is the identity, so the dependent signature quantifies over one case that does
nothing and one case this signature already covers. §1's own sentence about extending — "a third category is a case in
`rules::forgets` and a second registration" — becomes "a third category is a second `forget`", which is the same
bookkeeping in the same place.

What the language gains is the sentence prompt 144's specification states and prompt 159 earns: **no rule accepts a
program that conversion would reject.** That was the whole of what the acceptance rule cost, and it is paid in full
without the family.

## 5. What would re-open it

A reason to match on a syntax value. If an adapter ever needs to take a tree apart and learn its category in the branch
— rather than assert it, which `as_expression` already answers — then `Syntax` needs an eliminator, and a type with an
eliminator over an index wants that index to be a family. That is the order the work has to happen in: the eliminator
first, because it is the thing that makes the index worth refining, and the two mechanisms in §2 after it, because they
are its price. Re-opening on the strength of the family alone would buy the third row of §3's table, which is empty.
