---
id: 147a
slug: names-through-the-context
status: done
depends_on: [147]
phase: 3
---

# One Named Node, and Reduction Behaviour in the Context

## Task

Four of `Shape`'s remaining fourteen variants are the same term wearing four declarations: `Const` holds a whole
declaration group, `Def` holds an elaborated definition, `Base` holds an `Arc<BaseDeclaration>` with a kind and host
rules, and `Builtin` holds a δ-table. Each is *a context entry smuggled into a term*, which
`docs/rules/language/02-core-calculus.md` §1 and §6 forbid in as many words. Replace all four with one `Named` node, and
make a name's reduction behaviour a `Definition` the context answers for. Fourteen shapes become eleven; §1's seven are
then reached by 151 and 157 deleting the four that remain.

**No program's acceptance moves.** This is a relocation, not a semantics change.

## Read

- `docs/rules/language/02-core-calculus.md` §1 (the seven), §1.1 (families and recursors), §5.8 (base types and
  builtins), §6 (the boundary this prompt enforces).
- `crates/musa-calculus/src/family/constant.rs` — `Constant`'s doc comment is the argument *against* this change,
  written when the previous calculus governed. It is answered in **Design** rather than left to contradict the code.
- `crates/musa-calculus/src/{eval,quote,convert}.rs`, `src/context.rs`, `src/program.rs`, `src/base.rs` — where the four
  declarations are read today.
- `crates/musa-calculus/src/case.rs` §`motive_level` — why a recursor's universe is part of a term's identity until 152.
- `crates/musa-calculus/src/base.rs` §`check_finite_data` and §`check_structural_targets`, and
  `crates/musa-calculus/src/elab/infer.rs` §`declared_parameters` — the three readers that decide a base type from a
  declared family, and a registered operation from a definition, off a term alone. They are why [`Role`] has six arms.
- `/Users/jcreinhold/Code/Idris2/src/Core/TT/Term.idr` (`Ref` and `NameType`) and `Core/Context.idr` (`Def`) — the
  reference split: a term carries a name and a cheap role, and the context owns the definition.
- Peyton Jones ch. 3 — a name is a reference into an environment, and the environment is not part of the expression.

## Design

**The target.**

```rust
enum Shape {
    Named { name: Name, role: Role },   // was Const, Def, Base, Builtin
    …
}

/// What the *term* fixes about a name: Idris2's `NameType`, with the two arms
/// §5.8 adds and the payload §1.3 forces.
enum Role { Defined, Constructor, TypeConstructor, Recursor(Sort), Base, Builtin }

/// What the *context* answers about a name.
enum Definition {
    Undeclared,
    Defined(program::Def),        // 155 replaces this with Compiled(CaseTree)
    Declared(family::Constant),   // a family, a constructor, or a recursor
    Base(base::Base),
    Builtin(base::Builtin),
}
```

A term says *what kind of thing the elaborator resolved this name to*, which is what a reader of the term needs; the
context says *what it reduces to*, which is what only a reducer needs. That is the split this prompt is for, and
[`Role`] is the reader's half: one word, where the four removed variants held an `Arc` to a whole declaration group, an
elaborated definition, a base's kind and host rules, or a δ-table.

**`Role` has six arms, and the merge to four was tried against the code and refused.** The tempting reading is that
`Defined` and `Builtin` are both "a function" and that `TypeConstructor` and `Base` are both "a type", with the context
answering which. Three checks say otherwise, and each of them reads the distinction off a *term* with no context in
hand:

- `base.rs`'s `check_finite_data` admits a registered base type applied to data (§5.8's D1) *and* a declared family
  applied to data (prompt 141's `List`, `Option`, `Result`), and refuses a base type this registry never registered —
  `Refusal::UnknownBase`, which `base_laws.rs`'s "a δ signature over a base type nobody registered" pins. A registry
  cannot see declarations, so a merged arm would have to admit every unregistered spelling as a possible family, and
  that refusal would become unreachable.
- `check_structural_targets` tells `Refusal::TargetNotABase` from `Refusal::UnknownBase` by the same reading, and
  `base_laws.rs` pins both.
- `elab/infer.rs`'s `declared_parameters` reads a *registered* signature's arrow as its parameter list and a *source*
  definition's as a parameter list plus a returned function, and tells them apart by `matches!(head.shape(),
  Shape::Builtin(_))`. Merging `Defined` and `Builtin` would make `transpose(P8)` fully applied.

Refusing to move those is what "no program's acceptance moves" means here, so the tag keeps them apart. It stays a
*tag*: six words against four declarations, which is the whole content of §6's boundary.

**The recursor's payload is not optional either.** §1.3 has no universe polymorphism, so a recursor takes its motive
universe per use site: `case.rs`'s `motive_level` mints `Nat.elim` at `Type 1` whenever a `match` computes a type, and
`family::Constant`'s equality already says two eliminations into different universes are two terms. A payload-free
recursor arm would make them one term and move acceptance. Prompt 152 makes recursors level-polymorphic and the level an
argument; when it lands, this arm loses its payload. Until then the universe is part of *which name this is*, which is a
term fact and not a context fact — so carrying it here is the boundary holding, not leaking.

**`Definition` has five arms, not the six the split-out sketch named.** Two corrections, both from the code:

- `Compiled(CaseTree)` cannot hold what a definition holds today, which is an elaborated value and its type
  (`crate::program::Defined`). Case trees are prompt 155's and are not stubbed into existence here, so the arm is
  `Defined(program::Def)` and 155 replaces it.
- `Constructor`, `TypeConstructor`, and the recursor the sketch forgot are one arm, because one lookup already answers
  all three: `Cx::declared` returns a `family::Found`, and `Found::at(sort)` turns it into the `family::Constant` that
  every reduction rule in `family/` already takes. Three arms re-spelling that constant's three roles would be a second
  *payload* free to disagree with the first. [`Role`] names them separately because a tag is what a reader of the term
  reads without a context; the constant that carries the declaration is one thing, found once.

`Globals::definition(name, role)` is the one lookup, and the three `Cx` already has — `declared`, `definition`,
`extern_named` — are its three cases. **The role chooses which one; they are not searched in an order.** Shadowing
between a declaration, a definition, and a registration that share a spelling is `01-surface.md` §1.3's question, and
the elaborator answered it when it turned the author's word into this term. Asking it again at reduction time would be a
second resolver, free to disagree with the first — and no ordering of the three tables reproduces the first, because
`elab/name.rs`'s `registered` and `hosted` resolve in the *host's* namespaces on purpose, so that a binding which
happens to spell `Nat` cannot change what a written number means. The stdlib already exercises this: it defines
`sounded`, and this build also registers a builtin `sounded`, so a `Shape::Named { name: "sounded", role: Builtin }`
that an order-based lookup answered from the definitions table applies a value that is not a function — which is what
`serial_laws`, `lint_laws`, and `transformational_harmony_laws` report when the order is tried.

**How the table reaches reduction, and why it is the environment.** Resolving a name at reduction time needs a table at
`eval`, and therefore at `apply`, and therefore at every closure `quote` and `convert` force. Threading a parameter
through those signatures would touch every function in the crate and would still leave a closure able to be applied
under a table other than the one it was built under.

The environment is where it goes. `Env` is already handed to `eval` beside the term and is already captured by every
`Closure`, so a closure applied later resolves against the table it was *built* under, which is the only table that can
be right. `eval`, `apply_closure`, `quote`, and `convert` keep their signatures.

```rust
struct Env { locals: List<Value>, globals: Globals }
struct Globals(Option<Arc<Tables>>);   // declared groups, definitions, host registry
```

`Env` stops being an alias for `List<Value>` and becomes the pair. `Globals` is one `Arc` behind an `Option` so that a
clone is one refcount bump and the empty table is a null check rather than an allocation — a context is cloned per
binder and an environment per closure, while the table changes once per declaration. `Cx` holds one `Globals` in place
of its three fields and hands it to the environment it evaluates in.

This is the answer to `family::Constant`'s doc comment, which argues the opposite: it says holding the group in the term
is what avoids "a parameter on `eval`, `quote`, `apply`, and `Cx` alike". That was true and it is not the trade here —
the table rides in a parameter all four already take.

**The rigid heads carry the globals their own types are written in.** `crate::eval::neutral_type` answers a head's type
with no context to ask, and it answers three of them by *evaluating a declaration term*: a base's kind, a builtin's
signature, a constant's telescope. Those terms are closed in locals and not in names, so `Env::EMPTY` stops being an
environment they can be read in. `Head::Const`, `Head::Base`, and `Head::Builtin` therefore carry the `Globals` they
were resolved under, exactly as `Head::Var` and `Head::Def` already carry the types *they* would otherwise have to
recompute. Globals are not part of a head's identity and conversion does not compare them. `Head::Var`, `Head::Def`, and
`Head::Meta` are unchanged.

Storing the evaluated type instead was rejected: a constant's type is built and evaluated per occurrence today only
because `neutral_type` is rarely asked, and making it eager at every name in every term would pay for a question almost
no occurrence asks.

**A name that the context does not declare.** Today a term is self-contained and evaluates anywhere; after this, a term
evaluated under a context that does not declare its name has no definition to find. That is `Definition::Undeclared`,
and it is a `Malformed` refusal carrying the name — the kernel's "this term was built wrong", which it is, since the
elaborator resolved the name once already. Never a silent resolution to a different declaration of the same spelling,
which is the one way this change could move acceptance without a test noticing.

## Target

- `crates/musa-calculus/src/term.rs`: `Named`, `Role`, and `Definition`; `Const`, `Def`, `Base`, and `Builtin` gone.
- `crates/musa-calculus/src/value.rs`: `Env` becomes locals plus `Globals`; `Head` keeps the resolved declaration, which
  is a value fact and was never the problem, and the three rigid heads carry the table their types are read in.
- `crates/musa-calculus/src/context.rs`: `Globals`, and `Cx` holding one in place of its three fields; one lookup,
  `Definition` by `Name` and `Role`.
- `crates/musa-calculus/src/{eval,quote,convert,show,elab/,family/,base.rs,program.rs}`: read the definition through the
  context instead of out of the term.
- `crates/musa-compiler`: only what the removed variants require. The facade does not move.

## Check

```sh
cargo nextest run --workspace
cargo nextest run --workspace --run-ignored all
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
```

**The oracle is exact acceptance**, as at 147: `--run-ignored all` must show the same failure list, byte for byte, that
prompt 166's Check records — same tests, same messages, same counts. A *count* that changes means a semantics change
crept in.

There is no net-negative diff requirement here and the absence is deliberate: the reduction table moves rather than
disappears, so the crate is not obliged to shrink. The term language is what shrinks, and `Shape` having eleven variants
is the measure.

Commit as `Reach a name's definition through the context`.

## Stop

- No case trees beyond `Definition::Defined` (155), no universe change (152), no `Indexed` deletion (151), no
  records-as-data (157), no metavariable solving (153).
- No new diagnostics beyond the one `Definition::Undeclared` needs.
