---
id: 17
slug: notation-details
status: pending
depends_on: [09, 16]
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
  prompt carries the **symbols**; interpretation arrives in prompt 18.
- All of prompts 02–09's surfaces.

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
  cheap; otherwise defer entry to a later prompt and note it. Editing commands from prompt 16 must not break on
  annotated events.
- Fixture: add tuplets/ties/slurs/dynamics to `examples/` per §17.6.

## Target

- `musa-language`: new syntax + CST + typed wrappers + formatter rules.
- `musa-compiler`: annotation model, tie merging, tuplet elaboration, diagnostics.
- `musa-render`: plan + MEI + LilyPond support.
- Tests: snapshots at every layer for the new fixture; proptest: tuplet elaboration sums to the notated span (`3:2` of
  eighths spans one quarter); tie merging preserves total span and provenance.

## Check

```sh
cargo nextest run -p musa-language -p musa-compiler -p musa-render
cargo clippy --all-targets -p musa-language -p musa-compiler -p musa-render -- -D warnings
cargo fmt --check
cargo run -p musa-cli -- render examples/tuplet-fixture.musa --to mei -o /tmp/t.mei
cargo run -p musa-cli -- render examples/tuplet-fixture.musa --to lilypond -o /tmp/t.ly
```

Commit as `Add ties, slurs, dynamics, articulations, and tuplets`.

## Stop

- No interpretation of dynamics/articulations (prompt 18) — WAV output must remain byte-identical for pieces without the
  new constructs.
- No hairpins/crescendi (later; add to the annotation model only if trivially cheap).
- No beaming customization beyond meter defaults.
