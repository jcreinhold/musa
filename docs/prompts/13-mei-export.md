---
id: 13
slug: mei-export
status: done
depends_on: [07]
phase: 1
---

# MEI Export

## Task

Write the MEI backend: deterministic MEI XML generation from a `NotationPlan` with `xml:id` attributes carrying
`EventId`s, so Verovio can render the score and map rendered notes back to semantic events. Add `musa render --to mei`.

## Read

- Roadmap §12.1 (plan→backend pipeline), §12.2 (MEI/Verovio rationale, `xml:id` contract, quick-xml requirement), §17.4
  (backend tests).
- Prompt 07's `NotationPlan`.

## Design

- Add `quick-xml` to `musa-render`. Build XML through a real writer (§12.2) — no `format!()` string assembly scattered
  through traversal.
- Public surface (the §15.4 facade, completed incrementally):

  ```rust
  pub enum NotationTarget { Mei /* , MusicXml, LilyPond */ }

  pub fn render_notation(
      score: &ScoreSnapshot,
      target: NotationTarget,
      options: &NotationOptions,
  ) -> Result<RenderedNotation, RenderError>;
  ```

  `render_notation` calls `plan_notation` internally; backends never re-derive
  measures themselves.
- MEI details: `<mei>` with current MEI version; `<score>` containing one `<staff>` per part staff; `<layer>` per voice;
  `<note xml:id="event-<hex>" ...>` with `dur`/`oct`/`pname`/`accid`; ties as `tie="i|m|t"` or `<tie>` elements — choose
  per Verovio compatibility and record the choice; beams via `<beam>` groups matching the plan. `<scoreDef>` carries
  meter and key from the maps; clef per staff.
- `xml:id` format: `event-` + stable hex of the `EventId`. Tied pieces of one event get suffixed ids (`event-<hex>-t2`)
  with the base id recorded so the GUI (prompts 23-25) can resolve any rendered note to its `EventId`. Document the
  scheme in the module docs — it is a contract.
- Output is deterministic: fixed attribute order, fixed declaration order, no timestamps. Snapshot tests are the
  regression net.
- `musa render <file> --to mei [-o out.mei]`; default output is `<name>.mei` beside the input or stdout with `-o -`.

## Target

- `musa-render`: MEI writer + `render_notation` facade with `NotationTarget::Mei`.
- `musa-cli`: `render --to mei`.
- Tests: insta snapshots for all three examples; a well-formedness check parsing the output back with quick-xml; a test
  asserting every note/layer id in the MEI resolves to an `EventId` present in the snapshot; MEI schema validation if a
  lightweight validator is practical (§17.4), otherwise a tracked note in the module docs.
- If Verovio's JS toolkit is easily invocable locally (e.g. via node), smoke-render one MEI file to SVG as a manual
  check; do not add a Verovio dependency to CI.

## Check

```sh
cargo nextest run -p musa-render -p musa-cli
cargo clippy --all-targets -p musa-render -p musa-cli -- -D warnings
cargo fmt --check
cargo run -p musa-cli -- render examples/glass-mountain.musa --to mei -o /tmp/gm.mei
grep -c 'xml:id="event-' /tmp/gm.mei   # non-zero, matches snapshot event count
```

Commit as `Add MEI export with event-id mapping`.

## Stop

- No LilyPond (09), no MusicXML (22), no MIDI.
- No Verovio integration into the app — the file is the deliverable; the GUI renders it in prompt 20.
- No raw backend escape hatches of any kind (roadmap §7.2).
