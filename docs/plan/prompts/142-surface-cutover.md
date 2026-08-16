---
id: 142
slug: surface-cutover
status: pending
depends_on: [136a, 141, 141b, 141c, 141d, 141e, 141f, 141g, 141h]
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
- [`141h`](141h-track-and-machine-core.md), which gave `EventTrack` and `Machine` their core shapes and registered the
  seventeen builtins 141e left, on the grounds that they needed a reshape this prompt would perform. The reshape moved
  there; what stays here is deleting contextual `Music` and teaching the source the words.
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
answer the same question, one of them is going to disagree eventually.

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
  `check_material` calling 141g's lowering and `musa_core::check`, with the old checking path deleted. The reading is
  built; what this delivers is the passes that call it and the readback that turns a normal form into the musical value
  a consumer receives.
- `infer.rs` deleted, `core.rs`'s superseded arms deleted, `names_a_phase_type` deleted, second-path audit recorded.
- `stdlib/`, `examples/`, book fixtures, LSP fixtures, desktop generated fixtures, and the tree-sitter corpus migrated.
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
