# Grammar DSL Reference

Complete reference for tree-sitter's `grammar.js` JavaScript DSL.

## Grammar Object Structure

```js
module.exports = grammar({
  name: "language_name",

  // Tokens that can appear anywhere (whitespace, comments)
  extras: ($) => [/\s/, $.line_comment, $.block_comment],

  // Rules inlined into callers (no CST node produced)
  inline: ($) => [$._type, $._path, $._type_identifier],

  // Intentional GLR conflicts (array of rule arrays)
  conflicts: ($) => [
    [$.call_expression, $.type_application],
    [$._expression, $.pattern],
  ],

  // Tokens handled by external C scanner (src/scanner.c)
  externals: ($) => [
    $.string_content,
    $.block_comment_content,
    $._error_sentinel, // always last — error recovery detection
  ],

  // Named precedence tiers (descending order within each array)
  precedences: ($) => [["member", "call", "unary", "binary_times", "binary_plus"]],

  // Keyword extraction token — prevents "if" matching prefix of "ifM"
  word: ($) => $.identifier,

  // Abstract category nodes queryable as supertypes
  supertypes: ($) => [$._expression, $._type, $._pattern, $._statement],

  // Reserved keyword sets — first is global, rest are contextual
  reserved: {
    global: (_) => ["fn", "let", "if", "else", "match", "return"],
  },

  rules: {
    // Grammar rules here
    source_file: ($) => repeat($._statement),
    // ...
  },
});
```

## Combinators

### `seq(rule1, rule2, ...)`

Match rules in sequence. All must match.

```js
function_def: ($) => seq("fn", field("name", $.identifier), field("body", $.block));
```

### `choice(rule1, rule2, ...)`

Match any one alternative. Order matters for ambiguity: first match wins for lexer tokens.

```js
literal: ($) => choice($.integer, $.float, $.string, $.boolean);
```

### `repeat(rule)` / `repeat1(rule)`

Zero-or-more / one-or-more repetitions.

```js
parameter_list: ($) => seq("(", repeat(seq($.parameter, ",")), optional($.parameter), ")");
// Better with helper:
parameter_list: ($) => seq("(", commaSep($.parameter), ")");
```

### `optional(rule)`

Zero or one occurrence.

```js
return_statement: ($) => seq("return", optional($._expression));
```

### `field(name, rule)`

Assign a field name to a child node. **Critical for query consumers.**

```js
binary_expression: ($) =>
  prec.left(1, seq(field("left", $._expression), field("operator", "+"), field("right", $._expression)));
```

Fields are accessed in queries as `left:`, `right:`, `operator:`:

```scheme
(binary_expression left: (_) @left operator: "+" right: (_) @right)
```

### `token(rule)`

Collapse a complex rule into a single lexer token. Only terminal rules (strings, regexes, `token`, `prec`, `choice`,
`seq` of terminals) allowed inside.

```js
// Single token for line comments
line_comment: (_) => token(seq("//", /.*/));
```

**Lexical vs parse precedence:** `prec()` inside `token()` sets lexical precedence (which token to choose when multiple
match the same characters). `prec()` outside `token()` sets parse precedence (which parse tree to choose).

### `token.immediate(rule)`

Like `token()` but only matches if there is no preceding whitespace. Used for tokens that must be adjacent to the
previous token.

```js
// Escape sequences must follow backslash immediately
escape_sequence: (_) => token.immediate(seq("\\", choice("n", "t", "r", "\\", '"')));
```

### `alias(rule, name)`

Rename a node in the CST. String name = anonymous node; symbol reference = named node.

```js
// Reuse identifier regex but produce a different node type
_type_identifier: $ => alias($.identifier, $.type_identifier),
_field_identifier: $ => alias($.identifier, $.field_identifier),

// Rename an anonymous token to a named node
plus: $ => alias('+', $.plus_operator),
```

### `reserved(wordset, rule)`

Override the global reserved keywords for a specific rule. Used for contextual keywords.

```js
// "async" is a keyword globally, but allowed as identifier in this context
method_name: $ => reserved('method_context', $.identifier),
```

Requires defining the word set in the `reserved` grammar field.

## Precedence

### Parse Precedence: `prec(number, rule)`

Static LR(1) conflict resolution. Higher number = higher precedence = binds tighter.

```js
const PREC = { multiply: 10, add: 9, compare: 8, and: 7, or: 6, assign: 1 };

binary_expression: $ => choice(
  prec.left(PREC.multiply, seq($._expression, choice('*', '/'), $._expression)),
  prec.left(PREC.add, seq($._expression, choice('+', '-'), $._expression)),
  prec.left(PREC.compare, seq($._expression, choice('==', '!='), $._expression)),
  prec.left(PREC.and, seq($._expression, '&&', $._expression)),
  prec.left(PREC.or, seq($._expression, '||', $._expression)),
  prec.right(PREC.assign, seq($._expression, '=', $._expression)),
),
```

### Associativity: `prec.left([n], rule)` / `prec.right([n], rule)`

- `prec.left`: prefer shorter match (left-associative). `1 + 2 + 3` → `(1 + 2) + 3`
- `prec.right`: prefer longer match (right-associative). `a = b = c` → `a = (b = c)`

### Dynamic Precedence: `prec.dynamic(number, rule)`

**Runtime** GLR ambiguity resolution. Both parses are attempted; higher dynamic precedence wins. Only use when static
precedence cannot resolve the ambiguity.

```js
// GLR: this rule produces two valid parses; prefer the one with higher dynamic prec
call_expression: $ => prec.dynamic(1, seq($._expression, $.arguments)),
type_application: $ => prec.dynamic(0, seq($._type, $.type_arguments)),
```

### Named Precedence: `precedences` field

Define named tiers instead of magic numbers:

```js
precedences: $ => [
  ['member', 'call', 'unary', 'binary_multiply', 'binary_add', 'binary_compare'],
],

// Use names:
binary_expression: $ => prec.left('binary_add', seq($._expression, '+', $._expression)),
```

### Lexical Precedence (inside `token()`)

When two tokens can match the same characters, lexical precedence determines which wins:

```js
// "//" could start a comment or be two division operators
// Higher lexical precedence for comment ensures it wins
line_comment: _ => token(prec(1, seq('//', /.*/))),
```

## Configuration Fields Detail

### `extras`

Tokens automatically allowed between any two tokens in any rule. Default: `/\s/`.

```js
extras: $ => [
  /\s/,                    // whitespace
  $.line_comment,          // // comments
  $.block_comment,         // /* */ comments
],
```

Set to `[]` to require explicit whitespace handling (rare — used for whitespace-significant grammars).

### `inline`

Rules mechanically replaced at all call sites. **No CST node is produced.** The inline rule's content appears directly
in the parent. Use for:

- Wrapper rules that organize the grammar but add unwanted tree depth
- Rules used in only 1-2 places where the indirection adds no value

```js
inline: $ => [
  $._type,             // abstract type category
  $._path,             // path expressions
  $._type_identifier,  // alias($.identifier, $.type_identifier)
],
```

**Warning:** Inlining increases grammar size. Don't inline rules used in many places.

### `conflicts`

Declare intentional GLR conflicts. Each entry is an array of rules that conflict:

```js
conflicts: $ => [
  [$._expression, $.pattern],           // Expression and pattern overlap
  [$.call_expression, $.index_expression], // Ambiguous (a)[b] syntax
],
```

Tree-sitter generates a GLR parser that explores both branches. Use `prec.dynamic` to prefer one.

**Every conflict should be understood and documented.** Undocumented conflicts indicate grammar design problems.

### `externals`

Tokens handled by the external C scanner (`src/scanner.c`). The scanner's `scan()` function is called when any of these
tokens is valid.

```js
externals: $ => [
  $.string_content,
  $.block_comment_content,
  $.indent,
  $.dedent,
  $.newline,
  $._error_sentinel,   // Always add as last element
],
```

External tokens are referenced in rules like any other symbol:

```js
string: $ => seq('"', optional($.string_content), '"'),
```

### `word`

The keyword extraction token. Must be a simple terminal rule (usually `$.identifier`). Tree-sitter uses this to ensure
keywords like `if` don't match as prefixes of identifiers like `ifM`.

```js
word: $ => $.identifier,
```

**Caveat: `word` is incompatible with `alias()`.** If `identifier` is used as the source of any `alias()` call (e.g.,
`alias($.identifier, $.type_identifier)`), tree-sitter rejects it as the `word` token with the error "Non-terminal
symbol 'identifier' cannot be used as the word token." This directly conflicts with the best practice of aliasing
identifiers for semantic roles.

**Workarounds:**

1. **Use `reserved` keyword sets** (newer feature) instead of `word` — explicitly list reserved keywords
1. **Omit `word`** and rely on string literal keywords having higher lexical precedence than the identifier regex
1. **Restructure aliases** to not reference `$.identifier` (less practical)

### `supertypes`

Abstract category nodes. They appear in the CST as wrapper nodes that consumers can query generically:

```js
supertypes: $ => [$._expression, $._type, $._pattern],

rules: {
  _expression: $ => choice(
    $.binary_expression,
    $.call_expression,
    $.identifier,
    $.literal,
  ),
}
```

Query consumers can match `(expression)` to catch any expression type, or `(expression/binary_expression)` for a
specific subtype.

## Hidden Rules and Naming Conventions

- **`_`-prefixed rules** (e.g., `$._expression`) are hidden from the CST. They don't produce named nodes — their content
    is inlined into the parent.
- **Regular rules** (e.g., `$.binary_expression`) produce named CST nodes.
- **String literals** (e.g., `'fn'`, `'+'`) produce anonymous CST nodes (appear in tree but unnamed).

**Choose wisely:** Hidden rules reduce tree depth but remove queryable structure. Named rules add depth but enable
precise querying.

## String and Regex Tokens

```js
// String literals — exact match, treated as keywords if matching word token
'fn', 'let', 'if', '==', '+', '=>'

// Regex patterns — character-level matching
/[a-zA-Z_][a-zA-Z0-9_]*/     // identifier
/0[xX][0-9a-fA-F]+/          // hex literal
/\/\/.*/                       // line comment content
```

## Common Helper Functions

```js
// Outside the grammar object:

function commaSep1(rule) {
  return seq(rule, repeat(seq(",", rule)));
}

function commaSep(rule) {
  return optional(commaSep1(rule));
}

function sepBy1(sep, rule) {
  return seq(rule, repeat(seq(sep, rule)));
}

function sepBy(sep, rule) {
  return optional(sepBy1(sep, rule));
}

// Optional trailing separator (Go pattern)
function commaSepTrailing(rule) {
  return seq(rule, repeat(seq(",", rule)), optional(","));
}
```

## Grammar Inheritance

Extend an existing grammar:

```js
module.exports = grammar(require("tree-sitter-base"), {
  name: "extended",
  rules: {
    // Override or add rules
    _statement: ($, original) => choice(original, $.new_statement),
  },
});
```

The `original` parameter gives access to the base grammar's rule.
