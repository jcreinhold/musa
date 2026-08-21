; The scopes an editor's rename-what-you-see and occurrence-highlighting
; read. The language server is the semantic rename; this is the lexical one.
(piece_declaration) @local.scope
(template_declaration) @local.scope
(structure_declaration) @local.scope
(library_declaration) @local.scope
(motif_declaration) @local.scope
(block) @local.scope
(function_declaration) @local.scope
; An `impl` block is a scope: a function inside one is a scope of its own by
; the line above.
(impl_declaration) @local.scope
(music_expression) @local.scope
(events_quote) @local.scope
(match_expression) @local.scope
(lambda_expression) @local.scope

(motif_declaration name: (identifier) @local.definition)
(fragment_declaration name: (identifier) @local.definition)
(part_declaration name: (identifier) @local.definition)
(voice_declaration name: (identifier) @local.definition)
(piece_declaration template_name: (identifier) @local.definition)
(make_statement name: (identifier) @local.definition)
(make_statement template: (identifier) @local.reference)
(signature_declaration name: (identifier) @local.definition)
(signature_member name: (identifier) @local.definition)
(structure_declaration name: (identifier) @local.definition)
(structure_declaration signature: (identifier) @local.reference)
(data_declaration name: (identifier) @local.definition)
(data_member name: (identifier) @local.definition)
(data_variant name: (identifier) @local.definition)
(record_declaration name: (identifier) @local.definition)
(enum_declaration name: (identifier) @local.definition)
(type_parameter (identifier) @local.definition)
(enum_case name: (identifier) @local.definition)
(record_literal_expression type: (identifier) @local.reference)
(path_expression type: (identifier) @local.reference)
(bar_statement name: (identifier) @local.definition)
(patch_declaration name: (identifier) @local.definition)
(bus_declaration name: (identifier) @local.definition)
(signal_binding name: (identifier) @local.definition)
(let_declaration name: (identifier) @local.definition)
(function_declaration name: (identifier) @local.definition)
(parameter name: (identifier) @local.definition)

(name_expression (identifier) @local.reference)
(name_expression member: (identifier) @local.reference)
(mobile_statement fragment: (identifier) @local.reference)
(assign_statement source: (identifier) @local.reference)
(assign_statement destination: (identifier) @local.reference)
(route_statement source: (identifier) @local.reference)
(send_statement source: (identifier) @local.reference)
(send_statement bus: (identifier) @local.reference)
(modulate_statement signal: (identifier) @local.reference)
(parameter_path patch: (identifier) @local.reference)
