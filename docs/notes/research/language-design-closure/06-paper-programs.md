# Five complete paper programs

**Purpose:** test whether the proposed source calculus can express the chosen musical cases without hidden machinery.

These programs use the proposed language, not the current parser. They are labelled `text` rather than `.musa` because
the current compiler does not accept them. No program contains omitted code.

## 1. Shared compiler bridge

The examples assume this small, fixed adapter surface:

```text
notation_note:
  Text -> Nat -> Duration -> Result<Music, NotationError>

notation_chord:
  List<(Text, Nat)> -> Duration -> Result<Music, NotationError>

music_empty: Unit -> Music
music_then: Music -> Music -> Music
music_over: Music -> Music -> Music

quarter: Duration
half: Duration
whole: Duration
```

`Text` gives the notation adapter a spelled pitch such as `"D-flat"`; `Nat` gives the written register. The adapter
either returns a score recipe or explains why it cannot write the request. It knows nothing about keys, chords, harmonic
function, phrases, or ensemble tuning.

The stage examples also name these ordinary records. They are not source-language types:

```text
NotationContext = {
  staff: StaffId,
  clef_policy: ClefPolicy,
  target: NotationTarget
}

PrepareOptions = {
  sample_rate: 48000,
  block_size: 128,
  channels: 2
}
```

## 2. Tonal construction and analysis

This package keeps seven ideas separate: spelling, chord symbol, constructed chord, transposition, voicing, scale
degree, and harmonic function. The example is deliberately small, but none of its distinctions depends on being small.

### 2.1 Program

```text
data Spelling {
  C;
  CSharp;
  DFlat;
  D;
  EFlat;
  E;
  F;
  FSharp;
  GFlat;
  G;
  AFlat;
  A;
  BFlat;
  B;
}

data ChordSymbol {
  CMajor;
  CMinor;
  DFlatMajor;
  FMajor;
  GSeven;
  AMinor;
  BDiminished;
}

data ChordOrigin {
  Named(origin: ChordSymbol);
  TransposedChord;
}

data VoicingPolicy {
  Close;
  OpenBass;
}

data WrittenTone {
  Written(spelling: Spelling, register: Nat);
}

data Key {
  CMajorKey;
  CMinorKey;
}

data ScaleDegree {
  One;
  Four;
  Five;
  Six;
  Seven;
  FlatTwo;
  OutsideScale;
}

data HarmonicFunction {
  Tonic;
  Predominant;
  Dominant;
  Other;
}

data FunctionClaim {
  Claim(
    key: Key,
    symbol: ChordSymbol,
    degree: ScaleDegree,
    function: HarmonicFunction,
    evidence: Text
  );
}

data TonalError {
  NotationFailed;
  CannotAnalyzeTransposedChord;
}

signature TONAL {
  type Chord;
  type Voicing;
  let build: ChordSymbol -> Chord;
  let transpose_up_semitone: Chord -> Chord;
  let voice: VoicingPolicy -> Chord -> Voicing;
  let analyze: Key -> Option<ChordSymbol> -> Chord -> Option<ChordSymbol> -> Result<FunctionClaim, TonalError>;
  let notate: Voicing -> Result<Music, TonalError>;
}

structure Tonal: TONAL {
  data Chord {
    ChordValue(origin: ChordOrigin, tones: List<Spelling>);
  }

  data Voicing {
    VoicingValue(tones: List<WrittenTone>);
  }

  let spelling_text(spelling: Spelling): Text =
    match spelling {
      C => "C";
      CSharp => "C-sharp";
      DFlat => "D-flat";
      D => "D";
      EFlat => "E-flat";
      E => "E";
      F => "F";
      FSharp => "F-sharp";
      GFlat => "G-flat";
      G => "G";
      AFlat => "A-flat";
      A => "A";
      BFlat => "B-flat";
      B => "B";
    };

  let raise(spelling: Spelling): Spelling =
    match spelling {
      C => CSharp;
      CSharp => D;
      DFlat => D;
      D => EFlat;
      EFlat => E;
      E => F;
      F => FSharp;
      FSharp => G;
      GFlat => G;
      G => AFlat;
      AFlat => A;
      A => BFlat;
      BFlat => B;
      B => C;
    };

  let prepend_raised(spelling: Spelling, output: List<Spelling>): List<Spelling> =
    raise(spelling) :: output;

  let prepend_spelling(spelling: Spelling, output: List<Spelling>): List<Spelling> =
    spelling :: output;

  let reverse_spellings(input: List<Spelling>): List<Spelling> =
    fold_list(input, [], prepend_spelling);

  let named_tones(symbol: ChordSymbol): List<Spelling> =
    match symbol {
      CMajor => [C, E, G];
      CMinor => [C, EFlat, G];
      DFlatMajor => [DFlat, F, AFlat];
      FMajor => [F, A, C];
      GSeven => [G, B, D, F];
      AMinor => [A, C, E];
      BDiminished => [B, D, F];
    };

  let build(symbol: ChordSymbol): Chord =
    ChordValue(Named(symbol), named_tones(symbol));

  let transpose_up_semitone(chord: Chord): Chord =
    match chord {
      ChordValue(origin, tones) =>
        ChordValue(
          TransposedChord,
          reverse_spellings(fold_list(tones, [], prepend_raised))
        );
    };

  let close_step(spelling: Spelling, output: List<WrittenTone>): List<WrittenTone> =
    Written(spelling, 4) :: output;

  let upper_step(spelling: Spelling, output: List<WrittenTone>): List<WrittenTone> =
    Written(spelling, 4) :: output;

  let prepend_written(tone: WrittenTone, output: List<WrittenTone>): List<WrittenTone> =
    tone :: output;

  let reverse_written(input: List<WrittenTone>): List<WrittenTone> =
    fold_list(input, [], prepend_written);

  let close_voicing(tones: List<Spelling>): List<WrittenTone> =
    reverse_written(fold_list(tones, [], close_step));

  let open_voicing(tones: List<Spelling>): List<WrittenTone> =
    match tones {
      [] => [];
      bass :: upper =>
        Written(bass, 3) :: reverse_written(fold_list(upper, [], upper_step));
    };

  let voice(policy: VoicingPolicy, chord: Chord): Voicing =
    match chord {
      ChordValue(origin, tones) =>
        match policy {
          Close => VoicingValue(close_voicing(tones));
          OpenBass => VoicingValue(open_voicing(tones));
        };
    };

  let degree(key: Key, symbol: ChordSymbol): ScaleDegree =
    match key {
      CMinorKey =>
        match symbol {
          CMajor => One;
          CMinor => One;
          DFlatMajor => FlatTwo;
          FMajor => Four;
          GSeven => Five;
          AMinor => Six;
          BDiminished => Seven;
        };
      CMajorKey =>
        match symbol {
          CMajor => One;
          CMinor => One;
          DFlatMajor => FlatTwo;
          FMajor => Four;
          GSeven => Five;
          AMinor => Six;
          BDiminished => Seven;
        };
    };

  let function_of(
    key: Key,
    before: Option<ChordSymbol>,
    symbol: ChordSymbol,
    after: Option<ChordSymbol>
  ): HarmonicFunction =
    match symbol {
      CMajor => Tonic;
      CMinor => Tonic;
      DFlatMajor =>
        match after {
          Some(next) =>
            match next {
              GSeven => Predominant;
              _ => Other;
            };
          None => Other;
        };
      FMajor => Predominant;
      GSeven => Dominant;
      AMinor => Tonic;
      BDiminished => Dominant;
    };

  let evidence_for(function: HarmonicFunction): Text =
    match function {
      Tonic => "stability in this analysis";
      Predominant => "moves toward the dominant in this context";
      Dominant => "directed toward the tonic in this analysis";
      Other => "no function assigned by this method";
    };

  let analyze(
    key: Key,
    before: Option<ChordSymbol>,
    chord: Chord,
    after: Option<ChordSymbol>
  ): Result<FunctionClaim, TonalError> =
    match chord {
      ChordValue(origin, tones) =>
        match origin {
          Named(symbol) =>
            let function = function_of(key, before, symbol, after);
            Ok(Claim(key, symbol, degree(key, symbol), function, evidence_for(function)));
          TransposedChord => Err(CannotAnalyzeTransposedChord);
        };
    };

  let request_step(tone: WrittenTone, output: List<(Text, Nat)>): List<(Text, Nat)> =
    match tone {
      Written(spelling, register) => (spelling_text(spelling), register) :: output;
    };

  let prepend_request(request: (Text, Nat), output: List<(Text, Nat)>): List<(Text, Nat)> =
    request :: output;

  let reverse_requests(input: List<(Text, Nat)>): List<(Text, Nat)> =
    fold_list(input, [], prepend_request);

  let notate(voicing: Voicing): Result<Music, TonalError> =
    match voicing {
      VoicingValue(tones) =>
        let requests = reverse_requests(fold_list(tones, [], request_step));
        match notation_chord(requests, half) {
          Ok(music) => Ok(music);
          Err(error) => Err(NotationFailed);
        };
    };
}

let neapolitan: Tonal.Chord = Tonal.build(DFlatMajor);
let dominant: Tonal.Chord = Tonal.build(GSeven);
let tonic: Tonal.Chord = Tonal.build(CMinor);
let written_neapolitan: Tonal.Voicing = Tonal.voice(OpenBass, neapolitan);
let neapolitan_analysis: Result<FunctionClaim, TonalError> =
  Tonal.analyze(CMinorKey, None, neapolitan, Some(GSeven));
let raised_neapolitan: Tonal.Chord = Tonal.transpose_up_semitone(neapolitan);
```

### 2.2 Type derivation

The compiler assigns a fresh type name `mu_chord` to `Tonal.Chord` and `mu_voicing` to `Tonal.Voicing`.

1. `named_tones(DFlatMajor)` synthesizes `List<Spelling>`.
2. Inside the structure, `ChordValue` has type `ChordOrigin -> List<Spelling> -> mu_chord`, so `build(DFlatMajor)` has
   type `mu_chord`.
3. Outside the structure, `Tonal.build` has the public type `ChordSymbol -> Tonal.Chord`. The constructor `ChordValue`
   is absent.
4. `Some(GSeven)` checks against `Option<ChordSymbol>` because `analyze` supplies that expected type.
5. The `Ok` term that contains a `FunctionClaim` checks against `Result<FunctionClaim, TonalError>` because the declared
   return type supplies both type arguments.
6. Each data match lists every visible constructor, so coverage succeeds.

### 2.3 Evaluation trace

For the Neapolitan analysis:

```text
Tonal.analyze(CMinorKey, None, Tonal.build(DFlatMajor), Some(GSeven))
--> analyze(CMinorKey, None,
            ChordValue(Named(DFlatMajor), [DFlat, F, AFlat]),
            Some(GSeven))
--> Ok(Claim(CMinorKey,
             DFlatMajor,
             FlatTwo,
             Predominant,
             "moves toward the dominant in this context"))
```

The result proves the intended separation. `FlatTwo` is the scale-degree label. `Predominant` is the function assigned
by this analysis and context. Neither the type checker nor a tonic translation invents that function.

### 2.4 What belongs where

The package owns chord construction, spelling choices, transposition, voicing, degree labels, and this theory of
function. The core supplies only finite data, functions, matches, lists, abstraction, and `Result`. The notation adapter
only turns spelled, registered tones into score facts.

## 3. Phrase-led intent and lossy transcription

This is a pressure test, not a Karnatak package. It shows that a phrase can have a direct performance meaning while
staff notation remains an optional, lossy view.

### 3.1 Program

```text
data Svara {
  Sa;
  Ri;
  Ga;
  Ma;
  Pa;
  Da;
  Ni;
}

data Direction {
  Rising;
  Falling;
  Turning;
}

data GamakaIntent {
  Plain;
  Oscillate;
  SlideFromBelow;
}

data PhraseToken {
  Token(svara: Svara, direction: Direction, gamaka: GamakaIntent, length: Duration);
}

data TranscriptionLoss {
  ContinuousCurveReducedToMark(svara: Svara);
  DirectionKeptOnlyAsText(direction: Direction);
}

data PhraseError {
  EmptyPhrase;
  NotationFailedFor(svara: Svara);
}

data GestureIntent {
  HoldSvara(svara: Svara, length: Duration);
  OscillateSvara(svara: Svara, length: Duration);
  ApproachSvaraFromBelow(svara: Svara, length: Duration);
}

signature PHRASE {
  type Phrase;
  let make: List<PhraseToken> -> Result<Phrase, PhraseError>;
  let perform: Phrase -> List<GestureIntent>;
  let transcribe: Phrase -> Result<(Music, List<TranscriptionLoss>), PhraseError>;
}

structure PhraseModel: PHRASE {
  data Phrase {
    PhraseValue(tokens: List<PhraseToken>);
  }

  let make(tokens: List<PhraseToken>): Result<Phrase, PhraseError> =
    match tokens {
      [] => Err(EmptyPhrase);
      head :: tail => Ok(PhraseValue(tokens));
    };

  let gesture(token: PhraseToken): GestureIntent =
    match token {
      Token(svara, direction, gamaka, length) =>
        match gamaka {
          Plain => HoldSvara(svara, length);
          Oscillate => OscillateSvara(svara, length);
          SlideFromBelow => ApproachSvaraFromBelow(svara, length);
        };
    };

  let gesture_step(token: PhraseToken, output: List<GestureIntent>): List<GestureIntent> =
    gesture(token) :: output;

  let prepend_gesture(item: GestureIntent, output: List<GestureIntent>): List<GestureIntent> =
    item :: output;

  let reverse_gestures(input: List<GestureIntent>): List<GestureIntent> =
    fold_list(input, [], prepend_gesture);

  let perform(phrase: Phrase): List<GestureIntent> =
    match phrase {
      PhraseValue(tokens) => reverse_gestures(fold_list(tokens, [], gesture_step));
    };

  let svara_text(svara: Svara): Text =
    match svara {
      Sa => "C";
      Ri => "D";
      Ga => "E-flat";
      Ma => "F";
      Pa => "G";
      Da => "A-flat";
      Ni => "B-flat";
    };

  let losses_for(token: PhraseToken): List<TranscriptionLoss> =
    match token {
      Token(svara, direction, gamaka, length) =>
        match gamaka {
          Plain => [];
          Oscillate => [ContinuousCurveReducedToMark(svara)];
          SlideFromBelow => [ContinuousCurveReducedToMark(svara), DirectionKeptOnlyAsText(direction)];
        };
    };

  let prepend_loss(
    loss: TranscriptionLoss,
    output: List<TranscriptionLoss>
  ): List<TranscriptionLoss> =
    loss :: output;

  let reverse_losses(input: List<TranscriptionLoss>): List<TranscriptionLoss> =
    fold_list(input, [], prepend_loss);

  let append_loss(
    loss: TranscriptionLoss,
    output: List<TranscriptionLoss>
  ): List<TranscriptionLoss> =
    loss :: output;

  let append_losses(
    left: List<TranscriptionLoss>,
    right: List<TranscriptionLoss>
  ): List<TranscriptionLoss> =
    fold_list(reverse_losses(left), right, append_loss);

  let transcribe_step(
    token: PhraseToken,
    state: Result<(Music, List<TranscriptionLoss>), PhraseError>
  ): Result<(Music, List<TranscriptionLoss>), PhraseError> =
    match state {
      Err(error) => Err(error);
      Ok(pair) =>
        match pair {
          (music, losses) =>
            match token {
              Token(svara, direction, gamaka, length) =>
                match notation_note(svara_text(svara), 4, length) {
                  Err(error) => Err(NotationFailedFor(svara));
                  Ok(note) => Ok((music_then(music, note), append_losses(losses, losses_for(token))));
                };
            };
        };
    };

  let transcribe(phrase: Phrase): Result<(Music, List<TranscriptionLoss>), PhraseError> =
    match phrase {
      PhraseValue(tokens) =>
        fold_list(tokens, Ok((music_empty(()), [])), transcribe_step);
    };
}

let phrase_tokens: List<PhraseToken> = [
  Token(Ga, Falling, Oscillate, quarter),
  Token(Ri, Falling, SlideFromBelow, quarter),
  Token(Sa, Falling, Plain, half)
];

let phrase_result: Result<PhraseModel.Phrase, PhraseError> = PhraseModel.make(phrase_tokens);
```

### 3.2 Type derivation

1. `PhraseToken` has rank 2: its fields mention the earlier rank-1 types `Svara`, `Direction`, and `GamakaIntent`.
2. The private `Phrase` type has rank 3 because it contains `List<PhraseToken>`.
3. `PhraseModel.make` checks `Ok(PhraseValue(tokens))` against `Result<Phrase, PhraseError>` and therefore determines
   both arguments of `Ok`.
4. `transcribe_step` has a declared accumulator type. Its `Err(error)` and both `Ok` terms therefore check without
   guessing.
5. The outer fold is finite because it consumes the stored finite token list.

### 3.3 Evaluation and stage trace

For `phrase_tokens`, `PhraseModel.perform` evaluates to:

```text
[
  OscillateSvara(Ga, quarter),
  ApproachSvaraFromBelow(Ri, quarter),
  HoldSvara(Sa, half)
]
```

That value can pass directly to the gesture interpreter. `PhraseModel.transcribe` instead returns score `Music` plus:

```text
[
  ContinuousCurveReducedToMark(Ga),
  ContinuousCurveReducedToMark(Ri),
  DirectionKeptOnlyAsText(Falling)
]
```

The two outputs share source ancestry. The notation is not declared equal to the performance gesture.

### 3.4 What belongs where

The package owns svara names, phrase tokens, direction, ornament intent, performance interpretation, and the stated
transcription losses. The language supplies finite data and folds. The notation bridge accepts only the package's
explicit spelling choice.

## 4. Ensemble tuning and an acoustic target

This is a pressure test, not a Balinese gamelan package. It keeps ensemble-specific pitch intent separate from a global
pitch class and records the acoustic effect that paired tuning seeks.

### 4.1 Program

```text
data EnsembleDegree {
  Center;
  UpperThird;
  UpperFifth;
}

data Register {
  Low;
  Middle;
  High;
}

data Pairing {
  Single;
  Paired;
}

data TuneRequest {
  Tune(degree: EnsembleDegree, register: Register, pairing: Pairing);
}

data PitchTarget {
  ExactTarget(ratio_from_reference: Ratio, register: Register);
}

data AcousticTarget {
  NoBeating;
  PairedBeating(rate_hz: Ratio);
}

data TuningLoss {
  ExactRatioNotShown;
  AcousticTargetNotShown;
}

data TuningError {
  ArithmeticFailed;
  NotationFailed;
}

signature ENSEMBLE_TUNING {
  type Ensemble;
  let make: Ratio -> Ratio -> Ensemble;
  let realize: Ensemble -> TuneRequest -> Result<(List<PitchTarget>, AcousticTarget), TuningError>;
  let transcribe: TuneRequest -> Result<(Music, List<TuningLoss>), TuningError>;
}

structure EnsembleTuning: ENSEMBLE_TUNING {
  data Ensemble {
    EnsembleValue(reference: Ratio, pair_offset: Ratio);
  }

  let make(reference: Ratio, pair_offset: Ratio): Ensemble =
    EnsembleValue(reference, pair_offset);

  let degree_ratio(degree: EnsembleDegree): Ratio =
    match degree {
      Center => 1/1;
      UpperThird => 5/4;
      UpperFifth => 3/2;
    };

  let beat_rate(register: Register): Ratio =
    match register {
      Low => 4/1;
      Middle => 7/1;
      High => 11/1;
    };

  let realize(
    ensemble: Ensemble,
    request: TuneRequest
  ): Result<(List<PitchTarget>, AcousticTarget), TuningError> =
    match ensemble {
      EnsembleValue(reference, pair_offset) =>
        match request {
          Tune(degree, register, pairing) =>
            match ratio_mul(reference, degree_ratio(degree)) {
              Err(error) => Err(ArithmeticFailed);
              Ok(first) =>
                match pairing {
                  Single => Ok(([ExactTarget(first, register)], NoBeating));
                  Paired =>
                    match ratio_mul(first, pair_offset) {
                      Err(error) => Err(ArithmeticFailed);
                      Ok(second) =>
                        Ok((
                          [ExactTarget(first, register), ExactTarget(second, register)],
                          PairedBeating(beat_rate(register))
                        ));
                    };
                };
            };
        };
    };

  let degree_text(degree: EnsembleDegree): Text =
    match degree {
      Center => "C";
      UpperThird => "E";
      UpperFifth => "G";
    };

  let register_number(register: Register): Nat =
    match register {
      Low => 3;
      Middle => 4;
      High => 5;
    };

  let transcribe(request: TuneRequest): Result<(Music, List<TuningLoss>), TuningError> =
    match request {
      Tune(degree, register, pairing) =>
        match notation_note(degree_text(degree), register_number(register), half) {
          Err(error) => Err(NotationFailed);
          Ok(note) =>
            match pairing {
              Single => Ok((note, [ExactRatioNotShown]));
              Paired => Ok((note, [ExactRatioNotShown, AcousticTargetNotShown]));
            };
        };
    };
}

let ensemble: EnsembleTuning.Ensemble = EnsembleTuning.make(440/1, 1001/1000);
let paired_middle: TuneRequest = Tune(UpperFifth, Middle, Paired);
let target: Result<(List<PitchTarget>, AcousticTarget), TuningError> =
  EnsembleTuning.realize(ensemble, paired_middle);
```

### 4.2 Type derivation

1. `Ensemble` contains only `Ratio`, so it is a valid private finite data type of rank 1.
2. `ratio_mul` returns `Result<Ratio, ArithmeticError>`. Each match handles both constructors before a `Ratio` is used.
3. The two successful branches both check against `Result<(List<PitchTarget>, AcousticTarget), TuningError>`.
4. The private constructor `EnsembleValue` cannot be used by a client; only `make` can create the abstract type.

### 4.3 Evaluation and stage trace

Ignoring only fraction reduction in the display, `target` evaluates to:

```text
Ok((
  [ExactTarget(660/1, Middle), ExactTarget(33033/50, Middle)],
  PairedBeating(7/1)
))
```

The gesture pass can consume both exact pitch targets and the beating target. The transcription branch emits one staff
note and records that it omitted both exact tuning and the acoustic goal.

### 4.4 What belongs where

The ensemble package owns degrees, paired tuning, register policy, and acoustic goals. Exact checked rational arithmetic
belongs to the general language because flexible time also needs it. A global `PitchClass12` is neither required nor
useful here.

## 5. A complete score and studio preparation

This example uses the tonal package to make a finite score, then crosses the existing studio boundary. The studio plan
is not a source-language value.

### 5.1 Source program

```text
data PieceError {
  TonalFailure;
}

let make_cadence(unit: Unit): Result<Music, PieceError> =
  let first_chord = Tonal.build(DFlatMajor);
  let second_chord = Tonal.build(GSeven);
  let third_chord = Tonal.build(CMinor);
  let first_voicing = Tonal.voice(OpenBass, first_chord);
  let second_voicing = Tonal.voice(Close, second_chord);
  let third_voicing = Tonal.voice(OpenBass, third_chord);
  match Tonal.notate(first_voicing) {
    Err(error) => Err(TonalFailure);
    Ok(first_music) =>
      match Tonal.notate(second_voicing) {
        Err(error) => Err(TonalFailure);
        Ok(second_music) =>
          match Tonal.notate(third_voicing) {
            Err(error) => Err(TonalFailure);
            Ok(third_music) =>
              Ok(music_then(first_music, music_then(second_music, third_music)));
          };
      };
  };

let cadence: Result<Music, PieceError> = make_cadence(());
```

### 5.2 Type derivation

Each `Tonal.notate` call synthesizes `Result<Music, TonalError>`. Its match handles `Err` and `Ok`. The declared result
of `make_cadence` makes every outer `Err(TonalFailure)` and the final `Ok` check against `Result<Music, PieceError>`.
The program uses no process or audio type.

### 5.3 Complete stage trace

For a successful `cadence`, the remaining inputs are:

```text
notation_context = {
  staff: Staff1,
  clef_policy: TrebleAndBass,
  target: MusicXML4
}

bindings = {
  Staff1: Instrument("built-in.piano.v1")
}

seed = 28411

options = {
  sample_rate: 48000,
  block_size: 128,
  channels: 2
}
```

The typed stages are:

```text
cadence Music
  -> instantiate(cadence, notation_context)
  -> close(fragment)
  -> Term<ScoreFact>
  -> Timeline<ScoreFact>
  -> interpret("measured-keyboard.v1")
  -> Timeline<Gesture>
  -> prepare(gestures, bindings, seed, options)
  -> PreparedExecution
  -> step(input_block_0, state_0)
  -> (audio_block_0, state_1)
```

Every arrow has the inputs and errors listed in `05-stage-semantics.md`. The source term terminates at `Music`; the host
starts process stepping only after preparation succeeds.

### 5.4 What belongs where

The cadence uses a tonal theory package. `NotationContext` belongs to notation. Instrument bindings and render options
belong to studio preparation. Sample production belongs to the process engine. None should become an implicit field of
`Music`.

## 6. A finite live protocol and an unbounded run

This is a pressure test, not a bomba package. It models a finite call-and-response rule that a host may apply for as
long as musicians continue.

### 6.1 Program

```text
data Cue {
  Begin;
  Continue;
  Cut;
}

data PlayerMove {
  LeaderCue(cue: Cue);
  EnsembleHit;
  EnsembleSilence;
}

data ProtocolState {
  Waiting;
  Active(turn: Nat);
  Ended;
}

data Response {
  Hold;
  AnswerHit;
  Stop;
}

data ProtocolError {
  HitBeforeBegin;
  MoveAfterEnd;
  TurnCounterFull;
}

let protocol_step(
  state: ProtocolState,
  move: PlayerMove
): Result<(ProtocolState, Response), ProtocolError> =
  match state {
    Waiting =>
      match move {
        LeaderCue(cue) =>
          match cue {
            Begin => Ok((Active(0), Hold));
            Continue => Ok((Waiting, Hold));
            Cut => Ok((Ended, Stop));
          };
        EnsembleHit => Err(HitBeforeBegin);
        EnsembleSilence => Ok((Waiting, Hold));
      };
    Active(turn) =>
      match move {
        LeaderCue(cue) =>
          match cue {
            Begin => Ok((Active(turn), Hold));
            Continue =>
              match nat_add(turn, 1) {
                Ok(next) => Ok((Active(next), Hold));
                Err(error) => Err(TurnCounterFull);
              };
            Cut => Ok((Ended, Stop));
          };
        EnsembleHit => Ok((Active(turn), AnswerHit));
        EnsembleSilence => Ok((Active(turn), Hold));
      };
    Ended => Err(MoveAfterEnd);
  };
```

`nat_add` is the general bounded natural-number operation. It returns `Result<Nat, ArithmeticError>`, and this package
turns overflow into its own `TurnCounterFull` error.

### 6.2 Type derivation

1. The outer match covers `Waiting`, `Active`, and `Ended`.
2. Each live-state inner match covers every `PlayerMove` constructor.
3. Every successful branch checks against `Result<(ProtocolState, Response), ProtocolError>`.
4. Every error branch checks against the same result type.
5. `protocol_step` does not refer to itself, so the global value graph remains acyclic.

### 6.3 Finite and unbounded traces

One finite source evaluation is:

```text
protocol_step(Waiting, LeaderCue(Begin))
--> Ok((Active(0), Hold))
```

The host, not the source language, repeats the operation:

```text
(state_n, move_n) -> protocol_step(state_n, move_n) -> (state_(n+1), response_n)
```

For the finite input prefix:

```text
[LeaderCue(Begin), EnsembleHit, LeaderCue(Continue), LeaderCue(Cut)]
```

the host obtains:

```text
[(Active(0), Hold),
 (Active(0), AnswerHit),
 (Active(1), Hold),
 (Ended, Stop)]
```

There may be no last input. Each call still terminates, and each accepted response can become a finite gesture before
studio preparation or live control.

### 6.4 What belongs where

The package owns the musical meaning of moves and responses. The source language only checks a finite state transition.
The host owns input, repetition, clocks, devices, and the unbounded run. No source-language stream or effect is needed.

## 7. What the programs forced

Two materially different cases forced checked rational arithmetic: flexible time and ensemble tuning.

All five cases need finite data, and three need hidden constructors. Phrase transcription, notation failure, tuning
arithmetic, and the live protocol all need `Result`. Named losses and evidence need `Text`.

Nothing forced dependent types, CBPV, a world term, a link term, general recursion, or effects. The cases did force one
architectural choice: notation-led and performance-led paths must both be valid.
