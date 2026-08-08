---
id: 76
slug: realization-in-the-page
status: pending
depends_on: [59, 68]
phase: 2
---

# The Realization in the Page

## Task

Prompt 66 conceded a real cost: the score view stops being a function of the source alone. A composer who opens an
aleatory piece twice and sees two different pages, with nothing in the window explaining why, has been given a bug
rather than a feature.

Pay the cost. The page prints the freedom, the Origin view shows the decision, and Settings holds the seed.

## Read

- `docs/kernel/11-realization.md` (prompt 66) §"The cost, conceded up front" — this prompt is that paragraph's
  implementation, and it should be read as a debt being settled.
- Prompt 24 (`origin-view`) — the provenance lens. A decision is provenance, and this is a fourth step in an
  existing chain, not a fifth panel.
- Prompt 59 (`menus-and-settings`) — where a preference lives now that Settings exists.
- Prompt 68 — `FactKind::{Mobile, FreeDuration, Improvise}`, and prompt 58's rule about printing the instruction.
- `docs/interface/` §states and voice — the wording rules. "Seed 42" is not a sentence a musician reads.

## Design

### Three things the composer must be able to see

**What is free.** The page prints the instruction, per prompt 58's rule: a boxed fragment, *ad lib.* under an
improvised span, a bracketed duration, a repeat sign with "4–16×". This is engraving, and it comes from the facts
prompt 68 already puts in the timeline — no new data, only marks on the page.

**What was chosen.** The Origin view gains a step. Its chain today is *source → declaration → expansion →
occurrence*; a realized occurrence gains **decision**: which path, what was decided, and whether it came from the
seed or from a pin. One more row in a lens that exists, which is the test of whether prompt 24's design was right.

**How to change it.** Two controls and no more:

- A **seed** field in Settings, per prompt 59: a number, a "new performance" button that draws a fresh one, and the
  statement that a piece with no choices ignores it.
- A **pin** action on a decision in the Origin view: "keep this one", which writes a `ChoicePath` override into the
  project. Pinning is how a composer converges on a performance they like, one decision at a time, which is what
  actually happens when someone works with an open-form piece.

### Where the realization is stored

In the **project**, not the source. A `.musa` file is the work; a realization is a reading of it. Putting a seed in
the source would make two composers with the same file unable to disagree about a performance, which is the opposite
of what open form is for.

The consequence is that a realization must survive a session, so it belongs beside the other project state prompt 19
already persists. Exports carry it: a `.kernel` file's header (prompt 67), and a note in exported MEI/MusicXML that
this is one realization of an open work.

### The wording

`docs/interface/` governs, and the rule is that the interface says what happened, not what the machine did.

- Not "Seed: 42" but **"Performance 42"**, with "New performance" as the verb.
- Not "ChoicePath(Part(piano), Bar(fill), Ordinal(0))" but **"the fill in *piano*, first choice"**.
- A pinned decision reads **"kept"**, and an unpinned one reads as the performance it came from.

A composer should be able to use this without learning the word "realization", and if the interface makes them learn
it, the design is wrong.

### Determinate pieces show nothing

No seed field, no Origin step, no page marks — because there are no choices. The controls appear when the piece has
a decision, and vanish when it does not. A settings panel with a seed field in a piece that cannot use one is
exactly the "four-panel toolbar application" `docs/interface/` exists to avoid.

### The performance budget

Redrawing after a new seed is a full recompile and re-engrave, so it lands under B2 (≤400 ms p95) — which prompt 50
measured at 373 ms with 27 ms of headroom. A seed change is not a keystroke, so it is not B2's workload; measure it
separately and report it rather than assuming the budget covers it.

## Target

- `apps/musa-desktop/ui`: the seed control in Settings; the decision step in the Origin view; the pin action; the
  appear/vanish rule.
- `crates/musa-project`: the realization persisted in project state; pin and unpin commands with undo.
- `crates/musa-render/src/plan.rs`: the fragment box, the *ad lib.* text, the duration bracket, the ranged repeat
  sign.
- `docs/interface/`: the wording rules and the new Origin step.
- `apps/musa-desktop/ui/fixtures/`: an open-form fixture and its screenshot goldens, at two seeds.

## Check

```sh
cd apps/musa-desktop/ui && pnpm test
cargo nextest run -p musa-project
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
```

Plus, by hand and recorded in the prompt: open `examples/in-c.musa`, draw a new performance, pin one decision, draw
again, and confirm the pinned decision held. That is the feature; a test that does not do it has not checked it.

Commit as `Show the realization in the page`.

## Stop

- No seed in the source language.
- No performance history, no A/B comparison, no "shuffle until I like it" gallery.
- No per-decision UI beyond pin and unpin. Editing a decision to an arbitrary value is what pinning already does,
  once the composer has drawn one they want.
- No realization in the Compose screen's chrome. It belongs in Settings and in Origin, which are the two places the
  composer already goes to ask "why does it look like this".
- No export of the pin set as a separate file format.
