---
id: 75
slug: polymeter-and-polytempo
status: pending
depends_on: [64, 72, 74]
phase: 3
---

# Polymeter and Polytempo

## Task

Two parts in different time signatures at once, and — separately — two parts at different tempos at once. Prompt
63's `Scope::Voice` can already hold a meter and a tempo; prompt 64 and prompt 72 declined to produce one. This
prompt produces them, or declines a second time with a reason.

**This prompt has a gate, and it must be passed before the prompt is run.** See below.

## The gate

Polymeter has evidence: Balkan and Bulgarian folk ensembles play 7/8 against 4/4 as a matter of course, Reich's
phase pieces are built on it, and Afro-Cuban and West African ensemble music is layered by construction. It also has
consumers — MEI, MusicXML and LilyPond all express it.

**Polytempo does not.** It is not in `docs/course-correction.md` §33's falsification corpus, no fixture needs it,
and its export story is bad. Under §34 the burden is semantic necessity, so before any code:

1. Name a real piece in this prompt's Task section that musa should be able to write and cannot without it —
   Nancarrow's player-piano studies, Ives's *Fourth Symphony*, Lutosławski's *ad libitum* sections, or Carter — and
   add it to §33 as a row.
2. Name the consumer that reads it and does something different because of it.
3. If either cannot be named, **strike the polytempo half of this prompt** and ship polymeter alone. That is a
   legitimate outcome and the prompt should say so rather than treating it as failure.

Recording this gate is worth as much as passing it. Prompt 66 declined `choose` on the same reasoning, and the
pattern — a capability that the architecture makes easy is not thereby justified — is the one this repository most
needs to keep applying.

## Read

- Prompt 63 §"Inheritance is per kind" — meter and tempo are both `Override`, which is what makes per-voice work
  with no new rule.
- Prompt 61 `bars.rs` — `BarLines` is per-piece today. Polymeter makes it per-scope, which is the real change.
- `crates/musa-render/src/plan.rs::plan_staff` (:820) — the measure walk, which assumes one barline grid.
- `crates/musa-engine/src/playback.rs` (:90) — the frame merge. Read it before worrying about polytempo's engine
  cost; the news is good (see below).
- `docs/course-correction.md` §33 and §34.

## Design

### Polymeter: `BarLines` becomes per-scope

`ScoreSnapshot::bars()` becomes `bars(scope)`, resolved through the inheritance chain. A voice with its own meter
gets its own barline grid; every other voice inherits the piece's. Prompt 61 built `at`/`time_of` as inverses on an
arbitrary meter sequence precisely so this would be a change of *argument*, not of algorithm.

Two shapes, and both must work:

- **Same bar length, different grouping** — 3+2+2 against 4/4. One barline grid, different beaming. This is most
  Balkan music and it is nearly free.
- **Different bar lengths** — 7/8 against 4/4, where barlines genuinely diverge and only realign every 56 eighths.
  This is the one that needs per-scope `BarLines`, and it is what `plan_staff` must learn.

The engraver draws each staff its own barlines. `docs/interface/` must agree that a system may have staves whose
barlines do not align, because today's visual language assumes they do — a repair to that document, in this commit.

### Polytempo, if the gate passes

Per-voice tempo means per-voice `IntegratedTempoMap`, so `PerformancePlan` stops holding one shared map and holds
one per lane.

The good news, which the prompt should verify early rather than fear: **the engine needs no change.** Frames are
absolute by the time they reach `playback.rs`, and beat time never crosses into it, so lanes at different tempos
merge exactly as lanes at the same tempo do. `lower_performance` does all the work.

The bad news is export. **MIDI has one tempo track.** Polytempo is therefore exported by resolving every lane
against a single reference tempo and emitting the resulting absolute times — which is **sonically exact** and
**notationally lossy**: the file plays correctly and its notated tempo is wrong. Say exactly that in a warning, and
in `07-backend-contract.md`; do not let it be discovered.

MusicXML and MEI express per-staff tempo directions. LilyPond needs separate `\score` blocks or polymetric
`\scaleDurations`, which is a documented approximation.

### What must not follow

Per-voice *key* — polytonality — is the same shape and is **not** in this prompt. It has its own repertoire, its own
spelling consequences (the MIDI speller would need a scope), and no fixture. Adding it because the machinery allows
it is the exact move the gate exists to prevent.

## Target

- `crates/musa-compiler`: `bars(scope)`; per-scope meter resolution; per-lane tempo maps in `PerformancePlan` if
  the gate passes.
- `crates/musa-render/src/plan.rs`: per-staff barline grids; per-staff beaming.
- `crates/musa-project/src/midi.rs`: the single-tempo-track resolution and its warning.
- `docs/interface/`: non-aligned barlines within a system.
- `docs/course-correction.md` §33: the polymeter rows proven; the polytempo row added or the decline recorded.
- `examples/`: `bulgarian.musa` (7/8 against 4/4), `phase.musa` (Reich's shape). Polytempo's fixture only if the
  gate passes.

## Check

```sh
cargo nextest run --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
cd apps/musa-desktop/ui && pnpm test
cargo run -p musa-cli -- check examples/bulgarian.musa
cargo run -p musa-cli -- render examples/bulgarian.musa --to musicxml | grep -c '<time>'   # 2
cargo bench -p musa-compiler        # per-scope BarLines must not regress P1-P3
```

Commit as `Add polymeter` — or `Add polymeter and polytempo`, per the gate.

## Stop

- No polytonality. Same shape, no evidence, own prompt.
- No metric modulation between simultaneous tempos.
- No tempo relationship notation (`♩ = 3:2`) unless the gate's named piece needs it.
- No engine change. If polytempo appears to need one, the frames are being computed in the wrong place.
- Do not implement polytempo to "complete the symmetry". The gate is the prompt.
