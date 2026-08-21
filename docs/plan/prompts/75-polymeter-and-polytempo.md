---
id: 75
slug: polymeter-and-polytempo
status: done
depends_on: [64, 72, 74]
phase: 3
---

# Polymeter and Polytempo

## Task

Two parts in different time signatures at once, and — separately — two parts at different tempos at once. Prompt 63's
`Scope::Voice` can already hold a meter and a tempo; prompt 64 and prompt 72 declined to produce one. This prompt
produces them, or declines a second time with a reason.

**This prompt has a gate, and it must be passed before the prompt is run.** See below. **It was passed**, and both
halves shipped; the reasoning is recorded under "The gate", where the decline would have gone.

## The gate

Polymeter has evidence: Balkan and Bulgarian folk ensembles play 7/8 against 4/4 as a matter of course, Reich's phase
pieces are built on it, and Afro-Cuban and West African ensemble music is layered by construction. It also has consumers
— MEI, MusicXML and LilyPond all express it.

**Polytempo does not.** It is not in `docs/rules/kernel/08-open-questions.md`'s falsification corpus, no fixture needs
it, and its export story is bad. Under §34 the burden is semantic necessity, so before any code:

1. Name a real piece in this prompt's Task section that musa should be able to write and cannot without it — Nancarrow's
   player-piano studies, Ives's *Fourth Symphony*, Lutosławski's *ad libitum* sections, or Carter — and add it to §33 as
   a row.
2. Name the consumer that reads it and does something different because of it.
3. If either cannot be named, **strike the polytempo half of this prompt** and ship polymeter alone. That is a
   legitimate outcome and the prompt should say so rather than treating it as failure.

Recording this gate is worth as much as passing it. Prompt 66 declined `choose` on the same reasoning, and the pattern —
a capability that the architecture makes easy is not thereby justified — is the one this repository most needs to keep
applying.

### How the gate was answered

**1. The piece.** Nancarrow's *Study No. 21*, "Canon X": one voice begins slow and accelerates throughout, the other
begins fast and decelerates, and they cross in the middle. It is the shape the study is named after, and it is not
writable with one clock — a *rit.* over everything is a different piece, and there is no tuplet, no metric modulation
and no meter that produces it. Ives's *Fourth Symphony* II is the ensemble version of the same fact: it is scored for
two conductors because one is not enough. Added to `docs/rules/kernel/08-open-questions.md` as item 11, and written as
`examples/canon-x.musa`.

**2. The consumers, and what each does differently.** Three, of which two are outside musa:

- **musa's performance layer** builds one `IntegratedTempoMap` per part and schedules each lane against its own, so the
  two parts of *Canon X* are played at different speeds and cross where the study crosses.
- **MusicXML** carries a `<direction>` inside a part, so each part's marking is written into that part and read there.
  This is the format's own mechanism, not a musa convention.
- **MEI** carries `<tempo staff="n">`, so each staff's marking is attached to its staff.

**3. What it cost, which is the honest part.** SMF has one tempo track and no scope, so the MIDI export resolves every
lane against its own map, writes every note at the frame it actually sounds, and states the piece's tempo. Sonically
exact and notationally wrong, reported as an export warning and written into `docs/rules/kernel/07-backend-contract.md`
rather than left to be discovered.

The gate passed on the strength of the second criterion more than the first. A capability with a real piece behind it
and no consumer would have been struck — that is what the gate is for — and the deciding fact was that two of the three
interchange formats attach a tempo to a part or a staff *in the format*, which means polytempo is something musa can
hand to another program rather than something only musa knows.

## Read

- Prompt 63 §"Inheritance is per kind" — meter and tempo are both `Override`, which is what makes per-voice work with no
  new rule.
- Prompt 61 `bars.rs` — `BarLines` is per-piece today. Polymeter makes it per-scope, which is the real change.
- `crates/musa-notation/src/plan.rs::plan_staff` (:820) — the measure walk, which assumes one barline grid.
- `crates/musa-engine/src/playback.rs` (:90) — the frame merge. Read it before worrying about polytempo's engine cost;
  the news is good (see below).
- `docs/rules/kernel/08-open-questions.md` and §34.

## Design

### Polymeter: `BarLines` becomes per-scope

`ScoreSnapshot::bars()` becomes `bars(scope)`, resolved through the inheritance chain. A voice with its own meter gets
its own barline grid; every other voice inherits the piece's. Prompt 61 built `at`/`time_of` as inverses on an arbitrary
meter sequence precisely so this would be a change of *argument*, not of algorithm.

Two shapes, and both must work:

- **Same bar duration, different grouping** — 3+2+2 against 4/4. One barline grid, different beaming. This is most
  Balkan music and it is nearly free.
- **Different bar durations** — 7/8 against 4/4, where barlines genuinely diverge and only realign every 56 eighths.
  This is the one that needs per-scope `BarLines`, and it is what `plan_staff` must learn.

The engraver draws each staff its own barlines. `docs/rules/desktop/` must agree that a system may have staves whose
barlines do not align, because today's visual language assumes they do — a repair to that document, in this commit.

### Polytempo, if the gate passes

Per-voice tempo means per-voice `IntegratedTempoMap`, so `PerformancePlan` stops holding one shared map and holds one
per lane.

The good news, which the prompt should verify early rather than fear: **the engine needs no change.** Frames are
absolute by the time they reach `playback.rs`, and beat time never crosses into it, so lanes at different tempos merge
exactly as lanes at the same tempo do. `lower_performance` does all the work.

The bad news is export. **MIDI has one tempo track.** Polytempo is therefore exported by resolving every lane against a
single reference tempo and emitting the resulting absolute times — which is **sonically exact** and **notationally
lossy**: the file plays correctly and its notated tempo is wrong. Say exactly that in a warning, and in
`07-backend-contract.md`; do not let it be discovered.

MusicXML and MEI express per-staff tempo directions. LilyPond needs separate `\score` blocks or polymetric
`\scaleDurations`, which is a documented approximation.

### What must not follow

Per-voice *key* — polytonality — is the same shape and is **not** in this prompt. It has its own repertoire, its own
spelling consequences (the MIDI speller would need a scope), and no fixture. Adding it because the machinery allows it
is the exact move the gate exists to prevent.

## Target

- `crates/musa-compiler`: `bars(scope)`; per-scope meter resolution; per-lane tempo maps in `PerformancePlan` if the
  gate passes.
- `crates/musa-notation/src/plan.rs`: per-staff barline grids; per-staff beaming.
- `crates/musa-project/src/midi.rs`: the single-tempo-track resolution and its warning.
- `docs/rules/desktop/`: non-aligned barlines within a system.
- `docs/rules/kernel/08-open-questions.md`: the polymeter rows proven; the polytempo row added or the decline recorded.
- `examples/`: `bulgarian.musa` (7/8 against 4/4), `hemiola.musa` (6/8 against 3/4 — see repair 9). Polytempo's fixture
  only if the gate passes: `canon-x.musa`.

## Check

```sh
cargo nextest run --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
cd apps/musa-desktop/ui && npx pnpm run test:unit
cargo run -p musa -- check examples/bulgarian.musa
cargo run -p musa -- render examples/bulgarian.musa --to musicxml -o /dev/stdout | grep -c '<time>'   # 2
cargo bench -p musa-compiler        # per-scope BarLines must not regress P1-P3
                                    # measured: P1 large 2.29 ms, P2 2.01 ms, P3 241 µs — `large-score.musa`
                                    # declares no part meter, so `part_meters` is empty and the checks take
                                    # the piece's barlines by the same path they took before.
```

Commit as `Add polymeter` — or `Add polymeter and polytempo`, per the gate.

## Repairs made while implementing

1. **A part's meter and tempo are written in the part block**, beside its `clef`, and are in force for the whole part.
   The prompt's Design says "a voice with its own meter gets its own barline grid"; that granularity is unusable,
   because a staff has one set of barlines in MEI, in MusicXML and on paper, so a per-voice grid would be a document
   nothing can draw. `Scope::Voice` still resolves correctly — the machinery is unchanged — and the grammar simply does
   not produce one.

2. **`bars(scope)` is the whole of the compiler change**, as the prompt predicted, and the prediction was worth testing:
   `ScoreSnapshot::bars` gained a parameter and its body changed by one identifier. The reason is
   `ContextTrack::changes(scope)` and the `Override` row of `scope.rs`'s table, both of which predate this prompt.

3. **`part_metadata` became `part_context`**, returning a struct rather than a tuple. Four answers of which three are
   optional is a shape a caller mistakes; two were tolerable.

4. **The bar checks needed to know whose barlines.** `PendingBar` gained the scope it was written in;
   `check_measure_sanity`, `check_groove_has_a_meter` and `check_tuplets` moved their `bars` inside the part loop.
   `check_tuplets` needed an event → part index, because a tuplet names its first event and there is no other way back
   to its staff.

5. **`check_keys` and `resolve_position` stayed piece-scoped.** A modulation is checked against the piece's barlines and
   a `9:1` coordinate means the piece's measure 9. Both are piece-wide facts; making them per-part would give two
   answers to a question with one.

6. **Exactly one of `NotationPlan::tempos` and `StaffPlan::tempos` is populated.** The first draft filled both and the
   first staff printed two tempos. The invariant is the fix: the piece's markings when every part agrees, each staff's
   when they do not — because under polytempo the piece's tempo is a reading no staff plays, and printing it over a
   staff that plays something else is a page that lies. Backends print both lists and need no rule.

7. **MEI's measure count became the longest staff's, not the first's.** With divergent barlines the first staff is not
   the longest, and taking it dropped the tail of the piece rather than mis-spacing it.

8. **The MIDI warning lives in `musa-project/src/playback.rs`, not `midi.rs`.** The prompt named
   `crates/musa-project/src/midi.rs`, which is MIDI *input* — the step-entry buffer. `to_midi` is where the file is
   produced, so it is where the loss is stated; it now returns the bytes and the warnings together, and
   `ProjectSession::export` passes them on like every other target's.

9. **`examples/phase.musa` became `examples/hemiola.musa`.** The prompt asked for "Reich's shape" for the
   same-bar-duration case. *Piano Phase* is continuous phasing and is not notated music, and *Clapping Music* is one
   pattern against itself in one meter, so neither is a polymeter fixture. 6/8 against 3/4 is the same-bar-duration
   shape as it actually appears — *America*, and every sesquiáltera — and is what the fixture writes.

10. **`render --to musicxml` writes a file, so the Check line needed `-o /dev/stdout`.** Without it the command writes
    `examples/bulgarian.musicxml` beside the source and pipes nothing — the same repair prompt 73 made for `-o` and the
    same class of mistake.

11. **`is_polytempo` is on `PerformancePlan`, and there is no per-lane tempo accessor.** The lanes are scheduled against
    their own maps and the frames are absolute, so nothing downstream needs the maps themselves. A public accessor with
    no caller is the thing this repository does not add.

## Stop

- No polytonality. Same shape, no evidence, own prompt.
- No metric modulation between simultaneous tempos.
- No tempo relationship notation (`♩ = 3:2`) unless the gate's named piece needs it.
- No engine change. If polytempo appears to need one, the frames are being computed in the wrong place.
- Do not implement polytempo to "complete the symmetry". The gate is the prompt.
