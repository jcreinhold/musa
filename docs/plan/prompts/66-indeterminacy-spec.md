---
id: 66
slug: indeterminacy-spec
status: done
depends_on: [48, 49]
phase: 3
---

# Where Indeterminacy Lives

## Task

Specify — in `docs/`, before any code — how musa represents music that does not fix its own realization: *In C*'s
unbounded repeats, Klavierstück XI's orderings, Cage's and Feldman's free durations, a chart's improvised solo, a DJ's
variable-length loop. Answer the question Q2 deferred, by the trigger Q2 itself named: **the first aleatory surface
feature is now being designed.**

Specification only. Prompt 67 implements the mechanism; prompt 68 gives it a surface; prompt 76 gives it a page.

## Read

- `docs/rules/events/08-open-questions.md` **Q2** — read the working stance first. It already says each realized
  performance produces an ordinary finite event track timeline and that the choice mechanism lives in the surface and
  its provenance. This prompt is that stance being *confirmed by design*, not reversed.
- `docs/rules/events/08-open-questions.md` (no canonical `join`), §32 (do not prematurely decide), §34 (semantic
  necessity), §35 item 11 (do not add aleatory choice to the finite event track).
- `docs/rules/events/10-term-calculus.md` — T2 (`let` transparency), T3 (evaluation is normalization), T4 (totality and
  determinism); `05-normalization.md` N6 (the semantic hash).
- Prompt 43 — the semantic hash and what recompiles when it moves.
- Prompt 58 — *the timeline holds every pass; the page prints the instruction once*. This is the shape.

## Design

### The candidate that has to be refused, and why

The obvious design is a `choose` term form — `choose { a | b | c }` — on the same argument that justified `Progress` at
prompt 45: an interchange file should carry *the work*, not one performance of it. That argument is good and it still
fails, for four reasons the document must record, because it is the kind of design that will be proposed again.

1. **It cannot express the repertoire it is proposed for.** *In C* is "repeat each figure as many times as you like" —
   unbounded. Klavierstück XI is one of 19! orderings. Cage and Feldman specify durations from a continuum.
   Improvisation is not enumerable at all. A finite alternative set covers the narrowest subcase and leaves the music
   that motivated it out.
2. **It breaks T2.** In `let x = choose { a | b } in over x x`, does sharing share the *decision*? Both readings are
   musically real — one performer's choice heard twice, or two performers choosing independently — and neither is
   canonical. This is §16's "no canonical `join`" in a new costume, and T2 is what prompt 49's measured −36% time and
   −60% allocations rest on.
3. **It breaks T3, T4, and N6 together.** Evaluation stops being deterministic and stops being unique, so there is no
   normal form and therefore no semantic hash. Prompt 43 keyed playback on that hash: editing an unchosen branch would
   change the work without changing the hash, and the session would not recompile.
4. **It destroys the artifact that justified it.** A `.event track` file containing `choose` cannot be normalized or
   hashed without a choice environment, so the environment must ship alongside — which makes the file a *realization*
   corpus after all, at the cost of every theorem above.

Applying §34 literally: removing `choose` makes nothing impossible, and adding it makes two consumers — the engraver and
the interchange format — strictly worse. It stays out.

### What goes in instead

**A realization is a compile parameter; the freedom is a payload value; the event track does not change.**

```text
source  ──elaborate(realization)──▶  Term<ScoreFact>  ──evaluate──▶  Timeline
   │                                                                     │
   └── carries the freedom as facts ────────────────────────────────────┘
       (Fragment, Improvise, FreeDuration — printed by the page,
        resolved by the realization)
```

- The **freedom** is written in the source and survives into the timeline as ordinary occurrences, so the page can print
  *ad lib.*, a repeat-as-many-times instruction, or a boxed fragment. Prompt 58's rule exactly.
- The **decision** is a `Realization`: a seed plus a set of explicit overrides. Two compiles with the same source and
  the same realization produce the same timeline, the same normal form, and the same hash. T2–T4 and N6 are untouched,
  because by the time the event track sees anything, every choice is made.

### Identity: a choice must be nameable across an edit

The hard part is not making a choice; it is making the *same* choice again after the composer edits an unrelated bar.
That needs a stable name per decision site.

- **Not a source span.** Reformatting would re-roll the performance.
- **Not a `DeclarationId`.** Inserting a declaration renumbers everything after it.
- **A structural path of names**: `ChoicePath` = part, voice, motif, bar, and an ordinal within the innermost named
  thing. Insert a bar at the top and the paths below it are unchanged, because names do not shift. (Prompt 67 narrowed
  this to motif, bar and ordinal — see `docs/rules/events/11-realization.md`; a site among a voice's own items belongs
  to the piece, because a repeat barline crosses the system.)

Each site's randomness is derived **per path** — `fnv1a_128(seed ‖ path)`, using the workspace's one stable digest
(`musa-events/src/hash.rs`) — not drawn from a sequential stream. A stream would re-roll every later decision when a
site is inserted, which is the same failure the span-based identity has, arriving later and less visibly.

### The cost, conceded up front

The score view stops being a function of the source alone. Every law of the form "same source, same picture" becomes
"same source **and the same realization**, same picture". That is a real weakening and the document must state it, along
with its two consequences: fixtures pin a seed, and the interface must be able to *show* the realization (prompt 76) or
the composer cannot tell why the page changed.

This is the honest price. It is smaller than the price of `choose`, and it is paid in one place rather than in four
theorems.

### What this closes

**Q2 is resolved**, by its own stated trigger and in favour of its own working stance. §35 item 11 needs **no repair** —
it forbids aleatory choice *in the finite event track*, which is exactly what this design does.

## Target

- `docs/rules/events/11-realization.md` (new, status **candidate**): the refusal above with its four reasons; the
  realization model; `ChoicePath` identity and per-path derivation; the conceded law weakening; what a conforming
  consumer owes.
- `docs/rules/events/08-open-questions.md`: **Q2 resolved (prompt 66)**, with the trigger quoted.
- `docs/rules/events/08-open-questions.md`: the open-question list updated; a note that item 11 is *upheld*, not
  amended.
- `docs/rules/events/07-backend-contract.md`: a `.event track` file is the projection of one realization, and its header
  says which.
- No code. No `Realization` type, no grammar.

## Check

```sh
grep -n "Status: candidate" docs/rules/events/11-realization.md
grep -n "RESOLVED (prompt 66)" docs/rules/events/08-open-questions.md
grep -c "In C\|Klavierstück XI\|Feldman" docs/rules/events/11-realization.md   # the repertoire is named, not gestured at
cargo fmt --check
```

Commit as `Specify where indeterminacy lives`.

## Repairs made while implementing

**The repertoire is a table, not a sentence.** The Design names five works in prose; the document states them as a table
with a third column — *what a fixed alternative set would have to be* — because that column is the refutation. "19! ≈
1.2 × 10¹⁷" and "uncountable" argue against `choose` in a way "Klavierstück XI has many orderings" does not. Two
non-aleatory cases were added for the same reason: a folk tune with an unknown verse count and a run-time-sized
installation want the same mechanism, which is evidence that the mechanism is not a niche.

**The determinism law is written down as R1.** The Design states "two compiles with the same source and the same
realization produce the same timeline"; a document that prompt 67 implements against needs that as a numbered law it can
be tested for, on the same footing as T1–T5 and N1–N6.

**`08-open-questions.md`'s §33 corpus row moved too.** The Target names four files; item 9 of the corpus table said
"blocked on Q2" and would have stayed wrong the moment Q2 was answered. It now points at the design.

## Stop

- No implementation. Prompt 67.
- No grammar, no `improvise`, no `repeat n to m`. Prompt 68.
- Do not settle Q1 (patterns) or Q5 (recursion) in passing. An unbounded repeat count looks like both and is neither:
  the count is chosen at compile time and the result is finite.
- No probability distributions, no weighted choice, no Markov anything. A seed and an override set is the whole
  mechanism; generative composition is a different product.
- Do not specify the interface. Prompt 76.
