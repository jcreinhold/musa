---
id: 62
slug: mark-vocabulary
status: done
depends_on: [27, 42]
phase: 3
---

# An Open Vocabulary of Marks

## Task

`ArticulationMark` is a closed enum of five variants with **65 references across 13 files** in three crates. Replace it
with one table — `MarkDef`, describing a mark's name, where it attaches, what arguments it takes, and what each backend
calls it — and one payload, `Mark`, carrying a name and its arguments. Adding a sixth mark then costs one table row
instead of a variant, five `match` arms, three backend arms, and a `NAMES` constant.

Behaviour does not change. The five marks stay exactly five marks, spell the same, engrave the same, sound the same, and
every golden stays byte-identical.

## Read

- `crates/musa-compiler/src/score.rs` — `ArticulationMark` and its `NAMES`/`parse`/`name` triple. Count the `match` arms
  in `resolve.rs` (:558), `elaborate.rs` (:1146), `factext.rs` (:160), `performance.rs`, `project.rs`, `profile.rs`,
  `plan.rs`, `ly.rs`, `mei.rs`, `musicxml.rs` before starting; the count is the argument.
- `crates/musa-language/src/parser.rs::articulations` (:1198) — it accepts a **greedy list of bare identifiers** and
  validates nothing. This matters: the surface grammar is already open, and only the compiler is closed.
- `crates/musa-language/src/syntax_kind.rs` :85–87 — studio processor names are deliberately not keywords "so the studio
  vocabulary can grow without lexer changes". This prompt applies the same decision to notation, and cites it as
  precedent rather than inventing a rule.
- `crates/musa-compiler/src/resolve.rs::articulation_settings` (:588) — the profile side, which reads `gate` and
  `attack`.
- `docs/initial-design-roadmap.md` §2 and AGENTS.md's layer table: an articulation as written is not a gate multiplier.
  The table must not acquire a `gate` column.

## Design

### The one idea, stated precisely

Musa has no fermata. The reason is not that fermatas are hard — it is that `ArticulationMark` is an enum, so a fermata
is a breaking change to a public type that thirteen files match on exhaustively. Meanwhile the *parser* would have
accepted `g4 1 fermata;` since prompt 27 without a single edit.

So the closure is in exactly one place, and that is where to remove it. **The vocabulary becomes data.**

```rust
// crates/musa-compiler/src/marks.rs

/// Where a mark attaches to the music.
pub enum Placement {
    /// To the note or rest it is written on: `g4 1/4 staccato;`
    Attached,
    /// At an instant: a breath, a caesura, a rehearsal letter.
    Point,
    /// Over a stretch: a pedal, an ottava, a trill with a wavy line.
    Span,
}

/// What a mark's argument may be. Not `Text`-for-everything: an argument the
/// compiler cannot check is an argument the composer finds out about by
/// looking at the page.
pub enum ParamTy { Number, Ratio, Duration, Pitch, Interval, Text, Word(&'static [&'static str]) }

/// One entry in the notation vocabulary.
pub struct MarkDef {
    pub name: &'static str,
    pub placement: Placement,
    pub params: &'static [Param],
    /// What each backend calls it, or `None` where the backend has no such mark.
    pub mei: Option<&'static str>,
    pub musicxml: Option<&'static str>,
    pub lilypond: Option<&'static str>,
}

/// A mark as written: a name from the vocabulary and its arguments.
pub struct Mark { pub name: Box<str>, pub args: Vec<MarkArg> }

/// The vocabulary. This prompt ships exactly the five that exist today.
pub const VOCABULARY: &[MarkDef];

/// Look a mark up by the name the composer wrote.
pub fn lookup(name: &str) -> Option<&'static MarkDef>;
```

### What the backends get

Three emitters shaped by `Placement`, not N shaped by mark. `ly.rs`, `mei.rs`, and `musicxml.rs` each lose a five-arm
`match` and gain one lookup of the backend's own name column. That is the whole payoff, and it is the reason the table
carries backend names rather than the backends carrying tables: a mark's spelling in three formats is one fact about the
mark, and splitting it three ways is what produced three `match` expressions that could drift.

Only `Placement::Attached` has a producer in this prompt. `Point` and `Span` are in the enum because the table would
otherwise have to be redesigned at prompt 70 — but a `Placement` with no producer and no emitter is dead surface, so the
two unused emitters are **not** written here. Prompt 70 writes them.

**A mark with no backend name is an export warning, never a silent drop.** `None` in a column is a fact the table
states, so the exporter can say "MusicXML has no `caesura`; it is omitted" rather than producing a page quietly missing
something. This is the one behaviour this prompt adds, and it adds it for zero marks today because all five exist
everywhere.

### The profile side

`profile violin { articulation staccato { gate = 1/2; } }` becomes `mark staccato { … }`. Same settings, same
diagnostics, one word. `articulation_settings` keeps `gate` and `attack` and gains nothing: a per-mark settings table
would need a second and third setting to justify its shape, and prompt 71 (grace notes) is where those appear.
Generalizing it now would be designing against one example.

### The claim is measured, not asserted

The prompt's justification is a cost, so **record the cost**. Before: the files that must change to add `portato`.
After: the same count. Write both numbers into this prompt's `Repairs made while implementing` section.

If the number after is greater than two, the table did not pay for itself and this prompt should be struck rather than
shipped — say so, and leave the enum. A refactor that cannot show its saving is decoration.

### What must not happen

The table is a *notation* vocabulary. It must not grow a `gate`, `amplitude`, `steal`, or any other column describing
what a mark *does*, because that is the profile's job and §2's line is the one this repository most consistently
defends. If a column starts to look like performance, it belongs in a profile.

## Target

- `crates/musa-compiler/src/marks.rs` (new): `MarkDef`, `Placement`, `Param`, `ParamTy`, `Mark`, `MarkArg`, `VOCABULARY`
  (five rows), `lookup`.
- `crates/musa-compiler/src/score.rs`: `ArticulationMark` **deleted**; `Note`/`Rest` facts carry `Vec<Mark>`.
- `crates/musa-compiler/src/{resolve,elaborate,factext,performance,project,profile}.rs`: every match arm replaced by a
  lookup; `suggest` fed from `VOCABULARY` instead of `NAMES`.
- `crates/musa-render/src/{plan,ly,mei,musicxml}.rs`: one `Placement::Attached` emitter each, reading the backend
  column.
- `crates/musa-compiler/tests/marks.rs`: every vocabulary row round-trips through `lookup`; an unknown mark produces
  prompt 56's diagnostic with a suggestion; the table's names are unique.

## Check

```sh
cargo nextest run --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cd apps/musa-desktop/ui && npm test
for f in examples/*.musa; do cargo run -q -p musa -- check "$f"; done
grep -rn "ArticulationMark\b" crates --include="*.rs" | grep -v marks.rs | wc -l   # 0
git diff --stat -- examples/ crates/*/tests/snapshots              # see the repair below
cargo run -p musa -- check examples/broken/unknown-mark.musa
```

Commit as `Make the mark vocabulary a table`.

## Repairs made while implementing

**The claim, measured.** Adding `portato` before this prompt touched **four files**: `score.rs` (an enum variant, a
`NAMES` entry, a `parse` arm and a `name` arm), `mei.rs` (one arm), `ly.rs` (two — one per stem direction), and
`musicxml.rs` (one). After, it touches **one**: a row in `marks.rs`. The parser needed no edit either way, because
`parser.rs::articulations` has accepted a greedy list of bare identifiers since prompt 27 — the plan that produced this
prompt said six files, and six was wrong. Four to one is still well inside the prompt's own "strike it if the count
after is above two".

**`Placement`, `Param`, `ParamTy`, `params` and `args` were not written.** The Design specifies them; none has a
producer in this prompt, and the module-design rule that prompt 47 applied to `Term` applies here unchanged. Two facts
make deferring them free rather than merely cheaper:

- an empty argument list serializes identically to no argument list, so prompt 70 adds `args` with zero golden movement;
- a `Placement` with one variant carries no information, and the emitter it would select is the only emitter there is.

`Mark` is therefore `&'static MarkDef` — a row, not a name plus arguments — which also makes "what does MEI call this"
total rather than fallible.

**Backend columns are `&'static str`, not `Option<&'static str>`.** All five marks exist in all three formats, so the
`None` case and its export warning have no producer and could not be tested. Widening the field is a one-line change in
one file at the moment prompt 70 adds a row that needs it, which is exactly the cost the table exists to make cheap. The
warning is prompt 70's, stated there rather than shipped dead here.

**Six goldens moved, and each movement is a decision this prompt was told to make.**

- `examples/profile-fixture.musa` and `examples/kernel/profile-fixture.kernel`: the Design renames the profile rule head
  `articulation` to `mark`, which is eight characters shorter, so every provenance span after the first rule shifts. The
  occurrence *keys* are unchanged; only byte offsets moved. Regenerated with `UPDATE_KERNEL_GOLDENS=1` and read line by
  line.
- `crates/musa-compiler/tests/snapshots/profile_laws__profile_fixture.snap`: the same span shift, same fixture.
- `crates/musa-compiler/tests/snapshots/notation_details_laws__tuplet_fixture.snap` and
  `crates/musa-render/tests/snapshots/plan__tuplet_fixture.snap`: `Accent` became `Mark("accent")`. `Mark`'s *derived*
  `Debug` printed the whole vocabulary row, which would have made an annotation snapshot move whenever a *backend*
  spelling changed — a fact about the exporter, not about the score being snapshotted. The hand-written `Debug` prints
  the name alone, and that is what the goldens now hold. These are debug renderings; the rendered MEI, LilyPond and
  MusicXML for the same fixture did not move.

- `apps/musa-desktop/ui/src/lib/session/generated/spellings.json` and `fixtures/lexed/profile-fixture.json`: both are
  generated from the real lexer by prompt 26's rule, so the keyword list gained `mark` in place of `articulation` and
  the fixture's token offsets shifted with the source. Regenerated with `UPDATE_UI_FIXTURES=1`; no other lexed example
  moved.

MEI, LilyPond, MusicXML, MIDI, the notation plans, and every other UI fixture are byte-identical, which is the part of
the promise that was about behaviour.

**The rename reached further than the profile head.** `is not an articulation` became `is not a mark` in both places a
name is refused — the profile rule and the mark attached to a note — because a vocabulary that will hold `pedal` and
`sample` cannot call every row an articulation. `examples/broken/unknown-articulation.musa` is therefore
`unknown-mark.musa`, and `PerformanceProfile::set_articulation` is `set_mark`. The profile's *settings* keep their
names: `gate` and `attack` describe what an instrument does with a mark, and that is still articulation. The new fixture
brings a new golden, `crates/musa/tests/snapshots/cli__unknown-mark.snap`, which is the diagnostic in full — the refusal
names the mark, points at it, and suggests `staccato`.

## Stop

- No new marks. Five in, five out. Prompt 70 adds the vocabulary rows.
- No `mark` statement in the grammar, no `Point` or `Span` producer, no span emitter — prompt 70.
- No `smufl` column. Verovio resolves glyphs from MEI; a SMuFL codepoint would have no consumer.
- No per-mark profile settings beyond `gate` and `attack` — prompt 71.
- No performance column in the table, ever. §2.
- No mark on a chord member rather than the chord. That is a real question and it is Q7's, not this prompt's.
