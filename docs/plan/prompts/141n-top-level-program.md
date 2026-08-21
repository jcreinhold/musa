---
id: 141n
slug: top-level-program
status: done
depends_on: [135, 136a, 141g, 141i]
phase: 3
---

# Collect the Signatures Before the Bodies

## Task

`02-core-calculus.md` §2.4's last paragraph says what a document's declarations mean: "Top-level signatures are
collected before bodies are elaborated, so a later declaration may be referenced. From each body the checker records an
edge to every free named declaration. **Non-recursive** declarations must form an acyclic graph, exactly as before; a
*recursive* declaration is the case §2.4 admits, and it is admitted through the measure rather than through the graph."

`musa-calculus` cannot say any of it. [`musa_calculus::declare`](../../../crates/musa-calculus/src/lib.rs) takes a
mutually recursive *data* group and is the model for what is missing; there is no second door for definitions.
[`musa_calculus::check`](../../../crates/musa-calculus/src/lib.rs) takes one term. `Cx::define` pushes a binder that
[`Scope::new`](../../../crates/musa-calculus/src/scope.rs) names `None`, deliberately — α-equivalence is decided by
index — so nothing bound through it can be written by name. The only named binding the core offers is `RawShape::Let`,
and a `let` scopes forward only.

Give the core the program: one group of named definitions, every signature known before any body is elaborated.

## Read

- `../../rules/language/02-core-calculus.md` §2.4's last paragraph in full. It fixes three rules and this prompt
  implements exactly those three; anything else it seems to license is out of scope.
- `crates/musa-calculus/src/lib.rs`'s [`declare`](../../../crates/musa-calculus/src/lib.rs) — the door this one is
  modelled on, including its two doc-comment paragraphs. "One call declares *all* the families that may mention each
  other, because mutual recursion is not a relation between two finished declarations." The other half of a document is
  the same idea: a definition's body may name a definition that has not been elaborated yet, so neither is finished
  until both are. "The result is opaque on purpose. A caller brings the declaration into scope with `Cx::declaring`" —
  the same arrangement, one word over.
- `crates/musa-calculus/src/scope.rs`'s `Scope::new` and `Scope::lookup`, and `Cx::assume`'s doc comment on why a binder
  has no name. Together they are the argument against the obvious alternative: a named `Cx::define` would put names in
  the environment, and the environment is what α-equivalence is decided against.
- `crates/musa-calculus/src/rec.rs` in full. §2.4's measure is already built, and its shape decides this prompt's third
  rule: "a definition recurses on **one** argument, and the `match` at the top of its body is what says which". A
  hypothesis is minted by a `match` inside one body, so a *mutually* recursive pair has no hypothesis to become. The
  graph rule is therefore not a leftover — it is everything the measure does not reach.
- `crates/musa-compiler/src/core/mod.rs`'s `check_and_evaluate_metered`: `symbols` is built from every `raw` definition
  before the loop that checks bodies, `infer_open_declarations` runs between them, and `Code::DependencyCycle` — "these
  definitions call each other", with the cycle named — is the refusal at the end. That is the behaviour being replaced,
  and reading it is how to know the three rules are the three rules rather than a guess.
- Peyton Jones **ch. 6 §6.2.8** and **ch. 8**. §6.2.8 is dependency analysis: sort definitions "into minimal groups" and
  use `letrec` only "where it is actually necessary". Ch. 8 says when it has to run — "it is, however, important that
  the program is subjected to the dependency analysis referred to in Section 6.2.8 before type-checking", because a
  definition put in a `letrec` it does not belong in may fail to type-check at all. That is the derivation for computing
  the order rather than reading it off the document, and for computing it over the whole group rather than over the
  unannotated half: the analysis is what makes §2.4's first sentence true, because by the time a body is elaborated
  every definition it names has been through the checker already. Ch. 8's warning is against the *other* collapse —
  putting a definition into a recursive group it does not belong in — which is what "minimal groups" avoids and what
  rule 3 below is.
- [`136a`](136a-module-visibility.md)'s `Visibility`, `ModuleId`, and `Cx::in_module`. A `private fn` is filtered
  exactly as a private constructor is, and by the same mechanism; a second visibility rule for definitions would be the
  second path `02-core-calculus.md` §5's audit exists to catch.
- [`141g`](141g-raw-lowering.md)'s `Item::Definition` — an origin, a name, an optional type, and a value. That is
  already one member of this group, so this prompt adds no reading and 141g's "one direction" rule governs unchanged.
- [`141i`](141i-constrained-definitions.md), because a collected signature may be a `ConstrainedPi`: a `where` clause is
  part of what a forward reference has to see.
- [`142`](142-surface-cutover.md)'s Target — "the passes that call the whole of it" — which is what this prompt makes
  callable and does not itself call.

## Design

**The program is a group, and the group is the door.** `musa_calculus::declare_program` answers an opaque handle and
`Cx::defining` brings it into scope, exactly as `declare` and `Cx::declaring` do for families. That is not symmetry for
its own sake: it puts the definitions beside the declared groups in `Cx`, where `Scope::declared` already looks, rather
than in the binder environment, where a name would collide with the reason binders have none.

**Three rules, and each is one of §2.4's sentences.**

1. **A definition is elaborated after everything it names.** The order is computed from the group's edges rather than
   read off the document, so a body may name a declaration written later — which is why
   `examples/neo-riemannian.musa:72` may call the `compose_close` declared at `:151`. §2.4's "signatures are collected
   before bodies" is what this delivers: by the time a body is checked, every signature it can mention has already been
   elaborated.
2. **An unannotated definition has no signature to collect**, so it is inferred from its value — and the computed order
   is what makes that enough. It needs no restriction of its own: the analysis finishes it before anything that names
   it, exactly as it does an annotated one, so the two differ in how the type is *found* and not in where the name may
   be written. Imposing the restriction it looks like it should need — referable only from after itself — would refuse
   programs §2.4 admits, which is why signature collection and dependency ordering are one mechanism here and not two.
3. **Cycles.** A self-recursive definition goes to `rec.rs`'s measure, and must write its type, because the measure is
   checked against a type nothing else can supply. Anything else that closes a cycle is refused, with the cycle named,
   because the measure cannot reach it and taking it on trust is the one thing §2.4 forbids.

**A use is a reference, not a copy.** The term a use of a definition elaborates to is one node whose size does not
depend on the definition's body, and δ unfolds it during conversion. Inlining at the use site would type-check and would
make `stdlib/`'s two thousand definitions quadratic in the corpus, put the whole standard library inside every normal
form, and make every semantic hash a function of what a definition happens to be written as. Which encoding carries the
reference is the implementation's; that it is a reference is this prompt's.

**What a caller hands over is one group.** Every definition a document can see — its imports' and its own — in one call,
with each member's `ModuleId` saying where it came from. Ordering *libraries* is not this prompt's problem and not the
core's: `musa-compiler`'s `imports` already answers it, and a core that learned about import graphs would be a leaf that
learned what a file is.

## Target

- `RawProgram` and its member type in `crates/musa-calculus/src/raw.rs`; `musa_calculus::declare_program` and
  `Cx::defining` in the facade, each doc-commented with the rule it implements and the sentence of §2.4 it comes from.
- Signature collection by computed order: each definition's type is elaborated before its own body, and every definition
  it names before either. That is what §2.4's first sentence asks for and what makes a forward reference work.
- Dependency analysis over the whole group, ordering it and detecting cycles. One traversal, not two — the edges the
  order is computed from are the edges the cycle check reads.
- Self-recursion routed through `rec.rs` unchanged. No second measure, no second rewrite.
- Two refusals, each naming what a reader has to fix: a definition cycle, naming the cycle the way
  `Code::DependencyCycle` does today, and a definition that names itself without writing a type. Both with a code in
  `musa-compiler`'s `diagnose` and an arm in `lower/refusals.rs`, so the count in that module's doc comment moves with
  them.
- Visibility: a definition carries one, filtered by the mechanism 136a built.
- Laws in `crates/musa-calculus/tests/suite/`: a forward reference to an annotated definition checks; a forward
  reference to an unannotated one checks too, because the order is computed rather than read; a cycle of two is refused
  and the refusal names both; a self-recursive definition with a structural measure checks; one that names itself with
  no written type is refused; a private definition is invisible from another module; and the size law — a use's term
  size does not grow with the body it names.
- `docs/plan/code-map/spec-to-implementation-map.md` rows for `musa-calculus` and `musa-compiler`.

## Check

```sh
cargo build --workspace
cargo nextest run -p musa-calculus -p musa-compiler
cargo clippy --all-targets -p musa-calculus -p musa-compiler -- -D warnings
cargo fmt --check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
python3 scripts/renumber-prompts.py audit
```

Commit as `Collect the signatures before the bodies`.

## Stop

- **No wiring.** 142 owns the four passes, the readback, and the deletion of the old checker. This prompt is laws rather
  than a caller, for 141g's reason: an ordering inside one commit is not observable, and a checker replacement that also
  introduced the program would make a wrong program indistinguishable from a wrong migration.
- No module system, no import resolution, no library ordering. One group arrives; where its members came from is a
  `ModuleId` the caller supplies.
- No mutual recursion between definitions. §2.4 admits recursion "through the measure", `rec.rs`'s measure is
  per-definition, and widening it is a repair of §2.4 rather than an implementation choice.
- No change to `RawShape::Let`. A local binding is a term and stays one; this is a declaration.
- No new surface syntax and no change to 141g's reading.
- No performance work. 144 measures the finished checker.
