# An explicit surface and a user-owned elaborator

## Purpose

This note answers two questions that the surface work in [21-surface-syntax.md](21-surface-syntax.md) and
[22-syntax-extension.md](22-syntax-extension.md) left joined together and should not be:

> What must the canonical written form guarantee?

> Who gets to decide what the written form looks like?

The answers are: the canonical form guarantees that any chunk of music can be checked from that chunk alone; and
packages decide what notation looks like, through an expansion system modelled on Lean 4's macro layer but stopping
short of its elaboration monad.

Recommendations here land in note 20's Form 1 and the expansion step. None changes inference, complete calls, nominal
data, exhaustive match, or the kernel. This note governs nothing.

## Who writes Musa and who reads it

An earlier draft of this note optimized the surface for a musician typing it. That is the wrong reader model. Most
`.musa` source will be written by a language model and read by a human — in the file, in the desktop score view, or on
the web. Humans will mostly *tune* rather than *author*.

That inverts the objective:

- **Typing cost stops mattering.** Every argument that traded precision for fewer keystrokes is void.
- **Redundancy becomes cheap and often good.** A restated duration is not noise; it is a second copy of a fact that can
  be checked against the first.
- **Local decidability becomes the goal.** A generating model reasons over chunks and a reviewer reads diffs. Both need
  a passage to mean what it says without scanning backward for state that changed it.
- **Machine-checkable assertions become the highest-value syntax.** The errors a model makes when writing music are
  arithmetic: a bar that does not add up, a tuplet that does not close. Syntax that lets the compiler catch those is
  worth more than any economy of marks.

The first draft's metrics — marks per note, frame-to-content ratio, redundant repetitions — measured the wrong thing.
Replace them with:

1. **Chunk closure.** Can a bar's time arithmetic be verified from the text of that bar alone?
2. **Scan distance.** How far back must a reader look to know what an event means?
3. **Checked claims per bar.** How much of what is written is verified rather than assumed?
4. **Round-trip fidelity.** Does the notation an adapter prints re-read as the music it was printed from?

## The one law

> **Time is local. Pitch is absolute. Everything else is interpretation.**

The time arithmetic of any bar is computable from that bar. Clef, key, tempo, dynamic, and instrument change how the bar
is *interpreted, engraved, and performed*; they never change how long anything lasts. Meter participates only as the
value the bar is checked against, and it is declared in the header of the block being read.

This is the invariant that makes chunked reasoning safe, and everything below follows from it.

## What the first draft got wrong

| First draft | Status | Reason |
| --- | --- | --- |
| sticky durations (`c4 c g g`) | **withdrawn** | Non-local. A duration set three bars ago silently governs this one. |
| relative octaves (`from c4`, `'`/`,`) | **withdrawn** | Non-local, and the source of LilyPond's most notorious error class. |
| note defaults derived from the meter | **withdrawn** | A default that must be inferred from a header two levels up is not obvious in the chunk. |
| collapsing `score`/`part`/`voice` | **withdrawn** | `part` carries instrument, transposition, clef, staff count, score order, and studio binding. |
| the collision with LilyPond's note token | **moot** | It only existed because the draft borrowed LilyPond's stickiness. Without that, `c4` has no competing reading. |
| `tempo 4 = 104` | **withdrawn** | Note 21's original `tempo 1/4 = 104` is fully explicit and was right. Meter-derived beat is available as adapter sugar but should not be canonical. |
| `|` bar margin with a checked closing bar | **kept, and stronger** | With explicit durations the check is a pure assertion over written text: the highest-value syntax in the whole design. |
| `field: value` in records; `=` binds | **kept** | Ambiguity fix, not a brevity fix. |
| one-line `fn` bodies; qualified library naming | **kept** | Both remove ambiguity or nesting, neither trades away precision. |
| `->` and bar alignment in the formatter | **kept** | Free, and diff-friendly. |

## The one note-token change still worth making

Note 21's `c4/4` is close to right, and the remaining problem is precision, not verbosity. Write the duration as a
complete ratio, bound to the pitch by a mark that is not `/`:

```musa
| c4:1/4 c4:1/4 g4:1/4 g4:1/4 | a4:1/4 a4:1/4 g4:1/2 |
```

Four things improve, all of them about exactness:

- **The two `4`s stop colliding.** In `c4/4` the reader — human or model — must know that the first digit is a register
  and the second is a denominator. Here the register attaches to the letter and the duration is a whole ratio.
- **Dots stop being a special case.** A dotted quarter is `3/8`; double-dotted is `7/16`. No dot-counting, no arithmetic
  the reader performs in their head.
- **Tuplets become writable at all.** A triplet eighth is `1/12`. `c4/4.` has no way to express it without new grammar.
- **The surface says what the kernel means.** Musical time is exact rational (constitution §4). Any other duration
  spelling is a code the reader must decode into rationals before reasoning about a bar.

Group sugar stays available for the case where writing `1/12` three times is error-prone, because a tuplet bracket
already means exactly this: `(f4 g4 a4):1/4` is "these three notes fill a quarter." Under exact rationals, no tuplet
vocabulary is needed — the group form is a convenience over an exact expansion, and the adapter infers the printed
bracket from a denominator that is not a power of two.

`:` now labels at three scales — `piece "x":` opens a detail block, `sample_rate: 48_000` labels a field, `c4:1/4`
labels a duration. That is one job stated generally: **`:` introduces the detail of the thing on its left.**

Everything else about the note token stays as note 21 had it. The honest summary is that the note token was close to
right, and that the wins are in the checks and the extension model rather than the spelling.

## The extension answer, after Lean 4

Lean 4 is the right reference because it separates layers that note 22 currently merges, and because it has a piece Musa
needs that neither Racket nor Rhombus makes as prominent.

### Take: the macro layer

Lean's `macro_rules` runs in `MacroM`, which can pattern-match syntax, build syntax by quotation, generate hygienic
names, and fail with an error. It cannot see the environment's elaboration results, cannot see an expected type, and
cannot run arbitrary effects. That is note 22's `Transformer(InputKind, OutputKind)` almost exactly. The design is
already right; it is under-specified rather than wrong.

### Take: `Syntax` as an ordinary inductive datatype

This is the direct answer to finding H2, which said Musa never states what `Syntax` is or how a transformer takes one
apart. Lean's answer is a four-constructor inductive carrying source information:

```text
type Syntax:
  Missing
  Atom(info: SourceInfo, text: Text)
  Ident(info: SourceInfo, name: Name, scopes: List(MacroScope))
  Node(info: SourceInfo, kind: SyntaxKind, children: List(Syntax))
```

Musa can copy this shape almost verbatim. One difference matters: Musa's transformer language has no general recursion,
so the eliminator is the generated fold that note 19 already requires for strictly positive data, not a recursive
function. A staff adapter folding over a grouped block of events is well within that, and nested groups (tuplets, beams)
fold over the tree. Some transformers that would be natural to write recursively must be written as folds — the same
cost note 19 already accepted for user data, now paid in one more place.

### Take: delimited syntax categories, not an environment-dependent parser

Lean's parser consults the environment, so `syntax` declarations change how later text parses. Musa should not copy
that. Finding H3 argues the extension boundary belongs at grouping: one universal lexer, one universal layout grouper,
and adapters that rearrange core tokens inside a region entered by name. Lean's own `declare_syntax_cat` supports
exactly this delimited case, and it is what keeps the lossless tree, the formatter, and the tree-sitter grammar on one
set of rules.

### Refuse: the elaboration monad

`TermElabM` is where Lean gets its power: the expected type, metavariables, unification, and postponed elaboration. Musa
must not take it.

Note 19 commits to rank-1 Hindley–Milner with principal types, and note 22 already forbids macro access to inferred
types to keep inference acyclic. Those are the same commitment. A transformer that reads an expected type makes
expansion and inference mutually recursive, and principal types do not survive it. Lean can afford `TermElabM` because
its elaborator is bidirectional and dependently typed from the start, with metavariables and postponement built in.
Musa's is not, and adopting `ElabM` would mean giving up note 19 entirely rather than extending it.

The usual reasons to want type-directed elaboration are overloaded literals and overloaded operators. Note 19 forbids
overloading anyway, so the pressure that justifies `TermElabM` in Lean does not exist in Musa. Refusing it costs less
here than it would in almost any other language.

### Add: printers, which nobody has proposed

Lean pretty-prints elaborated terms back into surface notation through delaborators and `@[app_unexpander]`. Musa needs
the same direction and has a harder requirement than Lean does.

Constitution §1 says the source is the master record and that moving a note in the score editor changes the source.
Finding H4 observed that package-owned notation therefore breaks structured editing: the editor cannot write text in a
grammar it does not know. A printer solves it. An adapter ships a pair:

```text
expand : Syntax(Block) -> Result(Syntax(Expr), SyntaxError)
print  : Music -> Result(Syntax(Block), PrintLoss)
```

The editor's round trip becomes: user drags a note, the session computes the new music, the owning adapter prints it in
its own notation, and the result is applied as a text edit to the region it came from. `print` returns `Result` because
an adapter's notation need not cover every music it might be handed — a staff adapter cannot print a phrase-led gesture
— and the loss is the same kind of object the derivation records already carry.

This replaces H4's narrow repair. Instead of "only the standard library's adapters get structured editing," the rule
becomes **an adapter that ships a printer gets structured editing; one that ships only an expander is read-only in the
score view.** That is a conformance level a package can choose and a test can verify.

### Adapter conformance

Three obligations, all testable, on top of note 22's six expansion obligations:

1. **Round trip.** For music `m` that an adapter can print, `expand(print(m))` elaborates to `m`. This is a property
   test, and it is what makes the desktop app's editing model sound.
2. **Header-scoped defaults.** Any default an adapter honours must be recoverable from the header of the block being
   read. An adapter may not carry a default across a block boundary. This is the law "time is local" applied to
   extensions rather than to the standard library.
3. **Printable checks.** Whatever an adapter checks — bar completeness, cycle length, tuplet closure — it names in its
   diagnostics with the source range of the region that failed, not of its expansion.

## What this settles

The terse-versus-explicit argument stops being a language decision.

The standard library ships one canonical staff adapter that is fully explicit and locally decidable — the form above,
every event self-describing, every bar checked. That is what a model writes and what a diff shows. Anyone who wants
LilyPond's stickiness, or a tracker grid, or counting syllables, writes an adapter and gets it, along with a printer
that converts back. The language commits to the guarantee, not to the spelling.

It also settles what the extension system is *for*. Note 22 justified it with one example that the compiler will
probably implement in Rust anyway. The justification is stronger than that: it is the mechanism by which a project can
have both a canonical form suited to machine authorship and a human-facing form suited to whoever is reading — without a
second editable representation, which the constitution forbids.

## What it does not settle

- **The token boundary (H3).** Whether adapters can use marks outside the core token set is still open, and the
  `expand`/`print` pair does not change it.
- **Spanners (M6).** Ties, slurs, hairpins, and 8va are the one thing that is genuinely not local, since a tie relates
  two events in different bars. Explicit durations make this sharper, not softer. It needs its own design, and it is the
  first place the "time is local" law will be tested.
- **Missing notation.** No example in notes 20–25 shows a rest, a pickup bar, a repeat, or a first and second ending.
  All four are ordinary, and a musician opening a file will look for them before anything else.
- **Argument order.** Note 21 writes `transpose(P5, phrase)`; note 23 writes `transpose_spelled(pitch, P5)`. Fix one
  order — transformed thing first — and hold the standard library to it.

## Recommendation

Adopt the law, the explicit note token, and the Lean-derived split: take `MacroM`, refuse `TermElabM`, copy the `Syntax`
datatype with a generated fold, keep expansion delimited by grouping, and require a printer from any adapter that wants
to be edited.

Then write the five paper programs in the canonical form and measure chunk closure and checked claims per bar rather
than marks per note. If a model can write a page of it and the compiler can catch the arithmetic errors from the text
alone, the surface is doing its job, however many characters it takes.
