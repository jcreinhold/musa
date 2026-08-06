---
id: 25
slug: annotations-harmony
status: pending
depends_on: [17]
phase: 3
---

# Phrase, Form, and Harmony Annotations

## Task

Add the annotation layer that makes pieces legible at structure level: phrase/form spans and a harmony lane
(`at 1:1 am;`) — recorded in the score as annotations with provenance, displayed in notation and the GUI, interpreted by
nothing. Per roadmap §8.2, harmony is annotation, not ontology.

## Read

- Roadmap §8.2 (chord symbols recorded; the core never derives notes from harmony; later theory libraries are algorithms
  over the model), §8.3 (abstractions must lower transparently — annotations are inspectable, never magical), §6.3
  (`AnnotationStore`), §12.1 (annotations and spans in the plan).
- Prompt 17's annotation model (this extends it; do not fork it).

## Design

- Language:

  ```text
  phrase "A" { ... }              % wraps voice content with a named span
  section "Exposition" at 1:1;    % form marker at a measure:beat position
  harmony {
      at 1:1 am;
      at 3:1 fmaj7;
  }
  ```

  Positions are `measure:beat` coordinates resolved against the meter map.
- Compiler: `AnnotationStore` entries of kinds `Phrase`, `Section`, `Harmony`, each with span + provenance. Validation:
  positions inside the piece, chord-symbol parse (a small chord-symbol grammar: root, quality, extensions — parsed to a
  structured symbol, not stored as opaque text, so later §8.2 libraries can consume it). **No** analysis, no validation
  that notes fit the chord, no voice-leading opinion (§8.2 — that restraint is the feature).
- Notation: harmony lane renders above the staff in MEI (`<harm>`) and LilyPond (`\chordmode` or markup — pick what
  Verovio/LilyPond render best; document). Phrase/section markers render as rehearsal marks/text. MusicXML: `<harmony>`
  elements (extend prompt 22 — this prompt touches that backend too).
- GUI: harmony symbols visible in the score (Verovio renders `<harm>`); a parts-list level outline (sections/phrases)
  for navigation — click a section, scroll the score. No editing UI beyond source (entry via source is acceptable here;
  note it).
- Fixture: extend `glass-mountain.musa` or add `examples/annotated.musa` with a phrase, two sections, and a harmony
  lane.

## Target

- `musa-language`/`musa-compiler`: syntax + annotation kinds + chord-symbol model.
- `musa-render`: MEI + LilyPond + MusicXML rendering of annotations.
- `apps/musa-desktop`: outline navigation pane.
- Tests: annotation snapshots at each backend; chord-symbol parse table; position validation diagnostics.

## Check

```sh
cargo nextest run -p musa-compiler -p musa-render
cargo clippy --all-targets -p musa-compiler -p musa-render -- -D warnings
cargo fmt --check
cargo run -p musa-cli -- render examples/annotated.musa --to mei -o /tmp/a.mei
cargo run -p musa-cli -- render examples/annotated.musa --to musicxml -o /tmp/a.musicxml
```

Commit as `Add phrase, form, and harmony annotations`.

## Stop

- No harmonic analysis, Roman numerals, voice-leading checks, or auto-voicing (§8.2: libraries, later, user-requested).
- No chord-symbol → notes realization of any kind.
- No annotation editing UI beyond source text.
