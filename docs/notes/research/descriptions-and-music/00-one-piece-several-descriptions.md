# One piece can have several descriptions

**Status: research. This document does not set Musa's rules.**

## Purpose

This note tests one idea:

> A score, a studio setup, and an audio reference can describe the same music from different sides. A performance is one
> way the music can actually happen. None of these must come first, and none contains the whole music.

Here, a **musical event** means a passage with a beginning and end that is played, sung, or produced. It may happen on a
stage, in a studio, or through an electronic system.

The aim is not to erase the differences between notation, performance, and sound. The aim is to connect them without
turning the compiler's build order into a claim about music.

## 1. Three examples

We cannot know Bach's, Sibelius's, or Goldfrapp's private thoughts. We can study how the music was written, performed,
and made.

### 1.1 Bach and figured bass

An eighteenth-century German study of figured-bass teaching includes material from *J. S. Bach's Precepts and
Principles*. It describes exercises in which a player builds rich counterpoint from a sparse bass line. The player must
know common patterns, remember themes, and reuse them in the right places
([Gingras, abstract](https://www.cambridge.org/core/journals/eighteenth-century-music/article/abs/partimento-fugue-in-eighteenthcentury-germany-a-bridge-between-thoroughbass-lessons-and-fugal-composition/3F67B165D4141242F1F014B16FD36C14)).

Take one short bass line with figures. It does not name one keyboard performance. A player still chooses the upper
voices, spacing, rhythm, ornament, imitation, and how one measure leads to the next.

So the basic connection cannot be a function

`score -> sound`.

The score does not choose one sound. It describes a range of valid performances.

Measures also cannot be handled in isolation. Two good measures may join badly if their upper voices leap, cross, or
resolve poorly. Any useful account of a measure must include what can connect to it.

### 1.2 Sibelius's Fifth Symphony

Sibelius made three performed versions of the Fifth Symphony, in 1915, 1916, and 1919. The final version joined the
first two original movements and changed other material. The 1915 version had already been performed successfully. The
later score was not a fuller record of that performance
([Sibelius.fi account](https://sibelius.klubi.fi/english/musiikki/ork_sinf_05.htm)).

The National Library of Finland describes the larger pattern. Sibelius's works often survive as sketches, fair copies,
parts, proofs, and printed editions. He sometimes changed details during publication, and some changes were tied to
early performances
([National Library of Finland](https://www.kansalliskirjasto.fi/en/jean-sibelius-works/text-critical-approach-sibeliuss-works)).

This example gives us two warnings.

First, a revision does not always make the old description more specific. Sibelius removed, replaced, and reorganized
music. The 1919 score allows a different range of performances from the 1915 score, not just a smaller range.

Second, making music need not move in one direction. A composer can write, hear, revise, hear again, and rewrite. A
printed score can also contain a copying or engraving error. Musa may choose one source file as the file users edit, but
that is a software rule, not a fact about music.

The six final chords show the same point on a smaller scale. The score fixes their pitch, orchestration, and written
placement. It does not fix their exact length in seconds, their balance, or the sound of the hall. A performance adds
those facts.

### 1.3 Goldfrapp's “Ooh La La”

Will Gregory says that “Ooh La La” is driven by the sound of its bass synthesizer. He also says that, in work of this
kind, composition, arrangement, demo, and finished record may develop together
([MusicTech interview](https://musictech.com/features/goldfrapp-music-tech-interview/)).

One accident became part of the track. An open microphone captured the clatter of the keyboard keys along with the bass
line. Goldfrapp kept that sound in the record
([Goldfrapp interview](https://www.theguardian.com/culture/2026/jan/26/sex-scenes-how-goldfrapp-made-ooh-la-la-baudelaire)).

A staff transcription or MIDI file can preserve the repeated pitch and rhythm while losing the sound that drove the
song. The synthesizer, recording chain, key clatter, and mix are not decorations added to an already complete score.
They help define the result.

The same audio clip can also serve two purposes. As a recording, it tells us something about a past studio event. As a
sample or playback source, it helps make a later event. The data stay the same. The operation changes from recording to
playback.

Notation and audio therefore cannot be separate kinds of musical content. They also cannot be the same thing. A written
bass line and a captured key clatter can both shape one future performance while remaining different facts.

## 2. A precise proposal

Fix a **setting** `S` and a time span `I`. The setting lists the choices needed to judge the example: performers,
instruments, style, tuning, room, studio, or anything else that matters. It is an ordinary input, never hidden global
state or an unstated setting.

**Definition 2.1 (musical event).** `Event_S(I)` is the set of possible musical events in setting `S` during time span
`I`.

We do not define a musical event as a record with separate notation, gesture, and audio fields. A setting provides only
the ways of inspecting an event that it needs. It may let us hear the event, inspect a player's action, or take an
excerpt.

**Definition 2.2 (description).** `Description_S(I)` is the set of finite descriptions of events in `Event_S(I)`.

A description may contain notation, a studio setup, a verbal instruction, a live rule, an audio reference, or several of
these together.

**Definition 2.3 (match record).** For a description `d` and an event `e`, `Match_S(d,e)` is the set of records that
explain why `e` matches `d` in setting `S`.

We say that `e` matches `d` when `Match_S(d,e)` contains at least one record. A record may contain the chosen continuo
voices, a conductor's timing choice, a studio connection, an allowed error, or a source location.

The events that match `d` are

`Fits_S(d) = {e in Event_S(I) | Match_S(d,e) contains a record}`.

### Why keep a record?

A yes-or-no answer loses the reason. Two players may produce the same sound through different choices. An editor,
performer, or analysis tool may need those choices later.

A match record does not turn taste into a fixed rule. It only makes the grounds for the judgment clear.

## 3. Rules that follow from the proposal

The proposal gives us a few useful rules. It does not make chords, keys, scales, instruments, or meters part of the core
language.

### 3.1 A more specific description keeps the old promise

Say that `d2` is **more specific** than `d1` when every record showing that an event matches `d2` can be turned into a
record showing that the same event matches `d1`:

`less_detail : Match_S(d2,e) -> Match_S(d1,e)` for every event `e`.

For example, “play this bass line on organ with these upper voices” is more specific than “play this bass line.”

**Proposition 3.1.** If `d2` is more specific than `d1`, then every event that fits `d2` also fits `d1`.

**Proof.** Choose an event `e` that fits `d2`. It has a record in `Match_S(d2,e)`. The `less_detail` function turns that
record into one in `Match_S(d1,e)`. Therefore `e` also fits `d1`. ∎

The reverse claim need not hold. Two descriptions can allow the same events while preserving different reasons.

This is why revision needs a separate name. A revision may remove an old requirement, add a new one, or do both. It is
not called “more specific” unless the map above exists.

### 3.2 Two descriptions can both apply

Write `both(d1,d2)` for the description that requires both `d1` and `d2`. Its match record contains one record for each
requirement:

`Match_S(both(d1,d2),e) = Match_S(d1,e) × Match_S(d2,e)`.

**Proposition 3.2.** An event fits `both(d1,d2)` exactly when it fits both `d1` and `d2`.

**Proof.** A pair exists exactly when its first and second parts both exist. ∎

This is how notation and audio fit together. The same event can be required to match a staff description and a sound
description:

`both(bach_line, continuo_style)`

or

`both(bass_pattern, both(synth_sound, key_clatter))`.

The descriptions remain different. They simply apply to the same event.

An explicit choice can keep track of which option was used:

`Match_S(either(d1,d2),e) = Either(Match_S(d1,e), Match_S(d2,e))`.

Musa needs this only if an alternate passage, mobile form, or live choice must remain visible.

### 3.3 Descriptions and examples check each other

Let `A` be a set of descriptions. Let `X` be a set of musical events. Define

`fits_all(A) = {e | e matches every description in A}`,

and

`true_of_all(X) = {d | every event in X matches d}`.

**Proposition 3.3.** Every description in `A` is true of every event in `X` if and only if every event in `X` fits all
the descriptions in `A`.

**Proof.** Both sides say the same thing: each description in `A` matches each event in `X`. ∎

This plain fact lets information move both ways.

- Start from descriptions. `true_of_all(fits_all(A))` finds everything else those descriptions force.
- Start from performances or recordings. `fits_all(true_of_all(X))` finds every event that the current description
  language cannot tell apart from them.

Neither trip must return exactly where it began. A language may be too weak to describe one performance exactly. A
recording may leave out gesture, intention, room position, or notation.

### 3.4 The whole connection is one table

Make one table. Each row contains:

1. a description `d`;
2. a musical event `e`; and
3. a match record `m` from `Match_S(d,e)`.

Looking up all rows with the same `d` gives every known event that matches that description. Looking up all rows with
the same `e` gives every known description that the event matches:

`descriptions <- match records -> musical events`.

This is the full connection proposed here. Musa does not need public `world` or `link` features for it. Notation and
audio both provide descriptions on the left. The played or produced music is on the right.

A renderer adds a choice: for each description it supports, it returns one event and a record showing the match. A
transcriber makes the opposite choice: from an event, it returns a description and a match record. Neither operation
must work for every input.

### 3.5 You can take an excerpt; joining excerpts needs shared state

If `J` is a smaller time span inside `I`, we can take the part of an event that occurs during `J`. Local descriptions
and their match records can be cut down in the same way.

Small excerpts do not always determine the whole performance. Phrase shape, resonance, and memory can cross the cut.
Taking an excerpt is therefore a general operation. Rebuilding the whole from excerpts is not.

Joining two excerpts needs a record that their shared state agrees. That record may contain:

- the upper voices at a measure boundary;
- filter, delay, and resonator state in an audio program; or
- the current state of a live musical exchange.

Sequence then means:

1. make the left excerpt;
2. make the right excerpt;
3. check that their shared state agrees; and
4. join them.

Equal written length does not solve this problem. Shared state does.

Parts sounding together are also not assumed to be independent. Singers, paired instruments, and synth voices can affect
one another through tuning, balance, electronics, and the room. Their descriptions apply to one combined event.

## 4. Audio belongs at the beginning

Audio has two clear uses.

**Recording.** A recording function observes an event:

`record : Event_S(I) -> AudioClip`.

Recording loses information. The same samples may fit different gestures, intentions, instrument states, or studio
connections.

**Playing.** A playback function uses a clip to make a new event:

`play : AudioClip × PlaybackSetting -> Result(Event_S(I), PlaybackError)`.

A live audio program works one block at a time:

`run_block : AudioProgram × InputBlock × SavedState -> OutputBlock × SavedState`.

The recording and playback functions point in opposite directions and obey different rules. The clip itself can still be
one ordinary value.

An `AudioProgram` is finite source data. It can describe a system that runs without a fixed end. Each step still reads a
finite input block, writes a finite output block, and returns the state needed for the next step.

This is the distinction Musa needs:

- an endless sample stream is not a finite list of written events;
- a finite program that produces sound can still be part of the core language.

In the Goldfrapp example, one description can combine the pitch pattern, bass-synth program, recording setup, and sound
target. Audio is not an export added after the music is complete.

## 5. Five checks

| Test | Result | Lesson |
| --- | --- | --- |
| No instruction | Passes | An empty description allows every event in the chosen setting and time span. |
| One figured-bass measure | Passes | One description can allow many performances and keep the reason for each. |
| Two measures in sequence | Passes with a condition | The measures need matching voice and connection state. Equal length is not enough. |
| Sibelius revises after hearing a performance | Passes | A revision may replace old requirements instead of only adding detail. |
| A studio record is performed live | Passes with an open choice | The song's package must say which changes still count as a match. |

The last test exposes a real open question. A live performance of a studio record is rarely identical to the master.
Listeners may still accept it as the same song. Musa's core cannot choose that rule. A song or style package must state
the allowed difference and save the result in the match record.

## 6. What this proposal explains

The proposal explains why:

- one score can allow many performances;
- one recording can support several transcriptions;
- adding a requirement narrows the allowed events;
- notation and audio requirements can apply to the same event;
- a renderer chooses one allowed event rather than revealing the only correct one;
- joining passages needs shared state; and
- a finite audio program can describe a run with no fixed end.

The proposal does not define chords, keys, harmonic function, scale degree, rāga, meter, instruments, or tuning. Music
packages define those ideas. It also does not decide whether two performances are artistically equal, make transcription
automatic, or prove that a piece has one final description.

The earlier proposal tried to describe all of music through named connections. It failed because every musical idea
became an unexplained yes-or-no test. This proposal is narrower. It explains how descriptions and performed sound
relate. It does not pretend to explain all of music.

## 7. What this changes for Musa

Musa should not treat a finite event list as the basic shape of all music and place audio outside it. A placed event
list and an audio program are two finite ways to describe or make the same possible musical event.

The central ideas now have plain meanings:

| Name | Meaning |
| --- | --- |
| `Description` | Finite information about how music may be played or produced |
| `Match` | A record showing why one event fits one description |
| `TimeSpan` | The part with a beginning and end that we are discussing |
| `AudioClip` | Finite sampled sound that can be recorded or played |
| `AudioProgram` | A finite program that produces sound one block at a time |

These are working names, not chosen source syntax.

The current `Timeline` type still has a use. It stores facts placed at exact positions in one chosen clock. A clearer
public name may be `PlacedFacts` or `EventPlan`. Its length should use a name that says what it measures: written
duration, performed duration, seconds, or sample frames.

The phrase “process graph” describes one compiler data structure. It should not be the public musical name. The public
idea is an `AudioProgram`. The compiler may store that program as a graph internally.

Musa can still use a small language in which every ordinary function finishes. That language must be able to build
descriptions, audio programs, and match records. Running an audio program is a separate action that advances one block
at a time.

These examples do not require public `world` or `link` features. They also do not require a large type system. Musa may
later record time spans, sample rates, or channel counts in types if doing so removes real errors without making the
language harder to understand.

## 8. Decision and next test

This proposal is better than the current one-way account. It lets work begin from notation, live playing, studio work,
or sound. It keeps their differences while allowing them to describe the same music.

The examples support three core ideas:

1. match records connect descriptions to musical events;
2. descriptions can become more specific, apply together, or offer a visible choice; and
3. excerpts can be taken freely, while joining them needs shared state.

This remains research. It is not ready to replace Musa's current rules.

The next test is to write the smallest typed language that can express these three ideas and finite audio programs. The
same Bach, Sibelius, and Goldfrapp examples must work without separate notation, performance, and audio languages. If
each example needs a new built-in rule or an unexplained yes-or-no test, this proposal has failed.
