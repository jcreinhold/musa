---
id: 32
slug: musicxml-export
status: done
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
cargo run -p musa -- render examples/counterpoint.musa --to musicxml -o /tmp/cp.musicxml
# manual: open /tmp/cp.musicxml in two consumers, note results
```

## Repairs made while implementing

- **The consumer check was run against two independent importers, and both are recorded here.** libxml2 validated all
  seven exported examples against the official `MusicXML` 4.0 XSD (`xmllint --schema`, schema from `w3c/musicxml` v4.0)
  — *validates*, no warnings. Verovio 6.2.0's `MusicXML` importer — a codebase with no relation to this writer — loads
  and engraves all seven, and each piece draws the **same number of noteheads whether it arrives as MEI or as
  `MusicXML`**, which is a stronger statement than "it opened": the two encodings of one score agree.
- **No `<accidental>` element is written.** `<accidental>` means "print this symbol here", and musa has no
  cautionary-accidental model to justify forcing one. `<step>`/`<alter>`/`<octave>` already determine the spelling
  completely — B-flat and A-sharp differ in step — so the consumer derives the printed accidentals from the alteration,
  the key, and the measure, and gets back what the source spelled. MEI's `@accid` is the printed symbol and is written;
  the asymmetry is deliberate and documented on both backends.
- **Divisions are computed for the whole document, not per part.** One `<divisions>` value means one grid, and a
  `<backup>` from a voice in 3-space to a voice in 2-space cannot land between two ticks. The value is the least common
  multiple of what every measure duration, onset, and sounding duration demands; past `MAX_DIVISIONS` the export fails
  with `RenderError::Divisions` rather than rounding a duration that was exact when it was written.
- **`<duration>` is the sound, `<type>` is the symbol.** They differ exactly inside a tuplet, which is why a triplet
  eighth is `<type>eighth</type>` with a `<time-modification>` and a duration two thirds of one.
- **Beams carry their levels and their hooks.** A beamed run is not one flag repeated: level *n* of a note is `begin`,
  `continue`, or `end` by what its neighbors in the group carry, and a level neither neighbor has is a hook — which is
  how `MusicXML` spells the short side of a dotted-eighth/sixteenth pair. A beam group of one is not written at all; the
  note keeps its flags.
- **A voice that does not sound gets `<rest measure="yes"/>`, and a gap inside one gets `<forward>`.** The plan can
  leave a lane silent without writing a rest for it, and a partwise document has elements for both cases; inventing a
  rest instead would have put a symbol on the page that the source never wrote.
- **Slur numbers are allocated, not fixed.** `MusicXML` distinguishes concurrent slurs 1–6 and the number has to match
  across the pair, so the open ones are a per-voice stack: a slur takes the lowest free number and gives it back when it
  closes. A chain of slurs therefore reads `1`, `1`, `1` rather than climbing until it runs out.
- **The tuplet bracket marks its ends only.** The notes between them carry `<time-modification>` — how long they last —
  and nothing printed, so `<notations>` is written only when something goes in it.
- **The encoder string carries no version and no date.** An export of one source is the same bytes on every machine and
  every build, which is what the determinism test asserts and what makes the snapshots a regression surface rather than
  a diff of when they were taken.

Commit as `Add MusicXML export`.

## Stop

- No MusicXML import (Phase 4).
- No `partwise`↔`timewise` duality — partwise only.
- No event-id embedding or custom extensions; stay vanilla.
- No layout/page formatting elements beyond defaults.
