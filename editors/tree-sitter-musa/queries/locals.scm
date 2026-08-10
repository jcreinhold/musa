; Locals (prompt 80): the scopes an editor's rename-what-you-see and
; occurrence-highlighting read. The language server (prompt 78) is the
; semantic rename; this is the lexical one.
(piece_declaration) @local.scope
(template_declaration) @local.scope
(library_declaration) @local.scope
(motif_declaration) @local.scope
(block) @local.scope
(function_declaration) @local.scope
(music_expression) @local.scope
(match_expression) @local.scope

(motif_declaration name: (identifier) @local.definition)
(fragment_declaration name: (identifier) @local.definition)
(motif_parameter name: (identifier) @local.definition)
(part_declaration name: (identifier) @local.definition)
(voice_declaration name: (identifier) @local.definition)
(piece_declaration template_name: (identifier) @local.definition)
(make_statement name: (identifier) @local.definition)
(make_statement template: (identifier) @local.reference)
(bar_statement name: (identifier) @local.definition)
(patch_declaration name: (identifier) @local.definition)
(bus_declaration name: (identifier) @local.definition)
(signal_binding name: (identifier) @local.definition)
(let_declaration name: (identifier) @local.definition)
(function_declaration name: (identifier) @local.definition)
(parameter name: (identifier) @local.definition)

(name_expression (identifier) @local.reference)
(mobile_statement fragment: (identifier) @local.reference)
(assign_statement source: (identifier) @local.reference)
(assign_statement destination: (identifier) @local.reference)
(route_statement source: (identifier) @local.reference)
(send_statement source: (identifier) @local.reference)
(send_statement bus: (identifier) @local.reference)
(modulate_statement signal: (identifier) @local.reference)
(parameter_path patch: (identifier) @local.reference)
