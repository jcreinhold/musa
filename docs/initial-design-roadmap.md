# Design of a Notation-First Music Language and Workbench

## 1. Executive decision

Build a system with four distinct semantic layers:

```text
Source language
      │
      ▼
Compositional model
motifs, parts, voices, notes, phrases, transformations
      │
      ├──────────────► Notation model ──► MEI ──► Verovio ──► interactive score
      │                                      └──► LilyPond / MusicXML
      │
      ▼
Performance model
tempo, articulation, dynamics, timing, tuning
      │
      ▼
Studio model
instruments, synthesis, modulation, effects, routing
      │
      ▼
Compiled render plan
scheduled events + preallocated DSP graph
      │
      ▼
Live audio / WAV / MIDI
```

The critical design rule is:

> **The layers may refer to one another through narrow, typed bindings, but they must not share a single mutable object
> model.**

A violin part is not a synthesizer node. A written quarter note is not a MIDI event. A slur is not an ADSR envelope. A
low-pass-filter sweep is not a notation marking.

The application makes these things feel integrated, but the implementation keeps them separate.

The intended product is:

> **A score-centric composition environment with programmable motifs and transformations, immediate playback, and a
> deliberately small built-in studio.**

It is not:

- a general-purpose DAW;
- a publication-grade engraving system;
- a general-purpose programming language;
- a complete orchestral sample workstation;
- a universal formalization of music theory.

Those should remain external tools or later extensions.

---

# 2. Applying “Simple Made Easy”

Rich Hickey’s distinction is directly applicable:

- **Simple** means not interleaved or entwined with another concern.
- **Easy** means nearby, familiar, or requiring little immediate effort.

A system can be easy but deeply complected. A button that mutates a score, MIDI events, an audio graph, and serialized
project state simultaneously may feel convenient, but creates a system whose behavior is difficult to reason about.
Hickey’s argument is that simplicity produces reliability, flexibility, and comprehensibility, even when it requires
more deliberate design. citeturn372226search9

The design should therefore use this policy:

> **Simplicity belongs in the semantic core. Ease belongs at the interface.**

For example, the GUI may let the user choose “Warm Pad” from one menu. Internally, that command creates:

1. a patch declaration;
2. a binding from a score part to the patch;
3. a route from the patch to the master bus.

The user gets one easy action. The core still receives three simple facts.

## Things that must remain separate

| Concern | Must not be identified with |
| --- | --- |
| Written pitch | MIDI note number or frequency |
| Notated duration | Performed duration |
| Voice | Mixer track |
| Part | Synthesizer instance |
| Dynamic marking | Literal decibel value |
| Articulation | Fixed note-length multiplier |
| Motif definition | Its expanded occurrences |
| Score ordering | DSP evaluation ordering |
| Project source | GUI widget state |
| Audio graph | Visual graph layout |

This separation is not ceremony. Each pair varies independently.

For example, a written A4 can be:

- transposed because the instrument is transposing;
- tuned to something other than 440 Hz;
- ornamented during performance;
- synthesized by a sinusoid;
- played by a sampler;
- displayed differently because of clef or enharmonic spelling.

Representing all of those as “MIDI note 69” would make the easy representation the central abstraction and discard the
actual musical information.

---

# 3. Applying Ousterhout’s deep-module design

Ousterhout’s information-hiding principle says a module should conceal important design decisions behind a substantially
simpler interface. A deep module provides significant capability through a narrow surface; a shallow module merely
redistributes complexity to its callers. citeturn372226search1turn372226search17

That argues against creating a Cargo crate for every noun in the domain.

Do **not** begin with:

```text
pitch/
duration/
note/
voice/
part/
motif/
transform/
event/
```

as separate crates. That produces many shallow interfaces, dependency plumbing, and public types that become hard to
change.

Crate boundaries should correspond to genuine boundaries in:

- runtime requirements;
- reasons to change;
- platform dependencies;
- licensing;
- failure modes;
- information that must be hidden.

The deepest modules should look approximately like this:

```rust
compiler.compile(source) -> Compilation

notation.render(score, target, options) -> RenderedNotation

playback.build(score, performance, studio) -> PlaybackPlan

engine.install(plan)
engine.command(TransportCommand::Play)

project.apply(Command::InsertNote { ... }) -> ProjectUpdate
```

The caller should not need to know:

- how parser recovery works;
- how motifs are expanded;
- how voices are split into measures;
- how LilyPond durations are spelled;
- how an audio graph is topologically sorted;
- how DSP buffers are allocated;
- how CPAL negotiates an audio format;
- how a graphical edit is translated into source changes.

Those are the hidden design decisions that make the modules deep.

---

# 4. The product boundary

## Core use case

A user should be able to:

1. Open the application.
2. Create a piece.
3. Add violin, voice, bass, and synthesizer parts.
4. Enter notes from a MIDI keyboard or computer keyboard.
5. See a legible multi-staff score immediately.
6. Define or extract a motif.
7. repeat, transpose, stretch, invert, or vary it;
8. assign built-in sounds;
9. add a filter, modulation, delay, and reverb;
10. play the result;
11. export MIDI, WAV, MusicXML, MEI, or LilyPond.

No external plugins, sample libraries, LilyPond installation, or audio configuration should be necessary for steps 1–10.

## Explicit non-goals for the first system

### Audio recording and waveform editing

This would require:

- device routing;
- recording latency compensation;
- take management;
- waveform rendering;
- destructive and nondestructive editing;
- time stretching;
- fades and crossfades;
- comping;
- file lifecycle management.

It would bury the central composition-language idea beneath ordinary DAW engineering. Export stems to a DAW when
recorded vocals or advanced audio editing are needed.

### Complete engraving

LilyPond and other notation systems have spent decades handling engraving. This application should produce a strong
working score and export a semantically rich representation. It should not duplicate every page-layout and typographic
facility.

### Third-party plugin hosting

Audio Unit and CLAP hosting add discovery, compatibility, state serialization, GUI embedding, crash isolation,
threading, and platform-specific behavior. The available Rust CLAP support is intentionally low-level, while Apple’s
native audio stack models plugin hosting through its own node graph and runtime.
citeturn923438search0turn923438search3

A built-in DSP system is sufficient for the initial goal and guarantees zero setup.

### A general-purpose embedded language

Do not add unrestricted recursion, arbitrary I/O, threads, user-defined mutable state, or metaprogramming initially.

The composition language should be:

- pure;
- finite;
- deterministic;
- rapidly recompilable;
- statically diagnosable.

A finite `repeat 4` is useful. Arbitrary recursive generation is not necessary for the first composition environment.

---

# 5. The algebraic foundation

Category theory should guide the laws and interfaces. It should **not** leak category-theory terminology into ordinary
composition.

The user should write “play these phrases together,” not “take the monoidal product of these morphisms.”

## 5.1 Musical duration

Represent musical time using exact nonnegative rationals:

```rust
pub struct MusicalTime(Ratio<i64>);
pub struct MusicalDuration(Ratio<i64>);
```

Let `1` denote a whole note. Then:

```text
quarter note = 1/4
dotted quarter = 3/8
eighth-note triplet = 1/12
```

Do not use floating-point beats in the compositional or notation model.

Durations form an additive monoid:

```text
0             identity
a + b         composition
(a + b) + c = a + (b + c)
```

This provides the span law for sequential composition.

## 5.2 Sequential composition

A fragment can be followed by another fragment:

```text
a then b
```

with laws:

```text
empty then a = a
a then empty = a
(a then b) then c = a then (b then c)
```

Its timeline semantics are:

```text
span(a then b) = span(a) + span(b)

events(a then b)
  = events(a)
  ∪ shift(span(a), events(b))
```

## 5.3 Parallel composition

Independent voices can be overlaid:

```text
a together_with b
```

with:

```text
span(a together_with b) = max(span(a), span(b))

events(a together_with b)
  = events(a) ∪ events(b)
```

Parallel composition is associative. It is commutative only when the participating voice identities are distinct and
ordering has no semantic role.

Therefore, the canonical representation should not blindly overlay two anonymous event lists. It should overlay a map of
identified lanes:

```rust
BTreeMap<VoiceId, Fragment>
```

Attempting to overlay two values into the same monophonic voice should produce either:

- a diagnostic;
- an explicit polyphonic merge;
- or a new subvoice.

It should never silently guess.

## 5.4 Transformations

Transformations operate on musical values rather than mutating notes in place:

```text
transpose(interval, music)
stretch(ratio, music)
retrograde(music)
invert(axis, music)
```

Some preserve composition:

```text
transpose(i, a then b)
  = transpose(i, a) then transpose(i, b)

stretch(r, a then b)
  = stretch(r, a) then stretch(r, b)
```

Retrograde reverses order:

```text
retrograde(a then b)
  = retrograde(b) then retrograde(a)
```

This is an anti-homomorphism rather than an ordinary homomorphism. Encoding such laws explicitly is useful both
conceptually and for property testing.

Transformational music theory has often used groups and group actions, while category- and groupoid-based treatments
generalize the idea to partial, noninvertible transformations and musical objects of different kinds or cardinalities.
citeturn375652academia40turn375652search13

The design implication is:

> Do not assume that every musically meaningful transformation is an invertible operation on one uniform pitch-class
> set.

A transformation should be allowed to:

- fail with a meaningful diagnostic;
- change the type or shape of an object;
- require a declared pitch system;
- preserve some properties but not others.

## 5.5 Lowerings as structure-preserving maps

The lowering from a compositional fragment to scheduled events should preserve the relevant operations:

```text
lower(a then b)
  = lower(a) then lower(b)

lower(a together_with b)
  = merge(lower(a), lower(b))
```

Likewise, rendering two independent parts together should produce the same musical result as rendering each and
combining their lanes.

These are “functor-like” laws. The implementation does not need a generic `Functor` trait. The important part is that
the transformations have declared laws that can be tested.

## 5.6 The production graph

The DSP graph has a separate but related algebra:

- an audio processor is a mapping from input signal bundles to output signal bundles;
- serial composition connects outputs to inputs;
- parallel composition places processors side by side;
- mixers merge signal paths;
- splitters duplicate them;
- feedback is allowed only through an explicit delay.

Faust’s block-diagram algebra uses sequential, parallel, split, merge, and feedback composition as the foundation of its
signal-processing language. That is a better conceptual model for the studio layer than an arbitrary object graph.
citeturn988595search0turn988595search1

The score algebra and signal algebra are analogous, but they are not the same algebra. They should not share a universal
“node” type.

---

# 6. The semantic layers

## 6.1 Source syntax tree

The source syntax tree retains:

- every token;
- comments;
- whitespace;
- malformed constructs;
- exact source spans.

Its job is editing, diagnostics, and formatting.

It contains no resolved names, computed pitches, expanded motifs, or audio objects.

## 6.2 High-level compositional representation

The high-level representation retains authorial concepts:

- declarations;
- motifs;
- named parts;
- voices;
- transformations;
- references;
- phrase spans;
- score annotations;
- symbolic pitches;
- exact durations.

This is where:

```text
transpose minor_third { use sigh; }
```

still exists as a transformation rather than as five unrelated replacement notes.

## 6.3 Expanded score representation

Compilation expands finite motif applications and transformations into a normalized score:

```rust
pub struct ScoreSnapshot {
    pub parts: PartMap,
    pub tempo_map: TempoMap,
    pub meter_map: MeterMap,
    pub key_map: KeyMap,
    pub annotations: AnnotationStore,
}

pub struct Voice {
    pub events: Vec<ScoreEvent>,
}

pub struct ScoreEvent {
    pub id: EventId,
    pub origin: Origin,
    pub onset: MusicalTime,
    pub notated_duration: NotatedDuration,
    pub kind: ScoreEventKind,
}
```

This representation is:

- finite;
- sorted;
- immutable;
- independent of any particular notation backend;
- still musically spelled.

A pitch should preserve its written identity:

```rust
pub struct WrittenPitch {
    pub letter: Letter,
    pub accidental: Accidental,
    pub octave: i8,
}
```

Thus D-sharp and E-flat remain distinct even if they later map to the same 12-tone frequency.

`NotatedDuration` should retain both:

- exact temporal value;
- notational spelling.

A dotted quarter and a quarter tied to an eighth may have the same span but are not the same notation.

## 6.4 Performance representation

The performance layer converts score meaning into realization:

```rust
pub struct PerformancePlan {
    pub tempo: IntegratedTempoMap,
    pub lanes: Vec<PerformanceLane>,
}

pub enum PerformanceEvent {
    NoteOn { frame: u64, note: PerformedNote },
    NoteOff { frame: u64, voice: VoiceInstanceId },
    Parameter { frame: u64, target: ParameterId, value: f32 },
}
```

This layer applies:

- tempo;
- rubato;
- dynamics;
- articulation profiles;
- tuning;
- expressive timing;
- pedal and control curves;
- instrument transposition.

A staccato mark does not itself mean “multiply duration by 0.5.” Its realization is selected by an instrument or
performance profile.

Likewise, `p` is a symbolic dynamic relationship. It is not globally equivalent to a particular MIDI velocity or decibel
value.

## 6.5 Studio representation

The studio layer contains:

- patches;
- oscillators;
- samplers;
- effects;
- buses;
- sends;
- routings;
- automation bindings;
- parameter modulation.

The bridge from score to studio is narrow:

```text
PartId ──assigned to──► PatchId
DynamicLane ──────────► ExpressionParameter
AutomationLane ───────► ParameterId
PatchOutput ──────────► BusId
```

The studio does not inspect notes, measures, or slurs directly. It receives performance events.

---

# 7. Source language design

The syntax should be explicit, compact, and unsurprising to a programmer, but optimized for reading music rather than
resembling Rust for its own sake.

## 7.1 Proposed shape

```text
piece "Glass Mountain" {
    tempo quarter = 72;
    meter 4/4;
    key a minor;

    motif sigh(root: pitch = e5) {
        root 1/2;
        rest 1/4;
        c5   1/2;
        b4   1/4;
        a4   1/2;
    }

    score {
        part violin {
            clef treble;

            voice lead {
                use sigh();

                transpose down P5 {
                    use sigh();
                }
            }
        }

        part strings {
            voice upper {
                c5 1;
                c5 1;
                a4 1;
                gs4 1;
            }

            voice bass {
                a2 1;
                f2 1;
                d2 1;
                e2 1;
            }
        }
    }

    performance {
        profile violin {
            articulation staccato {
                gate = 0.55;
                attack = 8 ms;
            }
        }
    }

    studio {
        patch glass_pad {
            carrier = oscillator(sine);
            shimmer = oscillator(sine, ratio: 2)
                |> gain(-15 dB);

            mix(carrier, shimmer)
                |> envelope(adsr(
                    attack: 30 ms,
                    decay: 1.8 s,
                    sustain: 0.65,
                    release: 3.5 s
                ))
                |> lowpass(cutoff: 1400 Hz, q: 0.7)
                |> output;
        }

        lfo = oscillator(sine, frequency: 0.08 Hz)
            |> scale(250 Hz)
            |> bias(1400 Hz);

        modulate lfo -> glass_pad.lowpass.cutoff;

        bus hall {
            reverb(room: 0.82, damping: 0.55);
        }

        assign violin -> glass_pad;
        assign strings -> glass_pad;

        route violin -> master;
        route strings -> master;

        send violin -> hall at -18 dB;
        send strings -> hall at -14 dB;
        route hall -> master;
    }
}
```

This syntax is illustrative. Its semantic divisions are more important than its punctuation.

## 7.2 Design choices

### Explicit semicolons and braces

Newlines should be trivia, not syntax. That makes formatting, copy-and-paste, and error recovery more predictable.

### Sequence by lexical order

Inside a `voice` or `motif`, items occur sequentially unless an explicit parallel construct is used.

### Parallelism through identified voices

Do not use implicit chord inference from simultaneous cursor positions. Chords are explicit:

```text
chord [a3, c4, e4] 1/2;
```

Independent lines are explicit voices.

### Rational durations

Canonical duration syntax should be fractions of a whole note:

```text
1      whole
1/2    half
1/4    quarter
3/8    dotted quarter
1/12   triplet eighth
```

The editor may display familiar note symbols and accept shortcuts such as `q`, `h`, or `e`, but those should elaborate
into exact values.

### Finite constructs only

The first language should support:

```text
repeat 4 { ... }
transpose ...
stretch ...
retrograde ...
invert ...
```

It should reject recursive motif definitions:

```text
motif forever {
    use forever();
}
```

This guarantees responsive compilation and a finite score.

### Units are part of the syntax

Audio parameters should require units where applicable:

```text
1400 Hz
250 ms
-18 dB
0.08 Hz
72 bpm
```

A bare `1400` should not ambiguously mean hertz, MIDI units, or a normalized control value.

### No raw backend escapes

Do not initially provide:

```text
lilypond_raw { ... }
```

or:

```text
verovio_option "arbitrary-string";
```

Raw escape hatches make backend details infectious and prevent other renderers from representing the same piece.
Unsupported notation should produce an explicit diagnostic until the common model is intentionally extended.

---

# 8. Pitch, tuning, harmony, and theory

## 8.1 Do not over-generalize the initial pitch type

It would be tempting to parameterize the entire system:

```rust
Score<PitchSystem, Tuning, Temperament, HarmonicTheory>
```

That would make every function and data type carry abstractions most users do not need. Genericity itself can become
complecting.

The first implementation should use a concrete, well-designed written-pitch representation supporting conventional
Western notation.

Tuning remains a separate service:

```rust
pub trait Tuning {
    fn frequency(&self, pitch: &SoundingPitch) -> Hertz;
}
```

The default is twelve-tone equal temperament with configurable concert A. The semantic boundary leaves room for other
tuning systems without forcing them through every type.

## 8.2 Harmony should initially be annotation, not ontology

A chord symbol can be recorded:

```text
harmony {
    at 1:1 am;
    at 3:1 fmaj7;
}
```

But the core should not insist that all notes derive from chord symbols or one theory of harmonic function.

Later libraries may provide:

- tonal harmony analysis;
- Roman-numeral transformations;
- neo-Riemannian operations;
- scales and modes;
- voice-leading optimization;
- counterpoint constraints;
- automatic voicing.

These should be algorithms over the compositional model, not assumptions embedded in `Note`.

## 8.3 High-level abstractions must lower transparently

A useful abstraction has:

1. a clear musical meaning;
2. deterministic lowering;
3. retained provenance;
4. inspectable output.

For example:

```text
transpose down P5 { use sigh(); }
```

should let the user inspect:

```text
definition: sigh
transformation: transpose down P5
occurrence: violin.lead, measure 5
resulting pitch: A4
```

Avoid abstractions such as:

```text
make_more_sad();
cinematic_harmony();
```

They have no stable semantics and cannot support predictable editing.

---

# 9. Provenance and graphical editing

The system must know not only that an expanded note exists, but **why** it exists.

```rust
pub struct Origin {
    pub source_span: SourceSpan,
    pub declaration: DeclarationId,
    pub expansion_path: Vec<ExpansionStep>,
}

pub enum ExpansionStep {
    MotifApplication { call_site: SourceSpan },
    RepeatIteration(u32),
    Transposition(Interval),
    Stretch(Ratio<i64>),
    Retrograde,
}
```

This enables:

- clicking a rendered note and locating its source;
- displaying how a transformation produced it;
- meaningful diagnostics;
- preserving identity through rendering;
- deciding how a graphical edit should behave.

## Editing transformed music

Suppose the user clicks one note in the third occurrence of a motif and changes C5 to D5.

The application must not silently mutate an expanded cache. It should present two meaningful operations:

### Edit the motif definition

Changes every occurrence.

### Detach or specialize this occurrence

Replaces:

```text
use sigh();
```

with a local fragment or an override:

```text
use sigh() with {
    note 2 = d5;
}
```

This is a genuinely useful consequence of treating music as source-based structure rather than as anonymous piano-roll
events.

---

# 10. Parser and compiler design

## 10.1 Lexer: `logos`

Use `logos` for lexical recognition. It provides token iteration and source spans and is appropriate for a compact
language lexer. citeturn409710search0turn409710search19

The lexer must emit trivia tokens rather than discarding them:

```rust
enum SyntaxKind {
    Whitespace,
    LineComment,
    BlockComment,

    Identifier,
    Integer,
    String,
    PitchLiteral,
    UnitLiteral,

    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Semicolon,
    Arrow,
    PipeForward,

    PieceKw,
    ScoreKw,
    PartKw,
    VoiceKw,
    MotifKw,
    // ...
}
```

Keeping trivia is necessary for a lossless tree and stable formatting.

## 10.2 Parser: hand-written recursive descent

Use:

- recursive descent for declarations and statements;
- a small Pratt parser for arithmetic and signal expressions;
- event-based tree construction;
- explicit recovery sets.

I would **not** use a parser generator or a heavily generic parser-combinator system initially.

`chumsky` offers expressive combinators and error recovery, but its generic type machinery can make large parsers and
compiler errors harder to control. citeturn409710search1turn409710search27

A hand-written parser is preferable here because:

- the grammar is intentionally small;
- error recovery is product-critical;
- parser behavior must map precisely to a lossless tree;
- IDE-style partial parsing matters more than grammar brevity;
- the implementation remains directly inspectable.

## 10.3 Lossless concrete syntax tree: `rowan`

Build a lossless green tree using `rowan`. This follows the architecture used by rust-analyzer: resilient full-fidelity
syntax is kept separate from a more compact semantic representation. citeturn409710search3turn121767search3

The parser should emit events:

```rust
enum ParseEvent {
    StartNode(SyntaxKind),
    Token(SyntaxKind, TextRange),
    FinishNode,
    Error(ParseError),
}
```

A second pass builds the Rowan tree.

The parser always returns a tree, even for invalid input:

```rust
pub struct ParsedDocument {
    pub syntax: SyntaxNode,
    pub errors: Vec<SyntaxError>,
}
```

## 10.4 Error recovery

Recovery points should include:

- semicolon;
- closing brace;
- the next declaration keyword;
- the next `part`, `voice`, `patch`, or `bus`.

Malformed regions become `ERROR` nodes. The remainder of the score should continue compiling where safe.

Diagnostics should use a backend-neutral internal representation, with `miette` handling rich command-line rendering.
citeturn193176search0

## 10.5 Typed syntax wrappers

Provide typed views such as:

```rust
struct PieceDecl(SyntaxNode);
struct MotifDecl(SyntaxNode);
struct PartDecl(SyntaxNode);
struct VoiceDecl(SyntaxNode);
```

These are convenience wrappers over syntax. They must not cache semantic resolution.

## 10.6 Compiler pipeline

```text
text
  │
  ▼
lossless CST
  │
  ▼
typed syntax views
  │
  ▼
name resolution + unit checking
  │
  ▼
high-level compositional representation
  │
  ▼
motif expansion and transformation
  │
  ▼
normalized ScoreSnapshot + StudioSpec
  │
  ├──► NotationPlan
  └──► PerformancePlan
```

The compiler facade should expose one deep operation:

```rust
pub fn compile(
    source: &SourceDocument,
    options: &CompileOptions,
) -> Compilation;
```

> **Course correction (docs/course-correction.md):** the semantic core beneath this pipeline is now the finite
> temporal kernel specified in `docs/kernel/` — the "high-level compositional representation" and "motif expansion"
> stages above are the elaboration/HIR that evaluates into kernel timelines, and the normalized `ScoreSnapshot` is an
> adapter projection of them. Where this roadmap and the course correction disagree on semantic architecture, the
> course correction wins; everything else in this document stands.

Intermediate pass types should remain private unless another crate has a real semantic need for them.

## 10.7 Incrementality

Do not introduce Salsa in the first implementation.

Whole-document parsing and compilation of a small score should be inexpensive. Start with:

- debounced compilation;
- immutable snapshots;
- reuse of the last successful playback plan;
- compilation on a worker thread.

Salsa supports incremental computations, but it adds an additional dependency model and identity discipline. Introduce
it only when profiling demonstrates that recompilation is a user-visible limitation. citeturn409710search4

---

# 11. Formatting and source edits

The formatter should operate on syntax, not on the expanded semantic model.

Required law:

```text
semantic(parse(format(parse(source))))
    =
semantic(parse(source))
```

Comments must remain attached predictably.

Graphical commands should produce syntax-aware text edits:

```rust
pub enum EditCommand {
    InsertNote {
        voice: VoiceId,
        cursor: MusicalPosition,
        pitch: WrittenPitch,
        duration: NotatedDuration,
    },

    ChangePitch {
        event: EventId,
        pitch: WrittenPitch,
        mode: GeneratedEditMode,
    },

    ChangeDuration {
        event: EventId,
        duration: NotatedDuration,
    },

    ExtractMotif {
        events: Vec<EventId>,
        name: String,
    },

    AssignPatch {
        part: PartId,
        patch: PatchId,
    },
}
```

The project layer resolves these into text transactions. It does not mutate a second editable AST.

This gives the system one persistent truth:

> **The project source is canonical; score and studio interfaces are structured editors of that source.**

---

# 12. Notation and interchange

## 12.1 Backend-neutral notation planning

The expanded score is not yet sufficient for engraving. Derive a `NotationPlan` containing:

- measure boundaries;
- clefs;
- key and time signatures;
- voice allocation;
- beaming groups;
- tied duration decomposition;
- tuplet groups;
- staff assignment;
- spelling decisions;
- annotations and spans.

```text
ScoreSnapshot
      │
      ▼
NotationPlan
   ├──► MEI
   ├──► LilyPond
   └──► MusicXML
```

This prevents LilyPond- or MusicXML-specific assumptions from entering the compositional core.

## 12.2 MEI and Verovio for the live score

MEI should be the primary live-rendering format.

Verovio renders MEI to SVG, supports conversion from formats including MusicXML, and preserves MEI structure and
identifiers in the resulting SVG. That makes it especially suitable for mapping rendered notes back to semantic event
IDs. citeturn738899search5turn535118search26turn535118search30

The MEI writer should assign IDs such as:

```xml
<note xml:id="event-7f3c..." ... />
```

The frontend can then:

- select notes;
- highlight playback;
- show diagnostics;
- open context menus;
- map clicks to `EventId`.

Use `quick-xml` to construct MEI and MusicXML through a real XML writer rather than manual string concatenation.
citeturn535118search1

## 12.3 LilyPond export

LilyPond should be a high-quality print backend, not the internal representation or live editor. LilyPond is a compiled
textual engraving system and can produce engraved output and MIDI, but its source language is much broader than the
proposed composition model. citeturn391625search20turn391625search0

Use three stages:

```text
NotationPlan
      │
      ▼
typed LilyPond document
      │
      ▼
deterministic pretty-printer
      │
      ▼
.ly text
```

For example:

```rust
enum LyNode {
    Sequential(Vec<LyNode>),
    Simultaneous(Vec<LyNode>),
    Note(LyNote),
    Rest(LyDuration),
    Command(LyCommand),
    Context {
        kind: LyContextKind,
        name: Option<String>,
        body: Box<LyNode>,
    },
}
```

Do not scatter `format!()` calls throughout score traversal.

Generated files should contain:

- an explicit LilyPond version;
- deterministic declaration order;
- stable variable names;
- escaped text;
- source-map comments where useful;
- diagnostics for unsupported constructs.

The normal application should merely export `.ly`. Optionally, it may invoke a separately installed LilyPond executable
or an intentionally bundled sidecar after licensing and distribution review. LilyPond is GPL-licensed, whereas Verovio
uses the LGPL; this difference is another reason to keep them behind separate backend interfaces.
citeturn650761search0turn650761search1

Do not support LilyPond import. LilyPond includes extensive language features and embedded Scheme; reliable
round-tripping would effectively require implementing a substantial LilyPond frontend.

## 12.4 MusicXML

MusicXML is the appropriate interoperability export because it is an open format designed for exchanging digital sheet
music among applications. citeturn391625search1

It should not be the internal score model. Its document hierarchy and interchange-oriented conventions would cause
external-format choices to leak into composition semantics.

MusicXML import can come later and should produce warnings when information cannot be represented exactly.

## 12.5 MIDI

Provide two MIDI exports:

### Score MIDI

A literal, neutral realization useful for interchange.

### Performance MIDI

Includes:

- interpreted dynamics;
- articulation;
- tempo changes;
- control curves;
- expressive timing.

Use `midly` for Standard MIDI File reading and writing. Live MIDI input belongs in the engine through `midir`.
citeturn797105search1turn797105search2

MIDI must remain an edge format, never the canonical representation.

---

# 13. Sound synthesis and DSP

## 13.1 Audio architecture

```text
ScoreSnapshot
      │
      ▼
PerformancePlan
      │
      ▼
scheduled note/control events
      │
      ▼
voice allocators
      │
      ▼
instrument DSP
      │
      ▼
effects and routing graph
      │
      ▼
master output
```

Use CPAL for cross-platform audio-device and stream access. It exposes device enumeration, supported configurations, and
audio streams without dictating the synthesis architecture. citeturn797105search0

## 13.2 Real-time separation

The audio callback must not:

- allocate;
- acquire locks;
- access files;
- parse source;
- compile graphs;
- render diagnostics;
- write logs;
- destroy large graph objects.

The control side compiles a declarative graph into a preallocated `RenderPlan`. The audio side only executes that plan.

Use `rtrb`, a wait-free single-producer/single-consumer ring buffer, for control and transport commands crossing the
real-time boundary. citeturn193176search1

A second queue should return retired plans to the control thread so large structures are not destroyed in the callback.

## 13.3 Graph specification versus render plan

```rust
pub struct StudioGraphSpec {
    pub nodes: NodeMap,
    pub connections: Vec<Connection>,
    pub parameters: ParameterStore,
}

pub struct RenderPlan {
    processors: Vec<CompiledProcessor>,
    schedule: Vec<ProcessStep>,
    buffers: BufferArena,
    parameter_state: ParameterState,
}
```

`StudioGraphSpec` is declarative and editable.

`RenderPlan` contains:

- resolved ports;
- topological order;
- preallocated buffers;
- initialized DSP state;
- parameter smoothing;
- fixed process operations.

Do not use a general graph library as the public representation. A specialized compiler can validate:

- port compatibility;
- channel counts;
- disconnected nodes;
- cycles;
- illegal feedback;
- unreachable processors.

A cycle is allowed only if it passes through an explicit delay node. That makes causality visible rather than relying on
arbitrary graph-evaluation behavior.

## 13.4 Typed ports

At minimum:

```rust
enum PortKind {
    Audio { channels: u8 },
    Control,
    Gate,
    NoteEvents,
}
```

An audio-rate signal and a low-rate control value must not be interchangeable merely because both eventually contain
`f32`.

Explicit adapters include:

```text
control_to_audio
audio_envelope_follower
stereo_to_mono
mono_to_stereo
```

## 13.5 Sinusoidal synthesis

A basic oscillator maintains phase:

```text
phase[n + 1] = frac(phase[n] + frequency[n] / sample_rate)
output[n]    = sin(2π × phase[n])
```

The oscillator node needs:

- phase state;
- sample rate;
- frequency input;
- optional phase-reset or gate input;
- parameter smoothing.

A playable synthesizer is not merely an oscillator. Its structure is:

```text
note events
    │
    ▼
polyphonic voice allocator
    │
    ├──► pitch-to-frequency
    ├──► oscillator bank
    ├──► amplitude envelope
    ├──► optional filter envelope
    └──► per-voice gain/pan
              │
              ▼
             mix
```

Tuning maps sounding pitches to frequencies. This is deliberately separate from written pitch.

## 13.6 Initial built-in processors

The first useful set should be small:

### Sources

- sine oscillator;
- noise;
- constant control value;
- note pitch;
- note velocity;
- gate.

### Control

- ADSR envelope;
- LFO;
- scale;
- bias;
- clamp;
- smoothing;
- arithmetic combination.

### Filters and dynamics

- one-pole low-pass;
- biquad low-pass and high-pass;
- gain;
- pan;
- simple peak limiter on the master.

### Time effects

- delay;
- feedback delay;
- chorus;
- algorithmic reverb.

### Routing

- mixer;
- splitter;
- mono/stereo conversion;
- send and return.

A compressor, convolution reverb, distortion, tape simulation, and band-limited analog oscillators can come later.

FunDSP already provides Rust DSP graph concepts and processors including oscillators, filters, chorus, and
reverberation. It is useful as an implementation backend or reference, but its types should remain hidden behind the
workbench’s own processor interface. citeturn186041search0turn186041search1

This prevents the language and saved project format from becoming dependent on one DSP library.

## 13.7 Modulation

Modulation should be an explicit typed connection:

```text
lfo
  |> scale(250 Hz)
  |> bias(1400 Hz)
  -> filter.cutoff
```

The parameter system should know:

- unit;
- legal range;
- default;
- smoothing policy;
- modulation-combination policy.

For example:

```rust
pub struct ParameterDescriptor {
    pub unit: Unit,
    pub range: RangeInclusive<f32>,
    pub default: f32,
    pub smoothing: Smoothing,
    pub combination: ModulationCombination,
}
```

This is better than exposing an anonymous normalized `0.0..1.0` for every parameter.

## 13.8 Offline rendering

Offline WAV rendering should execute the same processors and scheduled events as live playback, without CPAL.

Use `hound` for WAV encoding. citeturn535118search9

This gives:

- deterministic exports;
- easier DSP testing;
- faster-than-real-time rendering;
- no divergence between live and exported sound.

---

# 14. GUI architecture

## 14.1 Technology recommendation

Use:

- **Tauri** for the desktop shell and Rust command backend;
- **TypeScript and Svelte** for UI components;
- **CodeMirror 6** for the optional source editor;
- **Verovio WebAssembly** for score rendering;
- ordinary SVG/HTML overlays for interaction.

Tauri’s architecture supports a Rust application core with a webview frontend and explicit command/message boundaries.
citeturn738899search2turn974330search18

CodeMirror models editor changes as transactions over immutable editor state, which fits the project’s
source-transaction model. citeturn974330search0turn974330search1

A pure-Rust immediate-mode GUI would avoid JavaScript but would require substantially more work to integrate:

- an interactive SVG score;
- a mature source editor;
- accessible text input;
- rich inspector components;
- scalable notation rendering.

Using a webview at the UI boundary does not compromise the Rust semantic core. Insisting on Rust for the interface would
optimize implementation ideology rather than the product.

## 14.2 State ownership

Rust owns:

```text
source text
compiled score
studio specification
diagnostics
playback plan
project revision
```

The frontend owns only ephemeral state:

```text
selected panel
scroll position
zoom
hovered event
open inspector section
temporary drag state
```

The frontend must never become a second authority for score semantics.

## 14.3 Default workspace

```text
┌─────────────────────────────────────────────────────────────┐
│ Play  Stop  Record/Step  Loop       bar 4:2       72 BPM    │
├──────────────┬──────────────────────────────────────────────┤
│ Parts        │                                              │
│              │              SCORE                           │
│ Violin       │                                              │
│ Strings      │       multi-staff Verovio SVG                │
│ Bass         │                                              │
│ Pad          │                                              │
├──────────────┴───────────────────────────────┬──────────────┤
│ optional diagnostics / source / mixer        │ Inspector    │
│                                              │ pitch        │
│                                              │ duration     │
│                                              │ sound        │
└──────────────────────────────────────────────┴──────────────┘
```

The score is the main interface.

The source, sound graph, and mixer are separate modes or collapsible panels. They should not permanently compete for
screen space.

## 14.4 Progressive disclosure

Provide four workspaces:

### Compose

Score, parts, transport, note inspector.

### Sound

Patch graph and selected-part sound controls.

### Mix

Tracks, buses, sends, levels, automation.

### Source

Text source, diagnostics, and score preview.

The default user never needs to open Source or the raw sound graph.

## 14.5 Note entry

Initial score editing should be keyboard-first and deterministic:

- select active part and voice;
- select duration with numeric shortcuts;
- enter pitch from MIDI keyboard;
- use arrow keys to transpose;
- use space to insert a rest;
- type chord notes while a chord-entry modifier is held;
- use a tie shortcut;
- use tab or arrow navigation between events;
- start playback from the selection.

Do not make dragging notes around the staff the primary interaction. Dragging is visually intuitive but ambiguous
around:

- voice ownership;
- accidentals;
- duration;
- ties;
- tuplets;
- generated motif occurrences.

Mouse interaction is appropriate for selection, range selection, and opening inspectors.

## 14.6 Score commands

Clicking an SVG element yields an `EventId`. The frontend sends a semantic command:

```rust
ProjectCommand::ChangeDuration {
    event: event_id,
    duration: DurationValue::Quarter,
}
```

The project module:

1. resolves the event’s provenance;
2. determines whether it is directly authored or generated;
3. computes source edits;
4. applies them transactionally;
5. recompiles;
6. returns an updated score SVG and diagnostics.

## 14.7 Invalid edits

While source is temporarily invalid:

- display diagnostics immediately;
- keep the last valid score visible;
- keep the last valid playback plan installed;
- visually indicate that playback reflects the last valid revision.

This is much less disruptive than disabling the whole program after every incomplete edit.

## 14.8 Minimal setup

A new installation should bundle:

- Verovio;
- the built-in DSP engine;
- several small synthesis presets;
- default score templates;
- a default audio-output selection algorithm.

Creating a piece should immediately yield an audible sine-based or pad-like instrument.

LilyPond, sample libraries, plugins, and external MIDI tools remain optional.

---

# 15. Cargo workspace

The workspace should start with seven substantial crates, not dozens of microscopic ones.

```text
musa/
├── Cargo.toml
├── crates/
│   ├── musa-language/
│   ├── musa-compiler/
│   ├── musa-render/
│   ├── musa-audio/
│   ├── musa-engine/
│   ├── musa-project/
│   └── musa-cli/
├── apps/
│   └── musa-desktop/
│       ├── src-tauri/
│       └── ui/
├── examples/
│   ├── invention.musa
│   ├── glass-mountain.musa
│   └── counterpoint.musa
└── tests/
    └── fixtures/
```

## 15.1 Dependency direction

```text
musa-language
      │
      ▼
musa-compiler
   │       │
   ▼       ▼
render    audio
             │
             ▼
           engine
       \      │      /
        \     ▼     /
          project
          │      │
          ▼      ▼
         cli   desktop
```

No dependency points upward.

In particular:

- compiler does not depend on rendering;
- compiler does not depend on audio;
- audio does not depend on the GUI;
- render does not know about source-editor widgets;
- the frontend does not know about CPAL or FunDSP.

## 15.2 `musa-language`

Owns:

- tokens;
- lexer;
- parser;
- CST;
- typed syntax wrappers;
- formatter;
- syntax diagnostics;
- text-edit utilities.

Dependencies:

```text
logos
rowan
miette
thiserror
```

Public interface:

```rust
pub fn parse(source: &str) -> ParsedDocument;
pub fn format(document: &ParsedDocument) -> FormattedSource;
pub fn apply_edits(source: &str, edits: &[TextEdit]) -> String;
```

It should not expose Logos token iterators or Rowan green-node implementation details beyond an intentional syntax API.

## 15.3 `musa-compiler`

Owns:

- name resolution;
- units;
- semantic checks;
- high-level composition model;
- transformations;
- motif expansion;
- exact musical time;
- score normalization;
- performance lowering;
- provenance;
- public immutable snapshots.

Dependencies:

```text
musa-language
num-rational
slotmap
indexmap
serde
thiserror
```

`slotmap` is suitable for efficient transient arena keys and secondary maps, but those keys should not be serialized as
permanent project identities. citeturn902998search0

Public interface:

```rust
pub fn compile(
    syntax: &ParsedDocument,
    options: &CompileOptions,
) -> Compilation;
```

Most compiler passes remain private modules inside this crate.

## 15.4 `musa-render`

Owns:

- `NotationPlan`;
- MEI generation;
- MusicXML generation;
- LilyPond document generation;
- MIDI-file export;
- deterministic text/XML serialization;
- source maps for rendered artifacts.

Dependencies:

```text
musa-compiler
quick-xml
midly
serde
thiserror
```

Public interface:

```rust
pub enum NotationTarget {
    Mei,
    MusicXml,
    LilyPond,
}

pub fn render_notation(
    score: &ScoreSnapshot,
    target: NotationTarget,
    options: &NotationOptions,
) -> Result<RenderedNotation, RenderError>;

pub fn render_midi(
    performance: &PerformancePlan,
    options: &MidiOptions,
) -> Result<Vec<u8>, RenderError>;
```

## 15.5 `musa-audio`

Owns:

- declarative studio graph types;
- graph validation;
- graph compilation;
- processor interface;
- oscillators;
- envelopes;
- filters;
- effects;
- block processing;
- offline rendering;
- built-in patch library.

Dependencies:

```text
musa-compiler
fundsp
hound
rtrb
thiserror
```

The public API must expose the project’s own graph and processor concepts—not FunDSP’s generic types.

## 15.6 `musa-engine`

Owns platform and real-time integration:

- CPAL device negotiation;
- output stream lifecycle;
- live scheduling;
- MIDI input;
- transport;
- real-time command queues;
- render-plan installation;
- clock synchronization.

Dependencies:

```text
musa-compiler
musa-audio
cpal
midir
rtrb
tracing
thiserror
```

Public interface:

```rust
pub struct AudioEngine { /* hidden */ }

impl AudioEngine {
    pub fn open(config: EngineConfig) -> Result<Self, EngineError>;
    pub fn install(&self, plan: PreparedPlaybackPlan)
        -> Result<(), EngineError>;
    pub fn command(&self, command: TransportCommand)
        -> Result<(), EngineError>;
}
```

The rest of the application should never see a CPAL stream.

## 15.7 `musa-project`

This is the application’s main deep module.

It owns:

- project loading and saving;
- source documents;
- revisions;
- commands;
- undo and redo;
- compiler orchestration;
- rendered-score snapshots;
- playback-plan preparation;
- autosave;
- asset references.

Dependencies:

```text
musa-language
musa-compiler
musa-render
musa-audio
musa-engine
serde
toml
tracing
```

Public interface:

```rust
pub struct ProjectSession { /* hidden */ }

impl ProjectSession {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, ProjectError>;

    pub fn snapshot(&self) -> ProjectSnapshot;

    pub fn apply(
        &mut self,
        command: ProjectCommand,
    ) -> Result<ProjectUpdate, ProjectError>;

    pub fn export(
        &self,
        request: ExportRequest,
    ) -> Result<ExportArtifact, ProjectError>;
}
```

This interface hides the fact that one user command may trigger parsing, compilation, score rendering, and playback-plan
rebuilding.

## 15.8 `musa-cli`

Commands:

```text
musa check piece.musa
musa format piece.musa
musa render piece.musa --to lilypond
musa render piece.musa --to musicxml
musa render piece.musa --to wav
musa play piece.musa
```

The CLI must call `musa-project` or other public facades. It should not recreate compiler orchestration.

## 15.9 `musa-desktop`

The Tauri shell should be deliberately thin:

- command adaptation;
- window lifecycle;
- file dialogs;
- frontend event delivery.

All project semantics remain in `musa-project`.

---

# 16. Project and album organization

The simplest project is one file:

```text
glass-mountain.musa
```

A project becomes a directory only when it needs shared pieces or assets:

```text
felt-mountain-project/
├── musa.toml
├── pieces/
│   ├── 01-opening.musa
│   ├── 02-waltz.musa
│   └── 03-final.musa
├── library/
│   ├── motifs.musa
│   └── patches.musa
└── assets/
    └── field-recording.wav
```

Do not build a package registry or generalized dependency solver.

Relative imports are sufficient:

```text
use "../library/patches.musa";
```

Imports should be:

- declarative;
- acyclic;
- local;
- side-effect-free.

An album manifest may specify ordering and shared metadata, but a piece remains independently compilable.

---

# 17. Testing strategy

## 17.1 Parser tests

Use snapshot tests for:

- CST shapes;
- diagnostics;
- formatting;
- error recovery;
- exported LilyPond and MEI.

`insta` is well suited to reviewable snapshots of complex textual output. citeturn535118search0

## 17.2 Property tests

Use `proptest` for algebraic laws and generated score fragments. It is a QuickCheck-family property-testing framework
that shrinks failures to smaller counterexamples. citeturn535118search6

Examples:

```text
seq(empty, x) = x

seq(seq(x, y), z) = seq(x, seq(y, z))

transpose(0, x) = x

transpose(a, transpose(b, x))
  = transpose(a + b, x)

stretch(1, x) = x

span(seq(x, y)) = span(x) + span(y)

lower(seq(x, y))
  = sequence(lower(x), lower(y))
```

## 17.3 Formatting laws

```text
format(format(source)) = format(source)

semantic(parse(format(source)))
  = semantic(parse(source))
```

## 17.4 Backend tests

- validate generated MEI;
- compile generated LilyPond in CI;
- open generated MusicXML in at least two independent consumers;
- parse generated MIDI back through `midly`;
- compare deterministic backend snapshots.

## 17.5 DSP tests

Test:

- oscillator frequency and phase continuity;
- envelope stages;
- filter impulse and frequency response;
- delay timing;
- modulation ranges;
- graph cycle rejection;
- deterministic offline rendering;
- silence for disconnected graphs;
- absence of NaN and infinity propagation.

Instrument the real-time callback in tests to detect:

- allocations;
- locks;
- unexpected destruction;
- excessive processing time.

## 17.6 End-to-end fixtures

Maintain a small musical corpus:

- monophonic melody;
- two-part counterpoint;
- chords and independent voices;
- tuplets and ties;
- transformed motif;
- tempo change;
- dynamic and articulation interpretation;
- sine synthesizer with low-pass modulation;
- send reverb;
- multi-part LilyPond export.

The examples are executable specifications, not merely demos.

---

# 18. Implementation sequence

## Phase 1: vertical semantic slice

Deliver:

- one `.musa` file;
- lexer, parser, CST, and diagnostics;
- parts, voices, notes, rests, chords, and rational durations;
- motif declarations and finite reuse;
- transpose and repeat;
- expanded score with provenance;
- MEI generation;
- Verovio score preview;
- simple sine-polyphonic synthesizer;
- play, stop, seek, and loop;
- WAV and LilyPond export.

This phase proves the architecture.

## Phase 2: useful composition environment

Add:

- direct score commands;
- MIDI step entry;
- ties, slurs, dynamics, articulations, and tuplets;
- multi-staff score navigation;
- low-pass filter;
- ADSR;
- LFO modulation;
- delay, chorus, and reverb;
- mixer buses and sends;
- MusicXML export;
- project undo and autosave.

## Phase 3: deeper compositional leverage

Add:

- stretch, inversion, retrograde, and variation;
- motif extraction;
- occurrence specialization;
- phrase and form annotations;
- harmony annotations;
- theory libraries;
- performance profiles;
- tempo and expression curves;
- relative imports and shared patch libraries.

## Phase 4: external sound ecosystem

Only after the native system is stable:

- sample playback;
- SoundFont or similar basic orchestral support;
- audio-file clips for ambience;
- optional CLAP hosting;
- optional Audio Unit bridge on macOS;
- MusicXML import.

Audio recording and full waveform editing should remain outside the project unless the product’s purpose materially
changes.

---

# 19. Decisions to reject or defer

## “Everything should be generic over arbitrary musical theories”

Reject for the initial implementation.

It would spread theoretical abstraction through every data type and make ordinary notation harder. Preserve enough
information to support later theories, and place theoretical operations behind libraries and services.

## “The source language should be as powerful as Rust”

Reject.

General computation would compromise termination, incremental recompilation, diagnostics, graphical editing, and
semantic provenance.

## “Use LilyPond as the AST”

Reject.

LilyPond is an excellent output system, but it combines musical description, layout controls, language facilities, and
embedded Scheme. It is too backend-specific and too broad to be the canonical model.

## “Build a notation renderer in Rust”

Reject initially.

Verovio already provides high-quality interactive SVG rendering with retained MEI structure. Reimplementing music
engraving would not advance the distinctive part of the project.

## “Keep the GUI entirely in Rust”

Reject as a goal in itself.

Tauri, Verovio, and CodeMirror provide a much shorter path to an ergonomic score and source interface. The semantic and
audio systems remain Rust; the frontend is a replaceable projection.

## “Support every effect”

Reject.

A small compositional studio needs:

- oscillators;
- envelopes;
- filters;
- modulation;
- delay;
- chorus;
- reverb;
- gain, pan, sends, and buses.

A complete plugin ecosystem is a different product.

## “Make the score, MIDI, and audio graph one representation”

Reject categorically.

They have different identities, equivalences, timing models, and composition laws. Their coordinated appearance belongs
in the application layer, not the semantic model.

---

# 20. Final architecture

The final conceptual design is:

```text
                         .musa source
                              │
                    lossless parser/CST
                              │
                         compiler facade
                              │
            ┌─────────────────┴─────────────────┐
            │                                   │
   high-level musical model                 StudioSpec
 motifs, references, transforms       patches, effects, routing
            │                                   │
     finite elaboration                         │
            │                                   │
       ScoreSnapshot                            │
            │                                   │
      ┌─────┴──────────────┐                    │
      │                    │                    │
NotationPlan        PerformancePlan             │
      │                    │                    │
 ┌────┼─────┐              └──────────┬─────────┘
 │    │     │                         │
MEI  Lily  MusicXML             playback compiler
 │                                      │
Verovio SVG                         RenderPlan
 │                                      │
interactive score             CPAL live / Hound WAV
      \                                  /
       \                                /
        └──────── ProjectSession ──────┘
                    │
             semantic commands
                    │
         Tauri/Svelte application
```

The project’s distinctive contribution is not any individual box.

It is the **provenance-preserving composition model** that allows:

- notation;
- source-level motifs;
- algebraic transformations;
- immediate playback;
- sound synthesis;
- structured score editing;

to coexist without collapsing into one tangled representation.

That is narrow enough to build, deep enough to be worthwhile, and materially different from both a conventional notation
editor and a conventional DAW.
