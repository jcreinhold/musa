---
id: 141k
slug: notation-lowering
status: pending
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
  `use name(args);` is the same rule, not a second invocation mechanism." Then §2's `in scale` pair, §3's higher-order
  programs, and §5's chords, rows, and explicit register.
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

**A statement contributes one call, and which call is the only thing being decided.** A note or a chord is `play` at its
origin and scope; a rest is `sounded` with `Fact.Rest`; every annotation — mark, slur, phrase, tuplet, dynamic, hairpin,
section, harmony, ending, repeat, mobile, improvise, grace — is `sounded` with its own `Fact` case; a transformation
block is the matching track builtin applied to the fold of its body. Nothing else is a statement. Writing the table out
in one place, as a function from `SyntaxKind` to the call it makes, is what keeps this readable at nineteen entries;
spreading it across nineteen methods would hide the very correspondence 141j's mirroring law exists to protect.

**Pitches resolve here, before any track exists.** `01-surface.md` §2 is explicit — "the two uses differ under `≈facts`,
because `in scale` is resolved where the pitches are resolved; saving `subject` does not freeze the scale" — and
`00-semantics.md` §3 repeats it. So `in scale` is a *lexical reading context* in this module, not a value and not a
fact: it emits no key signature, it is not a claim of modulation, and a `step` written with no scale in scope is a
diagnostic here rather than an implicit C major. The consequence to state plainly: a pitch is a literal in the raw term,
so the core never sees a scale, and `PitchTerm`'s deferred resolution has no successor. Whether that is a loss is the
one question this prompt must answer with evidence rather than assertion — if a program in `examples/` or `stdlib/`
needs a pitch whose scale arrives later, this design is wrong and the prompt is repaired before it is implemented.

**The reading context is a reader, not mutable state.** Scope (piece, part, voice), the lexical scale, and the origin
path travel *down* into a block and never back up. A statement that would change what follows it — a key, meter, tempo,
or clef change, a part or voice declaration, an import — is refused inside a `music` value with §3's sentence, because
"from here onward" has no unique meaning in a value usable at several places. The same statements are perfectly legal in
a *score*, and the score is prompt 142's; refusing them here is not refusing them.

**The block spellings and the function spellings are one call.** `transpose up P5 { … }` lowers to
`transpose(P5, <fold of the body>)`, which is exactly what `transpose(P5, e)` lowers to. State it as a law over a pair
of programs rather than as a comment: two source texts, one raw term up to the origins.

**A motif and a fragment are the declarations §2 says they are.** `motif` becomes a `fn` returning a track, `fragment`
becomes a `let` bound to one, and both keep their role — this module's `Sites` already carries what a role needs, and
the role is what lints, extraction, editing, and Origin read. A motif with parameters is an ordinary function with
ordinary parameters; the notation in its body is the fold above and nothing about it is special.

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
