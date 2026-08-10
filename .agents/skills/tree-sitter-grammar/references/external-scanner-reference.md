# External Scanner Reference

Complete reference for tree-sitter external scanners (`src/scanner.c`).

## When to Use External Scanners

Use when the grammar needs context-sensitive tokenization that cannot be expressed in the declarative grammar DSL:

- **Indentation-sensitive parsing** — indent/dedent tokens (Python, Haskell)
- **Nested block comments** — counting nesting depth (`/* /* */ */`)
- **String interpolation** — tracking delimiter state across interpolation boundaries
- **Context-sensitive keywords** — tokens that are keywords only in certain contexts
- **Complex string literals** — heredocs, raw strings, multiline strings
- **Whitespace sensitivity** — significant newlines, layout rules

## Required C API (5 Functions)

The external scanner must define exactly five functions. Replace `LANG` with your language name.

### `void *tree_sitter_LANG_external_scanner_create()`

Called once when the language is set on a parser. Allocate and return scanner state, or `NULL` for stateless scanners.

```c
#include "tree_sitter/alloc.h"

typedef struct {
    Array(uint16_t) indent_stack;
    bool inside_string;
} Scanner;

void *tree_sitter_musa_external_scanner_create() {
    Scanner *scanner = ts_calloc(1, sizeof(Scanner));
    array_init(&scanner->indent_stack);
    array_push(&scanner->indent_stack, 0);  // initial indent level
    return scanner;
}
```

### `void tree_sitter_LANG_external_scanner_destroy(void *payload)`

Called when parser is deleted or reassigned. Free all heap memory.

```c
void tree_sitter_musa_external_scanner_destroy(void *payload) {
    Scanner *scanner = (Scanner *)payload;
    array_delete(&scanner->indent_stack);
    ts_free(scanner);
}
```

### `unsigned tree_sitter_LANG_external_scanner_serialize(void *payload, char *buffer)`

Called after every successful token recognition. Copy complete state to `buffer`. Return bytes written. Maximum:
`TREE_SITTER_SERIALIZATION_BUFFER_SIZE`.

```c
unsigned tree_sitter_musa_external_scanner_serialize(void *payload, char *buffer) {
    Scanner *scanner = (Scanner *)payload;
    unsigned size = 0;

    buffer[size++] = (char)scanner->inside_string;

    // Serialize indent stack (skip first element — always 0)
    for (unsigned i = 1; i < scanner->indent_stack.size &&
         size < TREE_SITTER_SERIALIZATION_BUFFER_SIZE - 2; i++) {
        uint16_t indent = *array_get(&scanner->indent_stack, i);
        buffer[size++] = (char)(indent & 0xFF);
        buffer[size++] = (char)((indent >> 8) & 0xFF);
    }

    return size;
}
```

### `void tree_sitter_LANG_external_scanner_deserialize(void *payload, const char *buffer, unsigned length)`

Restore state from serialized bytes. Must handle `length == 0` (initial state).

```c
void tree_sitter_musa_external_scanner_deserialize(
    void *payload, const char *buffer, unsigned length
) {
    Scanner *scanner = (Scanner *)payload;

    // Always reset to initial state first
    scanner->inside_string = false;
    scanner->indent_stack.size = 0;
    array_push(&scanner->indent_stack, 0);

    if (length == 0) return;

    unsigned size = 0;
    scanner->inside_string = (bool)buffer[size++];

    // Restore indent stack
    while (size + 1 < length) {
        uint16_t indent = (uint8_t)buffer[size] | ((uint8_t)buffer[size + 1] << 8);
        array_push(&scanner->indent_stack, indent);
        size += 2;
    }
}
```

### `bool tree_sitter_LANG_external_scanner_scan(void *payload, TSLexer *lexer, const bool *valid_symbols)`

The main scanning function. Return `true` if a token was recognized (after setting `lexer->result_symbol`), `false`
otherwise.

```c
enum TokenType {
    STRING_CONTENT,
    BLOCK_COMMENT_CONTENT,
    INDENT,
    DEDENT,
    NEWLINE,
    ERROR_SENTINEL,  // always last
};

bool tree_sitter_musa_external_scanner_scan(
    void *payload, TSLexer *lexer, const bool *valid_symbols
) {
    Scanner *scanner = (Scanner *)payload;

    // Error recovery detection — return false to let tree-sitter handle it
    if (valid_symbols[ERROR_SENTINEL]) return false;

    // Fallthrough pattern: try each scanner, continue on failure
    if (valid_symbols[STRING_CONTENT]) {
        if (scan_string_content(scanner, lexer)) return true;
    }
    if (valid_symbols[BLOCK_COMMENT_CONTENT]) {
        if (scan_block_comment(scanner, lexer)) return true;
    }
    if (valid_symbols[NEWLINE] || valid_symbols[INDENT] || valid_symbols[DEDENT]) {
        if (scan_indentation(scanner, lexer, valid_symbols)) return true;
    }

    return false;
}
```

## TSLexer Interface

### Fields

| Field           | Type       | Description                                                                   |
| --------------- | ---------- | ----------------------------------------------------------------------------- |
| `lookahead`     | `int32_t`  | Next character (Unicode codepoint). `0` at EOF but also for NUL — use `eof()` |
| `result_symbol` | `TSSymbol` | Set to the token type before returning `true`                                 |

### Methods

| Method                       | Signature                        | Description                                                                      |
| ---------------------------- | -------------------------------- | -------------------------------------------------------------------------------- |
| `advance`                    | `void (*)(TSLexer *, bool skip)` | Consume one character. `skip=true` treats it as whitespace (excluded from token) |
| `mark_end`                   | `void (*)(TSLexer *)`            | Mark current position as token end. Enables lookahead beyond token boundary      |
| `get_column`                 | `uint32_t (*)(TSLexer *)`        | Current column (codepoints since line start). Recalculated each call             |
| `is_at_included_range_start` | `bool (*)(const TSLexer *)`      | For multi-language docs — detects range boundaries                               |
| `eof`                        | `bool (*)(const TSLexer *)`      | **Preferred** way to check end-of-file                                           |

### Helper Macros

```c
// Convenient wrappers
static inline void advance(TSLexer *lexer) { lexer->advance(lexer, false); }
static inline void skip(TSLexer *lexer) { lexer->advance(lexer, true); }
static inline bool at_eof(TSLexer *lexer) { return lexer->eof(lexer); }
```

## Patterns

### String Content Scanner

```c
static bool scan_string_content(Scanner *scanner, TSLexer *lexer) {
    bool has_content = false;

    while (!lexer->eof(lexer)) {
        switch (lexer->lookahead) {
            case '"':
                // End of string — don't consume the quote
                if (has_content) {
                    lexer->result_symbol = STRING_CONTENT;
                    return true;
                }
                return false;

            case '\\':
                // Escape sequence — consume backslash + next char
                has_content = true;
                advance(lexer);
                if (!lexer->eof(lexer)) advance(lexer);
                break;

            case '\n':
                // Reject newlines in single-line strings
                if (has_content) {
                    lexer->result_symbol = STRING_CONTENT;
                    return true;
                }
                return false;

            default:
                has_content = true;
                advance(lexer);
                break;
        }
    }

    // EOF while scanning string content
    if (has_content) {
        lexer->result_symbol = STRING_CONTENT;
        return true;
    }
    return false;
}
```

### Nested Block Comment Scanner

```c
static bool scan_block_comment(Scanner *scanner, TSLexer *lexer) {
    bool has_content = false;
    int nesting = 1;  // Already inside one /* from the grammar

    while (!lexer->eof(lexer) && nesting > 0) {
        if (lexer->lookahead == '/') {
            advance(lexer);
            has_content = true;
            if (lexer->lookahead == '*') {
                advance(lexer);
                nesting++;
            }
        } else if (lexer->lookahead == '*') {
            // Mark end BEFORE consuming potential closing delimiter
            lexer->mark_end(lexer);
            advance(lexer);
            if (lexer->lookahead == '/') {
                nesting--;
                if (nesting == 0) {
                    // Don't consume the closing */ — grammar handles it
                    // Token ends at mark_end position (before the *)
                    break;
                }
                advance(lexer);
                has_content = true;
            } else {
                has_content = true;
            }
        } else {
            advance(lexer);
            has_content = true;
        }
    }

    if (has_content) {
        lexer->result_symbol = BLOCK_COMMENT_CONTENT;
        return true;
    }
    return false;
}
```

### Indentation Scanner (Python Pattern)

```c
static bool scan_indentation(
    Scanner *scanner, TSLexer *lexer, const bool *valid_symbols
) {
    // Skip whitespace, counting indent level
    lexer->mark_end(lexer);
    bool found_newline = false;
    uint16_t indent_length = 0;

    while (!lexer->eof(lexer)) {
        if (lexer->lookahead == '\n') {
            found_newline = true;
            indent_length = 0;
            skip(lexer);
        } else if (lexer->lookahead == ' ') {
            indent_length++;
            skip(lexer);
        } else if (lexer->lookahead == '\t') {
            indent_length += 8;  // Tab = 8 spaces (Python convention)
            skip(lexer);
        } else if (lexer->lookahead == '\r') {
            skip(lexer);
        } else {
            break;
        }
    }

    // At EOF, emit remaining DEDENTs
    if (lexer->eof(lexer)) {
        found_newline = true;
        indent_length = 0;
    }

    if (!found_newline) return false;

    uint16_t current_indent = *array_back(&scanner->indent_stack);

    if (indent_length > current_indent && valid_symbols[INDENT]) {
        array_push(&scanner->indent_stack, indent_length);
        lexer->result_symbol = INDENT;
        return true;
    }

    if (indent_length < current_indent && valid_symbols[DEDENT]) {
        array_pop(&scanner->indent_stack);
        lexer->result_symbol = DEDENT;
        return true;  // Will be called again for additional DEDENTs
    }

    if (valid_symbols[NEWLINE]) {
        lexer->result_symbol = NEWLINE;
        return true;
    }

    return false;
}
```

### Float Literals — Use Grammar, NOT External Scanner

**Do NOT use an external scanner for float literal disambiguation.** Grammar-level tokens have higher priority than
external scanner tokens. If `integer_literal` is a grammar token, the internal lexer matches `3` as an integer before
the external scanner gets a chance to match `3.14` as a float.

**Correct approach:** Use `token(prec(N, ...))` in the grammar with higher lexical precedence than the integer rule:

```js
// grammar.js — float has higher lexical precedence than integer
float_literal: _ => token(prec(1, /[0-9]+\.[0-9]+([eE][+-]?[0-9]+)?/)),
integer_literal: _ => token(choice(
  /[0-9][0-9_]*/,
  /0[xX][0-9a-fA-F_]+/,
)),
```

This ensures `3.14` matches as `float_literal` (lexical prec 1) over `integer_literal` (lexical prec 0) at the tokenizer
level, before the parser ever sees the `.`.

## Error Recovery Detection

During error recovery, tree-sitter calls `scan()` with **all** `valid_symbols` set to `true`. The scanner must detect
this to avoid emitting bogus tokens.

### Error Sentinel Pattern (Recommended)

Add an unused token at the end of `externals`:

```js
// grammar.js
externals: $ => [
  $.string_content,
  $.block_comment_content,
  $._error_sentinel,  // never used in any rule
],
```

```c
// scanner.c
enum TokenType {
    STRING_CONTENT,
    BLOCK_COMMENT_CONTENT,
    ERROR_SENTINEL,  // must match order in grammar.js externals
};

bool tree_sitter_musa_external_scanner_scan(...) {
    if (valid_symbols[ERROR_SENTINEL]) return false;
    // ... normal scanning
}
```

### Impossible Combination Pattern (Python)

Check for a combination of tokens that can never be simultaneously valid:

```c
if (valid_symbols[STRING_CONTENT] && valid_symbols[INDENT]) {
    // These are never valid at the same time — must be error recovery
    return false;
}
```

## State Serialization Best Practices

1. **Keep state minimal** — serialized data is stored per syntax tree node for incremental reparsing
1. **Clear state first in `deserialize`** — handle `length == 0` cleanly
1. **Bound serialization size** — check against `TREE_SITTER_SERIALIZATION_BUFFER_SIZE` in loops
1. **Symmetric serialize/deserialize** — any mismatch corrupts incremental parsing
1. **Use `ts_malloc`/`ts_calloc`/`ts_free`** from `tree_sitter/alloc.h`, not libc
1. **Use `Array()` macro** from `tree_sitter/array.h` for dynamic arrays

```c
#include "tree_sitter/array.h"  // Array(), array_push, array_pop, etc.
#include "tree_sitter/alloc.h"  // ts_malloc, ts_calloc, ts_free
```

## Memory Management

### Stateless Scanner (Simplest)

```c
void *create()  { return NULL; }
void destroy(void *p) { }
unsigned serialize(void *p, char *b) { return 0; }
void deserialize(void *p, const char *b, unsigned l) { }
```

### Fixed-Size State

```c
void *create() { return ts_calloc(1, sizeof(Scanner)); }
void destroy(void *p) { ts_free(p); }
```

### Dynamic Arrays

```c
void *create() {
    Scanner *s = ts_calloc(1, sizeof(Scanner));
    array_init(&s->stack);
    return s;
}
void destroy(void *p) {
    Scanner *s = (Scanner *)p;
    array_delete(&s->stack);
    ts_free(s);
}
```

## Common Pitfalls

| Pitfall                                    | Prevention                                                                                                                                |
| ------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------- |
| Infinite loop from zero-width token        | Always advance or return in loops; check `eof()`                                                                                          |
| `result_symbol` set but returns `false`    | Only set `result_symbol` when returning `true`                                                                                            |
| `advance` without `mark_end` for lookahead | Call `mark_end` at every safe boundary (after initial digits, after decimal part, etc.) so failed speculation reverts to correct position |
| `scan()` early return prevents fallthrough | Use `if (scan_x(lexer)) return true;` not `return scan_x(lexer);` — failed scan must allow trying next token type                         |
| Serialization overflow                     | Check `size < TREE_SITTER_SERIALIZATION_BUFFER_SIZE` in loops                                                                             |
| Asymmetric serialize/deserialize           | Test with incremental edits; always clear state before deserialize                                                                        |
| `lookahead == 0` for EOF check             | Use `lexer->eof(lexer)` — NUL characters also have `lookahead == 0`                                                                       |
| Using `advance(false)` for whitespace      | Use `skip(lexer)` (i.e., `advance(true)`) for whitespace that shouldn't be part of any token                                              |
| Forgot error recovery handling             | Always add error sentinel; check at top of `scan()`                                                                                       |
| String keywords in externals               | Including `'if'` in externals forces scanner call for every `if` — use only for non-keyword tokens                                        |
| Wrong token type order                     | `enum TokenType` order must exactly match `externals` array order in grammar.js                                                           |
