# Typed quotation, splicing, and syntax patterns

This document fixes how an adapter *writes* syntax and how it *takes syntax apart*. It is the piece that decides whether
Musa is homoiconic in practice or only in principle: the phase already hands an adapter a syntax value and already lets
it hand one back, and what it does not do is let the adapter say what it means.

The measurement is in `stdlib/src/adapters/staff.musa`, and it is specific. Six helpers `call1`–`call7` assemble a call
by hand out of a layout group, an identifier, a parenthesis group, and comma tokens; twenty of their call sites write
what the author meant as an argument list. Fifty-six `syntax_built` calls carry twenty-seven distinct hand-allocated
role integers whose uniqueness nothing checks. `call3(spot, 2, "Note", …)` is a function name written as a string, an
identity written as a number, and a shape written as a nested tree — three things a quotation says once:

```musa
quote at spot { Note($spelled, $value, $tie) }
```

Everything below is the rules that make that line mean what it looks like.

This is **phase machinery**. `00-semantics.md` §2 fixes the phase environment `Σφ`, and nothing here is reachable from
ordinary source: an adapter module is checked in a scope where these names resolve, and a piece is checked in one where
they do not. Law 11 of the recursor — phase conservativity — is unchanged and is the reason that separation is stated
rather than assumed.

## 1. `Syntax<Cat>` is an indexed family

`Cat` is an ordinary two-case enum, and `Syntax` is an inductive family indexed by it (`02-core-calculus.md` §1.1):

```text
enum Cat { Expr, TokenTree }

Syntax : (c : Cat) → Type        % the constructors are compiler-owned (127da's builder facade)
```

The representation does not change. A syntax value is still the lossless token tree prompt 127da declared — `Missing`,
`Token`, `Identifier`, `Group` over a `SourceInfo` with no eliminator — and `Syntax<TokenTree>` is that tree with
nothing claimed about it. **The index is a claim about how the tree parses**, and `Expr` says that the real parser read
this tree as an expression.

**Two cases, not four.** Prompt 131 wrote `Item` and `Pattern` beside them and prompt 132's trial found that no program
constructs either: both trialled adapters build expressions and read token trees, and each unused case carries its own
share of the round-trip obligation below. `Item` comes back when an adapter expands a region into declarations rather
than into an expression, which no planned adapter does; re-adding it is an enum case and its round-trip test.

Two rules follow, and between them they are the whole discipline:

- **A refined claim is introduced only by an operation that establishes it.** The constructors are compiler-owned, so
  there is no way to assert `Syntax<Expr>` about a tree nobody parsed. The introduction forms are §2's quote, whose body
  the real parser read at that category; §4's pattern, which re-establishes a claim by matching a shape that carries it;
  and a **checked parse**, `as_expression : Syntax<TokenTree> -> Option<Syntax<Expr>>`, which runs the real parser over
  a tree the adapter already holds and answers `None` when it is not an expression.
- **A claim is forgotten wherever it is not needed.** A position of category `TokenTree` accepts a value of any
  category, because a token-tree position is precisely one that has not been parsed as anything more specific. Every
  other position requires its own category exactly. This is one acceptance rule rather than a `forget` operation the
  author writes, and it is why splicing an expression into an argument list needs no ceremony.

**What the index buys, and where.** Prompt 131 argued it at *construction*: an adapter that builds something in
expression position and gets it wrong would learn at the line that made it rather than at expansion time. Prompt 132's
trial found that argument does not survive its own conclusion — once construction goes through §2's quote, the parser
has already read the body, so a constructed node cannot be miscategorized and the index catches nothing there.

**What it buys is the splice boundary**, and that is a real thing to buy. An adapter that lifts a node out of the
composer's own region holds a `Syntax<TokenTree>` and must put it where an expression stands; `bar (4, 4) { { } }` puts
a brace group where a pitch belongs. `as_expression` is where that is decided, so the diagnostic lands on the composer's
line instead of on the region after a malformed tree has reached `checked_expression`. This is also why splicing the
composer's node is *preferable* to rebuilding one from its text: the spliced node keeps its `Original` source
information (§3), so Origin, `edit`, and `print` all point back at what was written.

**The obligation the index creates**, owed by prompts 138 and 147: every value of `Syntax<Expr>` prints as source that
the parser reads back as an expression, and `as_expression` answers `Some` exactly when it does. The index is a
certificate, and a certificate nobody checks is a comment. Until that is discharged, the index is a claim the elaborator
makes and the implementation is believed to keep.

## 2. `quote at here { … }`

```text
quote at p { body }        p : NodePath        body : Musa surface syntax
```

The quote elaborates to a `Syntax<c>` construction, where `c` is the category the expected type demands — and the demand
is always `Expr`, because construction is fixed at `⟨expr⟩` and a `⟨token-tree⟩` position receives the value by §1's
forgetting rule. What a quote builds is therefore never in doubt; what can fail is that the built tree is no expression,
which the certificate below checks.

**The body is read by the real parser.** Not a template dialect, not a string, not a token-stream approximation. A
quotation that does not share the parser is a second grammar to keep in step with the first, and it is the "sublanguage
by subtraction" root `AGENTS.md` forbids — every convenience the dialect lacked would be paid by every adapter author
instead of once by the compiler. The concrete consequence is that a quote's body is formatted, highlighted, and
diagnosed by the same machinery as the file around it.

**A quote builds at `⟨expr⟩`, and the certificate is checked rather than believed.** Trying each category until one
parses would be search, and it would make an ambiguous body's meaning depend on the order the elaborator tried — so the
category is not tried at all: `instantiate_quote` answers `Syntax ⟨expr⟩` for every quote, and before it answers, the
built tree must parse as an expression or the rule does not reduce. `let e = quote at p { Note($x) };` therefore infers
`Syntax<Expr>`; the annotation `let e: Syntax<TokenTree> = …` is the same value carried by §1's rule, not a second way
to build. This is the checking-form discipline this section stated before the registry fixed the category: its reason
was that two categories tempted a search, and fixing one by construction is that reason discharged.

**Splicing.**

```ebnf
splice := "$" IDENT | "${" expr "}" | "$.." IDENT
```

- `${ e }` splices the value of one expression where one node stands, and `$x` is its shorthand for the case where that
  expression is a name. The position's category must accept the value's (§1), and a mismatch is a compile-time error
  naming both categories and pointing at the splice.
- The expression form is not a convenience. A block holds exactly one expression and there is no `let` inside it
  (`01-surface.md` §1), so a splice argument that is *computed* — `${ dot_count(here, dots) }`,
  `${ stated_field(region, here, read.head.beats, "…")? } ` — has nowhere else to be written. Fifteen of the twenty
  construction sites in the staff adapter splice a computed value, and a helper function per site would be `call7` with
  better parameter names. `${ e }` is also the spelling `01-surface.md` §7 already fixed for the kernel quote's holes,
  so the two quotations agree here rather than differing (§6).
- `?` inside a splice leaves the enclosing *function*, exactly as `01-surface.md` §1 says: a quote is not a function
  boundary and a splice is not an argument to one.
- `$..xs` splices a `List<Syntax<c>>` where a sequence is grammatical — an argument list, a group's contents, a run of
  items or statements. Where a sequence is not grammatical it is refused, naming the position.
- A splice stands where a **whole node** stands and never inside one. There is no way to build the identifier `abc` out
  of `$a` and `bc`, because a splice is not string concatenation and a syntax value is not text. The separator a
  sequence splice needs — the commas of an argument list — is supplied by the grammar of the position, which is exactly
  the work `call2`–`call7` were doing by hand.
- `$` is part of the quote's grammar. It is not an operator, it has no meaning outside a quote, and it cannot be applied
  to something that is not a splice.

**Hygiene.** An identifier written literally in a quote and an identifier that arrives through a splice are different
names even when they are spelled the same, and neither captures the other. This is the same rule `01-surface.md` §7
already states for kernel quotes — kernel identifiers never capture host identifiers, and alpha-renaming prevents
capture among inserted terms — applied one stage up, where the values being inserted are syntax rather than terms. The
scopes a syntax value carries are opaque (127da): package code may compare two names and must preserve the scopes it
received, and has no operation that constructs one.

**What a quote is not.** It is not `eval`: the result is a syntax value that the phase then checks and elaborates like
any other, and there is no operation from `Text` to `Syntax<c>`. It is not a procedural macro over a token stream: the
body is parsed, so an adapter cannot assemble syntax the grammar does not admit. Both are refused in the Stop list of
the prompt that wrote this document, and both are refused here for the same reason: they are holes in the boundary
`00-semantics.md` §2 exists to hold.

## 3. Provenance the elaborator computes

**Spliced syntax keeps the identity it arrived with.** A node that came from the composer's source is still that node,
with its `Original` source information, wherever a quote puts it. Nothing about being quoted changes what a node is.

**Syntax written literally inside a quote is derived**, with an identity the elaborator computes:

```text
Derived { origin, quotation, path }
```

- `origin` is the `at` node — the anchor the adapter pointed at, which is the only part an author writes and the only
  part that is a value.
- `quotation` identifies the quote itself, which distinguishes two quotes written at one anchor.
- `path` is the literal node's position inside that quote's own tree.

**The identity law.** Two literal nodes are the same derived node exactly when their `origin`, `quotation`, and `path`
are all equal, and nothing else makes two derived nodes equal.

Uniqueness follows structurally rather than by convention, and that is the whole point. `path` is a position in a tree,
so two distinct literal positions in one quote have distinct paths — which is why two occurrences of the same shape
inside one quote are distinguishable without anybody counting. `quotation` separates two quotes at one anchor. `origin`
separates two expansions. No author-supplied number takes part, so **there is nothing to miscount**: the twenty-seven
role integers of the staff adapter, and the child numbers threaded through `call1`–`call7`, are not written more
carefully — they are not written.

**This mints identity in the vocabulary that already exists, and amends nothing above it.** Prompt 127da's `SourceInfo`
has two cases, `Original(source range)` and `Generated(expansion, node path)`. A derived node is `Generated`, and
`Derived { origin, quotation, path }` is *how its node path is computed*, not a third case. One stage up,
`../across-stages/02-derivation-diagrams.md` §3 gives `Generated(source root, generation site, target, evidence)` and §6
says an adapter expansion record is a source of `Generated` steps and nothing more; the quote is the generation site and
the anchor is the source root. So `../across-stages/04-identity-and-realization.md` needs no amendment, and this
document records that it checked rather than leaving the reader to infer it.

`checked_expression` keeps its duplicate-path gate, and its job changes twice. Its parse half generalizes to §1's
`as_expression`, which runs at any node rather than only over a whole result, so the "this is not an expression"
diagnostic can land on the composer's line. Its duplicate-path half stays and used to catch an author who reused a role
integer; now the derivation makes that unreachable, so what it catches is a compiler defect. Keeping it is not
belt-and-braces: it is the test that makes "unique by construction" a claim with evidence rather than an assertion, and
prompt 148's audit consumes it.

## 4. Quotation as a pattern

The inverse form destructures syntax by writing the shape it is looking for:

```musa
match node {
    quote { $head($..args) } -> reading(head, args),
    quote { { $..items } }   -> block(items),
    _                        -> refuse(node, "this adapter reads a call or a block"),
}
```

Two rules keep it honest.

- **A pattern quote binds only splice variables.** `$a` and `$..xs` bind; everything else in the pattern is a literal to
  be matched. There is no accidental capture of a literal token, and no wildcard that is not written as one. A pattern
  is read at the scrutinee's category, so the claims it binds are the claims the position carries.
- **Matching is on syntactic shape, never on provenance.** A derived node and a source node with the same shape match
  the same pattern. An adapter therefore cannot branch on where a node came from — which is prompt 127da's "an adapter
  cannot fabricate provenance" seen from the other side: it cannot read it either. `SourceInfo` still has no eliminator,
  and a pattern is not one.

**A quote pattern constrains and does not enumerate.** Like a literal pattern, it tests a shape rather than covering a
type's cases, so a `match` whose arms are quote patterns is not exhaustive and needs a catch-all — `02-core-calculus.md`
§6.2 decides coverage, and the case tree cannot know that a finite set of shapes exhausts the token trees. The refusal
arm above is not defensive style; it is the arm coverage requires, and it is where the adapter says what it reads.

At most one `$..xs` may appear in one sequence pattern. Two would make matching a search for a split point, and choosing
the split would be a guess about the author's intent.

**What this deletes, precisely, and what it does not.** The staff adapter's dispatch table has **three** halves.

1. **Shape decisions** — is this a call, a bracketed list, a block, an operator application — become patterns, and the
   `text_equal` chain over them goes.
2. **Leaf-token decisions** — is this token a `PitchLiteral` or a `Rational` — become `==` against prompt 138's typed
   `TokenKind` and `Delimiter`, which this document may name and does not define.
3. **Notation keywords** — `bar`, `rest`, `slur`, `clef`, fifteen of them — are neither, and **nothing here removes
   them.** They are not shapes: `bar (4, 4) { c5/1 }` is three siblings of a layout group and Musa's grammar has no form
   that matches it, so a quote pattern written at `Expr` cannot describe it. They are not token kinds: `bar` and `slur`
   are both `Identifier`.

Prompt 131 wrote "two halves" and prompt 132's trial found the third. The answer for it is not a quote pattern —
`quote { bra }` is a perfectly good quote of a perfectly good identifier, so a misspelling stays an arm that silently
never matches, which is the failure the `text_equal` chain already has. The answer is an ordinary `enum` of the
notation's words and one `Text`-to-case lookup, which centralizes every literal in one function and makes the dispatch
that reads it coverage-checked. That is `01-surface.md` §1.3's enum doing its job, not this form.

Saying which half is which is what stops prompt 145 from being surprised by the remainder, and the trial's measurement
is that the dispatch table shrinks by about a sixth rather than by a half.

**The trial found no quote pattern in either adapter**, for a reason worth recording here rather than discovering twice:
a quote pattern is written in Musa, and the notation both adapters read is not Musa. The form is for an adapter over
Musa syntax — a template dialect, a lint, a structured edit — and prompt 147's freeze is where an absent user is
decided.

## 5. The single-descent rule, satisfied rather than weakened

`00-semantics.md` §2 said descent into a syntax value happens in exactly one place. It now happens in two, and the
amendment lands with this document because it is the same decision. The reason the rule existed is unchanged: **the
phase must not grow a second uncontrolled traversal**, because an uncontrolled traversal is where totality, path
uniqueness, and opacity would all be lost at once. What makes the pattern form controlled is that it is not a traversal
at all:

- it destructures **one level** of a shape written in the source grammar, and any deeper reading is another `match` an
  author wrote, or the recursor;
- it **supplies no path**, because the sub-syntax it binds already carries its own;
- it **reveals no `SourceInfo`**, no scope, and no algebra;
- it **decreases**, because every bound sub-syntax is a proper child of the scrutinee, so a definition that recurses
  through a quote pattern is checked by the ordinary measure of `02-core-calculus.md` §2.4 rather than by a special
  rule.

Of the eleven recursor laws, the two a second descent could break are path uniqueness (law 4) and opacity (law 8), and
the four properties above are what discharge them for this form.

**What survives of the sealed-step recursor**, as a list rather than a gesture:

| Kept | Why |
| --- | --- |
| `recurse_syntax` and its inherited context | traversing syntax of unknown shape is a different job from matching a known one, and prompt 127dcfaf's design is the right answer to it |
| `run_syntax_step`, and sealing | a step still carries its own child and algebra, so nothing a pattern binds can be re-associated with a different traversal |
| `syntax_fold_from_leaves` | derived at `C = Unit` by law 9, and named so the eagerness is visible |
| `SyntaxStep<C, A>` is never storable | it holds four closures and a child (`02-core-calculus.md` §1.2); a pattern binds syntax, which is storable, so this form does not weaken the boundary |
| paths derived, `SourceInfo` unreadable, scopes unforgeable | 127da's three guarantees, none of which quotation touches |

**What does not survive:**

| Deleted | Replaced by |
| --- | --- |
| `call1`–`call7`, and their twenty call sites | one quote per site |
| every hand-written `syntax_group` / `syntax_token` assembly | the quote's own body |
| the role-integer argument to `syntax_built`, and twenty-seven hand-allocated values | §3's computed `Derived { origin, quotation, path }` |
| string dispatch on shapes, in the `text_equal` chain | §4's quote patterns |
| string dispatch on token kinds and delimiters | prompt 138's typed `TokenKind` and `Delimiter` |
| the phase operations `syntax_built`, `syntax_token`, `syntax_identifier`, `syntax_group` | the quote, which is the only construction form left |
| the phase operations `syntax_binder`, `syntax_reference`, `syntax_binding` | §2's hygiene rule, which is structural; no shipped adapter calls any of the three |

Seven of the fourteen phase operations therefore go, leaving `recurse_syntax`, `run_syntax_step`,
`syntax_fold_from_leaves`, `syntax_at`, `syntax_anchor`, `syntax_number`, and `as_expression`. `syntax_anchor` also
loses its third argument, because the place it took is what §3 computes — as `Derived { origin, quotation, path }` of
the anchored node, at a reserved site no quote can draw, since a δ rule is a function of its arguments and nothing else
(`02-core-calculus.md` §5.8's D3) and the node's own path is already the unique name. Prompt 139 owns the deletions;
prompt 145 owns the anchor's arity, as the prompt that removes the last caller of the place argument. The binding three
come back when an adapter introduces a name the composer can see and refer to, which neither the staff nor the studio
adapter does.

## 6. Two quotations, one discipline, and they are not merged

Musa has two quotation forms and they stay two:

|  | `kernel T { … }` (`01-surface.md` §7) | `quote at p { … }` (this document) |
| --- | --- | --- |
| Stage | elaboration | the expansion phase |
| Builds | a closed event-track term | `Syntax<Cat>` |
| Holes hold | host **values**, instantiated into a term with no functions | **syntax**, spliced into syntax |
| Hole spelling | `${e}` | `${e}`, with `$x` as its shorthand, and `$..xs` for a sequence |

Four rules are shared, and this is the one place they are stated together:

1. **Holes are typed.** A hole's type or category is checked where the hole is written, and a mismatch names both sides.
2. **No capture between quoted and host identifiers**, in either direction, and none among inserted values.
3. **The completed quote must close and check before it becomes anything.** A quote is not a value until it is whole.
4. **The locus is where a hole is instantiated, not where its result lands.** The two differ under `let`, and a hole in
   a `let` value is instantiated once at the `let`'s own locus.

Two rules are not shared, and belong to the kernel quote alone, because they are about payloads and time rather than
about quotation: a raw payload says what the material is and nothing about where it goes, and raw `shift`, `scale`, and
`restrict` move occurrences rather than rewriting payloads. `01-surface.md` §7 keeps both.

**The merge is refused, and the reason is written here so a later reader finds it rather than rediscovering it.** A
single `Quote<Stage, Cat>` would have two instantiations sharing no operation but the word "quote": one splices values
into a language with no functions, at a stage where types are already known; the other splices trees into trees, at a
phase where nothing has been elaborated yet. `02-core-calculus.md` §6.1 keeps those two stages apart on purpose, and a
type that spanned them would be the contextual-`Music` mistake in a new place — a generality whose only content is that
two things have a similar shape. There is no third quotation form, and adding one is an amendment.

## 7. What each failure names

| Failure | The diagnostic names |
| --- | --- |
| body that is not surface grammar | the parse failure, at the position inside the quote, in the ordinary parser's words |
| spliced tree that is no longer an expression | the instantiate certificate, refusing the expansion — the reporting of it is 147's (§8) |
| `$x` at a position of a different category | both categories, the splice, and the position |
| `$..xs` where a sequence is not grammatical | the position, and that it admits one node |
| two `$..xs` in one sequence pattern | both, and that a split point would be a guess |
| splice inside a token | the token, and that a splice stands where a node stands |
| pattern quote whose arms leave a shape uncovered | *non-exhaustive match*, since a shape test enumerates nothing (§4) |
| a `match` arm no shape can reach | *unreachable arm*, from the ordinary case-tree analysis |
| duplicate derived path reaching `checked_expression` | the path and both sites — now a compiler defect rather than an author's (§3) |

## 8. Obligations

**This section states obligations; it discharges none.** They join the matrix of `02-core-calculus.md` §5.

| Obligation | What discharges it | Owed by |
| --- | --- | --- |
| **Index soundness** — a `Syntax<Expr>` prints as source the parser reads as an expression, and `as_expression` answers `Some` exactly then | round-trip property tests over the corpus, and the construction rules of §1 | 138, 147 |
| **Derived-identity injectivity** — the triple of §3 is injective on literal nodes, and no two distinct literal positions collide | the path-uniqueness argument, plus `checked_expression`'s gate as its executable evidence | 139, 148 |
| **Hygiene** — no identifier written in a quote captures one spliced in, or the reverse | the scope discipline of 127da, restated for splicing | 139, 147 |
| **Construction/pattern round trip** — matching a quote pattern against a quote built from the same shape returns the spliced values unchanged | property tests over generated shapes | 140, 147 |
| **Single-level descent** — a pattern binds only proper children and supplies no path | §5's four properties, checked as laws | 140, 148 |
| **Phase conservativity** (law 11) — ordinary source can neither name nor obtain `Syntax<Cat>`, and the completed phase result is storable data | unchanged from 127da and 127dcfaf | 147 |

The theoretical provenance of the constructions here — staged quotation, splicing at syntactic categories, and hygiene —
is [`citations.md`](citations.md) §14. What is Musa's own is one thing: derived identity as a computed triple rather
than an author-allocated tag, which is priced by the measurement at the top of this document.
