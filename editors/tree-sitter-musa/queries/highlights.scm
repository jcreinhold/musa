; Highlighting, in the language's own vocabulary (prompt 80).
;
; Captures mirror `TokenClass` in crates/musa-language/src/highlight.rs:
; Comment, Keyword, Use, Pitch, Duration, Number, Text, Unit, Name,
; punctuation. Specific name captures come first; the generic identifier
; rule is last, so anything more specific already won.

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
  "ending"
  "mobile"
  "improvise"
  "over"
  "senza"
  "let"
  "fn"
  "music"
  "import"
  "match"
  "some"
  "none"
  "true"
  "false"
] @keyword

; `use` alone, as TokenClass::Use says: it is where material comes from.
; `import` is a structural keyword and belongs with the list above: it brings
; in names, not material.
"use" @keyword.import

; --- Keywords whose class depends on where they stand -----------------------
;
; `harmony`, `option`, `list`, `pitch`, `scale` are also module names —
; `parser.rs`'s MODULE_NAME lets a module be named after a type or a domain
; (`import std::harmony;`, `mod list;`). They are captured by parent here,
; never by bare text: a flat list cannot tell `harmony { ... }` from
; `std::harmony`, and guessing from text is exactly the bug this section
; exists to prevent.

(harmony_declaration "harmony" @keyword)
(option_type "option" @keyword)
(list_type "list" @keyword)
(type_name "pitch" @keyword)
(type_name "scale" @keyword)
(scale_expression "scale" @keyword)
(in_scale_statement "scale" @keyword)

; --- Modules: the name positions --------------------------------------------
;
; Every segment of an import path, a `mod` child's name, and an import's
; alias name a module. `_module_name` is hidden, so the borrowed keyword
; tokens are direct children of the statement — these patterns are exact.

(import_statement (identifier) @module)
(import_statement "harmony" @module)
(import_statement "option" @module)
(import_statement "list" @module)
(import_statement "pitch" @module)
(import_statement "scale" @module)

(mod_declaration (identifier) @module)
(mod_declaration "harmony" @module)
(mod_declaration "option" @module)
(mod_declaration "list" @module)
(mod_declaration "pitch" @module)
(mod_declaration "scale" @module)

; --- The music itself ------------------------------------------------------

(pitch_literal) @constant
(interval_literal) @constant

; Durations, meters, ratios: exact numbers.
(rational) @number
(integer) @number
(float) @number

(string) @string
(unit) @string.special

; --- Names the composer gave things ----------------------------------------

(motif_declaration name: (identifier) @function)
(fragment_declaration name: (identifier) @function)
(function_declaration name: (identifier) @function)
(let_declaration name: (identifier) @variable)
(application_expression (name_expression (identifier) @function.call))
(mobile_statement fragment: (identifier) @function.call)

(part_declaration name: (identifier) @type)
(patch_declaration name: (identifier) @type)
(bus_declaration name: (identifier) @type)

(voice_declaration name: (identifier) @variable)
; A template's own name, and the name a `make` gives what it makes.
(piece_declaration template_name: (identifier) @function)
(make_statement template: (identifier) @function.call)
(make_statement name: (identifier) @variable)
; A signature and a structure are types in the editor's vocabulary: they name
; structure, never a value.
(signature_declaration name: (identifier) @type)
(structure_declaration name: (identifier) @type)
(structure_declaration signature: (identifier) @type)
(signature_member name: (identifier) @variable)
(name_expression member: (identifier) @variable)
(profile_declaration name: (identifier) @variable)
(bar_statement name: (identifier) @label)

(setting_statement name: (identifier) @variable.parameter)
(motif_parameter name: (identifier) @variable.parameter)
(parameter name: (identifier) @variable.parameter)
(expression_argument name: (identifier) @variable.parameter)

; --- Vocabulary names -----------------------------------------------------
; Not the composer's names but the language's: marks, dynamics, and clefs are
; chosen from a fixed vocabulary the compiler checks, so they highlight like
; constants, not like names.

(mark_statement name: (identifier) @constant)
(mark_rule name: (identifier) @constant)
(dynamic_statement mark: (identifier) @constant)
(clef_statement name: (identifier) @constant)

(assign_statement source: (identifier) @type)
(assign_statement destination: (identifier) @type)
(route_statement source: (identifier) @type)
(send_statement source: (identifier) @type)
(send_statement bus: (identifier) @type)
(modulate_statement signal: (identifier) @variable)
(parameter_path patch: (identifier) @type)

(call_expression name: (identifier) @function.builtin)
(name_reference (identifier) @variable)

; Everything else a name can be: articulations, references, clefs, dynamics.
(identifier) @variable

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
  ">"
  "^"
] @punctuation.delimiter
