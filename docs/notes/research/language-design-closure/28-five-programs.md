# Five complete programs in the active language

## Purpose

This note asks one question:

> Can the language chosen in note 26 express the five musical cases without hidden context or a special-purpose core
> feature?

The answer is yes. The programs need ordinary inferred functions, package-owned data, hidden constructors, exhaustive
matches, finite folds, records, and explicit `Result` values. They do not need partial calls, dependent types, effects,
general recursion, or type-directed macros.

These are **paper Musa** programs. They contain no omitted branches or placeholder bodies, but the current compiler does
not yet accept the proposed syntax. The musical sources and limits remain those recorded in
[the five case studies](02-five-musical-cases.md). The culturally named cases remain pressure tests, not package
designs.

## 1. Shared standard operations

The examples import, rather than redefine, these standard operations:

```text
fold_list: List<A> × B × (A × B -> B) -> B
map_list: List<A> × (A -> B) -> List<B>
append_list: List<A> × List<A> -> List<A>
map_result: Result<A, E> × (A -> B) -> Result<B, E>
and_then: Result<A, E> × (A -> Result<B, E>) -> Result<B, E>
traverse_result: List<A> × (A -> Result<B, E>) -> Result<List<B>, E>
```

Each is implemented by a finite fold. Calls are complete and positional. A function value is always written with `fn` or
named by a complete function declaration.

The notation and stage packages expose these abstract types and total operations:

```text
StaffDocument
Music
MusicalContext
Term<ScoreFact>
Timeline<ScoreFact>
NotationPlan
GestureIntent
GestureTimeline
CheckedStudio
PreparedExecution

staff.to_music: StaffDocument -> Music
staff.sequence: List<StaffDocument> -> StaffDocument
staff.overlay: List<StaffDocument> -> StaffDocument
close_music: Music × MusicalContext -> Result<Term<ScoreFact>, MusicError>
evaluate_temporal: Term<ScoreFact> -> Timeline<ScoreFact>
engrave: Timeline<ScoreFact> × NotationOptions -> Result<NotationPlan, NotationError>
interpret: List<GestureIntent> × PerformanceContext -> Result<GestureTimeline, PerformanceError>
prepare:
  GestureTimeline
  × CheckedStudio
  × InstrumentBindings
  × Seed
  × PrepareOptions
  -> Result<PreparedExecution, PrepareError>
```

These signatures are stage boundaries, not primitive source syntax. Every context, binding, seed, and option is an
explicit argument.

## 2. Tonal harmony

### 2.1 Package source

This package keeps spelling, chord construction, scale degree, harmonic function, voicing, notation, and analysis
separate.

```musa
// trial/tonal.musa
import std.list as list
import std.notation.staff as staff

pub type Spelling:
  C
  CSharp
  DFlat
  D
  EFlat
  E
  F
  FSharp
  G
  AFlat
  A
  BFlat
  B

pub type ChordSymbol:
  CMajor
  CMinor
  DFlatMajor
  FMajor
  GSeven
  AMinor
  BDiminished

pub type VoicingPolicy:
  Close
  OpenBass

pub type Key:
  CMajorKey
  CMinorKey

pub type ScaleDegree:
  One
  Four
  Five
  Six
  Seven
  FlatTwo

pub type HarmonicFunction:
  Tonic
  Predominant
  Dominant
  Other

pub record FunctionClaim:
  key: Key
  symbol: ChordSymbol
  degree: ScaleDegree
  function: HarmonicFunction
  evidence: Text

pub type TonalError:
  CannotAnalyzeAnonymousChord

pub type Chord:
  private NamedChord(ChordSymbol, List<Spelling>)
  private AnonymousChord(List<Spelling>)

pub type Voicing:
  private VoicingValue(List<staff.WrittenPitch>)

fn tones(symbol):
  match symbol:
    CMajor -> [C, E, G]
    CMinor -> [C, EFlat, G]
    DFlatMajor -> [DFlat, F, AFlat]
    FMajor -> [F, A, C]
    GSeven -> [G, B, D, F]
    AMinor -> [A, C, E]
    BDiminished -> [B, D, F]

fn raise_semitone(spelling):
  match spelling:
    C -> CSharp
    CSharp -> D
    DFlat -> D
    D -> EFlat
    EFlat -> E
    E -> F
    F -> FSharp
    FSharp -> G
    G -> AFlat
    AFlat -> A
    A -> BFlat
    BFlat -> B
    B -> C

fn written_pitch(spelling, register):
  match spelling:
    C -> staff.pitch("c", "natural", register)
    CSharp -> staff.pitch("c", "sharp", register)
    DFlat -> staff.pitch("d", "flat", register)
    D -> staff.pitch("d", "natural", register)
    EFlat -> staff.pitch("e", "flat", register)
    E -> staff.pitch("e", "natural", register)
    F -> staff.pitch("f", "natural", register)
    FSharp -> staff.pitch("f", "sharp", register)
    G -> staff.pitch("g", "natural", register)
    AFlat -> staff.pitch("a", "flat", register)
    A -> staff.pitch("a", "natural", register)
    BFlat -> staff.pitch("b", "flat", register)
    B -> staff.pitch("b", "natural", register)

pub fn build(symbol):
  NamedChord(symbol, tones(symbol))

pub fn transpose_up_semitone(chord):
  match chord:
    NamedChord(symbol, members) ->
      AnonymousChord(list.map(members, fn(member): raise_semitone(member)))
    AnonymousChord(members) ->
      AnonymousChord(list.map(members, fn(member): raise_semitone(member)))

pub fn voice(policy, chord):
  let members = match chord:
    NamedChord(symbol, named_members) -> named_members
    AnonymousChord(anonymous_members) -> anonymous_members

  match policy:
    Close ->
      VoicingValue(list.map(members, fn(member): written_pitch(member, 4)))
    OpenBass ->
      match members:
        [] -> VoicingValue([])
        bass :: upper ->
          VoicingValue(
            [written_pitch(bass, 3)]
            ++ list.map(upper, fn(member): written_pitch(member, 4))
          )

fn degree(key, symbol):
  match key:
    CMinorKey ->
      match symbol:
        CMajor -> One
        CMinor -> One
        DFlatMajor -> FlatTwo
        FMajor -> Four
        GSeven -> Five
        AMinor -> Six
        BDiminished -> Seven
    CMajorKey ->
      match symbol:
        CMajor -> One
        CMinor -> One
        DFlatMajor -> FlatTwo
        FMajor -> Four
        GSeven -> Five
        AMinor -> Six
        BDiminished -> Seven

fn function_of(symbol, after):
  match symbol:
    CMajor -> Tonic
    CMinor -> Tonic
    DFlatMajor ->
      match after:
        Some(GSeven) -> Predominant
        Some(next) -> Other
        None -> Other
    FMajor -> Predominant
    GSeven -> Dominant
    AMinor -> Tonic
    BDiminished -> Dominant

fn evidence(function):
  match function:
    Tonic -> "stable goal under this analysis"
    Predominant -> "moves toward the dominant under this analysis"
    Dominant -> "moves toward the tonic under this analysis"
    Other -> "this method assigns no function"

pub fn analyze(key, before, chord, after):
  match chord:
    AnonymousChord(members) -> Err(CannotAnalyzeAnonymousChord)
    NamedChord(symbol, members) ->
      let assigned = function_of(symbol, after)
      Ok(FunctionClaim:
        key = key
        symbol = symbol
        degree = degree(key, symbol)
        function = assigned
        evidence = evidence(assigned)
      )

pub fn write(voicing, duration):
  match voicing:
    VoicingValue(pitches) -> staff.chord(pitches, duration)
```

`before`, `symbol`, and `members` are intentionally unused in small branches where the analysis does not need them. The
lint can report that fact without changing the semantics.

### 2.2 Client source

```musa
// pieces/neapolitan.musa
import trial.tonal as tonal
import std.notation.staff as staff

let neapolitan = tonal.build(tonal.DFlatMajor)
let dominant = tonal.build(tonal.GSeven)
let tonic = tonal.build(tonal.CMinor)

let claim = tonal.analyze(
  tonal.CMinorKey,
  None,
  neapolitan,
  Some(tonal.GSeven),
)

let page = staff.sequence([
  tonal.write(tonal.voice(tonal.OpenBass, neapolitan), staff.half),
  tonal.write(tonal.voice(tonal.Close, dominant), staff.half),
  tonal.write(tonal.voice(tonal.OpenBass, tonic), staff.whole),
])

let score = staff.to_music(page)
```

No adapter region appears, so expansion is the identity: the resolved program receives these ordinary expressions
unchanged.

### 2.3 Inference and evaluation

The compiler infers:

```text
build: ChordSymbol -> Chord
transpose_up_semitone: Chord -> Chord
voice: VoicingPolicy × Chord -> Voicing
analyze: Key × Option<ChordSymbol> × Chord × Option<ChordSymbol>
         -> Result<FunctionClaim, TonalError>
write: Voicing × WrittenDuration -> StaffDocument
claim: Result<FunctionClaim, TonalError>
page: StaffDocument
score: Music
```

The key result evaluates to:

```text
Ok(FunctionClaim {
  key = CMinorKey,
  symbol = DFlatMajor,
  degree = FlatTwo,
  function = Predominant,
  evidence = "moves toward the dominant under this analysis"
})
```

`FlatTwo` and `Predominant` are different fields because they answer different questions. Transposing the chord creates
an anonymous chord and makes this named-symbol analysis return `CannotAnalyzeAnonymousChord`; the type checker does not
invent a function from pitch content.

### 2.4 The two routes

The notation-led route is:

```text
Chord -> Voicing -> StaffDocument -> Music -> Term<ScoreFact>
      -> Timeline<ScoreFact> -> NotationPlan
```

Voicing adds register, order, spacing, and doubling. MIDI may later lose spelling. The analysis route instead produces
`FunctionClaim` and evidence. It does not pass through notation.

The performance-led route is:

```text
Voicing -> written-and-sounding pitch intent -> GestureIntent
        -> GestureTimeline -> PreparedExecution -> audio history
```

An instrument profile adds articulation, tuning, and timing. The score does not contain decibels or samples.

### 2.5 Count

| Measure | Count |
| --- | ---: |
| Required source type annotations | 0 |
| Compiler-owned musical surface forms | 0 |
| Adapter regions | 0 |
| Explicit stage contexts | 4: musical, performance, bindings, preparation |
| Locally reportable errors | anonymous chord analysis; notation failure; closure context failure |

The tonal package owns every tonal concept. The language owns only the means to define and hide the data.

## 3. Flexible time

### 3.1 Source

The page states written facts. A separate value states performance choices.

```musa
// pieces/flexible-time.musa
import syntax std.notation.staff as staff
import std.notation.staff as staff_value
import std.performance.time as timing

let melody = staff:
  instrument flute
  clef treble
  meter 4/4

  | c5/8 d5/8 e5/8 f5/8 g5/8 a5/8 b5/8 c6/8
  named held:
    | g5/4 rest/4 g5/2
  named feathered:
    feather accelerate:
      a5
      b5
      c6
      d6
  named cadenza:
    unmeasured:
      e6
      d6
      b5
      c6

let ostinato = staff:
  instrument hand_drum
  meter 3/4
  | c3/4 rest/4 g3/4

let page = staff_value.overlay([melody, ostinato])

let performance = timing.plan([
  timing.swing(staff_value.region(melody, "bar-1"), 2/1),
  timing.open_fermata(staff_value.region(melody, "held")),
  timing.metric_modulation(staff_value.eighth, staff_value.quarter),
  timing.linear_acceleration(
    staff_value.region(melody, "feathered"),
    timing.beats_per_minute(staff_value.quarter, 72),
    timing.beats_per_minute(staff_value.quarter, 144),
  ),
  timing.performer_timed(staff_value.region(melody, "cadenza")),
])

let score = staff_value.to_music(page)
```

### 3.2 Expansion

The two adapter regions expand before type inference. The first expands to:

```musa
std.notation.staff.make_document(
  InstrumentName("flute"),
  IdentityTransposition,
  Treble,
  NoWrittenKey,
  Meter(4, 4),
  RejectUnspelledExactDuration,
  [
    Bar(f0, 1, [
      Note(f1, C(5), NoteValue(8, 0), [], NoTie),
      Note(f2, D(5), NoteValue(8, 0), [], NoTie),
      Note(f3, E(5), NoteValue(8, 0), [], NoTie),
      Note(f4, F(5), NoteValue(8, 0), [], NoTie),
      Note(f5, G(5), NoteValue(8, 0), [], NoTie),
      Note(f6, A(5), NoteValue(8, 0), [], NoTie),
      Note(f7, B(5), NoteValue(8, 0), [], NoTie),
      Note(f8, C(6), NoteValue(8, 0), [], NoTie),
    ]),
    Named(f9, "held", [
      Bar(f10, 1, [
        Note(f11, G(5), NoteValue(4, 0), [], NoTie),
        Rest(f12, NoteValue(4, 0)),
        Note(f13, G(5), NoteValue(2, 0), [], NoTie),
      ]),
    ]),
    Named(f14, "feathered", [
      Feather(
        f15,
        Accelerate,
        [
          UnmeasuredPitch(f16, A(5), []),
          UnmeasuredPitch(f17, B(5), []),
          UnmeasuredPitch(f18, C(6), []),
          UnmeasuredPitch(f19, D(6), []),
        ],
      ),
    ]),
    Named(f20, "cadenza", [
      Unmeasured(f21, [
        UnmeasuredPitch(f22, E(6), []),
        UnmeasuredPitch(f23, D(6), []),
        UnmeasuredPitch(f24, B(5), []),
        UnmeasuredPitch(f25, C(6), []),
      ]),
    ]),
  ],
)
```

The second expands to:

```musa
std.notation.staff.make_document(
  InstrumentName("hand_drum"),
  IdentityTransposition,
  PercussionClef,
  NoWrittenKey,
  Meter(3, 4),
  RejectUnspelledExactDuration,
  [
    Bar(g0, 3/4, [
      Note(g1, C(3), NoteValue(4, 0), [], NoTie),
      Rest(g2, NoteValue(4, 0)),
      Note(g3, G(3), NoteValue(4, 0), [], NoTie),
    ]),
  ],
)
```

`f0` through `f25` and `g0` through `g3` are the exact source anchors assigned from left to right. The expansion omits
no staff value or source anchor.

### 3.3 Inference and evaluation

The compiler infers:

```text
melody: StaffDocument
ostinato: StaffDocument
page: StaffDocument
performance: TimingPlan
score: Music
```

Evaluating `page` produces two finite staff layers. The melody has unresolved timing requests in `feathered` and
`cadenza`; `score` is therefore a total recipe, not yet a closed timeline. Applying a `MusicalContext` that lacks those
choices returns `MissingTimingChoice`. Applying a context with finite durations returns one finite temporal term.

Overlay keeps the ostinato's length at `3/4`. It does not append a rest to make it as long as the melody.

### 3.4 The two routes

The notation-led route preserves equal written eighths, the fermata, metric equation, feathered beam, unmeasured group,
and polymeter. It need not choose performed onsets.

The performance-led route combines `score`, `performance`, and a finite set of performer choices. It adds swing ratios,
fermata length, acceleration onsets, cadenza timing, seconds, and rounded frame positions. These choices are recorded in
the derivation. A meter analysis is another named result; it is not the timeline's type.

### 3.5 Count

| Measure | Count |
| --- | ---: |
| Required source type annotations | 0 |
| Compiler-owned musical surface forms | 0 |
| Adapter regions | 2 staff blocks |
| Explicit context that hidden state would otherwise supply | 5 timing choices |
| Locally reportable errors | bar length; malformed meter; missing named region; missing closure choice |

The timing package owns swing, fermatas, metric modulation, and performance maps. The kernel continues to own only exact
finite placement after all required choices are supplied.

## 4. Phrase-led intent

### 4.1 Package source

This is a pressure test built from the distinctions in the earlier case study. It is not a claim that this finite model
captures Karnatak music.

```musa
// trial/phrase_led.musa
import std.list as list
import std.notation.staff as staff

pub type Svara:
  Sa
  Ri
  Ga
  Ma
  Pa
  Da
  Ni

pub type Direction:
  Rising
  Falling
  Turning

pub type GestureKind:
  Plain
  Oscillate
  SlideFromBelow

pub record PhraseToken:
  svara: Svara
  direction: Direction
  gesture: GestureKind
  weight: Ratio

pub record PhraseContext:
  tonic_hz: Ratio
  register_ratio: Ratio
  oscillation_width: Ratio

pub type PhraseError:
  EmptyPhrase
  NonPositiveWeight
  ArithmeticFailure

pub type TranscriptionLoss:
  CurveReducedToMark(Svara)
  DirectionKeptAsText(Direction)
  TonicReducedToStaffSpelling(Ratio)

pub type PhraseGesture:
  Hold(Svara, Ratio, Ratio)
  OscillateAround(Svara, Ratio, Ratio, Ratio)
  ApproachFromBelow(Svara, Ratio, Ratio)

pub type Phrase:
  private PhraseValue(List<PhraseToken>)

fn positive(token):
  ratio_greater(token.weight, 0/1)

pub fn make(tokens):
  match tokens:
    [] -> Err(EmptyPhrase)
    head :: tail ->
      let all_positive = list.fold(
        tokens,
        true,
        fn(token, state): state && positive(token),
      )
      if all_positive:
        Ok(PhraseValue(tokens))
      else:
        Err(NonPositiveWeight)

fn svara_ratio(svara):
  match svara:
    Sa -> 1/1
    Ri -> 9/8
    Ga -> 6/5
    Ma -> 4/3
    Pa -> 3/2
    Da -> 8/5
    Ni -> 9/5

fn center_hz(context, svara):
  let registered = checked_ratio_multiply(
    context.tonic_hz,
    context.register_ratio,
  )
  and_then(
    registered,
    fn(base): checked_ratio_multiply(base, svara_ratio(svara)),
  )

fn gesture_for(context, token):
  match center_hz(context, token.svara):
    Err(error) -> Err(ArithmeticFailure)
    Ok(center) ->
      match token.gesture:
        Plain -> Ok(Hold(token.svara, center, token.weight))
        Oscillate ->
          Ok(OscillateAround(
            token.svara,
            center,
            token.weight,
            context.oscillation_width,
          ))
        SlideFromBelow ->
          Ok(ApproachFromBelow(token.svara, center, token.weight))

pub fn perform(phrase, context):
  match phrase:
    PhraseValue(tokens) ->
      traverse_result(tokens, fn(token): gesture_for(context, token))

fn staff_pitch(svara):
  match svara:
    Sa -> staff.pitch("c", "natural", 4)
    Ri -> staff.pitch("d", "natural", 4)
    Ga -> staff.pitch("e", "flat", 4)
    Ma -> staff.pitch("f", "natural", 4)
    Pa -> staff.pitch("g", "natural", 4)
    Da -> staff.pitch("a", "flat", 4)
    Ni -> staff.pitch("b", "flat", 4)

fn note_for(token):
  let marks = match token.gesture:
    Plain -> []
    Oscillate -> [staff.text_mark("oscillating gesture")]
    SlideFromBelow -> [staff.text_mark("approach from below")]

  staff.note_with_marks(
    staff_pitch(token.svara),
    staff.exact_duration(token.weight),
    marks,
  )

fn losses_for(token):
  match token.gesture:
    Plain -> []
    Oscillate -> [CurveReducedToMark(token.svara)]
    SlideFromBelow -> [
      CurveReducedToMark(token.svara),
      DirectionKeptAsText(token.direction),
    ]

pub fn transcribe(phrase, context):
  match phrase:
    PhraseValue(tokens) ->
      let notes = list.map(tokens, fn(token): note_for(token))
      let token_losses = list.fold(
        tokens,
        [],
        fn(token, losses): losses ++ losses_for(token),
      )
      (
        staff.sequence(notes),
        token_losses ++ [TonicReducedToStaffSpelling(context.tonic_hz)],
      )
```

### 4.2 Client source

```musa
// pieces/phrase-pressure-test.musa
import trial.phrase_led as phrase
import std.result as result
import std.notation.staff as staff

let context = phrase.PhraseContext:
  tonic_hz = 264/1
  register_ratio = 2/1
  oscillation_width = 16/15

let checked_phrase = phrase.make([
  phrase.PhraseToken:
    svara = phrase.Ga
    direction = phrase.Falling
    gesture = phrase.Oscillate
    weight = 1/4,
  phrase.PhraseToken:
    svara = phrase.Ri
    direction = phrase.Falling
    gesture = phrase.SlideFromBelow
    weight = 1/4,
  phrase.PhraseToken:
    svara = phrase.Sa
    direction = phrase.Falling
    gesture = phrase.Plain
    weight = 1/2,
])

let gesture_route = and_then(
  checked_phrase,
  fn(value): phrase.perform(value, context),
)

let notation_route = result.map(
  checked_phrase,
  fn(value):
    let transcription = phrase.transcribe(value, context)
    let page = first(transcription)
    let losses = second(transcription)
    (staff.to_music(page), losses),
)
```

This file contains no adapter region, so expansion is again the identity.

### 4.3 Inference and evaluation

The compiler infers:

```text
make: List<PhraseToken> -> Result<Phrase, PhraseError>
perform: Phrase × PhraseContext -> Result<List<PhraseGesture>, PhraseError>
transcribe: Phrase × PhraseContext -> StaffDocument × List<TranscriptionLoss>
checked_phrase: Result<Phrase, PhraseError>
gesture_route: Result<List<PhraseGesture>, PhraseError>
notation_route: Result<Music × List<TranscriptionLoss>, PhraseError>
```

`gesture_route` evaluates to:

```text
Ok([
  OscillateAround(Ga, 3168/5, 1/4, 16/15),
  ApproachFromBelow(Ri, 594/1, 1/4),
  Hold(Sa, 528/1, 1/2),
])
```

The notation route records:

```text
[
  CurveReducedToMark(Ga),
  CurveReducedToMark(Ri),
  DirectionKeptAsText(Falling),
  TonicReducedToStaffSpelling(264/1),
]
```

### 4.4 The two routes

The performance-led route is primary:

```text
Phrase -> PhraseContext -> List<PhraseGesture>
       -> GestureTimeline -> PreparedExecution -> audio history
```

A gesture interpreter adds continuous pitch paths, exact onset choices, and instrument technique. The notation-led route
chooses Western staff spellings and text marks, then reports that those marks do not determine the curves.

Neither route can recover learned phrase membership from the other. The derivation graph connects them to the same
phrase source without declaring them equal.

### 4.5 Count

| Measure | Count |
| --- | ---: |
| Required source type annotations | 0 |
| Compiler-owned musical surface forms | 0 |
| Adapter regions | 0 |
| Explicit context that hidden state would otherwise supply | tonic, register, oscillation width |
| Locally reportable errors | empty phrase; non-positive weight; transcription failure |

The package owns every phrase-specific name and rule. The core sees ordinary finite data.

## 5. Ensemble-led tuning

### 5.1 Package source

This is an acoustic pressure test. It does not claim to model Balinese gamelan as a whole or to define a package that
could be shipped under that name.

```musa
// trial/ensemble_tuning.musa
import std.notation.staff as staff

pub type Degree:
  Center
  UpperThird
  UpperFifth

pub type Register:
  Low
  Middle
  High

pub type Pairing:
  Single
  Paired

pub record TuneRequest:
  degree: Degree
  register: Register
  pairing: Pairing

pub record AcousticTarget:
  lower_hz: Ratio
  upper_hz: Option<Ratio>
  target_beats_per_second: Ratio

pub type TuningError:
  NonPositiveReference
  NonPositivePairRate
  ArithmeticFailure

pub type TuningLoss:
  ExactFrequencyNotShown(Ratio)
  PairingNotShown
  BeatingTargetNotShown(Ratio)

pub type Ensemble:
  private EnsembleValue(Ratio, Ratio, Ratio, Ratio)

pub fn make(reference_hz, low_rate, middle_rate, high_rate):
  if ratio_greater(reference_hz, 0/1):
    if ratio_greater(low_rate, 0/1)
      && ratio_greater(middle_rate, 0/1)
      && ratio_greater(high_rate, 0/1):
      Ok(EnsembleValue(reference_hz, low_rate, middle_rate, high_rate))
    else:
      Err(NonPositivePairRate)
  else:
    Err(NonPositiveReference)

fn degree_ratio(degree):
  match degree:
    Center -> 1/1
    UpperThird -> 5/4
    UpperFifth -> 3/2

fn register_ratio(register):
  match register:
    Low -> 1/2
    Middle -> 1/1
    High -> 2/1

fn beat_target(register, low_rate, middle_rate, high_rate):
  match register:
    Low -> low_rate
    Middle -> middle_rate
    High -> high_rate

pub fn realize(ensemble, request):
  match ensemble:
    EnsembleValue(reference_hz, low_rate, middle_rate, high_rate) ->
      let degree_hz = checked_ratio_multiply(
        reference_hz,
        degree_ratio(request.degree),
      )
      let registered_hz = and_then(
        degree_hz,
        fn(value): checked_ratio_multiply(value, register_ratio(request.register)),
      )
      match registered_hz:
        Err(error) -> Err(ArithmeticFailure)
        Ok(lower) ->
          match request.pairing:
            Single -> Ok(AcousticTarget:
              lower_hz = lower
              upper_hz = None
              target_beats_per_second = 0/1
            )
            Paired ->
              let target_rate = beat_target(
                request.register,
                low_rate,
                middle_rate,
                high_rate,
              )
              match checked_ratio_add(lower, target_rate):
                Err(error) -> Err(ArithmeticFailure)
                Ok(upper) -> Ok(AcousticTarget:
                  lower_hz = lower
                  upper_hz = Some(upper)
                  target_beats_per_second = target_rate
                )

fn staff_degree(degree, register):
  match degree:
    Center -> staff.pitch("c", "natural", register)
    UpperThird -> staff.pitch("e", "natural", register)
    UpperFifth -> staff.pitch("g", "natural", register)

fn staff_register(register):
  match register:
    Low -> 3
    Middle -> 4
    High -> 5

pub fn transcribe(request, target):
  let page = staff.note_with_marks(
    staff_degree(request.degree, staff_register(request.register)),
    staff.note_value(2, 0),
    [staff.text_mark("ensemble-specific tuning")],
  )
  let frequency_losses = match target.upper_hz:
    None -> [ExactFrequencyNotShown(target.lower_hz)]
    Some(upper) -> [
      ExactFrequencyNotShown(target.lower_hz),
      ExactFrequencyNotShown(upper),
      PairingNotShown,
      BeatingTargetNotShown(target.target_beats_per_second),
    ]
  (page, frequency_losses)
```

### 5.2 Client source

```musa
// pieces/ensemble-pressure-test.musa
import trial.ensemble_tuning as tuning
import std.result as result
import std.notation.staff as staff

let request = tuning.TuneRequest:
  degree = tuning.UpperFifth
  register = tuning.Middle
  pairing = tuning.Paired

let target = and_then(
  tuning.make(440/1, 4/1, 7/1, 11/1),
  fn(ensemble): tuning.realize(ensemble, request),
)

let performance_route = result.map(
  target,
  fn(value): paired_frequency_gesture(
    value.lower_hz,
    value.upper_hz,
    value.target_beats_per_second,
  ),
)

let notation_route = result.map(
  target,
  fn(value):
    let transcription = tuning.transcribe(request, value)
    (staff.to_music(first(transcription)), second(transcription)),
)
```

Expansion is the identity because the program contains no adapter block.

### 5.3 Inference and evaluation

The compiler infers:

```text
make: Ratio × Ratio × Ratio × Ratio -> Result<Ensemble, TuningError>
realize: Ensemble × TuneRequest -> Result<AcousticTarget, TuningError>
transcribe: TuneRequest × AcousticTarget -> StaffDocument × List<TuningLoss>
target: Result<AcousticTarget, TuningError>
performance_route: Result<GestureIntent, TuningError>
notation_route: Result<Music × List<TuningLoss>, TuningError>
```

The chosen example evaluates to:

```text
Ok(AcousticTarget {
  lower_hz = 660/1,
  upper_hz = Some(667/1),
  target_beats_per_second = 7/1,
})
```

The lower and upper targets differ by exactly seven hertz. A real ensemble package would derive its degree, register,
and beating plans from measurements and named practice rather than use this three-rate toy. The point of the example is
that the relation fits as package data without becoming a global pitch type.

### 5.4 The two routes

The performance route turns both exact frequencies and the acoustic target into two bound gestures. The DSP boundary
approximates rationals by its numeric format and records that approximation.

The notation route prints one degree label and an ensemble-tuning instruction. It reports every exact frequency, the
pair relation, and the beating target as losses. Generic MIDI would lose still more.

### 5.5 Count

| Measure | Count |
| --- | ---: |
| Required source type annotations | 0 |
| Compiler-owned musical surface forms | 0 |
| Adapter regions | 0 |
| Explicit context that hidden state would otherwise supply | ensemble reference and three register-specific rates |
| Locally reportable errors | invalid ensemble; checked arithmetic; missing upper pair |

Checked rational arithmetic serves both this case and flexible time. Ensemble, degree, register, and acoustic target
remain package concepts.

## 6. Interactive performance

### 6.1 Package source

This package is a pressure test inspired by call-and-response between movement and drumming. It is not an account of
bomba, and it must not be published under that name without practitioner review.

```musa
// trial/live_dialogue.musa
import std.notation.staff as staff

pub type Cue:
  Begin
  Continue
  Cut

pub type Movement:
  LeaderCue(Cue)
  Motion(Text)
  Stillness

pub type State:
  Waiting
  Active(Nat)
  Ended

pub type Response:
  Listen
  DrumAnswer(Text)
  Stop

pub type ProtocolError:
  MotionBeforeBegin
  InputAfterEnd
  TurnCounterFull

pub type ProtocolLoss:
  ActualMovementMissing
  ActualTimingMissing
  PerformerJudgmentMissing

pub fn step(state, movement):
  match state:
    Waiting ->
      match movement:
        LeaderCue(Begin) -> Ok((Active(0), Listen))
        LeaderCue(Continue) -> Ok((Waiting, Listen))
        LeaderCue(Cut) -> Ok((Ended, Stop))
        Motion(name) -> Err(MotionBeforeBegin)
        Stillness -> Ok((Waiting, Listen))
    Active(turn) ->
      match movement:
        LeaderCue(Begin) -> Ok((Active(turn), Listen))
        LeaderCue(Continue) ->
          match checked_nat_add(turn, 1):
            Ok(next) -> Ok((Active(next), Listen))
            Err(error) -> Err(TurnCounterFull)
        LeaderCue(Cut) -> Ok((Ended, Stop))
        Motion(name) -> Ok((Active(turn), DrumAnswer(name)))
        Stillness -> Ok((Active(turn), Listen))
    Ended -> Err(InputAfterEnd)

pub fn gesture(response):
  match response:
    Listen -> silence_gesture(1/16)
    DrumAnswer(name) -> named_drum_gesture(name)
    Stop -> release_all_gesture()

pub fn cue_sheet():
  let page = staff.sequence([
    staff.text_cue("Begin: enter the dialogue"),
    staff.text_cue("Motion: lead drummer answers the observed movement"),
    staff.text_cue("Continue: keep the dialogue open"),
    staff.text_cue("Cut: stop together"),
  ])
  (
    page,
    [ActualMovementMissing, ActualTimingMissing, PerformerJudgmentMissing],
  )
```

### 6.2 Client source and graph expansion

```musa
// pieces/live-dialogue-pressure-test.musa
import syntax std.studio.graph as graph
import trial.live_dialogue as dialogue
import std.studio.graph as graph_value

let protocol = dialogue.step
let initial_state = dialogue.Waiting

let studio = graph:
  input live_moves: control

  instrument answer_drum = sample_player:
    voices = 8

  processor room = reverb:
    room = 3/4
    damping = 1/2
    mix = 1/4

  output main: audio(2)

  connect answer_drum.audio -> room.audio
  connect room.audio -> main.in

  bind lead_drum -> answer_drum

let checked_studio = graph_value.validate(studio)
let rehearsal = dialogue.cue_sheet()
```

The graph block expands to this one expression:

```musa
std.studio.graph.make_description([
  Input(i0, "live_moves", Control),
  Node(i1, "answer_drum", ProcessorName("sample_player"), [
    Parameter(i2, "voices", Count(8)),
  ]),
  Node(i3, "room", ProcessorName("reverb"), [
    Parameter(i4, "room", Plain(3/4)),
    Parameter(i5, "damping", Plain(1/2)),
    Parameter(i6, "mix", Plain(1/4)),
  ]),
  Output(i7, "main", Audio(2)),
  Connect(i8, PortPath("answer_drum", "audio"), PortPath("room", "audio")),
  Connect(i9, PortPath("room", "audio"), PortPath("main", "in")),
  Bind(i10, PartName("lead_drum"), "answer_drum"),
])
```

The input is declared because the host records live movement events beside the graph, but it is not wired as an audio or
control signal. The host consumes it through the typed protocol function. A later graph adapter may expose an explicit
event-to-control node; this example does not pretend movement is a scalar signal.

### 6.3 Inference and finite evaluation

The compiler infers:

```text
step: State × Movement -> Result<State × Response, ProtocolError>
gesture: Response -> GestureIntent
cue_sheet: Unit -> StaffDocument × List<ProtocolLoss>
protocol: State × Movement -> Result<State × Response, ProtocolError>
studio: StudioDescription
checked_studio: Result<CheckedStudio, StudioError>
rehearsal: StaffDocument × List<ProtocolLoss>
```

One source call finishes:

```text
step(Waiting, LeaderCue(Begin))
-> Ok((Active(0), Listen))
```

For this finite input prefix:

```text
[
  LeaderCue(Begin),
  Motion("high turn"),
  LeaderCue(Continue),
  Motion("low sweep"),
  LeaderCue(Cut),
]
```

the host obtains:

```text
[
  (Active(0), Listen),
  (Active(0), DrumAnswer("high turn")),
  (Active(1), Listen),
  (Active(1), DrumAnswer("low sweep")),
  (Ended, Stop),
]
```

The source language evaluates each call once. The host performs the repeated calls.

### 6.4 The two routes

The notation-led route produces a cue sheet and explicitly reports that the page omits movement, timing, and performer
judgment. It is useful for rehearsal, not a transcript of the performance.

The performance-led route is:

```text
live movement_n × state_n
  -> step
  -> response_n × state_(n+1)
  -> finite gesture_n
  -> prepared graph step
  -> audio block_n
```

The host may repeat this path without a last `n`. The finite protocol and every source call terminate. The resulting
audio history need not.

### 6.5 Count

| Measure | Count |
| --- | ---: |
| Required source type annotations | 0 |
| Compiler-owned musical surface forms | 0 |
| Adapter regions | 1 studio block |
| Explicit context that hidden state would otherwise supply | live input, current state, wall-clock time |
| Locally reportable errors | invalid transition; counter overflow; graph validation; preparation failure |

The package owns the vocabulary and response rule. The host owns input, repetition, devices, and clocks.

## 7. What the five programs force

The programs admit only features used in two different cases or required by a proof or stage boundary.

| Feature | Independent uses | Decision |
| --- | --- | --- |
| Rank-1 inference | every package and client | keep |
| Complete calls and explicit closures | every higher-order call | keep |
| Records and nominal variants | all five cases | keep |
| Hidden constructors | tonal, phrase, tuning | keep |
| Exhaustive matches | tonal, phrase, tuning, interactive | keep |
| Strictly positive recursive data and finite folds | staff form, syntax, lists | keep |
| `Text` | evidence, labels, losses, processor and movement names | keep |
| `Result` | construction, arithmetic, analysis, protocol, every stage error | keep |
| Exact checked ratios | flexible time and ensemble tuning | keep |
| Bounded syntax adapters | staff and studio | keep |
| Explicit stage contexts and loss records | all five cases | keep |
| Unequal-length overlay | flexible time and ordinary ensemble writing | keep in temporal kernel |
| Partial or named calls | none | reject |
| General recursion or source effects | none | reject |
| Dependent types or refinements | none | reject |
| CBPV or a source staging calculus | none | reject |
| Type-directed macros | none | reject |
| Universal pitch, metre, key, chord, or music value | contradicted by the cases | reject |

The examples use no local or function-result type annotations. Type declarations still name their field types; that is
the definition of the data, not a repeated annotation on each use.

## 8. Domain verdict

The active candidate passes the five-case domain gate.

It passes because the core language says little about music. Packages say what a chord, phrase, ensemble degree, timing
plan, or live protocol means. Syntax adapters help those packages present staff and graph data clearly. Explicit stage
functions then connect those values to finite temporal terms, notation, gestures, prepared processes, and sound.

The design does not claim universality. Lyrics, signed and embodied music, spatial music, DJ practice, studio-first
composition, ownership, ritual, and many musical traditions remain untested. Those gaps argue for keeping the language
small and the musical vocabulary open. They do not justify adding a type feature that no tested program uses.

The next gate is formal: the rules must make the inferred types, expansion order, termination, privacy, `Music` closure,
and stage composition proved facts rather than intentions.
