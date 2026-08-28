---
id: 208
slug: accept-captured-phrase
status: pending
depends_on: [202, 207]
phase: 2
---

# Keep a Played Phrase as One Source Edit

## Task

Close Capture and Keep that end to end. Place the reviewed notation proposal into the chosen part/voice as one
provenance-preserving source transaction, handle new voices and stale revisions explicitly, autosave it normally, and
make one undo restore the exact pre-capture project.

## Read

- Prompts 19, 25, 33, 43, 67, 84a, 182–183, 201–207; current project revision/undo/autosave, semantic edit planning,
  source formatter, stale-valid snapshot, and desktop recovery paths.
- Governing source authority, exact identity, provenance, and desktop stale-state rules. A take/proposal is derived
  temporary evidence; acceptance is the only point at which it may affect canonical identity.

## Design

Capture begins with a revision and destination anchor. Acceptance preflights against the current source. If unrelated
edits leave the anchor and required context exactly valid, rebase through recorded stable identities and show the new
candidate source; otherwise return **Piece changed—review this phrase against the current score** and preserve the take.
Never apply byte offsets from the capture revision to a later document.

Insert the formatter-produced proposal as the smallest structural source edit at the caret/range chosen in Review. A
monophonic proposal enters the selected voice. A proposal requiring additional voices must have an explicit reviewed
voice decision; acceptance may add ordinary source voice blocks through a syntax-owned edit plan, with names chosen
inline and collision-checked. It never silently replaces existing music, changes a meter/key/tempo, or manufactures a
part. Inserting over a selected range is a separate replace operation and must say exactly what will leave.

The preflight transaction carries proposal/take/policy identities and the complete derivation needed to validate and
explain the pending edit. Acceptance deliberately ends that special provenance: committed notes originate at their new
source spans exactly as typed notes do. Raw timestamps, velocities, calibration, and review decisions enter neither
score identity nor hidden project metadata and are released after acceptance. Preserving a performance take would
require an explicit source/archive feature in a later prompt; project undo stores source revisions, not MIDI blobs.

Accept compiles before commit, writes one project revision, triggers ordinary autosave/recovery, installs the new valid
score/audio plan, restores selection to the inserted phrase, and reports one musical sentence. One undo restores source,
score, audio, selection vicinity, and saved/dirty state. Failure leaves source, undo, autosave, and installed plan
unchanged and keeps Review open.

Keep that finds a suffix only from explicit recent-buffer facts: latest silence boundary under the measured policy,
current capture start marker, or a musician-adjusted boundary while auditioning. It never guesses by deleting opening
notes. Empty/noisy/overflowed history gives a direct remedy. Capture and Keep that converge on the identical take →
transcribe → review → accept path.

## Target

- One project accept-plan/apply operation with revision-safe rebase, syntax-owned structural insertion/replacement,
  voice addition, provenance, compile-before-commit, autosave, selection restoration, and exact undo.
- Complete desktop flows for explicit Capture and Keep that, including count-in, boundary adjustment, Review handoff,
  Accept/Discard, stale source, invalid source, new/empty piece, device loss, overflow, failure, and recovery.
- Differential law that equal reviewed proposals accepted at equal anchors produce byte-identical formatted source and
  equal score semantics, regardless of Capture versus Keep that origin.
- End-to-end tests proving source immutability before Accept and exact source/score/audio/undo behavior after it.

## Check

```sh
cargo nextest run -p musa-syntax -p musa-project -p musa-desktop
cargo clippy --all-targets -p musa-syntax -p musa-project -p musa-desktop -- -D warnings
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
cargo insta test --workspace --unreferenced=reject
cd apps/musa-desktop/ui && npx pnpm run check && npx pnpm run test:unit
cd apps/musa-desktop/ui && npx playwright test --project=screens
```

Commit as `Accept captured phrases as canonical Musa source`.

## Stop

- No persistence of rejected/raw takes, take browser, comping, overdub lane, waveform, audio recording, or DAW timeline.
- No silent range replacement, voice/part creation, context change, stale-offset rebase, or partial transaction.
- No step-entry deletion until prompt 209's closure proves the replacement and removes it cleanly.
