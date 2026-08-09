---
id: 81
slug: vscode-extension
status: done
depends_on: [77, 80]
phase: 3
---

# The VS Code Extension

## Task

Build `vscode-musa`: the extension that registers the `.musa` language with VS Code, starts the prompt 77 server,
and gives a composer diagnostics, hover, formatting, completion, and quick fixes in the editor most of them already
have open. The extension owns no vocabulary and no musical knowledge — both laws this repository has already paid
for once.

## Read

- Prompt 77 — the server; the extension is a launcher and a registration, nothing more.
- Prompt 80 — the tree-sitter grammar. VS Code cannot embed tree-sitter, so highlighting has two layers here; read
  the design before reaching for a TextMate grammar.
- `apps/musa-desktop/ui/src/lib/session/generated/` — the committed `spellings.json` / `token-classes.json` fixtures
  and the generator test that keeps them honest. This prompt reuses that mechanism rather than inventing a second
  one.
- `crates/musa-language/src/highlight.rs` — `SPELLINGS` and `TokenClass`: the vocabulary, stated once.

## Design

### Layout

A sibling repository `vscode-musa` (the extension ships independently of the Rust release cycle): `package.json`,
`language-configuration.json`, `src/{extension,client}.ts`, esbuild to one bundle, `vsce` to package.

`package.json` contributes the language (`id: "musa"`, extension `.musa`), a `musa.server.path` configuration, and
one command — *Musa: Restart Language Server*. `src/client.ts` is a `vscode-languageclient` over stdio: resolve the
binary from the setting, then `PATH`, else show one honest error saying what to install. No auto-download in this
prompt.

### Highlighting without a second vocabulary

VS Code wants a TextMate grammar for instant highlighting; the server provides semantic tokens a beat later. Ship
both, in that order of truth:

- A **generated** TextMate grammar: a build step turns the committed `spellings.json` / `token-classes.json` fixtures
  (the desktop's generator, prompt 26's mechanism) into `syntaxes/musa.tmLanguage.json`. The committed grammar is a
  build artifact; a stale one fails the generator's comparison test, exactly as the desktop's fixtures do.
- The server's **semantic tokens** refine it when running — they classify what a regex grammar cannot, and they come
  from the lexer itself.

`language-configuration.json` states comments, brackets, and the semicolon habit — small, declarative, and derived
from §7 rather than from taste.

### What the extension does not do

No webviews, no score preview, no status-bar playback. The desktop app is the score view (roadmap §14); an editor
extension that engraves is a second product wearing the first one's name.

## Target

- The `vscode-musa` repository: manifest, client, generated grammar plus its build step, `README.md` with install
  instructions (build `musa-lsp`, set `musa.server.path`).
- This repository: the fixture generator extended to emit the TextMate grammar, if the fixtures' home moves here.

## Check

```sh
npm --prefix <vscode-musa> run check && npm --prefix <vscode-musa> run test
npx --prefix <vscode-musa> vsce package
cargo nextest run -p musa-project   # the fixture generator's comparison stays green
```

Behavior, by hand before flipping status: open `glass-mountain.musa` — highlighted before the server answers, then
semantically refined; `broken/missing-semicolon.musa` shows its diagnostic and its quick fix; format, hover, and
completion work; killing the server leaves the TextMate highlighting standing.

## Stop

- No score preview, playback controls, or any engraving. The desktop app owns the page.
- No auto-downloaded server binaries; no marketplace publishing.
- No commands that duplicate the CLI (`render`, `play`) — an editor that shells out to `musa` can be configured by
  the composer who wants it.
- No hand-edited TextMate grammar. The generated one is the only one.
