; Locals (prompt 80): the scopes an editor's rename-what-you-see and
; occurrence-highlighting read. The language server (prompt 78) is the
; semantic rename; this is the lexical one.
(piece_declaration) @local.scope
(library_declaration) @local.scope
(motif_declaration) @local.scope
(block) @local.scope

(motif_declaration name: (identifier) @local.definition)
(fragment_declaration name: (identifier) @local.definition)
(motif_parameter name: (identifier) @local.definition)
(part_declaration name: (identifier) @local.definition)
(voice_declaration name: (identifier) @local.definition)
(bar_statement name: (identifier) @local.definition)
(patch_declaration name: (identifier) @local.definition)
(bus_declaration name: (identifier) @local.definition)
(signal_binding name: (identifier) @local.definition)

(use_statement name: (identifier) @local.reference)
(mobile_statement fragment: (identifier) @local.reference)
(assign_statement source: (identifier) @local.reference)
(assign_statement destination: (identifier) @local.reference)
(route_statement source: (identifier) @local.reference)
(send_statement source: (identifier) @local.reference)
(send_statement bus: (identifier) @local.reference)
(modulate_statement signal: (identifier) @local.reference)
(parameter_path patch: (identifier) @local.reference)
