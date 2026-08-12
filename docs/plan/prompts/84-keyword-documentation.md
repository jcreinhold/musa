---
id: 84
slug: keyword-documentation
status: done
depends_on: [77]
phase: 3
---

# Keyword Documentation

## Task

Every keyword in the language carries its own plain-English documentation — what the construct *is* and how to use it —
and hovering a keyword in any editor shows it. The table lives in `musa-language`, which owns the spellings; the
language server serves it over hover, so both editors inherit it; and `musa-project` re-exports it so the desktop's own
hover, when it arrives, reads the same words.

## Read

- `crates/musa-language/src/lexer.rs` — the logos `#[token(...)]` table: the keyword spellings' single source.
- `crates/musa-compiler/src/diagnose.rs` and `musa_project::explain` — the exhaustiveness discipline this table copies:
  a `Code` without an explanation does not compile. A keyword without a doc must not compile either.
- Prompt 77 — the hover feature and its markdown `answer` shape; prompt 78 — `token_at_offset` on the CST, the way a
  position becomes a token.

## Design

### The table

`crates/musa-language/src/keywords.rs`, public:

```rust
pub struct KeywordDoc {
    pub spelling: &'static str,
    /// One clause, for lists and tooltips.
    pub summary: &'static str,
    /// Plain English: what the construct is, how to use it, one `musa` example.
    pub doc: &'static str,
}

pub fn keyword_doc(kind: SyntaxKind) -> Option<&'static KeywordDoc>;
```

The lookup is a wildcard-free `match` over every `*Kw` variant — the workspace lints already forbid wildcard enum
matches, which is what makes the table's completeness a compile error rather than a convention: add `SenzaKw` to the
lexer and the build asks for its doc. `None` answers "not a keyword".

Doc style: two sentences at most — what the construct is, then how or when to use it — plus one fenced `musa` example
line, in the tone of `docs/rules/style-guide.md` (direct, no apology). A keyword a composer cannot hover is a keyword
the language has not finished explaining.

### The hover

`musa-lsp`'s `features/hover.rs` gains a keyword lookup: token at the caret via `musa_language::parse` and
`token_at_offset`; if its kind is a keyword, answer with the doc as markdown (`**tempo** — *summary*`, the doc, the
example fenced). It sits *after* the score-fact lookups: a `use` keyword is its use site, and the expansion is the
better answer there — the law tests pin both answers.

### The export

`musa_project` re-exports `KeywordDoc` and `keyword_doc`, next to `explain`. No desktop wiring: the vocabulary is the
deliverable; the UI consumes it when it builds hover.

## Target

- `crates/musa-language`: `src/keywords.rs` (the table), `src/lib.rs` (module + re-export); a unit test that round-trips
  every spelling through the lexer and requires non-empty summary, doc, and example.
- `crates/musa-lsp`: the keyword lookup in `features/hover.rs`; one law test — hovering `tempo` in the hand-counted
  fixture returns the doc, and hovering a pitch still returns the score fact.
- `crates/musa-project`: the re-export.
- This prompt and its README row.

## Check

```sh
cargo nextest run -p musa-language -p musa-lsp -p musa-project
cargo clippy --all-targets -p musa-language -p musa-lsp -p musa-project -- -D warnings
cargo fmt --check
```

## Stop

- No unit hover docs (`Hz`, `ms`, `s`, `dB`, `bpm`): units lex as `Unit`, not as keywords — a later, smaller addition.
- No desktop UI hover wiring: the export is this prompt's surface.
- No prose links between docs ("see also"): two sentences and an example, or the doc is trying to be the reference
  manual.
- No second copy of any spelling: the test round-trips the table through the lexer itself.
