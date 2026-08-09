---
id: 92
slug: elaboration-language-spec
status: done
depends_on: [49, 63, 86, 91]
phase: 3
---

# Specify the Elaboration Language Candidate

## Task

Turn `docs/elaboration-language.md` from a revised proposal into a precise candidate specification before any new syntax
or compiler path is implemented. Reconcile the roadmap, course correction, kernel elaboration documents, style guide,
and prompt stack around one staged design: a total value calculus; contextual, context-neutral `music`; closed kernel
terms; declaration templates; a typed kernel escape; and an explicit score→performance-gesture→instrument→signal→mix
pipeline. Settle the remaining surface spellings with a corpus that a musician can read and a language implementor can
type-check without hidden rules. The candidate is the implementation contract for prompts 93–136 but does not outrank
the existing governing documents until prompt 137's audit graduates it.

## Read

- `docs/elaboration-language.md`, in full. Its rejection of timeline flattening, distinction between open `music` and
  a closed term, theory-domain separations, equality relations, and private compiler boundary are the decisions this
  prompt makes precise rather than re-litigates.
- `docs/course-correction.md` §§2–5, 13–14, 19–20, 24, 29, 34–35 and every file in `docs/kernel/`, especially
  `06-surface-elaboration.md` and `10-term-calculus.md`. The kernel still has no join, lambda, scale, chord, or musical
  payload knowledge.
- Roadmap §§2–10, 15, 17–19; `docs/interface/03-interaction.md` and `04-provenance.md`.
- Open Music Theory (OMT) `005`, `013`–`021`, `023`–`028`, `033`–`036`, `049`–`051`, `061`–`076`, and
  `099`–`110`
  under `~/Code/papers/music-theory/open-music-theory/`. Cite the exact chapter file for every imported music-theory
  definition. Claims not supplied by OMT must be stated as Musa definitions and proved from those definitions.
- OMT `007-other-aspects-of-notation.md`, `084-drumbeats.md`, `098-twentieth-century-rhythmic-techniques.md`,
  `114-core-principles-of-orchestration.md`, and `116-transcription-from-piano.md` for dynamics, articulation,
  instrument-dependent realization, sampled/percussive timbre, fixed-time notation, and orchestration. Do not infer a
  DSP mapping from a notation term merely because both affect perceived sound.
- Roadmap §§6.4–6.5, 13, 14.4, 16, and 18 Phase 4; `docs/kernel/07-backend-contract.md`; the current
  `musa-compiler` performance/profile/studio code and `musa-audio` graph/plan code. Treat the shared-note-stream warning,
  ignored `PerformanceEvent::Parameter`, graph-addressing surface, and eager `f64` studio values as named design debts,
  not architectural precedents.

## Design

Create `docs/language/` as a small normative candidate specification, not a second essay:

1. `00-semantics.md` — the four representations and two staging judgments; ownership boundaries; contextual
   `instantiate`; context-neutrality; closure to `Term<ScoreFact>`; equality and provenance.
2. `01-surface.md` — grammar additions and desugarings, including `fn`, `let`, types, calls, `music`, `in scale`,
   assertions, structural templates, module parameters, and kernel quotation. Every example must be both readable aloud
   and unambiguous to the lossless parser.
3. `02-core-calculus.md` — monomorphic STLC after elaboration, finite inductive data, structural eliminators,
   call-by-value evaluation, static judgments, resource rejection, and the proof obligations later prompts discharge.
4. `03-musical-domains.md` — pitch/interval, spelled pitch class versus `pc12`, key versus scale, degree/register,
   chord class/triad/voicing, row, and analysis-result definitions with OMT citations or local proofs.
5. `04-templates-and-modules.md` — declaration-template judgment, stable generative identity, module signatures and
   static functors; no first-class pieces, voices, modules, or source reflection.
6. `05-verification.md` — constructor invariants, explicit assertions, interpretive analyses, laws, counterexamples,
   and the compatibility/performance gates for prompts 93–137.
7. `08-performance-and-sound.md` — the staged semantic judgments from score facts through exact performance gestures,
   physical scheduling, typed instrument contracts, private implementations, audio signals, and mix routing. Define
   standard and namespaced controls, exact control curves, instrument swapping, part isolation, block-partition
   invariance, offline/live agreement, and the one late boundary where exact values become frame/DSP values. A raw DSP
   parameter index is never a musical control, and neither a dynamic nor a slur denotes a filter or envelope.
8. `09-assets-and-packages.md` — project build closure, immutable asset identity, local and exact-pinned package
   resolution, offline compilation, sampled-instrument interchange adapters, and the distinct semantics of note-driven
   samples, beat-fitted clips, and fixed-media cues. Explicitly reconcile the roadmap's relative-import-only decision;
   retain no registry and no version solver.

In `08-performance-and-sound.md`, state the core factorization for each part `l`:

```text
Timeline[ScoreFact] --profile--> GestureTimeline[Signature]
                    --tempo-->   ScheduledGestureLane
                    --instrument implementation--> Signal
part Signals        --mix graph--> stereo Signal
```

Define an instrument by the behavior it exposes, not by how it is built: a typed signature of note gestures and
controls plus a private implementation which may be a native graph or sample map. Score, performance, instrument, and
mix declarations remain independently editable; one deep audio preparation operation binds them after compilation.
Compare this boundary with both the current independent `PerformancePlan`/`StudioSpec` handoff and a rejected combined
score-audio object.

Distinguish stable `PartId`, selected instrument declaration, and prepared instrument instance. Also distinguish a
semantic `ControlKey` from a private resolved render-plan parameter index. Standard controls include at least
expression, emphasis, separation, brightness, sustain, and legato/phrase grouping; libraries may declare typed
namespaced controls. Profiles interpret notation into gestures and normalized musical controls; instruments map those
controls to synthesis/sample behavior. Unsupported techniques and unbound controls follow an explicit
error/warning/fallback policy and are never silently ignored.

The surface corpus includes the root-dependent turn, major/dorian rebinding, a higher-order canon, a harmonizer using a
controlled pitch traversal, a key-parameterized piece, a parameterized voice, a chord class in two voicings, a generic
and symmetric twelve-tone row, a successful and failing assertion, a standalone `.musa.kernel` document, and a local
quote with antiquotation. It also includes two parts with swappable instruments and different profiles; one hairpin
driving an exposed `expression` control rather than a DSP address; an instrument with normalized and physical custom
controls; a shared room send; a sampled instrument; a beat-fitted loop; and a fixed-duration field-recording cue. For
each, state the desugaring and the equality under which it is correct.

Compare the rejected public `musa-elaboration` crate with the chosen private `musa-compiler` subsystem. Record the
actual callers and why `Type`, `Value`, `Closure`, `Music`, module environments, and theory algorithms stay private.
No new public API is justified by a specification document.

## Target

- `docs/language/{00-semantics,01-surface,02-core-calculus,03-musical-domains,04-templates-and-modules,
  05-verification,08-performance-and-sound,09-assets-and-packages}.md`.
- Deliberate repairs to `docs/{initial-design-roadmap,course-correction,style-guide}.md` and
  `docs/kernel/06-surface-elaboration.md`; remove or mark every contradiction while keeping existing governing
  precedence until prompt 137.
- `docs/language/README.md`: candidate status, precedence, scope, document map, and prompt-137 graduation condition.
- `docs/elaboration-language.md`: marked as non-governing design input and linked to the split candidate specification.
- A source-map table in `03-musical-domains.md`: concept, Musa definition, OMT chapter or local theorem, falsifying
  example, and implementing prompt.

## Check

```sh
test -s docs/language/00-semantics.md
test -s docs/language/01-surface.md
test -s docs/language/02-core-calculus.md
test -s docs/language/03-musical-domains.md
test -s docs/language/04-templates-and-modules.md
test -s docs/language/05-verification.md
test -s docs/language/08-performance-and-sound.md
test -s docs/language/09-assets-and-packages.md
rg -n "Timeline\[Timeline|context-neutral|pc12|declaration template|antiquotation" \
  docs/language docs/kernel/06-surface-elaboration.md
rg -n "GestureTimeline|instrument signature|ControlKey|fixed-media|offline" docs/language
git diff --check
cargo fmt --check
```

Commit as `Specify the elaboration language candidate`.

## Stop

- No Rust, Svelte, tree-sitter, or example-source changes.
- Do not add a kernel constructor or weaken the `.musa.kernel` calculus.
- Do not put signals, samples, instruments, buses, physical seconds, or DSP parameters in the kernel.
- Do not make a profile target a raw graph node or make a patch's private topology part of its public contract.
- Do not leave syntax alternatives in a normative candidate rule. Open punctuation is decided here from the corpus.
- Do not claim a music-theory law from terminology alone; cite OMT or give Musa's definition and proof.
- No macro system, general recursion, effects, first-class syntax, first-class piece/voice values, or public elaboration
  crate.
