# Theory-module paper prototypes

**Status: source-design probes, not musical package specifications. Governs nothing.** Candidate T₂
([31](31-candidate-theory-modules.md)) proposes nominal first-order data and abstract signature members as the one
plausible major source-language extension. This note writes the two paper prototypes its recommendation requires.

The second prototype is a **Karnatak pressure test**, not a definition of Karnatak music. It encodes only distinctions
supported by the limited sources in [19](19-domain-obligations.md): svara intent; gamaka as load-bearing gesture rather
than decoration; ascent/descent and characteristic phrase constraints; tonic/drone and lineage context; oral knowledge
not exhausted by notation. Names, constructors, and judgments require practitioner review before any library proposal.

The acceptance question is language-theoretic:

> Can both packages state their own carriers and operations without a universal `Pitch`, fake fields, text tags, full
> dependent types, or different core rules?

---

## 1. Prototype language subset

The prototype intentionally uses less than Candidate T₂ originally sketched:

```text
data D { C₁(F̄₁), …, Cₙ(F̄ₙ) }
data type T;                    // abstract signature member
data type T = D;                // private realization
```

Fields are monomorphic finite products of existing scalar, `Option`, `List`, and earlier nominal data types. There are
no user-generic or user-recursive data declarations in the first extension. This matters: `data Claim P E` and a general
strictly-positive fixpoint are convenient, but no current caller proves that either is needed. Existing compiler-owned
`List` supplies finite recursive structure; each theory can initially define its own non-recursive evidence/claim
carrier.

Constructors are private unless the signature exposes a manifest `data` declaration. Abstract members are referred to by
paths such as `P.Intent`. Structure templates are static and generative exactly as the current module design intends.

The examples use `Result A E` as readable **paper notation**, not as an admitted T₂a type constructor. The current core
has `Option` but no `Result`, and T₂a deliberately adds no generic user data. An exact first prototype expands each
materially different result to a monomorphic nominal sum, for example:

```musa
data ChordBuild {
    Built(pitches: List WrittenPitch),
    BuildFailed(error: Error),
}
```

That expansion is semantic, not an erasing index. If repeated monomorphic outcome declarations make the two package
spikes materially worse, a compiler-owned `Result` may earn a separate conservative-extension proof. The paper spelling
must not be mistaken for that proof already existing.

## 2. Prototype W — a sealed common-practice launch theory

This package is intentionally familiar because it must demonstrate that removing kernel privilege does not make Musa's
launch repertoire painful.

```musa
signature TonalTheory {
    data type NoteName;
    data type WrittenPitch;
    data type Interval;
    data type PitchClass;
    data type Scale;
    data type Key;
    data type Degree;
    data type ChordRecipe;
    data type Voicing;
    data type FunctionClaim;
    data type Evidence;
    data type Error;

    let transpose: Interval -> WrittenPitch -> Option WrittenPitch;
    let forget_spelling: WrittenPitch -> PitchClass;
    let degree: Key -> WrittenPitch -> Option Degree;
    let build_chord: Key -> ChordRecipe -> Voicing
        -> Result (List WrittenPitch) Error;
    let analyze_function: Key -> List WrittenPitch
        -> List FunctionClaim;
    let evidence: FunctionClaim -> Evidence;
}
```

The curried spelling is current Musa surface style; core instantiation remains monomorphic.

One implementation may privately reuse current compiler representations:

```musa
structure CommonPractice12 : TonalTheory {
    data type NoteName = private ExistingNoteName;
    data type WrittenPitch = private ExistingWrittenPitch;
    data type Interval = private ExistingInterval;
    data type PitchClass = private ExistingPc12;
    data type Scale = private ExistingScale;
    data type Key = private ExistingKey;
    data type Degree = private ExistingDegree;

    data ChordRecipeRep {
        StackedThirds(n: Nat),
        AddedTone(base: Nat, added: Degree),
        Suspension(replace: Degree, with: Degree),
    }
    data type ChordRecipe = private ChordRecipeRep;

    data VoicingRep {
        Close(register: Nat),
        Drop2(register: Nat),
        Explicit(pitches: List WrittenPitch),
    }
    data type Voicing = private VoicingRep;

    data FunctionClaimRep {
        Tonic(target: List WrittenPitch),
        Predominant(target: List WrittenPitch),
        Dominant(target: List WrittenPitch),
        Contextual(target: List WrittenPitch, label: Text),
    }
    data type FunctionClaim = private FunctionClaimRep;

    data EvidenceRep {
        Rule(rule: Text, observations: List WrittenPitch),
    }
    data type Evidence = private EvidenceRep;

    data ErrorRep {
        OutsideScale,
        UnrepresentablePitch,
        InvalidVoicing,
    }
    data type Error = private ErrorRep;

    let transpose = ...;
    let forget_spelling = ...;
    let degree = ...;
    let build_chord = ...;
    let analyze_function = ...;
    let evidence = ...;
}
```

### 2.1 What the types prevent

- `PitchClass` cannot be passed where `WrittenPitch` is required.
- `Degree` requires a `Key` argument to be located; it is not a global pitch coordinate.
- chord construction and harmonic-function analysis are separate operations.
- tonic, predominant, and dominant are analysis results, not orbits under tonic translation.
- a Neapolitan or contextual sonority can carry a theory-owned claim rather than being rounded to its scale degree.
- the package can later expose a second implementation without making either representation the language's pitch
  universe.

### 2.2 What does and does not fall out

Given a package-defined total interval action, structural list traversal derives transposition of a pitch collection.
Given a finite voiced pitch list, temporal overlay can place its members simultaneously. Neither construction derives
the classification “chord,” its spelling, voicing law, or harmonic function. Those stay in `build_chord` and
`analyze_function` because that is where the musical theory lives.

The prototype is not substantially more verbose for a user if the standard package is imported by default or through a
short prelude. Its extra text is paid once by the library owner, not on every note.

## 3. Prototype K — phrase/gesture-primary pressure test

The public signature does not mention `Pitch`, `PitchClass`, `Scale`, `Key`, `Chord`, `Meter`, or Western notation:

```musa
signature PhraseGesturePractice {
    data type Practice;
    data type Lineage;
    data type Context;
    data type SvaraIntent;
    data type GamakaIntent;
    data type Phrase;
    data type Gesture;
    data type AdmissibilityEvidence;
    data type Transcription;
    data type TranscriptionLoss;
    data type Error;

    let context: Practice -> Lineage -> Result Context Error;
    let svara: Context -> Text -> Result SvaraIntent Error;
    let bind: Context -> SvaraIntent -> GamakaIntent
        -> Result Phrase Error;
    let continue: Phrase -> Phrase -> Result Phrase Error;
    let admit: Context -> Phrase
        -> Result AdmissibilityEvidence Error;
    let realize: Context -> Phrase -> Result Gesture Error;
    let transcribe: Context -> Phrase
        -> Result Transcription TranscriptionLoss;
}
```

This signature already exposes a useful negative result: `GamakaIntent` is not an `Ornament` applied to a completed
note. The only public phrase constructor shown binds it with `SvaraIntent` under a context. A real package could export
other constructors after review without changing the metalanguage.

A schematic private realization might be:

```musa
structure KarnatakProbe : PhraseGesturePractice {
    data PracticeRep {
        NamedPractice(name: Text),
    }
    data type Practice = private PracticeRep;

    data LineageRep {
        School(name: Text),
        TeacherChain(names: List Text),
    }
    data type Lineage = private LineageRep;

    data ContextRep {
        ContextValue(
            practice: Practice,
            lineage: Lineage,
            tonic: Ratio,
            drone: List Ratio,
            phrase_vocabulary: List Text,
        ),
    }
    data type Context = private ContextRep;

    data SvaraIntentRep {
        Svara(label: Text, region: Option Ratio),
    }
    data type SvaraIntent = private SvaraIntentRep;

    data GamakaIntentRep {
        LearnedGesture(name: Text),
        ContextualGesture(name: Text, contour_hint: List Ratio),
    }
    data type GamakaIntent = private GamakaIntentRep;

    data BoundUnit {
        Bound(svara: SvaraIntent, gamaka: GamakaIntent),
    }

    data PhraseRep {
        Unit(value: BoundUnit),
        Path(units: List BoundUnit, direction_changes: List Nat),
    }
    data type Phrase = private PhraseRep;

    data GestureRep {
        GestureValue(
            pitch_trajectory: List Ratio,
            technique_trajectory: List Ratio,
            articulation_cues: List Text,
        ),
    }
    data type Gesture = private GestureRep;

    data AdmissibilityEvidenceRep {
        CharacteristicUse(name: Text, matched_region: List Nat),
        DirectionalUse(ascent: Bool, matched_region: List Nat),
        LineageRule(name: Text, matched_region: List Nat),
    }
    data type AdmissibilityEvidence = private AdmissibilityEvidenceRep;

    data TranscriptionRep {
        ApproximateSymbols(symbols: List Text),
    }
    data type Transcription = private TranscriptionRep;

    data TranscriptionLossRep {
        LostTrajectory(detail: Text),
        LostTiming(detail: Text),
        OralKnowledgeRequired(detail: Text),
    }
    data type TranscriptionLoss = private TranscriptionLossRep;

    data ErrorRep {
        UnknownVocabulary,
        ContextViolation(detail: Text),
        UnrealizedTechnique(detail: Text),
    }
    data type Error = private ErrorRep;

    let context = ...;
    let svara = ...;
    let bind = ...;
    let continue = ...;
    let admit = ...;
    let realize = ...;
    let transcribe = ...;
}
```

The numerical trajectory fields are placeholders for a finite exact gesture presentation, not a claim that sampled
ratios define gamaka. A production package might keep the gesture wholly abstract or use richer curve data. The public
interface is the design result; the private sketch exists only to prove the proposed data grammar can house it.

### 3.1 A generative admitted repertoire

```musa
signature AdmittedRepertoire {
    data type Phrase;
    data type Gesture;
    data type Admitted;
    data type Evidence;
    data type Error;

    let admit: Phrase -> Result Admitted Evidence;
    let realize: Admitted -> Result Gesture Error;
}

template structure InContext(
    P: PhraseGesturePractice,
    practice: P.Practice,
    lineage: P.Lineage,
): AdmittedRepertoire {
    data type Phrase = private P.Phrase;
    data type Gesture = private P.Gesture;
    data AdmittedRep { Witness(P.Phrase, P.AdmissibilityEvidence) }
    data type Admitted = private AdmittedRep;
    data type Evidence = private P.AdmissibilityEvidence;
    data type Error = private P.Error;

    let admit = ... P.context(practice)(lineage) ... P.admit ...;
    let realize = ... P.realize ...;
}
```

Each named `make` site yields a fresh `Admitted` type. Material admitted under one practice/lineage context cannot be
silently used under another. This achieves the relevant static separation without type conversion evaluating `practice`
or `lineage` values. The witness remains finite runtime data and can be serialized for provenance.

This prototype finds a syntax issue Candidate T₂ must solve: `data type Phrase = private P.Phrase` is an alias, not a
new representation. The language needs two different spellings:

```text
type Phrase = P.Phrase;            // transparent alias
data type Admitted = AdmittedRep;  // sealed nominal member
```

Calling both `data type` would make equality and generativity unclear.

## 4. Connecting either theory to time and audio

Neither theory carrier is itself a timeline. A named pass places or interprets its finite values:

```text
notate_W:
    CommonPractice12.Material
    -> Result (Derived Material (Timeline WesternBeat ScoreFact)) NotationError

perform_W:
    Timeline WesternBeat ScoreFact
    -> Result (Derived Score GestureTimeline) PerformanceError

perform_K:
    KarnatakProbe.Context -> KarnatakProbe.Phrase
    -> Result (Derived Phrase GesturePresentation) KarnatakProbe.Error

place_K:
    ConductedPlan -> GesturePresentation
    -> Result (Derived GesturePresentation (Timeline Performed Gesture)) TimingError
```

The `Derived` wrapper supplies lineage and losses. The K pressure test does not need to pass through Western score
notation before reaching a performed gesture. A transcription is another optional lossy branch of the artifact diagram.

An instrument structure then accepts the relevant gesture contract and hides its process graph. No theory package sees
DSP node ids or sample buffers.

## 5. Findings from writing the prototypes

### 5.1 Features which earned their place

- **Nominal user data:** both packages need private carriers the compiler registry should not own.
- **Abstract data members:** without them, a generic repertoire or instrument signature leaks representations or uses
  text tags.
- **Private constructors and exhaustive match inside the owner:** ordinary information hiding, not dependent typing.
- **Path-dependent member names:** `P.Phrase` and `P.Gesture` prevent theory mixing.
- **Static generative structure templates:** an admitted-context type is a real caller for fresh nominal identity.
- **Evidence and explicit loss values:** analysis/admission and transcription are not booleans or comments.

### 5.2 Features which did not earn their place

- user-generic ADTs;
- full dependent types or equality proofs;
- implicit type-class/structure search;
- subtyping between theory carriers;
- global `Pitch`, `Scale`, `Key`, `Phrase`, or `Gesture` interfaces;
- first-class worlds or theta-links; and
- a universal monoidal operation for chord, phrase, timeline, and processor construction.

### 5.3 Specification debts exposed

1. Distinguish transparent `type` aliases from sealed `data type` realization.
2. Decide whether a signature may expose constructors or only abstract members in the first release.
3. Define stable nominal identities across locked packages and generative instances.
4. Define how private constructor encodings remain canonical and versioned.
5. Prove template parameter substitution, signature matching, sealing, and generativity preserve core typing.
6. Decide how diagnostics display long paths such as `R.Practice.Intent` without making the language feel like a proof
   assistant.

## 6. Verdict

The prototypes support the central source-language hypothesis: **first-order nominal data plus abstract type members is
enough to state materially different theory shapes without global Western carriers.** The non-Western pressure case did
not force dependent types, a universal pitch, or a different temporal kernel.

This is not yet enough to schedule implementation. The module elaboration proof is missing, and the Karnatak-facing
public vocabulary has not received practitioner review. The next research artifact should specify the exact source
grammar and elaboration of the six earned features, prove preservation into K₃ᴱ, and test a prototype parser/checker on
these two files. Only after that should governing language docs and prompts change.
