---
id: 76
slug: realization-in-the-page
status: done
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

- `docs/rules/kernel/11-realization.md` (prompt 66) §"The cost, conceded up front" — this prompt is that paragraph's
  implementation, and it should be read as a debt being settled.
- Prompt 24 (`origin-view`) — the provenance lens. A decision is provenance, and this is a fourth step in an existing
  chain, not a fifth panel.
- Prompt 59 (`menus-and-settings`) — where a preference lives now that Settings exists.
- Prompt 68 — `FactKind::{Mobile, FreeDuration, Improvise}`, and prompt 58's rule about printing the instruction.
- `docs/rules/desktop/` §states and voice — the wording rules. "Seed 42" is not a sentence a musician reads.

## Design

### Three things the composer must be able to see

**What is free.** The page prints the instruction, per prompt 58's rule: a boxed fragment, *ad lib.* under an improvised
span, a bracketed duration, a repeat sign with "4–16×". This is engraving, and it comes from the facts prompt 68 already
puts in the timeline — no new data, only marks on the page.

**What was chosen.** The Origin view gains a step. Its chain today is *source → declaration → expansion → occurrence*; a
realized occurrence gains **decision**: which path, what was decided, and whether it came from the seed or from a pin.
One more row in a lens that exists, which is the test of whether prompt 24's design was right.

**How to change it.** Two controls and no more:

- A **seed** field in Settings, per prompt 59: a number, a "new performance" button that draws a fresh one, and the
  statement that a piece with no choices ignores it.
- A **pin** action on a decision in the Origin view: "keep this one", which writes a `ChoicePath` override into the
  project. Pinning is how a composer converges on a performance they like, one decision at a time, which is what
  actually happens when someone works with an open-form piece.

### Where the realization is stored

In the **project**, not the source. A `.musa` file is the work; a realization is a reading of it. Putting a seed in the
source would make two composers with the same file unable to disagree about a performance, which is the opposite of what
open form is for.

The consequence is that a realization must survive a session, so it belongs beside the other project state prompt 19
already persists. Exports carry it: a `.kernel` file's header (prompt 67), and a note in exported MEI/MusicXML that this
is one realization of an open work.

### The wording

`docs/rules/desktop/` governs, and the rule is that the interface says what happened, not what the machine did.

- Not "Seed: 42" but **"Performance 42"**, with "New performance" as the verb.
- Not "ChoicePath(Part(piano), Bar(fill), Ordinal(0))" but **"the fill in *piano*, first choice"**.
- A pinned decision reads **"kept"**, and an unpinned one reads as the performance it came from.

A composer should be able to use this without learning the word "realization", and if the interface makes them learn it,
the design is wrong.

### Determinate pieces show nothing

No seed field, no Origin step, no page marks — because there are no choices. The controls appear when the piece has a
decision, and vanish when it does not. A settings panel with a seed field in a piece that cannot use one is exactly the
"four-panel toolbar application" `docs/rules/desktop/` exists to avoid.

### The performance budget

Redrawing after a new seed is a full recompile and re-engrave, so it lands under B2 (≤400 ms p95) — which prompt 50
measured at 373 ms with 27 ms of headroom. A seed change is not a keystroke, so it is not B2's workload; measure it
separately and report it rather than assuming the budget covers it.

## Target

- `apps/musa-desktop/ui`: the seed control in Settings; the decision step in the Origin view; the pin action; the
  appear/vanish rule.
- `crates/musa-project`: the realization persisted in project state; pin and unpin commands with undo.
- `crates/musa-render/src/plan.rs`: the fragment box, the *ad lib.* text, the duration bracket, the ranged repeat sign.
- `docs/rules/desktop/`: the wording rules and the new Origin step.
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

## Repairs made while implementing

1. **"New performance" counts forward; it does not draw at random.** The Design asked for "a button that draws a fresh
   one", which is wrong in two ways: an unreproducible number cannot be tested, and a composer who liked performance 42
   has no way back to it. Counting forward costs nothing, because the choice at a site is `fnv1a_128(seed ‖ path)` and
   never a stream — 43 is as unrelated to 42 as any other number is. The field is editable for the same reason: the
   reading you liked is a number you can type back.

2. **A realization change mints a revision.** Prompt 66's `realize()` documented the opposite. Making a pin undoable is
   what forced it: the alternative was a second undo stack for decisions, so `HistoryEntry` gained the realization
   instead and reading again, keeping, and releasing are moves in the one history. Recorded in `05-states.md` §9, which
   distinguishes this from a preference — a performance changes the music on the page, so it must be reversible where
   the music is.

3. **The ranged repeat had to reach the fact, not just the reading.** `FactKind::Repeat` gained
   `range: Option<(u32, u32)>` and `RepeatMark` carries it, because prompt 58's rule is that the page prints the
   *instruction*: a bar that engraved `4×` where the composer wrote `2 to 6` would have replaced the piece with one
   performance of it. It is an `OpenShape::Passes` so all three backends print it through the one text-direction emitter
   they already had, and `losses()` gained one honest line. Only ranged repeats change the interchange encoding, so
   exactly two goldens moved: `examples/kernel/{loop-durations,in-c}.kernel`.

4. **A decision has *sites*, plural.** `repeat 2 to 6` written once in each of three voices is one question — that is
   prompt 67's rule — so `DecisionRecord` carries every span that asked, and an event finds its decision by the
   innermost site containing it. The first implementation deduplicated on the whole decision and produced three
   identical rows.

5. **The selection is let go of when the performance changes.** An event id is a position in the score, so a piece read
   again renumbers and `event-10` survives while naming a different note. The interface caught this before the test did:
   after a new performance the inspector went on describing "the note", which was by then a rest four bars earlier.
   `Workspace.reconcile` now drops the selection across a re-read rather than carrying it, because there is no honest
   neighbour to move to between two readings.

6. **The budget is B11, not "measured separately under B2".** Naming it made the reason explicit: B2's 400 ms includes
   the 180 ms typing debounce, and a click never pays it, so folding the two together would hide a slow redraw behind a
   wait it does not do. Measured at **p95 21 ms** from the snapshot to the ink.

7. **`loop-durations.musa` is the interface's open-form fixture, at performances 4 and 8** (two passes and six), rather
   than `in-c.musa`. One question in three voices is the case that catches a decision coming apart per voice; In C's 53
   sites would have made a 4000-line fixture prove less.

8. **Eight screenshot goldens were regenerated here.** They had drifted from prompts 69–75 — the fermatas of prompt 70
   among them — and were red on `HEAD` before this prompt started.

### The by-hand check, recorded

`examples/in-c.musa`, opened from a copy, at performance 42:

```
figure_one, first choice = 8 passes
figure_two, first choice = 7 passes
figure_three, first choice = 11 passes
```

Keeping the third, then drawing performance 43:

```
figure_one, first choice = 9 passes
figure_two, first choice = 8 passes
figure_three, first choice = 11 passes [kept]
```

The pin held; the other fifty-two were drawn again. The reading beside the piece reads:

```toml
performance = 43

[kept]
"f12:figure_three#1:0" = "count=11"
```

## Stop

- No seed in the source language.
- No performance history, no A/B comparison, no "shuffle until I like it" gallery.
- No per-decision UI beyond pin and unpin. Editing a decision to an arbitrary value is what pinning already does, once
  the composer has drawn one they want.
- No realization in the Compose screen's chrome. It belongs in Settings and in Origin, which are the two places the
  composer already goes to ask "why does it look like this".
- No export of the pin set as a separate file format.
