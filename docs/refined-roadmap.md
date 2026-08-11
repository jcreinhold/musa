# Musa Temporal Kernel — Candidate Grammar and Denotational Semantics

## 1. Purpose

The Musa kernel represents **typed facts supported in an ambient region of exact musical time**.

It deliberately does not contain primitive concepts for:

* note;
* rest;
* chord;
* voice;
* motif;
* key;
* meter;
* tempo;
* loop;
* transposition;
* harmony;
* probability;
* audio.

Those concepts belong to the surface language and domain libraries.

The kernel knows only:

1. exact musical time;
2. an ambient timeline;
3. typed occurrences supported within that timeline;
4. temporal succession;
5. temporal superposition.

Everything else must either:

* elaborate into these operations;
* live in the payload carried by an occurrence;
* or belong to another semantic layer, such as audio.

The design goal is not minimum syntax for its own sake. The goal is the **smallest semantic vocabulary that every
backend can agree on**.

---

# 2. Central semantic idea

A kernel term denotes:

[ (d,E) ]

where:

* (d\in\mathbb Q_{\ge0}) is the extent of the ambient musical-time region;
* (E) is a finite multiset of typed occurrences supported inside that region.

Thus the ambient exists independently of its contents.

For example:

```text
timeline 1 beat {
    occurrence ... from 0 beats to 1/4 beats;
    occurrence ... from 1/2 beats to 1 beat;
}
```

contains nothing during:

[ [1/4,1/2). ]

That absence is what a notation backend may later render as a rest.

There is no kernel value called `silence`.

---

# 3. Complete EBNF

The following grammar is intentionally first-order.

It includes a small payload-schema language so a kernel file is syntactically self-contained, but it contains no
general-purpose functions, recursion, classes, traits, effects, or user-defined operators.

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


(* ---------- Payload schemas ---------- *)

payload-type-declaration
    = "payload", type-name, "=",
      payload-type-definition,
      ";"
    ;

payload-type-definition
    = payload-type
    | record-type
    | variant-type
    ;

payload-type
    = "Boolean"
    | "Integer"
    | "Rational"
    | "Text"
    | type-name
    | "List", "[", payload-type, "]"
    ;

record-type
    = "record", "{",
          [ field-declaration,
            { ",", field-declaration } ],
      "}"
    ;

field-declaration
    = field-name, ":", payload-type
    ;

variant-type
    = "variant", "{",
          variant-case,
          { ",", variant-case },
      "}"
    ;

variant-case
    = constructor-name,
      [ "(",
            [ payload-type,
              { ",", payload-type } ],
        ")" ]
    ;


(* ---------- Compositions ---------- *)

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


(* ---------- Ambient timeline ---------- *)

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


(* ---------- Temporal composition ---------- *)

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


(* ---------- Payload values ---------- *)

payload-value
    = boolean-literal
    | integer-literal
    | rational-literal
    | string-literal
    | record-value
    | variant-value
    | list-value
    ;

record-value
    = "{",
          [ field-assignment,
            { ",", field-assignment } ],
      "}"
    ;

field-assignment
    = field-name, "=", payload-value
    ;

variant-value
    = constructor-name,
      [ "(",
            [ payload-value,
              { ",", payload-value } ],
        ")" ]
    ;

list-value
    = "[",
          [ payload-value,
            { ",", payload-value } ],
      "]"
    ;


(* ---------- Musical time ---------- *)

duration-literal
    = nonnegative-rational, beat-unit
    ;

position-literal
    = nonnegative-rational, beat-unit
    ;

beat-unit
    = "beat"
    | "beats"
    ;


(* ---------- Literals ---------- *)

boolean-literal
    = "true"
    | "false"
    ;

integer-literal
    = [ "-" ], natural-number
    ;

rational-literal
    = integer-literal, "/", positive-natural-number
    ;

nonnegative-rational
    = natural-number
    | natural-number, "/", positive-natural-number
    ;

natural-number
    = "0"
    | positive-natural-number
    ;

positive-natural-number
    = nonzero-digit, { digit }
    ;

string-literal
    = '"', { string-character }, '"'
    ;


(* ---------- Identifiers ---------- *)

type-name
    = upper-letter, { letter | digit | "_" }
    ;

constructor-name
    = upper-letter, { letter | digit | "_" }
    ;

field-name
    = lower-letter, { letter | digit | "_" }
    ;

composition-name
    = lower-letter, { letter | digit | "_" }
    ;

string-character
    = letter
    | digit
    | "_"
    | "-"
    | " "
    | "."
    | "#"
    | "/"
    | ":"
    ;

letter
    = upper-letter
    | lower-letter
    ;

upper-letter
    = "A" | "B" | "C" | "D" | "E" | "F" | "G"
    | "H" | "I" | "J" | "K" | "L" | "M" | "N"
    | "O" | "P" | "Q" | "R" | "S" | "T" | "U"
    | "V" | "W" | "X" | "Y" | "Z"
    ;

lower-letter
    = "a" | "b" | "c" | "d" | "e" | "f" | "g"
    | "h" | "i" | "j" | "k" | "l" | "m" | "n"
    | "o" | "p" | "q" | "r" | "s" | "t" | "u"
    | "v" | "w" | "x" | "y" | "z"
    ;

nonzero-digit
    = "1" | "2" | "3" | "4" | "5"
    | "6" | "7" | "8" | "9"
    ;

digit
    = "0"
    | nonzero-digit
    ;
```

The lexer may support comments and whitespace without affecting the abstract grammar.

---

# 4. What the grammar deliberately says

A kernel composition has only three actual structural forms:

```text
timeline
sequence
overlay
```

A composition name is only sharing.

An occurrence exists **inside a `timeline`**, not as an independent musical term.

That is intentional.

The grammar prevents us from accidentally returning to:

```text
Note
Rest
Chord
Motif
Voice
KeyChange
TempoChange
...
```

as competing subclasses of “music.”

---

# 5. Static well-formedness

EBNF gives syntax, not correctness. The kernel checker imposes the following judgments.

Write:

[ \Sigma\vdash A;\mathsf{payload} ]

for a valid payload type and

[ \Sigma\vdash M:\operatorname{Timeline}[A] ]

for a well-typed composition.

## 5.1 Payload types

The primitive carrier types are:

[
\begin{aligned}
\llbracket Boolean\rrbracket &= {false,true}\
\llbracket Integer\rrbracket &= \mathbb Z\
\llbracket Rational\rrbracket &= \mathbb Q\
\llbracket Text\rrbracket &= \text{finite Unicode strings}.
\end{aligned}
]

`List<A>` denotes finite lists of values of (A).

A record denotes a Cartesian product.

A variant denotes a disjoint sum.

For the initial kernel, named payload definitions must be non-recursive. Recursive or richer musical structures belong
above the kernel unless a real interchange requirement justifies them.

## 5.2 Timeline literals

For:

```text
timeline d {
    occurrence a from s to e;
    ...
}
```

the checker requires:

[ 0\le s\le e\le d. ]

Every occurrence payload must have the timeline's payload type.

Equal endpoints (s=e) are permitted and denote an instantaneous occurrence.

No requirement says that every portion of ([0,d]) contains an occurrence.

## 5.3 Sequence and overlay

Every operand must have the same payload type:

[ M_i:\operatorname{Timeline}[A]. ]

The resulting timeline also has payload type (A).

## 5.4 References

The composition-reference graph must be acyclic in the kernel.

This is **not** a restriction on the Musa surface language.

It guarantees that a kernel document denotes a finite, normalizable temporal object. Infinite loops, recursive
generators and live processes are observed over a finite window by the surface/runtime and then lowered into a finite
kernel document.

That makes the kernel suitable as an interchange representation.

---

# 6. Time domain

Let musical positions be rational:

[ T=\mathbb Q. ]

Under addition,

[ (T,+,0) ]

is an abelian group.

This is the appropriate global algebra for translations of musical time.

Kernel-local timelines begin at zero, so their lengths use only:

[ D=\mathbb Q_{\ge0}. ]

Under addition:

[ (D,+,0) ]

is a commutative, cancellative monoid.

It is **not** a group, because negative duration is meaningless.

Positive rational scaling acts on both positions and durations:

[ \mathbb Q_{>0}\curvearrowright T,\qquad r\cdot t=rt ]

and

[ \mathbb Q_{>0}\curvearrowright D. ]

Thus the orientation-preserving rational affine group

```text
[
\operatorname{Aff}^+(\mathbb Q)
===============================

\mathbb Q\rtimes\mathbb Q_{>0} ]
```

acts on global musical time by

[ t\mapsto rt+b. ]

This supplies the mathematics underlying operations such as:

* shifting;
* augmentation;
* diminution;
* global time rescaling.

These need not be primitive kernel syntax: a surface transformation can apply the action and emit the transformed kernel
timeline.

---

# 7. Denotation of a timeline

Fix a payload carrier set (A).

For duration (d\in D), define the possible occurrences:

```text
[
G_A(d)
======

{(s,e,a)\mid 0\le s\le e\le d,; a\in A }. ]
```

Define:

```text
[
\mathcal T_A(d)
===============

\operatorname{FinMultiset}(G_A(d)). ]
```

A timeline of duration (d) is therefore a finite multiset of typed occurrences supported in the ambient interval
([0,d]).

The complete denotation is:

```text
[
\mathcal T_A
============

\coprod_{d\in D}\mathcal T_A(d). ]
```

An element is written:

[ (d,E). ]

The ambient duration (d) exists independently of (E).

That is precisely why no silence constructor is required.

---

# 8. Denotation of `timeline`

For:

```text
timeline d {
    occurrence a1 from s1 to e1;
    ...
    occurrence an from sn to en;
}
```

define:

```text
[
\llbracket M\rrbracket
======================

\left( d,; [(s_1,e_1,\llbracket a_1\rrbracket), \ldots, (s_n,e_n,\llbracket a_n\rrbracket)] \right). ]
```

The brackets denote a multiset, not a set.

Therefore two identical occurrences remain two occurrences.

This is important musically:

> two performers sounding identical material are not automatically the same occurrence.

---

# 9. Empty ambient time

The literal

```text
timeline 4 beats {
}
```

denotes

[ (4,\varnothing). ]

This does **not** denote a primitive musical object called silence.

It denotes:

> a four-beat ambient temporal region with no occurrences.

A notation layer may interpret uncovered regions of a particular voice as rest notation.

An audio layer may interpret absence of sounding material differently.

The kernel makes neither choice.

---

# 10. Sequence

Suppose:

[ \llbracket M\rrbracket=(d,E) ]

and

[ \llbracket N\rrbracket=(e,F). ]

Define translation of an occurrence multiset by (d):

```text
[
\tau_d(F)
=========

{(d+s,d+t,a)\mid(s,t,a)\in F}. ]
```

Then:

```text
[
\boxed{
\llbracket sequence{M;N}\rrbracket
==================================

(d+e,;E\uplus\tau_d(F)). } ]
```

Here (\uplus) is multiset union.

For multiple operands, iterate this definition from left to right.

## Laws

Sequence is associative:

```text
[
(M;N);P
=======

M;(N;P) ]
```

denotationally.

Its identity is:

```text
timeline 0 beats {}
```

because:

```text
[
(0,\varnothing);(d,E)
=====================

# (d,E)

(d,E);(0,\varnothing). ]
```

Duration is a monoid homomorphism:

```text
[
\operatorname{duration}(M;N)
============================

\operatorname{duration}(M) + \operatorname{duration}(N). ]
```

Therefore:

[ (\mathcal T_A,,;,,0) ]

is a monoid graded by the duration monoid (D).

This is the kernel's fundamental algebra of **succession**.

---

# 11. Overlay

Suppose:

[ M=(d,E),\qquad N=(e,F). ]

Define:

[ m=\max(d,e). ]

The shorter timeline is simply regarded as content over the larger ambient interval; no new occurrence is inserted.

Then:

```text
[
\boxed{
M\oplus N
=========

(m,;E\uplus F). } ]
```

This is `overlay`.

## Laws

Overlay is associative:

```text
[
(M\oplus N)\oplus P
===================

M\oplus(N\oplus P). ]
```

It is commutative:

[ M\oplus N=N\oplus M. ]

Its global identity is the zero-duration empty timeline:

[ M\oplus(0,\varnothing)=M. ]

It is **not idempotent**:

[ M\oplus M\ne M ]

in general.

At a fixed duration (d),

[ \mathcal T_A(d) ]

is the free commutative monoid generated by (A)-occurrences over ([0,d]).

Its identity is:

[ 0_d=(d,\varnothing). ]

This is the algebra of **simultaneous presence**.

---

# 12. Ambient extension

If

[ d\le e, ]

there is a canonical map:

[ \operatorname{extend}_{d,e}: \mathcal T_A(d)\to\mathcal T_A(e) ]

defined by:

```text
[
\operatorname{extend}_{d,e}(d,E)
================================

(e,E). ]
```

Nothing is added.

This is the formal meaning of extending ambient time.

It is not padding with a silence event.

For each (d\le e\le f):

```text
[
\operatorname{extend}*{e,f}
\circ
\operatorname{extend}*{d,e}
===========================

\operatorname{extend}_{d,f}. ]
```

And:

[ \operatorname{extend}_{d,d}=id. ]

Each extension preserves overlay:

```text
[
\operatorname{extend}_{d,e}(M\oplus N)
======================================

\operatorname{extend}*{d,e}(M) \oplus \operatorname{extend}*{d,e}(N). ]
```

So the fixed-duration overlay monoids form a covariant system over the ordered duration domain.

---

# 13. Sequence and overlay do **not** form a semiring

This is an important non-law.

One might hope that overlay behaves like addition and sequence like multiplication.

But:

[ M;(N\oplus P) ]

contains one copy of (M).

Whereas:

[ (M;N)\oplus(M;P) ]

contains two copies of (M).

Therefore:

[ M;(N\oplus P) \ne (M;N)\oplus(M;P) ]

in general.

Likewise on the other side.

So the kernel is not naturally a ring or semiring.

There are:

* no additive inverses;
* no useful musical subtraction;
* no distributivity.

Forcing a semiring structure would identify genuinely different musical objects.

---

# 14. The synchronized interchange law

There is, however, one important law connecting succession and simultaneity.

Suppose:

[
\begin{aligned}
\operatorname{duration}(M)
&=
\operatorname{duration}(N)=d,\
\operatorname{duration}(P)
&=
\operatorname{duration}(Q)=e.
\end{aligned}
]

Then:

```text
[
\boxed{
(M\oplus N);(P\oplus Q)
=======================

(M;P)\oplus(N;Q). } ]
```

Both sides contain:

* (M,N) during the first (d) beats;
* (P,Q) translated by (d).

This is exactly the coherence we want for aligned voices:

```text
soprano section 1     soprano section 2
alto section 1        alto section 2
```

can be assembled either by:

1. overlaying voices inside each section and sequencing sections; or
2. sequencing each voice and then overlaying the complete voices.

The result is identical.

## Why the law is conditional

We must **not** require unrestricted interchange.

If two unital monoid structures on the same carrier share a unit and satisfy full interchange, the Eckmann–Hilton
argument forces them to coincide and become commutative.

That would imply, absurdly:

[ sequence=overlay ]

and sequence would be commutative.

Therefore the synchronization hypotheses are mathematically load-bearing.

We have a meaningful **graded/interchange structure**, not a full 2-monoid or indiscriminate duoidal algebra.

---

# 15. Categorical temporal locality

There is another categorical structure that is more fundamental than forcing sequence into an elaborate category.

Let (\mathcal I) be the poset-category of bounded rational time intervals:

* objects: intervals (I\subseteq\mathbb Q);
* one morphism (J\to I) exactly when (J\subseteq I).

For every payload type (A), observations of musical occurrences over intervals form a contravariant assignment:

[ \mathcal O_A: \mathcal I^{op}\to\mathbf{CMon}. ]

For interval (I):

[ \mathcal O_A(I) ]

is the commutative monoid of occurrence views visible in (I).

For:

[ J\subseteq I, ]

restriction:

[ \rho_{I,J}: \mathcal O_A(I)\to\mathcal O_A(J) ]

clips each occurrence to the smaller observation window.

Restriction obeys:

[ \rho_{I,I}=id ]

and:

```text
[
\rho_{J,K}\circ\rho_{I,J}
=========================

\rho_{I,K}. ]
```

Thus:

> observing a smaller window directly is equivalent to observing a larger window and then restricting it.

This is the categorical form of temporal locality.

Tidal's modern pattern representation is built around almost exactly this operational idea: rational time, finite
interval queries, and events retaining both their whole span and the part visible in the query.

---

# 16. Whole occurrence versus visible occurrence

To make restriction lossless, an observed occurrence should conceptually retain:

[ (\text{whole span},\text{visible span},a). ]

Suppose a note exists on:

[ [3,6) ]

but the current observation window is:

[ [5,8). ]

Its observation is:

[ (,[3,6),,[5,6),,a,). ]

Thus cropping does not falsely turn the event into one that began at beat 5.

This should belong to the normalized/query API even if the serialized kernel timeline stores only the whole support.

Again, this mirrors Tidal's distinction between the complete arc of an event and the currently visible part.

---

# 17. Patterns become an extension **over** the kernel

This gives a clean definition of a potentially unbounded pattern without putting it into the finite kernel grammar.

A pattern of (A) is approximately a compatible family:

[ P(I)\in\mathcal O_A(I) ]

for every bounded interval (I), satisfying:

[ J\subseteq I \implies \rho_{I,J}(P(I))=P(J). ]

So:

```text
[
\boxed{
Pattern[A]
==========

\text{compatible finite kernel observations over bounded time windows.} } ]
```

The kernel remains finite.

A loop, algorithmic generator, live-coded pattern, or aleatory realization can implement this interface.

Tidal likewise represents potentially indefinite musical patterns by answering finite time-range queries rather than
materializing an infinite event list.

Whether the compatible-family assignment satisfies a full sheaf gluing condition is a separate question. It likely can
when occurrence identity and whole-span information are retained, but **presheaf compatibility is enough for the kernel
design and should be the current claim**.

---

# 18. Functoriality in the payload

The temporal structure is independent of what occurs.

For any pure function:

[ f:A\to B, ]

define:

[ \mathcal T(f): \mathcal T_A\to\mathcal T_B ]

by:

[ (d,E) \mapsto (d,{(s,e,f(a))\mid(s,e,a)\in E}). ]

Then:

[ \mathcal T(id_A)=id_{\mathcal T_A} ]

and:

```text
[
\mathcal T(g\circ f)
====================

\mathcal T(g)\circ\mathcal T(f). ]
```

So temporal timelines are **functorial in their payload**.

They also preserve both principal operations:

```text
[
\mathcal T(f)(M;N)
==================

\mathcal T(f)(M); \mathcal T(f)(N) ]
```

and:

```text
[
\mathcal T(f)(M\oplus N)
========================

\mathcal T(f)(M) \oplus \mathcal T(f)(N). ]
```

This is the mathematical reason that things like transposition should normally live outside the temporal kernel.

If:

[ transpose_{P5}:Note\to Note, ]

then its action on complete music is induced automatically.

Hudak's temporal-media work and Haskore exploit closely related separation between the temporal algebra and the type of
musical material carried by it.

---

# 19. Time-scaling action

For:

[ r\in\mathbb Q_{>0}, ]

define:

```text
[
S_r(d,E)
========

\left( rd,; {(rs,re,a)\mid(s,e,a)\in E} \right). ]
```

Then:

[ S_1=id ]

and:

[ S_r\circ S_s=S_{rs}. ]

Thus positive rational scaling acts on timelines.

It respects sequence:

```text
[
S_r(M;N)
========

S_r(M);S_r(N), ]
```

and overlay:

```text
[
S_r(M\oplus N)
==============

S_r(M)\oplus S_r(N). ]
```

So augmentation and diminution are not arbitrary compiler rewrites; they arise from a genuine group action.

A surface operation such as:

```text
stretch_time(2, phrase)
```

can therefore elaborate by this semantic action and emit an ordinary kernel timeline.

No `Stretch` kernel node is required.

---

# 20. Translation

For nonnegative (b), define delayed placement:

```text
[
D_b(d,E)
========

(d+b,\tau_b(E)). ]
```

But:

```text
[
D_b(M)
======

( b,\varnothing ); M. ]
```

So delay is **derived from ambient time plus sequence**.

It should not be a kernel primitive.

For global absolute coordinates, all (b\in\mathbb Q) are allowed and the additive group of time translations acts
normally.

---

# 21. Why there is no canonical monad

The construction is clearly functorial:

[ A\mapsto\mathcal T_A. ]

It is tempting to demand a monad:

[ join: \mathcal T_{\mathcal T_A} \to \mathcal T_A. ]

But there is no canonical meaning for that operation.

Suppose an outer occurrence contains an inner timeline.

Should the inner timeline:

* start at the outer occurrence's onset?
* be scaled to fit its outer support?
* be clipped to the support?
* repeat within the support?
* retain its own absolute duration?
* determine the resulting event structure itself?

All are musically useful.

None is universally correct.

Tidal encounters precisely this issue when combining differently structured patterns: there are several meaningful
choices for which pattern's temporal structure governs the combination, and modern Tidal exposes the distinction rather
than pretending there is one universal flattening operation.

Therefore:

> `Timeline` is functorial in its payload, but Musa should **not** assume that it is canonically a monad.

Individual surface abstractions may define monads where their semantics earns one.

Aleatory choice, for example, may use a probability or nondeterminism monad independently of temporal composition.

---

# 22. Payload-specific algebra belongs above the kernel

The kernel does not assume a group, ring, lattice, or metric on payload values.

A particular musical domain may provide one.

Examples:

### Pitch

Intervals may form a group (G) acting on pitches:

[ G\curvearrowright Pitch. ]

Functoriality then lifts that action to whole timelines.

### Dynamics

Gain might form a multiplicative monoid.

### Pitch-class sets

May have set or quotient structures.

### Chord spaces

May carry metrics or group actions.

### Theory predicates

May define subsets or relations over kernel timelines.

None belongs to the universal temporal calculus.

This prevents Western harmony, MIDI conventions, or twelve-tone pitch arithmetic from becoming accidental foundations of
the language.

---

# 23. Tempo specifically

Tempo is not part of the kernel's musical-time algebra.

The kernel lives in exact beat time.

A tempo interpretation supplies a monotone map:

[ \tau: \mathbb Q_{\ge0} \to \mathbb R_{\ge0} ]

from beats to physical seconds.

A kernel occurrence:

[ (s,e,a) ]

is realized physically as:

[ (\tau(s),\tau(e),a). ]

Thus changing tempo changes the **interpretation of the time axis**, not the kernel composition.

A surface-language tempo declaration can therefore be retained as a typed temporal payload/context for notation and
editing while separately compiling to the physical-time map used by playback.

---

# 24. Key, meter and other contexts

The temporal kernel does not need separate special constructs for them.

A surface compiler may preserve them as temporal facts with appropriate payload types.

For example:

```text
payload KeyRegion = record {
    tonic: Text,
    mode: Text
};
```

A key prevailing from beat 0 through beat 16 is simply a typed occurrence with that support.

Likewise:

* meter regions;
* harmonic annotations;
* phrase annotations;
* lyrics;
* articulation spans;
* automation descriptions.

This does **not** claim that a key and a note have the same musical semantics.

It claims only that both can have **support in musical time**.

Their payload types and downstream interpreters remain distinct.

---

# 25. Canonical normal form

Every finite kernel composition normalizes to exactly the general shape:

```text
timeline <duration> {
    occurrence <payload> from <start> to <end>;
    ...
}
```

with:

* no `sequence`;
* no `overlay`;
* no composition references.

Occurrence order in the serialized normal form is canonical, for example by:

1. start position;
2. end position;
3. canonical payload encoding;
4. multiplicity.

Then kernel equality is:

[ M\equiv N ]

iff their normalized timeline literals are structurally equal.

This makes semantic equality decidable for the initial payload schema.

---

# 26. Why this is valuable for interoperability

Every exporter receives the same object:

[ (d,E). ]

It does not need to understand:

* motifs;
* loops;
* transposition;
* nested sequence trees;
* chord syntax;
* key-block syntax;
* functions;
* recursion;
* probabilistic choice.

Those have already been interpreted.

Thus:

```text
Musa surface
     ↓
kernel timeline
  ↙   ↓    ↘
MEI MIDI MusicXML
     ↓
performance
     ↓
audio
```

A backend has a very small semantic obligation:

> faithfully interpret typed facts located in exact musical time.

---

# 27. Why preserve structural source above the kernel

Normalization intentionally forgets:

```text
this was a motif
this was repeated four times
this was produced by transposition
these notes were originally written as a chord literal
```

That information matters to the editor.

Therefore the compiler should retain:

```text
surface AST / compositional HIR
          │
          ├── provenance/source map ─────────┐
          │                                  │
          ▼                                  │
kernel timeline                              │
          │                                  │
          ▼                                  │
rendered artifact ◄──────── stable IDs ──────┘
```

The **kernel is deliberately a semantic quotient of the source program**.

That is desirable for interoperability.

It would be undesirable as the sole editable representation.

---

# 28. Algebraic summary

For payload (A), the proposed kernel has:

### Time

[ (\mathbb Q,+,0) ]

an abelian group of global musical positions.

### Duration

[ (\mathbb Q_{\ge0},+,0) ]

an ordered commutative monoid.

### Fixed-duration timelines

[ (\mathcal T_A(d),\oplus,0_d) ]

a free commutative monoid of occurrences.

### Succession

[ ;: \mathcal T_A(d)\times\mathcal T_A(e) \to \mathcal T_A(d+e) ]

associative and unital.

### Ambient extension

[
d\le e
\implies
\operatorname{extend}_{d,e}
:
\mathcal T_A(d)\to\mathcal T_A(e). ]

### Payload functoriality

[ f:A\to B \implies \mathcal T(f): \mathcal T_A\to\mathcal T_B. ]

### Time-scaling action

[ \mathbb Q_{>0}\curvearrowright\mathcal T_A. ]

### Local observation

[ \mathcal O_A: \mathcal I^{op}\to\mathbf{CMon} ]

a presheaf of occurrence observations on bounded time intervals.

### Conditional synchronized interchange

```text
[
(M\oplus N);(P\oplus Q)
=======================

(M;P)\oplus(N;Q) ]
```

when the first pair and second pair respectively have equal durations.

### Explicit non-structures

The kernel is **not assumed to be**:

* a ring;
* a semiring;
* a full 2-monoid;
* a duoidal category;
* a monad in its payload argument.

Those stronger structures impose laws that do not correspond uniquely to actual musical composition.

---

# 29. The key hypothesis

The design rests on one falsifiable hypothesis:

> **Finite symbolic musical meaning can be represented faithfully as typed occurrences supported in an ambient exact
> musical-time region, with succession and superposition providing the only universally necessary composition
> operations.**

The surface language may contain far richer constructs.

They earn kernel status only if we find a real musical construction that cannot faithfully elaborate to this calculus
without losing information required by multiple independent consumers.

That is the appropriate test for whether the kernel is missing a primitive.
