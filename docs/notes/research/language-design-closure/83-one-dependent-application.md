# One dependent application

Prompt 176c repairs a surface distinction the core never had. Musa parsed `F<A>` as type-parameter application and
`F(a)` as value or index application, then lowered both to the same dependent application spine. That punctuation
classified an argument before the reached Π had checked it, duplicated CST and lowering paths, and encouraged a second
delimiter convention for parameters in a language whose types are already terms.

The replacement is one application, `F(a, b)`. The Π domain decides whether an argument is a type, a natural, or any
other term. Inference remains a real call-site distinction and is written on the binder—`{A: Type}`—because it changes
whether the caller supplies the argument. It is not a second application operation.

Inductive-family parameters and indices remain distinct for a different reason. `data Vec(A: Type): (n: Nat) -> Type`
declares `A` uniform across the declaration and `n` constructor-selected. Each indexed constructor writes its complete
result, such as `Nil: Vec(A, 0)`. Uniformity is never inferred from constructor bodies, so changing one constructor
cannot silently change the eliminator. This follows Agda's semantic distinction between declaration parameters and
family indices without copying Agda's whitespace surface syntax.

The change is a clean break in the unreleased language pass. Standard-library source, examples, fixtures, generated
documentation, pending prompts, and the in-repository tree-sitter grammar migrate together. Stored event tracks and
public runtime representations do not change: the old spellings already lowered to the same core applications.

The rejected alternatives are a new `index` or `parameter` keyword, retaining angle brackets, and treating every family
argument as an index. The first two preserve a syntactic split with no semantic operation behind it; the last changes
motives and makes uniform declaration context constructor-local. None improves the calculus or pattern-unification
discipline.
