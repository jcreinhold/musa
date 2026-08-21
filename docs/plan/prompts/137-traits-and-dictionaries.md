---
id: 137
slug: traits-and-dictionaries
status: done
depends_on: [136]
phase: 3
---

# Add Traits, Dictionaries, and Coherence

## Task

Implement the trait *semantics* prompt 130 specified, in `musa-calculus` and nowhere else: `trait` and `impl`
declarations elaborated to dependent records and their values, the instance table, coherence, the orphan rule and the
termination measure — all three checked at the declaration — `where` constraints, constraint resolution with
local-beats-global and postponement, and the generated `Storable` instances that retire the `d` type-variable class.
Prompt 137a spells all of this on the surface; this prompt is the mechanism it spells.

## Read

- `docs/rules/language/10-traits.md` §1–§4 and §9 — what a trait and an instance *are*, coherence, the orphan rule,
  instance lookup and its termination measure, local-beats-global, and the refused list. §5 (operators) and §6 (methods)
  are 137a's, and this prompt implements neither.
- Prompt 132's trial, for what it found about coherence in practice. A trait design that was never written against a
  program is a design; that note is the evidence.
- `docs/rules/language/02-core-calculus.md` §1.1 as rewritten by 129 — the `Storable` constraint, and the rule that its
  instances are generated from the declaration group and never written by hand. That is the single most important
  coherence property in the system: a user-writable `Storable` instance is a hole in the kernel payload boundary.
- `crates/musa-calculus/src/{family.rs, declare.rs}` — the precedent this follows exactly. A `data` declaration is
  elaborated in `cx.closed()`, its telescope is read binder by binder in the scope the previous binders built, and the
  group it produces is carried on a `Cx`. A trait is the same shape with a record where the constructors were, and
  reusing that shape rather than inventing a second one is what keeps `Cx` a single idea.
- `crates/musa-calculus/src/{elab.rs, unify.rs, meta.rs}` — `fresh_meta`, the pattern-fragment unifier, and `settled`.
  Postponing a constraint whose head is unknown is *not* new machinery: the hole is a metavariable, solving the
  constraint solves it, and `zonk` substitutes it away. Recovering an instance's parameters from a lookup is
  unification, not a hand-written matcher.
- `crates/musa-calculus/src/rec.rs`'s `field_type`, and `elab.rs`'s `projection` and `update` — how a telescope's field
  type is read at a subject. A dictionary is a record, so every question about reaching into one is already answered
  there.
- `crates/musa-compiler/src/infer.rs`'s `d`-variable handling — the machinery being replaced, and the failure messages
  it produces today, which the constraint form must match or beat.
- *A Philosophy of Software Design* ch. 7 — why this prompt stops at `musa-calculus`. A trait's semantics and a trait's
  spelling change for different reasons and are checkable by different tests; a single prompt covering both would be one
  abstraction spread over two layers, and neither half could be committed green on its own.

## Design

**Dictionaries are records, and that is the whole implementation.** A trait elaborates to a dependent record type; an
impl elaborates to a value of it; a constrained function takes it as an extra argument; a use site is resolved by lookup
and applied. Because 133 made records primitive with η, two dictionaries for the same instance are convertible without a
projection chain, which is what makes coherence checkable rather than merely asserted. Concretely, `trait Eq<A> { fn
equal(x: A, y: A) -> Bool; }` elaborates to the closed term `λ(A : Type ℓ). { equal : A → A → Bool }`, so `Eq τ` is an
*application* that β-reduces to the record type by the evaluator that was already there. No new `Term` shape, no new
conversion rule, no unfolding step written for traits.

**Every telescope is elaborated in the scope it will be read in.** This crate has no substitution and no shift on syntax
by design (`lib.rs`: "Reduction is never performed on syntax"), so a field's type is elaborated in a scope that already
assumes the fields before it, and its de Bruijn indices come out right because nothing moved them. This is why the
super-constraint fields come first: their types are terms this module assembles, so they must be assembled at a depth
known before any method is read. Where a type built under one scope has to be read under a deeper one, it crosses as a
**value** — values are de Bruijn *levels* and need no shifting — never as a re-indexed term.

**Resolution is a lookup, and the code must be structured so that it cannot become a search.** One table keyed by
(trait, head constructor), with `Eq`/`Hash` on the key and a `HashMap` behind it: no candidate list to filter, no
ordering to break a tie with, nothing to retry, and `None` is the refusal. A second impl for the same key is an error at
the *declaration*, naming both. No orphans: an impl lives with its trait or with its head type, checked at the package
boundary. No defaulting. Instance heads carry a decreasing measure — smaller than the head, and no type variable
occurring more often than it does in the head — checked at the declaration rather than at the use, because an error at
the use site blames the wrong author.

**Recovering an instance's parameters is unification, not matching.** A lookup finds `impl<T> Eq<List<T>>` for the key
`(Eq, List)`; the `T` is then recovered by making a metavariable per instance parameter, evaluating the instance's
written arguments in an environment of those metas, and unifying against the arguments actually asked for. The pattern
fragment 134 built is exactly strong enough, and writing a first-order matcher beside it would be a second unifier free
to disagree with the first.

**A postponed constraint is a metavariable.** §4 postpones a constraint whose head is not yet known, and the hole it
leaves in the output term is what a metavariable already is: `Eq.equal` elaborates to `?d.equal` with `?d : Eq ?A`
recorded pending, solving the constraint solves `?d`, and `zonk` substitutes it away like any other. A constraint still
blocked when the declaration ends is reported by the machinery that already reports an unsolved hole, so `Head` has
deliberately only two arms — rigid and local — and a metavariable head is not representable.

**Local beats global.** A dictionary bound by an enclosing `where` is preferred to a global instance for the same head.
This is what makes a generic function's behaviour a consequence of its own signature, and it is a one-line rule with a
large payoff in comprehensibility. A local head is a type *variable*, which no global instance can ever be declared for
— so the two lookups can never disagree under coherence, and what the rule buys is determinacy: instantiating a
parameter later cannot reroute a call that was already elaborated.

**A super-constraint is resolved where the impl is declared, not where the trait is used.** `impl<T> Ord<List<T>> where
Ord<T>` must produce the `Eq (List T)` field that `trait Ord<A> where Eq<A>` declares, and that field is filled by
*resolving* `Eq (List T)` — against the local dictionaries the impl's own `where` bound and then against the table. This
is the finding that fixes this prompt's boundary: declaration and resolution are one mechanism and cannot be two
commits, while surface spelling is genuinely separable and is 137a.

**`Storable` retires the `d` variable class.** Instances are generated from the declaration group by the same structural
check §1.1 already specifies — no source `impl Storable` is accepted, ever, and that refusal has its own test, behind
every spelling including a trait the author names `Storable` themselves. What changes for an author is that the
constraint appears in signatures they read, and that its failures are ordinary instance errors naming the field that is
not storable.

**`Eq` and `Id` are different words for different things, in the error messages too.** `=` is 129's identity type and is
introduced by `refl`; `Eq` is a trait with a method. A message that says "equality" without saying which one is a
message that will be misread, so the diagnostics name the trait or the type.

**Laws.** Coherence: at most one instance per key, tested by the rejection naming both declarations. Orphan refusal.
Dictionary elaboration is sound — a resolved use is convertible to the dictionary's field, and the re-checker accepts
every term this produces. Local beats global, tested by a program where they differ. Instance matching terminates on the
pathological head (`impl<A> C<F<A>> where D<G<A, A>>`). Resolution on an unconstrained type variable refuses and says to
write the constraint. `Storable` cannot be implemented by hand. The negative suite carries one case per refusal this
prompt adds.

## Target

- `crates/musa-calculus/src/class.rs`: `PackageId`, `Head`, `Key`, `Constraint`, `Trait`, `Instance`, `Classes`, and
  §4's size and occurrence measures.
- `crates/musa-calculus/src/dictionary.rs`: trait and impl elaboration, the declaration-time checks, constraint
  resolution with local-beats-global and postponement, and the generated `Storable` instances.
- `Cx` carries a `PackageId` and the `Classes` in scope, the way it already carries a `ModuleId` and its declared
  groups; `Raw` gains `RawTrait`, `RawImpl`, `RawConstraint`, and the method and `where` forms they need.
- The `Refusal` variants these checks report, and their `musa explain` codes in `crates/musa-score/src/diagnose.rs`:
  duplicate instance, orphan impl, unresolved instance, non-terminating instance head, and hand-written `Storable`,
  beside the declaration-shape refusals (reserved or headless class, duplicate or missing or unknown method, a replaced
  derived method, wrong class arity, a blanket instance).
- `crates/musa-calculus/tests/suite/{trait_laws.rs, coherence_laws.rs}` plus the compile-fail cases.
- `docs/plan/code-map/` rows for `musa-calculus`.

## Check

```sh
cargo build --workspace
cargo nextest run -p musa-calculus -p musa-compiler
cargo nextest run --workspace
cargo clippy --all-targets -p musa-calculus -p musa-compiler -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
```

Commit as `Add traits, dictionaries, and coherence`.

## Stop

- **No `musa-language` change.** No `trait`, `impl`, or `where` keyword, no grammar, no CST, no formatter, no
  tree-sitter. `musa-calculus` takes a `RawTrait` the way it takes every other `Raw`, and a test builds one directly —
  which is how every prompt since 133 has tested this crate.
- No operators, no method-call syntax, no `Type::item` paths, and no exact-receiver resolution. 137a.
- No `docs/rules/style-guide.md` rule and no `lint.rs` diagnostic. Their subject is spelling, and nothing is spellable
  yet.
- No instance search, no overlapping instances, no specialization, no defaulting, no return-type-directed overloading,
  no auto-deref, no blanket impls, no functional dependencies, and no associated-type resolution beyond what
  `10-traits.md` states. Every one of these turns the lookup into a search.
- No user-written `Storable` instance, behind any spelling.
- No deletion from `BUILTIN_OWNERSHIP`. Prompt 143 collapses the registry, after there is something to collapse it into.
- No `musa-compiler` checker wire-up beyond the `Code` table, and no `stdlib/` migration. Prompt 142.
- No `Syntax<Cat>`, no quotation, no collection library.
