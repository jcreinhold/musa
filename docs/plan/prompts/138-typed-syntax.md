---
id: 138
slug: typed-syntax
status: done
depends_on: [137a]
phase: 3
---

# Give Syntax a Category, and the Phase API Its Types

## Task

Replace the phase's one untyped `Syntax` with the indexed family `Syntax<Cat>` over `Expr` and `TokenTree`; add
`as_expression`, the checked parse that is the only way a tree an adapter holds acquires the finer claim; replace the
stringly-typed token-kind and delimiter arguments in the phase registry with real types; and name the derived-provenance
representation `Derived { origin, quotation, path }` that the phase already computes, so its identity law is testable
here rather than two prompts from now. Nothing an adapter writes should still be a string that the compiler parses back.

## Read

- `docs/rules/language/11-quotation.md` §1, §3, and §5 **as prompt 132's trial left them** — the index has **two**
  cases, not the four prompt 131 wrote; `as_expression` is a third introduction form; and §5's table assigns the
  *deletion* of the construction operations to prompt 139. Read §1's "Two cases, not four" paragraph before anything
  else in this prompt: it is the correction this prompt's first version was written against the wrong side of.
- `docs/notes/research/language-design-closure/43-dependent-language-trial.md` — prompt 132's trial, and specifically
  its finding that the index earns nothing at *construction* and earns the splice boundary. That is the argument for
  what `as_expression` is for.
- `docs/rules/language/00-semantics.md` §2 — the phase environment `Σφ`, the phase-local types, and the two-descent
  rule. Its sentence naming the index's cases is repaired by this prompt's own repair commit, so read the repaired text.
- `crates/musa-compiler/src/core/mod.rs`'s `SYNTAX_OWNERSHIP` — 14 phase operations, and the ones that take a kind or a
  delimiter as `Text`: `syntax_token`, `syntax_group`, and `checked_expression`. Read what each hides, because the
  ownership entry is the argument for the operation existing at all and a retyped operation still owes one.
- `crates/musa-compiler/src/core/mod.rs`'s `reconcile` — "the only place the checker compares two types". §1's
  forgetting rule is directional and unification is not, so it lands there and nowhere else.
- `crates/musa-compiler/src/core/mod.rs`'s `qualified_name` — the checker already joins `A.b` into one flat name. That
  is why `TokenKind.PitchLiteral` needs no namespacing feature in a checker that has none.
- `stdlib/src/adapters/staff.musa`'s `text_equal(kind, "…")` and `text_equal(delimiter, "…")` call sites — the ones this
  prompt makes ill-typed rather than merely discouraged. There are about ten, and they move mechanically.
- `crates/musa-compiler/src/quote/mod.rs` — `SourceInfo`, `NodePath`, `PathStep::Built { role, child }`, and
  `DELIMITERS`. `Derived` is the name for what `NodePath::built` already computes; it does not start a second
  representation.
- `docs/rules/across-stages/04-identity-and-realization.md` — read it to confirm it needs no amendment, which is what
  `11-quotation.md` §3 already records. `Derived` is *how a `Generated` node's path is computed* and is not a case of
  anything above it, so `crates/musa-compiler/src/derivation.rs` — the across-stage DAG — is not where it goes.

## Design

**The index has two cases, and the reason is a measurement.** `Cat` is `Expr` and `TokenTree`. Prompt 131 wrote `Item`
and `Pattern` beside them; prompt 132's trial rewrote ten programs and found that nothing constructs either, and that
each unused case carries its own share of the round-trip obligation. `Item` comes back when an adapter expands a region
into declarations, which no planned adapter does, and it comes back as an enum case and its round-trip test.

**What the index buys is the splice boundary, not the construction site.** Prompt 131 argued it at construction — an
adapter building something in expression position would learn at the line that made it. The trial found that argument
does not survive its own conclusion: once construction goes through a quote, the parser has already read the body, so a
constructed node cannot be miscategorized. What is left is real. An adapter that lifts a node out of the composer's own
region holds a `Syntax<TokenTree>` and has to put it where an expression stands, and `bar (4, 4) { { } }` puts a brace
group where a pitch belongs. `as_expression : Syntax<TokenTree> -> Option<Syntax<Expr>>` is where that is decided, and
deciding it there is what puts the diagnostic on the composer's line.

**So `as_expression` is this prompt's, and it is what keeps the index from being inert.** Prompt 139's quote is the
other introduction form and does not exist yet; without a checked parse, nothing in the language could produce a
`Syntax<Expr>` at all and the index would be a claim with no introduction rule. It is implemented by printing the tree
and reading it with the real parser — the machinery `syntax.rs` already has — which is also exactly what discharges §8's
index-soundness obligation rather than asserting it.

**There is no cast between categories.** Not `unsafe`, not compiler-only, not "just for the parser". A conversion that
exists will be used, and every use of it is the untyped `Syntax` back again with extra steps. Forgetting is not a cast:
§1's rule is that a position of category `TokenTree` accepts a value of *any* category, because a token-tree position is
precisely one that has not been parsed as anything more specific. That is one acceptance rule the checker applies, not a
`forget` operation an author writes, and it is why splicing a certified expression back into a group needs no ceremony.

**The acceptance rule is directional, so it goes where direction exists.** `Unifier::unify` is symmetric; making
`Syntax<TokenTree>` unify with `Syntax<Expr>` there would make the reverse hold too, and the reverse is the uncertified
splice the index exists to refuse. `reconcile(expected, found, …)` is the one site that knows which side is the
position, so the rule is one arm there, ahead of the unification, and it fires only when `found` already resolves to a
concrete category.

**`TokenKind` and `Delimiter` become types, without a namespacing feature.** The first version of this prompt justified
them by prompt 136's namespaced enum constructors — which prompt 136 delivered in `musa-calculus`, while the phase
environment is in `musa-compiler`'s checker, which has bare constructors and no namespaces. The checker does have
something better suited: `qualified_name` already folds `A.b` into one flat name. So the two are phase-local base types
whose values are compiler-owned constants named `TokenKind.PitchLiteral` and `Delimiter.Braces` in that flat namespace,
resolved by a `phase_value` lookup mirroring the existing `phase_type`. Nothing collides with an adapter's own `data`,
nothing about ordinary source changes, and no general namespacing is added to a checker prompt 142 deletes.

The case sets are **generated** from what `musa-syntax` already owns — `SyntaxKind`'s token variants and `syntax.rs`'s
`DELIMITERS` — rather than hand-written beside them, so the drift law covers them: a token kind the lexer knows and the
phase does not is a build failure, not a silent gap. `read_token`'s `format!("{:?}", token.kind())` is what generation
replaces, and it is the exact shape of the mistake — a name written by a `Debug` impl and read back by string
comparison.

**Comparison joins a family that already exists.** `token_kind_equal` and `delimiter_equal` are two registry entries
beside `syntax_number`, and they are the members of the `text_equal`/`nat_equal` family the phase's own types need. They
are not permanent: prompt 143 collapses that whole family behind `Eq`, and these go with it. Adding two members of a
family scheduled for collapse is cheaper than inventing a second comparison discipline for two types.

**`syntax_built` keeps its role argument, and that is not a compromise.** The first version of this prompt removed it on
the grounds that "now the path does" the work — which is true only once a quote supplies the quotation, and the quote is
prompt 139. `11-quotation.md` §5 assigns the deletion of `syntax_built` *and* its role integer to 139, replaced by the
quote. Removing the argument here would leave nothing to separate two construction sites reading one input node,
`check_expression`'s duplicate-path gate would fire, and `stdlib/src/adapters/staff.musa` would stop expanding — which
this prompt's own Check forbids.

**What this prompt does with `Derived` instead is name it, and test it.** `PathStep::Built { role, child }` appended to
an origin path *is* `Derived { origin, quotation, path }` already: the role is the quotation, and the child number is a
one-step path. Writing that down as the representation — with the identity law stated and tested by minting nodes
directly — is what makes the law checkable now rather than in prompt 139, and the Design that asked for it was right
about that even where it was wrong about the signature. A law that only becomes testable two prompts later is a law
nobody checks.

**What the registry looks like afterwards** is part of this prompt's output, not an accident of it: state each entry's
new signature, say which hid nothing once the types were real — `syntax_group` claims to hide "the fixed grouper's
delimiter set", which a `Delimiter` type now hides instead — and mark those for deletion in 143 rather than deleting
them here. Prompts 139 and 140 still need a working phase API to build on.

**Laws.** A category mismatch is a type error at a position that demands `Expr`, and is accepted at a position that
demands `TokenTree`; both directions tested. `as_expression` answers `Some` exactly when the node prints as source the
parser reads as an expression — the round trip, over the corpus, which is §8's index-soundness obligation discharged
rather than deferred. `TokenKind` covers the lexer's token set exactly, tested by generation. Derived identity is
injective in all three components and stable across re-elaboration. The existing `recurse_syntax` and `run_syntax_step`
laws from prompt 127dcfaf still hold. The phase's types remain non-storable — `Syntax<Cat>` never crosses the expansion
boundary as data.

## Target

- `Syntax<Cat>` in the phase environment with the two categories, every `SYNTAX_OWNERSHIP` entry retyped, and §1's
  forgetting rule in `reconcile`.
- `as_expression`, the checked parse, with the round-trip law over the corpus.
- Generated `TokenKind` and `Delimiter` types, their flat-namespace constants, and `token_kind_equal` /
  `delimiter_equal`; the generated case sets tied to `musa-syntax`'s lexer table by a drift test.
- `Derived { origin, quotation, path }` named in `crates/musa-compiler/src/quote/mod.rs` as what the phase already
  computes, with its identity law tested directly.
- `stdlib/src/adapters/staff.musa` moved off `text_equal` on kinds and delimiters, mechanically.
- The registry survey: new signatures, and the entries marked for deletion in 143 with the reason.
- `crates/musa-compiler/tests/suite/` cases for the category laws, the round trip, the generation drift test, and
  derived identity.
- `docs/plan/code-map/` rows.

## Check

```sh
cargo build --workspace
cargo nextest run -p musa-syntax -p musa-compiler -p musa-calculus
cargo nextest run --workspace
cargo clippy --all-targets -p musa-syntax -p musa-compiler -p musa-calculus -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
```

`stdlib/src/adapters/staff.musa` must still expand: this prompt retypes two arguments it passes, so the adapter's call
sites move with them, mechanically and without redesign. A rewrite of the adapter here is prompt 145's work done in the
wrong commit.

Commit as `Give syntax a category, and the phase API its types`.

## Stop

- No quotation form and no pattern form. Prompts 139 and 140.
- No cast, coercion, or escape between categories, under any spelling or visibility. Forgetting is the checker's
  acceptance rule, not an operation.
- No third category, no user-declared category, and no category parameterized by anything.
- No change to `syntax_built`'s signature, and no deletion of its role argument. Prompt 139 replaces the operation.
- No deletion of `recurse_syntax`, `run_syntax_step`, or the derived `syntax_fold_from_leaves`. `11-quotation.md` §5
  says they survive.
- No deletion of registry entries. Prompt 143.
- No namespaced enum constructors in `musa-compiler`'s checker. Prompt 142 replaces that checker; a namespacing feature
  built in it now is deleted before it has a second caller.
- No rewrite of `stdlib/src/adapters/staff.musa` beyond the mechanical retyping. Prompt 145 measures the rewrite, and a
  partial rewrite here would corrupt that measurement.
