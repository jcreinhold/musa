---
id: 24
slug: origin-view
status: done
depends_on: [23]
phase: 1.5
---

# Origin View: making provenance visible

## Task

Implement the application's signature interaction: a held lens that separates authored music from generated music on the
page, traces any generated note back to the occurrence that produced it, selects whole occurrences, and links score,
parts list, and source. Also surface diagnostics into the score. This is the one thing musa can show that no other
program can, and it is the precondition for prompt 25's editing choice being comprehensible.

## Read

- `docs/interface/04-provenance.md` in full — this prompt's specification.
- `05-states.md` §5 (diagnostics), `06-performance.md` B9.
- Roadmap §9 (editing transformed music: why the choice must be visible before it is offered).
- Prompt 06's `Origin` and expansion paths; prompt 13's id mapping; prompt 19's `ProjectSnapshot`.

## Design

- **Snapshot first.** Origin view must be drawable entirely from `ProjectSnapshot` (`03-interaction.md` §7: the frontend
  computes nothing about provenance). Audit what it needs and add it to the snapshot in `musa-project`: per-event
  `origin` (authored with a source span, or generated with an occurrence id and expansion path), and an occurrence table
  mapping occurrence id → declaration span, use-site span, and the events it produced. If a field is missing, extend the
  snapshot — do not derive it in TypeScript.
- **The lens** (`04-provenance.md` §2): hold `O` or `⌥`; 120 ms cross-fade (B9); authored stays full `--ink`, generated
  falls to `--plate` at 65 %; **no layout reflow** — ink and margin only. A pinned toggle in the top margin exists for
  users who cannot hold a key.
- **Run brackets**: each generated run gets an editorial bracket in `--plate` over exactly the notes that expansion
  produced, labelled with the occurrence, drawn in the prompt-22 overlay layer. Colour is never the only signal
  (`03-interaction.md` §5). `04-provenance.md` §2 first put this bracket in the system's left margin; that is repaired
  in this prompt, because glass-mountain's two occurrences share one system and two margin brackets at the same height
  answer nothing.
- **Trace on hover**: one `1px --plate` hairline from the note to its bracket, plus source highlighting of the `motif`
  declaration and the `use` statement when the drawer is open. Drawn instantly; it does not animate along its path.
- **Click selects the occurrence**: all events that expansion produced, across bars and staves. This becomes the
  selection unit prompt 25's edit-definition path acts through.
- **Inspector Origin row** (§3) upgraded from prompt 23's read-only display: each path segment clickable — occurrence
  selects, motif name reveals the declaration, line number opens the source there.
- **Diagnostics into the score** (`05-states.md` §5): clicking a diagnostic moves the source caret and flashes the
  corresponding system once; diagnostics carry a shape as well as `--chalk`; they are never toasts.
- **Restraint check** (§5): `--plate` at high coverage appears here and nowhere else in the app. If another lens is
  proposed later, something is removed.

## Target

- `musa-project`: `ProjectSnapshot` gains the origin/occurrence data (with tests that a motif-generated event reports
  its full expansion path and its occurrence's spans).
- `apps/musa-desktop/ui`: the held lens, margin brackets, hover trace, occurrence selection, clickable Origin row,
  diagnostic → score/source linking.
- Tests: snapshot tests for origin data on `glass-mountain.musa` (its violin part is ten generated notes from two
  occurrences of one motif — the canonical case); a Playwright test that holds the key, asserts the generated notes'
  computed color changed and layout did not move, hovers, and asserts one trace; screenshot goldens for Origin view in
  both themes; B9 assertion.

## Check

```sh
cargo nextest run -p musa-project
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cd apps/musa-desktop/ui && npm run check && npm run test
cd apps/musa-desktop && cargo tauri dev
# manual: open glass-mountain.musa, hold O — the two sigh() occurrences and the transposed one
#         must be immediately, obviously distinguishable from the authored strings parts
```

Commit as `Add Origin view for provenance`.

## Stop

- No editing (prompt 25). The choice dialog of `04-provenance.md` §4 belongs to that prompt; this prompt only makes it
  legible.
- No specialization (`use sigh() with {...}` — prompt 34).
- No additional lenses, x-ray modes, or overlays of any kind.
- No harmony or phrase annotation display (prompt 35).
