---
name: tree-sitter-grammar
description: Use for tree-sitter grammars (grammar.js, queries/*.scm, test/corpus/*.txt); wrong parse trees, missing highlights, grammar conflicts, editors/tree-sitter-musa/, the drift law against crates/musa-language.
---

# Tree-Sitter Grammar Authoring

## Overview

Tree-sitter is an incremental parsing framework that generates fast, error-tolerant parsers from declarative grammar
specifications. A grammar consists of:

- **`grammar.js`** — Declarative grammar specification using a JavaScript DSL
- **`src/scanner.c`** — Optional external scanner for context-sensitive tokens (indentation, string interpolation,
  nested comments). **musa needs none** — semicolons and braces are explicit (roadmap §7).
- **`queries/*.scm`** — S-expression query files for syntax highlighting, scoping, code navigation, folding, and
  indentation
- **`test/corpus/*.txt`** — Parser test cases in tree-sitter's test format

**Core principle:** Tree-sitter grammars describe _concrete syntax trees_ (CSTs), not abstract syntax trees. Every token
appears in the tree. Design for incremental reparsing and error recovery — the parser must produce useful trees even for
incomplete/invalid input.

## Critical Rule: Read the Source-of-Truth Parser First

**Before writing any tree-sitter rule, read the authoritative parser (`crates/musa-language/src/parser.rs`) and lexer
(`crates/musa-language/src/lexer.rs`) to understand:**

- Exact token dispatch (what leading token triggers which production — musa's hand parser dispatches one statement per
  first token, e.g. `Parser::voice_items`)
- Disambiguation strategy (lookahead, context, position, restricted sub-grammars)
- What the construct's syntax actually is (don't guess from examples)
- Whether an apparent ambiguity is real or resolved by parser structure

**Every grammar rule should trace back to a specific function or production in the authoritative parser, and every token
to a regex or literal in the lexer.** Node names mirror `syntax_kind.rs`, snake_cased, so queries read in the language's
own vocabulary. Where the trees disagree, the hand parser is right and the grammar changes — the drift law
(`crates/musa-language/tests/tree_sitter_fixtures.rs` + `editors/tree-sitter-musa/test/compare-tokens.js`) is what
notices.

## When to Use

- Writing or modifying `editors/tree-sitter-musa/grammar.js` rules
- Writing or fixing `queries/highlights.scm`, `locals.scm`, `indents.scm`, `folds.scm`, `outline.scm`, `tags.scm`
- Adding test cases (`test/corpus/*.txt`)
- Resolving grammar conflicts or precedence issues
- Improving parse tree structure (reducing nesting, adding field names)
- Keeping the grammar in lockstep with `crates/musa-language` under the drift law

**When NOT to use:** For the hand-written lexer/parser itself (that's `musa-language` work), PEG grammars, or
non-tree-sitter parser generators.

## Quick Reference

### Grammar DSL Combinators

| Combinator | Purpose | Example |
| --- | --- | --- |
| `seq(a, b, ...)` | Sequence | `seq('part', $.name, $.body)` |
| `choice(a, b, ...)` | Alternatives | `choice($.note_statement, $.rest_statement)` |
| `repeat(rule)` | Zero or more | `repeat($.statement)` |
| `repeat1(rule)` | One or more | `repeat1($.parameter)` |
| `optional(rule)` | Zero or one | `optional($.return_type)` |
| `field(name, rule)` | Named child | `field('name', $.identifier)` |
| `token(rule)` | Single lexer token | `token(seq('/', /[^/]*/, '/'))` |
| `token.immediate(rule)` | No preceding whitespace | `token.immediate(/[a-z]+/)` |
| `alias(rule, name)` | Rename node | `alias($.identifier, $.type_identifier)` |
| `prec(n, rule)` | Parse precedence | `prec(10, seq($.expr, '*', $.expr))` |
| `prec.left([n], rule)` | Left-associative | `prec.left(1, seq($.expr, '+', $.expr))` |
| `prec.right([n], rule)` | Right-associative | `prec.right(1, seq($.expr, '=', $.expr))` |
| `prec.dynamic(n, rule)` | Runtime GLR precedence | For genuine ambiguities only |

### Grammar Configuration Fields

| Field | Purpose |
| --- | --- |
| `extras` | Tokens allowed anywhere (whitespace, comments) |
| `inline` | Rules removed from CST by inlining |
| `conflicts` | Intentional GLR conflict declarations |
| `externals` | Tokens handled by external C scanner |
| `precedences` | Named precedence levels |
| `word` | Keyword extraction token (see caveat below) |
| `supertypes` | Abstract category nodes for queries |
| `reserved` | Reserved keyword sets |

### CLI Commands (in `editors/tree-sitter-musa/`)

| Command | Purpose |
| --- | --- |
| `npm run generate` | Generate parser from grammar.js |
| `npm test` | Generate + corpus tests + **drift law** (`compare-tokens.js`) |
| `npm run test:corpus` | Corpus tests only |
| `npm run test:update` | Auto-update expected test output |
| `tree-sitter parse FILE` | Parse a file, show S-expression |
| `tree-sitter parse -d FILE` | Parse with debug output |
| `tree-sitter highlight --check` | Validate capture names |
| `tree-sitter query QUERY FILE` | Run query against file |

## Best Practices

### Grammar Structure

1. **Define a `PREC` table** at the top of `grammar.js` mapping operator names to numeric precedence levels. Never use
   bare `prec(10, ...)`.

1. **Mirror the hand parser's dispatch tables with named constants.** musa's grammar defines `BAR_ITEMS` and
   `VOICE_ITEMS` arrays that are the mirror image of `Parser::voice_items` — one statement per first token, exactly as
   the hand parser dispatches. When a statement is legal in a new position, update the constant and the parser function
   together.

1. **Use helper functions** outside the grammar object (`commaSep1`, `commaSep`).

1. **Build binary expressions from a table** — the standard pattern. Note: tree-sitter has no `prec.none` for
   non-associative operators (like `==`, `<`). Use `prec.left` as fallback; enforce non-associativity in semantic
   analysis.

1. **Use `field()` on EVERY semantically meaningful child** — this is the primary query interface for consumers. A rule
   without field names is almost always wrong.

1. **Use supertypes** for abstract categories (`_expression`, `_type`, `_statement`). Prefix hidden rules with `_`.
   Declare supertypes in the grammar config.

1. **Use `inline`** for wrapper rules that organize the grammar but should not appear in the CST.

1. **Alias identifiers** for semantic roles:

    ```js
    _type_identifier: $ => alias($.identifier, $.type_identifier),
    ```

1. **Set the `word` token** to enable keyword extraction. **Caveat:** `word` is incompatible with `alias()` on the same
   rule. Workaround: omit `word` and rely on `reserved` keyword sets, or restructure aliases.

1. **Prefer static precedence** over `prec.dynamic`. Note: `prec.dynamic` often resolves ambiguities WITHOUT needing a
   `conflicts` declaration. Try `prec.dynamic` first; only add `conflicts` if tree-sitter still reports an unresolved
   conflict.

1. **Match `source_file` to the language's actual top-level structure.** If the language only allows items (parts,
   scores, motifs, imports) at the top level — not bare statements — then `source_file` should be `repeat($._item)`, not
   `repeat($._statement)`.

1. **Modularize large grammars** by splitting rules across files using `require()` and object spread.

### Query Files

1. **Use Zed's capture taxonomy as the target** — musa's editor targets are Zed (`zed-musa`) and VS Code (`vscode-musa`,
   which uses its own TextMate grammar). Zed uses a simpler capture set than nvim-treesitter: `@keyword` (not
   `@keyword.conditional`), `@function` (not `@function.call`), `@variable.special` (not `@variable.builtin`). See
   **zed-query-files-reference.md** for the authoritative Zed capture table.

1. **Pattern ordering matters** — more specific patterns should come after general ones. The last matching pattern wins.

1. **Use `#match?` for convention-based highlighting:**

    ```scheme
    ((identifier) @constant (#match? @constant "^[A-Z][A-Z\\d_]+$"))
    ```

1. **Use `#any-of?` for builtin sets** (dynamics, articulations, clefs):

    ```scheme
    ((identifier) @constant.builtin
      (#any-of? @constant.builtin "pp" "p" "mp" "mf" "f" "ff"))
    ```

### External Scanners

musa's grammar deliberately has **no external scanner**: semicolons and braces are explicit (roadmap §7), so the
one-token lookahead LR(1) gives is enough for the places the hand parser peeks (`name =`, `name(`, bare `name`). Before
adding one, confirm the ambiguity is truly context-sensitive (token identity depends on parse state) — see the
Precedence Taxonomy. If you do add one, follow **external-scanner-reference.md** and its checklist (error sentinel last,
`lexer->eof`, `mark_end` at safe boundaries, fallthrough scanning, minimal serialized state, `ts_malloc`/`ts_free`).

### Testing and the drift law

1. **Write tests continuously** — one test per rule permutation. Test format:

    ```
    ==================
    Test name
    ==================
    source code
    ---
    (expected_s_expression)
    ```

1. **Test attributes:** `:skip`, `:error` (expects parse error), `:fail-fast`, `:language(LANG)`.

1. **Anonymous nodes omit text in test output.** Write `operator:` not `operator: "+"`. Use `tree-sitter test -u` to
   auto-generate correct expectations, then verify field names are present.

1. **The drift law is the binding check.** `crates/musa-language/tests/tree_sitter_fixtures.rs` commits the *real*
   lexer's token stream for every compilable fixture and the *real* parser's verdict on every broken one;
   `test/compare-tokens.js` parses each fixture through the CLI's `--cst` and compares token for token. A grammar that
   disagrees with the lexer about a single token fails in CI. Run `npm test` before committing grammar changes.

1. **Commit all three generated files** — `src/parser.c`, `src/grammar.json`, `src/node-types.json`. The Zed extension
   compiles `parser.c` from the pinned rev; forgetting `grammar.json` or `node-types.json` causes silent failures
   downstream.

## Precedence Taxonomy

Tree-sitter has **four** distinct disambiguation mechanisms. Using the wrong one is a common source of bugs.

| Type | Mechanism | When to Use | Syntax |
| --- | --- | --- | --- |
| **Lexical** | Which _token_ wins when multiple tokens match same characters | `//` comment vs `/` duration; duration `/8` vs division | `token(prec(N, ...))` — `prec` **inside** `token()` |
| **Parse** | Which _parse tree_ wins when same token sequence has multiple valid parses | Operator precedence; associativity | `prec(N, ...)`, `prec.left(N, ...)`, `prec.right(N, ...)` — **outside** `token()` |
| **Context-sensitive** | Which _token to emit_ depends on parse state | (musa avoids these by design: explicit `;` and `{}`) | External scanner checking `valid_symbols` |
| **Token priority** | Grammar-level tokens always win over external scanner tokens at same position | Prefer `token(prec(N,...))` in grammar over an external scanner | Use `token(prec(N,...))` in grammar, not external scanner |

**Critical rule:** If the token _identity_ (not just the parse tree) depends on what the parser has already seen, you
need an **external scanner**. Neither lexical nor parse precedence can solve this.

## Statement-Boundary Ambiguity (musa's version of greedy matching)

Tree-sitter's LR parser greedily extends expressions — and, in a notation language, events. musa's canonical case: in
`root/8 tenuto`, whether `tenuto` is this note's articulation or the next event's pitch is settled by what *follows* it.
The hand parser looks ahead one token *kind*; the tree-sitter grammar resolves the same boundary with GLR and declared
`conflicts` plus `prec.dynamic` — see the comment block at the top of `grammar.js`.

When a new statement form creates a similar boundary ambiguity, follow the preference order in Conflict Resolution — and
never change the language syntax to accommodate tree-sitter. The grammar must accept exactly what `crates/musa-language`
accepts.

## Conflict Resolution Strategy

When you encounter a grammar ambiguity, follow this preference order (best to worst):

1. **Restructure the grammar** to eliminate the ambiguity entirely
1. **Static precedence** with `prec` / `prec.left` / `prec.right` and a PREC table
1. **Lexical precedence** with `token(prec(N, ...))` for token-level conflicts
1. **`token.immediate`** for whitespace-sensitive disambiguation
1. **`prec.dynamic`** alone — often resolves the ambiguity without needing a `conflicts` declaration
1. **External scanner** for context-sensitive tokens (musa should never need this; see above)
1. **`conflicts` + `prec.dynamic`** as last resort

**Every `conflicts` entry must be documented** with: what the ambiguity is, why other approaches cannot resolve it, and
which branch `prec.dynamic` prefers. `grammar.js` does this in its header comment — keep it current.

## Common Mistakes

| Mistake | Fix |
| --- | --- |
| Writing rules from syntax examples, not the parser | Read `parser.rs`/`lexer.rs` first. Trace each rule to a specific parser function. |
| Grammar drifts from the real lexer | Run `npm test` (drift law) before committing; update fixtures deliberately. |
| Deep expression nesting (6+ levels for `1 + 2`) | Flatten with `_`-prefixed hidden rules and `inline` |
| Conflicts declared but not understood | Each conflict = a grammar ambiguity. Refactor the grammar or add targeted `prec` |
| `word` token with `alias()` identifiers | `word` is incompatible with rules used in `alias()`. Use `reserved` sets or omit `word`. |
| `prec.dynamic` overuse | Use only for genuine GLR ambiguities; prefer static precedence |
| `prec.dynamic` + redundant `conflicts` | Try `prec.dynamic` alone first. Add `conflicts` only if tree-sitter still reports unresolved. |
| Lexical vs parse precedence confusion | `prec()` = parse precedence; `token(prec())` = lexical precedence. See Precedence Taxonomy. |
| Missing field names | Consumers can't query children without field names. Every semantic child needs `field()` |
| Magic precedence numbers | Define `const PREC = { ... }` table; never use bare `prec(10, ...)` |
| No `supertypes` declared | Add `supertypes: $ => [$._expression, $._statement, ...]` for abstract categories |
| Non-associative operators cause conflicts | Tree-sitter has no `prec.none`. Use `prec.left`; enforce non-associativity in semantic analysis. |
| `source_file` doesn't match language top-level | If language only allows items at top level, use `repeat($._item)`, not `repeat($._statement)`. |
| Test expects `operator: "+"` for anonymous nodes | Tree-sitter omits anonymous text in tests. Write `operator:` only. Use `tree-sitter test -u`. |
| Forgetting generated files in the commit | Commit `src/parser.c`, `src/grammar.json`, `src/node-types.json` — zed-musa pins by rev. |

## Musa-Specific Guidance

The musa tree-sitter grammar lives at `editors/tree-sitter-musa/`. The authoritative parser is
`crates/musa-language/src/parser.rs` (with `lexer.rs` and `syntax_kind.rs`). Key musa facts requiring careful grammar
design:

- **One statement per first token.** The hand parser dispatches on the leading token; the grammar mirrors this with the
  `BAR_ITEMS` and `VOICE_ITEMS` constants. Some statements (tempo, meter, key, clef, ...) are legal in the header, in a
  part, *and* inside a voice — one node kind, several positions, exactly as the parser has it.
- **Event-boundary ambiguity** (`root/8 tenuto`) — see the Statement-Boundary section. Resolved with declared
  `conflicts` + `prec.dynamic`, documented in the `grammar.js` header.
- **No external scanner** — explicit `;` and `{}` (roadmap §7). Don't add one without confirming true
  context-sensitivity.
- **Durations** (`/8`, `/4.`, tuplets) and **pitch spellings** are lexer-level; every token must match `lexer.rs`
  exactly or the drift law fails.
- **The language spec is layered:** `docs/language/` (candidate elaboration-language spec) governed by
  `docs/language-correction.md`. Grammar changes for new surface syntax land with their prompt; check `docs/prompts/`
  for the owning prompt.
- **Node names mirror `syntax_kind.rs`, snake_cased** — query files (`highlights.scm`, `outline.scm`, `indents.scm`,
  `folds.scm`, `locals.scm`, `tags.scm`) should read in the language's own vocabulary. `outline.scm` also feeds
  zed-musa's outline panel; keep its captures in Zed's taxonomy.
- **Capture keywords by parent node, never by bare text.** Any keyword that can appear in a name position — the members
  of `parser.rs`'s `MODULE_NAME` (`harmony`, `list`, `option`, `pitch`, `scale`), or the statement keywords
  `name_expression` borrows — must never sit in a flat `[ ... ] @keyword` list: the list fires on token text and paints
  `std::harmony` as a keyword. Scope the keyword positions (`(harmony_declaration "harmony" @keyword)`) and capture the
  name positions positively (`(import_statement "harmony" @module)`). The authoritative contextual classification is
  `musa_language::classify` (highlight.rs); the VS Code TextMate grammar gets the same words from the generated
  `module-names.json` fixture. Never `alias` a keyword token to `identifier` to fix this — the drift law compares leaf
  names with the real lexer and will fail.
- **After grammar changes:** `npm test` (generate + corpus + drift law), commit all three generated files, then update
  zed-musa's `extension.toml` rev to the new commit SHA (see the zed-extension skill).

## Supporting References

For detailed API documentation, see:

- **references/grammar-dsl-reference.md** — Complete grammar.js DSL API with all combinators, configuration fields, and
  usage details
- **references/query-patterns-reference.md** — Full query syntax, predicates, directives, and patterns for all query
  file types (nvim-treesitter capture conventions — useful background)
- **references/zed-query-files-reference.md** — Zed-specific query files: authoritative capture taxonomy, outline.scm,
  brackets.scm, indents.scm, overrides.scm, textobjects.scm, runnables.scm, semantic_token_rules.json (this is the
  binding target for musa's queries)
- **references/external-scanner-reference.md** — External scanner C API, TSLexer interface, state serialization, error
  recovery, and memory management (musa has no scanner; read only if that ever changes)
- **references/exemplary-patterns.md** — Concrete patterns extracted from tree-sitter-rust, tree-sitter-python,
  tree-sitter-haskell, and tree-sitter-go
