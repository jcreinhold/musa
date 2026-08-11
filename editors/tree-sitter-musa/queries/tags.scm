; Definitions and the references that find them, for code navigation that
; works without the language server.

(motif_declaration
  name: (identifier) @name) @definition.function

(fragment_declaration
  name: (identifier) @name) @definition.function

(function_declaration
  name: (identifier) @name) @definition.function

(let_declaration
  name: (identifier) @name) @definition.variable

(part_declaration
  name: (identifier) @name) @definition.type

(voice_declaration
  name: (identifier) @name) @definition.variable

(piece_declaration
  template_name: (identifier) @name) @definition.type

(make_statement
  name: (identifier) @name) @definition.variable

(make_statement
  template: (identifier) @name) @reference.call

(signature_declaration
  name: (identifier) @name) @definition.type

(structure_declaration
  name: (identifier) @name) @definition.type

(structure_declaration
  signature: (identifier) @name) @reference.type

(patch_declaration
  name: (identifier) @name) @definition.type

(bus_declaration
  name: (identifier) @name) @definition.type

(profile_declaration
  name: (identifier) @name) @definition.variable

(bar_statement
  name: (identifier) @name) @definition.function

(application_expression
  (name_expression (identifier) @name)) @reference.call

(mobile_statement
  fragment: (identifier) @name) @reference.call

(assign_statement
  source: (identifier) @name) @reference.implementation

(assign_statement
  destination: (identifier) @name) @reference.implementation
