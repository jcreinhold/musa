# How to read the cross-stage specification

## Scope

Musa has several formal stages, not one universal kernel language. This specification fixes their boundaries and the
objects which cross them. It answers:

- what sort of artifact exists at each stage;
- which judgments construct one stage from another;
- which equalities are meaningful;
- how lineage and loss compose; and
- what must be true for preparation, caching, and execution to be sound.

It does not define a universal theory of music. Pitch, metre, harmony, form, phrase, gesture, timbre, and analytical
function belong to named theory or realization owners. The built-in domains remain concrete launch definitions.

## Three distinct kinds of rule

1. **Formation and typing rules** decide whether a finite artifact is well formed.
2. **Operational rules** say how an accepted term, pass, or process advances.
3. **Denotational rules** identify the mathematical object an accepted artifact denotes.

An implementation optimization is not a fourth kind. It must refine the displayed rules.

## Notation

- `A,B` range over payload or value types.
- `P,Q` range over presentation kinds.
- `M,N` range over finite temporal values.
- `G` ranges over finite process graphs.
- `x≡_P y` is the admitted semantic equality for presentation `P`.
- `⟦x⟧_P` is the denotation of an accepted artifact in its native presentation.
- `Result X E` is a total result with successful carrier `X` and finite error carrier `E`.

The same glyph never licenses equality between different presentations. A cross-presentation relationship is a pass
judgment from Chapter 2.

## Authority and proof status

Normative definitions are stated directly in these chapters. The proof record is summarized in Chapter 5. A theorem
whose premises include an ownership, purity, resource, or processor-conformance contract is only as applicable as the
implementation's evidence for that contract. The architecture map records that evidence separately.
