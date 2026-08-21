# The phase registry after the syntax index

## Purpose

Prompt 138 gave `Syntax` a category and gave the phase's kind and delimiter arguments real types. Retyping an operation
does not merely change its signature: an entry in
[`SYNTAX_OWNERSHIP`](../../../../crates/musa-compiler/src/phase/mod.rs) exists because it *hides* something an adapter
has no other way to reach, and an argument that stops being text can take that reason with it. So this note is the
survey the prompt asked for — every entry, its signature as of this prompt, what it still hides, and which of the three
fates ahead of it it is headed for.

Nothing here governs. [`docs/rules/language/11-quotation.md`](../../../rules/language/11-quotation.md) §5 owns the
deletion table; this note records the state the table is applied to, and where it and the registry now disagree.

## The seventeen entries

`Syntax` below is always `Syntax<TokenTree>` unless the category is written out. Every reader and every builder works at
`TokenTree`: reading claims nothing about a node it descends into, and a tree a builder assembled is a tree nobody has
parsed. `Expr` appears in exactly two signatures, and both of them run the real parser.

| Operation | Signature | Still hides | Fate |
| --- | --- | --- | --- |
| `recurse_syntax` | `((P) -> A, (P, TokenKind, Text) -> A, (P, Text) -> A, (P, Delimiter, List<Step<C, A>>) -> A, C, Syntax) -> A` | the node representation, each node's path, and the suspended entry into a child | kept (§5) |
| `run_syntax_step` | `(C, Step<C, A>) -> A` | which child and which algebra a step was minted for | kept (§5) |
| `syntax_fold_from_leaves` | `((P) -> A, (P, TokenKind, Text) -> A, (P, Text) -> A, (P, Delimiter, List<A>) -> A, Syntax) -> A` | the node representation and each node's path | kept (§5) |
| `syntax_at` | `(Syntax, P) -> Option<Syntax>` | descent into the representation, and an input node's untouched source information | kept (§5) |
| `syntax_anchor` | `(Syntax, P) -> Option<Syntax>` | a node's place in the region's reading order | kept (§5), lost its third argument in 145 — the place derives from the anchored node at a reserved site (note 48) |
| `syntax_number` | `(Syntax) -> Option<Ratio>` | the reader's own numeric reading of a literal token | kept (§5) |
| `syntax_built` | `(P, Nat, Nat) -> NodePath` | path derivation, which keeps output paths disjoint from input paths | **139** — the quote computes it |
| `syntax_binding` | `(P, Nat) -> BindingPath` | name identity as a derived coordinate | **139** |
| `syntax_token` | `(P, TokenKind, Text) -> Syntax` | generated source information, which an adapter carries but cannot forge | **139** |
| `syntax_identifier` | `(P, Text) -> Syntax` | generated source information, and the absence of a hygiene scope | **139** |
| `syntax_group` | `(P, Delimiter, List<Syntax>) -> Syntax` | generated source information — **and nothing else, now** | **139** |
| `syntax_binder` | `(BindingPath, Text) -> Syntax` | the opaque hygiene scope a binding carries | **139** |
| `syntax_reference` | `(P, BindingPath, Text) -> Syntax` | the same scope | **139** |
| `token_kind_equal` | `(TokenKind, TokenKind) -> Bool` | which of the lexer's kinds a token was given | **143** — a member of the `text_equal` family |
| `delimiter_equal` | `(Delimiter, Delimiter) -> Bool` | which of the four delimiters a group carries | **143** — the same family |
| `as_expression` | `(Syntax) -> Option<Syntax<Expr>>` | the real parser, run over a tree the adapter holds | kept (§5) |
| `checked_expression` | `(Syntax) -> Result<Syntax<Expr>, Text>` | output well-formedness: unique generated paths, one binder per binding | kept |

`P` is `NodePath`. Seventeen, where §5 counted fourteen: this prompt added `token_kind_equal`, `delimiter_equal`, and
`as_expression`, and §5 already names the last of the three among the survivors.

## What the retyping took away

Two entries lost a claim, and the loss is the point rather than an accident.

**`syntax_group` used to hide "the fixed grouper's delimiter set".** It could, because a transformer named a delimiter
as text and the operation was the only thing that knew which strings were real. `Delimiter` is a type with four values
and no others, so there is nothing left to know: the set is hidden by the type, at the place the value is made, and the
operation is left hiding only the generated source information every builder hides. That is not enough to earn an entry
— it is the same fact thirteen times — which is exactly why §5 replaces all of them with one quote.

**`checked_expression` used to be asked a third question**, "does this group name a real delimiter", and answered it
after the fact with `NotAnExpression::UnknownDelimiter`. That refusal is deleted here, and the variant with it. A
question answered when the value is constructed cannot be failed later.

Neither loss is a deletion in this prompt. Prompts 139 and 140 build on a working phase API, and an operation removed
before its replacement exists is a prompt that cannot be checked.

## Where the registry and §5 now disagree

§5's deletion table was written when the registry had fourteen entries and predicted seven survivors. Two rows need
reading with this note beside them:

1. **"string dispatch on token kinds and delimiters → prompt 138's typed `TokenKind` and `Delimiter`"** is discharged,
   but it left two *new* operations behind. They are not permanent — 143 collapses the whole `text_equal`/`nat_equal`
   family behind `Eq` — and the alternative was a second comparison discipline for two types, which is worse than two
   entries in a family already scheduled for collapse.
2. **Seven of fourteen therefore becomes seven of seventeen**, and the survivors are the same seven §5 named. The three
   added entries are two 143 deletions and one survivor `as_expression` that §5 had already listed.

That is a count to repair in §5 when prompt 139 applies the table, not a disagreement about which operations go.

## The one thing the survey found that the prompt did not predict

`Value::ty` cannot answer a syntax value's category, because there is no category in the value: `Syntax<Expr>` is a
claim the checker established and then erased. The evaluator's own invariant check —
[`infer::admits`](../../../../crates/musa-compiler/src/infer.rs) — compares a checked type against a value's
reconstructed one, so it has to be told that comparing categories there would contradict the claim rather than re-check
it. It is the second rule in that function, beside the one for type variables, and for the same reason: both are places
where a proof the checker already has cannot be restated from a value alone.

This is worth recording because prompt 139's quote makes it more load-bearing, not less. Every quoted expression is a
`Syntax<Expr>` whose value is an ordinary tree, and every splice of one into a token-tree position runs through the same
two rules.
