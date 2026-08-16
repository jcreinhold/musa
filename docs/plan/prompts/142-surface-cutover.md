---
id: 142
slug: surface-cutover
status: pending
depends_on: [136a, 141, 141b, 141c, 141d, 141e, 141f, 141g, 141ga, 141h, 141ha, 141i, 141j, 141k, 141l, 141m, 141n, 141o]
phase: 3
---

# Move the Whole Language Over, Once

## Task

Close the seam. Wire `musa-compiler` to `musa-core`, migrate `stdlib/`, `examples/`, and every fixture corpus onto the
new language, delete contextual `Music` and the rank-1 Hindley–Milner checker along with every superseded checking path,
and discharge `docs/plan/clean-break-ledger.md`. This is the one migration, and it is green at the end rather than in
the middle. It absorbs prompt 127e, whose file stays as a superseded record rather than being repaired.

## Read

- `docs/plan/clean-break-ledger.md` in full — every source spelling, Rust API, serialized form, fixture, and test name
  it names as deleted rather than aliased. This prompt discharges the rows it owns and leaves the machine rows to
  150–153.
- The superseded prompt [127e](127e-source-language-clean-break.md), whose whole Task this prompt absorbs. Everything it
  specifies is this prompt's obligation: contextual `Music` goes, notation becomes ordinary values, and placement comes
  from the enclosing voice's left fold. Absorbing it was right because migrating `stdlib/` twice — once onto the old
  core's replacement, once onto the new language — would have been the same files rewritten for two different reasons
  within ten prompts.
- [`141b`](141b-base-types-and-builtins.md), which gives `musa-core` the base types, literals, and builtin registry
  without which no real Musa program can be elaborated by it.
- [`141e`](141e-compiler-registry.md), which *filled* that registry: the compiler's prelude declarations, the inert
  musical domains as base types, the 92 δ builtins and 14 δ phase builders as `musa_core::Builtin`s, and the sampled
  agreement with the old evaluator that makes them a translation rather than a rewrite. It also names the 28 entries it
  left and why — which is where this prompt's own list of what still has no core spelling comes from. Its accounting law
  is what will fail if this prompt forgets one.
- [`141f`](141f-phase-traversals.md), which registers the two phase traversals and turns the sealed step into a declared
  family with a private constructor. `staff.musa` calls `recurse_syntax` four times and does not compile without it, and
  `run_syntax_step` is a definition here rather than a builtin, so the migration writes `step.run(context)` where the
  old source wrote a call.
- [`141g`](141g-raw-lowering.md), which built the reading half of this prompt's first Target bullet — surface CST to
  `musa_core::Raw`, with the site table that points a refusal back at a span, and laws rather than a caller. It exists
  because this prompt's own Design asks for the three stages in order and an ordering inside one commit is not
  observable; the argument is 141e's, one level up. What is left here is the *wiring*: the passes that call it, the
  readback out of normal forms, and the deletion of what it replaces.
- [`141ga`](141ga-quotation-core.md), which gave a template an inert core shape and moved both quotation forms onto
  δ-rules that call `crate::syntax::instantiate` and `crate::syntax::matched` rather than a second copy of either. It
  exists because 141g's own implementation proved that building a quote out of the phase builders cannot express a
  spread in a separated position without a compiler-generated indexed fold. What is left here is deleting
  `ExprKind::SyntaxQuote` and the checker's own copy of the body walk — and **§1's forgetting rule**, which 141ga's
  pattern side needs and which every phase builder already needed: each of them reads at `Syntax ⟨tokentree⟩`, the core
  has no subtyping, and a `Syntax ⟨expr⟩` in hand is therefore a value with nowhere to go until this prompt says how a
  category is forgotten.
- [`141h`](141h-track-core.md), which gave `EventTrack` its core shape and registered the eight track builtins 141e
  left, on the grounds that they needed a reshape this prompt would perform. The reshape moved there; what stays here is
  deleting contextual `Music` and teaching the source the word.
- [`141ha`](141ha-machine-core.md), which gave `Machine` its core shape and registered §2's eight *grammatical* forms as
  constructors. It split off 141h because a machine reduces to nothing and needs a unit and a product that 141h's track
  does not. Two things stay here rather than there. **`primitive` is unregistered**, and deliberately: §1's ninth row is
  typed by the build-local registry — the written name and version select a descriptor that supplies the step, both
  ports, *and the configuration argument's type* — so it is a different type per registered pair rather than one Π short
  of writable, and only a stage that can perform that lookup can type it. This prompt is that stage; it is counted in
  `registry/rules.rs`'s `UNREGISTERED` until then. **Storability is unchecked**, for the same reason:
  `02-core-calculus.md` §1.2 states it as a constraint, a `Builtin`'s type is a `Term` with no constraint binder, and so
  `Machine ⟨step⟩ (Nat → Nat) Nat` is writable in the core today. Discharging it belongs to the elaboration here that
  has the constraint solver, with `03-machine-calculus.md` §5's preparation refusing what survives. Also here: deleting
  `Type::Machine` and the old checker's machine path.
- [`141i`](141i-constrained-definitions.md), which gave a free `fn`, `record`, and `enum` their `where` clause. The
  migration below needs it: a generic parameter acquires no method without a constraint, and `stdlib/` is full of
  generic definitions that will.
- [`141j`](141j-notation-vocabulary.md) and [`141k`](141k-notation-lowering.md), which are the notation half of this
  prompt's first Target bullet and were split out of it for 141g's reason. 141j registers what a notated block is built
  out of — `sounded`, `follow`, and `nothing` beside `play`, and `Fact` declared as a family mirroring `FactKind` — and
  141k reads the surface into it: a block is a left fold over `follow`, each statement is one call, and pitches resolve
  against the lexical `in scale` before any track value exists. Both exist because starting this prompt found that
  neither the words nor the reading were there: `play` was the one builtin that constructed anything, one `FactKind`
  case out of nineteen, and sequencing had no word at all, because in the deleted contextual-`Music` design placement
  was the evaluator's cursor rather than an operation. What is left here is the document *structure* those two do not
  own — `piece`, `score`, `part`, the `voice` declaration around a body, `section`, `bar`, `template`, `make`, and the
  context tracks for tempo, meter, key, and clef — together with the wiring, the readback, the migration, and the
  deletions.
- [`141l`](141l-qualified-path.md), which read the `::` path 141g left as its own text. The migration below needs it
  twice over: `stdlib/src/adapters/` spells the phase enumerations `TokenKind.Comma` and moves to `TokenKind::Comma`
  here, and every generic definition that acquires a `where` clause reaches its methods by `Trait::method` — the fix
  `01-surface.md` §1.5's refusal table names and the escape hatch `10-traits.md` §6's strictness is affordable because
  of. It also carries a finding for this prompt's second-path audit: `elab::constrained_function_type` names the
  dictionary binder after the trait, so `(Eq.equal)(x, y)` is a spellable projection out of a binder the core minted and
  a second route to the qualified reading.
- [`141m`](141m-rule-refusal.md), which gave a δ-rule a way to say the program is wrong and took `Result` off the
  notation vocabulary. Two things arrive here from it. Its survey table lists the eleven arithmetic, duration, and
  position rules still answering `Result τ Text` and the twenty corpus sites that branch on them, which is one item of
  this migration written out in advance. Its "what §2 still says that is not true" list is the other: `play(chosen,
  1/2)` and `-> EventTrack[WrittenTime, ScoreFact]` are both spellings this prompt makes true or repairs in
  `01-surface.md`.
- [`141n`](141n-top-level-program.md), which gave `musa-core` the thing a pass hands a whole document to: one group of
  named definitions, every signature collected before any body, so a later declaration may be referenced. It was split
  off when starting this prompt found that the core had no door for it — `declare` takes a *data* group, `check` takes
  one term, and `Cx::define`'s binder is nameless on purpose, so the only named binding on offer was `RawShape::Let`,
  which scopes forward only. `examples/neo-riemannian.musa:72` calls the `compose_close` declared at `:151`, and
  `examples/tonal-construction.musa` does the same kind of thing eighteen times, so a nested `let` chain would have
  refused the existing corpus before any of it was migrated. What is left here is building the group out of 141g's items
  and handing it over.
- [`141o`](141o-document-elaboration.md), which walks a whole document: which of the core's four doors each written
  declaration goes through, the order they open in, and the dependency ordering the family groups need for the reason
  141n's definitions needed one. It was split off when starting this prompt found that `crate::lower` had no caller at
  all outside its own laws — thirteen prompts of reading and vocabulary, and nothing had ever handed the new checker a
  real file. Its survey is this prompt's work list and its argument for scope: `stdlib/`'s sixteen non-adapter libraries
  elaborate as one document with exactly two faults left, `Music` and an anonymous product, and both are rows in the
  Target below. What is left here is the *readback*, the four passes, the migration, and the deletions.
- [`141c`](141c-structural-eliminators.md), which adds the second of §5.8's four families: a compiler-owned operation
  that takes a function argument and rewrites rather than computing. `recurse_syntax`, `run_syntax_step`, and
  `syntax_fold_from_leaves` are registered through it, and `staff.musa` does not compile without them. Its Design also
  says where the eight collection eliminators go — they become library code *in this prompt's migration*, which is why
  they are not registered there.
- [`141d`](141d-finite-constructor-builtins.md), which widens a δ-rule from literals to finite data. Thirty-eight of the
  92 δ-builtins in `BUILTIN_OWNERSHIP` answer an `Option`, a `Result`, or a `List`, and six take a `List`; without it
  the registry this prompt fills cannot be filled. Its `Datum` is the shape those 38 rules are written against here —
  one owned type in both directions, so a rule that is handed an `Option` and answers one transcribes nothing. Reading
  is untyped and writing is realized against the builtin's own result type; a rule here writes `Datum::Case` with the
  qualified constructor name and its fields, and never names a `Constant`.
- `crates/musa-compiler/src/infer.rs` and the checking paths in `core.rs` — what is deleted, and what has to keep
  working because something other than the checker uses it.
- `crates/musa-compiler/tests/suite/elaboration_compatibility.rs`, especially
  `existing_language_behavior_matches_the_migration_oracle`, and `tests/fixtures/elaboration-expected-changes.json` —
  the oracle fixes semantic hashes, kernel digests, diagnostic codes, Origin paths, and the MEI/LilyPond/MusicXML/MIDI
  corpora. This is the only prompt permitted to move an entry in it, and every moved entry is argued in the JSON, by
  slug.
- `docs/rules/language/00-semantics.md` §3 — reusable material as an ordinary value, which is the shape notation takes
  after contextual `Music` is gone.
- `apps/musa-desktop/ui/src/lib/session/generated/` and `apps/musa-desktop/ui/fixtures/`, and
  `docs/notes/toolchain/generated-files.md` — generated files are regenerated by their generator, never edited. A
  migration that hand-edits one has broken the rule the note exists for.
- The `module-design` skill's audit questions. Deleting a checker is the largest chance this project will get to make
  `core.rs` smaller; taking it requires knowing which boundary the deletion falls along.

## Design

**One commit, green at both ends.** The tree compiles and every gate passes before this prompt and after it, and not
necessarily anywhere between. That is the price of a clean break, and the alternative — a compatibility mode, a second
elaborator selected by a flag, a per-file opt-in — costs more, because every later reader has to learn both languages
and decide which one a file is in.

**Order the work so the migration is mechanical.** Elaborate through `musa-core` first and get the existing corpus
passing under the new checker with the old spellings still in place; then migrate spellings; then delete. Mixing the
three makes every failure ambiguous between "the new checker is wrong" and "this file was translated wrong", and there
will be enough failures that the distinction matters. Prompt 141e already took the first slice of this — the registry
the new checker reads names from — for the same reason and one prompt earlier: a table of 117 builtins is a
*translation* with an oracle to check it against, and burying it inside a diff that also moves 11,304 lines of `.musa`
would have made a wrong signature indistinguishable from a wrong migration.

**Contextual `Music` goes, and placement moves to the fold.** A fragment is an `EventTrack[WrittenTime, ScoreFact]`, a
motif is a function returning one, and placement is applied by the enclosing voice's left fold rather than read from an
ambient context. `00-semantics.md` §3 already says this; this prompt makes the source say it too.

**The oracle moves once, and every move is argued.** A changed semantic hash, kernel digest, diagnostic code, Origin
path, or rendered corpus file is an entry in `elaboration-expected-changes.json` with a defect and an observation,
naming the prompt by slug. An unargued move is indistinguishable from a regression, and after this prompt there is no
second chance to tell them apart. **A rendered-notation change is a defect until proven otherwise**: the MEI, LilyPond,
MusicXML, and MIDI corpora describe what a musician sees, and a type-checker replacement has no business changing any of
them.

**Delete the second path, and audit for a third.** `infer.rs`'s unifier, the superseded checking arms in `core.rs`,
`names_a_phase_type` and the collision it worked around, and every function that only existed to keep the old
representation working. Then run the second-path audit the core prompts already established: if two code paths can
answer the same question, one of them is going to disagree eventually. 141l's finding is already on the list: the
dictionary binder a `where` introduces is named after its trait, so a constrained body can project a method out of it by
a spelling no document offers, and the audit decides whether that binder should be unspellable the way a desugaring's
own binders are.

**Take the split `core.rs` has been asking for.** 15,017 lines in one file is not a module boundary; it is the absence
of one. The checker's departure is the moment to see what is left and whether it is one thing. Do not restructure
speculatively — the `module-design` rule is that a boundary hides a volatile decision — but do not put the remains back
in one file out of momentum either.

**Everything downstream moves in this commit**: `stdlib/`, `examples/`, `docs/book/` fixtures, LSP fixtures, desktop
fixtures via their generators, and `editors/tree-sitter-musa`'s corpus. The book's teaching pages quote fixtures and
`scripts/check-docs.sh` holds them to it, so a migrated fixture with an unmigrated page is a failing gate, not a
follow-up.

**`staff.musa` migrates; it is not rewritten.** It moves onto the new language with its structure intact, still
backwards, still without quotation. Prompt 145 rewrites it and measures the result against 2,404 lines, and a partial
rewrite here would destroy that measurement. The same applies to the studio adapter and prompt 146.

## Target

- `musa-compiler` elaborating through `musa-core`: `check_piece`, `check_arguments`, `check_template_voice`, and
  `check_material` calling 141g's and 141k's lowering and `musa_core::check`, with the old checking path deleted. The
  declaration reading is 141g's and the notation reading is 141k's; what this delivers is the document structure around
  them — the score's parts and voices, the context tracks a tempo, meter, key, or clef statement contributes, templates
  and their instances — the passes that call the whole of it, and the readback that turns a normal form into the musical
  value a consumer receives. A track is a literal at `EventTrack ⟨written⟩`, so the readback is a normal form, a
  literal, and the payload it holds; what costs something is `Program`'s shape, which exists to defer contextual
  instantiation and has nothing left to defer.
- `infer.rs` deleted, `core.rs`'s superseded arms deleted, `names_a_phase_type` deleted, second-path audit recorded.
- `primitive` typed against the build-local registry, and `registry/rules.rs`'s `UNREGISTERED` shortened by the row
  141ha left there. It is not one more signature: the name and version select the descriptor that supplies the ports and
  the configuration type, so this is elaboration reading a registry, not a table gaining an entry.
- The storable-port premise discharged where the constraint solver is — a port is `Storable`, and
  `Machine ⟨step⟩ (Nat → Nat) Nat` is refused here rather than left to `03-machine-calculus.md` §5's preparation.
- `stdlib/`, `examples/`, book fixtures, LSP fixtures, desktop generated fixtures, and the tree-sitter corpus migrated.
- The eleven arithmetic, duration, and position rules 141m left answering `Result τ Text` moved onto the refusal
  channel, and the twenty `match … { Ok(v) -> … }` sites its survey table lists rewritten. 141m could not: one table
  serves two checkers, so narrowing a declared answer there rewrote the corpus, which is this prompt. The criterion is
  141m's and unchanged — `Option` is a musical answer, `Result τ Text` is a diagnostic wearing a value's clothes — and
  what makes it worth doing here rather than never is that 141l routed `x + y` through `Add.add`.
- Contextual `Music` and `ContextualMusic` gone from source, compiler, and documents.
- `docs/plan/clean-break-ledger.md`: every row this prompt owns marked discharged, with the rows left for 150–153 named.
- `tests/fixtures/elaboration-expected-changes.json`: every moved oracle entry, argued.
- `docs/plan/code-map/` rows for every crate that changed, and `docs/rules/language/` repaired wherever the
  implementation proved a specification claim wrong.

## Check

```sh
cargo build --workspace
cargo nextest run --workspace
cargo nextest run --run-ignored all
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
cd editors/tree-sitter-musa && tree-sitter test
pnpm -r check && pnpm -r test
cargo deny check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make lint-ui
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
```

Every example in `examples/` must still compile and render, and every rendered corpus file must be byte-identical unless
its change is argued in `elaboration-expected-changes.json`.

Commit as `Move the whole language over, once`.

## Stop

- No compatibility mode, no alias, no deprecation shim, no per-file language selection, no "old syntax still accepted
  with a warning". The ledger's rule is rename, delete, and version-break in the prompt that owns the concept.
- No rewrite of `stdlib/src/adapters/staff.musa` or the studio adapter beyond mechanical migration. Prompts 145 and 146
  own those, and own their measurements.
- No builtin-registry collapse. Prompt 143.
- No re-translation of a signature or a rule 141e already wrote. A disagreement between the two is a defect in one of
  them and is repaired where it is, not worked around here.
- No performance work and no benchmark rerun. Prompt 144 measures the finished checker; measuring a half-migrated one
  would produce a number nobody can act on.
- No hand-edited generated file, under any deadline pressure.
- No new language feature. If the migration wants one, that is a finding and a repair, and it is far better to record it
  than to slip it in under a migration diff.
