# 08 — Open Questions

**Status: governing — as questions.** What is deliberately undecided, and the evidence that would settle each. Nothing
here is closed because closing it is convenient.

Deliberately undecided. Each entry states the question, the current working stance, and the evidence that would settle
it. Nothing here may be settled by convenience: no open question is closed because closing it is convenient.

## Q1 — Infinite / live patterns — **narrowed (prompt 127a)**

A `Pattern[A]` is **not** part of the finite core grammar. Working stance: a pattern is anything that can produce
coherent finite observations `P(I)` for every bounded interval `I`, obeying the compatibility law
`J ⊆ I ⟹ restrict_J(P(I)) = P(J)`. Loops, algorithmic generators, aleatory realizations, and live-coded patterns all
expose finite core observations without sharing a computation model. *Settle when:* a concrete loop/live feature shows
whether observation coherence needs core-level support (e.g. a `Pattern` type with a restrict-based contract) or stays a
library convention.

**What prompt 127a settles, and what it leaves open.** One class of unbounded source now has an answer: a source that
runs — an oscillator, a delay line, a live input — is a **machine** (`../across-stages/03-machine-calculus.md`), a
finite description with a total one-step function whose *history* is the infinite object. That is a second core value,
not a `Pattern`, and it means the question is no longer "how does the core hold something infinite" for anything on the
audio side.

What remains open is the *notated* case: a generator that yields score facts forever, of which a bounded stretch is
notated. A machine is the wrong shape for it — a machine's step is a sample frame, and a notated fact has no frame — so
the compatibility law above is still the working stance and still has no consumer. A proposal must show the smallest
failing term and two materially different musical uses (`../across-stages/05-metatheory.md` §5).

## Q2 — Aleatory semantics — **RESOLVED (prompt 66)**

Probability, nondeterminism, performer choice, and reactive improvisation are different phenomena; **no** universal
`Choice` event-track construct exists or is planned. The working stance was: each realized performance of an aleatory
surface construct produces an ordinary finite event track; the choice mechanism lives in the surface/HIR and its
provenance. The stated trigger was: *"the first aleatory surface feature is designed — its provenance needs (which
choice was taken?) will show whether the event track needs anything beyond occurrence payloads."*

That trigger fired at prompt 66, and the working stance is **confirmed, not reversed**.

**Answer: a realization is a compile parameter, the freedom is a payload value, and the event track gains nothing.** The
candidate — a `choose` term form — is refused for four reasons set out in `11-realization.md`: it cannot express the
repertoire it is proposed for (*In C* is unbounded, Klavierstück XI is 19! orderings, Cage and Feldman draw from a
continuum); it makes T2 ambiguous, because whether sharing shares the *decision* has two musically real readings and no
canonical one; it destroys T3, T4 and N6 together, so there is no normal form and no semantic hash, which is what prompt
43 keyed recompilation on; and it destroys the interchange artifact that justified it, since a file containing `choose`
needs a choice environment shipped alongside it.

The provenance need the trigger asked about is answered **above** the event track by a `ChoicePath` — a structural path
of names, not a span and not a `DeclarationId` — with randomness derived per path rather than from a stream. The event
track sees a term in which every choice is already made.

The conceded cost is stated in `11-realization.md` and is real: the score view is a function of the source *and the
realization*, so fixtures pin a seed and the interface must be able to show which decisions produced the page.

## Q3 — Voice identity — **RESOLVED (prompt 119)**

Candidates: payload metadata; a separate temporal relation; HIR structure plus provenance; or a combination. The working
stance (adopted by `06-surface-elaboration.md`) was **payload metadata + HIR structure** — voice identity carries (part,
voice) on the fact, the event track stays identity-free, and `ScoreSnapshot`'s lanes are an adapter projection. It was
to be settled by a consumer that needed voice-level temporal queries the projection could not answer.

**Answer: the working stance holds, and the event track gains no succession relation.** The deciding consumer is
voice-leading and counterpoint analysis, which is the one part of the system whose entire subject matter is *lines*:
seven style profiles and twenty-four rules, nine of them species rules about what a single line does from note to note.
`crates/musa-score/src/analysis/motion.rs` serves all of them from a `Strand` that is the lane's own `(PartId, VoiceId)`
plus its tones sorted by onset — succession is "the next tone in that vector," and line order is the strand's mean
diatonic height. No relation, no threading pass, no predecessor field, no second identity.

The one place a line genuinely is not available is honest about it rather than papered over: a jazz voicing written as a
chord in a single lane has no lines inside it, so the analysis reads a *position from the bottom* and says so
(`docs/rules/language/07-analysis.md` §8). Where a line exists the tag names it; where none exists the analysis does not
invent one. An event-track-level partial order would have had nothing to do in either case.

Two things this deliberately does not settle, both recorded rather than closed. A **partially ordered** succession (OMT
`110-row-properties.md` §"Partially ordered sets" — segments fixed, internal order free) and **divisi** (one line
splitting and rejoining) are still inexpressible with per-note tags. Neither has a consumer, and under the standing rule
demand precedes design; the analysis is in `docs/notes/research/kernel-hypothesis/06-evidence-log.md` Gate 2, which is
what a future proposal has to beat.

## Q4 — Time-varying continuous controls — **RESOLVED (prompt 45)**

Automation (crescendo, glissando, parameter curves) does not obviously belong to discrete occurrences. The candidates
were: typed interval payloads; a separate behavior/curve layer; the performance/audio model.

**Answer: a typed payload value, and no event-track operation.** `Progress` (`03-denotational-semantics.md`) is a
monotone piecewise-linear map from an occurrence's normalized *local* time to a unit-free fraction. Because it is
indexed by local time, every event-track operation acts on the span and leaves the payload byte-identical — the
span-alone theorem, tested as L24. A behaviour layer was not needed and an absolute-time curve would have forced the
event track to look inside payloads, violating the layering rule.

The evidence that settled it was falsification-corpus item 7 (crescendo): the shape existed, it was invented inside
`performance.rs` and discarded, and it could not be serialized. That is a specification hole, not an implementation
difference.

Two things this resolution deliberately did not settle. `Progress` expresses no steps, no units, no periodic shapes and
no easing catalogue — see the type's non-goals. And falsification-corpus item 6 (accelerando/ritardando) remains a
*tempo* question, which the performance layer already owns; `Progress` is available to it as a shared value type if a
future prompt wants one, and that is an implementation convenience, not a change of layer.

## Q5 — Recursive / generative source programs

The surface language may eventually need recursion or generative facilities; this does **not** imply the finite core
needs them. Working stance: surface programs must have finite observable output for any finite query; termination is a
surface-language static property (as with motif ordering today, roadmap §6.5). *Settle when:* a surface recursion
proposal exists; its elaboration must produce finite observations or be rejected.

## Q6 — Kernel-file parser — **RESOLVED (prompt 48)**

`01-grammar.md` defines the full interchange grammar (un-normalized expressions, named compositions), but prompt 09
implemented canonical **serialization** only (N5). The working stance was: no parser until a second producer/consumer of
events files exists (another implementation, a visualizer, a test oracle written in events text).

**That trigger has fired, and this is what fired it.** The consumer is `musa events` (prompt 48): a command that reads a
`.musa.events` file and evaluates it, which is a producer/consumer pair independent of the compiler and therefore the
second implementation the stance was waiting for. What made it worth building is sharing — a term language with `let`
says "this is that material again", which no serialization of *values* can say, and which is the difference between a
canon's interchange file being one subject or four copies of one.

The order the stance implies was followed rather than short-circuited: specify first (prompt 46, `10-term-calculus.md`),
implement and prove (prompt 47), and only then parse (prompt 48).

**Resolution.** `musa-events/src/text.rs` holds a printer and a parser over an opaque payload; `musa-compiler` supplies
`ScoreFact`'s form; `musa events <file.musa>` prints and `musa events --check <file.events>` reads. The deliverable is
`examples/events/*.musa.events` — nine committed files a second implementation is validated against by reading one,
computing its normal form and semantic hash, and comparing. The round-trip law holds over generated terms
(`musa-events/tests/terms.rs`) and over every fixture (`musa-compiler/tests/events_interop.rs`).

Two things the stance predicted wrong, both repaired in place:

- **N5 is not a subset of the grammar.** It writes the N3 key bare and has no version header, and the N3 key is the
  *equality* serialization — for `ScoreFact` it deliberately omits provenance an interchange file must carry. The two
  serializations are separate. At prompt 48 both stayed exactly as they were. Prompt 176a later found that N5 itself is
  not uniquely framed and moved semantic identity to N6's versioned binary record; `05-normalization.md` states the
  current repair.
- **The payload grammar in `01-grammar.md` had to go.** Record-shaped payload values would make the event track know
  what a note is. A payload is now an opaque quoted string, and `01-grammar.md` states that repair.

## Q7 — Chord regrouping fidelity

Chords elaborate to simultaneous per-pitch occurrences; the snapshot adapter regroups by (span, voice, origin)
(`06-surface-elaboration.md`). Open: is (span, voice, origin) the right grouping key when two different chords in the
same voice share a span via `together` of separately-written material? Working stance: origin distinguishes deliberate
chords from coincidental simultaneity, since coincidental simultaneity arises from different source constructs with
different origins. *Settle when:* prompt 11's parity tests exercise overlaid same-span material, or prompt 27's chord
notation exposes a counterexample.

## Q9 — A `Behavior` abstraction over the queries

(Numbered 9, not 8: prompt 40 answered and deleted an earlier Q8, and reusing the number would make its log entry below
read as though it settled this.)

`covering` and `prevailing` (D10–D11) are two concrete queries. The alternative considered at prompt 44 was an
FRP-shaped `Behavior<V>` — `EventTrack::behavior(rule)` returning a sampled function of time, with `Step`, `Ramp`, and
`Coverage` rules — which is conceptually tidier, because it names the fact that a finite occurrence set induces total
functions of time, and is what a functional-reactive treatment of this domain would reach for.

It was not taken. It buys a vocabulary and costs a trait with one implementor per rule, a closure at every call site,
and a sampling protocol (`sample_over(window, step)`) that no current caller wants; a one-implementor trait is a
concrete type wearing a costume. Working stance: two concrete queries. *Settle when:* a **third** rule appears with
**two** callers each. That is the evidence that would justify the abstraction; until then it is speculative generality.

Q4 was named here as the most likely source of that third rule, and it landed one prompt later — **without producing
one**. `Progress` is a payload *value* sampled by whichever consumer holds it, not a rule the event track evaluates, so
the `Ramp` behaviour an envelope would have wanted turned out to be the consumer's sampling policy. The count of rules
is still one (`prevailing`), and the threshold is unmet by a wider margin than before.

- **Prompt 48 (interchange format):** two documents were repaired against the implementation rather than the other way
  round. (1) `01-grammar.md`'s record-shaped `payload` declaration was struck for an opaque quoted string — a
  self-describing file is not worth the event track knowing what a note is. (2) `05-normalization.md`'s "N5 is a strict
  subset of the grammar" was false in both directions: N5 writes the N3 key bare and has no version header, and the N3
  key for `ScoreFact` quotients away the definition span and declaration id an interchange file must carry. The key
  stayed untouched, the interchange text became a second function, and at prompt 48 no golden or semantic hash moved.
  Prompt 176a later replaced ambiguous display hashing with versioned framed N6 bytes while preserving N5 display.
  Consequently `musa events --normalized` prints the *interchange* spelling of the normal form, which `--check` accepts
  — a strictly better artifact than N5 bytes, which nothing can read. Also: `examples/events/*.musa.events` are plain
  files rather than insta snapshots, because a corpus that exists to be read by another implementation must be readable
  as events text, not wrapped in a `.snap` preamble.

## Falsification corpus status

Before the architecture may be called settled, it must elaborate materially different music cleanly. Each item names
what it is a test *of*, not just a piece.

| # | Example | Tests | Status |
| --- | --- | --- | --- |
| 1 | Twinkle Twinkle | ordinary sequential pitched material and rests | **proven** (prompt 11): `examples/twinkle.musa`, parity + normal form |
| 2 | A four-part chorale | multiple synchronized voices, harmonic simultaneity | **proven at two parts** (prompt 11): `counterpoint.musa` parity + normal form; extend to four parts with prompt 27-era fixtures |
| 3 | A canon | reuse, delay, transformation, simultaneity | **proven** (prompt 11): `examples/canon.musa` — motif reuse, delay by ambient duration, transposition, `together` |
| 4 | Tuplets and polyrhythm | exact rational temporal relationships | blocked on surface syntax (prompt 27) |
| 5 | Changing meter and key | contextual temporal information with no semantic special case | **proven** (prompts 63–64, pushed to its edge at 74): `meter none` is a *value* of the meter context, not a mechanism beside it, so music with no barlines needed no event-track form, no second time coordinate, and no new special case. `examples/changing-meter.musa`, `modulation.musa`, `cadenza.musa`, `chant.musa` |
| 6 | Accelerando and ritardando | symbolic beat structure distinguished from physical-time realization | **proven** (prompt 73): a gradual change is a `Progress` in the tempo marking's payload, integrated exactly at realization and printed at both ends. The symbolic track does not move — no notehead changes place, which is the distinction the test states. `examples/rubato.musa`, `riser.musa` |
| 7 | Glissando and crescendo | where continuous temporal behaviour belongs | **proven** (prompt 44): a shape is a `Progress` in the payload, not a term form. `examples/annotated.musa` |
| 8 | Loop-based electronic music | surface iteration producing finite observations | **proven** (prompt 67): a ranged repeat is decided once at compile time and everything below it is an ordinary exact repeat. `examples/loop-lengths.musa` |
| 9 | Controlled aleatory | multiple realizations, each an ordinary finite event track | **proven** (settled 66, implemented 67, surfaced 68; `11-realization.md`): `examples/mobile.musa` — nineteen fragments, 19! orderings, one permutation in the payload — and `examples/in-c.musa`, fifty-three decision sites that survive an edit to each other |
| 10 | An improvisational / live process | the finite core still useful when the producer is reactive | **proven as far as it can be, and no further** (prompt 68): `examples/changes.musa` writes the improvised chorus as a frame of the right duration with the changes on it, so the interchange file holds a complete, finite, exactly timed piece plus the instruction a player needs. A *reactive producer* is refused rather than deferred — musa compiles a reading of the work, it does not follow one. See prompt 68's Stop list |
| 11 | Polymeter and polytempo | parts counted and paced independently of the score around them | **proven** (prompt 75), and worth reading for how little it cost: `Meter` and `Tempo` were already inherited by `Override`, and `BarLines` was already built on an arbitrary sequence of meters, so both are a **scope argument** — `bars(scope)`, `IntegratedTempoMap::new(score, scope, …)` — and neither is an event-track form, a term, or a second algorithm. `examples/bulgarian.musa` (7/8 against 4/4, barlines that diverge), `hemiola.musa` (6/8 against 3/4, one grid beamed two ways), `canon-x.musa` (Nancarrow's shape: one part accelerating while the other decelerates) |

The rule: if several of these require awkward or lossy lowering, reconsider the event track as a whole. Do not patch
examples independently.

The event track graduated at prompt 12 with items 1–3 proven. Only item 4 is still open, and it is blocked on surface
syntax rather than on anything the event track lacks.

One lossy lowering is real and is exactly one, which is what item 11 exists to expose: SMF has a single tempo track, so
a polytempo export is sonically exact and notationally wrong — and says so.

## Prompt-implementation log

Anything discovered while implementing prompts 09–12 is appended here with its resolution, so the spec's graduation
(prompt 12) reviews a complete list.

- *(empty at specification time — prompt 08)*
- **Prompt 09 (event-track implementation):** D6's phrasing `[s, e] ∩ [i, j] ≠ ∅` makes degenerate (point) occurrences
  unobservable — a half-open empty intersection is always empty. Refined: point occurrences at `s` are visible through
  `[i, j)` when `s ∈ [i, j)`, plus (prompt 10) a point exactly at the ambient duration's end is visible through a window
  ending at the duration — without it, `restrict` at the full duration is not the identity (L16). Spec D6 is updated to
  match when the candidate banner comes off (prompt 12).
- **Prompt 10 (law suite):** the L17 property caught that an empty window `[i, i)` observed non-degenerate spans
  containing `i` (half-open intersection is empty, but the naive `s < j && e > i` test passes). Fixed:
  `Span::visible_through` returns `false` for empty windows.
- **Prompt 11 (elaboration):** a surface `rest` elaborates to a `Rest` payload occurrence (notation intent), not to
  absence — parity with the oracle requires rest events to carry their origin, which a bare gap cannot represent. This
  refines `06-surface-elaboration.md`'s rest row: "no **note** occurrence" still holds; the event track gains no silence
  object. **Q3 evidence:** voice identity as payload metadata reproduced every fixture's lanes exactly; no consumer
  needed a temporal voice primitive. **Q7 evidence:** (span, voice, origin) regrouping is correct on all fixtures
  because coincidental simultaneity from separate constructs carries separate origins. **Transpose** is applied eagerly
  via the shared interval stack during elaboration rather than as a literal `map_payloads` pass; composition
  commutativity (prompt 06's law) makes this observably equal, documented in `06` at graduation.
- **Prompt 34 (variation transforms):** the three transformations added no event-track constructor, which is the
  evidence the governing design rule asks for on the "smallest complete basis" question. `stretch` is the existing
  scaling action (L13) applied during elaboration and then *renotated*, because augmentation is a notational act as well
  as a temporal one — the event track scales the span, and the surface layer respells the written value. `invert` is an
  ordinary `map_payloads`, with the unspellable mirror image (past a double accidental) surfacing as a diagnostic rather
  than a event-track-level failure. `retrograde` is the interesting one: it is a plain function over the finite
  occurrence list in `musa-compiler/src/elaborate/`, reflecting each span about the ambient duration, and it needed
  **no** reversal primitive — the finite core's occurrences are already a materialized set, so reversal is a mapping
  over them rather than a construct they must be built with. Its laws (involution; anti-homomorphism for `follow`) are
  proven at the elaboration level in `musa-compiler/tests/transform_laws.rs`. Tie marks are the one thing reversal must
  repair: a tie is a relation to the *next* sounding group, so reversing moves each mark back one group, and double
  reversal restores the original — also a test. **Occurrence specialization** likewise stays above the event track: a
  `with { note n = p; }` clause is a positional payload edit applied after the call's body elaborates, recorded as an
  extra provenance step.
- **Prompt 35 (annotations and harmony):** phrases, form markers, and chord symbols are score-level annotations, and the
  the event track gained nothing for any of them — an annotation is *about* the music, so it never becomes an occurrence
  and never has to. The evidence is a law: `musa-compiler/tests/annotation_laws.rs` compiles the same piece with and
  without annotations and compares every event, which is the precise sense in which they are outside the ontology. The
  one design question they raised is anchoring, and the answer is that it differs by kind: a **phrase** is anchored to
  events, like a slur, so re-barring keeps it on its notes; a **section** and a **chord symbol** are anchored to *time*,
  because a bar line is a place whether or not a note starts there. Positions (`at 3:1`) are resolved against the meter
  map during elaboration and refused when they fall past the end of the piece — a marker nobody reaches is a mistake,
  not a marker. Q8 was untouched by any of this; prompt 40 answered it.
- **Prompt 36 (imports and curves):** neither half touched the event track, for the same reason and in two different
  ways. An **import** is resolved before elaboration begins: a library contributes declarations, and a declaration is
  not an occurrence until something uses it, so an imported motif and a locally written one elaborate through identical
  code (`import_laws.rs` asserts the two produce the same events). A **tempo change** is a change to the map from beats
  to seconds, not to the track — every note keeps its symbolic position and the performance layer integrates the
  segments, which is why `a_hairpin_moves_no_note` and the tempo laws can both be stated as "the score is unchanged". A
  **hairpin** is an annotation with duration, like prompt 35's phrase, anchored to events; it is read at the performance
  boundary and nowhere else. falsification-corpus item 6 is therefore partly answered: stepwise tempo is exact, and a
  continuous ramp remains a surface-syntax question rather than an event track one — the piecewise map already has the
  shape a ramp would lower into.
- **Prompt 37 (observation and the end of `extend`):** two repairs, both of them removals. `extend` was deleted because
  no caller ever appeared: `follow` and `together` compute durations themselves, and the surface has no construct that
  asks a track to grow without adding material. Under the governing design rule that is the evidence the basis is one
  operation too large, so K3, D4, and L7–L8 are struck; ambient extension as a *concept* — `(d, ∅)`, and `together`
  taking the maximum without padding — is untouched, since it was never the operation. Re-adding `extend` requires a
  caller, not a taste. The second removal is `ObservedOccurrence`: it stored the visible span next to the whole span,
  two facts that must agree, kept in two places. `Observation<'a, A>` computes the visible span on the way out instead,
  and *narrowing an observation intersects the windows* — which makes L17 hold for arbitrary windows rather than for
  nested ones under a precondition the caller had to respect, and deletes the error that precondition would otherwise
  have needed. The observation carries the duration it was taken from, because "the final instant of a track is
  observable" is a fact about the track and not about the window; without it, narrowing a full-duration observation
  would drop the point occurrence the wider one reported (`the_final_instant_survives_narrowing`). D6 now states that
  rule positively, and L16 is derived from it. `06-surface-elaboration.md`'s rest row, which prompt 11 had already
  contradicted, is repaired to match the code.
- **Prompt 39 (every notated fact is an occurrence):** slurs, phrases, tuplets, dynamics, and hairpins became
  occurrences with their own spans, and the event track gained **nothing** — no constructor, no variant, no change of
  any kind. That is the evidence the governing design rule asks for on heterogeneity: a payload type with seven variants
  is a payload, and the event track never looks inside one. **Q3 is answered as far as evidence can answer it:** there
  is now exactly one track per compilation, with part and voice identity carried in the fact's `Scope`, and every
  fixture projects back to byte-identical events and annotations — a temporal voice primitive would have had nothing to
  do. **Q7** is strengthened for the same reason: chord regrouping by (span, scope, origin) still holds when the track
  also contains regions and points, because those are neither. The thing that went is `retie`: a tie was encoded as a
  flag copied onto notes, so reversing time broke a relation that had to be repaired afterwards. Merging tied noteheads
  at elaboration — at *every* nesting level, so an inner block's ties are resolved before it is reversed or scaled —
  deletes the relation instead of repairing it, and prompt 34's double-reversal law now passes for a simpler reason than
  it used to.
- **Prompt 40 (key, meter, and score annotations as occurrences):** **Q8 is answered, and deleted from this file.** The
  working stance was "context maps until the surface gives them duration"; the prompt gave them event-track duration
  *without* waiting for the surface, and that order turned out to be the right one. A region covering `[0, d]` is not a
  special case, but a piece-wide scalar named `MeterMap` is: had the region shape waited for `modulate`, the type would
  have grown a second representation and every consumer would have learned two ways to ask one question. The event track
  again gained nothing — no file in `musa-events` was touched — which is the core's stability promise demonstrated
  instead of asserted. Two smaller findings. First, deleting `piece_extent` (a maximum over event ends) in favour of the
  track's own duration is only sound because a written rest is an occurrence, so a piece ending in silence still ends
  where the silence ends; that is now a fixture rather than an argument. Second, `resolve_position` used to read the
  snapshot it was helping to build — the last place where a temporal fact was computed from the adapter's output rather
  than from the track — and it now reads the meter occurrence. **Tempo did not move and will not**: it is the map from
  symbolic to physical time, and a place where `stretch` and *ritardando* could be confused is exactly what the event
  track must not offer. `TempoMap` now carries that reasoning as a comment, because the next reader will otherwise ask
  why tempo was left behind and answer the question wrong.
- **Prompt 44 (coverage and prevailing-value queries):** the event track gained an *interface*, not an ontology — two
  queries, no constructor, no stored state, no payload requirement. The finding that justified them is that four
  consumers were answering "what is in force here" privately and **disagreeing**: two keyed the answer on
  `origin.definition_span` (source position, correct only while every context fact spans the whole piece), and the four
  differed on end instants, point occurrences, and coincident onsets. The four boundary conventions are now stated once
  in D10–D11 and tested as L20–L23. **Q9 opened** with the `Behavior<V>` alternative that was declined, and the evidence
  that would settle it. The performance rule was added because a point query invites O(n²): the event track defines what
  the answer is, and bulk derivation still does one ordered pass — P3 is what enforces it, and it did not move.
- **Prompt 45 (continuous shape):** **Q4 resolved.** The answer is a payload *value*, `Progress`, and it cost the event
  track no operation and changed no law — the third piece of governing-design-rule evidence after prompts 39 and 44. The
  whole design is one decision: index the curve by the occurrence's *normalized local* time, and every operation then
  acts on the span and leaves the payload byte-identical (the span-alone theorem, L24). An absolute-time curve would
  have forced the event track to look inside payloads to transform them, violating the layering rule and breaking
  L11–L15. Two boundaries were worth stating because they will be pushed on: shape is normative but *sampling policy* is
  the consumer's (`07-backend-contract.md`), and `Progress` expresses no steps, no units, no periodic shapes and no
  easing catalogue — a sudden change is a fact at a point, which D11 already answers. `serde` was deliberately not added
  to the event track for this; `musa-compiler` adapts the breakpoints instead.
- **Prompt 46 (the term calculus, specification only):** the calculus is six forms and a reference, and it adds no
  meaning — the acceptance test for a form is that it serves sharing, deferred observation, or interchange **and**
  denotes a track `03` already defines. Two forms were argued about and settled the same way. `shift` is sugar with a
  stated expansion, because a primitive that only restates `seq` is what D4's striking established the event track does
  not keep. `map f` is **not** a term at all, because naming a function is the door to general computation; the price is
  that an interchange file can say "this section is that section" but not "…transposed", and if that is ever wanted the
  answer is a payload-level interval, not a term-level function. Prompt 45's `Progress` arrived one prompt earlier and
  needed **no** curve form, which is the scope rule paying off on the first construct that tested it.
- **Prompt 119 (voice-leading and counterpoint profiles):** **Q3 resolved**, by the consumer that was always going to
  decide it. Prompt 39 could only say Q3 was "answered as far as evidence can answer it," because nothing yet *asked*
  about lines; twenty-four counterpoint rules ask about nothing else. They are served by the existing per-note tag plus
  time ordering, and the event track gained nothing — the fourth piece of governing-design-rule evidence after prompts
  39, 44, and 45, and the only one where the missing construct had a worked-out design waiting for it. Two findings are
  worth keeping. First, the analysis needs lines to be *ordered*, not merely distinguished, and mean diatonic height
  orders them without any declaration — "soprano" is a name, not a rank, and a rank read off the music survives a piece
  that names its voices badly. Second, the interesting case is the one with no lines at all: a jazz voicing in one lane
  is a chord, and the right move was to read vertical position and document the limit rather than synthesize lines the
  source did not write. An event track succession relation would have had to answer the same question and would have had
  to answer it the same way.
- **Prompt 127a (the event-track and machine core):** a governance amendment, not an implementation. Three effects on
  this file. **Q1 is narrowed**: a running source is a machine, so the unbounded-audio half of it is answered and only
  the notated half stays open. **Q5 is unchanged and now has a sharper reason**: the source language is total and has no
  general recursion, so a recursive source program is refused at the language level rather than deferred to elaboration.
  And every question here is now scoped to *one of two* core values — nothing in this file asks about machines, because
  `../across-stages/03-machine-calculus.md` §8 keeps its own open list. The vocabulary changed throughout
  (`timeline`→`track`, `extent`→`duration`, `sequence`→`follow`, `overlay`→`together`); the history above is reworded,
  not rewritten, because what was learned did not change.
