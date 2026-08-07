---
id: 32
slug: musicxml-export
status: pending
depends_on: [27]
phase: 2
---

# MusicXML Export

## Task

Add the MusicXML interchange backend, completing the notation export trio. Export only — import is Phase 4. MusicXML is
an edge format: the `NotationPlan` drives it, and no MusicXML convention enters the score model (roadmap §12.4).

## Read

- Roadmap §12.4 (interchange role, why it's not the internal model), §12.1 (plan → backend pipeline), §17.4 (open output
  in two independent consumers).
- Prompt 13's MEI writer (same plan, analogous structure), prompt 27's annotation support.

## Design

- `musa-render`: `NotationTarget::MusicXml` on the existing facade. quick-xml writer like prompt 13; no string assembly.
- Content: `score-partwise` document; `<part>` per part; `<measure>` per plan measure; `<note>` with
  `<pitch><step><alter><octave>`, `<duration>` in divisions, `<type>` (quarter/eighth/…), dots, `<tie>` +
  `<notations><tied>` for ties (both forms, per the spec), `<notations>` slurs/articulations, `<direction>` dynamics,
  `<attributes>` for divisions/key/time/clef, `<backup>` for multi-voice measures, tuplets via `<time-modification>` +
  `<notations><tuplet>`.
- Divisions: compute a per-part (or per-score) divisions value that exactly represents every duration in the plan (LCM
  of denominators relative to the quarter); irrational-in-MusicXML cases get an explicit `RenderError` — no silent
  rounding (§7.2 policy).
- Metadata: `<identification>` with software string; part names from the score. `xml:id`/event provenance: MusicXML has
  no standard note id — do not invent one in the output; provenance stays an MEI feature (document this asymmetry; it is
  why MEI is the live format, §12.2).
- CLI/project/desktop: `--to musicxml` everywhere MEI already appears.
- Consumer check (§17.4): open one exported file in two independent consumers (e.g. MuseScore and Finale/Verovio's
  MusicXML import or an online validator) and record the result in the commit message or a fixture README. This is
  manual but required — it is the interoperability evidence.

## Target

- `musa-render`: MusicXML writer covering notes, rests, chords, ties, slurs, articulations, dynamics, tuplets,
  multi-voice, key/time/clef.
- Wiring in CLI, project export, desktop export menu.
- Tests: insta snapshots for all examples incl. the prompt-17 fixture; well-formedness reparse; divisions exactness
  proptest (every plan duration maps to an integer MusicXML duration); determinism.

## Check

```sh
cargo nextest run -p musa-render -p musa-project
cargo clippy --all-targets -p musa-render -p musa-project -- -D warnings
cargo fmt --check
cargo run -p musa-cli -- render examples/counterpoint.musa --to musicxml -o /tmp/cp.musicxml
# manual: open /tmp/cp.musicxml in two consumers, note results
```

Commit as `Add MusicXML export`.

## Stop

- No MusicXML import (Phase 4).
- No `partwise`↔`timewise` duality — partwise only.
- No event-id embedding or custom extensions; stay vanilla.
- No layout/page formatting elements beyond defaults.
