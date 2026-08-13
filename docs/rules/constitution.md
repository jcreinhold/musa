# What Musa must preserve

This document states the project’s basic design choices. A **representation** is any form in which Musa holds part of a
project: source text, an event track, an engraved score, an analysis, a MIDI file, a machine description, or recorded
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

## 3. An event track records exact positions and lengths

Musa represents a finite stretch of musical time with an **event track**:

- a nonnegative exact rational length, such as `7/2` beats; and
- a finite multiset of occurrences, each with an exact start, end, and payload value.

An occurrence with a positive span occupies `[start, end)`: it includes its start instant and excludes its end. An
occurrence whose start equals its end is a point at that instant.

Putting one track after another adds their lengths and moves the second forward. That operation is called **follow**.
Placing two tracks in the same region keeps every occurrence of both and takes the longer length. That operation is
called **together**.

Suppose one part lasts eight beats and another enters at beat two and leaves at beat six. They may be placed together
directly. Musa does not insert four beats of rests to make their lengths equal. Empty time is simply time in which no
occurrence of the relevant kind exists. A written rest is still a real notation event when the score calls for one.

**Every track states whose time it measures.** A track's positions are tagged by a **coordinate**: written time,
performed time, or physical seconds. Two tracks in different coordinates do not combine, because adding a written beat
to a physical second is a mistake a type check should catch rather than a conversion the compiler should guess. Tempo,
swing, rubato, fermatas, and unmeasured passages are named conversions between coordinates, not properties of the track.

## 4. A machine is a finite description; the sound it produces is not

A **machine** is a finite description built from registered primitive units and fixed wiring: identity, chain
connection, side-by-side connection, initialized one-step feedback, copy, drop, and swap. It has a private state, a
start state, and one total deterministic **step**. The description, its changing state, and the history of outputs it
produces are three different things.

Running a machine for `n` steps yields `n` outputs. Running it forever yields no value at all. The description is
finite, so it can be a source value; the history is open-ended, so it cannot.

**One audio step is one sample frame.** Frame-by-frame stepping is the reference meaning of an audio machine. A host
device that asks for a block of frames at once may be served by a batch method only when that method provably produces
the same state and the same outputs as repeated frame steps. Callback size is a performance decision; it is never a
musical or audio meaning.

Feedback reads a stored value from the previous step and takes an explicit initial value, so the first step has an
answer and no loop asks for its own current output.

Two reasons keep the machine and its history apart, and both are worth stating because the separation is otherwise easy
to read as mere tidiness.

The first is about how each is defined. An output history is defined by what it produces at each step and has no last
element; an event track is a length plus a *finite* multiset of occurrences, and the guarantee that processing it
terminates is derived from that finiteness. Putting a history inside an event track would require either that the
history be finite, in which case it is not a history, or that the track give up finiteness, in which case every rule
that depends on processing terminating is gone.

The second is musical. A machine has no length. It names units and connections; asking how long it lasts is the same
kind of question as asking how long a mixing desk lasts. Every event-track rule is stated over a length, so something
with no length cannot be the payload of an occurrence.

What this decision no longer says, and deliberately: the machine is **not** outside the core language. A machine value
is built, typed, and checked by the same source language that builds event tracks, and the two meet through the checked
scheduler of §5. Treating the studio as a second calculus with its own semantics is the drift this section now exists to
prevent.

This decision reopens on either of two events and nothing else. **One:** someone proposes a written musa construct whose
meaning is genuinely an open-ended history *with* a notated length — something that sounds continuously and that other
music is positioned against. Live coding and reactive input are the likely sources. **Two:** the rule in §"Audio
preparation receives every choice as an argument" of the obligations is measured false — equal complete arguments
produce different prepared results, meaning the preparation step is reading state nobody passed it. Deferring without
one of these triggers is not permitted.

## 5. Scheduling is the named connection from tracks to machines

An event track says what occurs when, in a musical coordinate. A machine says what happens one step at a time. One
checked operation connects them:

```text
schedule(audio format, schedule policy, time map, event track)
    -> a running event source plus a record of every time decision, or an error
```

The time map says how source positions become exact physical times. The policy says how exact physical times become
bounded integer frame numbers, how messages landing on one frame are ordered, and which collapses are permitted. Both
are ordinary checked data supplied by the caller; neither is ambient state the scheduler reads later.

Scheduling either records every rounding, collision, and ordering choice it made, or it fails with a stated error. It
never calls two approximate values definitionally equal, and it never silently drops or moves an occurrence.

Scheduling does not identify notation with gesture or gesture with sound. It preserves what it can prove it preserves —
simultaneous placement, under a policy that decides each occurrence on its own — and says so explicitly where it cannot,
which is succession under a nonlinear time map or a non-additive rounding rule. A false general law here would be worse
than no law.

## 6. Representations stay connected by recorded conversions

Notation, analysis, performance gestures, MIDI, and machines do not share one data model. They stay coherent because
Musa records how one was made from another.

For example, a source note may produce an event-track occurrence, an MEI element, a performance gesture, and several
MIDI events. The conversion record says which source and intermediate events led to each result. It also records losses:
MIDI may preserve onset and approximate pitch while losing spelling, engraving, or tuning detail.

The project-wide object is therefore a set of representations joined by named conversion records. Musa does not need a
single “universal music object” that identifies all of them. A conversion may be reversible, but that must be proved for
that conversion; it is never assumed.

## 7. Each stored form states what “the same” means

Two values may count as equal for one job and different for another. Two score tracks can have the same occurrences even
if they were built by different source expressions. Two origin records may still differ because they came from different
places. Two machine runs may use the same description but receive different live input.

Each stored format must state:

- which fields determine equality;
- which fields are deliberately ignored;
- how those fields are encoded; and
- which version of that rule is in use.

Human-readable display text is not a safe identity format. A hash is also not proof of equality: two different byte
strings can have the same finite hash. Hashes may find likely matches, but code whose correctness depends on equality
must compare the full recorded values after the lookup.

Machines have two equalities and Musa decides only one of them. **Structural** equality compares primitive ids,
versions, configurations, and the exact wiring tree within one build's registry; Musa may decide it. **Behavioural**
equality asks whether two machines produce the same output for every input history; Musa may state it as a mathematical
relation and prove particular instances of it, but no type check and no cache may attempt to decide it.

## 8. One event-track structure serves every payload

The event track of §3 does not know what kind of thing its occurrences hold. Notation facts are one payload. Performance
gestures are another. Both are the same structure at a different payload, and both get the same lengths, the same
combination rules, and the same notion of equality without any of it being proved twice.

This is a decision about what the structure is a calculus *of*, and it has three consequences that a later change may
not quietly undo.

**A second temporal container is a defect.** If something has a length and finitely many things positioned inside it, it
is an event track. Writing a second one — a list of start/end/value triples with its own ordering and its own idea of
equality — is exactly the drift this decision exists to prevent, and it is audited for.

**The structure knows nothing musical.** A pitch, a part, a voice, and a gesture are defined by the layers above and
merely satisfy what the track asks of a payload. The event-track layer never depends on a musical type, in the same
direction that already holds for notation facts.

**Occurrences are not indexed by their musical role.** It is tempting to make part, voice, metre, tuning, and
transposition part of an occurrence's *type*, so that pairing the wrong two is rejected before it runs. Musa does not,
and the reason is that the decision is one-way. Once those indices are load-bearing, every rule, every proof, and every
consumer is written in terms of them, and they cannot be removed by deletion: operations that were only meaningful under
an index have no meaning without it, and callers that relied on an index to guarantee something must guarantee it
another way. Musa has one real motivation for indexing — that a wrong part-and-voice pairing should be caught early —
and that is already served by name resolution, at the cost of one diagnostic. Paying an irreversible rewrite for it is
not a decision this project has the evidence to make.

The one index Musa *does* carry is the time coordinate of §3, and it is carried because it is the one case where the
mistake has no diagnostic elsewhere: nothing about a written beat and a physical second looks different until the sound
is wrong.

## 9. One total source language builds both core values

There is one source language. It is pure, strict, total, and its types are inferred. It builds ordinary values, event
tracks, and machines; there is no second language for the studio and no contextual value that means something different
depending on where it is used.

Five properties are load-bearing, and each of them is a refusal:

- **Total.** There is no general recursion and no partial call. Every accepted program finishes. A resource budget may
  stop an evaluation, but it may not change the value an accepted program produces.
- **Inferred.** Types are rank-1 Hindley–Milner. Annotations are written where a public signature or separate checking
  needs one, not to teach the compiler what the program already determines.
- **Complete.** A call supplies every argument. A function of several parameters takes one product argument. Partial
  application, default parameters, and named hole filling are not features whose absence needs a workaround; they are
  ambiguity about what a call means.
- **Split into values and storable data.** Every type is a value type. A type is also **storable data** when it contains
  no source function at any depth and has a versioned, finite, exact encoding. Only storable data may be an occurrence
  payload, a machine port, a feedback value, a primitive configuration, or an argument to a foreign primitive. A source
  function may be used during evaluation and may never be stored inside a value that outlives it.
- **Closed at the machine boundary.** There is no public operation that lifts an arbitrary source function into a
  running machine. A unit that runs in the audio path is a registered primitive with a stated state, step, and resource
  contract, not a closure the source handed over.

What this rules out is as important as what it admits: no universal contextual `Music` value, no dependent or refinement
types, no call-by-push-value stratification, no first-class signals or streams, no type-directed macros, and no built-in
notes, chords, keys, metres, instruments, or cultural theories. Each of those may be proposed again, and each must then
meet the standard in the obligations: remove a real side condition in at least two different musical uses, or close a
safety boundary the current rules cannot state.
