---
id: 27
slug: notation-details
status: done
depends_on: [14, 25]
phase: 2
---

# Notation Details: Ties, Slurs, Dynamics, Articulations, Tuplets

## Task

Extend the language end-to-end — lexer, parser, compiler, notation plan, MEI, and LilyPond — with the expressive
notation layer: explicit ties, slurs, dynamic markings, articulations, and tuplets. Phase 2 begins here; every construct
travels the full pipeline in one prompt so no layer drifts ahead of another.

## Read

- Roadmap §7.1 (annotations in the example), §6.3 (`AnnotationStore`, spans), §12.1 (plan contents: annotations and
  spans), §17.6 (tuplets-and-ties fixture).
- Roadmap §2 separation table: a dynamic marking is not a decibel value; an articulation is not a gate multiplier. This
  prompt carries the **symbols**; interpretation arrives in prompt 28.
- All of prompts 02–14's surfaces.

## Design

- Syntax additions (explicit, semicolon-terminated):

  ```text
  c4 1/4 ~ d4 1/8;            % tie: postfix ~ on the first note... or `tie c4 1/4 d4 1/8` — pick one, document it
  slur { c4 1/4; d4 1/4; e4 1/4; }
  dynamic p;                  % in a voice: applies from this point
  accent c4 1/4;  staccato d4 1/8;   % prefix articulation, or postfix `c4 1/4 .` — pick one consistent form
  tuplet 3:2 { c4 1/8; d4 1/8; e4 1/8; }
  ```

  The roadmap fixes semantics, not punctuation; choose forms that parse unambiguously
  with the existing grammar and record the choice in the language docs. Tuplets must
  elaborate to exact rationals (§7.2: triplet eighth = 1/12 — `tuplet 3:2` of eighths
  gives each note 1/12 exactly).
- Compiler: new `ScoreEventKind` payload fields or `AnnotationStore` entries for slur spans, dynamics, articulations —
  follow §6.3: annotations live in the store with spans and provenance, not bolted onto events. Ties are
  duration-structure, not annotations: a tied pair lowers to **one** sounding event whose notation spells as two
  noteheads (decide representation: single `ScoreEvent` with a two-piece `NotatedDuration` spelling — preferred, matches
  prompt 07's tie decomposition which now gains a second source).
- Notation plan: slur/dynamic/articulation placement data (above/below defaults), tuplet groups with ratio, ties from
  both sources unified in one tie concept.
- Backends: MEI (`<slur>`, `<dynam>`, `<artic>`, `<tuplet>`, tie elements) and LilyPond (`( )`, `\p`, `--`, `->`,
  `\tuplet 3/2`, `~`) in the same prompt.
- GUI: render only — Verovio shows everything for free. Entry shortcuts (tie key, slur range command, dynamic menu) if
  cheap; otherwise defer entry to a later prompt and note it. Editing commands from prompt 25 must not break on
  annotated events.
- Fixture: add tuplets/ties/slurs/dynamics to `examples/` per §17.6.

## Target

- `musa-language`: new syntax + CST + typed wrappers + formatter rules.
- `musa-compiler`: annotation model, tie merging, tuplet elaboration, diagnostics.
- `musa-render`: plan + MEI + LilyPond support.
- Tests: snapshots at every layer for the new fixture; proptest: tuplet elaboration sums to the notated span (`3:2` of
  eighths spans one quarter); tie merging preserves total span and provenance.

## Repairs made while implementing

- **Punctuation, as the Design section asked to be chosen and recorded.** Ties are the postfix `~` on the first
  statement (`g5 1/4 ~;`), not a `tie` prefix: the tie belongs to a notehead, and putting it before two statements would
  have made the statement separator ambiguous. Articulations are postfix bare identifiers (`a5 1/4 accent;`), not
  prefixes: a prefix would have collided with a motif parameter reference, which is also a bare identifier in head
  position. Dynamics are `dynamic <mark>;` and attach to the **next** sounding event, reaching into whatever block
  follows. Tuplets are `tuplet 3/2 { … }` with a `Rational` token rather than `3:2`: the lexer already has the token,
  LilyPond spells it the same way, and `:` would have needed a context-sensitive rule in the formatter for no gain.
- **Articulations get their own CST node.** `root 1/8 tenuto;` has two bare identifiers in one statement, and
  `token_text` reads direct children only. `ArticulationList` keeps the pitch reference and the articulation names apart
  by structure instead of by position.
- **A tie is one event with a multi-piece `NotatedDuration`,** as the Design section preferred. `pieces` holds
  *sounding* values, so the invariant is simply that they sum to `value`; the tuplet annotation supplies the ratio that
  converts back to symbol values in the renderer. This is the same tie concept prompt 07 already used for
  measure-crossing decomposition — that pass now has a second source feeding it.
- **The frozen direct lowerer refuses the new constructs** rather than growing to match them. `lower.rs` says it must
  not grow with new features, and parity with the kernel path is what the differential test protects; a refusal keeps
  both true. `phase_two_constructs_are_kernel_only` in `crates/musa-compiler/tests/elaboration.rs` pins the boundary.
- **Annotations are emitted after tie merging, not during elaboration.** They name `EventId`s, and merging decides how
  many events exist. Marks ride along in the kernel payload and become annotations in `identify`, which also means a
  group's members are always a contiguous id range — that is what lets the notation plan expand a `TupletSpan` by
  iterating the range.
- **Tuplets must fit inside one measure** (diagnostic: `a tuplet must fit inside one measure`). A tuplet split across a
  barline would need the bracket itself decomposed, which neither MEI nor LilyPond expresses cleanly; the restriction is
  checked once on the finished snapshot.
- **The fixture's comments are ASCII, deliberately.** Written with em dashes, it became the first non-ASCII file in
  `examples/`, and `tests/unit/highlighting.test.ts` failed: the Rust lexer reports **byte** offsets, the CM6 tokenizer
  reports JavaScript string indices, and they diverge on any character outside ASCII. That is a real prompt-26 defect in
  the editor's span contract, not a notation one; the fixture stays ASCII so this prompt does not carry it, and the
  defect is recorded for its own repair.
- **A beaming bug the fixture found.** `assign_beams` compared each item's onset (real time) against its duration's
  *symbol* value, which put the third triplet eighth outside its own beam. It now converts the symbol value through the
  tuplet ratio before advancing the running onset.

## Check

```sh
cargo nextest run -p musa-language -p musa-compiler -p musa-render
cargo clippy --all-targets -p musa-language -p musa-compiler -p musa-render -- -D warnings
cargo fmt --check
cargo run -p musa -- render examples/tuplet-fixture.musa --to mei -o /tmp/t.mei
cargo run -p musa -- render examples/tuplet-fixture.musa --to lilypond -o /tmp/t.ly
```

Commit as `Add ties, slurs, dynamics, articulations, and tuplets`.

## Stop

- No interpretation of dynamics/articulations (prompt 28) — WAV output must remain byte-identical for pieces without the
  new constructs.
- No hairpins/crescendi (later; add to the annotation model only if trivially cheap).
- No beaming customization beyond meter defaults.
