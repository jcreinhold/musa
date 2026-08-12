# Five musical cases

**Purpose:** try to break the proposed small language with real differences in harmony, time, notation, tuning, and live
interaction.

## How to read the cases

Each case separates three things:

- **Verified:** what a source or the current Musa implementation says.
- **Proposed model:** the smallest data and conversion steps needed for the example.
- **Judgment:** what the example does and does not require from the source language.

The examples are deliberately small. They test language shape. The three culturally named examples are not complete
accounts of their practices and must not be shipped as such without specialist review.

## 1. Tonal harmony: a Neapolitan cadence

### The example

In C minor, construct and voice this progression:

```text
D-flat major in first inversion -> G major -> C minor
```

One common analysis labels it `♭II⁶–V–i`. The first chord is a Neapolitan sixth and has predominant function. The second
has dominant function. The final chord has tonic function.

### Verified

Open Music Theory distinguishes all the values this example needs:

- Roman numerals are relative to a key, while chord symbols are absolute performance shorthand
  ([“Chord Symbols,” §Chord Symbols vs. Roman Numerals](https://viva.pressbooks.pub/openmusictheory/chapter/chord-symbols/)).
- Harmonic function groups chords by their role in a phrase. `V` and `vii°` can both have dominant function
  ([“Introduction to Harmony”](https://viva.pressbooks.pub/openmusictheory/chapter/intro-to-harmony/)).
- A Neapolitan sixth is a major triad on lowered scale degree two, normally in first inversion, with predominant
  function ([“Neapolitan Sixth Chords”](https://viva.pressbooks.pub/openmusictheory/chapter/neapolitan-sixth-chords/)).
- A voicing decides register, spacing, doubling, and omission. Jazz practice may omit a root from one instrument when a
  bass player supplies it ([“Jazz Voicings”](https://viva.pressbooks.pub/openmusictheory/chapter/jazz-voicings/)).

The current compiler already keeps `Key`, `Degree`, `ChordClass`, `Voicing`, and `Roman` separate. It can build scale
chords, choose close or drop voicings, and project a voicing back to chord content.

### Proposed model

The theory package owns these abstract types:

```text
Key, Degree, ChordRecipe, ChordClass, Voicing, FunctionClaim, Evidence, Error
```

Its construction path is:

```text
Key + ChordRecipe -> Result<ChordClass, Error>
ChordClass + voicing policy -> Result<Voicing, Error>
Voicing + Duration -> Music
Music -> Term<ScoreFact> -> Timeline<ScoreFact> -> NotationPlan
```

Its analysis path is separate:

```text
Key + observed passage -> List<FunctionClaim>
FunctionClaim -> Evidence
```

A notation backend may print the same notes under an absolute chord symbol, a Roman numeral, both, or neither. That
choice does not change the sounded pitches.

### What is lost or added

- Choosing a voicing adds register, order, doubling, and omission.
- Printing a chord symbol may omit the analyst's functional reading.
- MIDI normally loses spelling.
- Analysis adds a method-bound claim and evidence; it does not reveal a hidden field of the chord.

### Judgment

Harmonic function is not translation by a tonic and is not scale degree. Several roots may share one function, and a
chromatic chord such as the Neapolitan is classified by context and motion. The language needs theory-owned data,
private constructors, and explicit analysis results. It does not need a built-in `HarmonicFunction` or a pitch torsor.

## 2. Flexible time: one page, several clocks

### The example

Consider a short score with:

- a melody written as straight eighth notes but performed with swing;
- a simultaneous three-beat ostinato that ends before the melody;
- a fermata whose length is left to the performer;
- a metric modulation equating an old subdivision with a new beat;
- a feathered acceleration; and
- an unmeasured cadenza before the final chord.

### Verified

- Swing notation commonly writes equal eighth notes while performance makes them unequal. The ratio changes with tempo
  and performer
  ([Open Music Theory, “Swing Rhythms”](https://viva.pressbooks.pub/openmusictheory/chapter/swing-rhythms/)).
- Ametric music may retain written durations while lacking a perceived meter. It can also leave room for performer
  phrasing.
- Polymeter uses more than one meter at once. Metric modulation relates a duration in one tempo to a duration in the
  next. Timeline notation may use seconds instead of bars. Feathered beaming states acceleration or deceleration without
  fixing each onset
  ([Open Music Theory, “Twentieth-Century Rhythmic Techniques”](https://viva.pressbooks.pub/openmusictheory/chapter/twentieth-century-rhythmic-techniques/)).
- Rūpak tāl is not identified by the count `3+2+2`: another seven-beat cycle can share that grouping while differing in
  gesture and drum pattern. Slow performances also show non-isochrony and depend on learned metrical knowledge
  ([Clayton 2020](https://mtosmt.org/issues/mto.20.26.1/mto.20.26.1.clayton.html)).

The current kernel already overlays unequal lengths without padding. The compiler already keeps meter, logical beat,
tempo mapping, groove, and sample frames separate. It has unmeasured notation, polymeter, swing, tempo ramps, and
performance profiles.

### Proposed model

Use four explicit layers:

```text
logical placement: exact rational positions in a finite timeline
notated metre: score facts attached to regions or parts
performance intent: fermata, swing, rubato, or acceleration gestures
physical schedule: a chosen monotone map from logical position to seconds and frames
```

The shorter ostinato stays shorter under overlay. Nothing inserts a rest merely to make both operands equal.

The metric modulation is an exact relation between two logical duration units. Swing and rubato are later performance
maps. A fermata or feathered beam may remain underdetermined until a performance profile or performer supplies a choice.
An unmeasured passage can still use exact relative durations when the notation states them. If it does not, Musa records
an ordered or gestural instruction rather than inventing exact note lengths.

### What is lost or added

- Engraving can preserve a swing word, fermata, feathered beam, or metric equation while omitting exact performed
  onsets.
- A chosen performance adds those onsets and records the choice.
- Conversion to frames rounds or approximates physical time and records that loss.
- A meter analysis may add a heard grouping that differs from the written meter.

### Judgment

Exact rational timelines survive as the meaning of finite placed facts. A finite set of pulse layers does not survive as
the meaning of metre. It fails on underdetermined timing, learned cycles, free performance, and live conducted time.
Metre belongs in named data and analyses; performed time belongs in an explicit later map.

This case needs checked rational arithmetic. It does not need extent indices, equal-length overlay, or dependent types.

## 3. Phrase-led intent: a Karnatak pressure test

### The example

Represent a short phrase whose source records:

- svara intent relative to a fixed tonic;
- whether the phrase ascends or descends;
- one characteristic phrase label;
- a required oscillating or sliding gesture on a svara; and
- an explicit statement that a staff transcription is approximate.

This is a language test, not a proposed rāga grammar.

### Verified

Schachter reports that svara notation gives an approximate melodic guide, while exact rhythmic placement and rich gamaka
realization rely on oral knowledge. He also distinguishes svara, gamaka, characteristic phrases, ascent and descent, and
tonic/drone context
([Schachter 2015, §§12–15 and 29–30](https://www.mtosmt.org/issues/mto.15.21.4/mto.15.21.4.schachter.html)).

The current Musa language cannot define these carriers. It can encode them only as compiler additions, loose names, or
Western pitch values. Its current `Music` path also assumes score material comes first.

### Proposed model

A trial package defines finite types such as:

```text
SvaraIntent, Direction, GestureIntent, PhraseToken, Phrase, TranscriptionLoss
```

The package hides their representations and exposes total constructors and interpreters:

```text
make_phrase : List<PhraseToken> -> Result<Phrase, PhraseError>
perform : Phrase -> PerformanceContext -> Result<GestureMaterial, RealizationError>
transcribe : Phrase -> Result<(Music, List<TranscriptionLoss>), TranscriptionError>
```

The primary path can go directly from phrase intent to finite gestures, instrument bindings, a prepared graph, and
sound. The notation path is optional and records what it approximates.

### What is lost or added

- A transcription chooses noteheads, written durations, and graphic marks that may not determine the gesture.
- A performer or performance profile adds detailed pitch motion and timing.
- Audio realization adds instrument technique and physical frequency curves.
- Neither the staff nor the curve alone recovers learned phrase membership or cultural meaning.

### Judgment

The language needs theory-owned finite data, abstract types, `Result`, and explicit loss records. It does not need a
universal `Pitch`, `Scale`, or `Raga` primitive. It also does not need dependent types: constructors can validate phrase
rules and return a stated error.

## 4. Ensemble-led tuning: a Balinese gamelan pressure test

### The example

Represent one scale-degree intent for a paired register of a named ensemble. Realization chooses a lower and upper
frequency whose difference produces a target beating rate. A second register may use a different octave treatment.

This is an acoustic pressure test, not a complete model of any gamelan.

### Verified

Vitale and Sethares analyze more than 8,000 measurements from 49 gamelan gong kebyar. Paired instruments are tuned in
relation to `ombak`; octaves may be stretched or compressed; treatment varies by degree, register, ensemble, region, and
history. The paired tunings are interdependent
([Vitale and Sethares 2021](https://journal.iftawm.org/previous/vol9no2/vitale-sethares/)).

The current score-to-MIDI path maps written pitch through twelve-tone equal temperament. That is a valid Western
realization, but it cannot express an ensemble-specific paired target without first pretending it is one written pitch.

### Proposed model

The package owns:

```text
Ensemble, Degree, Register, PairRole, AcousticTarget, ExactHz, RealizedTone, TuningError
```

The private ensemble value contains measured or chosen tuning data. Its public operation is total:

```text
realize_pair : Ensemble -> Degree -> Register -> AcousticTarget
            -> Result<(RealizedTone, RealizedTone), TuningError>
```

The notation branch may print a degree, instrument, and instruction. The performance branch turns each realized tone
into a finite frequency gesture, then binds it to the correct instruments. The audio stage converts exact targets to the
floating-point representation required by the processor and records the approximation.

### What is lost or added

- A staff label can omit the exact paired tuning and the intended beating.
- A generic MIDI pitch loses the ensemble relation almost completely.
- Acoustic realization adds measured or chosen frequency targets.
- The audio processor adds numerical approximation and the spectra of actual or modeled instruments.

### Judgment

Pitch realization may depend on ensemble and acoustic intent. A global quotient from written pitch to pitch class would
erase the point of this example. The same finite-data and abstraction features used by the phrase case are enough,
provided the language also has checked exact rational arithmetic. No dependent pitch index is needed.

## 5. Interactive music: a bomba pressure test

### The example

Describe a finite vocabulary of dancer movements and lead-drum responses, a finite interaction state, and one total step
function. During a performance, the runtime repeatedly receives a movement and emits a response. The number of steps is
not known in advance.

This does not attempt to reduce bomba to a state machine. It tests whether Musa can leave live human input outside a
fixed score.

### Verified

Smithsonian Folkways describes dance as integral to bomba. A dancer moves, and the lead drummer translates the movement
into sound in an interactive dialogue
([“Los Pleneros de la 21”](https://folkways.si.edu/los-pleneros-de-la-21-afro-puerto-rican-traditions/latin/music/article/smithsonian)).

The current Musa build starts from a finite project and prepares finite score and audio artifacts. It has no typed live
input protocol. The accepted process semantics can, however, run a finite stateful graph over an unbounded input
history.

### Proposed model

A package or adapter defines:

```text
Move, Response, State, ProtocolError
step : State -> Move -> Result<(State, Response), ProtocolError>
```

Source evaluation checks the finite vocabulary, initial state, and total step function. A live host receives actual
movement events and applies `step` repeatedly. Each response can become a finite gesture and then sound. A rehearsal
notation may list cues and possible responses, but it is not the performance itself.

### What is lost or added

- A written cue sheet omits the actual dancer's choices and timing.
- A recorded performance fixes one history but does not exhaust the protocol.
- The live host adds input events and wall-clock timing.
- Social meaning, dance knowledge, and the drummer's judgment are not recovered from the event log.

### Judgment

The source language needs finite nominal data and a total step function. It does not need effects or general recursion.
Unbounded repetition belongs to the host and process runtime. Treating the live history as a finite timeline term would
be the category error.

## 6. What the five cases require

| Feature | Cases that need it | Decision |
| --- | --- | --- |
| Finite nominal data and exhaustive matching | tonal, phrase, tuning, interactive | add |
| Abstract types and private constructors | tonal, phrase, tuning, interactive | add |
| `Result<A,E>` | phrase, tuning, interactive, checked arithmetic | add |
| `Text` | names, evidence, and loss reports across all cases | add |
| Checked rational arithmetic | flexible time, tuning | add |
| Explicit loss records between stages | all five | keep and complete |
| Unequal-length overlay | flexible time | keep |
| General recursion or effects | none | reject |
| User polymorphism | none | defer |
| Dependent types or refinement indices | none | reject for this closure |
| Call-by-push-value | none | reject for this closure |
| `world` or `link` syntax | none | reject for this closure |
| A universal pitch, key, metre, or chord | contradicted by several | reject |

The cases therefore support a modest source-language extension. They do not support a new universal musical kernel.

## 7. Cross-checks that did not become a sixth case

Form, texture, timbre, and orchestration do not force another core feature.

- Motives can be rhythmic, pitched, contour-based, or timbral; identifying and segmenting them is an analytical act
  ([Open Music Theory, “Foundational Concepts for Phrase-Level Forms”](https://viva.pressbooks.pub/openmusictheory/chapter/foundational-concepts-for-phrase-level-forms/)).
  This confirms that `Motif` should be package data or an analysis result, not a kernel normal form. A split idempotent
  becomes relevant only if a named theory actually supplies and uses the two maps.
- Texture can change within a work, and different analytical traditions group simultaneous material differently.
  Common-practice labels such as homophony and polyphony do not replace pop's functional layers
  ([“Texture”](https://viva.pressbooks.pub/openmusictheory/chapter/texture/) and
  [“Texture in Pop Music”](https://viva.pressbooks.pub/openmusictheory/chapter/texture-in-pop-music/)). Generic overlay
  should therefore combine occurrences without deciding that they are voices, chords, or layers.
- Orchestration can redistribute, split, double, omit, dovetail, and recolor material. A piano-to-orchestra version is
  often a new realization of an idea rather than a field-by-field copy
  ([“Core Principles of Orchestration”](https://viva.pressbooks.pub/openmusictheory/chapter/introduction-and-core-principles/)
  and [“Transcription from Piano”](https://viva.pressbooks.pub/openmusictheory/chapter/transcription-from-piano/)). This
  supports a typed conversion with recorded choices and losses, not a generic payload map that claims to preserve
  everything.

Finite package data, named analyses, performance adapters, and derivation records can state these distinctions. What
remains untested includes lyrics and language, spatial music, studio-born work, DJ practice, live coding, embodied and
signed music, ritual and ownership, and many musical traditions not named here. Those gaps bar a claim of universality.
They do not justify adding machinery that none of the tested programs needs.
