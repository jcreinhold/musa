# 08 — Open Questions

Deliberately undecided. Each entry states the question, the current working stance, and the evidence that would
settle it. Nothing here may be settled by convenience (course correction §32: "do not prematurely decide").

## Q1 — Infinite / live patterns

A `Pattern[A]` is **not** part of the finite kernel grammar (§18). Working stance: a pattern is anything that can
produce coherent finite observations `P(I)` for every bounded interval `I`, obeying the compatibility law
`J ⊆ I ⟹ restrict_J(P(I)) = P(J)`. Loops, algorithmic generators, aleatory realizations, and live-coded patterns all
expose finite kernel observations without sharing a computation model. *Settle when:* a concrete loop/live feature
(prompt 24-era or later) shows whether observation coherence needs kernel-level support (e.g. a `Pattern` type with a
restrict-based contract) or stays a library convention.

## Q2 — Aleatory semantics

Probability, nondeterminism, performer choice, and reactive improvisation are different phenomena (§32); **no**
universal `Choice` kernel construct exists or is planned. Working stance: each realized performance of an aleatory
surface construct produces an ordinary finite kernel timeline; the choice mechanism lives in the surface/HIR and its
provenance. *Settle when:* the first aleatory surface feature is designed — its provenance needs (which choice was
taken?) will show whether the kernel needs anything beyond occurrence payloads.

## Q3 — Voice identity

Candidates (§32): payload metadata; a separate temporal relation; HIR structure plus provenance; or a combination.
Working stance (adopted by `06-surface-elaboration.md`): **payload metadata + HIR structure** — `NotePayload.voice`
carries (part, voice) identity; the kernel stays identity-free; `ScoreSnapshot`'s lanes are an adapter projection.
*Settle when:* prompt 11's differential parity and prompt 21's score-editing show whether any consumer needs
voice-level temporal queries the payload projection can't answer cleanly (e.g. cross-voice alignment constraints).

## Q4 — Time-varying continuous controls

Automation (crescendo, glissando, parameter curves) does not obviously belong to discrete occurrences (§32).
Candidates: typed interval payloads (a `CurveRegion` payload); a separate behavior/curve layer; the
performance/audio model. Working stance: **undecided, deferred**. The §33 falsification items 6–7
(accelerando/ritardando, glissando/crescendo) exist precisely to force this question with evidence. *Settle when:*
prompt 31 (tempo/expression curves) designs the surface constructs — tempo is settled (§22: performance-layer map);
continuous *expression* is not.

## Q5 — Recursive / generative source programs

The surface language may eventually need recursion or generative facilities; this does **not** imply the finite
kernel needs them (§32). Working stance: surface programs must have finite observable output for any finite query;
termination is a surface-language static property (as with motif ordering today, roadmap §6.5). *Settle when:* a
surface recursion proposal exists; its elaboration must produce finite observations or be rejected.

## Q6 — Kernel-file parser

`01-grammar.md` defines the full interchange grammar (un-normalized expressions, named compositions), but prompt 09
implements canonical **serialization** only (N5). Working stance: no parser until a second producer/consumer of
kernel files exists (another implementation, a visualizer, a test oracle written in kernel text). *Settle when:*
that consumer appears. Today's golden files are written to remain parseable by the future grammar (N5 is a strict
subset).

## Q7 — Chord regrouping fidelity

Chords elaborate to simultaneous per-pitch occurrences; the snapshot adapter regroups by (span, voice, origin)
(`06-surface-elaboration.md`). Open: is (span, voice, origin) the right grouping key when two different chords in
the same voice share a span via `overlay` of separately-written material? Working stance: origin distinguishes
deliberate chords from coincidental simultaneity, since coincidental simultaneity arises from different source
constructs with different origins. *Settle when:* prompt 11's parity tests exercise overlaid same-span material, or
prompt 22's chord notation exposes a counterexample.

## Q8 — Key/meter regions

`06-surface-elaboration.md` keeps `key`/`meter`/`tempo` as piece-wide context maps because the current grammar has
no temporal extent for them. §21's typed interval payloads (`Timeline[KeyRegion]`, `Timeline[MeterRegion]`) are the
agreed shape *when extent matters* (`modulate to C major { … }`, meter changes, the §33 item 5). *Settle when:* the
surface grammar gains such a construct (prompt 30 era) — elaboration extends, kernel unchanged.

## Falsification corpus status (§33)

| # | Example | Status |
| --- | --- | --- |
| 1 | Twinkle Twinkle (sequential + rests) | **proven** (prompt 11): `examples/twinkle.musa`, parity + normal form |
| 2 | Four-part chorale (synchronized voices) | **proven at two parts** (prompt 11): `counterpoint.musa` parity + normal form; extend to four parts with prompt 22-era fixtures |
| 3 | Canon (reuse, delay, transformation, overlay) | **proven** (prompt 11): `examples/canon.musa` — motif reuse, delay by ambient extent, transposition, overlay |
| 4 | Tuplets / polyrhythm (exact rationals) | blocked on surface syntax (prompt 22) |
| 5 | Changing meter and key | blocked on surface syntax (see Q8) |
| 6 | Accelerando / ritardando | blocked on tempo curves (prompt 31); tempo-map semantics already settled (§22) |
| 7 | Glissando / crescendo | blocked on Q4 |
| 8 | Loop-based electronic music | blocked on surface loops (Phase 2/3; see Q1) |
| 9 | Controlled aleatory | blocked on Q2 |
| 10 | Improvisational / live process | blocked on Q1; the finite-kernel-as-observation stance is the hypothesis under test |

Rule (§33): if several of these require awkward or lossy lowering, reconsider the kernel as a whole. Do not patch
examples independently.

The kernel graduated at prompt 12 with items 1–3 proven and 4–10 tracked above.

## Prompt-implementation log

Anything discovered while implementing prompts 09–12 is appended here with its resolution, so the spec's graduation
(prompt 12) reviews a complete list.

- *(empty at specification time — prompt 08)*
- **Prompt 09 (kernel implementation):** D6's phrasing `[s, e] ∩ [i, j] ≠ ∅` makes degenerate (point) occurrences
  unobservable — a half-open empty intersection is always empty. Refined: point occurrences at `s` are visible through
  `[i, j)` when `s ∈ [i, j)`, plus (prompt 10) a point exactly at the ambient extent's end is visible through a window
  ending at the extent — without it, `restrict` at the full extent is not the identity (L16). Spec D6 is updated to
  match when the candidate banner comes off (prompt 12).
- **Prompt 10 (law suite):** the L17 property caught that an empty window `[i, i)` observed non-degenerate spans
  containing `i` (half-open intersection is empty, but the naive `s < j && e > i` test passes). Fixed:
  `Span::visible_through` returns `false` for empty windows.
- **Prompt 11 (elaboration):** a surface `rest` elaborates to a `Rest` payload occurrence (notation intent), not to
  absence — parity with the oracle requires rest events to carry their origin, which a bare gap cannot represent.
  This refines `06-surface-elaboration.md`'s rest row: "no **note** occurrence" still holds; the kernel gains no
  silence object. **Q3 evidence:** voice identity as payload metadata reproduced every fixture's lanes exactly;
  no consumer needed a temporal voice primitive. **Q7 evidence:** (span, voice, origin) regrouping is correct on all
  fixtures because coincidental simultaneity from separate constructs carries separate origins. **Transpose** is
  applied eagerly via the shared interval stack during elaboration rather than as a literal `map_payload` pass;
  composition commutativity (prompt 06's law) makes this observably equal, documented in `06` at graduation.
