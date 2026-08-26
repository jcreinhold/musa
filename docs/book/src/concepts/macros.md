# Adapters: macros as ordinary functions

An adapter reads a region of a piece and answers with syntax the compiler then reads as if the composer had written it.
That is Musa's macro system, and the whole of it. There is no macro language, no macro monad, and no separate macro
evaluator: an adapter is a function, and the thing that runs it is the thing that runs everything else.

## What an adapter is

A package module that declares a level and an `expand`:

```musa
    let level = "editable";
```

```musa
    let expand = fn (
        region: Syntax(TokenTree),
    ) -> Result(Syntax(TokenTree), Pair(Syntax(TokenTree), Text)) {
```

An ordinary `let`, at an ordinary function type, inside an ordinary `library`. It may declare its own data and its own
helpers, and it does, because it is an ordinary library and not a dialect:

```musa
    data Refusal {
        Nothing,
        Found(node: Syntax(TokenTree)),
    }
```

```musa
    let first_refusal = fn (found: Refusal, later: Refusal) -> Refusal {
        match found {
            Found(node) -> Found(node),
            Nothing -> later,
        }
    };
```

A piece reaches it with a syntax import and a region:

```musa
    import syntax std::adapters::doubled as doubled;

    let pair = syntax doubled { c4 };
```

The region's contents are read by the fixed lexer and grouper — an adapter does not extend either — and the value
`expand` answers with is resolved and checked in the *piece's* scope, so what the adapter writes, the piece imports.

## The two categories

`Syntax` takes a category, and there are two. `Syntax(TokenTree)` is a tree somebody wrote and nobody parsed;
`Syntax(Expr)` is a tree that has been read as an expression, which is a claim, established by running the ordinary
parser:

```text
as_expression : Syntax(TokenTree) -> Option(Syntax(Expr))
forget        : Syntax(Expr) -> Syntax(TokenTree)
```

Both directions are written. Musa has no subtyping, so an `Expr` standing where a `TokenTree` is wanted goes through
`forget`, at the site, in the source. It reads like noise the first time and it is the reason a category means anything:
a position that quietly accepted either would be a position that told you nothing.

## Quoting

The way to build syntax is to write it:

```text
quote at here { (1, ${ inner(here) }) }
```

`here` is a `NodePath`, and the only place one comes from is a traversal — an adapter cannot invent a place, which is
what keeps every node it builds attached to a node the composer wrote. A quote builds at `Expr`. `${ e }` splices one
node, `$x` is its shorthand when the expression is a name, and `$..xs` splices a run.

Names inside a quote are hygienic. A binder the quote writes is renamed on the way out, so it cannot capture a name
spliced into it, and a name spliced in still means what it meant where it was written. The one cost to an author is that
a quote may not itself write a name the renaming could produce — `item_g0` and the like are the compiler's, and writing
one is refused where the quote is, not where it expands.

## Provenance is carried and never read

Every node an adapter passes through keeps its own source information, and every node it builds records the derivation
that made it. An adapter cannot look at either: there is no operation from a node to a range, and none from a node to a
span. It can point — hand back a node it was given, and the compiler reads the place off that node — and that is all.

This is what makes [Source and provenance](provenance.md) hold across an expansion rather than up to it. A composer
hovering a generated note reaches the region that produced it because the thread was never the adapter's to cut.

## Why this is smaller than it usually is

Idris2 carries two macro systems. Syntax rewriting handles the shallow cases, and anything real runs in an `Elab` monad,
where every type crossing between the object language and the macro language needs a `Reify` and a `Reflect` instance —
roughly 3,200 lines of them in `TTImp/Reflect.idr` and `Core/Reflect.idr`, which exist because the macro language is a
different language from the one being compiled.

Musa needs none of it, and not because it is cleverer. An adapter is a function over ordinary values, checked by the
checker that checks the piece and run by the evaluator that evaluates the piece. There is nothing to reify *into*,
because there is no second world to cross into. Totality comes along for free: an adapter is a definition, so the
termination checker that reads every other definition reads this one — an `expand` that calls itself is told it "calls
itself on something this checker cannot see decrease", in the same words any recursive function gets. That is why the
phase can afford the ordinary evaluator instead of a sandboxed one.

The governing account, including the properties that keep quotation typed and hygienic, is
[Typed quotation](../../../rules/language/11-quotation.md).
