# Notes are values, not types

## Purpose

This note decides whether pitch transformation forces dependent types into Musa's source language.

It does not.

## The ordinary model

Within staff syntax, the notation adapter expands a written literal to a value:

~~~text
c#4
  -> SpelledPitch(NoteName(C, Sharp), 4)
~~~

The expression has one ordinary type:

~~~text
c#4 : SpelledPitch
~~~

A transformation consumes and returns values:

~~~musa
fn up_octave(pitch):
  transpose_spelled(pitch, P8)

fn answer(subject):
  map_pitches(subject, fn(pitch):
    transpose_spelled(pitch, P5)
  )
~~~

Inference gives up_octave the type:

~~~text
SpelledPitch -> Result(SpelledPitch, PitchError)
~~~

It gives answer a type based on the declared map_pitches operation. No annotation must repeat the particular pitch
value.

The Result is useful because not every requested spelling, register, or target notation need be representable. An opaque
validated type can remove the error when a package can establish stronger bounds once and hide its constructor.

## What a dependent version would say

A value-indexed pitch design could place the exact spelling and register in the type:

~~~text
c#4 : SpelledPitch(C, Sharp, 4)
~~~

Transposition would then compute a result type:

~~~text
transpose:
  SpelledPitch(letter, accidental, register)
  -> Interval(quality, number)
  -> SpelledPitch(new_letter, new_accidental, new_register)
~~~

This is possible mathematics. It is the wrong default interface.

The term already contains the exact pitch. Repeating it in the type makes a phrase's type grow with its notes. A small
edit changes a large type. Generic music transformations acquire arithmetic constraints. Error messages report type
normalization when the musician needs a spelling or range error. Principal Hindley–Milner inference no longer applies.

Musa evaluates these pure transformations while building the project. A failed transformation is therefore reported
before notation or sound preparation even when the failure is represented by Result rather than by an uninhabited type.

## Macros do not change the answer

A syntax adapter performs this path:

~~~text
written syntax
  -> expanded constructor expression
  -> inferred type
  -> evaluated value
~~~

The adapter generates syntax, not a value-dependent type. The ordinary type checker assigns SpelledPitch to the expanded
expression.

Syntax categories also need no dependent type system. Musa can use separate nominal types:

~~~text
ExprSyntax
BlockSyntax
DefinitionSyntax
~~~

or an ordinary parameter:

~~~text
Syntax(ExprKind)
Syntax(BlockKind)
~~~

These parameters range over types, not source values.

## Musical invariants without dependent types

The five pressure tests need several strong invariants. Each has a plain representation.

| Invariant | Plain interface |
| --- | --- |
| a row contains each pitch class once | opaque Row12 plus a checked constructor |
| a phrase is nonempty | opaque Phrase plus make returning Result |
| a voicing realizes a chord | opaque Voicing plus a checked voicing function |
| a pitch belongs to a scale | a named membership result or an opaque validated member |
| a studio graph is schedulable | prepare returns Result(PreparedExecution, GraphError) |
| a bar has the promised duration | notation validation with an exact source error |

These interfaces put a fact in the type when callers need a lasting distinction, such as Row12 versus List(Pc12). They
keep one-off evidence in a Result or a named witness value.

## When to reconsider

Reconsider a restricted indexed type only if two independent cases require all three conditions:

1. callers must know a relation before evaluating the value;
2. an opaque validated type or Result loses information needed for later checking; and
3. the index language has a small, decidable equality with readable inferred types.

Possible examples would be a transform that must preserve a statically known voice count across separately compiled
packages, or a hardware binding whose channel count must be known before preparation. Neither case has yet survived the
simpler opaque-type design.

If such a case appears, add the smallest index needed for that relation. Do not turn every musical value into its own
type.

## Decision

Keep rank-1 Hindley–Milner inference, ordinary nominal data, opaque validated types, and Result.

Treat c#4 as a SpelledPitch value. Let functions transform that value. Let the notation adapter report errors that
depend on the chosen target or context.

Dependent types remain out of the initial source language.
