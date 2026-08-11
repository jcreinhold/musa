---
id: 119
slug: voice-leading-and-counterpoint
status: done
depends_on: [102, 116, 117]
phase: 3
---

# Explicit Voice-Leading and Counterpoint Profiles

## Task

Add opt-in analysis/assertion profiles for SATB chord writing, species counterpoint, and jazz voicing motion. Each
profile states its style, voice/rhythm assumptions, and whether a rule is definitional, hard within that exercise, or a
guideline. Findings cite exact voices/times/intervals and never present one historical pedagogy as a universal law of
music.

## Read

- OMT `022-chords-in-satb-style.md`; `023-introduction-to-species-counterpoint.md`; `024`–`028` for first through fifth
  species; `030-16th-century-contrapuntal-style.md`; `039-embellishing-tones.md`; and `076-jazz-voicings.md`, especially
  §“Guidelines versus Rules”.
- Prompts 118–117: explicit assertion versus requested analysis and the evidence model.
- Current voice identity, ties, grace/point ordering, context tracks, and folded notation/performance positions.

## Design

Profiles are named data with typed parameters, not compiler modes hidden in configuration:

- `satb_common_practice` checks declared ranges, spacing, crossing/overlap, doubling, parallels/direct perfects, and
  tendency-tone/resolution claims only where OMT defines them and the request supplies harmonic context.
- `species_1` through `species_5` require a designated cantus/counterpoint and rhythmic relation; consonance,
  dissonance, preparation/resolution, motion, beginning/ending, and melodic-shape checks follow their cited chapters.
- `jazz_voice_leading` reports guide-tone retention, common-tone/small-motion evidence, omissions, spacing, and the 3–7
  paradigms as guidelines unless the request upgrades a named condition to an assertion.

Normalize voices to exact notated spans with ties before comparing simultaneities; never use MIDI frames or performed
groove time for notation/counterpoint claims. A perfect fourth's consonance depends on bass/context as OMT 023 notes;
the interval table must not encode it as globally consonant or dissonant. Fifth species combines previously defined
behaviors rather than gaining a vague catch-all branch.

Every rule has a stable id, source citation/local derivation, minimal passing/failing fixture, and a false-positive
counterexample. The assertion form preserves music on success; analysis findings remain separate reports.

An assertion's passage is the music inside its own braces and nothing else, which is what keeps the claim local: a
composer reading `assert follows(satb_parallel_perfects) { … }` can see everything it is about. So the assertable rules
— the ones decidable from a passage's own sonorities — have content only where the passage is chordal. Four voices
written as four voices are four passages, and a `follows` claim around one of them is vacuously true. The four-voice
fixtures are therefore read by `musa analyze`, and the assertion is demonstrated where it can fail.

## Target

- Analysis profiles/findings and assertion adapters through prompt 117/109 boundaries.
- `examples/analysis/{satb,species-1,species-2,species-3,species-4,species-5,jazz-voice-leading}.musa` with paired
  passing/failing regions, each file naming in its header comment the profile it is meant to be read under. The profile
  is named by the request and never by the source: a profile is the analyst's question, and a document that answered it
  in advance would be asserting a style it merely happens to be written in. `profile` is also already the performance
  keyword, so a second sense of the word in the surface language would be a collision as well as a category error.
- `examples/analysis/asserted-voicings.musa`: the assertable rules asserted on chordal passages, one passing and one
  failing per rule, with the header stating why an assertion around a single line would have claimed nothing.
- `crates/musa-compiler/tests/voice_leading_validation.rs`: one invariant, boundary, and counterexample per rule id;
  tie/span normalization; fourth-above-bass context; analysis/assertion separation.
- `docs/language/07-analysis.md`: rule table with OMT file/section, strength, assumptions, and known limits.

## Check

```sh
cargo nextest run -p musa-compiler -p musa-project -p musa
cargo clippy --all-targets -p musa-compiler -p musa-project -p musa -- -D warnings
cargo fmt --check
cargo run -p musa -- analyze examples/analysis/species-4.musa --kind counterpoint --profile species_4 \
    --cantus cantus --format text
cargo run -p musa -- analyze examples/analysis/jazz-voice-leading.musa --kind voice-leading \
    --profile jazz_voice_leading --format text
cargo bench -p musa-compiler
```

Commit as `Add explicit voice-leading analysis profiles`.

## Stop

- No universal “good voice leading” score, style inference, automatic correction, generated counterpoint, or voicing.
- No performed-time/groove judgment for a notated counterpoint rule.
- No hard error for a guideline unless the source explicitly asserts it.
- Do not add rules without a citation or local definition/proof and a false-positive counterexample.
