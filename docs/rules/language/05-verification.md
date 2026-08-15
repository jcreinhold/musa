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
finite typed value or the controlled view of a finite event track and returns either evidence or counterexamples. The
successful result is the unchanged body; assertions never repair or respell music.

Predicate names carry practice and assumptions: `species.first_above(cantus)` is honest; `valid_counterpoint(cantus)` is
not. Scale membership, range, spacing, and row property fit this layer. Diagnostics include predicate, assumptions,
witness locations, and a counterexample; they do not merely say “false.”

The predicate is drawn from a **fixed family the compiler owns**, not from the composer's own functions (prompt 116). An
arbitrary predicate would need the music handed to it, and the only thing there is to hand over is the elaborated score
— which would make every payload field a public interface, and would make what a claim *can* read a moving target. What
a claim reads instead is a controlled view: the sounded written pitches of the passage, with their exact spans, and
nothing else. The family is:

| Claim | Checks | Cites |
| --- | --- | --- |
| `fills_meter()` | the passage is exactly one measure of the meter in force where it is written | OMT `010` |
| `pitches_in(Scale)` | every sounded note is spelled as a member of the collection | OMT `013` |
| `realizes(ChordClass, policy)` | the sounded classes stand in the policy's relation to the chord's members, and the designated bass, if there is one, is the lowest note | OMT `017`–`019` |
| `voices(Nat)` | wherever anything sounds, exactly that many notes sound at once | OMT `022` |
| `within_ranges(List<(Pitch, Pitch)>)` | each voice of each sonority, counted from the bottom, lies in the range given for it | OMT `022` §Range |

The realization policy is one of three words — `exactly`, `may_omit`, `may_add` — and not a value of an
elaboration-language type: three inhabitants no function can take or return would be language surface with no caller.
`exactly` is set equality on pitch classes, `may_omit` lets a member be absent, and `may_add` lets other notes sound.
Doubling is invisible to all three, because which member is doubled is a fact about the voicing rather than about the
chord (OMT `019`).

`fills_meter()` is the claim `bar { … }` has been making since prompt 57. They are one obligation with two spellings,
recorded during elaboration and discharged once the barlines are settled — because a `meter` written in one voice
governs another voice's bars, so no claim about a measure can be answered where it is written.

## 3. Interpretive analyses

Analyses are named services returning `analysis[A]` with method, assumptions, observations, alternatives, confidence
where meaningful, and evidence locations. Roman-numeral analysis, tonicization/modulation readings, segmentation,
common-chord search, orchestration observations, and voice-leading labels belong here. Analysis results do not enter
kernel payloads unless the author explicitly writes an annotation derived from one, preserving provenance for that
choice.

There is no privileged “the analysis.” Two methods may return different well-typed results. A failed or ambiguous
analysis is data, not malformed music. Prompts 115–117 must keep construction APIs independent of analysis APIs.

`07-analysis.md` elaborates this layer: the observation boundary, what a finding and its evidence mean, and the rule a
new analysis kind must satisfy before it ships — an abstract domain, an abstraction map, and a soundness claim, in the
shape Peyton Jones (1987) §22.1 gives a program analysis. It governs the analysis service; this section governs it.

## 4. Semantic laws

Implementations must test the following at the equality named in `00-semantics.md`.

1. **Core type safety and normalization:** the theorems of `02-core-calculus.md`.
2. **Context identity:** `in_scale(s,m) ≈material m` when `m` does not read scale.
3. **Context shadowing:** `in_scale(s,in_scale(t,m)) ≈material in_scale(t,m)`.
4. **Context distribution:** `in_scale` distributes over `follow` and `together` under `≈material`.
5. **Independent context commutation:** rebinding different permitted fields commutes; the initial candidate exposes
   only scale, so no additional surface operation is inferred from this law.
6. **Chosen composition:** `follow` and `together` obey the equations in `00-semantics.md`; no nested-track flatten law
   exists.
7. **Pitch action:** identity, composition, and cancellation from `03-musical-domains.md`.
8. **Scale round trip:** `locate(realize(...))` on members returns the canonical degree/register.
9. **Chord/voicing projection:** every voiced pitch projects to a licensed member; projection is intentionally many-to-
   one.
10. **Row closure:** every row form remains a bijection; distinct-form count is not fixed at 48.
11. **Quotation hygiene:** alpha-renaming a quote binder does not change `≡core`; antiquotation cannot capture or be
   captured.
12. **Template determinism:** unchanged closure and source sites generate byte-identical declarations and IDs.
13. **Provenance erasure:** changing only Origin may change full equality but not `≈facts`.
14. **Gesture exactness and routing:** profile realization is deterministic, lanes are isolated by `PartId`, and no
   unsupported control is silently discarded.
15. **Audio partition invariance:** rendering the same scheduled gestures in any legal block partition yields the same
   samples up to the processor's stated numerical tolerance; offline and live execute the same render operation.
16. **Build reproducibility:** identical locked build closure, compiler version, options, and target produce identical
   semantic artifacts and deterministic offline bytes where the backend promises them.

### 4.1 Adapter expansion laws

A syntax adapter is the one place where source is read *as syntax*, and the eleven laws below are what make that
readable rather than a second, weaker language. They are stated over `02-core-calculus.md` §5.9's operations, and each
names the evidence that discharges it. Evidence is executable except where it is a proof, and a law whose evidence is a
paragraph is asserted rather than implemented.

| # | Law | Statement | Evidence |
| --- | --- | --- | --- |
| 1 | Sealed formation | A step exists only where the recursor's group case minted it: no constructor in any scope, no coercion from a node, no `data` field that could hold one. | Compile-fail: `SyntaxStep` is not a term; a region is not a step; a `data` field of step type, at any depth, is refused. |
| 2 | Association | A step run from inside a foreign traversal answers with *its own* algebra applied to *its own* child. | Differential test: an inner recursor over the original subject runs a captured outer step and gets the outer algebra's answer. |
| 3 | Inherited context | A branch is entered under exactly the context its parent's branch passed, and the root under the initial context. Nothing ambient supplies one. | A transformer whose branches emit their own context; the emitted contexts equal the passed ones, and change when the branch's choice changes. |
| 4 | Path uniqueness | Every node is entered at its own path, and the paths a recursor reports equal, in order, the paths the derived fold reports for the same region. | Law 9's differential comparison, whose two sides build their output from the paths they are handed and are compared byte for byte. |
| 5 | Structural decrease and reducibility | Each step application enters a strict subtree; termination is the §5.9 reducibility argument, which rests on definition acyclicity. | §5.9's lemma and fundamental-lemma cases; a nested recursor restarting on the original subject terminating under a finite budget. |
| 6 | Repeatability | A branch may run a step never, once, or several times, under different contexts each time. | Omission answering without reading its children; two runs of one step under two contexts, agreeing on subject and algebra. |
| 7 | Determinism | Two runs of one transformer over one region agree on the value **and** on the charge. | Repeated expansion over the region corpus, comparing both. |
| 8 | Opacity | No operation but `run_syntax_step` accepts a step, and none yields a node, path, range, scope, or algebra from one. | A registry law over every phase operation's signature, plus compile-fail cases handing a step to an operation that reads a node. |
| 9 | Fold derivation | `syntax_fold_from_leaves` is the recursor at a context nothing reads, and charges what the catamorphism it replaces charged. | Differential test over the region corpus: equal values; unchanged reduction kind, so no shipped adapter's budget moves. |
| 10 | Budget accounting | Minting is charged per child of every group entered, and running is charged per run; capture is not a way to buy work off the meter. | Charge comparisons: omitting < running once < running twice; a wider level entered costs more even when nothing is run. |
| 11 | Phase conservativity | `Syntax`, `NodePath`, `BindingPath`, and `SyntaxStep` are unknown in ordinary source; the completed phase result is storable data; the transformer uses the one checker and the one evaluator. | Compile-fail cases in ordinary source for every phase type and every phase operation; the registry's separateness law. |

## 5. Counterexample suite

Each rejected shortcut has a fixture that would fail if the shortcut returned:

| Shortcut to reject | Counterexample |
| --- | --- |
| a track value freezing its scale | bind one `step` phrase and use it in C major and C Dorian |
| implicit track join | place together two successions whose unequal durations make flattening choices disagree |
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
| 100–117 | domain constructor laws, source citations, assertions/analyses split, tonal and post-tonal counterexamples |
| 118–119 | standalone kernel and quotation preserve the existing kernel grammar and closure guarantees |
| 120–122 | formatter, LSP, workbench, and handbook expose exact syntax and teach domain distinctions |
| 123 | incremental and cold compilation meet recorded budgets; cache keys include context and build closure |
| 154–160 | sound vocabulary, exact values, gestures, typed instruments, part isolation, expression, and ergonomic binding |
| 161–167 | immutable assets, exact-pinned offline packages, sample adapters, media cues, and clip/fixed-media distinction |
| 168–171 | workbench and tooling preserve source authority; performance and conformance laws pass |
| 172 | full corpus, migration, docs, public API, performance, and governing-precedence audit |

No gate is satisfied by a unit test that recreates the implementation in the assertion. Property generators use small
independent models; end-to-end fixtures inspect public compilation/render results; compile-fail tests assert stable
diagnostic codes and salient labels, not whole prose strings.

## 7. Graduation evidence

Prompt 170 records: every law and counterexample fixture; benchmark baselines and variance policy; old/new corpus
compatibility; public API diff; kernel constructor diff (which must be empty); reproducible asset lock audit;
live/offline audio comparison; and a contradiction scan of all governing documents. Only then may `README.md` change
from candidate to governing.
