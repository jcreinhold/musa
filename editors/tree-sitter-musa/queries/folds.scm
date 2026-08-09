; Folding (prompt 80): braces are the language's explicit structure, the
; same law prompt 79's `textDocument/foldingRange` answers from the hand
; parser's tree. Comment runs fold in the editor, not the query — a run is
; not a node.
[
  (piece_declaration)
  (library_declaration)
  (score_declaration)
  (part_declaration)
  (voice_declaration)
  (motif_declaration)
  (fragment_declaration)
  (performance_declaration)
  (profile_declaration)
  (settings_block)
  (studio_declaration)
  (patch_declaration)
  (bus_declaration)
  (block)
  (with_clause)
  (grace_statement)
  (mobile_statement)
  (harmony_declaration)
] @fold
