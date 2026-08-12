# Terms and notation used in this specification

This chapter gives the minimum background needed to read the later rules.

## What this specification covers

A Musa project can have several **representations**. A representation is a form built for a particular job. Source text
is one representation. A finite score timeline, an engraving plan, an analysis result, a prepared audio graph, and a
recorded audio stream are others.

The chapters that follow answer four questions:

1. What data exists at each stage?
2. When is that data valid?
3. How does one stage produce the next?
4. What does equality mean at each stage?

They do not define pitch, metre, harmony, or form for all music. Music-theory packages define those ideas for the music
they serve.

## Three kinds of rule

The specification uses three kinds of rule:

- A **well-formedness rule** says whether a value is valid. For example, an event must end within its timeline.
- An **evaluation rule** says how a valid term or audio graph runs.
- A **meaning rule** gives the mathematical value represented by a valid term.

An optimization must preserve these rules. Faster code does not get a different meaning.

## Notation

- `A` and `B` stand for ordinary value types.
- `M` and `N` stand for finite timelines.
- `G` stands for an audio process graph.
- `P` and `Q` stand for kinds of representation, such as source, notation, or MIDI.
- `x ≡_P y` means that `x` and `y` are equal under the stated equality rule for representation `P`.
- `⟦x⟧` means “the mathematical value represented by `x`.”
- `Result X E` means that an operation returns either a value of type `X` or an error of type `E`.

The symbol `≡_P` only compares values from the same representation. A written pitch and a frequency may be related by a
tuning function, but they are not equal merely because they can both be stored as numbers.

## How to read a rule

A line above a horizontal bar lists the facts that must already hold. The line below the bar gives the conclusion. For
example:

```text
M is a valid timeline    N is a valid timeline
────────────────────────────────────────────────
overlay(M,N) is a valid timeline
```

The prose around each rule defines every symbol and explains why the rule exists. The proof-status chapter separates
proved claims from intended behavior that still needs implementation evidence.
