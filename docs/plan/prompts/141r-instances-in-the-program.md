---
id: 141r
slug: instances-in-the-program
status: done
depends_on: [137, 141n, 141o]
phase: 3
---

# Declare a Document's Instances With Its Definitions

## Task

[`document.rs::elaborate`](../../../crates/musa-compiler/src/document.rs) opens `musa-core`'s doors in this order:
families, then traits through `declare_trait`, then the definitions as one group through `musa_core::declare_program`,
then `Cx::defining`, and last a loop over the document's `impl`s calling `musa_core::declare_impl`.

Because that loop is last, **no definition in a document can resolve a method by receiver against an instance the same
document declares.** [`elab.rs::method`](../../../crates/musa-core/src/elab.rs) filters the traits declaring a spelling
by the ones with a dictionary at the receiver's head —

```rust
scope.discharged(&key).is_some() || classes.instance(&key).is_some()
```

— and while `declare_program` is running, `classes.instance(key)` is empty for every `impl` in the document. The refusal
is `Refusal::NoMethodForType`, at the call, naming a type the reader can plainly see has an instance twenty lines above.

The reproduction: `stdlib/src/pitch.musa` declares `trait Transposable<A> { fn transposed(subject: A, by: Interval) ->
A; }` with an `impl` for `Pitch` and one for `NoteName`. `stdlib/src/transformational.musa` writes `triad_root(refined)
up M3`, which [`lower/values.rs::transposition`](../../../crates/musa-compiler/src/lower/values.rs) lowers to a
`Raw::method(.., "transposed")`. `cargo nextest run -p musa-compiler lower::piece::laws::every_example_elaborates`
answers ``NoMethodForType: no method `transposed` for `PitchClass` ``.

This is not a missing feature of `10-traits.md` §6 — §6's rule is already the right rule, and it is already implemented.
It is a **declaration-order** mistake, of exactly the kind [`141n`](141n-top-level-program.md) removed from definitions
and did not remove from instances. Fix it the way 141n fixed the first half: one dependency graph, not two passes.

The order cannot simply be swapped. An `impl`'s method bodies are ordinary terms that may name any definition in the
document, so instances-first makes every `impl` blind to the definitions it calls, which is the same bug pointing the
other way. Neither kind comes first, so neither is declared first.

## Read

- [`docs/rules/language/02-core-calculus.md`](../../rules/language/02-core-calculus.md) §2.4's last paragraph, which is
  the rule being widened by one word: "Top-level signatures are collected before bodies are elaborated, so a later
  declaration may be referenced. From each body the checker records an edge to every free named declaration.
  **Non-recursive** declarations must form an acyclic graph." A document's `impl`s are declarations with bodies and the
  paragraph never excluded them; only the implementation did.
- [`docs/rules/language/10-traits.md`](../../rules/language/10-traits.md) §6, "Method syntax resolves by exact
  receiver", and §2's coherence statement — in particular that instance choice "does not depend on the order in which
  the elaborator reached its constraints … or on where in the program the constraint was discharged". A resolution that
  depends on whether the `impl` happened to be declared yet is that dependence, wearing a different hat. §3's orphan
  rule is what bounds the set of instances a document has to consider at all, and it is unchanged here.
- [`141n`](141n-top-level-program.md) — the whole prompt, and its Design's three rules. This one adds a fourth kind of
  member to the group it built and does not touch the three rules.
- [`141o`](141o-document-elaboration.md)'s Design, "The order the four doors open in is forced", and the module
  documentation it produced. That order is where the mistake is written down, so it is what this prompt rewrites: three
  doors, not four, with the third taking both kinds.
- [`137`](137-traits-and-dictionaries.md), for what `declare_impl` does and in what order — coherence at the key, the
  orphan check, the `where` clause, and only then the dictionary value from the method bodies. This prompt moves *when*
  that runs and changes nothing *in* it.
- [`143`](143-builtin-collapse.md), which is the prompt that meets this at scale: it moves `nat_add`, `text_equal`, and
  `duration_of` behind traits whose `impl` bodies call library functions, in the same documents as the definitions that
  call those traits' methods. Every one of those is the shape above.
- [`program.rs`](../../../crates/musa-core/src/program.rs)'s `free`, and its doc comment's warning — "a spurious edge is
  a spurious *cycle*, and a cycle is refused". That sentence is the whole difficulty of this prompt, because the
  definition-to-instance direction is the one direction that cannot be computed exactly.
- Peyton Jones **ch. 6 §6.2.8** and **ch. 8**, cited by 141n for the first half and load-bearing again here. §6.2.8's
  "minimal groups" is what the Design's two edge kinds protect: an analysis that puts a declaration into a recursive
  group it does not belong in may fail to type-check something that is perfectly well typed, and an edge that stands for
  "one of these" rather than "this one" is the standard way to manufacture that mistake.

## Design

**The instances are members of the program.** `RawProgram` grows an `instances: Vec<RawImpl>` beside its `definitions`,
`declare_program` takes both, and what comes back holds both — so `Definitions` becomes `Program`, and `Cx::defining`
brings the definitions into scope as global names *and* the instances into `Classes` in one call. `declare_impl` stays
public, for the caller that declares an instance against a context which is already finished; it stops being the door a
document walks through.

That is the whole of the interface change, and it is 141n's own argument one word over: the group is the unit because
`02-core-calculus.md` §2.4's forward reference is a property of the group rather than of a written order, and handing a
caller two lists to re-combine would be handing back the question the module exists to answer.

**Two kinds of edge, and only one of them can be a cycle.**

1. **A name edge is hard.** A declaration of either kind edges to every *definition* in the group whose name it writes
   free — read from an `impl`'s parameters, arguments, `where` clause, and method bodies exactly as it is read from a
   definition's type and value, under the same binder stack. These are 141n's edges, they mean "this cannot be
   elaborated before that one", and a cycle in them is `Refusal::DefinitionCycle`, unchanged. `recursive` is computed
   from these and from nothing else.

2. **A method edge is soft.** A declaration that writes a `RawShape::Method` spelled `m` edges to every instance in the
   group of every trait that `Classes::declaring_method` says declares `m`. This edge means something weaker than the
   first: not "before that one" but "before *one of* these", because which one it is depends on the receiver's type, and
   the receiver's type is not known until the declaration this edge belongs to has been elaborated.

**A soft edge orders and never refuses.** In the traversal, a back edge that is soft is *dropped*. A back edge that is
hard closes a cycle, and that cycle is `DefinitionCycle` only when it is hard the whole way round; if any edge inside it
was soft, the *deepest* such edge is dropped instead and the walk continues from there. The refusal belongs to the
cycle, not to the edge that happened to close it, and a cycle holding a soft edge is a cycle the graph guessed at — so
the guess is what gives way. This is the whole answer to the over-approximation, and it is what keeps

```musa
impl Eq<Pitch>    { fn equal(a, b) { a.name == b.name } }
impl Eq<NoteName> { fn equal(a, b) { a.letter == b.letter } }
```

— two instances, each writing the trait's own spelling, neither needing the other's dictionary in both directions — from
becoming a refused cycle between declarations that never named each other. Treated as hard, those two edges close a loop
and the document is rejected; treated as soft, the depth-first walk emits `Eq<Letter>`, then `Eq<NoteName>`, then
`Eq<Pitch>`, which is the order a reader would have picked.

What a dropped edge costs is stated rather than hidden: the declaration is elaborated with that instance not yet in
`Classes`, and if it really did call that instance's method, the call refuses with `NoMethodForType` **at the call**,
naming the method and the head type. That is a sentence about the program, and it is what a genuine mutual dependency
between a definition and a dictionary deserves — an instance is never `rec`, so a cycle through one has nowhere to go
even if the graph admitted it. What must not happen is the same program refused as a cycle among declarations that do
not name each other, which is what the naive edge produces and what this rule is for.

The two rules meet exactly where the graph's own shape puts them, which is why the "deepest soft edge" clause needs no
case analysis: a hard edge only ever points at a *definition* and a soft edge only ever points at an *instance*, so
every cycle that passes through an instance holds a soft edge and breaks at one, and a cycle among definitions alone
holds none and is refused by name. `DefinitionCycle` stays what 141n made it — a statement about definitions naming each
other — and no instance can be caught in one.

**Nothing about resolution changes.** §6 already resolves `x.m(y)` by the head of `x`'s concrete type against the
instances in scope, and that code is not touched. This prompt changes *which instances are in scope when a definition is
elaborated*, and it changes it for exactly one reason: they were declared by then in the document the author wrote, and
the implementation was declaring them later.

**Written order in, written order out.** The order the analysis finds is a fact about the call, not about the group, so
each kind comes back in the order the document wrote it — the positions travel with the nodes and the two lists are
re-sorted before the `Program` is built.

### Where the evidence is

This prompt is split out of [`142`](142-surface-cutover.md) *while 142 is being implemented*, which is exactly
[`141q`](141q-canonical-readback.md)'s situation and has the same consequence: the reproduction above is not
reproducible from `main`. `stdlib/src/pitch.musa` has no `Transposable` trait at `HEAD` — the trait, its two instances,
and the `import std::pitch` in `stdlib/src/transformational.musa` are all 142's working tree, and they are what turned a
latent ordering bug into a failing law.

142's tree also carries a **first cut** at this change, in which the method edge is hard. It resolves the reproduction
and it is not what this prompt asks for: with a hard method edge the two-`impl` shape above is a refused cycle, and 143
writes that shape in every document it touches. The soft rule above is the repair, and it is the reason this is a prompt
rather than a hunk inside a migration diff — a change to what the language accepts does not belong in a commit that also
moves eleven thousand lines of `.musa`, for the reason 142's own Design gives about 141e.

## Target

- `RawProgram::instances`, doc-commented with why the two kinds travel together: each can name the other and neither
  comes first.
- `Definitions` renamed to `Program`, holding both kinds, with `instances()` beside `members()`; `Cx::defining` taking
  it and extending both the definition scope and `Classes` in one call; `musa_core::declare_program` re-documented for
  what it now takes and what it now refuses.
- One dependency graph over both kinds in one index space, with the two edge kinds above. An `impl`'s free names read
  from its parameters, its arguments, its `where` clause, and its method bodies.
- `ordering` distinguishing the two: a soft back edge is dropped, a hard back edge is `Refusal::DefinitionCycle` with
  the cycle named *unless* the cycle it closes holds a soft edge, in which case that edge is dropped instead; a
  self-edge is 141n's recursive case for a definition and dropped for an instance.
- `document.rs`'s `elaborate` losing its `declare_impl` loop, and its module documentation rewritten to **three** doors
  in a forced order, with the third taking definitions and instances as one group and saying why.
- Laws in [`crates/musa-core/tests/suite/program_laws.rs`](../../../crates/musa-core/tests/suite/program_laws.rs):
  - a definition written *before* an `impl` may call its method by receiver;
  - an `impl`'s method body may name a definition written *after* it;
  - two `impl`s of one trait whose bodies both write that trait's method spelling both elaborate, and the one whose
    method the other actually calls is elaborated first — the soft edge orders and does not refuse;
  - a definition that names an `impl`'s method while that `impl`'s body names the definition is refused **at the call**
    with `NoMethodForType`, and not as a cycle;
  - two definitions that name each other are still `DefinitionCycle`, naming both, unchanged;
  - both kinds come back in the order the document wrote them, whatever order they were elaborated in.
- Rows in [`docs/plan/code-map/spec-to-implementation-map.md`](../code-map/spec-to-implementation-map.md): the
  `musa-core` row for §2.4's declaration group, which currently describes definitions only, and the `musa-compiler` row
  for `document.rs`, which currently says "instances last, because an `impl`'s method bodies are ordinary terms that may
  call any definition" — the half-truth this prompt completes.

### What lands with 142 instead

No law in `musa-compiler` and no change to `stdlib/`. The corpus demonstration — a document whose definitions call the
traits it declares, elaborated end to end — is 142's, for 141q's reason: `document/laws.rs` already holds 142's
rewritten fault surveys, and the trait this reproduction needs is 142's tree. Splitting one file across two commits to
keep a bullet true is not a thing to do to a bullet.

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

Commit as `Declare a document's instances with its definitions`.

## Stop

- **No change to `elab.rs::method`.** §6's resolution rule, its two refusals, and the `discharged`-or-`instance` filter
  stay exactly as they are. This prompt changes when an instance is in scope, not how one is found.
- **No split of `declare_impl` into a header pass and a body pass.** A separately declared key with a lazily filled
  dictionary would buy an exact definition-to-instance edge and cost a mutable cell in a value the whole crate treats as
  finished. If the soft edge ever proves too coarse, that is the repair to consider then, with a corpus that shows it.
- **No new refusal.** A dropped soft edge reports as the call site's `NoMethodForType`, which already names the method
  and the head. A second diagnostic for "the instance existed but not yet" would be the second path
  `02-core-calculus.md` §5's audit exists to catch.
- **No cross-*document* ordering.** Which library is elaborated before which is `crate::imports`', and a core that
  learned about import graphs would be a leaf that learned what a file is.
- **No change to coherence, the orphan rule, or `Storable`.** The generated instances arrive with the registry and are
  not members of any document's program.
- **No corpus work.** No trait added to a library, no `.musa` spelling changed, no example migrated. 142 owns the
  migration and 143 owns the traits that make this urgent.
