# 56. The macro layer is already one thing, and what prompt 160 has left to do

**Status: governs nothing.** `../../../rules/language/11-quotation.md` holds the decisions; this page holds the audit
behind prompt 160's repair. Written at prompt 160, before a line of it was implemented.

## 1. What 160 was written to deliver

Prompt 160 was drafted early in the language pass, when quotation was a construct of a separate compile-time checker and
an expander of its own. Its Task reads:

> a macro is an ordinary total function from `Syntax` to `Syntax`, run by the same evaluator that checks everything
> else, with typed quotation, splicing, and hygiene, and with its provenance preserved.

Every clause of that is now true. The prompts that made each one true landed between 138 and 159, and none of them was
this one. What follows is the audit, claim by claim, because a prompt whose Target is already met is drift of the kind
`../../../../AGENTS.md` forbids — and the repair has to say *which* prompt met it, or it is an assertion rather than a
finding.

## 2. The audit

| 160's claim | State | Where |
| --- | --- | --- |
| A macro is an ordinary function `Syntax -> Syntax` | **Held.** An adapter's `expand` is `fn (region: Syntax<TokenTree>) -> Result<Syntax<TokenTree>, Pair<Syntax<TokenTree>, Text>>`, an ordinary declaration in an ordinary `library`. | 138, 139 |
| Run by the same evaluator that checks everything else | **Held.** `phase::read_adapter_module` calls `document::elaborate`, and `phase::transform::expand_syntax` normalizes the resulting term with `musa_calculus`. There is no second checker and no second evaluator to delete. | 142 |
| Typed quotation | **Held.** `Syntax : Cat -> Type 0` with `Expr` and `TokenTree`; `quote at here { … }` builds at `Expr`. | 138, 145, 159 |
| Splicing, including sequence splices | **Held.** `${ e }`, `$x`, `$..xs`, lowered through `instantiate_quote` with the splices as a list of lists. | 139, 141ga |
| Hygiene | **Held, and lawed in both directions.** A quoted binder does not capture a spliced name; a quoted binder and a quoted use of it are one name; and a quote that writes a name the printer's own hygiene could produce is refused where it is written. | 139 |
| Provenance survives expansion | **Held.** `a_preserved_input_node_keeps_its_own_source_information`, and `SourceInfo` has no eliminator, so an adapter can neither read provenance nor fabricate it. | 139 |
| Totality | **Held.** An adapter module is checked by the same kernel as everything else, so its `expand` terminates; `expand/tests.rs` carries the structural-descent laws. | 155a, 158 |
| Phase conservativity | **Held.** `Syntax<Cat>` and the phase operations are unspellable in ordinary source, with compile-fail laws. | 142, and `quotation_laws.rs:613` |

Eight for eight. The prompt's Design section is a good argument that is no longer a plan.

## 3. The two claims that are not merely done

**"After 159, `c` is an index a `match` can refine" is false, and 159 is what made it false.**
[Note 55](55-cat-stays-a-base-type.md) measured the family and the repair to `11-quotation.md` §1 took `Cat` out of the
running: it is a base type with literal values, `Syntax` has no eliminator, and there is no `match` on a syntax value
for an index to be refined in. The category-refinement law in 160's Target cannot be written, for exactly the reason
note 55 §3 gives, and note 55 §5 states the condition on which it could be re-opened.

**"the builtins 159 removed no longer needed" describes a prompt that did not happen.** 159 removed a *coercion rule*
and added a row: `forget` is now an ordinary `SyntaxOp` an adapter writes. Nothing about the quotation vocabulary got
smaller, and nothing in `expand/` was left holding a builtin it no longer needs.

## 4. What is actually left

Two things, and they are both real.

**The claim is not checkable in one place.** Eight laws in six files, each true about its own construct, is not the same
artifact as "the macro layer is one thing, and here is how you would falsify that". The evaluator claim in particular is
*asserted* in prose — `phase/mod.rs`'s module doc says "the core this lowers into is `musa-calculus`" — and nothing
fails if a second evaluation path is added tomorrow.

**There is no chapter.** `docs/book/` has `concepts/`, `guide/`, `how-to/`, `reference/`, and `tutorials/`, and not one
page about writing an adapter. The one production adapter, `stdlib/src/adapters/staff.musa`, is read today by reading
its source. `docs/README.md` gives `docs/book/` the teaching role, so this is a gap in the directory that owns it rather
than a nicety.

## 5. The prose the cutover falsified

Three places still describe the state before prompt 142, and a reader checking the claim in §2 would be misled by each:

- `docs/plan/code-map/spec-to-implementation-map.md`, "Quotation in the core" — marked **implemented, unreached**, and
  its **Owes** paragraph asks prompt 142 to delete `ExprKind::SyntaxQuote` and supply "§1's forgetting rule — an
  *acceptance* rule the core does not have". `ExprKind` has one mention left in the whole crate, inside a comment about
  its own removal; the acceptance rule was supplied by 142 and deleted again by 159.
- `crates/musa-compiler/src/lower/quotes/laws.rs` — calls one side of its comparison "the old checker and evaluator".
  Both sides are the core. The law still discriminates, because one side goes through whole-document elaboration and the
  expansion machine while the other hands a hand-assembled program to `check`/`normalize` directly, but that is not what
  its prose says it is doing.
- `crates/musa-compiler/src/registry.rs` — "`BUILTIN_OWNERSHIP` and `SYNTAX_OWNERSHIP` are the *old* checker's name
  lookup". They are tables translated into the core registry by `registry::builtins`.

Naming them here rather than fixing them silently: each is a sentence a repair has to change, and the list is what makes
the repair checkable.
