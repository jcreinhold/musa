---
id: 51
slug: engraved-edition
status: done
depends_on: [27, 35]
phase: 2
---

# The Page as an Edition

## Task

Make what musa engraves look like a page torn out of a real book of music rather than a run of staves. The piece gains
the front matter a printed edition has — composer, arranger, subtitle, copyright — the page gains the apparatus an
edition has — measure numbers, instrument labels, a running head, page numbers, a final barline — and all four backends
carry it, because a title that exists only on screen is a UI feature pretending to be a document.

## Read

- Roadmap §2 (the layer table: **written text is not layout** — a composer's name is a fact about the piece, and where
  it sits on the page is the engraver's business), §12.2 (MEI), §12.3 (LilyPond), §12.4 (MusicXML).
- `docs/interface/02-engraving.md` — the quality bar this raises, and the Verovio options it currently disables.
- `docs/interface/01-visual-language.md` §3 — Academico *is* the score's text face, chosen precisely so that a title in
  the app frame and the same title on the page are one object. This prompt is what makes that claim true.
- Prompt 07's `NotationPlan`, prompt 13's MEI writer, prompt 05's `ScoreSnapshot`.

## Design

### What a printed page has that musa's does not

Held against a Suzuki volume, a Henle urtext, or any engraved edition, the current page is missing nine things. Each is
listed with where it belongs in the layer table, because half of them are facts about the piece and half are the
engraver's apparatus, and confusing the two is how a score editor grows a "layout" model it can never delete.

| Missing | Whose fact it is |
| --- | --- |
| Composer, on the right above the first system | the **piece** |
| Arranger, beneath the composer | the **piece** |
| Subtitle, under the title | the **piece** |
| Copyright, at the foot of the first page | the **piece** |
| Instrument names at the left of the first system, abbreviated after | the **score** (part names already exist) |
| A first-system indent to hold them | the **engraver** |
| Measure numbers | the **engraver** |
| A running head and page numbers after page 1 | the **engraver** |
| A final thin-thick barline | the **engraver** |

The first four are new language. The rest are already-known facts the backends were not emitting, or Verovio options
turned off.

### The language

Four statements inside `piece`, beside `tempo` / `meter` / `key`, each taking one string:

```musa
piece "Glass Mountain" {
    subtitle "for violin and strings";
    composer "Jacob Reinhold";
    arranger "after a folk tune";
    copyright "© 2026. Licensed CC BY-SA 4.0.";
```

- **A closed set of four, not a metadata bag.** Each has a first-class slot in MEI, MusicXML, *and* LilyPond; a
  key-value dictionary would have none of them and would push the mapping into every backend. If a fifth role is
  genuinely needed later it is a fifth statement, decided then.
- **The title stays the `piece` name.** There is no `title` statement; a piece already has a name and a second way to
  spell it is a way for them to disagree.
- **All four are optional, and absence prints nothing.** A page with an empty composer line is worse than a page
  without one (`05-states.md` §2 applies to paper too).
- **`musa.toml`'s `composer` is the fallback, not a competitor.** A piece that names its own composer uses it; a piece
  that does not inherits the project's. Resolution happens once, in `musa-project`, so every backend and the GUI see
  one answer. This is the reason `ProjectMeta` was built in the first place.
- Formatter: the four sit in a block with `tempo`/`meter`/`key`, in source order, one per line. The formatter does not
  reorder them — a composer who put the dedication-ish line first meant it there.

### The apparatus

None of this is new language; it is the backends emitting what the plan already knows.

- **`NotationPlan` gains a `FrontMatter`** — title, subtitle, composer, arranger, copyright, all `Option<String>` except
  the title — read from `ScoreSnapshot`. One struct, one accessor.
- **MEI** gains a real `<meiHead>`: `<titleStmt>` with `<title>`, `<title type="subtitle">`, `<composer>`, `<arranger>`,
  and `<pubStmt><availability>` for the copyright. Verovio renders these into the page head from the header itself, so
  the app stops passing `header: "none"`.
- **`<staffDef>` gains `<label>` and `<labelAbbr>`** from the part name — the abbreviation is the part name's first
  three letters plus a period unless the name is already shorter, which is what an engraver does when no abbreviation
  was given. Verovio indents the first system for them on its own.
- **The final measure gets `right="end"`**, which is the thin-thick barline. A score that stops mid-air reads as a
  fragment, and musa's pieces are finite by construction.
- **Measure numbers**: `mnumInterval: 0`, which is Verovio's spelling of one number at the head of each system — the
  modern editorial default, as against a number on every bar, which is a proof-reading copy. (The option counts a
  repeat interval, so 0 is per-system and any *n* > 0 means every *n* bars.) Verovio draws them above the top staff.
- **Running head, page numbers, and the copyright line**: Verovio's `header: "auto"` and `footer: "auto"` with the MEI
  head supply all three — title and subtitle centred on page 1 with composer and arranger to the right, a running head
  after it, and the copyright at the foot.
- **LilyPond** gains the matching `\header { title composer arranger subtitle copyright }` block, and **MusicXML**
  gains `<work><work-title>` and `<identification><creator type="composer">`, `type="arranger"`, and `<rights>`.
  A backend that dropped the front matter would make export a lossy operation, which §12 forbids.

### What this is not

- Not a page-layout model. Nothing here is positioned by musa; every one of these is a fact handed to an engraver that
  already knows where such facts go. The moment musa says *where* the composer's name sits, it owns page layout
  forever.
- Not a template or style system. One edition style, the one in `02-engraving.md`.

## Target

- `musa-language`: `SubtitleKw`, `ComposerKw`, `ArrangerKw`, `CopyrightKw`; the four statements in the CST and the
  formatter; the highlighter's keyword table.
- `musa-compiler`: `ScoreSnapshot` carries the four; `resolve` fills them.
- `musa-project`: the `musa.toml` composer fallback applied once, where the snapshot is built.
- `musa-render`: `FrontMatter` on `NotationPlan`; `<meiHead>`, `<label>`/`<labelAbbr>`, `right="end"`; LilyPond
  `\header`; MusicXML `<work>`/`<identification>`.
- `apps/musa-desktop/ui`: `header: "auto"`, `footer: "auto"`, `mnumInterval: 0`; the goldens re-shot.
- `examples/`: `glass-mountain.musa` and `twinkle.musa` gain front matter — they are the corpus the goldens are cut
  from, and a fixture with no composer would leave the new path untested.
- Tests: `insta` snapshots of MEI/LilyPond/MusicXML front matter, present and absent; a parser snapshot; a formatter
  idempotence case; the `musa.toml` fallback and its override; Playwright goldens of the engraved page.

## Check

```sh
cargo nextest run -p musa-language -p musa-compiler -p musa-render -p musa-project
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cd apps/musa-desktop/ui && npm run check && npm run test
```

Commit as `Engrave the page as an edition`.

## Stop

- No dedication, opus number, movement number, or catalogue number. Four roles; a fifth is a later decision.
- No per-page or per-system layout control of any kind.
- No fingerings, bowings, pedalling, or ossia staves.
- No cover page, table of contents, or multi-movement front matter — `examples/album/` is a directory of pieces, not a
  bound volume, and binding it is roadmap Phase 4.
