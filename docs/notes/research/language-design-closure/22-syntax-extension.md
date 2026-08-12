# One expansion system for musical syntax

## Purpose

This note corrects one claim in [21-surface-syntax.md](21-surface-syntax.md).

That note said Musa should defer public macros. The note treated staff notation as a fixed compiler convenience. It did
not ask who should own the notation or how another package could offer an equally concise form.

The written pitch c#4 exposes the gap.

## The concrete case

Consider:

~~~musa
let phrase = staff:
  c#4/4
  d4/4
~~~

Before type inference, the staff notation adapter can expand this to the ordinary meaning:

~~~musa
let phrase =
  sequence(
    note(SpelledPitch(NoteName(C, Sharp), 4), Duration(1/4)),
    note(SpelledPitch(NoteName(D, Natural), 4), Duration(1/4)),
  )
~~~

Nothing in the second program requires a special evaluator. It uses constructors and complete calls. The special part is
reading the first program.

That is macro work.

## Revised recommendation

Put one hygienic expansion system in the initial language design. Use it for Musa's own derived forms and expose a
bounded version to packages.

Do not make ordinary source values into syntax values. Do not add runtime eval. Do not let an import silently replace
the grammar of every file.

The public unit of extension should be a named syntax adapter:

~~~musa
staff:
  c#4/4
  d4/4
~~~

The leading name says which package owns the notation. The indented block gives that adapter a bounded region to read.
The adapter returns ordinary Musa syntax, which is then resolved, inferred, and evaluated like any other program.

A piece may choose one adapter as its local score default, so common staff notation need not repeat staff on every
voice. The choice remains explicit in the piece or package rather than universal in the core language.

Under this model, c#4 is an expression in staff syntax, not a universal expression in every Musa module. Its expansion
is an ordinary constructor expression. Outside a staff block or a piece that selected staff as its score adapter, code
uses the explicit constructor or a named staff conversion.

## Why this is not full homoiconicity

A homoiconic language represents programs with the same ordinary data structures that programs manipulate. Musa does not
need that promise.

Musa needs syntax objects during expansion. A syntax object contains:

- grouped source tokens;
- indentation and delimiter structure;
- lexical scopes;
- source locations; and
- the history of prior expansions.

Ordinary musical functions consume values such as Chord, Phrase, Music, or StudioDescription. Macro transformers consume
Syntax and produce Syntax. The two kinds of function run at different phases even though both finish before audio
starts.

Racket makes the same important distinction. Its syntax objects carry scope and source information in addition to a
datum. Rhombus shows that this design does not require S-expressions: it places a macro system over an
indentation-sensitive intermediate notation. See the official
[Racket syntax model](https://docs.racket-lang.org/reference/syntax-model.html),
[Rhombus notation guide](https://docs.racket-lang.org/rhombus-guide/Notation.html), and
[Rhombus syntax model](https://docs.racket-lang.org/rhombus-model/syntax-model.html).

## The two phases

The source pipeline gains one clear boundary:

~~~text
source text
  -> lossless grouped syntax
  -> resolve syntax bindings and expand hygienically
  -> resolved program
  -> inferred typed body
  -> total evaluation core
  -> typed values
~~~

Syntax-binding resolution and expansion are interleaved. Ordinary value and type resolution follows the expanded
program. A transformer cannot inspect an inferred type because no inferred type exists yet. This keeps the inference
problem acyclic and preserves principal types.

Macro packages are prepared before packages that use them. A macro import is explicit. Ordinary value imports do not
change the reader.

## The transformer language

Use the same total functional language for transformer code, but at an earlier phase and with a small syntax library.
The central contract is:

~~~text
Transformer(InputKind, OutputKind)
  = Syntax(InputKind) -> Result(Syntax(OutputKind), SyntaxError)
~~~

InputKind and OutputKind distinguish expressions, definitions, patterns, and blocks. An expression transformer cannot
silently emit an import or a package declaration.

Syntax quotation builds a template. Antiquotation inserts a matched syntax object. Introduced names are hygienic: they
refer to the bindings visible where the transformer was defined unless the transformer explicitly retains a name from
its use site.

Transformer functions are ordinary total functions over finite syntax. They may use Text, Nat, products, lists, options,
results, and finite folds. They may not:

- read files or the network;
- inspect a clock or random source;
- call audio or project services;
- inspect inferred types;
- mutate compiler state; or
- invoke runtime eval.

This keeps expansion reproducible and makes package builds safe to run.

## How expansion terminates

The total source language makes each transformer call finish, but that alone does not stop one macro from expanding to
another macro forever.

Give each syntax binding an expansion rank. Its output may contain only core forms or calls to syntax bindings with a
lower rank. The compiler rejects a cycle in the syntax dependency graph.

Each transformer returns finite syntax. The multiset of remaining expansion ranks decreases after every step. Expansion
therefore ends.

The compiler also charges a deterministic expansion budget for useful denial-of-service protection. A warm query cache
replays the same logical charge as a cold expansion.

## Hygiene and source ownership

Each expansion records:

- the adapter or macro definition;
- the exact use site;
- the input syntax;
- the produced syntax; and
- the parent expansion, if one exists.

Generated identifiers carry lexical scopes, so a temporary name cannot capture a musician's name or be captured by it.
Diagnostics walk the expansion record back to the shortest useful source location.

This expansion record is not the cross-stage musical derivation record. Expansion explains why source syntax exists.
Derivation explains how one musical representation became another. A note produced by a syntax adapter starts its
musical derivation at the adapter use site and retains the adapter definition as source context.

## What should be syntax

Syntax earns a macro when the written shape carries information that an ordinary function call cannot state clearly.

| Candidate | Initial owner |
| --- | --- |
| staff note, rest, chord, bar, and mark notation | staff syntax adapter |
| compact music sequencing | music block adapter |
| score, part, and voice nesting | score adapter |
| studio nodes, controls, and wiring | studio adapter |
| record declarations and record construction | general derived form |
| if | general derived form over match |
| motif and fragment declarations | music-derived forms with retained source roles |
| finite notation repeat | music-derived form over a finite fold |

The staff and studio adapters are materially different cases. One reads compact symbolic music. The other reads a graph
of processors, controls, and connections. Together they justify the shared expansion system.

## What should remain ordinary code

Most musical ideas are values and functions:

- chord construction;
- keys, modes, and scales;
- harmonic analysis;
- voicing;
- phrase models;
- tuning systems;
- orchestration policy;
- performance interpretation; and
- finite live protocols.

A package should not introduce grammar merely to make a function call shorter. Syntax is justified when it makes a
domain document readable or introduces a binding form that functions cannot express.

## What stays in the semantic core

Expansion makes the surface smaller. It does not remove the core rules for:

- immutable binding;
- functions and complete calls;
- nominal data;
- exhaustive match;
- module privacy;
- finite folds;
- type inference;
- contextual Music closure;
- temporal-kernel construction; and
- saturated compiler operations.

The expanded program uses only these forms. The proof of source type safety and termination begins after expansion.

Expansion needs its own smaller proof:

1. every accepted expansion finishes;
2. expansion is deterministic;
3. the result contains no macro calls;
4. introduced identifiers obey hygiene;
5. every output node has a source and expansion record; and
6. an expansion error names a finite use-site path.

## What not to copy

Racket and Rhombus support broad language construction. Musa should copy their sound ideas, not their full scale.

The initial design omits:

- arbitrary reader replacement for a whole file;
- unscoped global operator declarations;
- macro access to inferred types;
- phase levels beyond ordinary source and expansion;
- macro-generated imports;
- recursive macro definitions;
- runtime syntax reflection; and
- string-to-code eval.

These omissions keep one visible expansion phase and one ordinary source language.

## Implementation shape

The lossless syntax tree remains the editor's source of truth. Expansion should use lightweight syntax objects that
refer to original tree nodes and can also hold generated nodes. There is no need to copy the whole file into a public
AST.

The expander belongs in musa-compiler, after layout grouping and before ordinary resolution. Musa-language owns the
lossless grouping rules and editor recovery. A package receives only the bounded Syntax API, never Rowan internals.

Built-in derived forms and package forms use the same expansion engine. The compiler may implement its bootstrapping
forms in Rust, but they must produce the same syntax objects and expansion records as package transformers.

## Decision

Revise the earlier answer:

- yes, note notation is macro-like expansion;
- yes, the initial design should include a hygienic expansion model;
- yes, packages should receive bounded syntax adapters;
- no, ordinary runtime values should not become source syntax;
- no, Musa needs no runtime eval or unlimited reader macros; and
- no, macro expansion should not participate in type inference.

The next paper-program pass should implement two expansions on paper: the staff phrase above and one studio graph. If
both expand cleanly to the same small source forms, the abstraction has earned its place.
