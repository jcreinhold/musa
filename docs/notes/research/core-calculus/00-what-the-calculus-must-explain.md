# What the calculus must explain

**Status: research. This document does not set Musa's rules.**

## Purpose

This note fixes the problem before choosing the language.

Musa needs a small total programming language. A source program finishes. Its result may include a finite description of
a system that keeps running after compilation, just as a finite circuit diagram can describe a radio that keeps playing.

The core must make the path from a written note to a sound explicit. It must not claim that the written note and the
sound are the same thing.

## 1. The earlier false start

The note [One piece can have several descriptions](../descriptions-and-music/00-one-piece-several-descriptions.md)
introduced `Description`, `Event`, and `Match`. That can describe comparison work: whether a performance fits a score,
or whether a recording matches a reference. It does not tell us how to build a chord, schedule a note, connect a
synthesizer to an effect, or run the result.

That proposal therefore answers the wrong question. It may remain useful for testing or analysis. It is rejected as the
core calculus.

## 2. Six tests

The core must pass all six tests below.

### 2.1 A written passage

A finite written passage needs exact placement, succession, simultaneous material, repetition, and transformation.
Written rests, ties, articulations, key signatures, and note spellings must remain data. They cannot be inferred from an
empty interval or an audio waveform.

The Open Music Theory rhythm chapter makes the distinction concrete: a tie joins written notes and tells the performer
not to attack the later note again. One long sounding event and two tied written notes can therefore sound alike while
remaining different notation (`open-music-theory/009-notating-rhythm.md`).

### 2.2 Parts that enter, leave, and overlap

Music does not consist of equal-length voices padded with rests. Heterophony places variants of one melody together;
polyphony gives voices separate rhythms; texture can change during a piece (`open-music-theory/008-texture.md`).

The core must let one part begin late or end early. Missing material, a written rest, and audible silence are three
different facts.

### 2.3 Time without one meter

Meter must not be built into placement. Ametric music may have written durations without a perceived meter. A written
meter may differ from the meter a listener hears. Polymeter places several meters together. Timeline notation may use
seconds instead of bars (`open-music-theory/098-twentieth-century-rhythmic-techniques.md`).

The core therefore needs exact local positions. Meter, swing, rubato, tempo, and perceived grouping belong in data and
checked conversions.

### 2.4 Timbre as musical material

Orchestration combines material both at once and in succession. Attack and sustain may be split across instruments.
Timbre can mark a cadence, join sections, or carry a melody
(`open-music-theory/114-core-principles-of-orchestration.md`, `open-music-theory/115-subtle-color-changes.md`).

A core that stops at pitch and duration fails this test. It must be able to name an instrument or audio system and
connect that system to timed musical actions.

### 2.5 Audio made from a score or from the studio

A notation-led path must be possible:

```text
written facts
-> performance choices
-> timed actions
-> instrument and effects
-> audio frames
```

The reverse and sideways paths must also be possible. A microphone may be an input. A recording may be analyzed. A
synthesizer sound may lead the composition, as it did in the documented account of Goldfrapp's “Ooh La La.” The core
must not require every piece to begin with a score.

### 2.6 A live run

A live system may run until a person stops it. The source that describes the system must still be finite. Every finite
prefix of the run must be computable from a finite prefix of its inputs. A feedback loop must contain stored state, so
the present output never depends on itself before a value exists.

## 3. Three kinds of operation

Watching the examples reveals three operations that earlier drafts blurred together.

1. A function computes one finite value from another.
2. Putting one passage after another moves the second passage later in time.
3. Connecting two audio units sends each output step of the first into the next unit at the same step.

These operations have different laws. One symbol must not pretend they are the same.

Putting two passages together is also different from mixing audio. The first places two sets of facts in one time span.
The second adds or otherwise combines sample values. A mixer is an audio unit, not a generic wiring rule.

## 4. What should remain ordinary library code

The core should not define notes, chords, keys, scales, rāgas, tuning systems, meters, instruments, or harmonic
functions. Those are data types and functions in libraries.

The core should supply only what many such libraries need:

- a total functional language with inferred types;
- a finite collection of placed events; and
- a finite description of a typed state machine that can run one step at a time.

The next two drafts test whether even that much is necessary.

## 5. Evidence labels

- **Verified** means the cited file or implementation was read.
- **Derived** means the claim follows from a stated definition.
- **Judged** means it is a design choice that the evidence does not force.

Every review uses those labels.
