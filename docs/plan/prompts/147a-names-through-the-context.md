---
id: 147a
slug: names-through-the-context
status: pending
depends_on: [147]
phase: 3
---

# One Named Node, and Reduction Behaviour in the Context

## Task

Four of `Shape`'s remaining fourteen variants are the same term wearing four declarations: `Const` holds a whole
declaration group, `Def` holds an elaborated definition, `Base` holds an `Arc<BaseDeclaration>` with a kind and host
rules, and `Builtin` holds a δ-table. Each is *a context entry smuggled into a term*, which
`docs/rules/language/02-core-calculus.md` §1 and §6 forbid in as many words. Replace all four with one `Named` node,
and make a name's reduction behaviour a `Definition` the context answers for. Fourteen shapes become eleven; §1's seven
are then reached by 151 and 157 deleting the four that remain.

**No program's acceptance moves.** This is a relocation, not a semantics change.

## Read

- `docs/rules/language/02-core-calculus.md` §1 (the seven), §1.1 (families and recursors), §5.8 (base types and
  builtins), §6 (the boundary this prompt enforces).
- `crates/musa-calculus/src/family/constant.rs` — `Constant`'s doc comment is the argument *against* this change,
  written when the previous calculus governed. It is answered in **Design** rather than left to contradict the code.
- `crates/musa-calculus/src/{eval,quote,convert}.rs`, `src/context.rs`, `src/program.rs`, `src/base.rs` — where the
  four declarations are read today.
- `crates/musa-calculus/src/case.rs` §`motive_level` — why a recursor's universe is part of a term's identity until
  152.
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

/// What the *term* fixes about a name.
enum Role { Function, Constructor, TypeConstructor, Recursor(Sort) }

/// What the *context* answers about a name.
enum Definition {
    Undeclared,
    Defined { ty: Value, value: Value },   // 155 replaces this with Compiled(CaseTree)
    Constructor { group: Arc<Group>, family: u32, which: u32 },
    TypeConstructor { group: Arc<Group>, family: u32 },
    Base(Arc<BaseDeclaration>),
    Builtin(Builtin),
}
```

**`Role` has four arms and the fourth is not optional.** §1.3 has no universe polymorphism, so a recursor takes its
motive universe per use site: `case.rs`'s `motive_level` mints `Nat.elim` at `Type 1` whenever a `match` computes a
type, and `family::Constant`'s equality already says two eliminations into different universes are two terms. A three-
arm `Role` would make them one term and move acceptance. Prompt 152 makes recursors level-polymorphic and the level an
argument; when it lands, this arm loses its payload. Until then the universe is part of *which name this is*, which is
a term fact and not a context fact — so carrying it here is the boundary holding, not leaking.

**`Definition::Defined` is a placeholder with a date on it.** A definition today is an elaborated value and its type
(`crate::program::Defined`), not a compiled case tree; case trees are prompt 155's and are not stubbed into existence
here. 155 replaces this arm with `Compiled(CaseTree)` and nothing else in the enum moves.

**How the table reaches reduction, and why it is the environment.** Resolving a name at reduction time needs a table
at `eval`, and therefore at `apply`, and therefore at every closure `quote` and `convert` force. Threading a parameter
through those signatures would touch every function in the crate and would still leave a closure able to be applied
under a table other than the one it was built under.

The environment is where it goes. `Env` is already handed to `eval` beside the term and is already captured by every
`Closure`, so a closure applied later resolves against the table it was *built* under, which is the only table that can
be right. `eval`, `apply_closure`, `quote`, and `convert` keep their signatures; `Cx` hands its declarations,
definitions, and registry to the `Env` it evaluates in, and `Env::EMPTY` is the empty table — the context that declares
nothing, which is what every test written before this rule existed already had.

This is the answer to `family::Constant`'s doc comment, which argues the opposite: it says holding the group in the
term is what avoids "a parameter on `eval`, `quote`, `apply`, and `Cx` alike". That was true and it is not the trade
here — the table rides in a parameter all four already take.

**A name that the context does not declare.** Today a term is self-contained and evaluates anywhere; after this, a
term evaluated under a context that does not declare its name has no definition to find. That is `Definition::
Undeclared`, and it is a refusal at the point of use with the name in it — never a silent resolution to a different
declaration of the same spelling, which is the one way this change could move acceptance without a test noticing.

## Target

- `crates/musa-calculus/src/term.rs`: `Named`, `Role`, and `Definition`; `Const`, `Def`, `Base`, and `Builtin` gone.
- `crates/musa-calculus/src/value.rs`: `Env` carries the definition table; `Head` keeps the resolved declaration, which
  is a value fact and was never the problem.
- `crates/musa-calculus/src/context.rs`: `Cx` builds the table it evaluates under; one lookup, `Definition` by `Name`.
- `crates/musa-calculus/src/{eval,quote,convert,show,elab/,family/,base.rs,program.rs}`: read the definition through
  the context instead of out of the term.
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
disappears, so the crate is not obliged to shrink. The term language is what shrinks, and `Shape` having eleven
variants is the measure.

Commit as `Reach a name's definition through the context`.

## Stop

- No case trees beyond `Definition::Defined` (155), no universe change (152), no `Indexed` deletion (151), no
  records-as-data (157), no metavariable solving (153).
- No new diagnostics beyond the one `Definition::Undeclared` needs.
