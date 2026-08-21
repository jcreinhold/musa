---
id: 35
slug: annotations-harmony
status: done
depends_on: [27]
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
- Prompt 22's annotation model (this extends it; do not fork it).

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
  elements (extend prompt 32 — this prompt touches that backend too).
- GUI: harmony symbols visible in the score (Verovio renders `<harm>`); a parts-list level outline (sections/phrases)
  for navigation — click a section, scroll the score. No editing UI beyond source (entry via source is acceptable here;
  note it).
- Fixture: extend `glass-mountain.musa` or add `examples/annotated.musa` with a phrase, two sections, and a harmony
  lane.

## Target

- `musa-syntax`/`musa-compiler`: syntax + annotation kinds + chord-symbol model.
- `musa-notation`: MEI + LilyPond + MusicXML rendering of annotations.
- `apps/musa-desktop`: outline navigation pane.
- Tests: annotation snapshots at each backend; chord-symbol parse table; position validation diagnostics.

## Check

```sh
cargo nextest run -p musa-compiler -p musa-notation
cargo clippy --all-targets -p musa-compiler -p musa-notation -- -D warnings
cargo fmt --check
cargo run -p musa -- render examples/annotated.musa --to mei -o /tmp/a.mei
cargo run -p musa -- render examples/annotated.musa --to musicxml -o /tmp/a.musicxml
```

Commit as `Add phrase, form, and harmony annotations`.

## Repairs made while implementing

- **The three annotations are anchored two different ways.** A phrase is anchored to the events it brackets, like a
  slur, so re-spelling or re-barring keeps it on its notes. A section and a chord symbol are anchored to *time*, because
  a bar line is a place whether or not a note starts there — `section "Coda" at 9:1;` in a piece that stops at bar 8 is
  a diagnostic, not a marker that silently lands on the last note.
- **`harmony` is one lane per score.** Two lanes would be two answers to "what chord is sounding", and neither the
  formats nor a reader has a way to prefer one. A second lane is refused by name.
- **A chord symbol needs no new token.** A symbol is one word, and the lexer already reads that word as an `Identifier`,
  a `PitchLiteral` (`e7` lexes as a pitch), or an identifier followed by an integer. The parser takes the word; the
  compiler checks it is one word and parses the grammar. `dsus4` was the case that made the grammar earn its tests: the
  `s` belongs to `sus`, not to a sharpened root.
- **`LilyPond` gets `\new ChordNames \chordmode`, not markup.** It is what LilyPond engraves best — its own line above
  the system, spaced against the music — and what a LilyPond user would write by hand. The cost is that each symbol is
  translated into chordmode's vocabulary (`cmmaj7` → `c:m7+`, LilyPond's spelling of a raised seventh) rather than
  printed verbatim, which is exactly what parsing the symbol bought. Form markers are `\mark \markup`, placed in the
  topmost staff because `\mark` is a Score-level event.
- **A phrase prints as its name in `LilyPond`, without a bracket.** LilyPond has no phrase bracket that does not need an
  engraver added to the layout, and a prompt about annotations is not the place to start emitting `\layout` blocks. MEI
  gets a real `<phrase>` (plus a `<dir>`, because not every consumer draws the bracket) and MusicXML a `<bracket>` pair
  with `<words>`.
- **`MusicXML`'s `<kind>` vocabulary is coarser than the symbols musa reads.** It has no name for a suspended chord with
  a seventh and none for an augmented major seventh. Those fall back to the triad they are built on, and the `text`
  attribute carries what the composer wrote — nothing is invented, and no symbol becomes a different chord.
- **The outline is a table of contents, not an editor.** Sections and phrases become `OutlineFacts` on `ScoreFacts`,
  each carrying the event to reveal, the bar, and the frames it is reached at — resolved by the core, so the interface
  scrolls to a notehead rather than guessing at a coordinate. Rows light for *every* passage the selection is inside, so
  a phrase and the section around it are both marked. Entry is via source, as the prompt allows.
- **`examples/annotated.musa` is a new fixture, not an extension of `glass-mountain.musa`.** The demo score is the
  subject of the interface's raster goldens and of most engraving tests; adding chord symbols above its staff would have
  rewritten all of them to prove something a second fixture proves on its own. The desktop reads the new one through
  `FIXTURES`, which is what the outline screens tests drive.
- **The frozen phase-1 oracle rejects `section` and `harmony` by name**, as it already did `phrase`, rather than
  ignoring score-level declarations it does not know.

## Stop

- No harmonic analysis, Roman numerals, voice-leading checks, or auto-voicing (§8.2: libraries, later, user-requested).
- No chord-symbol → notes realization of any kind.
- No annotation editing UI beyond source text.
