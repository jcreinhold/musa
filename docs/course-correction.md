# Corrective Design Memo: Recenter Musa on a Small Temporal Kernel

## Purpose

Course-correct the current Musa implementation around a smaller and more principled semantic kernel.

The previous design has been drifting toward treating musician-facing concepts—motifs, repetitions, transpositions,
voices, rests, chords, and similar constructs—as increasingly fundamental compiler structures. Do **not** continue in
that direction.

The revised architecture should maintain two distinct pictures simultaneously:

1. **Musa source language:** expressive, concise, musician-oriented, and programmable.
2. **Musa temporal kernel:** very small, exact, backend-independent semantics into which the source language elaborates.

The kernel should not attempt to be a general-purpose programming language, nor should the surface language resemble a
declarative configuration DSL.

The purpose of the kernel is to give every downstream consumer one precise answer to:

> What musical facts exist, and where do they exist in musical time?

The purpose of the surface language is to let composers express those facts naturally.

---

# 1. Do not confuse surface concepts with semantic primitives

Musicians need concepts such as:

* notes;
* rests;
* chords;
* melodies;
* voices;
* parts;
* motifs;
* phrases;
* sections;
* repetitions;
* transformations;
* keys;
* meters;
* tempo changes;
* dynamics;
* loops;
* aleatory constructions.

This does **not** imply that each requires a corresponding kernel node.

In particular, do not continue growing a semantic enumeration of the form:

```text
Note
Rest
Chord
UseMotif
Repeat
Transpose
Arpeggiate
...
```

Every additional primitive becomes something that:

* the compiler must lower;
* every backend must understand;
* transformations must traverse;
* equality must account for;
* serialization must preserve;
* tests must special-case;
* future features must compose with.

A surface-language construct should earn kernel status only when it cannot be faithfully reduced to the existing
temporal semantics without losing information required by independent consumers.

---

# 2. The foundational correction: time is ambient

Do **not** model music as a tree of sounding and silent objects.

In particular, do not make:

```text
Silence(duration)
```

or:

```text
Rest(duration)
```

a primitive semantic alternative to sounding events.

The more fundamental model is:

> **Musical time exists independently of what occurs within it.**

A finite kernel object consists of:

1. an ambient region of exact musical time;
2. zero or more typed occurrences supported within that region.

If a region contains no note occurrence, that region is silent with respect to notes.

Nothing representing “silence” needs to exist there.

For example:

```text
timeline 4 beats {
    occurrence ... from 0 beats to 1 beat;
    occurrence ... from 2 beats to 4 beats;
}
```

contains no occurrence during:

```text
[1, 2)
```

That absence is enough.

A notation backend may later decide that the uncovered region of a particular notated voice requires a rest glyph.

That is a notation decision, not a kernel ontology.

---

# 3. Kernel semantic object

For a payload type `A`, the denotation of a finite kernel timeline is:

[ (d,E) ]

where:

[ d\in\mathbb Q_{\ge0} ]

is the extent of the ambient musical-time interval

[ [0,d], ]

and:

[ E ]

is a finite multiset of occurrences

[ (s,e,a) ]

such that:

[ 0\le s\le e\le d ]

and:

[ a:A. ]

Informally:

```text
Timeline[A] {
    duration
    occurrences
}
```

but do not treat this Rust-shaped notation as defining the source syntax.

An occurrence's duration comes from its temporal support:

```text
[start, end]
```

rather than being an intrinsic field of every musical payload.

For example, a `Note` describes what is sounding; its temporal occurrence describes when it sounds.

---

# 4. Exact musical time

Musical positions use exact rational beat time.

Use conceptually:

[ Time=\mathbb Q ]

and:

[ Duration=\mathbb Q_{\ge0}. ]

Do not use floating-point numbers as the canonical representation of symbolic musical time.

Global time positions form the abelian group:

[ (\mathbb Q,+,0). ]

Durations form the ordered commutative monoid:

[ (\mathbb Q_{\ge0},+,0). ]

This directly supports exact representation of:

* ordinary note divisions;
* tuplets;
* nested tuplets;
* unusual meters;
* arbitrary rational rhythmic relationships.

Physical seconds remain a separate domain introduced by performance realization.

---

# 5. The kernel has only three essential structural forms

The initial kernel should be centered on:

```text
timeline
sequence
overlay
```

plus named sharing/references if useful for serialization.

Conceptually:

```text
composition ::=
    timeline
    | sequence
    | overlay
    | reference
```

No primitive:

```text
note
rest
chord
motif
voice
repeat
transpose
key
tempo
```

is required at this level.

---

# 6. `timeline`

A `timeline` introduces an ambient finite temporal region and facts supported within it.

Example:

```text
timeline 4 beats {
    occurrence first_value from 0 beats to 1 beat;
    occurrence second_value from 2 beats to 4 beats;
}
```

The semantic checker requires:

[ 0\le start\le end\le duration. ]

Multiple equal occurrences are allowed.

Occurrences therefore form a **multiset**, not a set.

Two identical notes played by two performers must not collapse merely because all their visible values coincide.

---

# 7. `sequence`

`sequence` means temporal succession.

Given:

[ M=(d,E) ]

and:

[ N=(e,F), ]

define:

[
M;N
===

(d+e,\ E\uplus\tau_d(F)), ]

where:

[
\tau_d(s,t,a)
=============

(d+s,d+t,a). ]

So the second timeline is translated by the duration of the first.

Required laws:

[ (M;N);P=M;(N;P) ]

and:

[ 0;M=M=M;0, ]

where:

[ 0=(0,\varnothing). ]

Duration must satisfy:

[
duration(M;N)
=============

duration(M)+duration(N). ]

`sequence` is therefore associative temporal concatenation.

---

# 8. `overlay`

`overlay` means simultaneous presence in a common ambient timeline.

For:

[ M=(d,E) ]

and:

[ N=(e,F), ]

define:

[
M\oplus N
=========

(\max(d,e),E\uplus F). ]

Nothing is inserted into the uncovered portion of the shorter timeline.

The shorter timeline is simply regarded as living inside a larger ambient region.

Required laws:

[ M\oplus N=N\oplus M ]

and:

[
(M\oplus N)\oplus P
===================

M\oplus(N\oplus P). ]

Do **not** impose:

[ M\oplus M=M. ]

Multiplicity matters.

At any fixed duration (d), overlay forms a commutative monoid whose identity is:

[ (d,\varnothing). ]

---

# 9. Ambient extension is not silence padding

For:

[ d\le e, ]

define:

[
extend_{d,e}(d,E)
=================

(e,E). ]

No occurrence is introduced.

This is the correct interpretation of extending a temporal region.

Required coherence:

[ extend_{d,d}=id ]

and:

[
extend_{e,f}\circ extend_{d,e}
==============================

extend_{d,f}. ]

Overlay must respect ambient extension.

This concept is preferable to talking about “padding with silence.”

---

# 10. Sequence and overlay are not a semiring

Do not force attractive algebraic names or laws where the musical semantics does not support them.

In particular:

[ M;(N\oplus P) ]

contains one copy of (M), while:

[ (M;N)\oplus(M;P) ]

contains two.

Therefore distributivity generally fails:

[ M;(N\oplus P) \ne (M;N)\oplus(M;P). ]

The kernel is not naturally a ring or semiring.

Do not add algebraic structure merely because two binary operations exist.

---

# 11. Preserve the synchronized interchange law

There is, however, an important coherence law.

If:

[ duration(M)=duration(N) ]

and:

[ duration(P)=duration(Q), ]

then:

[
(M\oplus N);(P\oplus Q)
=======================

(M;P)\oplus(N;Q). ]

Musically:

```text
voice A, section 1    then    voice A, section 2
voice B, section 1            voice B, section 2
```

can be constructed either by:

* overlaying each section and sequencing the sections; or
* sequencing each voice and overlaying the complete voices.

The resulting temporal facts are identical.

This is an important compositional property and should have direct property tests.

Do not generalize this into unrestricted interchange. The synchronization conditions matter.

---

# 12. Payloads remain typed but musically opaque to the temporal kernel

The temporal kernel should support typed payloads without understanding their musical semantics.

Conceptually:

```text
Timeline[Note]
Timeline[KeyRegion]
Timeline[MeterRegion]
Timeline[Lyric]
Timeline[DynamicMark]
Timeline[SampleTrigger]
```

all use the same temporal machinery.

The temporal kernel knows:

```text
where
when
for how long
what typed value
```

but not what `Note`, `KeyRegion`, etc. mean.

Music-theoretic domains live above this level.

This is what allows Musa's temporal semantics to remain stable while its musical vocabulary grows.

---

# 13. Payload mapping is functorial, not another temporal primitive

Given:

[ f:A\to B, ]

there is a canonical induced transformation:

[ Timeline(f):Timeline\to Timeline[B] ]

that changes every payload while preserving temporal support.

Required laws:

[ Timeline(id)=id ]

and:

[
Timeline(g\circ f)
==================

Timeline(g)\circ Timeline(f). ]

It also preserves:

```text
sequence
overlay
```

This is the semantic basis for transformations such as transposition when they act only on payload information.

Do not add a generic opaque:

```text
Transform(...)
```

kernel node unless retaining that transformation itself is independently necessary.

Source provenance should preserve how the material was produced.

The normalized kernel should preserve what it means.

---

# 14. Time scaling is an external action

Positive rational scaling:

[ r\in\mathbb Q_{>0} ]

acts on a timeline by:

[ (d,E) \mapsto (rd,{(rs,re,a)}). ]

Required laws:

[ scale_1=id ]

and:

[ scale_r\circ scale_s=scale_{rs}. ]

Scaling preserves both sequence and overlay.

Thus augmentation/diminution can be evaluated into ordinary kernel timelines.

No permanent `Stretch` node is required in normalized kernel representation.

---

# 15. Delay is derived

A delayed timeline can be represented by extending the ambient before its occurrences.

Conceptually:

[
delay_b(M)
==========

(b,\varnothing);M. ]

Therefore delay does not need an independent primitive semantic constructor either.

Again, no silence object is involved.

---

# 16. Do not make `Timeline` a monad by assumption

The temporal construction is clearly functorial in its payload.

It does **not** have an obvious canonical monadic `join`.

For:

```text
Timeline[Timeline[A]]
```

there is no unique musically correct answer to how the inner timeline occupies the support of its outer occurrence.

Possible meanings include:

* begin the inner timeline at the outer onset;
* stretch the inner timeline to fit;
* crop it;
* repeat it;
* preserve its original duration;
* use outer or inner temporal structure.

These are genuinely different musical operations.

Therefore do not invent a universal flattening rule merely to obtain a familiar PL abstraction.

Specific higher-level musical abstractions may legitimately have monadic structure.

The universal temporal kernel need not.

---

# 17. Temporal locality and restriction

The kernel should support a clear notion of restricting an existing timeline to a bounded subinterval.

If an occurrence exists over:

[ [3,6) ]

and the requested observation window is:

[ [5,8), ]

the observer should know both:

```text
whole support   = [3, 6)
visible support = [5, 6)
```

Cropping must not falsely claim that the occurrence began at beat 5.

This suggests an observation representation containing:

```text
whole_span
visible_span
payload
```

even if the normalized serialized kernel only stores whole spans.

Restriction must obey:

[ restrict_I=id ]

and nested restriction composition:

[
restrict_K(restrict_J(M))
=========================

restrict_K(M) ]

for:

[ K\subseteq J\subseteq I. ]

This gives Musa temporal locality and makes finite-window observation mathematically coherent.

---

# 18. `Pattern` belongs above the finite kernel

Do not make `Pattern[A]` part of the initial finite kernel grammar.

Instead, define a higher-level pattern as something capable of producing coherent finite observations.

Conceptually, a pattern supplies:

[ P(I) ]

for every bounded musical-time interval (I), with the compatibility law:

[ J\subseteq I \Longrightarrow restrict_J(P(I))=P(J). ]

Thus:

```text
loop
algorithmic generator
aleatory realization
live-coded pattern
```

can all expose finite kernel observations without requiring identical computation models.

The kernel itself stays finite and interoperable.

---

# 19. Surface loops and repetitions should not be expanded eagerly

The surface/compiler HIR may preserve:

```text
repeat
loop
motif reference
transformation
```

for:

* efficiency;
* editing;
* provenance;
* diagnostics;
* structural display.

Do not eagerly duplicate thousands of nodes merely to obey the normalized kernel model.

Instead maintain the distinction:

```text
source / musical HIR
        ↓ evaluate or observe
finite temporal kernel
```

Normalization is a semantic boundary, not necessarily the internal representation used during every compiler pass.

---

# 20. Preserve provenance outside the semantic quotient

The temporal kernel intentionally forgets distinctions such as:

```text
this phrase was a motif
this was the fourth repetition
this came from transposing the original theme
this chord was written using chord syntax
```

Those distinctions matter to the editor and user.

They do not necessarily matter to MIDI or playback.

Therefore preserve them through:

* stable source IDs;
* provenance paths;
* source maps;
* HIR structure.

Do not pollute the temporal kernel solely to preserve editing history.

The kernel should be understood as a **semantic quotient** of richer source structure.

---

# 21. Key, meter, harmony, etc.

Do not create special temporal kernel forms for them.

They can be represented as typed values supported over time when their temporal extent matters.

For example, conceptually:

```text
Timeline[KeyRegion]
Timeline[MeterRegion]
Timeline[HarmonyAnnotation]
```

This does **not** claim that notes, keys, and meters have identical musical semantics.

It means only that:

> each can be associated with a region of musical time.

Their interpretation belongs to their own theory modules and downstream consumers.

This lets the surface language remain natural:

```text
key A minor
meter 4/4

...

modulate to C major {
    ...
}
```

while the temporal kernel remains generic.

---

# 22. Tempo is different

Tempo changes the interpretation of musical time into physical time.

Keep symbolic kernel positions in beats.

A performance layer supplies:

[ tempo: Beat\to Second ]

more precisely, a monotone map from musical positions into physical time.

An occurrence:

[ (s,e,a) ]

then realizes as:

[ (tempo(s),tempo(e),a). ]

Changing tempo therefore does not rewrite the symbolic temporal meaning of the composition.

Do not conflate:

```text
stretch musical material
```

with:

```text
perform the same material more slowly
```

The first changes the symbolic timeline.

The second changes its physical-time interpretation.

## Where the implementation stands (prompt 72)

There are two things called tempo, and this section is about the second one.

| | What it is | Who reads it |
| --- | --- | --- |
| The **marking** | `♩ = 92`, *Allegro*, written at a place in the score | the engraver, the exporters |
| The **map** | the monotone `Beat → Second` above | the performance plan, the engine |

The marking is a scoped context fact on the timeline, like a key, a meter or a clef: it has a place, it is printed,
and it is part of the piece's semantic identity. The map is not in the snapshot at all — `IntegratedTempoMap` builds
it in `musa-compiler/src/performance.rs` from the markings that carry a metronome, and from no others.

`tempo "Andante";` is what proves they are separate: it prints, it enters the timeline, and it contributes no
segment. A piece that states no metronome mark anywhere is performed at a quarter = 120, and that default lives in
the performance layer rather than in the score, because it is a fact about playing an unmarked page rather than a
fact about the page.

Until prompt 72 the implementation had one struct, `TempoMap`, that the notation planner and the performance lowerer
both read — the notated/performed collapse roadmap §2 forbids, and the reason a tempo-only edit did not move the
semantic hash and so did not reinstall the playback plan.

---

# 23. Audio remains a separate semantic layer

Do not force audio/DSP into the occurrence kernel.

Symbolic music is fundamentally:

```text
typed occurrences over musical time
```

Audio is fundamentally closer to:

[ Signal: PhysicalTime\to Sample. ]

The two should meet through a realization/instrument boundary.

Conceptually:

```text
temporal kernel
      ↓ tempo/performance
physical musical events
      ↓ instruments
audio signals
      ↓ DSP
output
```

This keeps score semantics and signal-processing semantics independently coherent.

> **Candidate refinement (prompt 92; not governing until prompt 137):** `docs/language/08-performance-and-sound.md`
> makes the realization/instrument boundary typed and explicit:
> `Timeline[ScoreFact] → GestureTimeline[InstrumentSignature] → ScheduledGestureLane → Signal → stereo mix`.
> Profiles interpret notation into exact musical gestures and semantic `ControlKey`s; instrument implementations map
> those controls privately to native graphs or sample maps. Part identity survives to its prepared instrument. This
> repairs the current shared-note-stream and ignored-parameter debts without moving audio, seconds, samples, or buses
> into the kernel.

---

# 24. Proposed kernel serialization grammar

Use clear names. Avoid unexplained shorthand such as `par`, `seq`, `atom`, etc.

The candidate syntax should revolve around:

```ebnf
kernel-file
    = "kernel", string-literal, "{",
          { declaration },
      "}"
    ;

declaration
    = payload-type-declaration
    | composition-declaration
    ;

composition-declaration
    = "composition", composition-name,
      ":", "Timeline", "[", payload-type, "]",
      "=",
      composition-expression,
      ";"
    ;

composition-expression
    = timeline-expression
    | sequence-expression
    | overlay-expression
    | composition-name
    | "(", composition-expression, ")"
    ;

timeline-expression
    = "timeline", duration-literal, "{",
          { occurrence-statement },
      "}"
    ;

occurrence-statement
    = "occurrence", payload-value,
      "from", position-literal,
      "to", position-literal,
      ";"
    ;

sequence-expression
    = "sequence", "{",
          composition-expression,
          ";",
          composition-expression,
          { ";", composition-expression },
      "}"
    ;

overlay-expression
    = "overlay", "{",
          composition-expression,
          ";",
          composition-expression,
          { ";", composition-expression },
      "}"
    ;
```

Complete the payload-schema/literal grammar separately but keep it first-order and deliberately boring.

The kernel file is an interchange/semantic language.

It is **not** the syntax musicians primarily write.

---

# 25. Canonical normal form

Every finite kernel composition must normalize to:

```text
timeline <duration> {
    occurrence <payload> from <start> to <end>;
    ...
}
```

with no remaining:

```text
sequence
overlay
references
```

Occurrence ordering must be canonical.

Kernel semantic equality is equality of these normalized forms, modulo canonical payload serialization.

This gives:

* deterministic serialization;
* straightforward golden tests;
* simple backend contracts;
* simple differential testing;
* cache-friendly semantic hashes.

---

# 26. Compiler architecture

The desired pipeline is now:

```text
Musa source
     ↓
lossless CST
     ↓
music-oriented HIR
     ↓
elaboration / evaluation
     ↓
finite temporal kernel
     ↓
ScoreSnapshot or replacement canonical adapter
     ↓
notation / analysis / performance
```

Do not expose the temporal kernel's low-level machinery directly throughout the entire compiler.

Use deep modules.

The temporal kernel should have a narrow public interface approximately around:

```text
construct/check timeline
sequence
overlay
restrict
normalize
compare
map payloads
scale time
```

Internal representation choices should remain hidden.

---

# 27. Relationship to the existing `ScoreSnapshot`

Do not discard the existing exact rational time and provenance work.

`ScoreSnapshot` is already close to the **normalized finite temporal denotation** required here.

The course correction is primarily conceptual and architectural:

* the source language should elaborate through a principled small temporal kernel;
* `ScoreSnapshot` should be evaluated as an implementation of, or adapter from, that normalized denotation;
* it should not become the place where arbitrary surface-language semantics accumulate.

If `ScoreSnapshot` contains note-specific assumptions that prevent it from serving as the generic temporal denotation,
retain it as the score-specific interpretation of the kernel rather than forcing generic temporal semantics into it.

Do not destabilize working notation code unnecessarily.

---

# 28. Relationship to `NotationPlan`

Keep the current `ScoreSnapshot → NotationPlan → backend` boundary where possible.

Notation should not understand:

* motif expansion;
* repetition semantics;
* source functions;
* looping constructs;
* arbitrary transformations.

It should consume already-resolved temporal facts and make notation-specific decisions.

The current separation between compositional semantics and backend-neutral notation planning is directionally correct.

Do not reverse it.

---

# 29. Immediate implementation changes

## Stop

Do not add more dedicated compiler/lowering cases for new musical concepts such as:

```text
retrograde
inversion
arpeggiation
variation
harmonization
new repetition forms
new motif operations
```

unless required merely to parse existing surface sugar.

Do not grow the existing direct CST-to-score lowering architecture into the permanent semantic model.

## Preserve

Keep:

* lossless parsing;
* formatting;
* source spans;
* exact rational time;
* provenance;
* current working `ScoreSnapshot`;
* current `NotationPlan`;
* notation backends that consume only `NotationPlan`.

## Add

Introduce a deep temporal-kernel module/crate responsible for:

* exact temporal domains;
* typed occurrences;
* sequence;
* overlay;
* restriction;
* normalization;
* semantic equality;
* relevant algebraic property tests.

Do **not** split each concept into its own microcrate.

---

# 30. Migration strategy

Do not rewrite Musa all at once.

### Step 1 — Write the kernel specification

Before adding significant new semantic source constructs, add authoritative documentation for:

```text
docs/kernel/
    00-purpose.md
    01-grammar.md
    02-static-semantics.md
    03-denotational-semantics.md
    04-algebraic-laws.md
    05-normalization.md
    06-surface-elaboration.md
    07-backend-contract.md
    08-open-questions.md
```

Clearly mark the kernel as a candidate until the following tests pass.

### Step 2 — Implement the temporal algebra independently

Implement:

```text
Timeline<A>
Occurrence<A>
sequence
overlay
restrict
extend
scale
normalize
```

without changing source syntax.

### Step 3 — Property-test the laws

At minimum:

```text
sequence associativity
sequence zero identity

overlay associativity
overlay commutativity
overlay fixed-duration identity
overlay non-idempotence

ambient-extension identity/composition

restriction identity/composition

payload-map identity/composition

time-scaling identity/composition

time-scaling preservation of sequence
time-scaling preservation of overlay

synchronized interchange
```

Also add explicit tests showing that distributivity is **not** assumed.

### Step 4 — Elaborate existing source constructs

Translate the current surface concepts into the candidate kernel.

Examples:

```text
note
    → typed occurrence

rest
    → advance/construct ambient temporal extent with no note occurrence

chord
    → simultaneous note occurrences

voice
    → surface/HIR organization and labels

motif
    → binding/reference + provenance

repeat
    → repeated structural source/HIR, observed into temporal kernel

transpose
    → payload-level transformation before normalization
```

Do not require immediate syntax changes.

### Step 5 — Differentially test against the existing lowerer

For existing examples, compare:

```text
old source → ScoreSnapshot
```

against:

```text
source → HIR → temporal kernel → ScoreSnapshot
```

They must agree on:

* exact positions;
* durations/support;
* pitch spelling;
* part/voice identity where preserved;
* multiplicity;
* ordering where semantically relevant;
* provenance mapping.

Keep the old lowering path as a regression oracle until parity is established.

### Step 6 — Switch the canonical semantic path

Only after differential parity should the new elaboration path replace direct semantic lowering.

---

# 31. Surface-language design was left open; a candidate now exists

Do **not** mistake this memo for a final surface syntax specification. Prompt 92 now supplies the precise candidate at
`docs/language/`, including settled punctuation, a total higher-order value calculus, contextual and context-neutral
`music`, declaration templates, static modules, and typed kernel quotation. It is the implementation contract for
prompts 93–136 but remains lower precedence than this memo until prompt 137 audits and graduates it.

The user-facing language should remain free to be substantially richer and more musician-oriented than the kernel.

Potential surface constructs include:

```text
piece
part
voice
motif
phrase
section
chord
key
meter
tempo
repeat
loop
transpose
```

and ordinary compositional programming facilities.

The important rule is:

> Surface richness must usually elaborate into existing kernel semantics rather than continuously enlarging the kernel.

Specialized musician-friendly syntax is desirable when it improves composition.

The anti-pattern is not “domain-specific syntax.”

The anti-pattern is **domain-specific semantic accretion without a stable lower algebra**.

The candidate respects that rule: every score construction closes to the existing `Term[ScoreFact]`, and its sound and
asset declarations remain downstream consumers. The fact that the candidate settles a surface choice is not authority
to add a kernel constructor.

---

# 32. Open questions — do not prematurely decide

The following remain intentionally unresolved:

### Infinite/live patterns

Treat them initially as producers of coherent finite kernel observations. Do not add them to the finite kernel until
necessary.

### Aleatory semantics — **resolved (prompt 66)**

Probability, nondeterminism, performer choice, and reactive improvisation are not the same phenomenon, and there is no
one universal `Choice` kernel construct — that warning stands, and prompt 66 acted on it rather than around it.

The answer is that **a realization is a compile parameter and the freedom is a payload value**. What is written in the
source — an open repeat, a boxed fragment, a free duration, an improvised region — survives into the timeline as
ordinary occurrences, so the page can print the instruction; what is *decided* is a seed plus an override set, supplied
to elaboration, so that by the time a `Term` exists every choice is made and evaluation is still total, deterministic
and hashable. The candidate `choose` term form is refused in `docs/kernel/11-realization.md`, with the four reasons it
fails: it cannot express its own repertoire, it makes T2 ambiguous, it destroys T3/T4/N6 together, and it destroys the
interchange file that was the argument for it.

**Item 11 below is upheld, not amended.** It forbids aleatory choice *in the finite kernel*, which is precisely what
this resolution does. The price paid instead is stated openly: the score view is a function of the source *and* the
realization, so fixtures pin a seed and the interface must be able to show the decisions.

### Voice identity

Determine whether voice identity is best represented as:

* payload metadata;
* a separate temporal relation;
* HIR structure plus provenance;
* or some combination.

Do not promote it to a primitive temporal operation without evidence.

### Time-varying continuous controls — **resolved (prompt 45)**

The question was whether automation belongs as typed interval payloads, a separate behavior/curve layer, or the
performance/audio model.

It is a **typed payload value**: `Progress`, a monotone piecewise-linear map from an occurrence's *normalized local*
time to a unit-free fraction in `[0, 1]`. It cost the kernel no new operation and changed no existing law, because
indexing by local time makes every operation act on the span and leave the payload untouched (the span-alone theorem,
`docs/kernel/03-denotational-semantics.md`, tested as L24). A behavior layer would have been a second way to say what
occurrences already say; an absolute-time curve would have forced the kernel to look inside payloads, violating §12.

The warning that produced this section still stands for what remains: `Progress` says *how far along*, never how loud or
how fast, and it expresses no steps, no units, no periodic shapes. Continuous control was not forced into discrete
occurrences — the occurrence supplies the span, the value supplies the shape, and the meaning stays above the kernel.

### Recursive/generative source programs

The surface language may eventually need recursion or other generative facilities. This does not imply that the finite
kernel needs them.

---

# 33. Falsification tests for the kernel

Before declaring this architecture settled, use materially different musical examples.

The candidate kernel should support clean elaboration of at least:

1. **Twinkle Twinkle Little Star** Ordinary sequential pitched material and rests.

2. **A four-part chorale** Multiple synchronized voices and harmonic simultaneity.

3. **A canon** Reuse, delay, transformation, and overlay.

4. **Tuplets and polyrhythm** Exact rational temporal relationships.

5. **Changing meter and key** Contextual temporal information without semantic special cases. *(Proven at prompts
   63–64, and pushed to its edge at prompt 74: `meter none` is a **value** of the meter context, not a mechanism
   beside it, so music with no barlines needed no kernel form, no second time coordinate, and no new special case.
   `examples/changing-meter.musa`, `examples/modulation.musa`, `examples/cadenza.musa`, `examples/chant.musa`.)*

6. **Accelerando/ritardando** Distinguish symbolic beat structure from physical-time realization. *(Proven at
   prompt 73: a gradual change is a `Progress` in the tempo marking's payload, integrated exactly at realization
   and printed at both ends on the page. The symbolic timeline does not move — no notehead changes place —
   which is the distinction stated as a test. `examples/rubato.musa`, `examples/riser.musa`.)*

7. **Glissando/crescendo** Determine where continuous temporal behavior belongs. *(Proven at prompt 44: a shape is
   a `Progress` in the payload, not a term form. `examples/annotated.musa`.)*

8. **Loop-based electronic music** Surface iteration producing finite observations. *(Proven at prompt 67: a ranged
   repeat is decided once at compile time and everything below it is an ordinary exact repeat.
   `examples/loop-lengths.musa`.)*

9. **Controlled aleatory** Multiple possible realizations producing ordinary finite kernels. *(Design settled at
   prompt 66, implemented at 67, given a surface at 68: `docs/kernel/11-realization.md`. Proven by
   `examples/mobile.musa` — nineteen fragments, 19! orderings, one permutation in the payload — and by
   `examples/in-c.musa`, fifty-three decision sites that survive an edit to each other.)*

10. **An improvisational/live process** Verify that the finite kernel remains a useful observation/interchange target
    even when the producer is reactive. *(Proven at prompt 68 as far as it can be, and no further:
    `examples/changes.musa` writes the improvised chorus as a frame of the right length with the changes on it, so
    the interchange file holds a complete, finite, exactly-timed piece and the instruction a player needs. What is
    **not** proven, and is refused rather than deferred, is a reactive producer: musa compiles a reading of the
    work, it does not follow one. See prompt 68's Stop list.)*

11. **Polymeter and polytempo** Parts counted and paced independently of the score around them. *(Proven at
    prompt 75, and the item is worth reading for how little it cost: `Meter` and `Tempo` already inherited by
    `Override` and `BarLines` was already built on an arbitrary sequence of meters, so both are a **scope argument**
    — `bars(scope)`, `IntegratedTempoMap::new(score, scope, …)` — and neither is a kernel form, a term, or a second
    algorithm. `examples/bulgarian.musa` (7/8 against 4/4, barlines that diverge), `examples/hemiola.musa` (6/8
    against 3/4, one grid beamed two ways), `examples/canon-x.musa` (Nancarrow's shape: one part accelerating while
    the other decelerates). The lossy lowering this item exists to expose is real and is exactly one: SMF has one
    tempo track, so a polytempo export is sonically exact and notationally wrong, and says so.)*

If several of these require awkward or lossy lowering, reconsider the kernel.

Do not patch each example independently.

---

# 34. Governing design rule

Use this rule for every proposed new kernel feature:

> **A construct belongs in the kernel only if removing it makes an important class of musical meanings impossible or
> unnatural to represent faithfully across multiple independent consumers.**

“Musicians use this concept” is not enough.

“It's convenient to parse this way” is not enough.

“It's easier to implement this feature as another enum variant” is not enough.

The burden is semantic necessity.

Conversely, do not worship minimality. If a concept repeatedly requires convoluted encodings, duplicated conventions, or
backend-specific reconstruction, that is evidence that the kernel is missing a genuine primitive.

The target is not the *fewest constructors*.

The target is the **smallest complete semantic basis**.

---

# 35. Immediate directive

For the next implementation phase:

1. **Freeze semantic grammar growth.**
2. **Do not rewrite working notation infrastructure.**
3. **Specify and implement the finite temporal kernel described above.**
4. **Treat ambient musical time—not sounding/silent objects—as foundational.**
5. **Use exact rational beat time.**
6. **Make `timeline`, `sequence`, and `overlay` the initial structural basis.**
7. **Keep payload semantics outside the temporal kernel.**
8. **Preserve source structure and provenance above the normalized kernel.**
9. **Elaborate existing Musa syntax into the kernel without requiring an immediate source-language redesign.**
10. **Differentially validate the new path against the existing lowering behavior before replacing it.**
11. **Do not add `Pattern`, recursion, aleatory choice, DSP, or general-purpose language machinery to the finite kernel
    merely to anticipate future features.**
12. **Use real musical examples to falsify the kernel before extending it.**

The intended architecture is:

```text
musician-facing Musa source
            │
            ▼
    rich compositional HIR
            │
            │ elaboration / finite observation
            ▼
┌───────────────────────────────┐
│      TEMPORAL KERNEL          │
│                               │
│ exact ambient musical time    │
│ typed temporal occurrences    │
│ sequence                      │
│ overlay                       │
│ restriction / normalization   │
└───────────────┬───────────────┘
                │
        normalized timeline
                │
      ┌─────────┼─────────┐
      ▼         ▼         ▼
   notation   analysis  performance
                          │
                    tempo realization
                          │
                          ▼
                         audio
```

This is the course correction.

Do not continue treating the present surface grammar as if it defines Musa's ontology.

The surface language should be designed for composers.

The temporal kernel should be designed for semantics.
