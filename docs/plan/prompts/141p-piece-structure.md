---
id: 141p
slug: piece-structure
status: done
depends_on: [141k, 141o]
phase: 3
---

# Give the Fold a Voice to Belong To

## Task

[`crate::lower::notation`](../../../crates/musa-compiler/src/lower/notation/mod.rs) reads a block of statements and
refuses seven of them. Four — `key`, `meter`, `tempo`, `clef` — are refused as *misplaced*, because "a `music` value is
usable at several places, and 'from here onward' has no unique meaning there". Three — `bar`, `senza`, `assert` — are
refused as having no core spelling yet, at a node whose label reads **"a voice to belong to" is written here**.

Both refusals are about the same missing thing, and it is not a statement. It is the document structure that would give
a statement a scope: the score, its parts, their voices, and the numbering every `ScoreFact` is constructed at.

Build that structure, and use it to say what `examples/` still needs.

## Read

- [`141k`](141k-notation-lowering.md)'s `Reading` and its module documentation, in particular **"Scope is given, not
  discovered"**: a `music { … }` value "has no voice of its own and reads at `crate::Scope::Piece`", and "when prompt
  142 lowers a *voice's* body it passes the voice's scope down the same way, and nothing here changes". This prompt is
  that sentence, and it is worth reading as a *prediction*: if reading a voice needs anything from `notated` beyond a
  different `Reading`, the prediction was wrong and that is the finding.
- [`141o`](141o-document-elaboration.md) in full — the walk this one hands a piece to, and the precedent for its shape.
  Its Stop says why `examples/` were left out: "they are pieces rather than libraries, so surveying one means walking a
  `piece` and its templates, which is document *structure*". This prompt is the first half of that sentence and the
  survey it was blocking.
- `crates/musa-compiler/src/elaborate.rs`'s `elaborate_score` and `context_facts`. They are the behaviour being
  replaced, and three of their decisions are load-bearing rather than incidental: a part's id is a running count over
  the score, a voice's id is its *position* among the part's items, and the piece's extent is the **maximum** of its
  voices' rather than the sum. Read them for those three and for the shape of the final `Term::together(parts ++
  [context])`; the projection and the barline machinery around them are 142's.
- `crates/musa-compiler/src/resolve.rs`'s `part_context` and the `PartContext` beside it. A part's clef, its own meter,
  and its own tempo are already read there, with the refusals that reading needs, and this prompt calls it rather than
  re-deriving it — root `AGENTS.md`, "hand a consumer what we already computed".
- `crates/musa-compiler/src/registry/notation.rs`'s module documentation on why `sounded` is not a source word: "a
  source `sounded` would hand every program the ability to put a `Fact.Key` in the middle of a voice". That is the
  reason the four context statements stay refused inside a `music` value even after this prompt gives them a scope
  everywhere else, and it is why the fix is a wider `Reading` rather than a wider statement table.
- `../../rules/language/00-semantics.md` §3's composition equations. `follow` is what putting one statement after
  another means and `together` is what putting one voice beside another means; a piece is the second equation over its
  voices exactly as a block is the first over its statements. Nothing here is a new combinator.
- `../../rules/kernel/03-denotational-semantics.md`'s **D3**, which is where the extent rule actually comes from:
  `together(M, N) = (max(d, e), E ⊎ F)`, and "nothing is inserted into the uncovered portion of the shorter track". Read
  `crates/musa-compiler/src/scope.rs` beside it for what `Scope` already promises about inheritance — a part's 7/8 does
  not rejoin the piece's 4/4, and that is `Override` rather than anything this prompt writes.
- Peyton Jones **ch. 3**. A piece is elaborated by *enriching* the reading the fold already has with the one fact it
  lacks, not by writing a second traversal beside it. If this prompt ends up with a second statement table, that is the
  sublanguage-by-subtraction shape root `AGENTS.md` forbids, one level up.

## Design

**A piece is a term, and it is the same two equations.** A voice is `follow` over its statements; a part is its voices;
a piece is `together` over its parts and one more track carrying the context. That is what `elaborate_score` already
builds by hand, and writing it as a raw term rather than a kernel one is the whole of the change.

**What the structure is *for* is scope.** The seven refused statements are refused for one reason between them, and this
prompt removes it by widening `Reading` rather than by widening the statement table: reading a voice's body is
`notated(node, Reading { scope: Scope::Voice { part, voice }, .. })` and nothing else. A piece's header statements are
the same nodes read at `Scope::Piece`, a part's at `Scope::Part { part }`. The four context statements therefore stay
refused in a free `music` value — that is `registry::notation`'s argument and it survives untouched — and become
ordinary where a voice or a piece encloses them.

**The distinguishing bit is `placed`, not the scope.** 141k's prediction was right about the mechanism and one word off
about the discriminator: a free `music { … }` value reads at `Scope::Piece` and so does a piece *header*, so a rule
keyed on the scope would refuse the header too. What separates them is whether the block stands at one place in the
piece, which is a third field on `Reading` and the only thing the four statements consult before choosing a scope. Found
by writing the refusal and watching it fire on `piece "…" { key g major; … }`.

**Numbering is positional and counts every item.** A part's id is a running count over the score; a voice's id is its
position among the part's items, *including* the instance sites this prompt does not expand. A `make` between two voices
therefore keeps the number it will have when 142 expands it, so the two paths bucket into the same voices and the
migration's diff is about spellings rather than about renumbering.

**A region is the longest voice, not the sum of one.** `notation::extent` sums, because a block is a fold and a fold's
extent is a sum. A piece's is D3's maximum over its parts, so the header facts' region is computed from the voices
rather than by calling the same function one level up. Stating it here because it is exactly the mistake the shared word
invites.

**Templates and `make` are not this prompt's.** An instance site mints an expansion path, `Sites` numbers nodes and has
no notion of one, and deciding what provenance a spliced voice carries is a design question with its own evidence. Bar
structure is the other one: a `bar`, a `senza`, and an `assert` are checked against barlines a *pass* resolves, and
`notation.rs` already says so at the refusal. Both are recorded by the survey and left to 142.

**The survey is the deliverable again.** 141o's is the standard library; this one is `examples/`, all fifty-four, with
the remaining faults recorded exactly and each named as something 142 already owns. Until it exists nobody can say
whether a piece that fails to elaborate is a migration or a defect, and that is the only question worth answering before
a cutover.

**No caller.** For 141o's reason one level up.

## Target

- `crates/musa-compiler/src/lower/piece.rs`: `Lowering::piece`, answering the whole piece as one raw track together with
  the parts and voices a consumer needs to bucket a projection — their ids, in written order, and their names. Nothing
  else, because nothing else has a caller.
- `Reading` widened to the two scopes it lacks, and the four context statements and the score's own structure read
  through it. No second statement table and no new combinator.
- Part and voice numbering, with the duplicate-name refusals the numbering forces, restated at the declaration that
  repeated the name.
- Part context read through `resolve::part_facts` rather than re-derived, and its clef, meter, and tempo entering the
  piece's context track at `Scope::Part`. **The reading answered `part_facts` and not `part_context`**: the existing
  function also resolves the part's *profile*, which is a declaration this walk has nothing to do with, so the two facts
  a shared reading owes were split out from the one thing only the replaced path wants — with `resolve::Marking` split
  out of `tempo_fact` for the same reason, since what a writer here needs is the marking and not the `FactKind` the old
  path builds from it. `Lowering::heard` is what makes the shared reading callable: `crate::resolve` reports on the
  resolver and answers what it could still make of the source, everything in `lower` answers `Option`, and one adapter
  between them is cheaper than a second reading of a part's header.
- Each context fact carrying **its own statement's origin**, not the piece's, and an unwritten meter carrying
  `Origin::UNKNOWN` — which `provenance_at` already reads as "nowhere of its own to point", and which is the same empty
  span `elaborate.rs` writes there today. Not in the first draft, and put here because the first run of the laws showed
  every header fact pointing at the whole declaration.
- `Document::term`, elaborating a raw term in a document's own context, with `Document::value` written in terms of it —
  one question rather than two, because a name is a term. Beside it `Document::piece`, which is what lets a caller read
  a piece *out of the document that binds its motifs*; the two are one door, since a piece's track is a raw term and the
  context it is checked in is the document's.
- Laws beside the module: a voice's statements are read at the voice's scope; a piece's header facts cover the longest
  voice; two parts with one name are refused, and two voices of one part likewise; a `key` written in a piece is
  ordinary and the same `key` written in a `music` value is still misplaced.
- **The survey**, as a law: every `examples/*.musa` elaborated **with the standard library in scope**, with the
  remaining faults recorded exactly and each one named as something [`142`](142-surface-cutover.md)'s Target owns. The
  corpus is `document/laws.rs`'s own, shared rather than copied, for the reason 141o's survey gives for reading sixteen
  files as one document: a survey that reported `option_fold` missing would be measuring its harness. One fault it finds
  was *not* a row 142 had — a notation statement whose argument is a bound name — so this prompt is preceded by a repair
  that adds it.
- A `docs/plan/code-map/spec-to-implementation-map.md` row for the piece reading.

## Check

```sh
cargo build --workspace
cargo nextest run -p musa-compiler
cargo clippy --all-targets -p musa-compiler -- -D warnings
cargo fmt --check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
python3 scripts/renumber-prompts.py audit
```

Commit as `Give the fold a voice to belong to`.

## Stop

- **No caller and no deletion.** `elaborate.rs`, `core.rs`, and the four passes are untouched; 142 owns all of them.
- **No readback.** The piece is answered as a raw term and checked as one. Turning a normal form into a `VoiceTrack`, a
  projection, or a snapshot is 142's.
- **No templates and no `make`.** An instance site is recorded by the survey, not expanded.
- **No bar structure.** `bar`, `senza`, and `assert` stay refused exactly as `notation.rs` refuses them now; barlines,
  meter resolution, and assertion obligations are a pass over a finished voice.
- **No migration.** Not one line of `.musa` changes.
- No performance work, and no new language feature. If the survey wants one, that is a finding for 142.
