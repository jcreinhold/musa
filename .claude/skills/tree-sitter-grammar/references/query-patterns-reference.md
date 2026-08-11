# Query Patterns Reference

Complete reference for tree-sitter query files (`queries/*.scm`).

## Query Syntax

### Basic Patterns

S-expressions matching CST nodes:

```scheme
; Match any binary_expression node
(binary_expression)

; Match with specific children
(binary_expression (number_literal) (number_literal))

; Match anonymous nodes (keywords, operators) with quotes
(binary_expression operator: "!=")
```

### Field Names

Prefix child patterns with `field_name:`:

```scheme
(function_definition
  name: (identifier) @func_name
  parameters: (parameter_list) @params
  body: (block) @body)
```

### Negated Fields

Match nodes that LACK a field with `!field_name`:

```scheme
(function_definition
  name: (identifier) @func_name
  !return_type)  ; functions without return type annotation
```

### Captures

Name nodes with `@capture_name`:

```scheme
(identifier) @variable
(type_identifier) @type
"fn" @keyword
```

### Wildcards

- `(_)` — matches any named node
- `_` — matches any named or anonymous node

### Special Nodes

- `(ERROR)` — parse error nodes
- `(MISSING)` — inserted missing tokens
- `(MISSING identifier)` — specific missing token type
- `(MISSING ";")` — specific missing anonymous token

### Supertype Matching

```scheme
; Match any expression subtype
(expression) @expr

; Match specific subtype via supertype
(expression/binary_expression) @bin_expr
```

### Quantification

- `(node)+` — one or more
- `(node)*` — zero or more
- `(node)?` — optional

```scheme
(comment)+ @doc_comment          ; one or more consecutive comments
(class_declaration (decorator)* @dec)  ; zero or more decorators
(arguments (string)? @str_arg)   ; optional string argument
```

### Grouping

Parentheses group sibling nodes for quantification:

```scheme
(
  (number)
  ("," (number))*
)
```

### Alternations

Square brackets `[]` for alternatives:

```scheme
; Match either keyword
["break" "continue" "return"] @keyword

; Match different node types
(call_expression
  function: [
    (identifier) @function
    (member_expression property: (property_identifier) @method)
  ])
```

### Anchors (`.` operator)

Constrain child position within parent:

```scheme
(array . (identifier) @first_element)           ; must be FIRST named child
(block (_) @last_statement .)                    ; must be LAST named child
(dotted_name (identifier) @prev . (identifier) @next)  ; must be ADJACENT siblings
```

Anchors ignore anonymous nodes.

## Predicates (filter matches)

### `#eq?` — exact text equality

```scheme
; Capture text equals string literal
((identifier) @name (#eq? @name "self"))

; Two captures have same text
((binary_expression left: (identifier) @a right: (identifier) @b)
 (#eq? @a @b))
```

### `#not-eq?` — negated equality

```scheme
((identifier) @var (#not-eq? @var "self"))
```

### `#match?` — regex match

```scheme
; ALL_CAPS identifiers are constants
((identifier) @constant (#match? @constant "^[A-Z][A-Z\\d_]+$"))

; Capitalized identifiers are constructors
((identifier) @constructor (#match? @constructor "^[A-Z]"))
```

### `#not-match?` — negated regex

```scheme
((identifier) @variable (#not-match? @variable "^[A-Z]"))
```

### `#any-of?` — match against set

```scheme
((identifier) @type.builtin
  (#any-of? @type.builtin "Int64" "Float64" "String" "Bool" "Unit" "Type" "Nat"))

((identifier) @variable.builtin
  (#any-of? @variable.builtin "self" "super" "module"))
```

### `#is?` / `#is-not?` — check properties

```scheme
; Only highlight as function if not a local variable
((identifier) @function.call
 (#is-not? local))
```

### Quantified Variants

- `#any-eq?` / `#any-not-eq?` — any node in quantified capture matches
- `#any-match?` / `#any-not-match?` — any node in quantified capture matches regex

## Directives (set metadata)

### `#set!` — associate key-value pairs

```scheme
((comment) @injection.content
  (#set! injection.language "markdown"))

; Set priority for overlapping highlights
((identifier) @type (#set! priority 110))
```

### `#select-adjacent!` — filter to adjacent nodes

```scheme
(
  (comment)* @doc
  .
  (function_definition name: (identifier) @name) @definition.function
  (#select-adjacent! @doc @definition.function)
)
```

### `#strip!` — remove regex-matched text from capture

```scheme
(
  (comment)* @doc
  .
  (function_definition name: (identifier) @name) @definition.function
  (#strip! @doc "^//\\s*")
)
```

______________________________________________________________________

## Highlight Capture Naming Conventions

Standard names from nvim-treesitter. Theme matching uses longest prefix match.

### Identifiers

| Capture | Use For |
| --- | --- |
| `@variable` | General variables |
| `@variable.builtin` | `self`, `this`, `super` |
| `@variable.parameter` | Function parameters |
| `@variable.parameter.builtin` | Special parameters (`cls` in Python) |
| `@variable.member` | Struct/object fields |
| `@constant` | Constants |
| `@constant.builtin` | `true`, `false`, `nil`, `None` |
| `@constant.macro` | Macro-defined constants |
| `@module` | Module/namespace names |
| `@module.builtin` | Built-in modules |
| `@label` | Labels, goto targets |

### Literals

| Capture | Use For |
| --- | --- |
| `@string` | String literals |
| `@string.documentation` | Doc strings |
| `@string.regexp` | Regex literals |
| `@string.escape` | Escape sequences (`\n`, `\t`) |
| `@string.special` | Other special strings |
| `@string.special.symbol` | Symbols/atoms (`:foo` in Ruby/Elixir) |
| `@string.special.url` | URLs |
| `@string.special.path` | File paths |
| `@character` | Character literals |
| `@character.special` | Special characters |
| `@boolean` | `true` / `false` |
| `@number` | Integer literals |
| `@number.float` | Float literals |

### Types

| Capture | Use For |
| --- | --- |
| `@type` | Type names |
| `@type.builtin` | Built-in types (`int`, `String`) |
| `@type.definition` | Type being defined (in `type Foo = ...`) |
| `@attribute` | Attributes/annotations (`#[...]`, `@decorator`) |
| `@attribute.builtin` | Built-in attributes |
| `@property` | Object/struct property names |

### Functions

| Capture | Use For |
| --- | --- |
| `@function` | Function names (definition) |
| `@function.builtin` | Built-in functions (`print`, `len`) |
| `@function.call` | Function names (call site) |
| `@function.macro` | Macro names |
| `@function.method` | Method names (definition) |
| `@function.method.call` | Method names (call site) |
| `@constructor` | Constructor names |
| `@operator` | Operator symbols |

### Keywords

| Capture | Use For |
| --- | --- |
| `@keyword` | General keywords |
| `@keyword.coroutine` | `async`, `await`, `yield` |
| `@keyword.function` | `fn`, `func`, `def`, `lambda` |
| `@keyword.operator` | `and`, `or`, `not`, `in` (word operators) |
| `@keyword.import` | `use`, `import`, `from`, `include` |
| `@keyword.type` | `type`, `struct`, `enum`, `data` |
| `@keyword.modifier` | `pub`, `mut`, `static`, `const`, `abstract` |
| `@keyword.repeat` | `for`, `while`, `loop` |
| `@keyword.return` | `return`, `yield` |
| `@keyword.debug` | `assert`, `debug`, `unreachable` |
| `@keyword.exception` | `try`, `catch`, `throw`, `raise` |
| `@keyword.conditional` | `if`, `else`, `match`, `switch`, `case` |
| `@keyword.conditional.ternary` | `?` `:` in ternary |
| `@keyword.directive` | Preprocessor directives |
| `@keyword.directive.define` | `#define` |

### Punctuation

| Capture | Use For |
| --- | --- |
| `@punctuation.delimiter` | `,` `;` `:` `::` `.` |
| `@punctuation.bracket` | `(` `)` `[` `]` `{` `}` `<` `>` |
| `@punctuation.special` | String interpolation delimiters, template syntax |

### Comments

| Capture | Use For |
| --- | --- |
| `@comment` | General comments |
| `@comment.documentation` | Doc comments (`///`, `/** */`) |
| `@comment.error` | ERROR/FIXME in comments |
| `@comment.warning` | WARNING/HACK in comments |
| `@comment.todo` | TODO/XXX in comments |
| `@comment.note` | NOTE/INFO in comments |

### Markup (for embedded documentation)

`@markup.strong`, `@markup.italic`, `@markup.strikethrough`, `@markup.underline`, `@markup.heading`,
`@markup.heading.1`–`.6`, `@markup.quote`, `@markup.math`, `@markup.link`, `@markup.link.label`, `@markup.link.url`,
`@markup.raw`, `@markup.raw.block`, `@markup.list`, `@markup.list.checked`, `@markup.list.unchecked`

### Special

| Capture | Use For |
| --- | --- |
| `@diff.plus` / `@diff.minus` / `@diff.delta` | Diff highlights |
| `@tag` / `@tag.builtin` / `@tag.attribute` / `@tag.delimiter` | HTML/XML tags |
| `@none` | Explicitly no highlighting |
| `@conceal` | Concealable text |
| `@spell` / `@nospell` | Spellcheck control |

______________________________________________________________________

## locals.scm — Scope and Variable Resolution

Fixed capture names with special semantics:

```scheme
; Define scopes
[
  (block) (function_definition) (module_definition)
  (if_expression) (match_expression) (for_expression)
] @local.scope

; Definition sites
(function_definition name: (identifier) @local.definition.function)
(parameter name: (identifier) @local.definition.var)
(let_statement name: (identifier) @local.definition.var)

; Reference sites
(identifier) @local.reference
((type_identifier) @local.reference (#set! reference.kind "type"))
```

### Definition Subcaptures

`@local.definition.var`, `@local.definition.function`, `@local.definition.method`, `@local.definition.type`,
`@local.definition.field`, `@local.definition.import`, `@local.definition.namespace`, `@local.definition.macro`,
`@local.definition.constant`

### Integration with Highlights

Use `(#is-not? local)` in highlights.scm to avoid re-coloring locally scoped names:

```scheme
((identifier) @function.call (#is-not? local))
```

______________________________________________________________________

## injections.scm — Language Injection

Embed sub-languages within nodes:

```scheme
; Hard-coded language injection
((comment) @injection.content
  (#set! injection.language "markdown")
  (#set! injection.include-children))

; Dynamic language from node text
(fenced_code_block
  (info_string) @injection.language
  (code_fence_content) @injection.content)

; Self-injection (re-parse with same language)
((macro_invocation (token_tree) @injection.content)
  (#set! injection.self))

; Combined injection (all matches parsed as single document)
((comment) @injection.content
  (#set! injection.combined)
  (#set! injection.language "comment"))
```

### Injection Properties

| Property | Meaning |
| --- | --- |
| `injection.language` | Hard-code the injection language |
| `injection.combined` | All matching nodes parsed as one document |
| `injection.include-children` | Include child node text in injected content |
| `injection.self` | Re-parse with same language |
| `injection.parent` | Re-parse with parent document's language |

______________________________________________________________________

## tags.scm — Code Navigation

Used by editors for "Go to Definition", outline view, symbol search.

```scheme
; Definitions
(function_definition name: (identifier) @name) @definition.function
(module_definition name: (identifier) @name) @definition.module
(data_definition name: (type_identifier) @name) @definition.class
(typeclass_definition name: (type_identifier) @name) @definition.interface
(constant_definition name: (identifier) @name) @definition.constant

; References
(call_expression function: (identifier) @name) @reference.call
(type_application type: (type_identifier) @name) @reference.type

; Docstring extraction
(
  (comment)* @doc
  .
  (function_definition name: (identifier) @name) @definition.function
  (#strip! @doc "^//\\s*")
  (#select-adjacent! @doc @definition.function)
)
```

### Standard Tag Captures

`@definition.class`, `@definition.function`, `@definition.interface`, `@definition.method`, `@definition.module`,
`@definition.type`, `@reference.call`, `@reference.class`, `@reference.implementation`, `@reference.type`

______________________________________________________________________

## indents.scm — Auto-Indentation (nvim-treesitter)

```scheme
; Increase indent
[
  (function_definition) (module_definition) (block)
  (if_expression) (match_expression) (for_expression)
  (data_definition) (typeclass_definition)
] @indent.begin

; End of indented block
(block "}" @indent.end)

; Dedent the node itself (branch keywords/closers)
["}" ")" "]" "else" "elif"] @indent.branch

; Ignore indentation inside these
[(line_comment) (string_literal)] @indent.ignore
```

### Indent Captures

| Capture | Meaning |
| --- | --- |
| `@indent.begin` | Children should be indented |
| `@indent.end` | End of indented block |
| `@indent.align` | Hanging/aligned indent (Python-style) |
| `@indent.dedent` | Dedent children |
| `@indent.branch` | Dedent the node itself |
| `@indent.ignore` | No indentation changes inside this node |
| `@indent.auto` | Use `autoindent` behavior |
| `@indent.zero` | Set to column 0 |

______________________________________________________________________

## folds.scm — Code Folding

Mark foldable regions:

```scheme
[
  (function_definition) (module_definition) (block)
  (if_expression) (match_expression)
  (data_definition) (typeclass_definition) (impl_block)
  (block_comment)
] @fold

; Fold consecutive imports
(use_declaration)+ @fold
```

Falls back to `@local.scope` captures if no folds.scm exists.

______________________________________________________________________

## brackets.scm — Bracket Matching

Simple bracket pair definitions:

```scheme
"(" @punctuation.bracket
")" @punctuation.bracket
"[" @punctuation.bracket
"]" @punctuation.bracket
"{" @punctuation.bracket
"}" @punctuation.bracket
```

______________________________________________________________________

## Test File Format

### Corpus Tests (`test/corpus/*.txt`)

```
==================
Test name
:attribute   (optional — :skip, :error, :fail-fast, :language(LANG), :platform(PLATFORM))
==================
source code here
---
(source_file
  (function_definition
    name: (identifier)
    body: (block)))
```

### Highlight Tests (`test/highlight/*`)

```javascript
var abc = function (d) {
  // <- keyword
  //          ^ keyword
  //               ^ variable.parameter
  // <- !variable
};
```

- `^` tests the column directly above
- `<-` tests the column of the comment character
- `!` prefix negates the assertion

### Tags Tests (`test/tags/*`)

Same annotation format as highlight tests, checking tag captures.
