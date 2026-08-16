---
id: 141f
slug: phase-traversals
status: pending
depends_on: [141c, 141e]
phase: 3
---

# Let a Traversal Name What It Builds

## Task

Prompt 141c gave `musa-core` §5.8's second family — a structural eliminator that fires on a literal target and rewrites
to a term — and said in its own Task why it was a prerequisite: `recurse_syntax`, `run_syntax_step`, and
`syntax_fold_from_leaves` traverse a `Syntax`, which is a base type, so no library traversal can recurse on one. Prompt
141e went to register those three and could not.

The reason is one sentence long. All three have a group branch typed `NodePath → Delimiter → List A → A`, so firing at a
group node means building a **list term**, which means naming `List.Cons` — and a `Rewrite` is handed a `&Builtin` and a
`&Literal` and nothing else, while `Constant`'s fields are `pub(crate)`. A traversal that cannot name the constructors
of the type its own signature mentions cannot be written.

Settle it, and register the three.

## Read

- [`141c`](141c-structural-eliminators.md) in full — `Rewrite`, `Builtin::structural`, the target index, `eval.rs`'s
  structural arm, and `Registry::new`'s two target checks. Its Design proposed `fn(&[&Term]) -> Option<Term>`; the
  implementation narrowed it, and this prompt is where the narrowing is paid for.
- [`141e`](141e-compiler-registry.md)'s Design, which records the finding and the two encodings that were tried and
  refused: a λ standing where a `SyntaxStep` value belongs is ill-typed at a base type, and a payload holding a de
  Bruijn index is meaningless once it leaves the spine it was minted in.
- `crates/musa-compiler/src/core.rs`'s `SyntaxOp::instantiate` for `Fold`, `Recurse`, and `Run` — the three signatures,
  with `Type::SyntaxStep` and its two ordinary variables — and `eval_syntax`'s arms for them, which are the reductions
  said again.
- [`../../rules/language/02-core-calculus.md`](../../rules/language/02-core-calculus.md) §5.9 on the phase registry, and
  [`../../rules/language/11-quotation.md`](../../rules/language/11-quotation.md) §1 on what the sealed step is *for*:
  which child and which algebra a step was minted for, and that a transformer may enter a proper child and nothing else.
- [`136a`](136a-module-visibility.md) — `private`, module identity, and the filter. This prompt moves a seal from
  "opaque base type" to "constructor nothing outside the module can write", and that is the machinery that already
  exists to do it.
- `stdlib/src/adapters/staff.musa:1932`, `:1957`, `:2234`, and `stdlib/src/adapters/doubled.musa:107` — the four calls,
  which are what "registered" has to mean.
- Peyton Jones ch. 4 §4.3 and ch. 6 §6.2: a structured type's eliminator is a constant whose reduction rule *names the
  type's own constructors*, and the enriched calculus is defined as constants together with the rules that consume them.
  A rewrite that cannot name a constructor is not a reduction rule for the type it claims to eliminate.

## Design

**A structural builtin registers the vocabulary its rewrite may write.** The narrow reading of 141c's problem is that
`Rewrite` needs more arguments; the real one is that a rewrite needs *names*, and the host is the only thing that has
them. So `Builtin::structural_with(name, ty, target, vocabulary, rewrite)` takes a `Vec<Term>` of closed terms the host
resolved from its own context — `List.Empty`, `List.Cons`, `SyntaxStep.Step` — and `Builtin::vocabulary()` hands them
back to the rewrite, which already holds the `&Builtin`. Nothing about D3 moves: the terms are fixed at registration, a
`fn` still captures nothing, and two compilers that register the same vocabulary reduce identically.
`Builtin::structural` stays, as the no-vocabulary case, because the worked example in `base_laws.rs` needs none and a
required empty `Vec` at every call site is a parameter every caller passes the same value for.

**Everything else the rewrite needs, it already has by index.** A traversal's type parameters are arguments: the fold is
`(A : Type 0) → branches… → Syntax ⟨token-tree⟩ → A`, so `A` is nameable exactly the way a branch is, and
`List.Cons A child rest` is writable. That is the whole reason the answer is read in the environment of the arguments —
141c chose that, and this prompt is the first thing to use it for something other than a branch.

**The sealed step becomes a declared family holding a function, and `run_syntax_step` stops being a builtin.** A step
has to *capture* — which child, and which algebra — and the only sound capture in this calculus is a closure, because
the core evaluates a rewrite's answer in the argument environment and a λ in that answer becomes a value that holds it.
A base type cannot hold a closure; a declared family can hold a field of function type. So

```musa
data SyntaxStep (Context) (Answer) { private Step(run : Context -> Answer) }
```

and `run_syntax_step(context, step)` is that field applied. The seal survives the move and is *better* stated: it used
to be "this type is opaque because the compiler says so", and it becomes "this constructor is private to the phase
module", which is 136a's own mechanism and is checked by the same filter as everything else. What a transformer can do
is unchanged — it receives steps and runs them, and it cannot mint one for a node it chose.

**Sixteen registered, one defined, and the count says which.** `run_syntax_step` leaves the phase registry because a
projection is not a compiler-owned operation: it hides nothing, which is the test each `SYNTAX_OWNERSHIP` row already
states for itself. `Registry::new` would have refused it anyway — its target is a declared family, and a rewrite over a
declared family is the second ι-rule that check exists to catch. That refusal agreeing with the ownership test is
evidence the design is right rather than a coincidence to route around.

**Termination is argued, not assumed.** §5.8's structural obligation is that every application of the builtin in the
answer stands at a literal strictly smaller than the target. Both traversals apply themselves only at a *child* of the
node they fired on, and a child of a finite tree is strictly smaller in the node count that measures it. That is the
argument to write down and to sample, and §4's meter stays the backstop.

## Target

- `crates/musa-core/src/base.rs`: `Builtin::structural_with`, `Builtin::vocabulary`, the vocabulary on
  `BuiltinDeclaration`, and `Builtin::structural` restated as the empty-vocabulary case — doc-commented with their
  invariants before the implementation. `Rewrite` keeps its signature.
- `crates/musa-core/tests/suite/base_laws.rs`: a worked traversal whose rewrite builds a list from its registered
  vocabulary and names a type parameter by index, with the laws — the answer is evaluated by the core, a vocabulary term
  is closed and therefore scope-independent, and two registries with the same vocabulary reduce a term identically.
- `crates/musa-compiler/src/prelude.rs`: `SyntaxStep` declared, with its constructor private to the phase module.
- `crates/musa-compiler/src/registry.rs`: `recurse_syntax` and `syntax_fold_from_leaves` registered as structural
  eliminators with their vocabularies and their rewrites, and `run_syntax_step` defined over `SyntaxStep` rather than
  registered. 141e's accounting law updated to say 16 registered and 1 defined.
- Laws in `crates/musa-compiler/tests/suite/`: each traversal reduces a token, an identifier, and a nested group to the
  same answer the old `eval_syntax` gives on the same tree; a branch is passed through unevaluated; a neutral node
  leaves a neutral; and the termination argument sampled — every self-application in an answer stands at a strictly
  smaller node.
- `docs/plan/code-map/` rows for `musa-core` and `musa-compiler`.

## Check

```sh
cargo build --workspace
cargo nextest run -p musa-core -p musa-compiler
cargo clippy --all-targets -p musa-core -p musa-compiler -- -D warnings
cargo fmt --check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
python3 scripts/renumber-prompts.py audit
```

Commit as `Let a traversal name what it builds`.

## Stop

- No `Value` in a public signature, no callback into the evaluator, and no interior mutability. A vocabulary is a list
  of closed terms fixed at registration; anything that could change between two reductions re-opens D3.
- No surface change, no `.musa` file touched, and no old checking path deleted. Prompt 142 owns the cutover, and the old
  `eval_syntax` is what these rules are checked *against* until it does.
- No collection eliminator registered. 141c argued that exclusion and it stands.
- No amendment to §5.8 or §5.9. The seal moving from a base type to a private constructor is a change of mechanism, not
  of what the phase may do; if it turns out to be more than that, it is a finding and stop condition 4.
- No new phase operation. Seventeen rows in, sixteen registrations and one definition out.
