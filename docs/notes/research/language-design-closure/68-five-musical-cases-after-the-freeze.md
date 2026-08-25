# 68. Five musical cases after the adapter freeze

**Status: governs nothing.** This discharges note 26 §9's five-case audit on the language that exists after prompts
128–167. The complete declarations are the no-elision programs in note 43 §5; this note fixes one concrete execution of
each and writes every expansion, value, transition, loss, and added choice used by that execution. No omitted list
member is represented by an ellipsis.

## 1. Tonal harmony

The client constructs `DFlatMajor`, `GSeven`, and `CMinor`. Their inferred/elaborated declarations are:

```text
neapolitan : Chord = NamedChord(DFlatMajor, [DFlat, F, AFlat])
dominant   : Chord = NamedChord(GSeven, [G, B, D, F])
tonic      : Chord = NamedChord(CMinor, [C, EFlat, G])
claim      : Result FunctionClaim TonalError
page       : EventTrack WrittenTime ScoreFact
```

There is no syntax-adapter step. Evaluation of `analyze(CMinorKey, neapolitan, Some(GSeven))` is `Ok({key=CMinorKey,
symbol=DFlatMajor, degree=FlatTwo, function=Predominant, evidence="moves toward the dominant under this analysis"})`.
The notation-led route voices the three chords as `[DFlat3,F4,AFlat4]`, `[G4,B4,D4,F4]`, and `[C3,EFlat4,G4]`, places
them for `1/2`, `1/2`, and `1/1`, then hands the finite exact track to notation. The performance-led route takes the
same exact onsets and durations into a later gesture/realization choice. No information is lost before that fork. Added
choices are `OpenBass`, `Close`, written durations, and the C-minor analysis; the harmonic function is a claim under
that analysis, not a core fact.

## 2. Flexible time

The two `staff` regions expand independently to `StaffDocument` values. The melody contains, in order,
`c5/8,d5/8,e5/8,f5/8,g5/8,a5/8,b5/8,c6/8`, a held region `g5/4,rest/4,g5/2`, a feathered region `a5,b5,c6,d6`, and a
cadenza `e6,d6,b5,c6`. The ostinato contains `c3/4,rest/4,g3/4`. Each item carries its original anchor; generated
constructors carry quote-derived paths. The elaborated bindings are:

```text
melody, ostinato : StaffDocument
page             : StaffDocument
performance      : PerformancePlan
logical_choices  : LogicalChoices
score            : EventTrack WrittenTime ScoreFact
```

Expansion chooses only notation structure. Evaluation overlays the two documents without equalizing their lengths. The
next transition adds swing ratio `2/1`, an open fermata, eighth-to-quarter metric modulation, acceleration from
quarter=72 to quarter=144, and performer timing for the cadenza. Logical closure supplies feathered durations
`[1/8,1/12,1/16,1/32]` and cadenza durations `[1/4,1/8,1/8,1/2]`. The notation-led route retains written values, bars,
names, and layout. The performance-led route adds tempo shape and performer timing. The losses are explicit: open
fermata and performer-timed cadenza have no unique finite real-time duration until a performance supplies one.

## 3. Phrase-led intent

`make` checks three tokens: `Ga/Falling/Oscillate/1/4`, `Ri/Falling/SlideFromBelow/1/4`, and `Sa/Falling/Plain/1/2`. The
elaborated bindings are:

```text
context        : PhraseContext
checked_phrase : Result Phrase PhraseError
gesture_route  : Result (List PhraseGesture) PhraseError
notation_route : Result (EventTrack WrittenTime ScoreFact, List TranscriptionLoss) PhraseError
```

Evaluation returns `Ok(PhraseValue([the three tokens above]))`. With tonic `264/1`, register ratio `2/1`, and
oscillation width `16/15`, the performance-led result is
`[OscillateAround(Ga,3168/5,1/4,16/15), ApproachFromBelow(Ri,594/1,1/4), Hold(Sa,528/1,1/2)]`. The notation-led result
spells `[e-flat4/4,d4/4,c4/2]` and records `[CurveReducedToMark(Ga), CurveReducedToMark(Ri),
DirectionKeptAsText(Falling), TonicReducedToStaffSpelling(264/1)]`. Those four values are the losses; none is silently
discarded. Added choices are the chosen staff spellings and register. The phrase value itself precedes and survives the
fork.

## 4. Ensemble-led tuning

Evaluate `make(440/1,1/1,3/2,2/1)` and then realize `{degree=UpperThird,register=Middle,pairing=Paired}`. The elaborated
values are:

```text
ensemble : Result Ensemble TuningError
request  : TuneRequest
target   : AcousticTarget
written  : (EventTrack WrittenTime ScoreFact, List TuningLoss)
```

The exact target is `{lower_hz=550/1, upper_hz=Some(1103/2), target_beats_per_second=3/2}`. The performance-led route
hands both frequencies and the `3/2` beating target to gesture preparation. The notation-led route chooses written
`e4/2` plus the text mark `ensemble-specific tuning` and reports `[ExactFrequencyNotShown(550/1),
ExactFrequencyNotShown(1103/2), PairingNotShown, BeatingTargetNotShown(3/2)]`. Staff spelling, octave, duration, and
prose mark are added choices. Exact acoustic targets are losses of the notation projection, not losses of the
performance route.

## 5. Interactive performance

The finite input sequence is `[LeaderCue(Begin), Motion("answer-one"), LeaderCue(Continue), Stillness, LeaderCue(Cut)]`.
Its elaborated types are:

```text
state     : Phase
movement  : Movement
response  : Response
gesture   : GestureIntent
cue_sheet : (EventTrack WrittenTime ScoreFact, List ProtocolLoss)
```

Evaluation steps are exactly `Waiting -> Active(0)/Listen`, `Active(0) -> Active(0)/DrumAnswer("answer-one")`,
`Active(0) -> Active(1)/Listen`, `Active(1) -> Active(1)/Listen`, and `Active(1) -> Ended/Stop`. The performance-led
route maps those responses to `[silence(1/16),named_drum("answer-one"),silence(1/16),silence(1/16),release_all()]`; this
finite gesture sequence then drives a prepared process, whose later audio history may be unbounded. The notation-led
route is the four cue texts `Begin`, `Motion`, `Continue`, and `Cut`. It reports
`[ActualMovementMissing,ActualTimingMissing,PerformerJudgmentMissing]`. The observed motion, timing, and judgment are
performance inputs; reducing them to cue text would be a loss. The concrete motion name and every response are added
choices made during the performance.

## 6. Admission result

All five programs use the dependent core without requiring a dependent index of their own. The only expansions are the
two staff blocks in flexible time, and they neither inspect an inferred type nor receive compiler privilege. Written
rhythm stays distinct from exact ambient time; finite typed values cross each stage; notation-led and performance-led
routes fork rather than pretending to share a final representation. No rejected feature is needed.
