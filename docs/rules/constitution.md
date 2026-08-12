# What Musa must preserve

This document states the project’s basic design choices. A **representation** is any form in which Musa holds part of a
project: source text, an internal timeline, an engraved score, an analysis, a MIDI file, an audio plan, or rendered
sound.

## 1. Musa source is the master record

Users edit `.musa` source. Every other representation is made from that source and its locked package inputs.

For example, the score editor may look like a graphical notation program, but moving a note changes the source. The
editor does not keep a second score database that can disagree with the file. Engraving, analysis, MIDI, and audio may
be cached, but Musa must be able to rebuild them from the source and the recorded choices used for that build.

This choice requires derived results to keep enough origin information for a user to move from a note, warning, or
waveform back to the source that produced it. It does not require every result to be rebuilt eagerly or stored forever.

## 2. Musa does not impose one theory of music

No single definition of pitch, metre, chord, key, phrase, or gesture fits all music. Musa will ship useful definitions
for Western staff notation and common tonal practice, but those definitions are library code and compiler services, not
the definition of music itself.

A theory package defines its own values, operations, and test for equality. One package might use spelled notes and
keys. Another might use tuning ratios and characteristic melodic motion. Another might organize music by gesture,
timbre, spoken cues, or cycles rather than by chords. A project may use more than one package, but it must call a named
conversion when it moves between them.

This choice leaves the first package system and the final standard-library contents open. It rules out making every
piece declare a key, a 12-note pitch class, or a regular metre.

## 3. A finite timeline records exact positions and durations

Musa represents a finite stretch of musical time with:

- a nonnegative rational length, such as `7/2` beats; and
- a finite collection of events, each with an exact start, end, and value.

Putting two timelines one after the other adds their lengths and moves the second timeline forward. Playing two
timelines together keeps every event and uses the longer length.

Suppose one part lasts eight beats and another enters at beat two and leaves at beat six. They may be overlaid directly.
Musa does not insert four beats of rests to make their lengths equal. Empty time is simply time in which no event of the
relevant kind occurs. A written rest is still a real notation event when the score calls for one.

This timeline contains musical positions, not seconds or sample frames. Tempo, swing, rubato, fermatas, and unmeasured
passages are interpreted later by named performance rules.

## 4. An audio graph is a recipe, not a timeline

A studio graph is a finite recipe: it names processors, typed connections, state, and delays. Running that recipe can
produce an arbitrarily long stream of samples. The recipe, its changing state, and the sample stream are three different
things.

The audio graph therefore has its own rules. A processor runs only after all of its current inputs are ready. A feedback
loop must pass through an explicit stored value or delay, so the loop reads an earlier result rather than asking for its
own current result.

Musical timelines do not contain sample streams. Preparing audio is the explicit step that chooses sample rate,
channels, instruments, processor settings, and initial state.

Two independent reasons keep them apart, and both are worth stating because the separation is otherwise easy to read as
mere tidiness.

The first is about how each is defined. A sample stream is defined by what it produces at each observation and has no
last sample; a timeline is a length plus a *finite* collection of events, and the guarantee that processing it always
terminates is derived from that finiteness. Putting a stream inside a timeline event would require either that the
stream be finite, in which case it is not a stream, or that the timeline give up finiteness, in which case every rule
that depends on processing terminating is gone.

The second is musical. A studio graph has no length. It names processors and connections; asking how long it lasts is
the same kind of question as asking how long a mixing desk lasts. Every timeline rule is stated over a length, so
something with no length cannot be the value of a timeline event.

This decision reopens on either of two events and nothing else. **One:** someone proposes a written musa construct whose
meaning is genuinely a signal *with* a notated length — something that sounds continuously and that other music is
positioned against. Live coding and reactive input are the likely sources. **Two:** the rule in §"Audio preparation
receives every choice as an argument" of the obligations is measured false — equal complete arguments produce different
prepared results, meaning the preparation step is reading state nobody passed it. Deferring without one of these
triggers is not permitted.

## 5. Representations stay connected by recorded conversions

Notation, analysis, performance gestures, MIDI, and audio do not share one data model. They stay coherent because Musa
records how one was made from another.

For example, a source note may produce an internal score event, an MEI element, a performance gesture, and several MIDI
events. The conversion record says which source and intermediate events led to each result. It also records losses: MIDI
may preserve onset and approximate pitch while losing spelling, engraving, or tuning detail.

The project-wide object is therefore a set of representations joined by named conversion records. Musa does not need a
single “universal music object” that identifies all of them. A conversion may be reversible, but that must be proved for
that conversion; it is never assumed.

## 6. Each stored form states what “the same” means

Two values may count as equal for one job and different for another. Two score timelines can have the same musical
events even if they were built by different source expressions. Two origin records may still differ because they came
from different places. Two audio runs may use the same plan but receive different live input.

Each stored format must state:

- which fields determine equality;
- which fields are deliberately ignored;
- how those fields are encoded; and
- which version of that rule is in use.

Human-readable display text is not a safe identity format. A hash is also not proof of equality: two different byte
strings can have the same finite hash. Hashes may find likely matches, but code whose correctness depends on equality
must compare the full recorded values after the lookup.

## 7. One timeline structure serves every kind of event

The timeline of §3 does not know what kind of thing its events hold. Notation events are one kind. Performance gestures
are another. Both are the same structure at a different kind of value, and both get the same lengths, the same
combination rules, and the same notion of equality without any of it being proved twice.

This is a decision about what the structure is a calculus *of*, and it has three consequences that a later change may
not quietly undo.

**A second temporal container is a defect.** If something has a length and finitely many things positioned inside it, it
is a timeline. Writing a second one — a list of start/end/value triples with its own ordering and its own idea of
equality — is exactly the drift this decision exists to prevent, and it is audited for.

**The structure knows nothing musical.** A pitch, a part, a voice, and a gesture are defined by the layers above and
merely satisfy what the timeline asks of a value. The timeline layer never depends on a musical type, in the same
direction that already holds for notation events.

**Events are not indexed by their musical role.** It is tempting to make part, voice, metre, tuning, and transposition
part of an event's *type*, so that pairing the wrong two is rejected before it runs. Musa does not, and the reason is
that the decision is one-way. Once those indices are load-bearing, every rule, every proof, and every consumer is
written in terms of them, and they cannot be removed by deletion: operations that were only meaningful under an index
have no meaning without it, and callers that relied on an index to guarantee something must guarantee it another way.
Musa has one real motivation for indexing — that a wrong part-and-voice pairing should be caught early — and that is
already served by name resolution, at the cost of one diagnostic. Paying an irreversible rewrite for it is not a
decision this project has the evidence to make.
