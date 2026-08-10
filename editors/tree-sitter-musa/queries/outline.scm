; Outline (prompt 80): the shape of the piece, in the editor's symbol pane.

(piece_declaration
  name: (string) @name) @item

; A template is named where it is declared, and again where it is made.
(piece_declaration
  template_name: (identifier) @name) @item

(make_statement
  name: (identifier) @name) @item

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
