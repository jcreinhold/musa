---
id: 141k
slug: notation-lowering
status: done
depends_on: [141g, 141h, 141i, 141j]
phase: 3
---

# Read a Notated Block as a Track Term

## Task

Prompt 141g built the reading half of the surface: `Lowering::item` answers for a `data`, an `enum`, a `record`, a
`trait`, an `impl`, a `fn`, and a `let`, and for everything else it answers `None` because "a `piece`, a `library`, a
`structure`, and an `import` are containers and statements that a pass reads for itself". The pass that read them for
itself is the one prompt 142 deletes. Nothing lowers a note, a rest, a chord, a `use`, an `in scale`, a transformation
block, or the `music { … }` expression — `MusicExpr` is refused at the node with prompt 142 named as its owner. Give the
notation surface its reading: **a notated block is a track term**, built by a left fold out of the words prompt 141j
registered.

## Read

- [`../../rules/language/01-surface.md`](../../rules/language/01-surface.md) §2 in full — it is the specification. Three
  sentences fix the whole desugaring: `motif turn(...) { body }` is "a named `fn turn(...) -> EventTrack[WrittenTime,
  ScoreFact] { music { body } }` with a `Motif` role retained for lints, extraction, editing, and Origin";
  `fragment name { body }` is "`let name = music { body };` with a `Fragment` role"; and `use e;` "checks that `e` is a
  written-time score track, `follow`s it onto the voice at the current cursor, and advances by `duration(e)`. Existing
  `use name(args);` is the same rule, not a second invocation mechanism." Then §2's `in scale` sentences — the heading
  claim and the closing line, **not** the `figure()`/`subject` example between them, which the Design below withdraws —
  §3's higher-order programs, and §5's chords, rows, and explicit register.
- [`../../rules/language/00-semantics.md`](../../rules/language/00-semantics.md) §3 — the two composition equations, and
  the list of what a block may not contain. That list is this prompt's refusals, and each one is refused *because* the
  value is usable at several places, which is a sentence a diagnostic can say.
- [`141j`](141j-notation-vocabulary.md) — `Fact`, `sounded`, `follow`, `nothing`, and the argument that none of the four
  is a source word. This prompt is their only writer.
- [`141g`](141g-raw-lowering.md) — `Sites`, `Lowering`, the one-direction rule, and the module doc's boundary: reading,
  desugaring, and numbering, and never checking. A notation statement is desugaring, which is why it belongs in this
  module and not in a pass.
- [`141h`](141h-track-core.md) — the eight track builtins and their signatures, which are what a transformation block
  lowers to. `00-semantics.md` §3 says the function and block spellings "invoke the same semantic action, so their
  equality is an implementation theorem rather than a duplicated convention"; here that theorem is one law, because both
  spellings produce the same call.
- `crates/musa-compiler/src/core.rs` — the notation arms this replaces, and in particular `PitchTerm` and its three
  errors. A written pitch is resolved "as far as a scale-free evaluator can", and `in scale` supplies what is missing;
  read why that staging exists before deciding where scale resolution goes here.
- `crates/musa-compiler/src/scale.rs`, `chord.rs`, `pitch.rs`, `harmony.rs` — the theory algorithms a statement's
  payload is read *by*. None of them moves: this prompt calls them where the old checker called them, and their answers
  become literals in a raw term.
- [`../../rules/language/03-musical-domains.md`](../../rules/language/03-musical-domains.md) — the domains a statement's
  payload lands in, and the layer separation the roadmap §2 table states: written pitch is not a MIDI number and a
  notated duration is not a performed one.
- `crates/musa-compiler/src/lower/laws.rs` — the posture these laws take: source in, raw term out, and where a program
  is refused, refused at the node that caused it.

## Design

**A block is a left fold, and the fold is the whole design.** A notated block reads as
`follow(follow(follow(nothing, s₁), s₂), …)` over its statements in source order, where each statement contributes the
track it denotes. Placement never appears: `00-semantics.md` §3 deletes the cursor, and a fold has no cursor to delete.
`use e;` contributes `e` itself, which is what makes "checks that `e` is a written-time score track" a *typing*
statement rather than a step — the core checks it against `EventTrack ⟨written⟩` because that is what `follow` demands,
and the diagnostic arrives at the node this module numbered.

*Decided during implementation: a block answers its own questions.* 141j registered `sounded` and `play` as fallible —
they answer `Result<EventTrack ⟨written⟩, Text>`, because a fact whose payload does not typecheck has no track — so the
fold is written in the surface's own `?` and drained at the closing brace. A block therefore denotes
`Result<EventTrack ⟨written⟩, Text>`, and the `Result.Ok` is unconditional: an empty block wraps `nothing` in `Ok` too,
so a block's *type* does not depend on which statements a composer happened to write inside it. Draining at the brace
rather than at the enclosing function is what makes `music { … }` a complete expression instead of one that only
typechecks in a function that answers a `Result`. The consequence for §2 is recorded under the motif paragraph below.

**A statement contributes one call, and which call is the only thing being decided.** A note or a chord is `play` at its
origin and scope; a rest is `sounded` with `Fact.Rest`; every annotation — mark, slur, phrase, tuplet, dynamic, hairpin,
section, harmony, ending, repeat, mobile, improvise, grace — is `sounded` with its own `Fact` case; a transformation
block is the matching track builtin applied to the fold of its body. Nothing else is a statement. Writing the table out
in one place, as a function from `SyntaxKind` to the call it makes, is what keeps this readable at nineteen entries;
spreading it across nineteen methods would hide the very correspondence 141j's mirroring law exists to protect.

*Decided during implementation: `stack` is the one statement that reaches `play`.* This paragraph reads "a note or a
chord is `play`", and only half of that survived contact with 141j's signature: `play` takes a `Voicing`, and
`Voicing::new` takes a `ChordClass` alongside its pitches. `stack c4 major/2` names a class and is one `play`. A
bracketed simultaneity does not — `[c4 c#4]` is a spelling no chord class covers, and inventing one to fit the
constructor would be the compiler asserting a harmonic reading the composer declined to write. So `[c4 e4 g4]` is one
`sounded(Fact.Note)` per pitch folded with `together`, which says exactly what was written: three notes, at once, named
by nothing. `03-musical-domains.md`'s separation is the same point — a simultaneity is a *notation*, a voicing is a
*theory reading of one* — and the fold is where a notation stops.

**Pitches resolve here, before any track exists.** `01-surface.md` §2 states it twice — "a track value is an ordinary
value, and `in scale` is lexical rather than captured", and "an absent scale makes `step` a type-context diagnostic, not
an implicit C-major choice" — and `00-semantics.md` §3 says the same from the other side: a block may contain "lexically
scoped `in scale`", which "is resolved while pitches are resolved — before any track value exists". So `in scale` is a
*lexical reading context* in this module, not a value and not a fact: it emits no key signature, it is not a claim of
modulation, and a `step` written with no scale in scope is a diagnostic here rather than an implicit C major. The
consequence to state plainly: a pitch is a literal in the raw term, so the core never sees a scale, and `PitchTerm`'s
deferred resolution has no successor.

*Answered during preparation, and one citation withdrawn.* This paragraph used to quote §2's "the two uses differ under
`≈facts` … saving `subject` does not freeze the scale" as its authority. That sentence and the `figure()`/`subject`
example above it say the opposite of the three sentences quoted here, and they contradict §2's own heading and closing
line: the body of `fn figure()` has no lexically enclosing `in scale`, so under "lexical rather than captured" its
`step 1` is the diagnostic §2's last sentence describes, and the example cannot compile to have two readings. Only
*dynamic* capture makes it differ at two use sites, which is the deleted contextual-`Music` behaviour. The example is a
survival, not a decision, and repairing it belongs to [`149`](149-language-pass-closure.md), whose Task is the
contradiction audit and which "may repair `docs/rules/language/` as its own candidate specification". Recorded there so
the audit does not have to rediscover it.

The evidence question this paragraph used to leave open — whether any program needs a pitch whose scale arrives later —
is answered no, by reading every program that could:

- `examples/scale-context.musa` is the only one, and it names the deleted design as its reason in its own header
  comment: "Because a `music` value is contextual, the same saved phrase elaborates differently at two use sites." It is
  written in `Music`, the type name prompt 127a deleted (`01-surface.md` §2), and its final voice lets a `key` statement
  supply a collection to a scale-less `step`, which is the implicit choice §2's last sentence refuses. It is a migration
  item for prompt 142, not a requirement on this one.
- `stdlib/src/tonal/schemas.musa` already writes the idiom this design prescribes: a collection is an explicit argument
  (`fn sixth_over(collection: Scale, bass: Degree)`), never ambient. The library never needed the capture.
- `examples/broken/no-scale-in-force.musa` already asserts the refusal — "an absent scale is never an implicit C major"
  — so the negative fixture encodes the new rule before this prompt implements it.
- `examples/module-functor-study.musa` and `examples/template-study.musa` open `in scale` and contain no `step`, so
  neither constrains where resolution happens.

A saved fragment that wants a scale is written `fn figure(under: Scale) -> EventTrack[WrittenTime, ScoreFact]`, which
makes the dependency an argument rather than ambient context — the direction §3's deletion of the cursor and the
contextual stage was already moving.

**The reading context is a reader, not mutable state.** Scope (piece, part, voice), the lexical scale, and the origin
path travel *down* into a block and never back up. A statement that would change what follows it — a key, meter, tempo,
or clef change, a part or voice declaration, an import — is refused inside a `music` value with §3's sentence, because
"from here onward" has no unique meaning in a value usable at several places. The same statements are perfectly legal in
a *score*, and the score is prompt 142's; refusing them here is not refusing them.

**The block spellings and the function spellings are one call.** `transpose up P5 { … }` lowers to
`transpose(P5, <fold of the body>)`, which is exactly what `transpose(P5, e)` lowers to. State it as a law over a pair
of programs rather than as a comment: two source texts, one raw term up to the origins.

*Decided during implementation: the law compares a block with its body, not with a whole program.* The whole-program
pair this paragraph imagines — `Ok(transpose(M3, music { c4/4 }?)?)` beside `music { transpose up M3 { c4/4 } }` —
cannot be written yet. `e?` elaborates to a `match` on `e`, a `match` in scrutinee position has no inferable type, and a
block that drained a question is already a `match`, so `music { … }?` is refused by the core. That is the `?`
desugaring's limitation and prompt 142 owns the surface; the half this module decides — that the transformation's track
argument is the body's own fold and nothing else — is stated over four pairs, and the other half is
`Lowering::application`'s one line, read where `values.rs` is read.

**A motif and a fragment are the declarations §2 says they are.** `motif` becomes a `fn` returning a track, `fragment`
becomes a `let` bound to one, and both keep their role — this module's `Sites` already carries what a role needs, and
the role is what lints, extraction, editing, and Origin read. A motif with parameters is an ordinary function with
ordinary parameters; the notation in its body is the fold above and nothing about it is special.

*Decided during implementation, three ways.* **No written return type.** §2 spells the desugaring
`fn turn(...) -> EventTrack[WrittenTime, ScoreFact] { music { body } }`, and that annotation is stale relative to 141j:
a block answers a `Result`, so writing §2's type would be writing a false one. The desugaring omits the annotation
rather than inventing `Result<EventTrack[WrittenTime, ScoreFact], Text>` on §2's behalf — the value states its own type,
and reconciling §2's sentence with the fallible vocabulary is prompt 142's, which owns both the surface and the
migration. **Parameters come from the typed AST.** The grammar gives a motif parameter no `Param` node — it writes a
bare identifier beside a type name — so `written_parameters`, which finds `Param` nodes, finds none and a parameterized
motif would lower to its body with nothing bound. `ast::MotifDecl::params()` is what the parser already computed, and
the binders are numbered at the declaration because that is the node the names were written on. **The declaration
identity is the enclosing one.** A fact carries the declaration it belongs to, and until 142's piece walk supplies one,
every fact this module builds says `DeclarationId(0)` — the enclosing declaration — rather than a fresh number per
statement, because a number invented here would be a second identity scheme for 142 to reconcile with the real one.

**Laws, not a caller.** `mod lower` stays behind its dead-code expectation and prompt 142 is still the first caller,
exactly as 141g, 141ga, 141h, 141ha, and 141i leave it. The laws are the caller, and they are stated as source text in,
raw term out, so a reader can check the desugaring by reading it.

## Target

- `crates/musa-compiler/src/lower/notation.rs`: the fold, the statement table, the reading context, and the pitch and
  chord reading, with the module doc stating the fold and the refusal list.
- `Lowering::item` answering for `MotifDecl` and `FragmentDecl` with the desugaring §2 fixes, and `Lowering::expr`
  answering for `MusicExpr` — the `not_yet` refusal for it deleted.
- The refusals `00-semantics.md` §3 asks for, each naming the statement and the sentence, under the diagnostic code that
  says *misplaced* rather than *unsupported*: they are permanent answers.
- Laws in `crates/musa-compiler/src/lower/laws.rs` (or a `lower/notation/laws.rs` beside the module, if the count says
  so): the empty block is `nothing`; two statements are one `follow`; `use e;` is `e`; a rest is `sounded` at
  `Fact.Rest`; a chord is one `play`; the block and function spellings of transpose, stretch, retrograde, and inversion
  agree; `in scale` changes the pitch a `step` reads and emits no fact; a key change inside a `music` value is refused
  at its own node; a motif is the function §2 says it is and a fragment the `let`.
- `docs/plan/code-map/spec-to-implementation-map.md`: the lowering row records the notation half and loses `MusicExpr`
  from its refused-at-the-node list.

## Check

```sh
cargo build --workspace
cargo nextest run -p musa-core -p musa-compiler
cargo clippy --all-targets -p musa-core -p musa-compiler -- -D warnings
cargo fmt --check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
python3 scripts/renumber-prompts.py audit
```

Commit as `Read a notated block as a track term`.

## Stop

- No wiring. `check_piece`, `check_arguments`, `check_template_voice`, and `check_material` are untouched, the old
  checker still checks every program, and nothing in `examples/` or `stdlib/` changes. Prompt 142 is the cutover.
- No document structure: `piece`, `score`, `part`, `voice` as a *declaration*, `section`, `bar`, `template`, `make`, and
  the context tracks for tempo, meter, key, and clef are 142's, because they are what a pass builds and this module
  reads what a block says. A voice's *body* is a block and is here; the voice declaration around it is not.
- No readback. A raw term is the deliverable; turning a normal form back into a score is 142's.
- No deletion of the old notation checker. It still runs, and until 142 it is what every fixture is checked by.
- No new registered word. If the fold needs an operation 141j did not register, that is a repair of 141j — stop, repair,
  commit the repair, and resume.
- No surface change and no grammar change. Every form this module reads is already parsed; if one is not, that is a
  finding for 142 rather than a token added here.
