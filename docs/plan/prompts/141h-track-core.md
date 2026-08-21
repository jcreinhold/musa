---
id: 141h
slug: track-core
status: done
depends_on: [141e, 141f, 141g, 141ga]
phase: 3
---

# Give the Track a Core Shape

## Task

Prompt 141e registered 92 δ-builtins and 141f the two phase traversals, and left 26 rows of the two ownership tables
unregistered. Eight of them are the **track** builtins — `transpose`, `stretch`, `retrograde`, `invert`, `shift`,
`together`, `map_note_pitches`, `play` — and `crates/musa-compiler/src/registry/rules.rs` says why in one sentence:

> a **track** or **machine** builtin needs `EventTrack` or `Machine`, and prompt 142 reshapes both when it deletes
> contextual `Music`. Registering them against the shape that is about to go would be work thrown away.

Do the reshaping for the track, and register the eight. **The nine machine builtins moved to prompt 141ha**, on the
evidence in *What this prompt found* below: a machine is a constructor with type indices, which is a third reduction
shape `musa-calculus` does not have, and it needs a unit and a product type nothing has declared. After this prompt and
141ha the only rows left out of the core's registry are the eight collection eliminators, which become library code, and
the one phase projection, which is defined.

## What this prompt found

The four findings below are why this file was repaired before any code was written. Each is a fact about the core as
prompts 141b–141ga left it, checked against `crates/musa-calculus/src/eval.rs` and `base.rs` rather than inferred.

**A δ-rule never sees a function, and never sees a type.** `eval.rs`'s `canonical` answers `None` at `Form::Lam` and at
`Form::Universe`, and `delta` fires only when *every* argument on the spine is canonical data. So a builtin whose
signature has an arrow or a type parameter can be registered and can never fire: the spine blocks forever and the
operation is dead. That decides two things at once — `map_note_pitches` cannot be a δ-shaped rule, and the whole machine
family cannot be one either.

**There is nothing to translate.** The repaired sentence is the old one: this prompt said the eight rules were
"translated from `eval_builtin`'s arms rather than rewritten". They are not translatable. `eval_builtin`'s eight track
arms build a `MusicOperation` — a *deferred contextual* description that `elaborate.rs`'s `instantiate_music` closes
later against a key, a placement, a scope, and an origin. None of them computes an `EventTrack`, so there is no
arithmetic in them to carry over. What is pure and does carry over is one layer down: `WrittenPitch::transpose` and
`::invert`, `ScoreFact::stretched` and `::inverted`, `EventTrack::scale` and `::map_payloads`, and
`musa_kernel::together`. The rules are *written* from those, and the sentence they answer to is §5.7's rather than the
old arm's.

**The payload is not an index the core can write.** This prompt said `EventTrack` is "indexed the way `Duration` and
`Syntax` already are, and the payload parameter is what makes the index two rather than one".
`Registry::check_finite_data` admits a base type at a *literal* index and at nothing else, because a `fn` rule can build
a literal and cannot build a constructor — so a payload index would have to be a base type of payloads with a literal
per payload, and the compiler has exactly one payload today (`EventTrack<WrittenTime, ScoreFact>` is its only
instantiation, in `elaborate.rs`, `project.rs`, and `bench.rs`). A one-literal index is a knob with no caller, which is
the thing `.claude/skills/module-design` rule 6 names. `EventTrack` is registered at `Coordinate → Type 0`, and the
payload is fixed by the registration. A performance payload earns a second index the prompt that has a caller for it,
the way `Coordinate` earned its three literals from `Duration` and `Position` together.

**`play` mints facts, and a fact carries an `Origin`.** §5.7 requires that "all facts have the requested scope, exact
nonnegative placement, and a complete `Origin`". Seven of the eight transform a track they were handed, so the facts
already carry origins, and the *step* each appends — `ExpansionStep::Transposition`, `Stretch`, `Retrograde`,
`Inversion`, `MapNotePitches` — carries no span and is constructible from the rule's own arguments. `play` is the
constructor, and a `source_span`, a `definition_span`, and a `DeclarationId` are not things a `fn` pointer can invent.
So the origin is an *argument*, at a new inert base type, exactly the way `instantiate_quote` takes the anchor it builds
under (`11-quotation.md` §3): the one number the rule cannot compute is the one the caller supplies.

## Read

- `crates/musa-calculus/src/eval.rs`'s `delta`, `canonical`, and `constructed`, and `base.rs`'s `Reduction`,
  `Builtin::new`, and `Builtin::structural_with`. These are the two reduction shapes a builtin may have, and reading
  them is what produced the first finding above. `structural_with`'s doc argues `Family::Eliminator` "by construction
  rather than by parameter", on the grounds that "§5.8's other three families all compute a value from values" —
  `map_note_pitches` is the counterexample, and repairing that claim is part of this prompt.
- `crates/musa-compiler/src/registry/rules.rs`'s `UNREGISTERED` and `BEYOND`, and the accounting law in
  `registry/laws.rs` that counts each group again off the tables themselves. That law is what tells you this prompt is
  finished. `BEYOND` is also the precedent for a registered operation in neither ownership table, which this prompt uses
  once.
- `crates/musa-compiler/src/registry/traversal.rs` — the one existing structural registration, its vocabulary, and the
  termination argument in its module doc. `map_note_pitches` is registered the same way and its termination argument is
  shorter, because it applies itself nowhere.
- `crates/musa-compiler/src/core.rs`'s `BUILTIN_OWNERSHIP` rows for the eight, `Family::Track`, and
  `Builtin::parameters` — the signatures as the old checker states them, which are what is being re-stated over
  `EventTrack` rather than redesigned.
- `crates/musa-compiler/src/elaborate.rs` — `ScoreFact` and its `stretched`, `inverted`, and `pitch_of`; `Origin` and
  `ExpansionStep` in `origin.rs`; and `stretch_segment`, `retrograde_segment`, `invert_segment`, and
  `map_note_pitches_segment`, which are the contextual versions and stay where they are. What the new rules take from
  them is the per-fact arithmetic, not the `Segment` plumbing.
- `crates/musa-kernel/src/track.rs` — `EventTrack::scale`, `map_payloads`, `occurrences`, `duration`, `normalize`, and
  the free `track`, `event`, `follow`, `together`. Every one is total or answers a `KernelError`, which is what makes
  the result types below honest rather than uniform.
- [`../../rules/language/02-core-calculus.md`](../../rules/language/02-core-calculus.md) §5.7 (track-construction
  safety) and §5.8's third family. §5.7 is carried forward as an obligation to re-derive rather than an assumption to
  keep, and this is the prompt that re-derives its construction half against the new core.
- [`../../rules/constitution.md`](../../rules/constitution.md) §4 and §7, on why the core is a calculus of occurrences
  of a canonical payload over exact rational time, and
  [`../../rules/kernel/05-normalization.md`](../../rules/kernel/05-normalization.md) on what an `EventTrack` already
  guarantees — normalized, exact, versioned identity — which is the whole of the inertness argument below.
- [`../clean-break-ledger.md`](../clean-break-ledger.md) §1 and §2 — the rows prompt 142 owns. This prompt gives
  `EventTrack` a core spelling so that 142 can delete the type name `Music`; it deletes nothing itself.
- [`141b`](141b-base-types-and-builtins.md) and [`141e`](141e-compiler-registry.md) for how a base type is registered
  and how a signature is written in the core's terms, [`141d`](141d-finite-constructor-builtins.md) for what a rule may
  read and write, and [`141ga`](141ga-quotation-core.md) for the anchor-as-argument precedent and for the third count in
  `rules.rs`'s accounting, against which this prompt's arithmetic is stated.
- [`141g`](141g-raw-lowering.md), which reads the surface and deliberately leaves `EventTrack` unspellable — because
  until this prompt it denotes nothing.

## Design

**A track is a base type, and §5.8's own test is what decides it.** A base type is *inert*: it has no eliminator, no
source program takes it apart, and two closed values of it are convertible exactly when the host says they agree.
`musa-kernel`'s `EventTrack` is normalized, has a versioned exact identity, and is taken apart by nothing the source
language can write — every operation over it is a compiler-owned builtin. A declared family would have to expose
constructors that `05-normalization.md`'s normal form does not admit, which is the opposite of what a track's identity
is for. It is registered at `Coordinate → Type 0` for the third finding's reason, so the eight signatures are written at
`EventTrack ⟨written⟩` the way a beat is written at `Duration ⟨written⟩`.

**An origin is a base type for the same reason and with a narrower argument.** Nothing in the source language reads an
`Origin` — the Origin view reads a *compiled projection*, one stage down — and two origins agree exactly when their
fields do. It is registered plain, and the only operation that takes one is `play`.

**Partiality goes in the result type, one builtin at a time.** D2's rule is that a rule answers `None` at closed data of
its declared argument types only when the host's table is wrong, and `Malformed::BuiltinStuck` now reports it. So every
track builtin whose underlying operation can fail says so in its own result: `transpose` and `invert` because
`WrittenPitch::transpose` and `::invert` answer `Option` at a fixed-width coordinate overflow, `stretch` because a
factor at or below zero is not a stretch, `shift` because a negative offset is not a placement, and `play` because
`musa_kernel::event` answers a `KernelError`. `retrograde`, `together`, and `map_note_pitches` cannot fail and say so by
answering a track. Uniformity is not a reason to give a total operation an error case (`.claude/skills/module-design`
rule 5), and the composition cost — a `Result` where the old spelling had a track — is 142's to spend on the surface,
not this prompt's to hide.

**`map_note_pitches` is a track builtin with a structural reduction, and that is one repair to `musa-calculus`.** Its
mapper is a function, so no δ-rule can fire at it. `Builtin::structural_with` is the mechanism that fires on a literal
and answers a term, and it hard-codes `Family::Eliminator` on an argument this prompt's evidence falsifies. Take the
family as a parameter — the family says which of §5.8's four admissibility arguments covers the operation, the reduction
says how it computes, and `map_note_pitches` is the case that proves they are two questions. Its rewrite reads the track
literal, applies the mapper to each written pitch it holds, and hands the answers back through one registered operation
in neither ownership table:

```text
map_note_pitches(f, t)  ⟶  set_note_pitches(⟦t⟧, [f ⟨p₀⟩, …, f ⟨pₙ⟩])
```

`set_note_pitches : EventTrack ⟨written⟩ → List Pitch → EventTrack ⟨written⟩` is total by construction — it replaces the
*i*th note's pitch with the *i*th member and keeps the rest — and it is not a source word, for `BEYOND`'s reason:
`map_note_pitches` is the controlled transform §5.7 admits, and a spelling that put arbitrary pitches into a track
directly is the control removed. Termination needs no argument at all: the rewrite applies `map_note_pitches` nowhere.

**Nothing is deleted and nothing is wired.** Contextual `Music`, `MusicOperation`, `MusicRole`, and the `close` and
`instantiate_music` path are prompt 142's rows on the ledger, and the old checker keeps using them until it goes. What
this prompt adds is a second, *unused* spelling of the same values in the core's terms — which is what 141e, 141f, and
141ga also added, and is what makes the cutover a rewiring rather than a rewrite.

## Target

- `crates/musa-calculus/src/base.rs`: `Builtin::structural_with` takes the family rather than fixing it, with its doc
  repaired to say why the family and the reduction are two questions, and `Builtin::structural` unchanged at
  `Family::Eliminator` since a traversal is one. `traversal.rs`'s two registrations pass the family they already had.
- `crates/musa-compiler/src/registry.rs`: `EventTrack` registered at `Coordinate → Type 0` and `Origin` registered
  plain, both doc-commented with the §5.8 inertness test that decided base-type-over-family, and an `origin_literal`
  beside `coordinate_literal` for the one caller that writes one.
- The eight track builtins registered as `musa_calculus::Family::Track`, seven with δ-rules and `map_note_pitches` with
  a structural one, in a `registry/track.rs` beside `traversal.rs`. `set_note_pitches` registered beside them and named
  in a `TRACK_BEYOND` counted the way `BEYOND` is.
- `crates/musa-compiler/src/registry/rules.rs`: `REGISTERED` and `UNREGISTERED` updated, leaving
  `("structural eliminators", 8)`, `("machine builtins", 9)`, and `("phase projections", 1)`, with the doc saying what
  each remaining group is waiting for and which prompt owns it.
- Laws beside them: each of the eight answers what the pure operation it is written from answers, on a track built by
  hand; §5.7's construction clause discharged as a law about `play` — the track it answers is one the kernel accepts,
  with the requested placement and a complete `Origin` on every fact; each partial one refuses in its result type rather
  than getting stuck, which is D2 at the one family that is not δ; and `map_note_pitches` applies the mapper exactly
  once per note and leaves every non-note fact alone.
- `docs/plan/code-map/` rows for `musa-compiler`, and the row-16 sentence naming 141h for `Machine` repaired to name
  141ha.
- Prompt 142's Read and `depends_on` repaired to name this prompt by its new slug.

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

Commit as `Give the track a core shape`.

## Stop

- No machine builtin, no `Machine` or `Primitive` base type, no unit and no product type. Prompt 141ha owns all of it,
  and the two arguments share no lemma — §5.7 says so itself.
- No deletion. Contextual `Music` and every ledger row 142 owns survive this prompt; the old checker still runs the
  corpus.
- No collection eliminator registered. 141c argued that exclusion and it stands: they become library code in 142.
- No `.musa` file changes and no surface spelling for `EventTrack`. 141g deliberately leaves it unspellable and 142 is
  where the source learns the word.
- No amendment to §5.7 or §5.8. If a signature cannot be written in the core's terms, that is a finding and stop
  condition 4 — the same rule 141c, 141f, and this repair worked under.
- No change to `musa-kernel`. It stays a leaf, and the track's core spelling is a base type registered by the compiler,
  not a type the kernel exports.
