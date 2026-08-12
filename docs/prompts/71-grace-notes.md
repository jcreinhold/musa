---
id: 71
slug: grace-notes
status: done
depends_on: [70]
phase: 3
---

# Grace Notes

## Task

`grace { c5; d5; } e5 1/4;` — an acciaccatura, an appoggiatura, a Scottish snap, a mordent written out, a blues crush
note. Grace notes have a written pitch and no written duration, which is the one shape musa's note model cannot hold.

They are **not** marks, and the prompt exists partly to say why.

## Read

- `crates/musa-compiler/src/elaborate.rs` — the doc comment above `FactKind` (:70), which states the rule that decides
  this: *does it have an extent and an identity?* An articulation has neither; a slur has both. Apply the rule to a
  grace note before designing.
- `docs/kernel/03-denotational-semantics.md` — a **point occurrence** is `s == e`, already legal and already normalized.
- `docs/kernel/05-normalization.md` **N2** — the ordering: `(start, end, payload key)`. This is the constraint that
  forces the design.
- Prompt 28 and prompt 70 — profile settings, which is where the time a grace note steals is decided.
- `docs/roadmap.md` §2 — notated duration ≠ performed duration.

## Design

### Why not a mark

Apply the repository's own rule. A staccato dot has no span but its note's and cannot move without the note; a grace
note has its own pitch, its own accidental, its own beam, its own slur to the principal, and there can be four of them
in an order that matters. It has an identity. It is an occurrence.

Making it a mark would mean a mark whose payload is a list of pitches, which is a note by another name.

### The shape

```rust
FactKind::Grace { pitch: Pitch, spelling: Spelling, index: u8, marks: Vec<Mark> }
```

as a **point occurrence** at the principal note's start: `s == e`, zero written duration, which is exactly what a grace
note is.

### The `index` is load-bearing

N2 orders occurrences by `(start, end, payload key)`. Two grace notes at the same instant have the same start and the
same end — so without something in the payload to separate them, `grace { c5; d5; }` and `grace { d5; c5; }` normalize
to the *same timeline* and are semantically equal. They are not equal; they sound different and they print differently.

`index` is that something. It is the first case in musa where ordering information had to enter a payload, and the
document should say so, because the alternative — giving grace notes non-zero durations so they sort — would put
performed time into notation and break §2.

This is prompt 45's answer shape reused: **extend the denotation with a value, not the calculus with a form.**

### What time it steals is the profile's, and that is the point

MusicXML writes `<grace steal-time-previous="50"/>`. That bakes an interpretation into the notation, forcing the
*editor* to settle a question on which Baroque and Romantic practice genuinely disagree — a Baroque appoggiatura takes
half the principal's value; a Romantic acciaccatura is crushed before the beat.

Musa puts it where it belongs:

```musa
profile baroque  { grace { steal = 1/8;  from = principal; } }
profile romantic { grace { steal = 1/32; from = previous;  } }
```

Same notation, two performances, and neither is written into the file. This is the clearest payoff §2 has produced so
far, and it is worth stating in `docs/roadmap.md` §2 as a worked example.

Absent a profile, the default is `steal = 1/16, from = principal` — a short grace on the beat, which is the reading a
modern performer defaults to. Defaults go inside; no caller passes this.

### The performance consequence

A point occurrence has zero duration, so `lower_performance` must give it one, taken from the principal or the previous
note per the profile. The principal's performed start or duration moves; its **notated** duration does not. The test is
that the notation golden is unchanged when the profile changes and the MIDI is not.

### Beaming and slurs

A run of grace notes beams together and slurs to its principal by convention. That is `plan.rs`'s job and needs the
`index` to order them — the second consumer of the field, which is what makes it a payload value rather than a
tie-breaker hack.

### The exporters

|  |  |
| --- | --- |
| MEI | `<graceGrp attach="pre">` with `<note grace="unknown">`, ordered by `index` |
| MusicXML | `<grace slash="yes"/>`; **do not** emit `steal-time-*` — the interpretation is not in the file |
| LilyPond | `\grace { … }` — the one command that carries no reading |
| MIDI | the realized notes, at the profile's timing |

LilyPond looked like the interesting one: `\acciaccatura` and `\appoggiatura` each bake in a reading, so the export
seemed forced to pick from the profile. It is not — `\grace` is neutral, and it is what a notation backend must write,
because a backend that consulted the profile would break the law the Check asserts one line later. See repair 5 below,
and `07-backend-contract.md`.

## Target

- `crates/musa-language`: `grace { … }` before a note statement; pitches without durations inside it; recovery,
  formatting (a grace group stays inline).
- `crates/musa-compiler`: `FactKind::Grace`; the `index`; the N2 ordering test that fails without it; the `grace`
  profile settings — which are the second and third settings prompt 62 deferred generalizing for, so the per-mark
  settings table is designed **here**, with three examples rather than one.
- `crates/musa-render`: the four exporters; beaming and the slur to the principal.
- `crates/musa-compiler/src/performance.rs`: the steal, both directions.
- `examples/`: `graces.musa` — the same three-note figure under a Baroque and a Romantic profile, which is the fixture
  that proves the split.
- `docs/roadmap.md` §2: the worked example.

## Check

```sh
cargo nextest run --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
cargo run -p musa -- check examples/graces.musa
# order matters — the two must differ:
diff <(cargo run -q -p musa -- kernel examples/graces.musa) \
     <(cargo run -q -p musa -- kernel examples/graces-reordered.musa) && exit 1
# notation is profile-independent, performance is not — the pair of tests
# that say so, in the two crates that own the two halves:
cargo nextest run -p musa-render --test graces -p musa-compiler --test graces
```

Commit as `Add grace notes`.

## Repairs made while implementing

1. **The profile rule is `grace { … }`, not `mark grace { … }`.** The prompt's own thesis is that a grace note is not a
   mark; writing its settings under the `mark` head would have said the opposite in the one place a reader looks for the
   answer. It is a nameless rule head of its own, following prompt 69's `groove` precedent.
2. **`SettingStmt` gained a word value and `SettingStmt::word()`.** `from = principal;` is the first setting whose value
   is a *choice between named readings* rather than a quantity, and the grammar had no way to write one. All four number
   readers were taught to read a word too, so a word where a quantity belongs is refused by name
   (``​`wide` is not a hold``) instead of being dropped silently.
3. **The steal is bounded at both ends, and the bounds are stated rather than discovered.** Graces taken from the
   principal may take at most half its written value; graces taken from the previous note may reach back at most to that
   note's midpoint, and asking for more — or standing at the start of a voice — is read as taking from the principal
   instead. Without the first bound a greedy profile produces a note whose off precedes its on; without the second, a
   grace swallows the note behind it.
4. **`from = previous` really shortens the note before.** The previous note's note-off is pulled back to where the grace
   starts (never later — a staccato note does not grow because the next note has a grace). "Stealing" that left the
   previous note ringing under the grace would have been the word without the deed.
5. **LilyPond writes `\grace`, and no notation backend sees a profile.** The prompt asked the LilyPond export to choose
   `\acciaccatura`/`\appoggiatura` from the profile. It must not: `NotationPlan` is built without one, and giving one to
   a notation backend would break "the page does not say how a grace is played" — the law this prompt exists to
   establish — in the same commit that asserts it. `\grace` is neutral and is what a house style expects to interpret.
6. **MEI writes `grace="unknown"`, not `grace="acc"`.** MEI's `acc` and `unacc` are the two readings; `unknown` is the
   page. The prompt named `acc` by inheriting `MusicXML`'s framing, which is exactly the framing it rejects.
7. **`MusicalTime::new` clamps negatives, so the reach-back test is on the ratio.** Comparing constructed times made a
   grace before the start of the piece look legal, and it sounded on top of the note it was supposed to precede — a bug
   that only a test asserting on *frames* could see.
8. **Beaming needed no change.** The prompt assigned `plan.rs` grace beaming and a slur to the principal. Because a
   `PlannedGrace` lives *inside* its `NotatedItem` rather than among the items — which is what all three backends want —
   beaming is untouched by construction, and the slur is a house-style flourish that each of the three neutral grace
   forms deliberately omits.
9. **The example pair is byte-aligned.** `graces-reordered.musa` is `graces.musa` with two groups reversed and its
   explanatory comment at the *bottom* of the file, so every byte above is identical and the kernel diff shows the
   reordering and nothing else. Occurrence payloads carry source spans; a comment at the top would have moved every span
   and buried the finding.

## Stop

- No `steal-time` in the source language, and none in MusicXML output.
- No automatic ornament expansion — prompt 70 already refused it, and a written-out mordent is what `grace` is for.
- No grace notes after a note (nachschlag) unless a fixture needs one; it is a different attachment and a different
  span.
- No grace note on a rest.
- No new kernel operation. A point occurrence is D0 as written.
