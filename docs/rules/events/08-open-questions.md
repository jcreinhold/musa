# Open questions

**Status: governing as questions.** These are the event-track decisions Musa has deliberately not made. Each question
states the current boundary and the evidence required to move it. Convenience is not evidence.

Resolved questions do not remain here as alternative designs. Their answers live in the specifications that now own
them; the arguments and rejected alternatives remain in `docs/notes/research/`.

## Q1 — Infinite notated patterns

`Pattern(A)` is not part of the finite event-track grammar. A possible pattern would produce a coherent finite
observation `P(I)` for every bounded interval `I`, satisfying

```text
J ⊆ I  ⟹  restrict_J(P(I)) = P(J).
```

This question concerns notation generated without a finite end. It does not concern an oscillator, delay, live input, or
other running source: those are finite machine descriptions with total one-frame steps
([machine calculus](../across-stages/03-machine-calculus.md)). Their histories are infinite; their source values are
not.

**Current boundary.** Surface programs may compute finite tracks, and ranged repetition or a chosen realization may
produce a finite result. The event track itself stays finite.

**Evidence required.** A proposal must present one smallest inexpressible program and two materially different musical
uses. It must show whether observation coherence needs a core contract or can remain a library property, and it must
preserve exact identity and finite interchange observations.

## Q7 — Chord regrouping fidelity

A written chord elaborates to simultaneous per-pitch occurrences. The score projection regroups occurrences by span,
voice, and origin.

**Open point.** Two separately written passages may be combined so that different chords in one voice share a span.
Origin should distinguish deliberate chords from coincidental simultaneity, but the grouping key has not been proved
complete for every such construction.

**Evidence required.** A source example in which two distinct written chords have the same span, voice, and effective
origin but must remain separate. The repair belongs in provenance or projection unless the example demonstrates a
musical fact the event track cannot represent.

## Q9 — A common abstraction for temporal queries

The event track exposes concrete queries such as `covering` and `prevailing`. An FRP-shaped `Behavior(V)` could instead
represent a total value sampled through time, but it would also introduce a sampling protocol and an abstraction with
too little use.

Continuous shapes do not supply the missing case: `Progress` is an opaque payload value sampled by the consumer that
understands it, not a rule evaluated by the event track.

**Current boundary.** Keep concrete queries with concrete result types.

**Evidence required.** A third temporal rule with at least two independent callers, where a common abstraction removes
real duplicated semantics rather than merely giving two functions a shared name.

## Closed questions and their owners

| Former question | Current answer |
| --- | --- |
| Aleatory semantics | A realization is a compile parameter; each result is one ordinary finite track ([realization](11-realization.md)). |
| Voice identity | Part and voice remain payload/projection facts; the event track gains no succession relation ([surface elaboration](06-surface-elaboration.md)). |
| Continuous controls | A continuous shape is a typed payload value, not a track operation ([denotational semantics](03-denotational-semantics.md)). |
| Recursive source programs | Structural totality is checked in the source calculus; every accepted observation is finite ([core calculus](../language/02-core-calculus.md#24-termination-is-structural)). |
| Events-file parsing | The closed interchange grammar has an independent parser and evaluator ([grammar](01-grammar.md)). |

## Falsification corpus

The event-track architecture has executable coverage for materially different music, not only unit examples.

| Need | Evidence |
| --- | --- |
| Sequential notes and rests | `examples/twinkle.musa` |
| Several simultaneous lines | `examples/counterpoint.musa`, `examples/analysis/satb.musa` |
| Reuse, delay, transformation | `examples/canon.musa` |
| Tuplets and polyrhythm | `examples/tuplet-fixture.musa`, `examples/hemiola.musa` |
| Changing and absent meter | `examples/changing-meter.musa`, `examples/cadenza.musa`, `examples/chant.musa` |
| Tempo ramps | `examples/rubato.musa`, `examples/riser.musa` |
| Continuous annotations | `examples/annotated.musa` |
| Finite generated repetition | `examples/loop-lengths.musa` |
| Controlled aleatory | `examples/mobile.musa`, `examples/in-c.musa` |
| Improvisational instructions in a finite reading | `examples/changes.musa` |
| Polymeter and polytempo | `examples/bulgarian.musa`, `examples/hemiola.musa`, `examples/canon-x.musa` |

If several new examples require awkward or lossy lowering, reconsider the event track as a whole rather than patching
each example independently. One known export loss does not alter the core: Standard MIDI Files have one tempo track, so
polytempo MIDI can be sonically exact without preserving the page's independent tempo lanes. The exporter reports that
loss.
