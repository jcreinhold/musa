; Highlighting, in the language's own vocabulary.
;
; The colors are the theme's business; the captures are ours. Zed's default
; themes paint the standard captures the way Rust readers expect: @function
; blue, @property red, @type cyan, @keyword purple, @string green,
; @constant yellow, @variable plain. So this file maps musa's vocabulary to
; those captures — a motif is a function, a named argument is a field, a
; dynamic is a variant — and the theme does the rest.
;
; Two Zed rules the file obeys:
;
;   1. Only captures in Zed's taxonomy highlight at all (see the
;      zed-extension skill's query-files reference). `@module` and
;      `@function.call` are not in it; using them renders text UNCOLORED.
;   2. The LAST matching pattern wins. General fallbacks come first, the
;      specific name captures last.

; --- Comment ------------------------------------------------------------

(comment) @comment

; --- Keywords -------------------------------------------------------------

[
  "piece"
  "library"
  "template"
  "signature"
  "structure"
  "mod"
  "make"
  "as"
  "tempo"
  "meter"
  "key"
  "subtitle"
  "composer"
  "arranger"
  "copyright"
  "motif"
  "fragment"
  "score"
  "part"
  "voice"
  "clef"
  "transpose"
  "down"
  "up"
  "rest"
  "repeat"
  "slur"
  "dynamic"
  "groove"
  "grace"
  "tuplet"
  "performance"
  "profile"
  "mark"
  "studio"
  "patch"
  "modulate"
  "bus"
  "assign"
  "route"
  "send"
  "master"
  "at"
  "output"
  "stretch"
  "retrograde"
  "invert"
  "around"
  "with"
  "note"
  "phrase"
  "section"
  "crescendo"
  "diminuendo"
  "to"
  "bar"
  "assert"
  "ending"
  "mobile"
  "improvise"
  "over"
  "senza"
  "let"
  "fn"
  "music"
  "kernel"
  "import"
  "match"
  ; `use` alone, as TokenClass::Use says: it is where material comes from.
  ; Zed has no keyword subcategories, so it takes the keyword color.
  "use"
] @keyword

; --- Keywords whose class depends on where they stand -----------------------
;
; `harmony`, `pitch`, `scale` are also module names — `parser.rs`'s
; MODULE_NAME lets a module be named after a domain (`import std::harmony;`,
; `mod pitch;`). They are captured by parent here, never by bare text: a flat
; list cannot tell `harmony { ... }` from `std::harmony`, and guessing from
; text is exactly the bug this section exists to prevent.

(harmony_declaration "harmony" @keyword)
(option_type "Option" @keyword)
(list_type "List" @keyword)
(result_type "Result" @keyword)
(scale_expression "scale" @keyword)
(in_scale_statement "scale" @keyword)

; Literals that behave like keywords: booleans and the option constructors.
; Rust readers know these as the yellow-ish built-ins, not as control words.
[
  "true"
  "false"
] @boolean

[
  "Some"
  "None"
  "Ok"
  "Err"
] @constant.builtin

; --- The music itself ------------------------------------------------------
;
; Written pitches and intervals are the piece's constants: yellow, like the
; variants in `ExitCode::SUCCESS`.

(pitch_literal) @constant
(interval_literal) @constant

; Durations, meters, ratios: exact numbers.
(rational) @number
(integer) @number
(float) @number

(string) @string

; Units (`Hz`, `dB`, `bpm`) are a fixed vocabulary the compiler checks, so
; they paint like constants, not like text the composer wrote.
(unit) @constant

; --- Operators and punctuation ----------------------------------------------

[
  "->"
  "|>"
  "="
  "-"
  "~"
] @operator

[
  "{"
  "}"
  "["
  "]"
  "("
  ")"
] @punctuation.bracket

; `|` is a barline and `>` and `^` are the accent and the marcato. They are
; punctuation for the same reason `/` is: the real highlighter files all three
; under `TokenClass::Punctuation`, and a second reader owns no vocabulary.
;
; `<` and `>` are also the two halves of a type parameter, and they take no
; second entry for it. `>` reads one way — accent in a bar, closing half in a
; type — because the real lexer has one token for it, and `<` joins the same
; list rather than the bracket list above: brackets are the list literal, and
; that is the whole point of the character having moved.
;
; `#` is here rather than with the pitches because that is where the real
; highlighter puts it: a sharp inside `g#4` never reaches this list — the
; whole literal is one token — and the one that stands alone, in `key g#
; minor` and `f#m7`, is `TokenClass::Punctuation` there.
[
  ";"
  ","
  ":"
  "."
  "/"
  "#"
  "|"
  "<"
  ">"
  "^"
] @punctuation.delimiter

; --- The fallback: every name is plain until a rule below says otherwise ----
;
; Rust leaves `let x = ...` uncolored, and so does musa: a name the composer
; gave a value takes no ink. Everything more specific comes after this line
; and wins over it.

(identifier) @variable

; --- Modules -----------------------------------------------------------------
;
; Every segment of an import path, a `mod` child's name, and an import's
; alias names a module. `_module_name` is hidden, so the borrowed keyword
; tokens are direct children of the statement — these patterns are exact.
; Modules are names, and Rust leaves module paths uncolored too, so they
; take the variable capture on purpose.

(import_statement (identifier) @variable)
(import_statement "harmony" @variable)
(import_statement "pitch" @variable)
(import_statement "scale" @variable)

(mod_declaration (identifier) @variable)
(mod_declaration "harmony" @variable)
(mod_declaration "pitch" @variable)
(mod_declaration "scale" @variable)

; --- Types: cyan --------------------------------------------------------------
;
; The language's own type words are builtins; what the composer declares as
; structure — parts, patches, buses, signatures, structures — is a type in
; the editor's vocabulary.

; A type is an identifier — a capital is what makes it one — so there is no
; keyword to capture here and the position does all the work.
(type_name (identifier) @type)

(part_declaration name: (identifier) @type)
(patch_declaration name: (identifier) @type)
(bus_declaration name: (identifier) @type)
(signature_declaration name: (identifier) @type)
(structure_declaration name: (identifier) @type)
(structure_declaration signature: (identifier) @type)

(assign_statement source: (identifier) @type)
(assign_statement destination: (identifier) @type)
(route_statement source: (identifier) @type)
(send_statement source: (identifier) @type)
(send_statement bus: (identifier) @type)
(parameter_path patch: (identifier) @type)

; --- Functions: blue ------------------------------------------------------------
;
; Definitions and calls alike: Zed does not distinguish @function.call, so
; both take @function.

(motif_declaration name: (identifier) @function)
(fragment_declaration name: (identifier) @function)
(function_declaration name: (identifier) @function)
; A template's own name, and the template a `make` is made from.
(piece_declaration template_name: (identifier) @function)
(make_statement template: (identifier) @function)

(application_expression (name_expression (identifier) @function))
(mobile_statement fragment: (identifier) @function)

; The studio's processors are the language's builtins.
(call_expression name: (identifier) @function.builtin)

; A claim is one of a fixed few the compiler knows, so it paints as a builtin
; rather than as something the piece defined — nothing in a `.musa` file
; declares `pitches_in`.
(assert_statement claim: (identifier) @function.builtin)

; --- Fields and arguments: red ---------------------------------------------------
;
; Rust paints struct fields red; musa's fields are the named parts of a
; statement or expression: settings keys, signature members, and named
; arguments.

(setting_statement name: (identifier) @property)
(signature_member name: (identifier) @property)
(expression_argument name: (identifier) @property)

; Parameters are the declaration side of an argument.
(motif_parameter name: (identifier) @variable.parameter)
(parameter name: (identifier) @variable.parameter)

; --- Vocabulary names: yellow -----------------------------------------------------
; Not the composer's names but the language's: marks, dynamics, and clefs are
; chosen from a fixed vocabulary the compiler checks, so they highlight like
; the variants of an enum, not like names.

(mark_statement name: (identifier) @constant)
(mark_rule name: (identifier) @constant)
(dynamic_statement mark: (identifier) @constant)
(clef_statement name: (identifier) @constant)

; A bar's name is a target, not a value.
(bar_statement name: (identifier) @label)

; --- A kernel quote ---------------------------------------------------------
;
; `Timeline` and its payload are types, the same two words `musa-kernel`'s own
; classifier calls types (`crates/musa-kernel/src/editor.rs`). Inside the body
; the words belong to the kernel's grammar, and an editor colouring them from
; here would be a second copy of that lexis; what is marked instead is the
; seam — the quote's head, and the `${` that lets the host back in.
(kernel_quote constructor: (identifier) @type)
(kernel_quote payload: (identifier) @type)
(kernel_hole "$" @punctuation.special)
