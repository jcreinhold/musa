# A surface syntax for Musa

## Purpose

This note asks one question:

> What is the smallest written language that makes Musa programs easy to read?

It compares three surface styles against the same musical work. It selects one style for the next paper programs. It
does not change the parser or the governing language documents.

**Later update:** [22-syntax-extension.md](22-syntax-extension.md) corrects this note's rejection of public macros. The
indentation-based surface still stands. The revised candidate adds one bounded, hygienic expansion system so packages
can own named notation and studio blocks. It also scopes c#4 to staff syntax instead of making that spelling universal
in every Musa module.

## Recommendation

Use a small, indentation-based functional language with a few typed musical blocks.

The language should look closer to Python than Rust, but it should not have Python's mutable objects, exceptions, or
dynamic types. Its meaning remains the small pure language chosen by the type-system work:

- names are immutable;
- local types are inferred;
- calls supply all arguments;
- functions are ordinary values;
- matches cover every case;
- user functions always finish;
- failures are values such as Result; and
- source evaluation happens while Musa builds the project.

The musical blocks are not a second general programming language. They are typed literals for a particular stage: music,
score, performance, or studio. Ordinary functions may construct and combine the values that those blocks produce.

Do not add public macros or make source syntax a first-class value in the initial language. Fixed surface conveniences
may be rewritten by the compiler with exact source maps.

## Start with a score

The simplest Musa program should read like this:

~~~musa
piece "Twinkle":
  composer "traditional"
  arranger "musa"

  tempo 1/4 = 104
  meter 4/4
  key c major

  score:
    part piano:
      voice melody:
        | c4/4 c4/4 g4/4 g4/4
        | a4/4 a4/4 g4/2
        | f4/4 f4/4 e4/4 e4/4
        | d4/4 d4/4 c4/2
~~~

The page is mostly musical material. Indentation shows ownership. Bar lines show musical grouping. No punctuation
pretends that a piece is an imperative program.

This example fixes the first principle:

> Use general syntax for computation and musical syntax for musical documents.

Trying to express the score as nested constructors would make the language smaller on paper and worse in use.

## A written pitch is an expression

The token c#4 is an ordinary surface expression with one type:

~~~text
c#4 : SpelledPitch
~~~

It means the written pitch C-sharp in register 4. It does not mean a frequency, MIDI note number, pitch class, scale
degree, or ensemble tuning target. The notation adapter owns SpelledPitch; the general language does not claim that all
music uses this pitch system.

The compiler rewrites the literal to ordinary data:

~~~text
c#4
  -> SpelledPitch(NoteName(C, Sharp), 4)
~~~

Inside a music block, an event combines a pitch expression with notation duration:

~~~text
c#4/4
  -> note(SpelledPitch(NoteName(C, Sharp), 4), Duration(1/4))
~~~

The compact event has type Music. Every entry in a music or voice block must have that type, and the block sequences its
entries. A saved phrase or transformation therefore appears directly:

~~~musa
fn one_note(root):
  music:
    root/4

let phrase = music:
  c4/4
  d4/4

let answer = music:
  phrase
  transpose(P5, phrase)
~~~

Musa needs no use statement to splice Music into Music. The type already says what the entry contributes.

After these rewrites, inference and evaluation see ordinary constructors, complete notation-adapter calls, and explicit
sequence. The temporal kernel later receives an opaque ScoreFact payload. It does not gain a special pitch type.

This literal is a deliberate convenience for the first notation adapter, not an extension hook. Another pitch system
uses its own package types and constructors. A future typed block may give that system concise notation without making
c#4 ambiguous.

## Then test a function

A music transformation should look like an ordinary inferred function:

~~~musa
fn canon(subject, answer, gap):
  overlay(subject, shift(gap, answer(subject)))

let answer = fn(music):
  transpose(P8, music)

let duet = canon(theme, answer, 1/2)
~~~

The compiler infers the types. A writer may add an annotation when it helps:

~~~musa
fn canon(subject: Music, answer: Music -> Music, gap: Duration) -> Music:
  overlay(subject, shift(gap, answer(subject)))
~~~

The annotated and unannotated functions mean the same thing. An annotation checks the inferred type; it does not select
an overload or change evaluation.

The anonymous function is explicit. Musa does not turn an incomplete call into a function:

~~~musa
// Clear: this is a function.
let answer = fn(music):
  transpose(P8, music)

// Rejected: transpose needs two arguments.
let answer = transpose(P8)
~~~

## Theory belongs in ordinary data and functions

The tonal and phrase-led cases need distinct types, hidden constructors, matches, and Result. They do not need special
grammar for chords, keys, modes, phrases, or harmonic function.

~~~musa
type HarmonicFunction:
  Tonic
  Predominant
  Dominant
  Other

type FunctionClaim:
  Claim(
    degree: ScaleDegree,
    function: HarmonicFunction,
    evidence: Text,
  )

fn classify(symbol, next):
  match symbol:
    CMajor -> Tonic
    CMinor -> Tonic
    DFlatMajor ->
      match next:
        Some(GSeven) -> Predominant
        _ -> Other
    GSeven -> Dominant
    _ -> Other
~~~

Constructors start with capitals. Values and functions start with lower-case names. A match arm uses an arrow because it
maps one pattern to one result. Commas remain inside ordinary argument and field lists; lines end declarations and match
arms.

The analysis remains a package function. The type checker does not infer that D-flat major has predominant function. It
infers only that every branch of classify returns HarmonicFunction.

## Modules should be simpler than the old candidate

A source file is a module. Names are private unless marked public. A public opaque type exports its name and hides its
constructors.

~~~musa
pub opaque type Phrase:
  Phrase(tokens: List(PhraseToken))

pub fn make(tokens):
  match tokens:
    [] -> Err(EmptyPhrase)
    _ -> Ok(Phrase(tokens))

pub fn perform(phrase):
  match phrase:
    Phrase(tokens) -> fold(tokens, [], add_gesture)
~~~

Clients can pass Phrase values and call public functions. They cannot name Phrase's constructor.

This covers the use cases that the earlier design assigned to signatures and structures. The initial language should
therefore omit signatures, structures, functors, templates, and make declarations. Add one only if a complete musical
package cannot be expressed with file modules, public names, opaque types, records, and ordinary functions.

Imports stay explicit and qualified:

~~~musa
import std.tonal.harmony as harmony

let dominant = harmony.numeral_chord(home, five)
~~~

Qualification tells the reader where a theory comes from. A short alias removes repetition without flattening every
package into one global namespace.

## Records replace named and default calls

Configuration should be data, not a special calling rule.

~~~musa
record PrepareOptions:
  sample_rate: Nat
  block_size: Nat
  channels: ChannelLayout

let options = PrepareOptions:
  sample_rate = 48_000
  block_size = 128
  channels = Stereo

let prepared = prepare(gestures, bindings, seed, options)
~~~

Record fields are labelled because order is not their meaning. Function arguments remain positional and complete. This
keeps function application simple while making long-lived configuration readable.

A record is surface sugar for one nominal product type with named projections. It does not add structural record
subtyping to the type system.

## Stage blocks say what kind of document follows

Musa needs several readable document forms because notation, performance, and sound preserve different information.

~~~musa
let phrase = music:
  c4/4
  d4/4
  e4/2

piece "One phrase, two views":
  score:
    part violin:
      voice line:
        phrase

  performance:
    realize line with lyrical

  studio:
    bind violin to solo_strings
    output solo_strings to main
~~~

Each heading names a stage. The compiler can reject a score command in a studio block without inventing a universal
music object. A performance-led package may construct gestures directly and provide notation only through an explicit
lossy adapter.

The exact grammar of performance and studio blocks remains owned by their stage specifications. This note fixes their
common visual rule: a named block ends when its indentation ends.

## Three candidates

The same examples were considered in three forms.

| Candidate | What it gets right | What it makes worse |
| --- | --- | --- |
| Rust-like braces and semicolons | Familiar delimiters; simple recovery | Visual noise; suggests statements, mutation, methods, and ownership that Musa does not have |
| S-expressions | Tiny parser; syntax is easy to transform | Parentheses dominate scores; ordinary musicians must read prefix notation; good macros still need scopes and source locations |
| Indentation-based functional Musa | Music and declarations dominate the page; few marks; familiar block shape | Requires indentation tokens and careful continuation rules in the parser and editor |

The indentation-based form wins for the next draft. Its implementation costs are local and testable. The costs of the
other two forms are paid by every reader.

## Why this is not merely Python

The syntax borrows Python's colon and indentation because they state region ownership with little noise. The semantics
are different:

- the last expression of a function is its result;
- let introduces an immutable name;
- match is exhaustive;
- algebraic data constructors are nominal;
- types are inferred statically;
- there are no classes, inheritance, mutation, exceptions, decorators, or runtime reflection; and
- a source function cannot loop.

The closest existing comparison is a layout-based version of Gleam: a small expression language with inferred types,
algebraic data, exhaustive matching, Result, and opaque module types. Gleam itself uses braces. Musa replaces those
braces with layout because its deeply nested score blocks benefit more from the saved punctuation.

## Why build-time evaluation does not require macros

Ordinary Musa functions already run while the project is built. They can:

- construct chords and voicings;
- map a phrase into gestures;
- generate repeated or transformed music;
- validate a theory value and return Result;
- construct a studio description; and
- choose among finite representations.

These functions operate on checked values. Macros would operate earlier, on source that has not yet been resolved or
typed. That difference does not vanish merely because both happen before audio runs.

A public macro system would need:

- syntax values carrying lexical scope and source locations;
- a rule separating macro imports from ordinary imports;
- expansion termination and resource limits;
- useful errors that point through generated code;
- formatter, rename, hover, and structured-editor behavior; and
- provenance for musical material introduced by expansion.

Racket provides this machinery through syntax objects, lexical scopes, and separate expansion phases. Its macros are not
powerful merely because programs use lists. See Racket's
[syntax model](https://docs.racket-lang.org/reference/syntax-model.html) and
[macro guide](https://docs.racket-lang.org/guide/macros.html).

Musa does not need that second language to implement if, motif, fragment, repeat, or a typed music block. The compiler
can rewrite each fixed surface form into the small core and retain a map to the written source.

## What fixed rewrites may do

Compiler-owned rewrites may remove conveniences before type inference:

| Written form | Smaller meaning |
| --- | --- |
| if condition | match the condition with true and false |
| motif declaration | a named function returning Music, plus a retained Motif source role |
| fragment declaration | a named Music value, plus a retained Fragment source role |
| record declaration | a one-constructor nominal product with field projections |
| repeated notation | a finite fold that combines Music |
| multi-argument function | one function over one product of arguments |

These are not user extensions. Their grammar, type rule, source map, and error behavior ship with the compiler.

Most musical concepts should not appear in this table. Chord construction, modal theory, phrase models, tuning systems,
analysis, orchestration policy, and studio builders remain package functions and data.

## What must stay primitive

Macros or rewrites cannot remove the semantic center. The language still needs direct rules for:

- immutable binding;
- functions and complete application;
- nominal data and exhaustive matching;
- module privacy and opaque types;
- finite folds;
- inferred types;
- typed stage blocks;
- contextual Music closure;
- temporal-kernel construction; and
- saturated compiler operations.

Moving one of these rules into a macro would hide it, not eliminate it.

## Open syntax questions

The next paper programs must settle four small points:

1. **Line continuation.** A parenthesized expression may cross lines. Decide whether any unparenthesized continuation is
   worth supporting.
2. **Result chaining.** Test whether plain match plus library functions is readable before adding a try form.
3. **Record construction.** Test the colon form above against a constructor form before fixing the grammar.
4. **One-line bodies.** Decide whether a short form such as fn twice(x): add(x, x) helps enough to permit two layouts.

These are surface choices. None changes the inferred core or stage semantics.

## Acceptance test

Rewrite five complete examples in this syntax:

1. a short score;
2. the tonal construction package;
3. the phrase-led package and its transcription loss;
4. the ensemble-tuning package and acoustic target; and
5. the live finite protocol.

For each example, count:

- written type annotations;
- punctuation-only tokens;
- nested match depth;
- compiler-only surface forms; and
- concepts that require special grammar.

The syntax passes only if the score remains score-like and the three packages remain ordinary typed code. If either side
needs an escape language, this split is wrong.

## Decision for the next draft

Stop aiming for Rust-like syntax. Test the indentation-based form above.

Keep the semantic language smaller than the surface:

~~~text
layout surface
  -> fixed source-mapped rewrites
  -> resolved program
  -> inferred typed body
  -> total evaluation core
  -> typed values
  -> explicit musical stage adapters
~~~

Defer public macros and full homoiconicity. Revisit macros only when two unrelated packages need syntax that ordinary
functions, data, modules, records, and the fixed musical blocks cannot express clearly.
