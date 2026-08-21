; The shape of the piece, in the editor's symbol pane.

(piece_declaration
  name: (string) @name) @item

; A template is named where it is declared, and again where it is made.
(piece_declaration
  template_name: (identifier) @name) @item

(make_statement
  name: (identifier) @name) @item

(signature_declaration
  name: (identifier) @name) @item

(structure_declaration
  name: (identifier) @name) @item

(data_declaration
  name: (identifier) @name) @item

(record_declaration
  name: (identifier) @name) @item

(enum_declaration
  name: (identifier) @name) @item

; An `impl` block has no name to outline. It is listed by the type whose
; namespace it opens, which is the only thing that distinguishes one `impl` in
; a file from the next.
(impl_declaration
  head: (type_expression) @name) @item

; A module file has no piece to outline, so its children are the outline. The
; name is matched by position, since `mod list;` names a module with a word
; the lexer writes as a type keyword.
(mod_declaration
  name: _ @name) @item

(motif_declaration
  name: (identifier) @name) @item

(fragment_declaration
  name: (identifier) @name) @item

(function_declaration
  name: (identifier) @name) @item

(let_declaration
  name: (identifier) @name) @item

(part_declaration
  name: (identifier) @name) @item

(voice_declaration
  name: (identifier) @name) @item

(section_statement
  name: (string) @name) @item

(phrase_statement
  name: (string) @name) @item

(profile_declaration
  name: (identifier) @name) @item

(patch_declaration
  name: (identifier) @name) @item

(bus_declaration
  name: (identifier) @name) @item
