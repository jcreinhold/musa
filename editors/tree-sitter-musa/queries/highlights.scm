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
  "pitch"
  "stretch"
  "retrograde"
  "invert"
  "around"
  "with"
  "note"
  "phrase"
  "section"
  "harmony"
  "crescendo"
  "diminuendo"
  "to"
  "bar"
  "ending"
  "mobile"
  "improvise"
  "over"
  "senza"
] @keyword

; `use` alone, as TokenClass::Use says: it is where material comes from —
; an import of a library or a call of a motif.
"use" @keyword.import

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
(use_statement name: (identifier) @function.call)
(mobile_statement fragment: (identifier) @function.call)

(part_declaration name: (identifier) @type)
(patch_declaration name: (identifier) @type)
(bus_declaration name: (identifier) @type)

(voice_declaration name: (identifier) @variable)
(profile_declaration name: (identifier) @variable)
(bar_statement name: (identifier) @label)

(setting_statement name: (identifier) @variable.parameter)
(motif_parameter name: (identifier) @variable.parameter)

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
