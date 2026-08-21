---
id: 26
slug: source-workspace
status: done
depends_on: [25]
phase: 1.5
---

# Source Workspace

## Task

Turn the source drawer into roadmap §14.4's **Source** workspace: CodeMirror 6 with musa language support, diagnostics
in the gutter, formatting on command, and live two-way linking between text and score. Then graduate
`docs/rules/desktop/` from candidate to the governing interface specification, the same way prompt 12 graduated the
event track.

## Read

- Roadmap §14.1 (CM6 and why: transactions over immutable state fit the source-transaction model), §14.4 (Source is a
  workspace, not a panel), §10.7 (debouncing), §11 (the source is canonical).
- `docs/rules/desktop/01-visual-language.md` §3 (Recursive Mono Linear; the source is set, not dumped),
  `03-interaction.md` §1 (one selection model — the source participates in it), `05-states.md` §5 (diagnostics),
  `06-performance.md` B1.
- Prompt 02's token kinds, prompt 04's formatter and `TextEdit`, prompt 25's edit pipeline.

## Design

- Replace the prompt-21 textarea with **CodeMirror 6**, configured against the token system: `--leaf` gutter-less
  background on the drawer's own surface, `--ink` text, Recursive Mono Linear, no ligatures, 2-space indent matching the
  formatter.
- **Musa language support** as a small CM6 package in `src/lib/lang-musa/`:
  - a stream or Lezer tokenizer covering musa's token kinds from prompt 02 — the highlighting must be derived from the
    real token list, not from a hand-written regex set that drifts;
  - highlight roles mapped to tokens, using **ink weight and a single accent**, not a rainbow: declarations and keywords
    in `--ink` medium, pitches and durations in `--ink`, identifiers in `--ink-muted`, string literals and units in
    `--ink-muted` italic, `use` occurrences in `--plate` (they are the generated-material sites — the same hue means the
    same thing in the source as on the page);
  - bracket matching, block folding on `piece` / `score` / `part` / `voice` / `motif`, and comment toggling.
- **Diagnostics** in the gutter and as underlines, from the snapshot — never recomputed client-side. Hovering shows the
  compiler's own message. Clicking a diagnostic in the list moves the caret (`05-states.md` §5).
- **Two-way linking**, the point of the workspace:
  - selecting an event on the score reveals and highlights its source span;
  - moving the source caret highlights the corresponding event(s) on the score;
  - in Origin view, `use` statements and their generated notes highlight together (prompt 24's trace, now
    bidirectional).
- **Formatting**: `⇧⌥F` runs prompt 04's formatter through `ProjectCommand::ApplyEdits`, preserving selection and
  scroll. Format-on-save is a preference, default off.
- **Source workspace layout**: a real workspace, not the drawer — source and score side by side (source left, leaf
  right), diagnostics beneath the source. The drawer remains available inside Compose for a quick look. Workspace
  switching is `⌘1` (Compose) / `⌘4` (Source), registered in prompt 23's command map; `⌘2`/`⌘3` are reserved for Sound
  and Mix, which do not exist yet and must not appear as empty tabs.
- **Graduate the specification**: promote `docs/rules/desktop/` from *candidate* to governing — update each file's
  status line, `docs/rules/desktop/README.md`, `docs/plan/prompts/README.md`, and `AGENTS.md`. Reconcile any place the
  built app and the specification diverged: either fix the code or repair the specification deliberately, in this
  commit, and say which in the commit body.

## Target

- `apps/musa-desktop/ui/src/lib/lang-musa/`: CM6 musa language support derived from prompt 02's token kinds.
- `apps/musa-desktop/ui`: CM6 editor, gutter diagnostics, two-way linking, format command, Source workspace and
  workspace switching.
- `docs/rules/desktop/`: status promoted; divergences reconciled.
- Tests: highlighting snapshot over `examples/*.musa` (proves token coverage); a test that every prompt-02 token kind
  has a highlight role; linking tests both directions; format-preserves-selection test; B1 still met with CM6 in place.

## Repairs made while implementing

- **Four-space indent, not two.** The Design section said "2-space indent matching the formatter"; the formatter
  (`crates/musa-syntax/src/formatter.rs`) indents by four. The editor follows the formatter, since the point of the rule
  is that formatting a file the editor indented changes nothing.
- **`⌘⇧F`, not `⇧⌥F`.** Prompt 23 already registered `edit.format` as `CmdOrCtrl+Shift+F`, and the registry is the one
  source the menu, the palette and the keyboard sheet all read. Moving it would have been a repair to prompt 23 with
  nothing gained; `03-interaction.md` §3 now records the binding it has.
- **Highlighting is derived twice over, and neither derivation can drift.** `musa-syntax` gained `TokenClass` (an
  exhaustive match over `SyntaxKind` — a new kind is a compile error) and `SPELLINGS` (a table proven against the lexer
  by `spellings_lex_as_their_kind`); the frontend tokenizer is then checked token-for-token against the Rust lexer's own
  reading of every file in `examples/` (`tests/unit/highlighting.test.ts`).
- **Folding is the brace, not a keyword list.** `piece`/`score`/`part`/`voice`/`motif` are all braced blocks, and so are
  `transpose` and `overlay`; a list of foldable keywords would have gone stale the first time the language grew one.
- **Bindings without ⌘ or ⌥ belong to the score pane, read off the accelerator.** Scope had been declared per command
  group, which made `F`, `L`, `Space` and `Return` global — so typing `forte` in the drawer toggled follow. The rule is
  now derived from the binding itself.
- **Two bugs the workspace exposed, both fixed in the session facade.** A command now flushes the pending draft before
  it runs (formatting a piece the composer had just retyped answered with the text they had replaced), and a command
  that rewrites the source drops the draft afterwards (otherwise the format, undo, or structured edit stayed hidden
  behind the draft it produced).
- **Revealing a span does not always take the keyboard.** A diagnostic is somewhere the composer is going to type, so
  the caret arrives focused; choosing a note on the page is not, so the text follows without the hands leaving the
  score. That distinction is now part of the request (`src/lib/state/reveal.ts`).
- **The budgets run alone.** `06-performance.md`'s numbers are measured on an unloaded machine, so `perf.spec.ts` is its
  own Playwright project that runs after the rest rather than beside it.

## Check

```sh
cargo nextest run -p musa-syntax -p musa-project
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cd apps/musa-desktop/ui && npm run check && npm run test
cd apps/musa-desktop && cargo tauri dev
# manual: ⌘4, edit glass-mountain.musa with highlighting and folding, click a note and watch the
#         source follow, break the source and read the gutter, ⇧⌥F to format
```

Commit as `Add source workspace and graduate the interface specification`.

## Stop

- No LSP, no completion, no rename refactoring — highlighting, folding, diagnostics, and linking only.
- No Sound or Mix workspaces (they follow prompt 31 and are their own prompts when the DSP exists).
- No multi-file or album projects (roadmap §16 later part; imports are prompt 36).
- No autosave (prompt 33).
