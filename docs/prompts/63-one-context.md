---
id: 63
slug: one-context
status: pending
depends_on: [40, 61]
phase: 3
---

# One Context, Four Questions

## Task

Musa answers "what key is this", "what meter is this", "what clef is this", and "how fast is this" with four
different mechanisms: `KeyMap` (a scalar), `MeterMap` (a scalar), `Part::clef` (metadata on the part), and
`TempoMap` (a piece-level singleton). All four are the same question — *what is in force here* — and none of them can
say "here", because none of them has a location.

Give them one shape: a **scoped fact with a real span**, projected into one boundary list per kind. Delete the four
mechanisms. Change nothing a composer can observe.

This is the last of the three refactors, and the largest. It is what makes prompts 64, 65, 72, 73, 74 and 75 each a
small prompt instead of each a redesign.

## Read

- Prompt 40, which made key, meter, sections and harmony into occurrences — and then projected them straight back
  into scalars. `elaborate.rs::context_facts` (:576) is where that happens; note that it spans `Span::new(Beat::ZERO,
  extent)`, so the occurrence carries a location it does not mean.
- `crates/musa-compiler/src/elaborate.rs::Scope` (:52) — two variants, `Piece` and `Voice`. There is no `Part`, and
  it is `pub(crate)`.
- `crates/musa-compiler/src/score.rs` — `KeyMap` (:404), `MeterMap` (:358), `Part::clef` (:216, :233),
  `TempoMap` (:336) and its `changes` field, which nothing ever populates (`resolve.rs::parse_tempo` :761 writes
  `Vec::new()` unconditionally).
- `crates/musa-kernel/src/timeline.rs` — `covering` (:302) and `prevailing` (:323), prompt 44's queries. Read
  `prevailing`'s documentation before assuming it answers this prompt's question; the Design section says what it
  does and does not give you.
- `crates/musa-compiler/src/resolve.rs::part_metadata` (:701) — clef is read here and last-wins **silently**.
- `crates/musa-project/src/session.rs` (:387) — the MIDI speller spells against the *piece's* key.
- `docs/course-correction.md` §2 (ambient time), §22 (tempo is `Beat → Second`), §34 (semantic necessity).

## Design

### The one idea

A context is not a property of a piece. It is a fact that holds **from somewhere, in some scope, until something
replaces it** — which is precisely an occurrence with a span, in the kernel musa already has. Prompt 40 saw this and
made them occurrences; what it did not do was let the span mean anything, because every consumer downstream still
wanted a scalar and there was nowhere to put the answer.

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

`is_constant` is not a convenience. It is what lets MEI, LilyPond and MusicXML keep their present output
byte-for-byte: a track that never changes emits exactly what a scalar emitted, on the same code path, and the
mid-piece path is dead until prompt 64 makes it live.

### Inheritance is per kind, and that is the part a naive design gets wrong

A single "innermost scope wins, then latest time" rule is wrong for key, and the counterexample is ordinary: a viola
part written in a different key from the piece must not be dragged back by a piece-level key signature at bar 60,
but it *must* follow the piece's modulations. Clef is the opposite — a part's clef is the part's business and no
piece-level clef should ever reach it.

| Kind | Rule | Why |
| --- | --- | --- |
| Clef | `Override` — the innermost scope with any answer wins outright | a part's clef is the part's business |
| Meter | `Override` | this *is* polymeter: a 7/8 lane does not rejoin 4/4 at the piece's next barline |
| Tempo | `Override` | likewise polytempo |
| Key | `Latest` — flatten the whole scope chain, time order decides | a part follows the piece's modulations |

Declared **once**, as a table in `scope.rs`, next to `Scope` itself. Not inferred from the fact kind at each call
site, and not a parameter callers pass — every caller would pass the same value, which is the signature that the
default belongs inside.

### `Scope` moves and grows a `Part`

`Scope` is `pub(crate)` in `elaborate.rs` and appears in `ScoreFact`, which is public. That is already a leak;
this prompt makes it load-bearing, because a clef is scoped to a *part* and there is no such variant. Move it to
`crates/musa-compiler/src/scope.rs` with `Piece`, `Part { part }`, `Voice { part, voice }`, plus:

```rust
impl Scope {
    /// This scope and every scope containing it, innermost first.
    pub fn chain(self) -> impl Iterator<Item = Scope>;
    /// Whether `inner` is this scope or inside it.
    pub fn contains(self, inner: Scope) -> bool;
}
```

`score.rs` must not depend on `elaborate.rs`; that is the concrete reason for the move, and it is worth more than
the tidiness.

### Spans stop lying

A context occurrence currently spans `[0, extent)` regardless of where it was written. Change it to span **from
where it is written until the next occurrence of the same kind in the same scope**, or to the extent if there is
none. Today that is the same span, because there is always exactly one — so the kernel goldens do not move, and the
`.kernel` corpus keeps saying what it said. When prompt 64 lets a composer write a second one, the file is already
correct.

The one exception is `clef`: a part that declares one gains an occurrence it did not have, so
`examples/kernel/*.kernel` moves for those pieces. That diff is reviewed by hand and is the only golden movement
this prompt is allowed.

### `prevailing` gets real work, and the honest accounting

It would be convenient to say this prompt finally justifies prompt 44's `prevailing`. Half of that is true. Every
*exporter* needs boundaries, which `prevailing` does not give — that is what `ContextTrack` is for, and pretending
otherwise would mean calling `prevailing` once per instant in a scan. The honest new callers are the ones that ask
about **one** instant:

- `session.rs`'s MIDI speller, which spells an incoming note against `self.valid.score.key()` — the *piece's* key.
  With a `ContextTrack` it spells against the key in force at the caret, which is a **live wrong answer being
  fixed**, not a refactor.
- The interface's point questions: what the inspector shows for the selected event, what the Origin view reports.

Two real callers is enough. Do not add a third by rewriting a sweep as a query.

### What this prompt must not disturb

`plan.rs`'s `Fold` (prompt 58) and `elaborate.rs`'s `Share` (prompt 49) both interact with context, and both are
prompt 64's problem: a shared body under two meters, and a folded repeat crossing a meter change. Write the
invariants into `context.rs`'s module documentation — a context statement inside a motif or bar body is forbidden,
because a motif is *material* and a context change is *a place in the piece* (§2: motif definition ≠ its
expansions). Nothing can violate it yet; prompt 64 is where it starts earning its keep.

### The bug this fixes on the way past

`TempoMap` is not in the timeline, so a tempo-only edit does not move the semantic hash, so
`session.rs::install_current_plan` does not reinstall the plan and playback keeps the old tempo. Moving the tempo
*marking* into the timeline fixes it. That fix belongs to prompt 72, which is where the marking becomes a fact —
this prompt only builds the shape and states the bug so 72 has something to point at.

## Target

- `crates/musa-compiler/src/scope.rs` (new): `Scope` with `Part`, `chain`, `contains`, and the inheritance table.
- `crates/musa-compiler/src/context.rs` (new): `ContextTrack<V>`, `at`, `changes`, `is_constant`; the `Fold`,
  `Share`, and motif-body invariants in the module doc.
- `crates/musa-compiler/src/score.rs`: `ScoreSnapshot` gains `key_at`, `meter_at`, `clef_at`, and the matching
  `ContextTrack` accessors; `KeyMap`, `MeterMap`, `Part::clef`, and `TempoMap::changes` deleted.
- `crates/musa-compiler/src/elaborate.rs`: `context_facts` emits real spans; `FactKind::Clef`.
- `crates/musa-compiler/src/resolve.rs`: `part_metadata`'s silent last-wins becomes a duplicate-declaration
  diagnostic under prompt 56's machinery, with the first declaration as the secondary label.
- `crates/musa-render/src/{plan,ly,mei,musicxml}.rs`, `crates/musa-project/src/{facts,midi,session}.rs`: every
  scalar read replaced; `is_constant` guards the existing emission path.
- `crates/musa-compiler/tests/context.rs`: the four inheritance rules, each with the counterexample that motivates
  it; `changes()` on a constant track yields exactly one entry.

## Check

```sh
cargo nextest run --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
cd apps/musa-desktop/ui && npm test
grep -rn "KeyMap\|MeterMap" crates apps --include="*.rs" | wc -l    # 0
git diff --stat -- crates/*/tests/snapshots apps/musa-desktop/ui/fixtures   # empty
git diff --stat -- examples/kernel/                                  # only pieces declaring a clef
cargo bench -p musa-compiler                                         # P1-P5, no regression
```

Commit as `Unify key, meter, and clef as scoped context`.

## Stop

- No grammar. A composer still cannot write a second `key`, `meter`, or `clef` — prompt 64.
- No tempo fact. The shape is built here; `TempoMap` keeps its scalar until prompt 72.
- No polymeter, no polytempo. `Scope::Voice` can *hold* a meter; nothing produces one.
- No `senza misura` (prompt 74) and no proportional notation. `ContextTrack` must not gain an "unmeasured" variant
  here.
- Do not make `ContextTrack` public from `musa-kernel`. It is a projection of a timeline, not an operation on one;
  the kernel gains nothing in this prompt.
- No interface work. The inspector keeps showing the piece's key until prompt 64 gives it a second one to show.
