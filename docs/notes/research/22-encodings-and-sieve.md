# K₁ encodings and the five-case sieve

**Status: constructive falsification notebook. Governs nothing.** These are schematic encodings, not standard-library
designs and not authoritative formalizations of the named practices. Their purpose is to expose dummy fields, forced
translations, and missing generic structure in [20](20-candidate-staged-algebras.md)–[21](21-semantics-of-k1.md).

The standard is not “the language can encode it somehow.” A general-purpose lambda calculus can encode almost anything.
The standard is:

1. the practice's own salient distinctions remain visible in public types;
2. no Western surrogate is required;
3. composition uses a small named operation with the expected laws;
4. notation and audio can be connected without identification; and
5. failure or information loss appears in a type rather than a comment.

---

## 1. Common-practice tonality as a package

This is the launch case because Musa already implements much of it and Open Music Theory gives a concrete corpus. It
does not receive core privilege.

```text
signature WesternTonal {
    data type NoteName;
    data type WrittenPitch;
    data type Pc12;
    data type Interval;
    data type Scale;
    data type Frame;
    data type Key;
    data type Degree;
    data type Voicing;
    data type ChordClass;
    data type Roman;

    let forget_register  : WrittenPitch → NoteName;
    let forget_spelling  : WrittenPitch → Pc12;
    let frame             : Key → Frame;
    let degree            : Frame × WrittenPitch → Option Degree;
    let realize_degree    : Frame × Degree × Register → WrittenPitch;
    let build_chord       : Frame × ChordRecipe × Voicing → Result (List WrittenPitch) ChordError;
    let classify_chord    : Frame × List WrittenPitch → List (AnalysisClaim ChordClass);
    let analyze_function  : AnalysisContext × List WrittenPitch → List (AnalysisClaim Roman);
}
```

Several useful things “fall out,” in a precise sense:

- key-relative degree is a function from a frame, not a global coordinate;
- transposition is an action defined by this package's interval algebra;
- a chord constructor uses finite data and package functions;
- chord classification is separate from construction and may return several evidence-bearing analyses;
- a static structure template parameterized by a `WesternTonal` signature is a functor in the ordinary module sense.

Nothing falls out merely from generic parallel composition. `over(notes)` tells us which tones coexist; it does not
prove a chord class or harmonic function.

### Test result

**Pass.** The current concrete pitch tower fits as one sealed package. K₁ makes it slightly less convenient than today's
built-in base types until import and type inference are designed well, but it loses no domain distinction.

**Required engineering concession.** Musa should ship a short default import or package alias for its launch repertoire.
Avoiding ontological privilege does not require making ordinary notation verbose.

## 2. Karnatak rāga without reducing it to a scale

The source evidence in [19](19-domain-obligations.md) requires svara, gamaka, prayoga, tonic/drone context, instrument
or vocal realization, lineage, and improvisational or compositional practice to remain relatable without asserting that
one is the decoration of another.

One schematic signature is:

```text
signature RagaPractice {
    data type Raga;
    data type SvaraIntent;
    data type GamakaGesture;
    data type Prayoga;
    data type Phrase;
    data type DroneContext;
    data type Lineage;
    data type Technique;
    data type RenderedGesture;
    data type Evidence;

    let phrase       : List PhraseUnit → Phrase;
    let prayoga      : Phrase → List (Claim Evidence Prayoga);
    let admissible   : Raga × Lineage × Phrase → Verdict Evidence;
    let realize      : Raga × DroneContext × Lineage × Technique × Phrase
                    → Result RenderedGesture RealizationError;
    let transcribe   : NotationSystem × Phrase
                    → Approximation NotatedPhrase TranscriptionLoss;
}

data PhraseUnit = Svara SvaraIntent | Gamaka GamakaGesture | Bound SvaraIntent GamakaGesture;
```

The sum does not claim the three cases are an emic taxonomy. It is a falsification device: if a practice requires a
svara and gamaka to be inextricably bound, the third constructor records that rather than forcing a prior note plus an
ornament. A real package may make `Phrase` abstract and expose only practitioner-reviewed constructors.

For a fixed rāga/lineage, a static functor can seal validated phrases behind a fresh nominal type:

```text
template structure Repertoire(R : RagaPractice, r : R.Raga, l : R.Lineage) {
    data type AdmittedPhrase;              // constructor private
    let admit : R.Phrase → Result AdmittedPhrase R.Violation;
    let realize : R.Technique × AdmittedPhrase → Result R.RenderedGesture R.RealizationError;
}
```

This obtains the useful effect of a dependent family `Phrase(r,l)` without putting arbitrary musical values into type
conversion. The module instance carries the dependency statically; dynamic repertoire selection uses `Result`.

### Test result

**Conditional pass.** Abstract data, sums, evidence values, and generative modules express the relationships without a
global pitch or scale type. A `RenderedGesture` may include continuous technique and pitch trajectories and then be
placed on a performed timeline.

**Warning found by the sieve.** If `Timeline c A` were the only music-bearing type, this case would fail: phrase
vocabulary and admissibility are not temporal occurrence multisets. The total data language—not the timeline—is
load-bearing.

## 3. Arabic maqām as melodic pathways

The temptation is to reuse `WesternTonal.Scale` with fractional accidentals. That would preserve intervals while losing
the melodic vocabulary and modulation pathways emphasized by the source.

```text
signature MaqamPractice {
    data type Maqam;
    data type Jins;
    data type IntonationContext;
    data type MelodicMove;
    data type Sayr;
    data type Phrase;
    data type Modulation;

    let vocabulary  : Maqam × IntonationContext → PhraseGrammar;
    let path        : Maqam → Sayr;
    let interpret   : Sayr × List MelodicMove → Verdict PathEvidence;
    let modulation  : Sayr × PhraseRegion → List (Claim ModulationEvidence Modulation);
    let intone      : IntonationContext × Phrase → ContinuousPitchGesture;
}
```

`PhraseGrammar` is finite data describing a language or constraint system; it need not enumerate its potentially
unbounded utterances. A seeded improviser can be a causal `Process InteractionTick ContextEvents PhraseEvents` or a
finite generator consumed by a host. A recording remains a finite performed timeline or an asset.

### Test result

**Pass at the metalanguage level; domain content unchecked.** K₁ does not force the pathway into a scale. It also does
not supply a specialized grammar formalism. Strictly positive syntax plus folds can represent finite automata, grammars,
or transition graphs, but a real package may reveal that a generic constraint/grammar library deserves a deep module.
That is library evidence, not yet kernel evidence.

## 4. Ensemble-relative tuning in Balinese gamelan

A global `PitchClass` is actively misleading here because paired tuning and beating are part of a particular ensemble's
sound.

```text
signature GamelanEnsemble {
    data type Ensemble;
    data type KeyPosition;
    data type Instrument;
    data type Pairing;
    data type Stroke;
    data type Mode;
    data type TimbreModel;

    let pairing       : Ensemble × Instrument → Option Pairing;
    let frequency     : Ensemble × Instrument × KeyPosition → Hz;
    let beating       : Ensemble × Pairing × KeyPosition → Hz;
    let strike_model  : Ensemble × Instrument × KeyPosition × Stroke → TimbreModel;
    let processor     : ∀ r. Ensemble × Instrument → Process (Frames r) StrikeEvents StereoAudio;
}
```

The `KeyPosition` is an ensemble/instrument-relative label. `frequency` is a realization function, not a quotient from a
universal pitch. A static ensemble module can again export a fresh `Pitch` type whose representation contains only
positions valid for that ensemble.

Simultaneously striking the paired instruments is timeline overlay at the strike-event layer and processor parallel at
the audio layer. Those are two different operations joined by `processor`; K₁'s rejection of one monoidal syntax is
therefore observable here.

### Test result

**Pass.** Ensemble identity and beating survive into audio preparation naturally. A Western notation projection, if
requested, is an explicit lossy transcription and not the input carrier.

## 5. Hindustani tāl and long-form non-isochrony

The package should expose a cycle's vocabulary and gestures without claiming that its count determines it.

```text
signature TalPractice {
    clock Cycle;
    data type Tal;
    data type CyclePosition;
    data type Vibhag;
    data type Gesture;
    data type ThekaStroke;
    data type TempoPractice;

    let position    : Tal × CyclePosition → Rat;
    let grouping    : Tal → List Vibhag;
    let gesture     : Tal × CyclePosition → Option Gesture;
    let theka       : Tal → Timeline Cycle ThekaStroke;
    let perform     : TempoPractice × Tal → Warp Cycle Performed;
}
```

Two seven-cycle values may have the same rational positions and different gestures, patterns, group interpretations, or
practice. Equality of `Tal` is package equality, not equality of the resulting list of offsets. The timeline contains
one finite presentation of the cycle; it is not the definition of tāl.

### Test result

**Conditional pass.** The clock/warp separation handles non-isochronous performed groups while retaining a logical
cycle. K₁ still assumes that a chosen finite notated or analytical cycle can be coordinatized by rationals. If a
practice needs only qualitative order or variable-length interaction, it should use a process or a package-defined
constraint structure rather than inventing fake rational locations.

## 6. Puerto Rican bomba as a live causal exchange

This is the case that earns a causal process parameterized by a discrete tick rather than an audio-only `Process r`.

```text
signature BombaPractice {
    tick InteractionTick;
    data type DanceMovement;
    data type LeadDrumResponse;
    data type EnsembleCue;
    data type ExchangeState;

    let exchange : Process InteractionTick
        { dance : Events DanceMovement,
          ensemble : Events EnsembleCue }
        { lead_drum : Events LeadDrumResponse };
}
```

The encoding says only what the cited example licenses: dance events are inputs and lead-drum responses are outputs in
an interactive protocol. It does not claim this is a complete theory of bomba or reduce dance to control data.

An audio workbench may provide an explicit clock adapter:

```text
timestamp : Process DeviceTick (Events DanceMovement) (Events (Timestamp Seconds DanceMovement))
schedule  : InputPolicy × List (Timestamp Seconds A) → Timeline (Frames r) A
```

That adapter is a realization choice. The abstract exchange does not have to mention frames, equal temperament, score,
or meter.

### Test result

**Pass with a scope warning.** The finite process syntax can state the causal interface and its composition. K₁ cannot
define the culturally appropriate response function; that is exactly right. A source-only finite score cannot replace
this protocol, so the existing kernel remains one stage rather than the whole language.

## 7. Ametric notation, cue structure, and finite constraints

Some finite artifacts prescribe order or cues without exact duration.

```text
data CueGraph A = CueGraph {
    nodes : List (CueId × A),
    before : List (CueId × CueId),
    alternatives : List (ChoiceId × List CueId)
}
```

A validated constructor checks finite identity and acyclicity. A realization chooses an allowed path and only then
produces a `Timeline c A`. This handles the finite portion of mobile/ossia/cue cases without adding partial order to the
timeline denotation.

### Test result

**Provisional pass, with a pressure point.** Ordinary algebraic data can represent the structure, but every package
should not hand-roll graph validation, restriction, choice provenance, and realization. If the rāga/maqām grammar,
mobile form, and live protocol all need the same finite constraint engine, a deep `Presentation` or event-structure
module may earn admission above the value core. It has not yet earned primitive syntax.

## 8. Western score through performance to audio

This is the concrete joined artifact Musa needs now:

```text
structure Study {
    let material : Western.Material = ...;

    let score : Timeline WesternBeat Western.ScoreFact =
        Western.notate(material);

    let performance : Timeline Performed Western.Gesture =
        Western.interpret(profile, score)?;

    let seconds : Timeline Seconds Western.Gesture =
        warp(tempo_and_groove, performance);

    let studio : Process (Frames 48000) NoteEvents StereoAudio =
        serial(poly_sine, serial(reverb, limiter));

    let plan : PreparedSpec =
        prepare(bindings, seconds, studio, seed, options)?;
}
```

The source score, performance, and studio are not separate projects or untyped blobs. They are definitions in one sealed
module. The types prevent accidental substitution; the definition graph and Origin values preserve the chosen
relationship; R1 states what preparation may forget.

### Test result

**Pass.** This is simpler than first-class worlds and links. The artifact is coherent because the edges are checked and
named, not because every view is a projection of an unknowable universal carrier.

## 9. Analyses as evidence-bearing values

```text
data Claim Theory Target Proposition Evidence = Claim {
    theory      : Theory,
    target      : Target,
    proposition : Proposition,
    evidence    : Evidence,
    confidence  : Option Confidence,
    provenance  : Origin
}
```

A common-practice function analysis, set-class segmentation, rāga reading, motif label, or formal division instantiates
different nominal types. Two claims may conflict without making the underlying artifact ill typed. A theory package can
define stronger checked evidence when appropriate.

### Test result

**Pass.** This matches the present Musa analysis direction and prevents analytical vocabulary from becoming generic
normalization.

## 10. The five-case sieve

The rethink-math sieve asks the candidate to survive increasingly diagnostic examples.

### Case 1 — trivial: no placed material

```text
empty(3/2) : Timeline c A
```

**Result: pass.** Ambient time can exist without a rest, note, key, or meter. The type requires only a clock and payload
data type. This is useful in every timeline domain and does not assert musical silence in an absolute sense.

### Case 2 — simplest nontrivial: one fact and one total definition

```text
place(1,0,1,a) : Result (Timeline c A) BoundsError
map(f, place(...)) : Result (Timeline c B) BoundsError
```

**Result: pass.** Bounds partiality is explicit. Payload mapping preserves spans. No musical structure is inferred.

### Case 3 — compositional: unequal entrances and an audio tail

Let `M` have extent `4`, `N` enter at `2` and end at `3`, and a reverb process continue statefully after its last note
event.

**Result: pass.** `over(M,N)` has extent `4`; no rest or padding payload is inserted. The reverb has no timeline extent
at all: its decay is process state rendered until an explicit offline stopping policy. This directly refutes the
equal-extent overlay and “audio is another timeline” proposals.

### Case 4 — indexed: incompatible audio interfaces

```text
gain   : Process (Frames r) Mono Mono
pan    : Process (Frames r) Mono Stereo
reverb : Process (Frames r) Stereo Stereo
```

`serial(gain,reverb)` is rejected; `serial(gain,serial(pan,reverb))` is admitted. A process at `Frames 44100` cannot
connect to one at `Frames 48000` without an explicit rate adapter.

**Result: pass.** Restricted indices remove a real class of graph errors. No musical plan or rational expression enters
the solver.

### Case 5 — hardest combined: oral vocabulary, live choice, and audio

Take a finite phrase vocabulary learned in a named practice, validate a phrase relative to context, let a performer
select and shape it during an interaction, then realize it through an ensemble-specific instrument model.

```text
PhraseGrammar
  → Process InteractionTick ContextEvents PhraseEvents
  → timestamp / interpret
  → Timeline Seconds ContinuousGesture
  → Process (Frames r) GestureEvents StereoAudio
```

**Result: partial pass.** K₁ keeps each domain type distinct and can type each explicit bridge. It does not yet have a
generic account of a process crossing from `InteractionTick` to `Frames r`; `timestamp` and `schedule` currently form a
two-stage adapter outside process composition. That may be correct—the boundary includes buffering, latency, and
quantization policy—but it must be specified. The candidate also has no proved account of a generative grammar whose
possible runs are unbounded while its description is finite.

This is the first genuine pressure point, not a reason to add dependent types. The likely missing object is a typed
event/trace interface with explicit clock adaptation, not a universal pitch or metrical index.

## 11. Furniture audit

| Feature | Two independent callers? | Removes side conditions? | Verdict |
| --- | --- | --- | --- |
| Total functions and finite data | every theory package; elaboration | yes | house |
| Abstract nominal data types | incompatible pitch/time/practice theories | yes | house |
| Rank-1 polymorphism | generic collections; timeline/process combinators | yes | likely house |
| Static structure functors | repertoire sealing; ensemble-specific types | yes | house |
| Timeline | notation; gesture/control plans | yes | house, already implemented |
| Warp | tempo/groove/rubato; performed-to-seconds | yes | house, exact fragment only |
| Causal process | audio graphs; live interaction | yes | house if live scope is accepted |
| Natural/port indices | vectors; audio interfaces | yes | small house |
| Full dependent types | none found | no | furniture |
| CBPV | none beyond staging already expressed | no | furniture |
| First-class worlds | none | no | furniture |
| General theta-link/profunctor | no current example needs generic two-sided action | no | furniture |
| One monoidal syntax | conflates native laws | adds side conditions | rejected |
| Generic finite constraint/event structure | mobile form; grammars; perhaps live traces | maybe | open |
| Cross-clock process adapter | live input to audio; score following | yes | open, next definition problem |

## 12. What K₁ still over-assumes

Even after the cross-tradition probes, K₁ retains assumptions that must be stated honestly:

1. Every source value is finitely presentable and source evaluation terminates.
2. Every timeline observation uses a linear rational coordinate and finite occurrences.
3. Every live process is observed through a discrete tick and is causal in prefix order.
4. Every abstract data type has decidable equality and canonical serialization.
5. Every cross-domain relationship used by source evaluation can be oriented as a total function returning explicit
   failure, alternatives, approximation, or evidence.

These assumptions define Musa's tractable subset; they are not definitions of music. A practice outside them can still
appear as an opaque asset plus metadata, but it will not receive the same equational reasoning. If a desired Musa use
case violates one, the kernel must be extended or its scope narrowed explicitly.

## 13. Current verdict

K₁ survives better than the motive, CBPV, indexed-meter, torsor, or one-SMC candidates. Its strongest evidence is
negative: it lets Western tonal theory be concise without asking Karnatak, Arabic, gamelan, tāl, or bomba examples to
instantiate Western fields.

It is not ready for governance. Before that:

1. define the event/trace and cross-clock boundary from Case 5;
2. prove the fixed metatheory and warp/process laws;
3. review the proof independently;
4. prototype the types against current Rust values; and
5. get practitioner review before claiming any non-Western package is adequate.

The concrete near-term decision remains stable: do **not** index the temporal kernel by exact extent and do **not**
require equal extents for overlay. The existing maximum-extent semantics is both simpler and more musically honest.
