# Draft A: make everything a flow

**Status: rejected draft. This document does not set Musa's rules.**

## Purpose

This draft tries the most tempting answer: represent written passages, performance, and audio with one type.

## 1. The proposal

Let `Flow<C, A>` mean values of type `A` changing over a time coordinate `C`. A finite written part, a sequence of
gestures, and an audio signal would all be flows with different `C` and `A`.

The proposed operations are:

```text
map       : (A -> B) -> Flow<C, A> -> Flow<C, B>
after     : Flow<C, A> -> Flow<C, A> -> Flow<C, A>
together  : Flow<C, A> -> Flow<C, A> -> Flow<C, A>
connect   : Flow<C, A -> B> -> Flow<C, A> -> Flow<C, B>
feedback  : Flow<C, A -> A> -> Flow<C, A>
section   : Span<C> -> Flow<C, A> -> FiniteView<C, A>
```

Written notes would be sparse flows. Audio would be dense flows. A synthesizer would be a flow of functions from control
values to samples. `connect` would apply it through time.

## 2. Why it looks attractive

The type gives notation and audio a common temporal shape. Taking an excerpt is `section` in either case. Mapping a
transposition over notes and mapping gain over samples use one operation. The source program can name both without a
special stage boundary.

Research on causal stream functions and signal-flow diagrams supports part of this picture. Finite diagrams can denote
unbounded streams, and series and side-by-side composition have clean laws
([Di Lavore, de Felice, and Román](https://arxiv.org/abs/2212.14494), [Coya](https://arxiv.org/abs/1805.08290)).

## 3. Proposed meaning

For discrete `C`, a flow is a function from positions to values:

```text
Flow<C, A> = Position<C> -> A
```

A sparse event flow uses `Option<A>`. An audio flow uses an audio frame at every position.

`after(x,y)` shifts `y` by the length of `x`. `together(x,y)` combines values point by point. `feedback(f)` is a fixed
point of `f`.

The proposal is precise enough to test. The next note rejects it.

