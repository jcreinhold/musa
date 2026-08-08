---
id: 68
slug: open-form
status: pending
depends_on: [67]
phase: 3
---

# Open Form

## Task

The three remaining kinds of written freedom, on prompt 67's mechanism: a **mobile form** whose sections may be
played in any order, a **free duration** the performer chooses from a range, and an **improvisation region** with a
notated frame and unnotated contents. Each is a `Decision` variant that already exists, a `FactKind` the page can
print, and no kernel change.

The acceptance test is repertoire, not syntax: *In C*, Klavierstück XI, and a jazz chart's solo section must each be
writable, and the corpus is where they are.

## Read

- `docs/kernel/11-realization.md` — the repertoire it names is what this prompt must deliver.
- Prompt 67 — `Decision::{Order, Duration}` exist and have no producer. This prompt is the producer.
- Prompt 58 §"the timeline holds every pass; the page prints the instruction once" — the rule for how a freedom
  appears on the page.
- `docs/course-correction.md` §33 rows 7–10 — the falsification corpus. Adding pieces to it is part of this prompt.

## Design

### The grammar

```musa
// A named fragment: material that can be reordered, repeated, or skipped.
fragment a { c5 1/4; e5 1/4; }
fragment b { g5 1/2; }

// A mobile: its fragments in any order, chosen once per performance.
mobile { a; b; c; }

// A free duration: the performer picks, the compiler picks for playback.
g4 1/4 to 2/1;

// An improvised region: a frame with no notes in it.
improvise 8/1 over "Dm7 | G7 | Cmaj7";
```

`fragment` is a named body like a motif — prompt 57 already established the namespace and its diagnostics, so this
adds a `Material` tag rather than a namespace.

### What reaches the timeline

Each construct emits **both** the realized music and a fact describing the freedom, because the page must print the
instruction and the playback must sound something:

| Construct | Realized | Fact |
| --- | --- | --- |
| `mobile` | the fragments, in the chosen order | `FactKind::Mobile { fragments }` over the whole span |
| `x to y` | one duration | `FactKind::FreeDuration { min, max }` on the note |
| `improvise` | silence of the frame's length | `FactKind::Improvise { over }` over the span |

That is prompt 58's shape applied three times: the realized music is what the timeline holds, the fact is what the
engraver reads to draw a box, a bracket, or an *ad lib.*

### *In C* is the test that matters

*In C* is fifty-three figures, each repeated as many times as the player likes, entered when the player likes. It
needs prompt 67's ranged repeat **and** this prompt's fragments, and it needs them per player — which is the case
that will show whether `ChoicePath` is right, because fifty-three fragments in each of several parts is exactly where
an ordinal-based identity would fall over.

Write it. If it cannot be written, this prompt's design is wrong and the repair goes in `11-realization.md` first.

### Mobile form and the ordering decision

`Decision::Order(Vec<u32>)` is a permutation of the fragment list. Drawn per path from the seed by Fisher–Yates
over a per-path derived stream, which is deterministic and needs no `rand`. Klavierstück XI's 19 fragments give 19!
orderings — the number that killed `choose` — and here it costs nineteen `u32`s.

### Free duration and where the *notated* value goes

`g4 1/4 to 2/1` is written as a quarter and may be held to a double whole. The **notated** duration is the minimum;
the **performed** duration is the decision. That is §2's row exactly — notated duration ≠ performed duration — and
it is why this does not need a new time representation. The engraver draws a quarter with a bracket; the performance
plan uses the drawn value.

### Improvisation is a frame, not a hole

`improvise 8/1 over "Dm7 | G7 | Cmaj7"` occupies eight whole notes of real time so everything after it lands where
it should. It sounds as silence, because musa does not improvise, and the chord text is prompt 35's harmony payload,
reused rather than reinvented. A future prompt could sound a comping pattern; this one must not, because "musa
invents notes" is a product decision, not a feature.

### The exporters

MEI and MusicXML have no mobile form, no free duration bracket, and no improvisation region. So each emits the
**realized** music plus a text direction carrying the instruction — which is lossy, and is stated as lossy in
`07-backend-contract.md` rather than discovered. LilyPond gets closer with `\markup` and repeat bars. MIDI emits the
realization and nothing else; that is exact for what MIDI is.

## Target

- `crates/musa-language`: `fragment`, `mobile`, `improvise`, and `to` in a duration; recovery and formatting.
- `crates/musa-compiler`: three `FactKind` variants; `Decision::{Order, Duration}` producers; Fisher–Yates over the
  per-path stream; the notated-versus-performed split for free durations.
- `crates/musa-render`: the three lossy emissions above, each warning once; `plan.rs` draws the bracket and the box.
- `examples/`: `in-c.musa` (Riley — the fifty-three-figure test), `mobile.musa` (Klavierstück XI's shape),
  `changes.musa` (a chart with an improvised chorus).
- `docs/course-correction.md` §33: rows 7–10 marked proven, or the reason they are not.
- `docs/kernel/11-realization.md`: graduated from candidate to governing.

## Check

```sh
cargo nextest run --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
cargo run -p musa-cli -- check examples/in-c.musa
diff <(cargo run -q -p musa-cli -- kernel examples/mobile.musa --seed 7) \
     <(cargo run -q -p musa-cli -- kernel examples/mobile.musa --seed 7)     # stable
cargo run -p musa-cli -- render examples/changes.musa --to musicxml 2>&1 | grep -i "improvis"  # the warning
grep -L "Status: candidate" docs/kernel/11-realization.md
```

Commit as `Add open form`.

## Stop

- No performer-timed entry, no cueing between players, no real-time coordination. *In C*'s players enter "when they
  like"; musa compiles one plausible reading and says so.
- No generated improvisation. Silence and a frame.
- No graphic or text-only scores. A page musa cannot engrave is not a page musa should accept.
- No probability weights on a mobile's orderings.
- No new kernel operation, still.
