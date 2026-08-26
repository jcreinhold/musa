---
id: 207
slug: transcription-review
status: pending
depends_on: [205, 206]
phase: 2
---

# Review the Few Decisions That Matter on the Score

## Task

Build an exceedingly direct transcription Review surface: show the best complete proposal as notation, mark only genuine
ambiguities, let the musician hear raw performance against the proposed reading, and revise a note, group, beat, or
voice with the same musical commands used on an ordinary selection. Source remains unchanged until prompt 208 accepts.

## Read

- Prompts 201, 204–206; desktop visual, engraving, interaction, provenance, state/voice, and frame-budget rules; prompts
  22–26, 52–56, 76, 189–191, and 198.
- Prompt 203's measured ambiguity classes and review-correction counts. Do not invent a control for a failure the corpus
  did not exhibit.
- Current anchored engraving, overlay geometry, selection/focus, project facts/IPC generation, candidate text preview,
  transport, prepared playback, stale revision, and generated-edit choice surfaces.

## Design

Review is a temporary reading of one immutable take and one immutable proposal, not a piano roll, wizard, settings wall,
or second editable AST. The leaf shows the proposed engraving in the normal score visual language; a narrow top-margin
line states destination, take extent, and **Accept / Discard**. Raw take evidence and alternative candidates stay behind
the project facade.

Start with the best candidate and progressive disclosure. Mark a location only when prompt 203's calibrated cost gap or
an explicit unsupported/loss condition requires a choice. Selecting the mark shows two or three musical readings in
place—e.g. straight eighths / triplet, chord / rolled chord, one voice / two, pickup / beat one—with a short explanation
and affected notes. No percentages, “AI confidence,” global expert panel, or unexplained score.

Provide three high-leverage review gestures:

1. **Tap pulse:** while auditioning the raw take, taps create monotone beat anchors; one explicit downbeat gesture fixes
   phase. The project reruns only the dependent timing region and keeps unaffected proposal bytes/ids stable.
2. **Choose an alternative:** select one offered reading; the decision becomes an immutable constraint and yields a new
   proposal.
3. **Transform a selection:** duration numbers, diatonic/accidental movement, and Transpose selection reuse prompt 206's
   intent vocabulary against proposal identities. The project applies constraints/retranscription and returns a new
   immutable proposal; the frontend never mutates candidate notes.

Add contextual operations for **Make chord / Keep rolled**, **Assign voice**, **Split here**, **Join these**, **Tie / Do
not tie**, and **Respell**, but only at candidate sites whose structure admits them. Selecting several notes and
pressing `8` makes each an eighth; raising a group previews all new written pitches; voice assignment previews the
complete resulting notation. Each action is reversible inside Review through a local decision history independent of
project undo because source has not changed; accepting still creates exactly one project revision.

Audition has an A/B control: **Played** reproduces captured timing/controllers through the selected prepared instrument;
**Written** renders the current exact notation proposal through the ordinary performance path. Switching is click-free
at a phrase boundary or explicit stop, never by running two engines. The playhead and selected notes use existing event
identity overlays. Review preserves raw evidence after every change.

Accessibility is complete: every proposal note has a musical accessible name; ambiguity marks announce their
alternatives; selection, tap, choice, transform, A/B, accept, and discard are keyboard reachable; focus survives
proposal replacement; screen readers receive one concise result per action, not per re-engraved note.

## Target

- Project/desktop Review session facade and generated IPC for proposal engraving, ambiguity facts, constraints, decision
  history, raw/written audition, selection transformations, and source preview.
- Progressive leaf/top-margin interface with tap-pulse/downbeat, local alternative choices, group transformations,
  voice/chord/tie/spelling operations, A/B audition, Accept/Discard controls, and stale/device-loss states.
- Screenshot/Playwright fixtures for clean one-click acceptance and every measured ambiguity class, at narrow/wide,
  light/dark, 200% text, reduced motion, keyboard-only, and screen-reader roles/names.
- Interaction budgets for input-to-audition, action-to-preview, local retranscription, engraving, and focus restoration.

## Check

```sh
cargo nextest run -p musa-project -p musa-desktop
cargo clippy --all-targets -p musa-project -p musa-desktop -- -D warnings
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
cargo insta test --workspace --unreferenced=reject
cd apps/musa-desktop/ui && npx pnpm run check && npx pnpm run test:unit
cd apps/musa-desktop/ui && npx playwright test --project=chromium
```

Commit as `Review transcription as notation and musical choices`.

## Stop

- No source mutation/acceptance transaction, autosave, step-entry deletion, piano roll, waveform, or raw event table.
- No editable frontend score model, generic parameter panel, confidence percentage, cloud/learned model, or hidden
  retry.
- No review control without a measured ambiguity, semantic project operation, keyboard path, and accessible name.
