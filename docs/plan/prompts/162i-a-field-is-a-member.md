---
id: 162i
slug: a-field-is-a-member
status: done
depends_on: [157, 161, 162b]
phase: 3
---

# A Field Is a Member, So Calling One Should Not Cost a Pair of Parentheses

## Task

`interval_group.compose(M3, m3)` is refused with "no method `compose` for `Group`", and the author must write
`(interval_group.compose)(M3, m3)`. `interval_group` is a `Group<Interval>` and `Group` is a `record` whose `compose`
field has type `G -> G -> G`, so the field is there, the call is well typed, and only the spelling is wrong. The cause
is that `.` means two different things depending on whether a `(` follows it: `x.f` lowers to `RawShape::Project` and
`x.f(a)` lowers to `RawShape::Method`, and the method rule looks only in `Head`'s namespace. Make `x.f(…)` resolve over
**one candidate set drawn from two tables** — the definition `Head.f`, and the field `f` of the receiver's type where
that type is a one-constructor family — with none and two both refused, and amend `01-surface.md` §1.5, which today
claims there is "one candidate by construction".

This is the missing half of prompt [162b](162b-parameterized-record-literals.md). That prompt made `Group<Interval>`
constructible; this one makes a constructed one callable. Prompt 146 deleted traits on the argument that "a structure
becomes a record… strictly more than the trait had", and a structure whose every use costs a pair of parentheses that
reads like a mistake is not strictly more than anything.

## Read

- `docs/rules/language/01-surface.md` §1.5, in full — "`x.m(y)` resolves **by exact receiver and in one step**… There is
  one candidate by construction — the head names the namespace, the namespace and the member spell one name, and a name
  resolves to one definition — so the answer is that definition or the failure." That sentence and its three
  consequences are what this prompt amends, and the Design says which clauses survive.
- `docs/rules/language/01-surface.md` §1 — the grammar's `projection := expr "." IDENT` and
  `method-call := expr "." IDENT "(" args? ")"`, two productions that differ by a paren, and §1.2's "**Projection is the
  only way to read a field**", which stays true and is the reason the fallback is not a new form.
- `docs/rules/language/01-surface.md` §1.4 — "`trait Group<G>` becomes `record Group(G : Type) { … }`, and an instance
  becomes an ordinary value. This is strictly more than the trait had". The claim this prompt makes good on.
- `crates/musa-calculus/src/elaboration/elab/infer.rs`'s `method`, `namespaced`, and `receiving` — the three-step
  lookup, and `namespaced`'s doc comment in particular: "A rule that needed the elaborator would need metas, and a meta
  here would be the trial elaboration this design does not have." That sentence decides the shape of the new rule.
- `crates/musa-calculus/src/elaboration/elab/record.rs`'s `projection` and `product` — the term a field reading must
  produce, and the one place in the crate that answers "is this a record".
- `crates/musa-calculus/src/kernel/family/group.rs`'s `Product` — `fields: Arc<[Parameter]>`, in declaration order.
- `crates/musa-compiler/src/lower/values.rs`'s `name`, `application`, `method_call`, and `operator` — where the paren
  decides which raw shape is built, and where `x ⊕ y` is made "the same term" as `x.equal(y)`.
- `crates/musa-compiler/src/lower/items.rs`'s `constructor` — positional fields are named `_0`, `_1`, … and §1's
  identifiers do not start with `_`, which is why this prompt needs no rule about them.
- `stdlib/src/algebra.musa` and `stdlib/src/pitch.musa:120` — the three structures and the three values, and the comment
  that already asserts what this prompt implements: "`P5.compose(M3)` and `interval_group.compose` are one definition
  rather than two spellings of one operation."
- Prompt [163](163-laws-as-record-fields.md) and [164](164-builtin-collapse.md) — the first two consumers. 163 adds law
  fields to the three structures and 164 builds the instances at declared carriers, and both are written on the
  assumption that a structure can be *used*.

## Design

**One rule, one candidate set, two tables.** `x.f(…)` where `x : T` and `Head` is the rigid head of `T` collects:

- the definition `Head.f`, where one is in scope and visible; and
- the field `f`, where `T` is a one-constructor family whose constructor names a field `f`.

Exactly one candidate is the reading. **None** is the refusal it is today, improved. **Two** is a new refusal naming
both and the two spellings that pick them apart. Both tables are consulted at every call, so the answer does not depend
on which is read first and there is no order to reason about.

**The field reading is the projection rule and not a copy of it.** `x.f(a)` with `f` a field elaborates to exactly the
term `(x.f)(a)` elaborates to: the family's generated accessor at the parameters `T` stands at, applied to `x`, applied
to `a`. `RawShape::Method` holds no arguments — `x.f(a, b)` is that shape applied through the ordinary `App` rule — so
this is a change to what `Method` *infers to* and to nothing else. Factor the accessor half of `record.rs::projection`
into a function taking an already-inferred receiver, and call it from both rules; that is what makes "one term" true by
construction rather than by a test that could drift.

**A method reading takes the receiver as its first argument and a field reading does not.** `Head.compose : Group<G> ->
G -> G -> G` and `compose : G -> G -> G` are different types at different arities, which is why the two readings are
*kept apart in the term* even though they are joined in the spelling. Nothing coerces between them and nothing tries
both.

**Presence decides, and type never does. This is not §1.5's type-directed disambiguation and must not be built as one.**
That mechanism picks among several *definitions of one name in scope* using an expected type already in hand: the
candidates are interchangeable in kind, and the type is a filter over them. Here the two candidates are not two
definitions of one name — they are two forms, with different terms, different arities, and different first arguments —
so filtering them by type would mean elaborating both and keeping whichever converts. That is the trial elaboration
`infer.rs::namespaced` refuses by name, it would make elaboration depend on the order constraints are reached, and it is
the search §1.5 exists to say the language does not do. So the collision is refused rather than typed apart, and the
refusal is the price of not having a search.

**Refusing the collision is what keeps a package addition from changing a meaning.** §1.5's first consequence — "adding
a definition to a package would change what existing code means" — is the property at stake, and a plain fallback
ordered namespace-first would give it up: writing `impl Group { fn compose(…) }` in a later release would quietly
recapture every `g.compose(a, b)` that had been reading the field. Under the refusal it breaks loudly at every such site
and names both spellings. This is the same trade §1.5 already takes for the bare-member rule, where "two is an error
naming both", so the language gains no new *kind* of rule — it applies an existing one to a second table.

**No accepted program changes meaning, and the one class that could is checked.** Every program the fallback newly
accepts is refused today, so acceptance only widens. The exception is the collision: a receiver type that today has both
a field `f` and a namespaced `Head.f` reads as the method and would become a refusal. Sweep `stdlib/` and `examples/`
for one, report the count in the commit message, and if the count is not zero, repair this prompt before implementing —
a rule that breaks committed code needs an argument this Design does not make.

**Operators go through it, because they already go through `.`.** `x + y` lowers to `Raw::method(x, "add")` applied to
`y` precisely so that `x == y` and `x.equal(y)` are one term, and that stays true: a record with a field `add` is what
`+` finds at that receiver, and one with both a field `add` and an `impl` `add` refuses at the operator with the
operator's own span. Restricting the fallback to written method calls would make `x ⊕ y` and `x.m(y)` two rules again,
which is the thing `values.rs::operator`'s doc comment exists to prevent.

**Deciding by the field's type is refused for the same reason as deciding by the goal.** A field `n: Nat` written
`p.n(3)` reads as the field and then fails as `NotAFunction` at the application, where the mistake is. Firing the
fallback only for fields whose type is a Π would be type-directed selection wearing a smaller hat, and it would make
`p.n(3)`'s diagnostic report a missing method rather than an applied natural.

**A receiver with no rigid head acquires no field either.** `MethodOnVariable` is unchanged: a generic `A` names no
family, so it has no field table any more than it has a namespace, and the repair is still the written path.

**The none-case diagnostic says what is actually there.** `NoMethodForType` carries the receiver type's field names
where the receiver is a product, and the lowered diagnostic lists them: "`Group` has fields `unit`, `compose`,
`inverse`". This is what makes the miss legible after the change as well as before it — and it is the whole of the
alternative repair this prompt rejected, which was to keep the refusal and teach it to say "write `(x.compose)(…)`".
That repair is not enough, because the thing it explains is not a rule the language means: `x.compose` is already a
projection, and `.` changing what it denotes when a `(` follows is the defect rather than the design.

## Target

- `crates/musa-calculus/src/elaboration/elab/infer.rs`'s `method` resolves over both tables; the field reading is the
  accessor term factored out of `record.rs::projection` and shared by both rules.
- `Refusal::NoMethodForType` carries the receiver's field names where its type is a product;
  `crates/musa-compiler/src/lower/refusals.rs` lists them in the diagnostic's help.
- One new refusal for the collision, with one new `musa_score::diagnose::Code` and its kebab-case spelling, naming the
  member, the head, and both repairs — `Head::f(x, …)` and `(x.f)(…)`.
- `docs/rules/language/01-surface.md` §1.5 amended: the one-candidate-by-construction sentence is replaced by the
  two-table rule; the three consequences are restated with the second one — no fallback to a free function whose first
  parameter happens to fit — kept verbatim, because a field of the receiver's *own type* is not that; and one paragraph
  states why presence and not type decides, referring to the disambiguation rule directly above it.
- `docs/plan/code-map/spec-to-implementation-map.md`'s "Method resolution" section: its "Three refusals" line becomes
  four, and one sentence records the second table.
- `examples/pitch-algebra.musa` gains the structure-passing half of what it already demonstrates: a function taking a
  `Group<Interval>` and calling `g.compose(x, x)`, applied to `interval_group`, beside the `P5.compose(P8)` that is
  already there — so the file exhibits both spellings of one definition, which is what its own comment and
  `stdlib/src/pitch.musa:108` already claim.
- Laws, in `crates/musa-calculus`:
    - `x.f(a)` and `(x.f)(a)` are the same term where `f` is a field and no `Head.f` exists;
    - `x.f(a)` is `Head.f(x, a)` where `Head.f` exists and `f` is not a field — the existing law, unchanged;
    - both existing is refused, and the refusal names both;
    - neither existing is refused, and the refusal names the head, the member, and the fields;
    - a non-function field applied is `NotAFunction` at the application;
    - a receiver at a type variable is still `MethodOnVariable`, field table or no;
    - `x == y` and `x.equal(y)` remain one term at a receiver whose type has an `equal` field.
- The collision sweep over `stdlib/` and `examples/`, with its count in the commit message.

## Check

```sh
cargo build --workspace
cargo nextest run -p musa-calculus -p musa-compiler -p musa-score
cargo nextest run --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo insta test -p musa-compiler --unreferenced=reject
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
```

```sh
cargo run -q -p musa -- check examples/pitch-algebra.musa
```

```sh
printf 'piece "M" {\n  import std::algebra;\n  import std::pitch;\n  fn twice(g: Group<Interval>, x: Interval) -> Interval { g.compose(x, x) }\n  let two_octaves: Interval = twice(interval_group, P8);\n}\n' > /tmp/field-call.musa
cargo run -q -p musa -- check /tmp/field-call.musa
```

`cargo nextest run --workspace` and `cargo insta test --workspace` carry whatever reds the prompts already in flight
own; a *new* red under either is this prompt's.

Commit as `Let a record's field be called like a member`.

## Stop

- **No mirror rule for the unapplied direction.** `x.m` where `m` is a namespaced definition and not a field stays the
  refusal it is. A projection's receiver is not an argument, so `x.m` as a partial application would need §1.3's
  completeness rule to say what an unapplied call means, and §1.5 already gives `Head::m(x, _)` as the spelling. That is
  its own decision and needs its own prompt.
- No grammar change, no lexer change, no `editors/tree-sitter-musa` change. `x.f(a)` already parses; what it *means* is
  the whole of this prompt.
- No auto-deref, no receiver coercion, no fallback to a free function whose first parameter fits, and no third table.
  §1.5's second consequence stands unamended.
- No type-directed choice between the two readings, and no trial elaboration. If the implementation reaches for a meta
  here, the design is wrong and the prompt needs repair.
- No new core term and no new raw shape. `RawShape::Method` infers to a different term in one case; `RawShape::Project`
  is untouched.
- No sweep of `stdlib/` into the new spelling. One example is the evidence; the adapters are prompts
  [166](166-staff-rewrite.md) and [167](167-studio-rewrite.md)'s, and the structures' own consumers are
  [163](163-laws-as-record-fields.md)'s and [164](164-builtin-collapse.md)'s.
- No change to what the resolver records for an editor. Which table a `.f` came from is an LSP question and belongs with
  the language server's own prompts.
