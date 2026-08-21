# The active Musa language candidate

## Purpose

This note gives one answer to the language question that notes 19–25 explored:

> What is the smallest total language in which musicians can define their own concepts and connect them to Musa's
> existing notation, temporal, performance, and sound stages?

This note supersedes notes 19–25 as the active research candidate. Those notes remain beside their reviews because they
show why this answer was chosen. This note governs nothing. The current implementation and `docs/rules/` remain in force
until the proof and promotion gates at the end of this note pass.

The answer has three parts:

1. ordinary Musa code is a small, strict functional language with broad type inference;
2. packages may give named blocks their own notation through a restricted syntax adapter; and
3. each musical stage keeps its own data and connects to the next stage through a checked conversion record.

The syntax adapter is smaller than a general macro system. It changes how a bounded block is written. It does not change
the module grammar, inspect inferred types, or run arbitrary code.

## 1. Start with the page a reader sees

The standard staff adapter keeps Musa's current note spelling:

```musa
import syntax std.notation.staff as staff

let melody = staff:
  | c4/4 d4/4 e4/4 f4/4
  | g4/4. a4/8 b4/2
```

Each note states an absolute written pitch and a written duration. A reader need not find a previous octave or duration.
The first bar is checked as four quarters. The second is checked as a dotted quarter, an eighth, and a half.

An author who means an exact rational span rather than a particular staff spelling may write:

```musa
let exact = staff:
  c4(3/8)
```

These two notes occupy the same exact span:

```musa
c4/4.
c4(3/8)
```

They do not make the same presentation claim. The first asks for a dotted quarter. The second gives an exact duration
and leaves its staff spelling to a named notation policy. A target that cannot choose a faithful spelling returns an
error or records the choice as a loss.

Tuplets retain the written values and the tuplet relation:

```musa
let triplet = staff:
  tuplet 3/2:
    c4/8
    d4/8
    e4/8
```

The written notes are eighth notes. The group occupies one quarter. Replacing them with three anonymous `1/12` spans
would lose the bracket and its musical meaning.

## 2. Ordinary code

Outside named notation blocks, Musa is a small expression language. Names are immutable. Evaluation is strict: a call
evaluates its function and arguments from left to right before it enters the function body. Every accepted source
function terminates.

The common case needs no type annotation:

```musa
fn twice(f, value):
  f(f(value))

let up_octave = fn(music):
  transpose(music, P8)
```

The compiler infers:

```text
twice: (('a -> 'a), 'a) -> 'a
up_octave: Music -> Music
```

`'a` means any one type chosen consistently by a caller. A public annotation may document or narrow an inferred type,
but an ordinary local function does not repeat information the body already supplies.

### 2.1 Types

The initial language has:

- `Unit`, `Bool`, `Nat`, `Ratio`, and `Text`;
- products and functions;
- `Option<T>`, `List<T>`, and `Result<T, E>`;
- nominal records and variant types;
- abstract nominal types whose constructors are private to their module; and
- compiler bridge types such as `Music`, `MusicalContext`, and `StudioDescription`.

A nominal type has one build-local identity. Two modules that declare textually identical types still declare different
types. No identity is promised across separate builds.

The language uses rank-1 Hindley–Milner inference. A `let`-bound definition may work at several types. A function
parameter cannot require an argument that is itself polymorphic. The language has no subtyping, overloading, implicit
numeric conversion, higher-rank type, refinement type, or value-dependent type.

### 2.2 Data and finite folds

One user data type may refer to itself through products, `List`, `Option`, and `Result`. It may not occur under a
function arrow, and two nominal types may not refer to each other. In plain terms, a value may contain smaller values of
its own type, but it cannot hide recursion inside a callback or a second type.

This type is accepted:

```musa
type Form<T>:
  Leaf(T)
  Group(List<Form<T>>)
```

This type is rejected:

```musa
type Bad:
  Bad(Bad -> Nat)
```

Every accepted recursive type generates a total fold. User code consumes recursive data through that fold. It cannot
write a general recursive function. Values are finite, so each fold finishes.

This controlled recursion is needed in two independent places: musical form and grouped syntax. It does not add
unbounded source evaluation.

### 2.3 Complete calls and explicit functions

A call supplies one complete product of arguments:

```musa
transpose(music, P5)
```

Supplying fewer arguments is an error. A function value is written as a function:

```musa
let up_fifth = fn(music):
  transpose(music, P5)
```

Musa has no partial, named, or default calls. Configuration uses records:

```musa
record PrepareOptions:
  sample_rate: Nat
  block_size: Nat
  channels: ChannelLayout

let options = PrepareOptions:
  sample_rate: 48_000
  block_size: 128
  channels: Stereo
```

`=` binds a name. `:` introduces the body of a declaration, a record, a branch, or another indented structure. It is not
used inside note tokens or ratios.

### 2.4 Matches and failure

A source match is ordered and exhaustive:

```musa
fn function_of(symbol, next):
  match symbol:
    I -> Tonic
    V -> Dominant
    FlatII ->
      match next:
        Some(V) -> Predominant
        _ -> Other
    _ -> Other
```

The compiler checks that every possible constructor is covered. Failure to match one source arm means trying the next
arm; it is not a value that source code can observe.

The evaluation core shares non-uniform fallbacks with local join points:

```text
join fallback() = other
switch symbol:
  I       -> tonic
  V       -> dominant
  FlatII  -> switch next:
               Some(V) -> predominant
               _       -> jump fallback()
  _       -> jump fallback()
```

A join point is a compiler-local label with parameters. It may be entered only by a tail `jump`, and it cannot escape as
a function value. This preserves source order without copying `other` or adding a runtime pattern-failure value.

### 2.5 Modules

A source file is a module. Names are private unless marked `pub`. Imports are explicit and qualified:

```musa
import std.tonal.harmony as harmony

let dominant = harmony.numeral_chord(home, five)
```

The initial language has no structures, signatures, functors, declaration templates, or first-class modules. File
modules, public definitions, records, ordinary functions, and abstract types cover the five case studies with fewer
rules.

A public abstract type exposes its name and selected functions while hiding its constructors. The compiler rejects a
cycle among value definitions or nominal type declarations. One nominal type may refer to itself through the finite
forms in the formal specification and receives one generated fold.

The package resolver supplies one finite resolved import graph for a build. Exact Git source packages remain in scope.
Registries, version solving, stable compiled identities, and persistent compiled-value caches remain out of scope.

## 3. One fixed reader, bounded syntax adapters

A syntax adapter owns a named, delimited region:

```musa
import syntax std.notation.staff as staff

let phrase = staff:
  c#4/4
  d4/4
```

The name `staff` says which package reads the block. An ordinary import cannot change syntax. A syntax import appears in
the fixed module header, before any definition that uses it.

The adapter expands the block to ordinary Musa expression syntax. It does not evaluate the expression or select its
type. For example, the block above may expand to the ordinary meaning:

```musa
sequence(
  written_note(c_sharp(4), note_value(4, 0)),
  written_note(d_natural(4), note_value(4, 0)),
)
```

Ordinary resolution and type inference check that result.

### 3.1 Fixed compiler order

The compiler processes a module in this order:

1. Lex and group the entire file with fixed rules.
2. Parse the fixed module header.
3. Resolve the header's syntax imports against the already resolved package graph.
4. Expand named expression and block regions.
5. Parse each expansion as one ordinary expression.
6. Resolve ordinary module, type, constructor, and value names.
7. Infer and check types.
8. Lower successful typed bodies to the total evaluation core.

Package adapters may emit expressions and the contents of expression blocks. They may not emit imports, modules, type
declarations, value declarations, or new syntax adapters. Compiler-owned forms such as `record` declarations remain
compiler-owned. This rule makes the set of module declarations known before package expansion and breaks the
expansion-resolution cycle found in review 24.

An adapter package is compiled before a package that uses it. Adapter dependencies form a finite acyclic graph. The
compiler derives each adapter's rank from that graph; authors do not write rank numbers.

### 3.2 Fixed lexing and grouping

Packages do not extend the lexer. The universal lexer recognizes:

- identifiers, including digits after the first character;
- decimal natural numbers and exact rational numbers;
- quoted text with one escape rule;
- the fixed delimiters `()`, `[]`, and `{}`;
- the fixed multi-character core marks such as `->`; and
- every other non-alphanumeric code point as a symbol token.

Comments and whitespace remain in the lossless syntax tree. A syntax object can retain them without giving them semantic
meaning.

The grouper forms a node for each matched delimiter pair. A line whose last significant token is `:` opens one layout
group when the following line is indented farther. No other token changes grouping. An adapter may interpret symbol
tokens such as `#`, `b`, or `𝄪`, but it may not change how their source text was split or grouped.

This fixed boundary can represent the current staff forms. For example, `c#4/4.` is grouped as an identifier and fixed
symbol/number tokens inside one staff block; only the staff adapter assigns the musical meaning.

### 3.3 Syntax values

The transformer language receives finite syntax values:

```text
SourceInfo = Original(source range)
           | Generated(expansion id, node path)

Syntax = Missing(SourceInfo)
       | Token(SourceInfo, token kind, exact text)
       | Identifier(SourceInfo, name, scopes)
       | Group(SourceInfo, delimiter, List<Syntax>)
```

`delimiter` is `Parentheses`, `Brackets`, `Braces`, or `Layout`. `scopes` is an opaque finite list maintained by the
compiler. Package code may compare names and preserve scopes, but it cannot construct an arbitrary scope id.

`BlockSyntax` and `ExprSyntax` are abstract checked wrappers around `Syntax`. The compiler creates `BlockSyntax` from a
named layout block. An adapter builds output through a small, separate transformer API. Ordinary Musa expressions have
no quotation or syntax-reflection form.

The compiler supplies `fold_syntax`, generated by the same finite-fold rule as user data. An adapter can inspect every
token and nested group without general recursion.

### 3.4 Adapter interface

The expansion capability is:

```text
expand_A: ExpansionContext × BlockSyntax -> Result<ExprSyntax, SyntaxError_A>
```

`ExpansionContext` is an opaque, explicit value. Its identity is the structural path to this adapter call: the build
node, source file, adapter-region ordinal, and parent expansion path. The compiler derives it by a fixed traversal. It
does not read a counter or choose a random id.

An adapter is a total module compiled for the expansion phase. It may use finite data, text, products, options, results,
folds, and the transformer operations below. It may preserve input syntax, make definition-scoped names, and make a
local name at an explicit finite path. Calling one local binding path twice denotes the same name; two binders use
different binding paths. Builder node paths are derived from input-tree paths plus a small role number, so an adapter
can handle input of any finite size. `checked_expression` rejects duplicate node paths and two binder declarations at
one binding path.

The transformer operations construct tokens, identifiers, groups, checked expressions, and anchors. Each is a pure total
function of its displayed arguments. A node made at output path `p` receives `Generated(ctx.expansion_id, p)`.
Definition names are resolved when the adapter is checked. Existing input nodes keep their use-site names and source
information. This is a small phase language, not a macro system hidden inside ordinary source evaluation.

It may not:

- inspect an expected or inferred type;
- read an ordinary value from the importing module;
- read files, the network, a clock, randomness, or project services;
- mutate compiler state;
- call audio services;
- evaluate text as source; or
- emit a call to an adapter at the same or a higher rank.

Each successful expansion produces:

```text
ExpansionRecord = {
  adapter definition,
  exact adapter package source version,
  use-site source range,
  input syntax,
  output syntax,
  parent expansion, if any,
}
```

Every generated node points to one entry in this record. This is a compiler source map, not a musical derivation edge.
When expanded syntax later creates music, that music begins its derivation at the adapter use site and retains the
adapter definition as source context.

### 3.5 Termination, determinism, and charges

Adapter definitions contain no adapter regions. This gives the transformer language a small bootstrap. Each transformer
is a total function over finite input. Every adapter call in its output has a lower graph-derived rank. Therefore the
multiset of unexpanded ranks decreases after each expansion, and expansion terminates.

The input syntax, adapter definitions, import graph, and compiler options determine one expansion result. An adapter has
no hidden input, so expansion is deterministic.

Compiler limits use phase-tagged logical charges:

```text
CompilerLimits = {
  expansion_steps,
  generated_syntax_nodes,
  type_constraints,
  evaluation_steps,
}
```

Each phase charges its own fixed events. A session cache may reduce wall-clock work, but a hit replays the same logical
charge as the miss it replaces. Cache warmth cannot change whether a source file is accepted.

## 4. Source-preserving edits

Source remains the only editable record. An adapter never owns a second mutable score or studio graph.

An adapter may declare an edit command type and two total functions:

```text
edit_A:
  BlockSyntax
  × Edit_A
  -> Result<List<TextEdit>, EditError_A>

apply_A:
  Value_A
  × Edit_A
  -> Result<Value_A, EditError_A>
```

`Value_A` is the ordinary value produced after the adapter's expansion is checked and evaluated. The adapter descriptor
names that ordinary result type; expansion itself does not inspect the type.

The edit law is:

> If `edit_A(s, c)` returns patch `p`, then applying `p` changes only ranges inside `s`. Expanding, checking, and
> evaluating the patched block gives the same adapter value as `apply_A` gives from the original value, under the
> adapter's stated semantic equality.

Bytes outside the returned edit ranges remain unchanged. Comments, layout, names, and helper definitions outside those
ranges are not regenerated. The new source version receives new anchors, with retained origin links for untouched and
moved material.

An optional printer creates a new block when no source block exists:

```text
print_A: Value_A -> Result<BlockSyntax, PrintLoss_A>
```

For values it accepts, expanding and evaluating the printed block must return an adapter-equal value. This law does not
claim equal source, comments, layout, or origin. A printer alone does not make an existing block safely editable.

Adapters have three declared conformance levels:

1. **Readable:** expansion only; structured views are read-only.
2. **Editable:** expansion plus edit commands and the edit law.
3. **Generative:** editable plus a printer for new blocks.

The standard staff and studio adapters must be generative. A third-party adapter may choose a lower level.

## 5. Staff duration and exact time

The staff adapter owns written pitch and written rhythm. Neither is a universal language literal.

Its duration data distinguishes:

```text
WrittenDuration = NoteValue(denominator, dot count)
                | Exact(Ratio)
                | Compound(List<WrittenDuration>)
```

`NoteValue` records conventional staff spelling. `Exact` records a rational duration without selecting a staff spelling.
`Compound` records the written pieces joined by a tie. Tuplet relation, grace status, and spanners remain explicit staff
structures rather than being inferred from the reduced ratio.

The common spellings are:

```text
c4/4       NoteValue(4, 0), exact span 1/4
c4/4.      NoteValue(4, 1), exact span 3/8
c4/4..     NoteValue(4, 2), exact span 7/16
c4(3/8)    Exact(3/8), exact span 3/8
```

The current spaced exact form, `c4 3/8`, receives a mechanical fix to `c4(3/8)` in the new grammar.

No note inherits a pitch register or duration from an earlier note. A block may declare context such as meter, key,
clef, tuning, or notation policy in its own header. Such context may affect validation, engraving, analysis, or
performance; it does not silently replace an event's written pitch or duration.

The general locality rule is:

> Given an adapter version, a block header, and a block body, expansion returns one expression or one error. It does not
> depend on an earlier block, an inferred type, or mutable compiler state.

This is not a claim that all music has bars or absolute pitch. A staff block may use those ideas. Another adapter may
use scale degrees, tuning ratios, instrument strokes, spoken cues, proportional positions, or another theory.

## 6. Four compiler forms and several targets

Musa uses four front-end forms. Each answers one question.

### 6.1 Lossless grouped syntax

This is the source text, including errors, comments, spaces, and exact byte ranges. `musa-syntax` owns it. Formatting,
syntax highlighting, and text editing use it.

Expansion is an operation over syntax objects and records. It is not a fifth stored program form.

### 6.2 Resolved program

This form contains the expanded source expressions and one build-local id for every module, type, constructor, value,
and use. It retains source and expansion anchors. It contains no unresolved name and need not be well typed.

### 6.3 Typed body

This source-shaped form records the inferred monotype of each expression, the generalized scheme of each eligible `let`,
constructor access, match coverage, positivity results, and module privacy checks. Every non-recursive `let` is eligible
because the language is pure.

### 6.4 Evaluation core

This form makes closures, captures, evaluation order, constructor decisions, joins, folds, and saturated compiler
operations explicit. Pattern syntax and polymorphic schemes are gone. Type information needed for validation and the
proof remains; erased type variables do not become runtime data.

The common front end stops after this core evaluates to typed values. Musa needs no control-flow-graph MIR, universal
musical intermediate form, or LLVM-like target.

## 7. Music and the stage boundary

`Music` is an abstract, finite recipe. Theory- and adapter-specific choices are supplied before a package constructs it.
Closing it needs only ambient placement, score scope, and a source root:

```text
close_music:
  Music
  × MusicalContext
  -> Result<Term<ScoreFact>, MusicError>
```

The context is an explicit argument at the closing boundary. It is not mutable global state and does not act as an
open-ended package dictionary. A successful result is a finite, closed, well-typed temporal term. A failure names an
invalid placement, scope, anchor, or payload.

The complete path is:

```text
source
  -> lossless grouped syntax
  -> expanded syntax plus source map
  -> resolved program
  -> typed body
  -> evaluation core
  -> typed values
  -> closed temporal term or another stage-specific finite value
  -> timeline, notation, analysis, gesture, or prepared process graph
  -> deterministic process steps
  -> audio history
```

Notation, analysis, gesture, and sound do not share one final representation. Adjacent stage records compose only when
the first ends at the exact representation and anchor where the second begins. Loss lists concatenate in stage order.

Source expressions and temporal terms normalize to finite values. A prepared process graph advances by deterministic
steps. An unbounded audio history does not normalize to one finite value.

## 8. Features deliberately omitted

The candidate omits:

- partial, named, and default calls;
- mutation, exceptions, general recursion, and unrestricted effects;
- subtyping, implicit conversions, overloading, type classes, and higher-rank types;
- refinement and dependent types;
- call-by-push-value syntax and type distinctions;
- signatures, structures, functors, declaration templates, and first-class modules;
- a general elaboration monad or macros that inspect types;
- module-, import-, or type-generating package macros;
- reader replacement, runtime syntax reflection, and text-to-code evaluation;
- registries, version solving, stable compiled identities, and persistent compiled-value caches; and
- a universal object that identifies source, notation, analysis, performance, and sound.

Reopen one only when two materially different musical cases need it, or when an existing safety or stage theorem cannot
be stated without it.

## 9. Tests required before formalization

Two unprivileged adapters must pass first.

The staff adapter must cover notes, rests, chords, dots, exact durations, ties across bars, slurs, tuplets, grace notes,
pickups, repeats, alternate endings, meter changes, transposing instruments, diagnostics, and one structured edit.

The studio adapter must cover processors, named ports, connections, parameters, instrument bindings, graph inputs and
outputs, diagnostics, and one structured edit.

Both must use the public syntax data, fold, expansion record, and edit law. Neither may call a private parser or receive
an inferred type.

After those trials, the five musical cases must be rewritten with no ellipses. They must show inferred types, expansion,
evaluation, stage transitions, losses, added choices, and both notation-led and performance-led routes.

The candidate may proceed to proof only if:

- the two adapters require no compiler privilege;
- each block is determined by its own header, body, adapter version, and compiler options;
- type inference remains independent of expansion;
- diagnostics point to original source;
- written rhythm and exact time remain distinct;
- edits preserve unrelated source bytes;
- the five programs need no rejected language feature; and
- every new core feature is used by two materially different cases or required by a proof boundary.

## 10. Proof and promotion gates

The proof must establish expansion termination, determinism, hygiene, and source attribution; decidable resolution and
principal inference; preservation, progress, deterministic evaluation, and source termination; constructor privacy;
sound adapter expansion; finite `Music` closure; and exact composition of stage records.

The proof receives one hostile review, one repair if needed, and one final review. Promotion requires a final verdict of
correct under the stated contracts with no fatal, high, or medium finding.

If the gate passes, this decision replaces the current candidate language specification and repairs the roadmap, code
map, and pending prompts. If the gate fails, this note and its proof remain research and the blocker is published beside
them.

No compiler implementation begins during this research closure.
