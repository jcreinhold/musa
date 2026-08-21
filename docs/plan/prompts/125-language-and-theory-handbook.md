---
id: 125
slug: language-and-theory-handbook
status: done
depends_on: [104, 106, 115, 118, 119, 121, 124]
phase: 3
---

# Score Language and Theory Handbook

## Task

Turn the implemented score language and bundled theory library into one tested reference with two reading paths:
musicians can learn by musical task and language developers can recover the grammar, typing, elaboration, laws,
ownership, and performance model precisely. Replace the proposal's provisional examples with compiling Musa source and
make every public standard-library operation discoverable from source, editor hover, and the handbook without
duplicating its definition. This is not the final whole-language handbook: prompt 190 adds performance, instruments,
studio, assets, packages, samples, and clips before prompt 193 graduates the complete specification.

## Read

- `docs/rules/language/`, `docs/rules/events/`, the roadmap language sections, and the style guide.
- Prompts 92–124 and every bundled `.musa` source file introduced by them.
- The relevant Open Music Theory chapters under `~/Code/papers/music-theory/open-music-theory/` cited by prompts
  100–119. In particular, use `013-major-scales-scale-degrees-and-key-signatures.md`, `016-intervals.md`,
  `017-triads.md`, `018-seventh-chords.md`, `020-roman-numerals.md`, `022-chords-in-satb-style.md`, the `023`–`030`
  species/counterpoint chapters, `033`–`035` on schemas, `072-neo-riemannian-triadic-progressions.md`, and the
  `099`–`110` pitch-class/set/serial chapters. Cite the exact file used rather than only its number.

## Design

The handbook has a musician-facing task path and an implementor-facing reference path, joined by stable anchors:

- start the musician path with notes, bars, voices, reusable phrases, key/scale degrees, chord symbols and voicings;
  introduce functions, templates, modules, assertions, transformations, analysis, and event track escape hatches only
  when a musical task needs them;
- state where Musa deliberately preserves distinctions that informal practice may blur: written pitch versus sounding
  pitch, pitch class versus `pc12`, key versus scale, chord symbol/class versus voicing, construction versus analysis,
  rule violation versus stylistic evidence, and source declaration versus generated occurrence;
- document major/minor, modal, chromatic, jazz-symbol, post-tonal, serial, schema, voice-leading, and counterpoint
  facilities at the generality actually implemented. Do not present one repertoire's conventions as universal;
- give every nontrivial theoretical claim an adjacent OMT citation when OMT supports it. For algebraic or PL claims
  absent from OMT, cite the numbered definition/law/proof in `docs/rules/language/`; do not launder an implementation
  choice into “music theory says”;
- make the developer path specify surface grammar, typing judgments, context requirements, desugaring, evaluation,
  normalization, resource diagnostics, provenance, caching invariants, module ownership, public/private boundaries, and
  extension recipes. Include a worked trace from source through contextual `Music`, event-track term, occurrences, and a
  rendered result;
- generate the standard-library API index and editor documentation from authoritative declarations/doc comments.
  Handwritten prose may teach and cross-link but must not repeat signatures or parameter defaults.

Every fenced Musa example is an executable fixture or extracted from one. Add negative examples for common category
errors and snapshot their plain-language diagnostic and repair. Test all internal anchors and local citations.

## Target

- A musician tutorial and task cookbook under `docs/rules/language/`, with compact complete pieces for tonal, modal,
  post-tonal/serial, contrapuntal, and generative-template use.
- An implementor reference covering syntax-to-event-track elaboration, laws, extension points, and ownership boundaries.
- Generated standard-library API pages and `scripts/check-docs.sh`, which proves them synchronized with bundled source
  and validates the cited local chapters and internal links.
- An explicit citation map from each implemented music-theory domain to the relevant local OMT chapter or Musa proof.
- Compiling positive examples and diagnostic goldens for negative examples in `examples/` or focused doc-test fixtures.
- Reconciliation of the design essay: mark resolved choices as implemented, link to governing candidate
  `docs/rules/language/`, and retain rejected alternatives and rationale as design history. Do not call the candidate
  governing before prompt 193.

## Check

```sh
cargo nextest run -p musa-syntax -p musa-compiler -p musa-project -p musa-lsp
cargo clippy --all-targets -p musa-syntax -p musa-compiler -p musa-project -p musa-lsp -- -D warnings
cargo fmt --check
./scripts/check-docs.sh
find examples -name '*.musa' -print0 | xargs -0 -n1 cargo run -q -p musa -- check
```

Also run the repository's Markdown link checker over `docs/rules/language/` and record a fresh-reader pass by one route
each: write a harmonized phrase from the musician tutorial, and add a small total standard-library function from the
implementor guide. Commit as `Publish the Musa language and theory handbook`.

## Stop

- No documentation-only aliases, convenience syntax, or examples that the shipped parser/compiler does not accept.
- No exhaustive music-theory textbook and no claim that the bundled conventions cover every musical culture or style.
- No copied OMT chapter text; cite and explain only the concepts Musa actually implements.
- No separate hand-maintained LSP documentation table or public exposure of compiler pass types.
- No placeholder prose for audio features that prompts 177–189 have not implemented; leave stable anchors for the later
  generated sound-language reference instead.
