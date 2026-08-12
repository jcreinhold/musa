# From source to sound

**Purpose:** state exactly how the proposed source language reaches notation, analysis, performance, and audio.

Musa does not need one value that is at once source, score, gesture, and sound. It needs typed conversions between those
different things, with a record of what each conversion kept, added, or lost.

## 1. A small example

Suppose a source program asks for a written C4 held for one quarter note. The compiler can produce a finite score event.
A performer may then begin it late, bend its pitch, and release it early. A synthesizer may turn that gesture into
thousands of samples.

Those objects are not equal:

- source text says what the author wrote;
- a score fact says what notation should show;
- a gesture says what a performer should do over time;
- a process graph says how audio state changes at each step; and
- an audio history records the samples produced so far.

Musa connects them with conversions. Calling them one universal object would erase the choices between them.

## 2. The common result of a finite stage

A successful finite stage returns:

```text
PassResult<A> = {
  output: A,
  derivations: List<DerivationPath>,
  losses: List<LossRecord>
}
```

A `DerivationPath` links an output item to the input items and rule that made it. A `LossRecord` states information that
the output cannot recover. For example, staff notation may record the main pitch path of a gamaka while losing its
continuous curve.

Each pass has a versioned rule set. A result is valid only when every path names an input, output, and rule admitted by
that set. Empty derivation lists are allowed only for explicitly generated facts, whose record names the pass and
generation site.

The accepted cross-stage specification already gives these records their formal composition law. This document uses that
law; it does not replace it.

## 3. The eight boundaries

### 3.1 Source to resolved, typed core

**Input.** Exact source files, one finite resolved import graph, and compiler version.

**Output.** A closed core program, a public module environment, a private data environment, source spans, and fresh
build-local type names.

**Errors.** Missing or ambiguous names, import errors, duplicate declarations, cyclic data or value definitions, type
errors, hidden-constructor use, and non-exhaustive matches.

**Equality.** Two outputs count as the same checked program when consistent renaming of fresh type names makes their
core terms, environments, and source links equal. This equality is local to the build. It is not a compiled-file
identity.

**Record.** Each core node points to the source span and elaboration rule that made it. Generated nodes also name their
generation site.

### 3.2 Core evaluation to values and `Music`

**Input.** A closed, well-typed core program and a resource budget.

**Output.** Its exported finite values. Some values may have type `Music`.

**Errors.** Resource exhaustion is a compiler error. Checked arithmetic and other expected failures are ordinary `Err`
values.

**Equality.** The evaluator is deterministic: the same checked term, budget, and compiler version produces the same
value, charge trace, and diagnostics. The language does not claim a decidable equality for functions or `Music`.

**Record.** Value records link results to definitions, arguments, and reduction rules. Evaluation does not discard
source ancestry.

### 3.3 Context application to a closed temporal term

**Input.** A `Music` value and an explicit `NotationContext`.

**Output.** Either a stated error or a closed `Term<ScoreFact>` in the temporal kernel.

**Errors.** Unsupported notation requests, missing open ports, invalid exact placements, or adapter-specific limits.

**Equality.** The output uses the temporal kernel's semantic equality. The compiler does not claim that two arbitrary
`Music` values are decidably equal.

**Record.** Every score fact points back to the recipe request and source facts that produced it. The record also names
facts supplied by the explicit notation context.

### 3.4 Temporal evaluation to a finite timeline

**Input.** A closed `Term<A>` for an admitted payload type `A`.

**Output.** A finite `Timeline<A>`. Each occurrence has an exact rational start time, duration, and payload. Overlay
takes the greater extent; a shorter voice does not need padding.

**Errors.** A well-typed closed term does not fail. Invalid rational bounds are rejected before this boundary.

**Equality.** Timeline equality is the accepted normalized semantic equality: the same extent and the same normalized
occurrences.

**Record.** Each occurrence keeps the derivation path of its payload and the temporal operations that placed it.

### 3.5 Timeline to notation and analysis

This boundary branches. Notation and analysis are peers, not the same pass.

**Notation input and output.** A `Timeline<ScoreFact>` plus notation options becomes a finite `NotationPlan`, then MEI,
LilyPond, MusicXML, MIDI, or a visual score.

**Notation errors.** Unsupported target features and explicit lowering losses.

**Notation equality.** Semantic equality belongs to the target adapter. Byte equality is stronger and is used only where
that adapter promises stable bytes.

**Analysis input and output.** A finite timeline plus an analysis method and its explicit context becomes finite claims
with evidence. The method owns the meaning of labels such as harmonic function.

**Analysis errors.** Insufficient evidence, an inapplicable method, or an explicit `Unknown` result. Analysis
uncertainty is not a type error.

**Analysis equality.** The selected method defines equality of its claims and evidence. Musa has no universal equality
for all analyses.

**Records.** Both branches cite the occurrences they used. A notation loss or an analysis assumption appears in the
result, not in hidden prose.

### 3.6 Musical intent to finite performance gestures

**Input.** Finite musical intent, an explicit performance context, and a versioned interpretation method. Intent may
come from a score, a phrase-led package, an ensemble-tuning package, live protocol output, or another typed source.

**Output.** A finite `Timeline<Gesture>` and its derivation and loss records. A gesture describes finite performed
control: for example, onset time, continuous pitch path, force, or pedal motion.

**Errors.** Missing performer or instrument capabilities, impossible timing, or a method-specific interpretation
failure.

**Equality.** Gesture equality is exact equality in the versioned gesture schema. Two different interpretations of one
score need not be equal.

**Record.** Every gesture names the intent, context facts, and interpretation rule that made it. A phrase may go
directly to a gesture without first becoming staff notation.

### 3.7 Gestures and studio choices to a prepared process graph

**Input.** The complete gesture timeline, instrument bindings, seed, render options, and processor versions.

**Output.** A complete `PreparedExecution`: a finite process graph, initial state, schedule, buffers, output ports,
derivations, losses, and all data needed to run it.

**Errors.** Missing bindings, incompatible channels or rates, invalid graph wiring, resource limits, and processor
preparation failures.

**Equality.** The accepted `R1` law applies to complete semantic inputs and a complete result. Equality is not inferred
from a hash. A hash may only find a candidate for an exact comparison.

**Record.** The result links processors and control events to their gestures, bindings, and preparation rules. The
private graph uses whole-node step semantics with explicit delayed state at feedback boundaries.

### 3.8 Process steps to an audio history

**Input.** A prepared execution, a finite input block, and current process state.

**Output.** One finite output block and the next state. Repeated steps produce an audio history prefix.

**Errors.** A prepared graph has no dynamic type or scheduling error. Hardware or host failure is outside the pure
process semantics and is reported by the engine.

**Equality.** For fixed input blocks, initial state, processor versions, and floating-point contract, deterministic
steps produce the same output blocks and next states. Cross-platform bit equality is promised only by processors whose
contracts state it.

**Record.** Each block records the prepared execution and input prefix that produced it. The record may be stored out of
the real-time callback; recording must not break the callback's safety rules.

## 4. Two valid paths

Notation-led work can follow this path:

```text
source
  -> Music
  -> score timeline
  -> notation
  -> interpreted gesture
  -> prepared process
  -> audio history
```

Performance-led work can follow this path:

```text
source phrase or live protocol
  -> finite intent
  -> gesture
  -> prepared process
  -> audio history
          \
           -> optional transcription -> notation
```

The second path matters. Requiring every gesture to pass through staff notation would misstate the Karnatak,
ensemble-tuning, and live examples. A later transcription can still produce notation, provided it records its loss.

## 5. Where information changes

| Step | Keeps | May add | May lose |
| --- | --- | --- | --- |
| Source to core | names, literals, source spans | fresh type names, resolved references | comments from semantic terms |
| Core to value | program meaning | evaluated choices | source syntax shape, while ancestry remains |
| `Music` to score facts | recipe requests | explicit context facts | unsupported requests only through an error or loss |
| Kernel term to timeline | payloads and exact placement | normalized placement | surface grouping that has no semantic effect |
| Timeline to notation | notatable facts | layout and target syntax | details the target cannot carry |
| Intent to gesture | selected musical intent | performer and instrument choices | alternatives not chosen |
| Gesture to process | control paths | processors, buffers, seeds | ideal controls a processor cannot realize |
| Process to audio | state transition result | numerical samples | symbolic reasons unless kept by derivation records |

Loss is not always a defect. Choosing one voicing loses other possible voicings. Transcribing a continuous ornament
loses detail. The defect would be pretending no choice occurred.

## 6. Three different completion claims

The source language terminates. Every closed, well-typed source term reaches a value.

The temporal kernel terminates. Every closed, well-typed temporal term reaches a finite normalized timeline.

An audio process does neither of those things. One process step finishes, but the host may ask for another block
forever. An unbounded run therefore has no final finite value. Its meaning is the ordered family of all finite audio
prefixes produced by its steps.

This is why a stream type or source-language effect is not required. The host owns repetition; the process graph owns
one deterministic step.

## 7. Coherence without a universal representation

A Musa project is coherent when:

1. every representation has a stated type and equality rule;
2. every conversion has typed inputs and outputs;
3. every produced item has a valid derivation path or generated-item record;
4. recorded losses use the versioned rule set of their pass; and
5. composed paths agree with the accepted derivation composition law.

This definition gives notation and audio a common history without claiming that notation is audio or that audio is
notation. The connection is the diagram of conversions. That diagram is the useful part of the earlier “motive” idea.

No additional `World` or `Link` term appears in the source language. The existing pass input, output, and derivation
record already say which representation a fact belongs to and how it crossed a boundary. A new term is justified only if
a later case cannot express that information with these records.

## 8. Open checks before promotion

The paper programs must show that all eight boundaries can be named without hidden context or an unlisted error. The
proof must then show that the finite source stages are safe and that typed derivation paths compose. Process safety
remains governed by the accepted process specification.

If either check fails, these documents remain research notes.
