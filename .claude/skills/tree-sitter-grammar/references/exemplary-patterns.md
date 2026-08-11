# Exemplary Patterns

Concrete patterns extracted from well-implemented tree-sitter grammars (Rust, Python, Haskell, Go). Use these as
reference when building or improving a grammar.

## Grammar Organization

### PREC Table Pattern (All Major Grammars)

Define a constant precedence table at the top of `grammar.js`:

```js
const PREC = {
  // Highest precedence first
  call: 15,
  field: 14,
  try: 13,
  unary: 12,
  cast: 11,
  multiplicative: 10,
  additive: 9,
  shift: 8,
  bitand: 7,
  bitxor: 6,
  bitor: 5,
  comparative: 4,
  and: 3,
  or: 2,
  range: 1,
  assign: 0,
  closure: -1,
};
```

**Why:** Centralizes all precedence decisions. Easy to audit, reorder, and understand relative priorities.

### Binary Expression Table Pattern (Rust, Go)

Build binary expressions from a `[precedence, operator]` table:

```js
binary_expression: $ => {
  const table = [
    [PREC.and, '&&'],
    [PREC.or, '||'],
    [PREC.bitand, '&'],
    [PREC.bitor, '|'],
    [PREC.bitxor, '^'],
    [PREC.comparative, choice('==', '!=', '<', '>', '<=', '>=')],
    [PREC.shift, choice('<<', '>>')],
    [PREC.additive, choice('+', '-')],
    [PREC.multiplicative, choice('*', '/', '%')],
  ];

  return choice(
    ...table.map(([precedence, operator]) =>
      prec.left(precedence, seq(
        field('left', $._expression),
        field('operator', operator),
        field('right', $._expression),
      )),
    ),
  );
},
```

**Why:** Eliminates per-operator rule definitions. Adding an operator = one table row.

**Non-associative operators:** Tree-sitter has no `prec.none`. Operators like `==`, `!=`, `<`, `>` that should be
non-associative (reject `a == b == c`) must use `prec.left` as a fallback. Enforce non-associativity in semantic
analysis, not the grammar. Using bare `prec()` without left/right causes "Unresolved conflict" errors.

### Helper Functions (All Grammars)

```js
// Place OUTSIDE the grammar object, at file top level

function commaSep1(rule) {
  return seq(rule, repeat(seq(",", rule)));
}

function commaSep(rule) {
  return optional(commaSep1(rule));
}

// With optional trailing comma (Go, Rust)
function commaSepTrailing(rule) {
  return seq(rule, repeat(seq(",", rule)), optional(","));
}

// Generic separator
function sepBy1(sep, rule) {
  return seq(rule, repeat(seq(sep, rule)));
}

function sepBy(sep, rule) {
  return optional(sepBy1(sep, rule));
}
```

### Alias Identifiers for Semantic Roles (All Grammars)

Reuse a single identifier regex with different semantic names:

```js
rules: {
  identifier: _ => /[a-zA-Z_\u00C0-\u00FF][a-zA-Z0-9_\u00C0-\u00FF]*/,

  // Aliased variants (placed in `inline` for CST cleanliness)
  _type_identifier: $ => alias($.identifier, $.type_identifier),
  _field_identifier: $ => alias($.identifier, $.field_identifier),
  _module_identifier: $ => alias($.identifier, $.module_identifier),
},

inline: $ => [$._type_identifier, $._field_identifier, $._module_identifier],
```

**Usage in rules:**

```js
struct_item: $ => seq(
  'struct',
  field('name', $._type_identifier),   // produces type_identifier node
  field('body', $.field_declaration_list),
),

field_declaration: $ => seq(
  field('name', $._field_identifier),   // produces field_identifier node
  ':',
  field('type', $._type),
),
```

**Why:** One identifier regex, multiple semantic types. Query consumers get precise `(type_identifier)` vs
`(field_identifier)` vs `(identifier)` matching.

### Modular Grammar Organization (Haskell)

Split large grammars across files:

```
grammar/
  exp.js       # Expressions
  type.js      # Types
  pat.js       # Patterns
  decl.js      # Declarations
  literal.js   # Literals
  general.js   # Shared helpers
```

```js
// grammar.js
const exp = require("./grammar/exp.js");
const type = require("./grammar/type.js");
const pat = require("./grammar/pat.js");
const decl = require("./grammar/decl.js");

module.exports = grammar({
  name: "haskell",
  rules: {
    ...exp,
    ...type,
    ...pat,
    ...decl,
    module: ($) => seq(/* ... */),
  },
  externals: ($) => [
    /* ... */
  ],
  conflicts: ($) => [
    /* ... */
  ],
  // etc.
});
```

Each module file exports an object of rule definitions:

```js
// grammar/exp.js
module.exports = {
  _expression: $ => choice(
    $.binary_expression,
    $.application_expression,
    $.lambda_expression,
    // ...
  ),

  binary_expression: $ => /* ... */,
  application_expression: $ => /* ... */,
  lambda_expression: $ => /* ... */,
};
```

**Why:** Keeps individual files under 200 lines. Enables parallel development. Easier code review.

## Expression Flattening

### The Problem: Deep Nesting

A naive grammar for `1 + 2`:

```
(source_file
  (expression_statement
    (binary_expression
      (unary_expression
        (postfix_expression
          (primary_expression
            (integer_literal))))
      (unary_expression
        (postfix_expression
          (primary_expression
            (integer_literal)))))))
```

### The Fix: Hidden Rules + Supertypes

```js
supertypes: $ => [$._expression],

rules: {
  _expression: $ => choice(
    $.binary_expression,
    $.unary_expression,
    $.call_expression,
    $.field_expression,
    $.identifier,
    $.literal,
    $.parenthesized_expression,
  ),

  binary_expression: $ => /* ... uses $._expression ... */,
  unary_expression: $ => prec(PREC.unary, seq(
    field('operator', choice('-', '!', '~')),
    field('operand', $._expression),
  )),
}
```

Result for `1 + 2`:

```
(source_file
  (expression_statement
    (binary_expression
      left: (integer_literal)
      operator: "+"
      right: (integer_literal))))
```

**Key:** `_expression` is hidden (no CST node) and flat (all expression types at same level).

## Type/Expression Disambiguation

### Go's Generic Type Ambiguity

`f[T]` could be index expression or type instantiation. Go declares the conflict and uses dynamic precedence:

```js
conflicts: $ => [
  [$._simple_type, $._expression],
  [$.qualified_type, $._expression],
  // ...
],

type_instantiation_expression: $ => prec.dynamic(1, seq(
  field('type', $._type),
  field('type_arguments', $.type_arguments),
)),
```

### Rust's Zero-Conflict Approach

Rust avoids conflicts entirely by using distinct token sequences:

- Type arguments: `foo::<T>` (turbofish)
- Comparison: `foo < bar`

The `::<` token makes the grammar unambiguous without GLR.

**Lesson:** If you can add a syntactic disambiguator (like turbofish), prefer it over GLR conflicts.

## Keyword Handling

### Using `word` Token

```js
word: $ => $.identifier,

rules: {
  identifier: _ => /[a-zA-Z_][a-zA-Z0-9_]*/,
}
```

With `word` set, tree-sitter ensures that `if` in a rule like:

```js
if_expression: ($) => seq("if", $._expression, $.block);
```

will NOT match `ifM` or `iffy`. The keyword `if` is extracted from the word token pattern.

### Reserved Keywords

```js
reserved: {
  global: _ => ['fn', 'let', 'if', 'else', 'match', 'return', 'true', 'false'],
  // Contextual keywords — reserved only in certain rules
  type_context: _ => ['where'],
},

rules: {
  // "where" is a keyword here
  where_clause: $ => seq(reserved('type_context', 'where'), /* ... */),
  // "where" can be an identifier elsewhere
  identifier: _ => /[a-zA-Z_][a-zA-Z0-9_]*/,
}
```

## Field Name Patterns

### Consistent Field Naming (Rust)

```js
function_item: $ => seq(
  optional($.visibility_modifier),
  optional('async'),
  'fn',
  field('name', $.identifier),
  field('type_parameters', optional($.type_parameters)),
  field('parameters', $.parameters),
  field('return_type', optional(seq('->', $._type))),
  field('body', $.block),
),
```

**Convention:** Use consistent field names across similar constructs:

- `name` for the defining identifier
- `body` for the block/expression body
- `parameters` / `arguments` for parameter/argument lists
- `type` / `return_type` for type annotations
- `left` / `right` / `operator` for binary expressions
- `operand` / `operator` for unary expressions
- `condition` for if/while conditions
- `consequence` / `alternative` for if branches
- `value` for the right side of assignments/bindings

## Error Recovery Patterns

### Rust's Approach: No External Error Tokens

Rust avoids external scanner complexity by keeping all tokenization in the grammar DSL. Error recovery relies on
tree-sitter's built-in mechanisms:

- `extras` (whitespace/comments) auto-skipped
- Grammar structure provides recovery points (`;`, `}`, etc.)

### Python's Approach: Scanner-Aware Error Recovery

Python uses the "impossible combination" pattern:

```c
if (valid_symbols[STRING_CONTENT] && valid_symbols[INDENT]) {
    return false;
}
```

And also adds bracket closers as externals so the scanner can suppress DEDENT inside brackets:

```js
externals: $ => [
  $._newline, $._indent, $._dedent,
  $.string_start, $.string_content, $.string_end,
  $.comment, $.DEDENT,
  ']', ')', '}',  // bracket closers suppress indentation changes
  'except',
],
```

## Unicode Support

### Rust's Identifier Pattern

```js
identifier: _ => /(r#)?[_\p{XID_Start}][_\p{XID_Continue}]*/,
```

### Go's Simple Unicode Range

```js
identifier: _ => /[a-zA-Z_\u00C0-\u00FF][a-zA-Z0-9_\u00C0-\u00FF]*/,
```

### Musa-Specific: Unicode Operators

Musa uses Unicode operators (`→`, `⊔`, `⊓`, `⊢`). These should be explicit tokens:

```js
// In the grammar:
_arrow: _ => choice('->', '→'),
_join: _ => choice('max', '⊔'),
_meet: _ => choice('min', '⊓'),
_turnstile: _ => choice('|-', '⊢'),
```

## Conflict Resolution Strategy

### Step 1: Understand the Conflict

Run `tree-sitter generate --report-states-for-rule RULE` to see which states have conflicts.

### Step 2: Classify

| Type | Resolution |
| --- | --- |
| Operator precedence | `prec.left` / `prec.right` with PREC table |
| Lexical ambiguity | `token(prec(N, ...))` |
| Type/expression overlap | Syntactic disambiguator (turbofish) or `prec.dynamic` + `conflicts` |
| Keyword sensitivity | `reserved` sets |
| Context-sensitive tokens | External scanner |

### Step 3: Prefer Static Over Dynamic

1. **Restructure grammar** to eliminate ambiguity (best)
1. **Static precedence** with `prec` / `prec.left` / `prec.right` (good)
1. **Lexical precedence** with `token(prec(...))` (for token conflicts)
1. **`conflicts` + `prec.dynamic`** (last resort for genuine GLR ambiguity)

### Step 4: Document Each Conflict

For every entry in `conflicts`, add a comment explaining:

- What the ambiguity is
- Why it can't be resolved statically
- Which branch `prec.dynamic` prefers and why

```js
conflicts: $ => [
  // `f(x)` could be function call or type constructor application
  // Prefer function call (dynamic=1) over type constructor (dynamic=0)
  [$.call_expression, $.type_constructor],
],
```

## Test Coverage Strategy

### Minimum Tests Per Rule

1. **Basic case** — simplest valid input
1. **Nested/recursive case** — rule used inside itself
1. **Edge case** — empty optionals, trailing separators
1. **Precedence case** — operators at different levels
1. **Error case** (`:error` attribute) — known invalid inputs that should produce ERROR nodes

### Field Name Verification

Always include field names in test S-expressions. **Important:** tree-sitter test output omits the text of anonymous
nodes — write `operator:` not `operator: "+"`. Use `tree-sitter test -u` to auto-generate correct S-expressions, then
verify field names are present.

```
==================
Function with return type
==================
fn add(x: Int, y: Int) -> Int { x + y }
---
(source_file
  (function_definition
    name: (identifier)
    parameters: (parameter_list
      (parameter name: (identifier) type: (type_identifier))
      (parameter name: (identifier) type: (type_identifier)))
    return_type: (type_identifier)
    body: (block
      (binary_expression
        left: (identifier)
        right: (identifier)))))
```

Note: `operator:` is omitted because `"+"` is an anonymous node. Tree-sitter test format only shows named nodes and
field labels.

### Test-Driven Grammar Development

1. Write the test case first (expected S-expression)
1. Run `tree-sitter test -i "Test name"` — watch it fail
1. Add/modify the grammar rule
1. Run `tree-sitter generate && tree-sitter test -i "Test name"` — watch it pass
1. Run full test suite to check for regressions

### Useful Test Commands

```bash
# Generate parser and run all tests
tree-sitter generate && tree-sitter test

# Run specific test by name
tree-sitter test -i "Function with return type"

# Auto-update expected output (for bulk test creation)
tree-sitter test -u

# Parse a real file and see the tree
tree-sitter parse examples/sample.musa

# Parse with debug output (shows shift/reduce actions)
tree-sitter parse -d examples/sample.musa

# Check highlight captures against standard names
tree-sitter highlight --check examples/sample.musa

# Run query against file
tree-sitter query queries/highlights.scm examples/sample.musa

# Fuzz test the parser
tree-sitter fuzz

# Interactive playground
tree-sitter playground
```
