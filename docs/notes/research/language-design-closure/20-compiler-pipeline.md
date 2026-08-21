# A small compiler pipeline for Musa

## Purpose

This note decides how many internal program forms Musa needs and what each one is for. It adapts the useful parts of
rustc's AST, HIR, THIR, MIR, and query design without copying a pipeline built for borrow checking and native code.

The answer is four front-end forms, followed by several musical targets. Each form must make one class of questions
easy. If two forms answer the same questions, one should go.

This is a research proposal. It governs nothing and does not authorize compiler changes.

## The whole path

```text
.musa text
    |
    v
lossless syntax tree
    |  expand hygienic syntax adapters; resolve names and modules
    v
resolved program
    |  infer types; check data, matches, and module boundaries
    v
typed body
    |  make evaluation order, captures, cases, and primitives explicit
    v
evaluation core
    |  run the total source program
    v
typed source values
    |
    +--> contextual Music --close with context--> Term<ScoreFact>
    |                                           --> Timeline<ScoreFact>
    |                                               +--> NotationPlan --> notation files
    |                                               +--> AnalysisReport
    |                                               +--> Timeline<Gesture>
    |
    +--> performance intent -----------------------> Timeline<Gesture>
    |
    +--> studio description
              |
              +-- with gestures, bindings, seed, and options --> PreparedExecution
                                                                  |
                                                                  +--> audio steps
```

Notation and audio share the source language, resolved names, inferred types, values, source anchors, and derivation
records. They do not share one final representation. A score timeline has finite musical extent. A prepared audio graph
has state and advances one step at a time.

## What rustc teaches us

The Rust compiler keeps several representations because each supports different work:

- its AST stays close to parsed source;
- HIR resolves and simplifies source for analysis;
- THIR contains fully typed bodies and supports match checking and MIR construction;
- MIR makes control flow explicit for borrow checking, data-flow analysis, optimization, and code generation; and
- LLVM IR belongs to the native-code backend.

Rust also computes many facts through memoized queries rather than one unconditional march through every pass. A query
records which other queries it used, so an edit invalidates dependents rather than the whole compiler.

Musa should copy the separation of questions, not the names or scale. Musa has no borrow checker, unsafe-code checker,
drop elaboration, monomorphization, native control-flow optimizer, or LLVM backend. A control-flow graph would add a
large form with no present job.

The relevant rustc sources are the official *Rust Compiler Development Guide* pages on the
[compiler overview](https://rustc-dev-guide.rust-lang.org/overview.html),
[HIR](https://rustc-dev-guide.rust-lang.org/hir.html), [THIR](https://rustc-dev-guide.rust-lang.org/thir.html),
[MIR](https://rustc-dev-guide.rust-lang.org/mir/index.html), and
[queries](https://rustc-dev-guide.rust-lang.org/query.html).

Peyton Jones gives the complementary lesson for a small functional language: translate the friendly surface into a small
enriched core, retain `let` for polymorphic checking and sharing, compile patterns into cases, and lower each
convenience in a separate, checkable step. Musa combines that method with rustc's source-like and typed-body split.

## Form 1: the lossless syntax tree

### Its question

> What did the musician write, including incomplete or invalid text?

`musa-syntax` already owns this form. It retains every token, comment, space, and byte range. The formatter, syntax
highlighter, text edits, and editor recovery use it.

This form should contain no resolved names, inferred types, pitches computed from context, expanded motifs, or audio
objects. It is a faithful syntax record, not a semantic model.

Musa does not need a second conventional AST. Typed syntax wrappers over the lossless tree provide the convenient read
API. Copying the whole document into another unresolved tree would create cost and drift without answering a new
question.

### Output contract

Parsing always returns a tree and syntax diagnostics. Each semantic node has a source range. Invalid regions remain in
the tree so editor features can continue around them.

## Expansion is an operation, not another stored program form

Some written forms have a smaller meaning. An if expression can become a two-branch match. A staff block can turn c#4/4
into constructors and a complete notation call. A studio block can turn named nodes and wires into an ordinary studio
description.

The compiler expands these forms between parsing and resolution. Each expansion returns its output and a map back to the
written form. The map lets type errors point at the code the musician wrote. It also lets the formatter and editor keep
showing that code instead of the compiler's replacement.

Built-in forms and package syntax adapters use one hygienic expansion engine. Expansion operates on finite syntax
objects with lexical scopes, source locations, and expansion history. It is a compiler operation, not a fifth full
program representation that every later pass must retain. [The syntax-extension decision](22-syntax-extension.md) states
the bounded public contract.

## Form 2: the resolved program

### Its question

> Which declaration does each name mean, and what source-level construct remains after simple sugar is removed?

This is Musa's HIR-like form. It belongs privately to `musa-compiler`.

It contains:

- one build-local id for each value, function, type, constructor, module, and source file;
- a finite module and declaration dependency graph;
- source anchors on every declaration and expression;
- ordinary function, closure, `let`, product, constructor, record, and `match` forms;
- explicit calls with complete positional argument lists;
- explicit nominal type and constructor references; and
- music, performance, and studio forms that still resemble the source vocabulary.

It removes only sugar whose meaning does not require inferred types. For example, a written multi-parameter declaration
may become one parameter pattern, and a library shorthand may become its declared ordinary function call. Type-directed
choices do not happen here.

Name resolution happens once. Later forms use ids rather than look names up again. A source anchor remains attached, so
an error still points to the written name rather than an internal id.

### Why modules belong here

Imports, signatures, structures, and package paths decide which declarations exist before expression inference can
begin. Resolution builds the finite graph for the current build. It does not assign a persistent compiled identity.
Abstract nominal types receive fresh build-local ids.

### Output contract

A successful resolved program has no unknown names, duplicate declarations, dependency cycles among values, or illegal
private references. It may still be ill typed.

## Form 3: the typed body

### Its question

> What is the most general type of each expression, and does every match and module boundary make sense?

This is Musa's THIR-like form. It is created one declaration body at a time and may be dropped after lowering. The
resolved program and inferred public schemes remain available for editor queries.

It adds:

- one inferred monotype for every expression and pattern;
- one generalized type scheme for each eligible non-recursive `let` or exported definition;
- explicit instantiations of a polymorphic name at each use;
- the chosen nominal constructor and field types;
- an exhaustiveness result for each match;
- checked positivity and generated fold types for recursive data; and
- checked module sealing, including hidden constructors.

It retains `let`, closures, constructors, records, and source-shaped matches. Error messages are best here because the
program still looks like the code the musician wrote.

### Inference order

The compiler processes declarations in dependency order. For one body it:

1. assigns fresh type variables to unknown local types;
2. gathers and solves equality constraints by unification;
3. reports a mismatch or infinite type at the smallest useful source expression;
4. generalizes variables not fixed by the surrounding environment; and
5. checks any written annotation or module signature against the inferred scheme.

There is no separate “expected type required” rule for `None`, `[]`, `Ok`, or `Err`. Their polymorphic constructor
schemes supply fresh variables that later uses may constrain.

### Output contract

A successful typed body has principal inferred types, exhaustive matches, legal constructor access, and no unresolved
type variables except variables generalized in a scheme. It has not yet run the program or constructed music.

## Form 4: the evaluation core

### Its question

> What exact total computation should the evaluator perform, in what order, and which source anchor caused each step?

This is the small counterpart of MIR, but it is not a control-flow graph. Musa's source language is pure and total, so a
typed expression-and-case language is the more natural form.

The core contains only:

- literals and local variables;
- products and nominal constructors;
- field selection;
- explicit closures with their capture lists;
- non-recursive `let`;
- complete function calls;
- decision trees over constructor tags;
- finite folds over strictly positive data;
- saturated compiler primitives; and
- typed construction of contextual music, performance intent, and studio descriptions.

Evaluation order is explicit and left to right. A lowering pass may use an administrative form such as:

```text
let x = evaluate first argument;
let y = evaluate second argument;
let result = call f with (x, y);
return result;
```

This does not mean users see temporary variables. It gives the evaluator, resource meter, and proof one unambiguous next
step.

Pattern syntax is gone. A checked match becomes a decision tree that tests a constructor once, extracts its fields, and
enters one branch. The compiler never carries an internal “pattern failed, try later” value because source matches are
exhaustive and ordered before lowering.

### Polymorphism at runtime

Musa does not monomorphize inferred functions as Rust does. Rank-1 parametric code has one implementation. Type schemes
are checked before evaluation, and type arguments are erased. The typed core retains enough type information for proofs,
debug output, and validation, but runtime values do not carry a copy of every inferred type.

Nominal constructor tags remain because evaluation must distinguish data constructors. Their ids need be unique only
inside the current build.

### Compiler-backed functions

A compiler-backed source function is an ordinary closure whose body invokes a saturated primitive. For example, the
source name `transpose` may lower to the equivalent of:

```text
closure (interval, music) {
    primitive transpose(interval, music)
}
```

Passing `transpose` stores the ordinary closure. The primitive itself is not a first-class value and has no partial
application state.

### Output contract

Lowering preserves the inferred type, source anchor, and free-variable meaning. The evaluation core has one next-step
rule for every non-value. Evaluation either produces a typed value, produces an explicit `Result` error as data, or hits
the deterministic compiler resource limit. It cannot loop.

## The common front end ends at values

Rust's common pipeline continues to machine code. Musa's should stop at typed values because its backends consume
different mathematical objects.

One source evaluation may produce several kinds of value:

- ordinary theory-library data;
- a contextual `Music` recipe;
- a finite performance instruction or protocol;
- a studio description; or
- an analysis request or rule set.

These values share a language, not a forced representation. Target adapters begin here.

## Musical target 1: close contextual music

A `Music` value is a finite recipe. It may ask for an explicit musical context such as a scale or meter. Closing it
supplies that context and either returns a closed `Term<ScoreFact>` or a stated error.

The closed kernel term contains exact rational placement and typed occurrences. It is the first public semantic object
in this route. `musa-kernel` checks and normalizes it into `Timeline<ScoreFact>`.

The source evaluator does not directly emit an engraving or sample buffer. That would erase the decisions between
musical intent, notation, performance, and sound.

## Musical target 2: notation and analysis

A notation adapter reads `Timeline<ScoreFact>` plus the explicit notation context it needs. It returns a private
`NotationPlan`, then MEI, LilyPond, MusicXML, MIDI, or another export.

An analysis reads the representation named by its contract and returns findings with supporting evidence. Some analyses
read score facts; another may read gestures or audio observations. “Analysis” is not one universal compiler pass.

Both outputs record which input anchors they used, what they added, and what they could not preserve.

## Musical target 3: performance gestures

A performance profile interprets notation and musical intent under explicit tempo, groove, tuning, instrument-neutral
technique, and realization choices. Its finite result is `Timeline<Gesture>`.

Not every gesture must come from notation. A phrase-led source value may enter this target through its own explicit
adapter. That route records the transcription or notation information it did not have. It does not manufacture a score
as an intermediate step.

## Musical target 4: prepared sound

Preparation combines finite gestures, instrument and studio bindings, a seed, sample rate, channel layout, and options.
It returns an opaque `PreparedExecution` or an error.

The private audio form is a validated process graph with allocated state and a fixed schedule. It is not the source
evaluation core and not a temporal-kernel term. Repeated process steps extend an audio history; an unbounded live run
does not normalize to one finite value.

## Source anchors and derivation records

Every lowering keeps two related records:

1. A **source map** connects private compiler nodes to the written syntax for diagnostics, hover, rename, and debugging.
2. A **derivation record** connects musical representations across meaningful stage changes.

These are not the same object. Renaming a temporary in the evaluation core is a source-map concern. Turning one written
note into several performed gestures is a derivation concern with explicit preservation, generation, or loss.

The resolved program creates stable anchors for one compilation. Later front-end forms retain them. A target adapter
uses those anchors when it creates its derivation edges. No backend reconstructs origin by matching equal values after
the fact.

## Queries without a cache calculus

Musa can borrow rustc's demand-driven shape without promising persistent compiled identities or a disk cache.

Within one `ProjectSession`, private memoized queries may include:

```text
parse(document_revision)
resolve(module_id)
infer_scheme(definition_id)
typed_body(definition_id)
evaluation_core(definition_id)
evaluate(root_definition_id)
close_music(root_value_id, musical_context)
kernel_timeline(closed_term)
notation_plan(timeline_id, notation_options)
gesture_timeline(intent_id, performance_context)
prepared_execution(gestures_id, bindings, seed, options)
```

Each query has exact typed inputs and a deterministic result. It records dependencies on other queries. Editing one
definition invalidates its dependents, not unrelated modules or backend products.

This is an in-memory revision cache, not a package ABI. Build-local ids may change after a new build. A future disk
cache needs its own exact encoding and equality proof.

### Resource limits must ignore cache warmth

The language's deterministic resource limit cannot charge less because a query happened to be warm. Each accepted query
result therefore carries a logical cost summary, or the compiler performs an equivalent deterministic preflight. A cache
hit charges the same logical work as the corresponding miss. Wall-clock work may shrink; language acceptance does not
change.

## Diagnostics and editor use

Different tools stop at different stages:

| Tool question | Last required form |
| --- | --- |
| Format or highlight incomplete text | lossless syntax tree |
| Go to definition or rename | resolved program |
| Show a type or type error | typed body or stored inferred scheme |
| Check match coverage | typed body |
| Explain evaluation or resource use | evaluation core |
| Show the elaborated score | closed kernel term or score timeline |
| Engrave | notation plan |
| Play | prepared execution |

The LSP may recover partial answers from invalid source, but it does not publish the private forms as a public crate
API. It asks narrow compiler queries and receives editor facts.

## Why there is no Musa LLVM

LLVM IR is useful because many source languages converge on native machine operations. Musa's important targets do not:

- engraving needs notational structure and spelling;
- analysis needs musical facts and evidence;
- performance needs gestures and contextual timing;
- audio preparation needs a stateful process graph; and
- live audio needs real-time steps.

Forcing these through one low-level instruction language would discard the distinctions Musa exists to keep. The
evaluation core is the last common **program** form. The derivation diagram, not another universal IR, connects the
different musical outputs.

## Ownership in the Rust workspace

| Form or operation | Owner | Public? |
| --- | --- | --- |
| lossless syntax tree and typed syntax wrappers | `musa-syntax` | narrow parsing and editing facade |
| resolved program, typed body, inference tables | `musa-compiler` | no |
| evaluation core and source evaluator | `musa-compiler` | no |
| contextual recipes and target adapters | `musa-compiler` | recipes private; caller-ready facts public |
| exact temporal term and timeline | `musa-kernel` | yes, through its small algebra |
| notation plan and exporters | `musa-notation` | plan private; render operations public |
| process validation and prepared execution | `musa-dsp` | graph private; preparation public |
| live stepping and device transport | `musa-playback` | engine facade only |
| revision inputs and query coordination | `musa-project` | session facade only |

No dependency direction changes.

## Difference from the current compiler

The current compiler traverses the lossless syntax with typed wrappers, while much of raw declaration building,
checking, evaluation, closure handling, and built-in dispatch lives together in `core.rs`. Other music elaboration walks
live beside it.

The proposal does not call that code wrong. It says where the next language should create seams:

- resolution produces one explicit source-like program;
- inference consumes that program and returns typed bodies and schemes;
- lowering consumes only successful typed bodies;
- evaluation consumes only the small core; and
- musical adapters consume typed values rather than syntax nodes.

This separation should happen only after the language proof passes. It is a replacement path with differential tests,
not a request to split files before the data contracts exist.

## Pass laws

Every arrow above needs a short law and a direct test.

1. **Parsing is lossless:** printing the syntax tree reproduces the source bytes.
2. **Resolution is binding-safe:** each use names one permitted declaration, and alpha-renaming ids does not change the
   program.
3. **Inference is principal:** every other valid type is an instance of the inferred scheme.
4. **Typing survives lowering:** resolved and lowered bodies have the same inferred result type.
5. **Lowering preserves results:** evaluating a typed body and its evaluation core gives related values.
6. **Evaluation is total and deterministic:** one accepted closed core program yields one value within its stated
   logical cost or receives the same resource rejection on every run.
7. **Music closure is checked:** success yields a closed well-typed kernel term; failure names the unsatisfied request.
8. **Target derivations compose:** adjacent records meet at the exact same representation and anchors.
9. **Audio steps are deterministic:** equal prepared state and equal input frames yield equal next state and output.

The proof should follow these arrows. A theorem that ranges over three undocumented internal representations is a sign
that the pipeline is still too vague.

## Recommendation

Use the four front-end forms above. Do not add a CFG-based MIR, a universal musical IR, or an LLVM-like backend layer.
Build a small demand-driven compiler around exact stage contracts, but defer persistent query caching until the source
language and package format exist.

Keep one source-mapped, hygienic expansion operation before resolution. Built-in forms and bounded package syntax
adapters use it. Ordinary source values do not become syntax values, and macro expansion does not inspect inferred
types.

The next research artifact should rewrite the five complete musical programs against this pipeline. Each program should
show where inference ends, what typed value evaluation returns, which target adapter it selects, and which derivation
record connects the result to the next stage.
