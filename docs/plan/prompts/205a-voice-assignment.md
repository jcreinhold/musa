---
id: 205a
slug: voice-assignment
status: done
depends_on: [204b, 205]
phase: 2
---

# Assign Voices Jointly with Rhythm

## Task

Extend the shared candidate DAG from 204b so every retained rhythm candidate also carries a complete voice assignment,
at most four voices, decided jointly with rhythm rather than after it. Voice labels are a ranked proposal, not an
authorial claim: crossings and leaps are finite penalties, and an explicit voice-count or crossing constraint outranks
any inferred preference.

## Read

- Prompt 203's measured voice/group results in `docs/notes/research/90-midi-transcription-trial.md` (59/63 label
  matches, 176/176 grouping pairs, the crossing texture's four corrections).
- Prompt 204b's search DAG and candidate/cost record in `crates/musa-project/src/transcription_search.rs` and
  `rhythm.rs`.
- Prompt 205's completed-note/group structure.
- Open Music Theory `022`–`031` (voice leading as one named practice), and Madsen–Widmer / Nakamura et al.'s
  pitch-proximity, continuity, and crossing costs as candidate costs, not laws.

## Design

- Extend the 204b DAG node with a voice assignment carried per completed note (the same arena, no reparse, no second
  lattice). Keeping rhythm and voice in one DAG leaves the 96-state/128-note/128 KiB bounds as-is.

- Costs may use pitch proximity, temporal continuity, register, overlap, repeated patterns, and crossings. A crossing is
  a penalty, never invalidity. New-voice cost is finite. The selected destination voice count and an explicit
  one-/two-/N-voice constraint outrank inferred preference.

- A new voice, hand split, or crossing the search picks must surface in Review, not silently become staff structure.

- Corpus admission: at least 59/63 top-one voice-label matches after label permutation and 176/176 grouping-pair
  matches. The four crossing corrections are not waived: the intended crossing occurs in top five or is recoverable by
  one local crossing/voice constraint that leaves unaffected candidate bytes identical.

## Target

- Private voice/rhythm joint completion over prompt 204a/204b candidates and prompt 205's completed notes.
- Grouping and voice alternative constraints with locality laws: constraining one marked ambiguity leaves unaffected
  candidate regions byte-identical.
- Corpus thresholds above, plus deterministic and bounded adversarial tests (four-voice ceiling, crossing recovery).

## Check

```sh
cargo nextest run -p musa-project
cargo clippy --all-targets -p musa-project -- -D warnings
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
cargo insta test --workspace --unreferenced=reject
cargo bench -p musa-project --bench transcription_trial
```

Commit as `Assign voices jointly with rhythm`.

## Stop

- No pitch spelling, written-end inference, or NotationProposal (205b–205c).
- No hard no-crossing / piano-hand / equal-temperament rule; no unbounded voices; no learned model.
- No source text or Review UI.
