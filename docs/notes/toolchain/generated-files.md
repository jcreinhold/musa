# Do not format generated files

## The rule

Two directories under `apps/musa-desktop/ui/` are written by a program, and their exact bytes are test fixtures:

- **`src/lib/session/generated/`** — `ts-rs` DTOs, the token and module tables.
- **`fixtures/`** — the `.snapshot.json`, `.mei`, and `lexed/` goldens, written by `musa-project`'s generator tests.

A currency test regenerates each file and compares. Reformatting one — by hand, by a formatter, or by a tidy-up pass —
changes the bytes without changing the generator, so the comparison fails and the failure names the file rather than the
edit that broke it.

If a generated file needs to change, change the generator and regenerate:

```sh
make snapshots        # regenerates every golden and UI fixture
```

or, for one crate's fixtures only:

```sh
UPDATE_UI_FIXTURES=1 cargo test -p musa-desktop
```

## What it looks like when it goes wrong

```text
Error: .../generated/spellings.json is stale — rerun with UPDATE_UI_FIXTURES=1 and commit the result
test result: FAILED. module_names_are_current, spellings_are_current
```

The diagnostic is accurate but its cause is easy to misread as a real semantic change. Check first whether the file was
merely reformatted:

```sh
git log --oneline -3 -- apps/musa-desktop/ui/src/lib/session/generated/spellings.json
```

A commit about formatting, readability, or consistency touching a generated file is the answer. Regenerating restores
byte-identical content to the version before that commit, which is the confirmation.

This has happened, in one pass, across both directories: `spellings.json` collapsed from 450 lines to 114, and
`large-score.snapshot.json` gained 44,848 lines. Eight currency tests failed from then on. Regenerating reproduced the
pre-pass files byte for byte, which is what proved no semantics had changed.

## Why Prettier now ignores both directories

The two directories had a standing fight between two writers. The clearest case is the DTOs:

- `ts-rs` writes them from the Rust type definitions and emits `export type SpanDto = { start: number, end: number, };`
- Prettier rewrites that to `export type SpanDto = { start: number; end: number };`

Whichever ran last won. Running the Rust suite dirtied the working tree with a dozen reformatted DTOs that nobody had
edited; running `pnpm run format` put them back. The `.json` goldens under `fixtures/` had the same fight, and lost it
harder — that is the pass described above.

The generators win, because their output is what the currency tests compare against and Prettier's is not.
`apps/musa-desktop/ui/.prettierignore` now lists `src/lib/session/generated` and `fixtures`, so the formatter skips them
entirely, and the generator-emitted form is what is committed.

Verified after the change:

| Check | Result |
| --- | --- |
| `pnpm run format:check` | names no file under `generated/` or `fixtures/` |
| `pnpm -r check` (Svelte + `tsc` over all three projects) | clean — the UI at 460 files, 0 errors, 0 warnings |
| `pnpm run lint` (ESLint) | clean |
| `cargo test -p musa-desktop`, then `git status` | tree clean; what it regenerates matches what is committed |

The strongest confirmation is that `pnpm run format` — the exact hand-run `--write` that rewrote these directories in
the first place — was run again afterwards and reformatted ninety hand-written files while touching neither generated
directory.

`prettier --check` and `eslint` used to be in no build target, which is why nothing caught the rewrite at the time — and
neither were `taplo fmt --check` or `mdwright fmt-check`, though `AGENTS.md` called all of them must-be-green. They are
now gates: `make fmt-check` runs all four formatters, `make lint-ui` runs ESLint, and `make verify` runs both. So a
formatting pass over a generated file is stopped by the ignore file, and drift anywhere else fails the build rather than
waiting to be swept up by a tidy-up commit.

## Who owns which files

Four formatters run over this repo, and they only coexist because their territories are disjoint:

| Formatter | Owns | Kept off |
| --- | --- | --- |
| `cargo fmt` | `*.rs` | — |
| `taplo` | `*.toml` | — |
| `mdwright` | `*.md`, everywhere | — |
| Prettier | the UI's `*.ts`, `*.svelte`, `*.json`, `*.html`, `*.css` | Markdown, and both generated directories |
| the generator tests | `src/lib/session/generated/`, `fixtures/` | — |

Prettier is barred from Markdown because it pads table columns and `mdwright` collapses them; left overlapping, the two
rewrite each other forever. That is the same failure as the generated directories, and it has the same remedy: one
writer per file, stated in `.prettierignore`.
