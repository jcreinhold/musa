# Verification contract

Musa distinguishes what a value *is*, what a composer explicitly *requires*, and what an analyst *interprets*.
Verification preserves those distinctions and makes the candidate falsifiable.

## 1. Constructor invariants

Constructor invariants are necessary for a value to inhabit its type and are checked at construction:

- a `duration` is a nonnegative exact rational;
- a scale is nonempty and has no duplicate spelled member within its period;
- a `triad` satisfies the declared tertian-member invariant;
- a voicing contains exact pitches licensed by its chord-class and omission policy;
- a `row12` is a bijection over `pc12`;
- a kernel quote closes, type-checks, and contains decodable `ScoreFact` payloads;
- a control curve has ordered exact points in the control's domain;
- an instrument implementation conforms to its declared instrument signature;
- an asset resolves to the immutable identity in the build closure.

Failure creates no value and is an error at the smallest source range that falsifies the invariant.

## 2. Explicit assertions

`assert predicate(args) { body }` is used for optional, decidable compositional requirements. The predicate observes a
finite typed value or the controlled view of a finite `music` value and returns either evidence or counterexamples.
The successful result is the unchanged body; assertions never repair or respell music.

Predicate names carry practice and assumptions: `species.first_above(cantus)` is honest;
`valid_counterpoint(cantus)` is not. Scale membership, range, spacing, row property, and user-declared finite predicates
fit this layer. Diagnostics include predicate, assumptions, witness locations, and a counterexample; they do not merely
say “false.”

## 3. Interpretive analyses

Analyses are named services returning `analysis[A]` with method, assumptions, observations, alternatives, confidence
where meaningful, and evidence locations. Roman-numeral analysis, tonicization/modulation readings, segmentation,
common-chord search, orchestration observations, and voice-leading labels belong here. Analysis results do not enter
kernel payloads unless the author explicitly writes an annotation derived from one, preserving provenance for that
choice.

There is no privileged “the analysis.” Two methods may return different well-typed results. A failed or ambiguous
analysis is data, not malformed music. Prompts 110–112 must keep construction APIs independent of analysis APIs.

## 4. Semantic laws

Implementations must test the following at the equality named in `00-semantics.md`.

1. **Core type safety and normalization:** the theorems of `02-core-calculus.md`.
2. **Context identity:** `in_scale(s,m) ≈music m` when `m` does not read scale.
3. **Context shadowing:** `in_scale(s,in_scale(t,m)) ≈music in_scale(t,m)`.
4. **Context distribution:** `in_scale` distributes over sequence and overlay under `≈music`.
5. **Independent context commutation:** rebinding different permitted fields commutes; the initial candidate exposes
   only scale, so no additional surface operation is inferred from this law.
6. **Chosen composition:** instantiation of sequence and overlay obeys the equations in `00-semantics.md`; no nested-
   timeline flatten law exists.
7. **Pitch action:** identity, composition, and cancellation from `03-musical-domains.md`.
8. **Scale round trip:** `locate(realize(...))` on members returns the canonical degree/register.
9. **Chord/voicing projection:** every voiced pitch projects to a licensed member; projection is intentionally many-to-
   one.
10. **Row closure:** every row form remains a bijection; distinct-form count is not fixed at 48.
11. **Quotation hygiene:** alpha-renaming a quote binder does not change `≡kernel`; antiquotation cannot capture or be
   captured.
12. **Template determinism:** unchanged closure and source sites generate byte-identical declarations and IDs.
13. **Provenance erasure:** changing only Origin may change full equality but not `≈facts`.
14. **Gesture exactness and routing:** profile realization is deterministic, lanes are isolated by `PartId`, and no
   unsupported control is silently discarded.
15. **Audio partition invariance:** rendering the same scheduled gestures in any legal block partition yields the same
   samples up to the processor's stated numerical tolerance; offline and live execute the same render operation.
16. **Build reproducibility:** identical locked build closure, compiler version, options, and target produce identical
   semantic artifacts and deterministic offline bytes where the backend promises them.

## 5. Counterexample suite

Each rejected shortcut has a fixture that would fail if the shortcut returned:

| Shortcut to reject | Counterexample |
| --- | --- |
| `music = Timeline[ScoreFact]` | bind one `step` phrase and use it in C major and C Dorian |
| implicit timeline join | overlay two sequences whose unequal extents make flattening choices disagree |
| key equals scale | ask for degree 6 in a minor key without natural/harmonic/melodic policy or register |
| pitch class equals `pc12` | spell C-sharp and D-flat in a notation-preserving transform |
| chord equals voicing | realize one Cmaj7 class in close and drop-2 voicings |
| analysis equals truth | provide a passage with plausible tonicization and modulation readings |
| dynamic equals gain/filter | swap two conforming instruments whose expression implementations differ |
| slur equals envelope | use a slur for phrase grouping on an instrument without continuous sustain |
| part equals patch | route two simultaneous parts to different instruments and assert isolation |
| `ControlKey` equals node index | replace a native graph instrument with an SFZ implementation |
| clip equals fixed media | change tempo across a beat-fitted loop and a field recording |
| source path equals asset identity | modify bytes without changing the file name and require lock mismatch |
| package range resolution | build offline with two packages requesting incompatible exact revisions |

## 6. Compatibility and performance gates

| Prompts | Gate |
| --- | --- |
| 93–96 | baseline fixtures captured; grammar/core type safety; finite budget diagnostics; no public elaboration API |
| 97–99 | contextual laws, higher-order corpus, and bundled library pass without eager occurrence explosion |
| 100–112 | domain constructor laws, source citations, assertions/analyses split, tonal and post-tonal counterexamples |
| 113–114 | standalone kernel and quotation preserve the existing kernel grammar and closure guarantees |
| 115–117 | formatter, LSP, workbench, and handbook expose exact syntax and teach domain distinctions |
| 118 | incremental and cold compilation meet recorded budgets; cache keys include context and build closure |
| 119–125 | sound vocabulary, exact values, gestures, typed instruments, part isolation, expression, and ergonomic binding |
| 126–132 | immutable assets, exact-pinned offline packages, sample adapters, media cues, and clip/fixed-media distinction |
| 133–136 | workbench and tooling preserve source authority; performance and conformance laws pass |
| 137 | full corpus, migration, docs, public API, performance, and governing-precedence audit |

No gate is satisfied by a unit test that recreates the implementation in the assertion. Property generators use small
independent models; end-to-end fixtures inspect public compilation/render results; compile-fail tests assert stable
diagnostic codes and salient labels, not whole prose strings.

## 7. Graduation evidence

Prompt 137 records: every law and counterexample fixture; benchmark baselines and variance policy; old/new corpus
compatibility; public API diff; kernel constructor diff (which must be empty); reproducible asset lock audit; live/offline
audio comparison; and a contradiction scan of all governing documents. Only then may `README.md` change from candidate
to governing.
