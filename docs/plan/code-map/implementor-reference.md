# Implementor's reference

For someone changing the compiler. It says where each decision lives, what it may assume, and what it owes downstream.
It does not restate the specification: each section names the document that decides, and adds the orientation a
specification does not.

## 1. The pipeline, and who owns what

Dependency direction is one-way and never points back:

```text
musa-language → musa-compiler → {musa-notation, musa-dsp} → musa-playback → musa-project → {musa, musa-lsp, musa-desktop}
                     ↑
                musa-kernel (leaf)
```

| Crate | Owns | Never exposes |
| --- | --- | --- |
| `musa-language` | tokens, lexer, parser, lossless CST, formatter, text edits | Rowan types |
| `musa-kernel` | exact time, coordinates, typed occurrences, `empty`/`event`/`follow`/`together`/`map_payloads`/`duration`, normalization | anything musical |
| `musa-calculus` | the dependently typed core calculus: terms, NbE, elaboration, inductive families | `Value`, the evaluator, quotation |
| `musa-score` | the musical values: pitch, chords, scales, exact time, marks, score and performance snapshots, provenance, diagnostics, analysis | any way to *build* one from text |
| `musa-compiler` | resolution, typing, expansion, elaboration into the kernel — the passes that compute those values | pass types, `Type`, the resolver |
| `musa-notation` | `NotationPlan`, MEI, LilyPond, MusicXML, MIDI | intermediate plan internals |
| `musa-project` | `ProjectSession`: documents, revisions, commands, exports, facts | compiler internals, byte offsets |

`musa-lsp` is the one shell that also depends on `musa-language`, because highlighting and completion must answer on
half-typed source, which a session's facts cannot describe.

Three rules follow, and they are the ones most often reached for:

- **No public item without a caller.** A facade grows when something needs it, not in anticipation.
- **The source is canonical.** No second editable AST, no mutable expanded cache. Every UI edit is a text edit.
- **Exact time.** Musical time is `num-rational`. Floats appear at the performance and DSP edge and nowhere earlier.

## 2. Surface to kernel

The surface grammar is settled in [`01-surface.md`](../../rules/language/01-surface.md) §1; the total value calculus and
its metatheoretic obligations are [`02-core-calculus.md`](../../rules/language/02-core-calculus.md); the elaboration
rules per construct are `../../rules/kernel/06-surface-elaboration.md`.

The shape to hold in mind:

1. **Parse** into a lossless CST. Every node keeps its trivia, so a formatter round-trips and a doc comment is a fact
   about the tree rather than a line-scan.
2. **Resolve** names, imports, templates, signatures, and structures. This pass records what an editor is told —
   `ItemDoc` per checked declaration, `NameReference` per use.
3. **Check** the value calculus. Total, strongly normalizing, monomorphized. Types are `musa-compiler`'s own; they do
   not cross the crate boundary.
4. **Elaborate** into a core term: an `EventTrack[WrittenTime, ScoreFact]` built from `track`, `follow`, `together`,
   `shift`, `scale`, and `restrict`, with `let` for sharing.
5. **Normalize** the term (`../../rules/kernel/05-normalization.md`), which fixes occurrence order, payload
   serialization, semantic equality, and the semantic hash.
6. **Project** into a `ScoreSnapshot` and a performance snapshot, which is what `musa-notation` and `musa-dsp` consume.

The kernel is a leaf and stays one. A surface convenience must never become a seventh basis operation: if a construct
cannot be elaborated from the six that exist (`../../rules/kernel/00-purpose.md`), the specification is what changes,
not `musa-kernel`.

### Typing, briefly

Judgments are in [`02-core-calculus.md`](../../rules/language/02-core-calculus.md) §2 and the staging judgments in
[`00-semantics.md`](../../rules/language/00-semantics.md) §2. Two things surprise newcomers:

- **Reusable material is an ordinary value.** Prompt 127a deleted the contextual `Music` type: a fragment is a value of
  type `EventTrack[WrittenTime, ScoreFact]`, a motif is a function returning one, and placement is applied by the
  enclosing voice's left fold rather than read from an ambient context (`../../rules/language/00-semantics.md` §3). The
  code still spells the old type; prompt 142 removes it.
- **A nullary `fn` is a function.** `fn f() -> T` has type `() -> T` and is called `f()`, and the record an editor shows
  says so rather than spelling it `let f: T`. There is one deliberate exception, and it is the motif affordance: a bare
  reference to a nullary `() -> EventTrack[WrittenTime, ScoreFact]` function *where a track is expected* is applied, so
  `use subject;` and `use subject();` mean the same thing. It is one case in the checker, not a general coercion.

### Context requirements

A construct that needs a fact from context — `step` needs a collection, `assert fills_meter()` needs a meter, a degree
needs a frame — states the requirement and fails with a diagnostic naming what was missing, rather than inventing a
default. `examples/broken/no-scale-in-force.musa` is the shape of that failure.

## 3. A worked trace

`examples/canon-functions.musa` is two notes and a transformation, and it exercises the whole path.

**This trace is the post-127c output.** The temporal spellings are current — `EventTrack`, `track`, `follow`,
`together`, `% musa-kernel-2`. The type name `Music` is not: prompt 142 replaces it, and the pairs still owed are in
[`../clean-break-ledger.md`](../clean-break-ledger.md). What the trace *shows* about provenance, sharing, and exact time
is unchanged by either rename.

The source:

```musa
fn canon(subject: Music, answer: Music -> Music, gap: Duration) -> Music {
    together(subject, shift(gap, answer(subject)))
}
```

```musa
use canon(subject, octave_answer, 1/2);
```

`musa kernel examples/canon-functions.musa` prints the elaborated term:

```text
% musa-kernel-2
kernel "Canon Functions" {
  composition main : EventTrack[WrittenTime, ScoreFact] =
    let shared0 = together {
        track 1/2 {
          occurrence "voice … note c4 1/4 [… def 300:304 #1]" from 0 to 1/4;
          occurrence "voice … note d4 1/4 [… def 313:317 #1]" from 1/4 to 1/2;
        };
        shift by 1/2 track 1/2 {
          occurrence "voice … note c5 1/4 [… def 300:304 #1 via transpose 7 12]" from 0 to 1/4;
          occurrence "voice … note d5 1/4 [… def 313:317 #1 via transpose 7 12]" from 1/4 to 1/2;
        }
      }
    in together {
      shared0 @ "depth 0 origin 635:674 scope voice 0 0 via motif 635:674";
      track 1 {
        occurrence "piece meter 4/4 [0:0]" from 0 to 1;
      }
    };
}
```

Read what each part is doing.

- **`together` and `shift by 1/2`** are the `canon` function's body, elaborated. Nothing about the term remembers that a
  function was involved; what it remembers is where the notes came from.
- **`def 300:304`** is the byte span of the *declaration* the note came from — the `c4/4` inside `subject`. Both the
  original and the transposed copy carry the same `def`, because there is one declaration and two occurrences.
- **`via transpose 7 12`** is an expansion step: the interval as a written pair, staff displacement and chromatic
  displacement, so provenance keeps the spelling the operation kept.
- **`shared0` and `@ "…"`** are sharing. `subject` is elaborated once and referenced; the `@` annotation records the
  locus each reference stands at, so two placements of one phrase are distinguishable without the material being
  elaborated twice.
- **`piece meter 4/4`** is a context fact, occupying its own span. Meter is a fact about the passage, not a property of
  a note.
- The durations are exact rationals throughout. `1/4` is a quarter, not 0.25.

From the term, `musa-notation` builds a `NotationPlan` and then MEI, LilyPond, MusicXML, or MIDI. Every rendered element
can name the occurrence it came from, and every occurrence can name the source span, which is what makes clicking a note
on the page move the caret to the text that wrote it.

## 4. Provenance

Provenance is not a debugging aid bolted on. It is a component of the payload, and every pass that creates an occurrence
owes it.

An occurrence carries the **declaration** it came from, the **expansion path** of steps taken to get there (`motif`,
`transpose`, `splice`, `assertion`, and the rest), and, for a piece that leaves something open, the **choice path** that
says which alternative was taken. The interface contract is `../../rules/desktop/04-provenance.md`; the realization
model, including seeds and decisions, is `../../rules/kernel/11-realization.md`.

Three invariants to preserve when adding a pass:

- **Plural origins stay plural.** A note that two sources contributed to has two origins. Choosing the convenient one is
  a bug that shows up much later as an editor jumping to the wrong place.
- **Spans index the document that produced them.** A span from a bundled module travels with its URI. In `musa-project`
  this is enforced structurally: the UTF-16 translation walk rewrites two-field `{start, end}` objects using the open
  document's index, and a foreign span is carried in a four-field shape so the walk passes it over.
- **A generated fact is not editable.** Navigating to the editable source is the answer; synthesizing an edit into
  generated, quoted, or included material is not.

## 5. Resource acceptance

Termination is not enough: a total language can still ask for a score nobody can hold. One deterministic meter runs over
checking and evaluation, charging a finite operation's known count *before* it enters its loop
([`02-core-calculus.md`](../../rules/language/02-core-calculus.md) §4). It covers monomorphized definition count and
closure environment size, fold work including products induced by nesting, generated occurrence and kernel binding
counts, instantiation count and dependency depth, and quotation size after substitution.

Two properties matter to a caller:

- **Rejection is deterministic and explains itself.** The diagnostic names the operation, the metric, the amount
  attempted, and the limit. These are language-version constants, not timeouts and not observations of the machine.
- **Nothing partial escapes.** Values stay private until the whole declaration graph succeeds, so exhaustion publishes
  neither a partial value nor a partial score.

Interactive cancellation is a compiler operation, not a language effect.

## 6. Caching and revisions

`musa-project` holds a document, its revision, and the last revision that compiled. The interface consumes facts, never
compiler internals.

- Every fact a snapshot hands out is tagged with the revision it came from. The engraved score, the playback plan, and
  the recorded declarations all come from the *last valid* revision, which may be older than the text on screen; the
  snapshot says which.
- Semantic identity is the kernel's `SemanticHash` over the normalized term. Two documents that differ only in
  whitespace, in declaration order where order does not matter, or in a name that was inlined, hash the same.
- Expanding an Origin chain or an analysis finding must not trigger recompilation. Everything those views show was
  already recorded by the compile they belong to.
- An analysis report carries the revision it read, so an interface can say "this reading is of an older score" as a fact
  rather than a guess, and can leave the report on screen rather than emptying it.

## 7. Modules, ownership, and what is public

[`04-templates-and-modules.md`](../../rules/language/04-templates-and-modules.md) decides; the operational consequences
are:

- **A signature seals.** A member a signature does not list is private to the structure that defines it, and naming it
  from outside is an error rather than a coincidence that works. The generated reference publishes exactly the sealed
  surface, which is how the rule stays visible.
- **Instantiation is generative.** Identity comes from the site, not from the arguments. The same functor applied twice
  to equal arguments is two declarations.
- **Expansion is binding, not rewriting.** A functor body is checked once per instance with the parameter naming the
  structure the site passed. No syntax is copied, so spans stay where they were written.
- **A template body reads its own parameters and the file's lexical root, and nothing else.** No site-dependent
  resolution, ever.

## 8. Extension recipes

**Adding a base type.** A new base type is admitted by a registry entry rather than a new induction
([`02-core-calculus.md`](../../rules/language/02-core-calculus.md) §5.8). The price of that cheap admission is a row in
[`03-musical-domains.md`](../../rules/language/03-musical-domains.md) §6 stating what the type means, where the meaning
comes from, and a falsifying example. A row with no falsifier is a type that has not said what it is for, and could have
been a `Nat`.

**Adding a standard-library operation.** Write it in `stdlib/src/` as ordinary Musa, with a comment block above it: the
comment is what the editor shows on hover and what the generated reference publishes, and `scripts/check-docs.sh` fails
if a published name has none. Add the module to `stdlib/src/lib.musa` if it is new. Nothing else is registered anywhere
— a bundled module is compiled from its own `mod` declarations, so a file that is not declared is a fault rather than a
hidden module.

**Adding an analysis kind.** The admission rule is [`07-analysis.md`](../../rules/language/07-analysis.md) §2 and §6,
and it is demanding on purpose: state the abstract domain, the abstraction map α, and what the concretization γ admits.
Without a stated α, "candidate" and "fact" mean nothing.

**Adding an assertion kind.** Don't, without changing [`05-verification.md`](../../rules/language/05-verification.md).
The family is fixed at five, and a style rule is not an assertion kind — it is an argument to `follows`.

**Adding a diagnostic.** Add a fixture to `examples/broken/`. The rendered report is snapshotted whole, at a fixed width
and without colour, so a help line cannot stop matching its message unnoticed. A diagnostic about a *different* document
— one raised while reading an adapter module the composer imported — is a `Cause` on the diagnostic about the import,
not a longer message: its labels are spans in that document, it carries no fix, and all three renderers already know how
to show one.

**Adding a surface construct.** Decide its elaboration in `../../rules/kernel/06-surface-elaboration.md` before writing
the parser, and add a positive fixture to `examples/`. If it cannot be elaborated from the three kernel combinators,
that is the finding — report it, do not extend the kernel.

## 9. What to run

```sh
cargo nextest run --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
./scripts/check-docs.sh
```

The workspace lints are strict deliberately: fix the code rather than allow-listing the lint. Slow tests carry
`#[ignore]` and say so in their names, and marking one requires a doc comment arguing what it protects and what still
covers that contract in the fast suite.
