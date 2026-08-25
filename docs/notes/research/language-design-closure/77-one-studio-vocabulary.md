# 77. One studio vocabulary

**Status: governs nothing.** This record preserves prompt 175's amendment to the candidate sound specification.

> **Ownership superseded by note 79.** The `q`→`resonance` spelling and migration remain. The claim below that
> `musa-dsp` owns the editable/source catalogue is preserved as the argument that was later corrected: source
> declarations own that vocabulary, while `musa-dsp` owns registered primitive and preparation facts.

## Reason

The sound page promised that filter `q:` would remain accepted temporarily while newly inserted source used
`resonance:`. That promise would require two surface keys in every compiler, hover, completion, structured-editor, and
reference reader. Before prompt 175 there was no authoritative catalogue or deprecation path, so the compatibility row
described machinery that did not exist. One stable catalogue is the engineering reason to end that split now.

`resonance` is also the musician-facing concept. Its technical note can explain that this implementation represents it
as quality factor Q; requiring the graph parameter's implementation name in source would expose the private DSP model
the same sound specification otherwise keeps behind an instrument implementation.

## Affected source and replacement rule

The committed sources that stop compiling are `examples/glass-mountain.musa`, the album patch library, the studio
handbook example, and three test fixtures. Each changes only the label `q:` to `resonance:`. No stored event track,
score snapshot, prepared plan, audio sample, or public serialized identity changes.

The replacement rule is: `lowpass` and `highpass` declare one unitless parameter named `resonance`. A written `q` never
binds as an alias. It produces a stable hard diagnostic with the certain edit `q` → `resonance`; all other unknown
parameter names retain the ordinary unknown-parameter diagnostic.

## Ownership and migration

Prompt 174b placed editable studio vocabulary in `musa-dsp`, below the compiler. The versioned surface catalogue lives
beside that vocabulary. The compiler consumes it for checking and fixes; LSP, project/UI facts, and generated reference
material read the same entries. The private graph descriptor may keep the implementation key `q`, joined to the public
parameter by a checked stable processor/parameter mapping.

Musa has no released stable project format at this boundary. Repository source migrates in the prompt 175 commit, and
older text gets an exact code action. Keeping `q` as a deprecated alias was rejected because it would make the alias a
permanent input to every catalogue consumer while buying no preservation of compiled or serialized data.
