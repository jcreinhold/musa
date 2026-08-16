---
id: 141h
slug: track-and-machine-core
status: pending
depends_on: [141e, 141f, 141g, 141ga]
phase: 3
---

# Give the Track and the Machine a Core Shape

## Task

Prompt 141e registered 92 δ-builtins and 141f the two phase traversals, and left 26 rows of the two ownership tables
unregistered. Eight of them are the **track** builtins — `transpose`, `stretch`, `retrograde`, `invert`, `shift`,
`together`, `map_note_pitches`, `play` — and nine are the **machine** builtins — `primitive`, `machine`, `identity`,
`connect`, `beside`, `feedback`, `copy`, `drop`, `swap`. `crates/musa-compiler/src/registry/rules.rs` says why in one
sentence:

> a **track** or **machine** builtin needs `EventTrack` or `Machine`, and prompt 142 reshapes both when it deletes
> contextual `Music`. Registering them against the shape that is about to go would be work thrown away.

Do the reshaping, and register them. After this prompt the only rows left out of the core's registry are the eight
collection eliminators, which become library code, and the one phase projection, which is defined.

## Read

- `crates/musa-compiler/src/registry/rules.rs`'s `UNREGISTERED`, which names the four groups and their counts, and the
  accounting law in `registry/laws.rs` that counts each group again off the tables themselves. That law is what tells
  you this prompt is finished.
- `crates/musa-compiler/src/core.rs`'s `SYNTAX_OWNERSHIP` and `BUILTIN_OWNERSHIP` rows for the seventeen,
  `Family::Track` and `Family::Machine`, `MachineOp` and its `instantiate`, and `Eliminator` — the signatures as the old
  checker states them, which are what is being translated rather than redesigned.
- [`../../rules/language/02-core-calculus.md`](../../rules/language/02-core-calculus.md) §5.7 (track-construction
  safety) and §5.8's third and fourth families, which say what a track builtin and a machine builtin *are* and what
  obligation each carries. §5.7 is carried forward as an obligation to re-derive rather than an assumption to keep, and
  this is the prompt that re-derives it against the new core.
- [`../../rules/across-stages/03-machine-calculus.md`](../../rules/across-stages/03-machine-calculus.md) §2 — the nine
  admissible forms, which are a closed set: a tenth is a change to the calculus and not an addition to a library.
- [`../../rules/constitution.md`](../../rules/constitution.md) §4 and §7, on why a machine is a finite description and
  never the history it produces, and why the core is a calculus of occurrences of a canonical payload over exact
  rational time. `EventTrack[C, A]` is the first thing in this pass whose *payload* is a parameter.
- [`../clean-break-ledger.md`](../clean-break-ledger.md) §1 and §2 — the rows prompt 142 owns. This prompt gives
  `EventTrack[WrittenTime, ScoreFact]` a core spelling so that 142 can delete the type name `Music`; it does not delete
  anything itself.
- [`141b`](141b-base-types-and-builtins.md) and [`141e`](141e-compiler-registry.md) for how a base type is registered
  and how a signature is written in the core's terms, and [`141d`](141d-finite-constructor-builtins.md) for what a rule
  may read and write. The three decide most of what is left here: the question is which of §5.8's families each of the
  seventeen belongs to, and each row already says.
- [`141g`](141g-raw-lowering.md), which reads the surface and deliberately leaves `EventTrack` and `Machine` unspellable
  — because until this prompt they denote nothing.
- [`141ga`](141ga-quotation-core.md), which registered the `Template` base type and the two quotation rules and added
  the third count to `rules.rs`'s accounting. This prompt's `UNREGISTERED` arithmetic is stated against that table as
  141ga leaves it.

## Design

**Whether a track is a base type or a declared family is this prompt's one real decision, and it is decided by §5.8's
own test.** A base type is *inert*: it has no eliminator, no source program takes it apart, and two closed values of it
are convertible exactly when the host says the payloads agree. `musa-kernel`'s `EventTrack<C, A>` is normalized, has a
versioned exact identity, and is taken apart by nothing the source language can write — every operation over it is a
compiler-owned builtin. So it is a base type, indexed the way `Duration` and `Syntax` already are, and the payload
parameter is what makes the index two rather than one. A declared family would have to expose constructors that
`05-normalization.md`'s normal form does not admit, which is the opposite of what a track's identity is for.

The same test applied to `Machine[K, δ, δ]` gives the same answer for the same reason: §2's nine forms are its only
constructors, they are compiler-owned, and the tenth is a calculus change.

**`play` is where §5.7 is re-derived, not restated.** The obligation is that a constructed track is one the kernel would
have accepted, and under the old checker that was argued about a checked `Expr`. Under the new core it is a property of
a registered builtin's rule, which is where D2 already lives: `play` answers a track or it answers a refusal in its
result type, and a track it answers is normalized before it leaves the rule. State it as a law and cite §5.7's clause it
discharges.

**A machine builtin's type is written, not inferred.** `MachineOp::instantiate` builds a rank-1 scheme against the old
unifier, and this prompt writes the same signature as an ordinary Π in the core — `identity`'s polymorphism in its step
and its ports is ordinary quantification, and that is the whole reason 128 admitted a dependent core. `primitive` is the
exception the old code already names: its type comes from the build-local registry, so it is registered against what
that registry says and refused when it says nothing.

**Nothing is deleted and nothing is wired.** Contextual `Music`, `MusicOperation`, `MusicRole`, and the `close` and
`instantiate_music` path are prompt 142's rows on the ledger, and the old checker keeps using them until it goes. What
this prompt adds is a second, *unused* spelling of the same values in the core's terms — which is what 141e and 141f
also added, and is what makes the cutover a rewiring rather than a rewrite.

## Target

- `crates/musa-compiler/src/registry.rs`: `EventTrack` and `Machine` registered as indexed base types, with the payload
  and port indices their kinds require, doc-commented with the §5.8 test that decided base-type-over-family.
- The eight track builtins and nine machine builtins registered, with their rules translated from `eval_builtin`'s arms
  rather than rewritten — the arithmetic, the normalization, and the wording of every refusal are the ones that were
  there.
- `crates/musa-compiler/src/registry/rules.rs`: `REGISTERED` and `UNREGISTERED` updated, leaving `("structural
  eliminators", 8)` and `("phase projections", 1)`, with the doc saying what each of the two remaining groups is waiting
  for and which prompt owns it.
- Laws beside them: each of the seventeen reduces to the answer the old evaluator gives on the same arguments, sampled
  the way 141e's δ agreement law samples; §5.7's clause discharged as a law about `play`; and a machine whose step or
  ports are still open is a well-typed value that is simply not yet *a* machine.
- `docs/plan/code-map/` rows for `musa-compiler`.
- Prompt 142's Read and `depends_on` repaired to name this prompt.

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

Commit as `Give the track and the machine a core shape`.

## Stop

- No deletion. Contextual `Music` and every ledger row 142 owns survive this prompt; the old checker still runs the
  corpus.
- No collection eliminator registered. 141c argued that exclusion and it stands: they become library code in 142.
- No `.musa` file changes and no surface spelling for either type. 141g deliberately leaves both unspellable and 142 is
  where the source learns the words.
- No amendment to §5.7, §5.8, or `03-machine-calculus.md` §2. If a signature cannot be written in the core's terms, that
  is a finding and stop condition 4 — the same rule 141c and 141f worked under.
- No change to `musa-kernel`. It stays a leaf, and the track's core spelling is a base type registered by the compiler,
  not a type the kernel exports.
