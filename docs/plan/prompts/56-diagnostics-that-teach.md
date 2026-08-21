---
id: 56
slug: diagnostics-that-teach
status: done
depends_on: [05, 19, 26]
phase: 2
---

# Diagnostics That Teach

## Task

A musa diagnostic becomes a small piece of writing with a stable name, a labelled place, the other places that explain
it, one line of advice, and — where the fix is unambiguous — an edit the reader can apply without typing. The same
structure reaches the terminal and the app: `musa check` renders it with source context, and the app's problems list
stops printing a byte offset where a location belongs.

## Read

- Roadmap §15 — `miette` is the named rendering dependency. Nothing else is added; the ariadne look is a rendering
  quality bar, not a second crate (see Design).
- Roadmap §10.6, §15.7 — compiler types stop at `musa-project`. The diagnostic is restated at each boundary, as it is
  today; this prompt widens all three restatements together.
- `docs/rules/desktop/05-states.md` §4 (a problem is a place, not a notification), §2 (empty states).
- `docs/rules/desktop/03-interaction.md` §7 — what the frontend may compute. Line and column are **not** on that list,
  so Rust computes them.
- `crates/musa-syntax/src/edits.rs` — `TextEdit` already exists and is already how the app writes source. A fix is a
  `Vec<TextEdit>` and nothing new.
- The current messages: `crates/musa-syntax/src/parser.rs` (about thirty `error_here` calls),
  `crates/musa-compiler/src/resolve.rs`, `elaborate.rs`, `studio.rs`, `imports.rs`, `harmony.rs`.

## Design

### The one idea

**A diagnostic is not a sentence, it is a small document.** Today every one of them is a single string and an optional
range, so every one of them has to carry the whole explanation in a clause, and none of them can point at the *second*
place that would make it obvious. Rust's diagnostics are good not because their prose is good but because their shape is
right: a claim, the place, the other places, the advice, and the edit. Give musa that shape and the prose gets shorter,
not longer.

### The type

One shape, restated at three boundaries as the existing `Diagnostic` already is:

```rust
pub struct Diagnostic {
    pub severity: Severity,
    /// Stable, readable, kebab-case: `unknown-name`, `does-not-add-up`.
    pub code: Code,
    /// What is wrong. One line, no trailing period, no apology.
    pub message: String,
    /// Where. The primary label is first and is the place to jump to.
    pub labels: Vec<Label>,
    /// What to do about it.
    pub help: Option<String>,
    /// The rule behind it, when knowing the rule prevents the next one.
    pub note: Option<String>,
    /// Edits that resolve it, when there is no guesswork.
    pub fixes: Vec<Fix>,
}

pub struct Label { pub span: Span, pub text: String, pub primary: bool }
pub struct Fix { pub title: String, pub edits: Vec<TextEdit> }
```

`message` and `label.text` say different things and the difference is the whole discipline. The message is what is wrong
with the piece; the label is what is wrong *at that character*. `cannot find 'sigh'` / `not declared in this piece` —
never the same words twice.

### Codes, not numbers

`E0412` is a lookup key for people who already know the system. `unknown-name` is a lookup key and an explanation, and
it survives being read aloud. Codes are kebab-case, stable once shipped, and listed in one module. `musa explain <code>`
prints the long form: what the rule is, one example that breaks it, and the same example fixed. Only codes whose rule is
non-obvious get an explanation; the rest are their own.

### Fixes are data

A fix is a title and text edits, so the terminal and the app spend the same object. The CLI prints the title and the
replacement; the app offers it as a control and applies it through the edit path it already has. **A fix is offered only
when it is certain.** `add ';'` is certain — the character is missing and there is one place it goes. A spelling
suggestion never is: `sigh` and `sign` are both one edit from `sigb`, and which one the composer meant is not something
a compiler knows. So a suggestion is always a *help line* and never a fix, and the app offers a control only where the
core offered exactly one fix.

### Suggestions come from what the composer wrote

Every "not found" diagnostic searches the names actually in scope and offers the nearest, by Damerau-Levenshtein, within
a third of the written name's duration and never fewer than one edit or more than three. A tie offers nothing: two
candidates at the same distance means the suggester does not know, and saying so by staying quiet is the honest answer.
No dependency: it is a page, private to the compiler, tested against the corpus. The same routine serves motifs, parts,
voices, patches, buses, and studio parameters.

### Line and column belong to Rust

`musa-project` resolves each label's byte range to `{ line, column }` — 1-based, columns counted in characters, not
bytes and not UTF-16 code units, because that is what a person counts. The app prints `12:5`, never `184`. This is
`utf16.rs`'s neighbour and lives beside it.

### The writing standard

Applied to every message in the repo, not just new ones:

- **Say what happened, not what the parser wanted.** `expected SemicolonKw` → `this statement is missing its ';'`.
- **No jargon.** No token names, no non-terminal names, no "unexpected EOF". The composer has never read the grammar.
- **Omit needless words.** `it seems that there may be an unclosed block` → `this block is never closed`.
- **No apology and no blame.** Not "sorry", not "you forgot".
- **A list of twelve alternatives is not help.** `expected a use, tempo, meter, key, subtitle, composer, arranger,
  copyright, motif, score, performance, or studio declaration` becomes a message naming what was found, a label saying a
  declaration belongs here, and a help line naming the three or four a piece almost always wants next — with `musa
  explain` holding the full list.

### The terminal

`miette`'s graphical renderer, given labels instead of one anonymous span, plus help and note, is the ariadne look: the
gutter, the underlines, the arrows, the boxed advice. The renderer is a detail of the CLI and stays there. What changes
in `main.rs` is that `CliDiagnostic` grows real labels, `help`, and a fix footer, and that a summary line follows the
run: `3 problems (2 errors, 1 warning)`.

## Target

- `crates/musa-syntax`: `SyntaxError` gains `code`, secondary labels, `help`, and `fixes`; parser messages rewritten to
  the standard above; the recovery fixtures re-snapshotted.
- `crates/musa-compiler`: `Diagnostic` as above; `Code`; a private `nearest` name-suggester; every `Diagnostic::error` /
  `warning` call site given a code and a labelled span, and a help line where one exists.
- `crates/musa-project`: the restated `Diagnostic` with `Position` on every label; `explain(code)`.
- `crates/musa`: labels, help, note, and fixes rendered; `musa explain <code>`; the summary line; a non-zero exit that
  still prints the count.
- `apps/musa-desktop`: the problems list shows `line:column`, the primary label, and — when a diagnostic carries exactly
  one fix — a control that applies it. Byte offsets leave the interface.
- `docs/rules/desktop/05-states.md`: the problems list's contract updated to match.
- Fixtures: `examples/broken/*.musa` — one file per code worth demonstrating, with `insta` snapshots of the rendered
  terminal output. These are the diagnostics' goldens and the reason a message cannot silently rot.

## Check

```sh
cargo nextest run -p musa-syntax -p musa-compiler -p musa-project -p musa
cargo clippy --all-targets -p musa-syntax -p musa-compiler -p musa-project -p musa -- -D warnings
cargo fmt --check
cargo run -p musa -- check examples/broken/missing-semicolon.musa   # renders with a fix
cargo run -p musa -- explain unknown-name
npm --prefix apps/musa-desktop/ui run check && npm --prefix apps/musa-desktop/ui run test
```

## Stop

- No language changes. Bars and repeats are prompts 57 and 58; this prompt only makes their diagnostics possible.
- No quick-fix menu in the app beyond a single certain fix per diagnostic. A ranked list of candidate fixes is an editor
  feature and needs its own prompt.
- No `ariadne`, and no second renderer. Roadmap §15 names `miette`.
- No LSP. The structure here is what an LSP would need, which is the point, but the server is not this prompt.
