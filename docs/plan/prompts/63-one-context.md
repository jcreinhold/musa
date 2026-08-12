---
id: 63
slug: one-context
status: done
depends_on: [40, 61]
phase: 3
---

# One Context, Four Questions

## Task

Musa answers "what key is this", "what meter is this", "what clef is this", and "how fast is this" with four different
mechanisms: `KeyMap` (a scalar), `MeterMap` (a scalar), `Part::clef` (metadata on the part), and `TempoMap` (a
piece-level singleton). All four are the same question — *what is in force here* — and none of them can say "here",
because none of them has a location.

Give them one shape: a **scoped fact with a real span**, projected into one boundary list per kind. Delete the four
mechanisms. Change nothing a composer can observe.

This is the last of the three refactors, and the largest. It is what makes prompts 64, 65, 72, 73, 74 and 75 each a
small prompt instead of each a redesign.

## Read

- Prompt 40, which made key, meter, sections and harmony into occurrences — and then projected them straight back into
  scalars. `elaborate.rs::context_facts` (:576) is where that happens; note that it spans `Span::new(Beat::ZERO,
  extent)`, so the occurrence carries a location it does not mean.
- `crates/musa-compiler/src/elaborate.rs::Scope` (:52) — two variants, `Piece` and `Voice`. There is no `Part`, and it
  is `pub(crate)`.
- `crates/musa-compiler/src/score.rs` — `KeyMap` (:404), `MeterMap` (:358), `Part::clef` (:216, :233), `TempoMap` (:336)
  and its `changes` field, which nothing ever populates (`resolve.rs::parse_tempo` :761 writes `Vec::new()`
  unconditionally).
- `crates/musa-kernel/src/timeline.rs` — `covering` (:302) and `prevailing` (:323), prompt 44's queries. Read
  `prevailing`'s documentation before assuming it answers this prompt's question; the Design section says what it does
  and does not give you.
- `crates/musa-compiler/src/resolve.rs::part_metadata` (:701) — clef is read here and last-wins **silently**.
- `crates/musa-project/src/session.rs` (:387) — the MIDI speller spells against the *piece's* key.
- `docs/rules/kernel/00-purpose.md` (ambient time), §22 (tempo is `Beat → Second`), §34 (semantic necessity).

## Design

### The one idea

A context is not a property of a piece. It is a fact that holds **from somewhere, in some scope, until something
replaces it** — which is precisely an occurrence with a span, in the kernel musa already has. Prompt 40 saw this and
made them occurrences; what it did not do was let the span mean anything, because every consumer downstream still wanted
a scalar and there was nowhere to put the answer.

This prompt builds the somewhere to put it.

```rust
// crates/musa-compiler/src/context.rs

/// What is in force, and from where, in one scope.
///
/// Consumers need *boundaries*, not points. "Where does the meter change" is
/// what every exporter asks, and neither D10 (`covering`) nor D11
/// (`prevailing`) answers it — they answer about an instant. This type is the
/// projection that turns a timeline of context facts into the shape its
/// consumers actually read.
pub struct ContextTrack<V> { /* ascending, disjoint, non-empty stretches */ }

impl<V> ContextTrack<V> {
    /// What is in force at `at`.
    pub fn at(&self, at: MusicalTime) -> &V;
    /// Every change, in time order, starting with the value at time zero.
    pub fn changes(&self) -> impl Iterator<Item = (MusicalTime, &V)> + '_;
    /// True when nothing ever changes — the case every exporter has a fast path for.
    pub fn is_constant(&self) -> bool;
}
```

`is_constant` is not a convenience. It is what lets MEI, LilyPond and MusicXML keep their present output byte-for-byte:
a track that never changes emits exactly what a scalar emitted, on the same code path, and the mid-piece path is dead
until prompt 64 makes it live.

### Inheritance is per kind, and that is the part a naive design gets wrong

A single "innermost scope wins, then latest time" rule is wrong for key, and the counterexample is ordinary: a viola
part written in a different key from the piece must not be dragged back by a piece-level key signature at bar 60, but it
*must* follow the piece's modulations. Clef is the opposite — a part's clef is the part's business and no piece-level
clef should ever reach it.

| Kind | Rule | Why |
| --- | --- | --- |
| Clef | `Override` — the innermost scope with any answer wins outright | a part's clef is the part's business |
| Meter | `Override` | this *is* polymeter: a 7/8 lane does not rejoin 4/4 at the piece's next barline |
| Tempo | `Override` | likewise polytempo |
| Key | `Latest` — flatten the whole scope chain, time order decides | a part follows the piece's modulations |

Declared **once**, as a table in `scope.rs`, next to `Scope` itself. Not inferred from the fact kind at each call site,
and not a parameter callers pass — every caller would pass the same value, which is the signature that the default
belongs inside.

### `Scope` moves and grows a `Part`

`Scope` is `pub(crate)` in `elaborate.rs` and appears in `ScoreFact`, which is public. That is already a leak; this
prompt makes it load-bearing, because a clef is scoped to a *part* and there is no such variant. Move it to
`crates/musa-compiler/src/scope.rs` with `Piece`, `Part { part }`, `Voice { part, voice }`, plus:

```rust
impl Scope {
    /// This scope and every scope containing it, innermost first.
    pub fn chain(self) -> impl Iterator<Item = Scope>;
    /// Whether `inner` is this scope or inside it.
    pub fn contains(self, inner: Scope) -> bool;
}
```

`score.rs` must not depend on `elaborate.rs`; that is the concrete reason for the move, and it is worth more than the
tidiness.

### Spans stop lying

A context occurrence currently spans `[0, extent)` regardless of where it was written. Change it to span **from where it
is written until the next occurrence of the same kind in the same scope**, or to the extent if there is none. Today that
is the same span, because there is always exactly one — so the kernel goldens do not move, and the `.kernel` corpus
keeps saying what it said. When prompt 64 lets a composer write a second one, the file is already correct.

The one exception is `clef`: a part that declares one gains an occurrence it did not have, so `examples/kernel/*.kernel`
moves for those pieces. That diff is reviewed by hand and is the only golden movement this prompt is allowed.

### `prevailing` gets real work, and the honest accounting

It would be convenient to say this prompt finally justifies prompt 44's `prevailing`. Half of that is true. Every
*exporter* needs boundaries, which `prevailing` does not give — that is what `ContextTrack` is for, and pretending
otherwise would mean calling `prevailing` once per instant in a scan. The honest new callers are the ones that ask about
**one** instant:

- `session.rs`'s MIDI speller, which spells an incoming note against `self.valid.score.key()` — the *piece's* key. With
  a `ContextTrack` it spells against the key in force at the caret, which is a **live wrong answer being fixed**, not a
  refactor.
- The interface's point questions: what the inspector shows for the selected event, what the Origin view reports.

Two real callers is enough. Do not add a third by rewriting a sweep as a query.

### What this prompt must not disturb

`plan.rs`'s `Fold` (prompt 58) and `elaborate.rs`'s `Share` (prompt 49) both interact with context, and both are prompt
64's problem: a shared body under two meters, and a folded repeat crossing a meter change. Write the invariants into
`context.rs`'s module documentation — a context statement inside a motif or bar body is forbidden, because a motif is
*material* and a context change is *a place in the piece* (§2: motif definition ≠ its expansions). Nothing can violate
it yet; prompt 64 is where it starts earning its keep.

### The bug this fixes on the way past

`TempoMap` is not in the timeline, so a tempo-only edit does not move the semantic hash, so
`session.rs::install_current_plan` does not reinstall the plan and playback keeps the old tempo. Moving the tempo
*marking* into the timeline fixes it. That fix belongs to prompt 72, which is where the marking becomes a fact — this
prompt only builds the shape and states the bug so 72 has something to point at.

## Target

- `crates/musa-compiler/src/scope.rs` (new): `Scope` with `Part`, `chain`, `contains`, and the inheritance table.
- `crates/musa-compiler/src/context.rs` (new): `ContextTrack<V>`, `at`, `changes`, `is_constant`; the `Fold`, `Share`,
  and motif-body invariants in the module doc.
- `crates/musa-compiler/src/score.rs`: `ScoreSnapshot` gains `key_at`, `meter_at`, `clef_at`, and the matching
  `ContextTrack` accessors; `KeyMap`, `MeterMap`, `Part::clef`, and `TempoMap::changes` deleted.
- `crates/musa-compiler/src/elaborate.rs`: `context_facts` emits real spans; `FactKind::Clef`.
- `crates/musa-compiler/src/resolve.rs`: `part_metadata`'s silent last-wins becomes a duplicate-declaration diagnostic
  under prompt 56's machinery, with the first declaration as the secondary label.
- `crates/musa-render/src/{plan,ly,mei,musicxml}.rs`, `crates/musa-project/src/{facts,midi,session}.rs`: every scalar
  read replaced; `is_constant` guards the existing emission path.
- `crates/musa-compiler/tests/suite/context.rs`: the four inheritance rules, each with the counterexample that motivates
  it; `changes()` on a constant track yields exactly one entry.

## Check

```sh
cargo nextest run --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cd apps/musa-desktop/ui && npm test
for f in examples/*.musa; do cargo run -q -p musa -- check "$f"; done
grep -rn "KeyMap\|MeterMap" crates apps --include="*.rs" | wc -l    # 0
git diff --stat -- crates/*/tests/snapshots apps/musa-desktop/ui/fixtures   # empty
git diff --stat -- examples/kernel/                                  # only pieces declaring a clef
cargo bench -p musa-compiler                                         # P1-P5, no regression
```

## Repairs made while implementing

**`TempoMap::changes` is not dead, and it stays.** The Read section said nothing populates it, citing
`resolve.rs::parse_tempo`'s unconditional `Vec::new()`. `elaborate.rs::elaborate_tempo_changes` fills it in, from
`tempo quarter = 96 at 9:1;` — a feature with examples and tests. Deleting it would have deleted mid-piece tempo to tidy
a field. Struck from the Target; the tempo *marking* still becomes a fact in the prompt that makes it one.

**`ContextTrack` may be empty, and `at` returns an `Option`.** The Design gave it non-empty stretches and a total `at`.
A piece that names no key has none, and the only way to keep `at` total would be to invent C major at the one place the
code is entitled to say "nothing has been said" — which is the error this shape exists to *stop* inventing. `meter_at`
is where the default is real (4/4 governs a piece that never says so), so the unwrap happens there, once, in the
accessor whose documentation states it.

**`Inheritance` is private, and callers name a `ContextKind` instead.** The Design said the rule must not be a parameter
callers pass. The strongest form of that is that the enum naming the rules is not public at all: a track is constructed
from its *kind*, carries the kind, and reads the table itself. The public surface is three variants — `Key`, `Meter`,
`Clef` — and no rule at all. `Tempo` is not a row, because no track is built from it and a rule nothing can exercise is
a rule nothing can test.

**The inheritance tests are unit tests in `context.rs`, not `tests/context.rs`.** Every counterexample the table exists
for — a part in its own key, a lane in its own meter, a piece-level clef that must not reach a part — is unwritable in
`.musa`: there is no grammar for a part-scoped key, a second meter, or a piece-level clef. An integration test would
therefore have required making `ContextTrack::state` public with no non-test caller, which is the "public item for
future use" the module rules forbid. The counterexamples are all present; they are next to the code they constrain.

**`prevailing` gained no caller, and the MIDI-speller bug named in the Design does not exist yet.**
`ProjectSession::midi_entry` takes no position — the step-entry state holds pressed notes, not a place — so there is no
caret to spell against, and `session.rs` asks `key_at(Scope::Piece, MusicalTime::ZERO)`, which is the answer it already
had. Nor could it be wrong today: a key cannot change until the grammar can write a second one. The honest accounting is
that this prompt built the shape and found that the second honest caller the Design promised was not there; the prompt
that gives note entry a position is where it arrives.

**The header's readings moved to the `Resolver`, and `set_meter`/`take_meter`/`set_key`/`take_key` are gone.** The
snapshot used to hold the header's key and meter until the projection overwrote them — a second answer to "what key is
this" with a live window in which it was the one anybody reading the snapshot would get. Staging them on the resolver
leaves the snapshot with exactly one answer, which is the projection.

**Three small things the design implied but did not name.** `Clef::name` was added as the inverse of `Clef::parse`, so a
clef crosses the interchange file as the word a composer typed rather than as an ordinal. `Scope::Part` keys canonically
as `{part}|*|`, which sorts a part's clef before that part's voices at the same instant — where a reader meets it. And
`plan_staff` now takes the snapshot, because a staff's clef is a question about the score rather than a field on the
part.

**P3 costs 4–6% more, and that is the honest number.** Benchmarked against `HEAD` in a `git worktree`, same machine,
same session. Medians, base → change: P1 `large` 1.91 → 1.87 ms, `shared` 1.79 → 1.79 ms; P2 `large` 1.80 → 1.79 ms,
`shared` 1.77 → 1.72 ms; P4 `large` 383 → 387 µs; P5 `large` 641 → 645 µs — all inside run-to-run noise. **P3 `large`
223 → 232 µs, `shared` 248 → 263 µs, `small` 3.21 → 3.40 µs.** The projection is the pass this prompt changed: it now
builds three tracks and pushes a stretch per context fact instead of assigning two scalars. Reported rather than
absorbed into "no regression", because it is a real cost and it is where a later prompt would look. It buys nothing back
yet and buys a great deal at prompt 64. Against B1's 120 ms budget for a whole compile, P1 `large` is 1.87 ms.

**The golden movement is exactly what the prompt allowed, and no more.** Seven `examples/kernel/*.kernel` files gained
one `clef` occurrence per part that declares a clef — additions only, no line changed. Four compiler debug snapshots
moved, because `ScoreSnapshot`'s shape is what they print. **MEI, LilyPond, MusicXML, MIDI, WAV, every notation-plan
snapshot, every CLI golden and every UI fixture are byte-identical**, which is the claim that was about behaviour.

Commit as `Unify key, meter, and clef as scoped context`.

## Stop

- No grammar. A composer still cannot write a second `key`, `meter`, or `clef` — prompt 64.
- No tempo fact. The shape is built here; `TempoMap` keeps its scalar until prompt 72.
- No polymeter, no polytempo. `Scope::Voice` can *hold* a meter; nothing produces one.
- No `senza misura` (prompt 74) and no proportional notation. `ContextTrack` must not gain an "unmeasured" variant here.
- Do not make `ContextTrack` public from `musa-kernel`. It is a projection of a timeline, not an operation on one; the
  kernel gains nothing in this prompt.
- No interface work. The inspector keeps showing the piece's key until prompt 64 gives it a second one to show.
