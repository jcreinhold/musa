# Can one adapter interface serve both notation and sound?

## Purpose

This note tests the adapter interface from [the active language decision](26-language-design-decision.md) against two
different jobs:

1. reading staff notation; and
2. reading a studio graph.

The test is concrete. Each adapter receives the same public `BlockSyntax`, expands to an ordinary Musa expression, and
offers source-preserving edits. Neither adapter receives an inferred type, a private parser, or mutable compiler state.

The result is positive. One interface serves both cases. The trial also finds one useful correction: an adapter block
should expand to its own data value, not directly to `Music` or a prepared audio graph. Ordinary package functions make
those later conversions. This keeps expansion independent of type checking and keeps errors at the right stage.

The code below is **paper Musa**. It states the candidate language precisely, but the current compiler does not yet
accept this syntax.

## 1. What both adapters receive

The compiler supplies the same finite syntax tree to every adapter:

```text
SourceInfo = Original(source range)
           | Generated(expansion id, child number)

Syntax = Missing(SourceInfo)
       | Token(SourceInfo, token kind, exact text)
       | Identifier(SourceInfo, name, scopes)
       | Group(SourceInfo, delimiter, List<Syntax>)
```

The public library provides these total operations:

```text
fold_syntax:
  Syntax
  × (SourceInfo -> A)
  × (SourceInfo × TokenKind × Text -> A)
  × (SourceInfo × Name × Scopes -> A)
  × (SourceInfo × Delimiter × List<A> -> A)
  -> A

significant_lines: BlockSyntax -> List<LineSyntax>
line_parts: LineSyntax -> List<Syntax>
source_of: Syntax -> SourceInfo
quote_expr: QuotedExpression -> ExprSyntax
fresh_name: Text -> Identifier
```

`significant_lines` drops blank lines and comments from the adapter's semantic view. It does not remove them from the
lossless source tree. An edit still sees the original bytes and ranges.

Both adapters expose the same three jobs:

```text
expand: BlockSyntax -> Result<ExprSyntax, SyntaxError>
edit: BlockSyntax × EditCommand -> Result<List<TextEdit>, EditError>
print: AdapterValue -> Result<BlockSyntax, PrintLoss>
```

`expand` reads source. `edit` changes an existing block without regenerating unrelated text. `print` creates a new
block. These jobs must not be conflated.

## 2. Trial one: staff notation

### 2.1 The source block

This one block covers every required staff feature:

```musa
import syntax std.notation.staff as staff

let clarinet_page = staff:
  instrument bb_clarinet
  written_to_sounding down M2
  clef treble
  key d major
  meter 4/4
  exact_duration shortest_readable

  pickup 1/4:
    | g4/4

  repeat 2:
    | c5/4 d5/4. e5/8 f5/4 ~
    | f5/8 rest/8 [g4 b4 d5]/4 c5(3/8) rest/8

    ending 1:
      | slur:
          tuplet 3/2:
            a4/8
            b4/8
            c5/8
          d5/4
        grace:
          e5
        f5/2

    ending 2:
      meter 3/4
      | g5/4 a5/4 b5/4
```

The block is locally readable:

- every ordinary note states its octave and written value;
- the pickup states its exact expected length;
- `d5/4.` claims a written dotted quarter;
- `c5(3/8)` states only an exact span;
- the tie joins `f5/4` to the `f5/8` at the start of the next bar;
- the tuplet retains three written eighth notes and the `3/2` bracket;
- grace notes have written pitch but no written duration;
- the transposition says how written pitch maps to sounding pitch; and
- the second ending changes meter where the change occurs.

No earlier block supplies a hidden octave, duration, meter, or transposition.

### 2.2 The value produced by expansion

The staff package owns staff concepts. The core language does not:

```musa
type WrittenDuration:
  NoteValue(Nat, Nat)
  Exact(Ratio)

type StaffItem:
  Note(Anchor, WrittenPitch, WrittenDuration, List<StaffMark>, Tie)
  Rest(Anchor, WrittenDuration)
  Chord(Anchor, List<WrittenPitch>, WrittenDuration, List<StaffMark>, Tie)
  Bar(Anchor, Ratio, List<StaffItem>)
  Slur(Anchor, List<StaffItem>)
  Tuplet(Anchor, Nat, Nat, List<StaffItem>)
  Grace(Anchor, List<GraceNote>)
  MeterChange(Anchor, Nat, Nat)
  Repeat(Anchor, Nat, List<StaffItem>, List<Anchor × Nat × List<StaffItem>>)

record StaffDocument:
  instrument: InstrumentName
  written_to_sounding: IntervalTransform
  clef: Clef
  key: WrittenKey
  opening_meter: Meter
  exact_duration_policy: ExactDurationPolicy
  items: List<StaffItem>
```

`StaffItem` is finite recursive data. Its generated fold is later used by validation, engraving, analysis, and
conversion to `Music`. No staff constructor is built into the source calculus.

The complete adapter expansion is the following ordinary expression. `a0` through `a31` are source anchors inserted by
the adapter. They name exact ranges in the block above.

```musa
std.notation.staff.make_document(
  InstrumentName("bb_clarinet"),
  Down(M2),
  Treble,
  WrittenKey(D, Major),
  Meter(4, 4),
  ShortestReadable,
  [
    Bar(a0, 1/4, [
      Note(a1, G(4), NoteValue(4, 0), [], NoTie),
    ]),
    Repeat(a2, 2, [
      Bar(a3, 1, [
        Note(a4, C(5), NoteValue(4, 0), [], NoTie),
        Note(a5, D(5), NoteValue(4, 1), [], NoTie),
        Note(a6, E(5), NoteValue(8, 0), [], NoTie),
        Note(a7, F(5), NoteValue(4, 0), [], TieNext),
      ]),
      Bar(a8, 1, [
        Note(a9, F(5), NoteValue(8, 0), [], NoTie),
        Rest(a10, NoteValue(8, 0)),
        Chord(a11, [G(4), B(4), D(5)], NoteValue(4, 0), [], NoTie),
        Note(a12, C(5), Exact(3/8), [], NoTie),
        Rest(a13, NoteValue(8, 0)),
      ]),
    ], [
      (a14, 1, [
        Bar(a15, 1, [
          Slur(a16, [
            Tuplet(a17, 3, 2, [
              Note(a18, A(4), NoteValue(8, 0), [], NoTie),
              Note(a19, B(4), NoteValue(8, 0), [], NoTie),
              Note(a20, C(5), NoteValue(8, 0), [], NoTie),
            ]),
            Note(a21, D(5), NoteValue(4, 0), [], NoTie),
          ]),
          Grace(a22, [GraceNote(a23, E(5), [])]),
          Note(a24, F(5), NoteValue(2, 0), [], NoTie),
        ]),
      ]),
      (a25, 2, [
        MeterChange(a26, 3, 4),
        Bar(a27, 3/4, [
          Note(a28, G(5), NoteValue(4, 0), [], NoTie),
          Note(a29, A(5), NoteValue(4, 0), [], NoTie),
          Note(a30, B(5), NoteValue(4, 0), [], NoTie),
        ]),
      ]),
    ]),
  ],
)
```

The inferred type is `StaffDocument`. Expansion does not ask for that type. Ordinary resolution finds
`std.notation.staff.make_document`; ordinary inference discovers the result.

### 2.3 How expansion works

The staff transformer uses `fold_syntax` in two passes.

The first pass turns fixed tokens and groups into small staff tokens:

```text
identifier + accidental marks + octave  -> pitch token
"/" + denominator + dot marks           -> written-value token
"(" + exact ratio + ")"                 -> exact-duration token
"[" + pitch tokens + "]"               -> chord token
layout group                             -> parsed child lines
all other fixed tokens                  -> tagged tokens with their source
```

The second pass folds the finite list of lines with this state:

```text
StaffParseState = {
  header fields already seen,
  stack of open layout forms,
  completed top-level items,
  pending tie source and pitch, if any,
  current meter,
  next generated anchor number,
}
```

Each line has one rule. A line either updates a header field, starts a form whose body is already a parsed layout group,
or parses a complete list of staff events. Duplicate headers, an unknown word, a malformed pitch, a missing duration, an
empty chord, a zero tuplet number, and a dangling tie return `SyntaxError`. Closing a bar adds exact spans and checks
the stated length. Closing a tie checks equal written pitch. Closing the document checks that all opened forms ended.

This is a left fold over a finite line list. Nested layout groups were already folded from the leaves upward. The
adapter needs no recursion, private parser, inferred type, or compiler callback.

### 2.4 What expansion does not decide

Expansion records written facts. Later package functions make later choices:

```musa
let page = clarinet_page
let music = std.notation.staff.realize(page, std.notation.staff.literal_realization)
let notation = std.notation.staff.engrave(page, NotationOptions:
  exact_duration_policy: ShortestReadable
)
```

`realize` preserves written and sounding pitch as distinct facts and returns a stated error if a required choice is
missing. Applying `Down(M2)` gives the sounding pitch needed by analysis and performance. `engrave` may display
`Exact(3/8)` as a dotted quarter under `ShortestReadable`; its derivation records
`ChosenWrittenDuration(Exact(3/8), NoteValue(4, 1))`. The source still says `c5(3/8)`.

Grace timing is not chosen here. A performance profile later decides how much time a grace takes and whether it steals
from the principal or the preceding note.

### 2.5 Staff diagnostics retain source

If the first note of the second bar is changed to `f#5/8`, expansion returns:

```text
staff tie changes pitch
  tie starts at a7: f5/4 ~
  continuation at a9: f#5/8
```

If the last rest in that bar is removed, expansion returns:

```text
bar at a8 has length 7/8; meter requires 1
```

Both errors point to original source. Generated expression ranges point back through the expansion record to the same
anchors.

### 2.6 A structured staff edit

The editor command is:

```text
SetWrittenPitch(anchor = a12, pitch = D(5))
```

`edit_staff` returns one text edit:

```text
replace the `c5` range inside `c5(3/8)` with `d5`
```

The source becomes `d5(3/8)`. The `(3/8)`, nearby rest, spacing, comments, and every other byte stay unchanged.

`apply_staff` changes only the pitch in `Note(a12, C(5), Exact(3/8), [], NoTie)`. Re-expanding the patched source gives
that same `StaffDocument` under structural equality. This is the edit law for the command.

The standard printer emits the canonical spellings shown here. Printing can lose comments and layout, so its result
reports `PrintLoss` when the input value contains source-only detail that it cannot reproduce.

## 3. Trial two: a studio graph

### 3.1 The source block

The studio block uses the same fixed lexer, layout groups, and adapter interface:

```musa
import syntax std.studio.graph as graph

let live_studio = graph:
  input notes: note_events
  input expression: control

  instrument lead = poly_sine:
    voices = 16
    attack = 3/100 s
    release = 7/10 s

  processor expression_depth = scale:
    factor = 1/2

  processor room = reverb:
    room = 4/5
    damping = 11/20
    mix = 3/10

  output main: audio(2)

  connect notes.out -> lead.notes
  connect expression.out -> expression_depth.control
  connect expression_depth.control -> lead.expression
  connect lead.audio -> room.audio
  connect room.audio -> main.in

  bind clarinet -> lead
```

This syntax states a finite graph. It does not run it. Every processor, port, connection, parameter, input, output, and
instrument binding is explicit.

### 3.2 The value produced by expansion

The studio package owns the graph-description types:

```musa
type PortKind:
  Audio(Nat)
  Control
  NoteEvents

type StudioDecl:
  Input(Anchor, Text, PortKind)
  Node(Anchor, Text, ProcessorName, List<Parameter>)
  Output(Anchor, Text, PortKind)
  Connect(Anchor, PortPath, PortPath)
  Bind(Anchor, PartName, Text)

record StudioDescription:
  declarations: List<StudioDecl>
```

The complete expansion is:

```musa
std.studio.graph.make_description([
  Input(s0, "notes", NoteEvents),
  Input(s1, "expression", Control),
  Node(s2, "lead", ProcessorName("poly_sine"), [
    Parameter(s3, "voices", Count(16)),
    Parameter(s4, "attack", Seconds(3/100)),
    Parameter(s5, "release", Seconds(7/10)),
  ]),
  Node(s6, "expression_depth", ProcessorName("scale"), [
    Parameter(s7, "factor", Plain(1/2)),
  ]),
  Node(s8, "room", ProcessorName("reverb"), [
    Parameter(s9, "room", Plain(4/5)),
    Parameter(s10, "damping", Plain(11/20)),
    Parameter(s11, "mix", Plain(3/10)),
  ]),
  Output(s12, "main", Audio(2)),
  Connect(s13, PortPath("notes", "out"), PortPath("lead", "notes")),
  Connect(
    s14,
    PortPath("expression", "out"),
    PortPath("expression_depth", "control"),
  ),
  Connect(
    s15,
    PortPath("expression_depth", "control"),
    PortPath("lead", "expression"),
  ),
  Connect(s16, PortPath("lead", "audio"), PortPath("room", "audio")),
  Connect(s17, PortPath("room", "audio"), PortPath("main", "in")),
  Bind(s18, PartName("clarinet"), "lead"),
])
```

The inferred type is `StudioDescription`. As in the staff trial, expansion does not know or request that type.

### 3.3 How expansion and validation divide the work

The transformer folds lines into declarations. It checks only syntax that it owns: name shape, balanced paths, unit
spelling, duplicate parameter text on one node, and the grammar of a connection.

Ordinary package code performs graph checks:

```musa
let checked = std.studio.graph.validate(live_studio)
```

The inferred type is `Result<CheckedStudio, StudioError>`. `validate` resolves processor descriptors, checks parameter
units and ranges, resolves named ports, checks exact port-kind equality, checks bindings, and rejects an instantaneous
cycle. Every declaration carries its source anchor, so these later errors still point to the block.

Preparation remains a later operation:

```musa
let prepared = std.studio.prepare(
  checked_studio,
  gestures,
  bindings,
  seed,
  options,
)
```

It returns a complete result or a stated error. The graph adapter neither allocates processors nor steps audio.

### 3.4 Studio diagnostics retain source

Changing the third connection to this line is syntactically valid:

```musa
connect expression_depth.control -> room.audio
```

`validate` returns:

```text
connection at s15 joins control to audio(2)
  source: expression_depth.control
  target: room.audio
  add an explicit control-to-audio processor if that conversion is intended
```

Changing `mix = 3/10` to `mix = 3/2` returns a range error at `s11`. A duplicate node name points to both declaration
anchors. A graph cycle names every connection anchor in the cycle.

### 3.5 A structured studio edit

The editor command is:

```text
SetParameter(node = "room", parameter = "mix", value = Plain(2/5))
```

`edit_graph` returns one text edit replacing only `3/10` with `2/5`. It does not regenerate the node, reorder
parameters, normalize `s` units, or move comments.

`apply_graph` changes only `Parameter(s11, "mix", Plain(3/10))`. Re-expanding the patched source gives the same
`StudioDescription` as `apply_graph` under structural equality. The studio edit therefore satisfies the same law as the
staff edit.

The graph printer emits a canonical block when a program or desktop action creates a new graph. It is not used to edit
an existing block.

## 4. Does the interface stay general?

The two adapters share all compiler-facing parts:

| Question | Staff | Studio |
| --- | --- | --- |
| Input | `BlockSyntax` | `BlockSyntax` |
| Traversal | `fold_syntax` and finite list folds | `fold_syntax` and finite list folds |
| Output | one ordinary expression | one ordinary expression |
| Result value | `StaffDocument` | `StudioDescription` |
| Later validation | bars, ties, notation policy | descriptors, ports, units, graph cycles |
| Source map | staff anchors | graph anchors |
| Edit result | minimal pitch replacement | minimal parameter replacement |
| Hidden compiler input | none | none |
| Inferred-type access | none | none |

Their musical ideas are different. Their syntax machinery is not.

The trials also answer the five open findings from review 24:

1. **Expansion order:** syntax imports resolve before bounded expression expansion; adapters cannot emit declarations or
   imports.
2. **Public syntax:** four finite constructors and a generated fold suffice.
3. **Token boundary:** one fixed lexer and grouper serve pitches, ratios, units, arrows, paths, and layout.
4. **Editing:** `edit` changes existing source; optional `print` creates new source.
5. **Ordered matches:** the compiler lowers source fallbacks to local joins; adapters need no observable match failure.

## 5. What the trial rejects

The trial does not justify a general macro system. Neither adapter needs to inspect an expected type, create a module,
define a type, run an effect, or evaluate source text. Giving adapters those powers would create more phase coupling
without making either example simpler.

The trial also rejects a single built-in musical syntax. Staff and studio notation are package data with package
parsers. The compiler owns only the fixed token and grouping rules, the finite syntax value, hygienic quotation, source
maps, deterministic limits, and the three adapter operations.

## 6. Decision

Both adapters pass the boundary test. The formal language may use the interface in note 26 with these exact points:

- a block expands to one ordinary expression whose type is inferred later;
- adapter data retains source anchors for errors and edits;
- nested syntax is consumed only through generated finite folds;
- package expansion cannot add imports, modules, declarations, types, or peer adapters;
- later semantic checks are ordinary total package functions or explicit stage operations; and
- the standard staff and studio adapters must satisfy the edit and print laws before implementation conformance passes.

No new core language feature was needed. Strictly positive finite data and generated folds already serve both musical
form and syntax traversal, which meets the two-case admission rule.
