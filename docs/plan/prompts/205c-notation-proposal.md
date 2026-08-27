---
id: 205c
slug: notation-proposal
status: done
depends_on: [205b]
phase: 2
---

# Compose Checked Notation Proposals with Source Previews

## Task

Compose the completed pitches, voices, and written ends into an immutable `NotationProposal` with an exact canonical
Musa source preview that parses and compiles under the destination project context. The proposal is a ranked
explanation, not an answer: it names its derivation to captured events, declares its losses, and never asks Review to
repair an internally uncheckable source.

This prompt ships the facade and the pitch/voice half of source previewing. The rhythmic half — how a measured duration
becomes a tie, tuplet, dot, grace, or rest in the written source — is prompt 205ca's step, and every duration this
prompt emits is a binary subdivision (a whole, half, quarter, eighth, sixteenth, or thirty-second).

## Read

- Prompt 204b's report facade and prompt 205–205b's completed-note, voice, spelling, and written-end stages.
- Prompt 200's report/survivor conventions in `crates/musa-project/src/barlines.rs` (immutable revision-scoped reports,
  source previews) and the compiler facade `compile`/`format_document` in `musa-compiler`.
- Prompt 203's bounds and thresholds in `docs/notes/research/90-midi-transcription-trial.md`.
- Open Music Theory `022`–`031` (what a scored voice must be able to say).

## Design

- The proposal is immutable and carries: rhythm candidate identity, exact score facts per voice (pitch, written end,
  rest/tie/tuplet), chord/rest structure, local alternatives and constraints, a complete derivation to captured event
  ids, declared losses, the take/revision/policy identities, and a canonical source preview.

- The source preview is built with the existing formatter and **parsed and compiled under the destination project
  context**. An uncheckable proposal is an internal error, never something the musician is handed to repair.

- Destage the `#[cfg(test)]` gates on `transcription_pairing`, `transcription_voice`, `transcription_spell`, and the
  `spell_alternatives` helper: the proposal composes them, so they now have a production caller.

- Preserve prompt 203's bounds: at most five proposals, 96 retained states per layer, 128 completed notes, four voices,
  128 KiB candidate/back-pointer storage, and the 50 ms reference-host target for a 128-note phrase.

- Corpus admission through the composed facade: 54/58 top-one voice-label matches after label permutation and 166/166
  grouping-pair matches, with key-release duration accuracy at least 47/58 and pedal-extended sound reported as a fact,
  never scored as a written end. The `unmeasured` fixture is a typed `WriteSource` refusal through the facade, so its
  five notes leave the 63-voice/176-pair denominators (the stage-local 205a numbers). The four crossing corrections stay
  recoverable in top five or by one local constraint.

## Target

- One narrow project proposal/report operation returning the immutable `NotationProposal` (fallible: a `ProposalError`
  when the generated source is uncheckable).
- Exact source previews and derivations for monophonic melody, block/rolled chords (written as voices), two-hand
  texture, crossing voices, repeated notes, and chromatic/atonal spelling — all with binary note durations.
- Corpus thresholds plus deterministic and bounded adversarial tests through the facade.

## Check

```sh
cargo nextest run -p musa-syntax -p musa-compiler -p musa-project
cargo clippy --all-targets -p musa-syntax -p musa-compiler -p musa-project -- -D warnings
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
cargo insta test --workspace --unreferenced=reject
cargo bench -p musa-project --bench transcription_trial
```

Commit as `Compose checked notation proposals with source previews`.

## Stop

- No rhythmic written-end spelling (tuplets, ties, dots, grace, rests) — those are prompt 205ca's step.
- No Review UI, acceptance/source mutation, batch editing, or step-entry deletion (207–208).
- No mutable proposal AST; no unbounded voice count; no learned model or frontend musical inference.
