# Zed Query Files Reference

Complete reference for all tree-sitter query files that Zed loads from `languages/<lang>/`. These files tell Zed how to
use the parse tree for highlighting, navigation, indentation, and other editor features.

Zed loads query files from the extension's `languages/<lang>/` directory. If a file is missing, that feature is simply
disabled — no error.

## Table of Contents

1. [highlights.scm](#highlightsscm)
1. [outline.scm](#outlinescm)
1. [brackets.scm](#bracketsscm)
1. [indents.scm](#indentsscm)
1. [injections.scm](#injectionsscm)
1. [overrides.scm](#overridesscm)
1. [textobjects.scm](#textobjectsscm)
1. [runnables.scm](#runnablesscm)
1. [redactions.scm](#redactionsscm)
1. [folds.scm](#foldsscm)
1. [locals.scm](#localsscm)
1. [tags.scm](#tagsscm)
1. [semantic_token_rules.json](#semantic_token_rulesjson)

______________________________________________________________________

## highlights.scm

Maps syntax tree nodes to highlight capture names. Zed themes assign colors to each capture.

### Zed Highlight Captures

These are the captures Zed themes recognize. Using captures outside this set will compile but produce no highlighting.

| Capture | Use for |
| --- | --- |
| `@attribute` | Attributes, decorators, annotations |
| `@boolean` | Boolean literals (`true`, `false`) |
| `@comment` | Line and block comments |
| `@comment.doc` | Documentation comments |
| `@constant` | Named constants |
| `@constant.builtin` | Built-in constants (`true`, `false`, `nil`, `self`) |
| `@constructor` | Constructors, variant names |
| `@embedded` | Embedded content (e.g., code in templates) |
| `@emphasis` | Emphasized text (italic) |
| `@emphasis.strong` | Strongly emphasized text (bold) |
| `@enum` | Enum types |
| `@function` | Function definitions and references |
| `@function.builtin` | Built-in functions |
| `@function.method` | Method definitions |
| `@function.special` | Special functions (macros, decorators) |
| `@hint` | Hint annotations |
| `@keyword` | General keywords |
| `@label` | Labels, targets |
| `@link_text` | Link text in markup |
| `@link_uri` | Link URLs in markup |
| `@number` | Numeric literals |
| `@operator` | Operators (`+`, `-`, `->`, etc.) |
| `@predictive` | Predictive/ghost text |
| `@preproc` | Preprocessor directives |
| `@primary` | Primary/prominent elements |
| `@property` | Object properties, struct fields |
| `@punctuation` | General punctuation |
| `@punctuation.bracket` | Brackets `()`, `[]`, `{}` |
| `@punctuation.delimiter` | Delimiters: `,`, `;`, `:` |
| `@punctuation.list_marker` | List markers in markup |
| `@punctuation.special` | Special punctuation |
| `@string` | String literals |
| `@string.escape` | Escape sequences in strings |
| `@string.regex` | Regular expressions |
| `@string.special` | Special strings |
| `@string.special.symbol` | Symbols (e.g., Ruby `:symbol`) |
| `@tag` | Tags (HTML, XML) |
| `@tag.doctype` | DOCTYPE declarations |
| `@text.literal` | Literal/verbatim text |
| `@title` | Titles, headings |
| `@type` | Type names |
| `@type.builtin` | Built-in types (`i32`, `String`, etc.) |
| `@variable` | Variables |
| `@variable.parameter` | Function/method parameters |
| `@variable.special` | Special variables (`self`, `this`, `super`) |
| `@variant` | Enum variants, union members |

### Highlight Priority

When multiple patterns match the same node, the **last matching pattern** in the file wins. Place general patterns at
the top and specific overrides at the bottom.

### Pattern Examples

```scheme
; Keyword groups (general — place early)
["fn" "let" "if" "else" "match" "return"] @keyword

; Specific keyword subcategories (place after general)
["part" "voice" "score"] @keyword      ; Zed themes may not distinguish subcategories
["data" "codata" "type"] @keyword

; Definitions with field access
(function_definition name: (identifier) @function)
(data_definition name: (type_identifier) @type)
(constructor_definition name: (type_identifier) @constructor)

; Calls
(call_expression function: (identifier) @function)
(method_call method: (field_identifier) @function.method)

; Regex-based matching for constructors (capitalized identifiers)
((identifier) @constructor (#match? @constructor "^[A-Z]"))

; Built-in type matching
((named_type) @type.builtin
  (#match? @type.builtin "^(u8|u16|u32|u64|i8|i16|i32|i64|f32|f64|bool|String)$"))
```

### Differences from nvim-treesitter

| nvim-treesitter | Zed | Notes |
| --- | --- | --- |
| `@keyword.function` | `@keyword` | Zed themes typically don't distinguish keyword subcategories |
| `@keyword.return` | `@keyword` | Same — use `@keyword` for all keywords |
| `@keyword.conditional` | `@keyword` | Same |
| `@function.call` | `@function` | Zed may not distinguish definition vs. call |
| `@type.definition` | `@type` | Use `@type` for both definitions and references |
| `@number.float` | `@number` | No float subcategory in Zed |
| `@variable.builtin` | `@variable.special` | Different name for `self`/`this` |
| `@punctuation.bracket` in highlights | `@punctuation.bracket` | Same, but brackets.scm uses `@open`/`@close` |

**Practical advice:** Start with the standard captures from the table above. If a subcategory capture
(`@keyword.function`) doesn't produce different highlighting from the parent (`@keyword`), it's because the active Zed
theme doesn't define a separate color for it. This is fine — the capture still works and may gain distinct colors in
future themes.

______________________________________________________________________

## outline.scm

Defines the code structure shown in Zed's outline panel (Cmd+Shift+O) and breadcrumbs.

### Captures

| Capture | Purpose |
| --- | --- |
| `@item` | Marks a node as a navigable outline entry |
| `@name` | The label shown for the entry (must be inside `@item`) |
| `@context` | Parent context shown above the entry (e.g., `impl Foo` above a method) |
| `@context.extra` | Additional context (e.g., type parameters) |
| `@annotation` | Annotations shown alongside the entry |

### Pattern Examples

```scheme
; Functions appear in the outline
(function_definition
  name: (identifier) @name) @item

; Data types with their name
(data_definition
  name: (type_identifier) @name) @item

; Methods with their impl context
(impl_definition
  head: (_) @context
  method: (impl_method
    name: (identifier) @name) @item)

; Modules
(module_definition
  name: (module_identifier) @name) @item

; Effect definitions
(effect_definition
  name: (type_identifier) @name) @item

; Constants
(const_definition
  name: (identifier) @name) @item
```

### Key Rules

- Every `@item` must contain a `@name` — entries without names are ignored.
- `@context` creates a visual grouping (shown as a breadcrumb parent). The `@context` node must be an ancestor of
  `@item`.
- Outline entries appear in document order. Nesting follows the parse tree structure.
- This file is Zed-specific — nvim-treesitter does not use `outline.scm`.

______________________________________________________________________

## brackets.scm

Defines bracket pairs for matching and rainbow highlighting.

### Captures

| Capture | Purpose |
| --- | --- |
| `@open` | Opening bracket |
| `@close` | Closing bracket |

### Pattern Examples

```scheme
["(" "[" "{"] @open
[")" "]" "}"] @close
```

### Disabling Rainbow Colors

```scheme
; Disable rainbow colors for specific brackets
("{" @open "}" @close
  (#set! rainbow.exclude))
```

### Key Rules

- These captures are only for bracket highlighting/matching — not for syntax highlighting (that's `@punctuation.bracket`
  in `highlights.scm`).
- Each `@open`/`@close` should appear as a standalone pattern or a paired pattern.

______________________________________________________________________

## indents.scm

Controls automatic indentation when pressing Enter or reformatting.

### Captures

| Capture | Purpose |
| --- | --- |
| `@indent` | Increase indent after this node |
| `@end` | Decrease indent at this node |
| `@indent.always` | Always indent (even if on same line as parent) |
| `@outdent` | Outdent this line one level |

### Pattern Examples

```scheme
; Block constructs increase indent
[
  (block)
  (argument_list)
  (parameter_list)
  (data_definition)
  (match_expression)
  (handle_expression)
] @indent

; Closing tokens decrease indent
["}" "]" ")"] @end

; Match arms get their own indent level
(match_arm) @indent
```

### Key Rules

- `@indent` says "children of this node are indented one level."
- `@end` says "this token should be dedented to match its opening."
- These interact with `increase_indent_pattern` / `decrease_indent_pattern` from `config.toml`. The query-based approach
  takes precedence when both are present.

______________________________________________________________________

## injections.scm

Tells Zed to parse regions of the file with a different language's grammar.

### Captures

| Capture | Purpose |
| --- | --- |
| `@injection.content` | The text region to parse with the injected language |
| `@injection.language` | A node whose text names the language |

### Properties

```scheme
; Inject a fixed language
(doc_comment) @injection.content
(#set! injection.language "markdown")

; Inject based on node text (e.g., fenced code blocks)
(fenced_code_block
  (info_string (language) @injection.language)
  (code_fence_content) @injection.content)

; Combined injection (merge all matching nodes into one parse)
(#set! injection.combined)
```

### Key Rules

- Set the language either via `@injection.language` capture or `(#set! injection.language "name")`.
- `injection.combined` merges all matches into a single parse tree (useful for scattered template expressions).

______________________________________________________________________

## overrides.scm

Overrides Zed settings inside specific syntax scopes.

### Pattern Examples

```scheme
; Inside strings, disable auto-close and change word characters
(string_literal) @string
(#set! override.disable_autoclose true)
(#set! override.word_characters "-")

; Inside comments, treat hyphens as word characters
[(line_comment) (block_comment)] @comment
(#set! override.word_characters "-")
```

The scope name (e.g., `@string`) must match a scope referenced in `config.toml`'s `[overrides.<scope>]` section.

______________________________________________________________________

## textobjects.scm

Defines Vim-style text objects for `i`/`a` motions.

### Captures

| Capture | Purpose |
| --- | --- |
| `@function.around` | Whole function (for `af`) |
| `@function.inside` | Function body only (for `if`) |
| `@class.around` | Whole class/type (for `ac`) |
| `@class.inside` | Class/type body only (for `ic`) |
| `@comment.around` | Whole comment (for `agc`) |
| `@comment.inside` | Comment content only |

### Pattern Examples

```scheme
(function_definition) @function.around
(function_definition body: (_) @function.inside)

(data_definition) @class.around
(data_definition body: (_) @class.inside)

[(line_comment) (block_comment)] @comment.around
```

______________________________________________________________________

## runnables.scm

Detects runnable code (test functions, main functions) and shows a run button in the gutter.

### Captures

| Capture | Purpose |
| --- | --- |
| `@run` | Marks a node as runnable |

### Properties

Use `#set!` to configure how the code is run:

```scheme
; Detect test functions
(function_definition
  (attribute_body name: (identifier) @_attr)
  name: (identifier) @run
  (#eq? @_attr "test")
  (#set! tag "test")
)
```

### Custom Environment Variables

Runnables can expose captured values as environment variables:

```scheme
(function_definition
  name: (identifier) @run @_name
  (#set! tag "test")
  (#set! env.TEST_NAME @_name)
)
```

______________________________________________________________________

## redactions.scm

Marks sensitive content that should be hidden during screen sharing (Zed's collaboration feature).

### Captures

| Capture | Purpose |
| --- | --- |
| `@redact` | Content to hide during screen sharing |

### Pattern Examples

```scheme
(string_literal) @redact
(integer_literal) @redact
```

______________________________________________________________________

## folds.scm

Defines which constructs can be collapsed with code folding.

### Captures

| Capture | Purpose |
| --- | --- |
| `@fold` | Node that can be folded |

### Pattern Examples

```scheme
[
  (block)
  (module_definition)
  (data_definition)
  (codata_definition)
  (effect_definition)
  (typeclass_definition)
  (impl_definition)
  (handle_expression)
  (match_expression)
  (import_group)
] @fold
```

______________________________________________________________________

## locals.scm

Defines variable scoping for rename refactoring and reference highlighting.

### Captures

| Capture | Purpose |
| --- | --- |
| `@local.scope` | Creates a new scope |
| `@local.definition` | Defines a variable in the current scope |
| `@local.reference` | References a variable |

### Pattern Examples

```scheme
; Scope-creating constructs
[
  (source_file)
  (block)
  (module_definition)
  (handle_expression)
] @local.scope

; Variable definitions
(parameter name: (identifier) @local.definition)
(lambda_parameter name: (identifier) @local.definition)

; References
(identifier) @local.reference
```

______________________________________________________________________

## tags.scm

Defines tags for code navigation (similar to ctags). Used for "go to definition" and workspace symbols.

### Captures

| Capture | Purpose |
| --- | --- |
| `@definition.function` | Function definition |
| `@definition.method` | Method definition |
| `@definition.type` | Type definition |
| `@definition.module` | Module definition |
| `@definition.constant` | Constant definition |
| `@reference.call` | Function/method call |

### Pattern Examples

```scheme
(function_definition name: (identifier) @definition.function)
(data_definition name: (type_identifier) @definition.type)
(call_expression function: (identifier) @reference.call)
```

______________________________________________________________________

## semantic_token_rules.json

Maps LSP semantic token types and modifiers to Zed highlight captures. This lets the LSP's type-aware analysis override
or supplement tree-sitter highlighting.

### Format

```json
{
  "rules": [
    {
      "token_type": "function",
      "highlight": "@function"
    },
    {
      "token_type": "type",
      "highlight": "@type"
    },
    {
      "token_type": "variable",
      "modifiers": ["readonly"],
      "highlight": "@constant"
    },
    {
      "token_type": "parameter",
      "highlight": "@variable.parameter"
    },
    {
      "token_type": "keyword",
      "highlight": "@keyword"
    }
  ]
}
```

### Key Rules

- Users must enable semantic tokens in Zed settings: `"semantic_tokens": "combined"` (or `"replace"` to fully replace
  tree-sitter highlighting).
- Rules with `modifiers` are more specific and take precedence over rules without.
- The `token_type` values come from the LSP's `semanticTokensProvider` capability — check what your language server
  reports.
- The `highlight` value must be a Zed capture name from the highlights.scm table above.
