---
id: 71
slug: grace-notes
status: pending
depends_on: [70]
phase: 3
---

# Grace Notes

## Task

`grace { c5; d5; } e5 1/4;` — an acciaccatura, an appoggiatura, a Scottish snap, a mordent written out, a blues
crush note. Grace notes have a written pitch and no written duration, which is the one shape musa's note model
cannot hold.

They are **not** marks, and the prompt exists partly to say why.

## Read

- `crates/musa-compiler/src/elaborate.rs` — the doc comment above `FactKind` (:70), which states the rule that
  decides this: *does it have an extent and an identity?* An articulation has neither; a slur has both. Apply the
  rule to a grace note before designing.
- `docs/kernel/03-denotational-semantics.md` — a **point occurrence** is `s == e`, already legal and already
  normalized.
- `docs/kernel/05-normalization.md` **N2** — the ordering: `(start, end, payload key)`. This is the constraint that
  forces the design.
- Prompt 28 and prompt 70 — profile settings, which is where the time a grace note steals is decided.
- `docs/initial-design-roadmap.md` §2 — notated duration ≠ performed duration.

## Design

### Why not a mark

Apply the repository's own rule. A staccato dot has no span but its note's and cannot move without the note; a grace
note has its own pitch, its own accidental, its own beam, its own slur to the principal, and there can be four of
them in an order that matters. It has an identity. It is an occurrence.

Making it a mark would mean a mark whose payload is a list of pitches, which is a note by another name.

### The shape

```rust
FactKind::Grace { pitch: Pitch, spelling: Spelling, index: u8, marks: Vec<Mark> }
```

as a **point occurrence** at the principal note's start: `s == e`, zero written duration, which is exactly what a
grace note is.

### The `index` is load-bearing

N2 orders occurrences by `(start, end, payload key)`. Two grace notes at the same instant have the same start and
the same end — so without something in the payload to separate them, `grace { c5; d5; }` and `grace { d5; c5; }`
normalize to the *same timeline* and are semantically equal. They are not equal; they sound different and they print
differently.

`index` is that something. It is the first case in musa where ordering information had to enter a payload, and the
document should say so, because the alternative — giving grace notes non-zero durations so they sort — would put
performed time into notation and break §2.

This is prompt 45's answer shape reused: **extend the denotation with a value, not the calculus with a form.**

### What time it steals is the profile's, and that is the point

MusicXML writes `<grace steal-time-previous="50"/>`. That bakes an interpretation into the notation, forcing the
*editor* to settle a question on which Baroque and Romantic practice genuinely disagree — a Baroque appoggiatura
takes half the principal's value; a Romantic acciaccatura is crushed before the beat.

Musa puts it where it belongs:

```musa
profile baroque  { mark grace { steal = 1/2;  from = principal; } }
profile romantic { mark grace { steal = 1/32; from = previous;  } }
```

Same notation, two performances, and neither is written into the file. This is the clearest payoff §2 has produced
so far, and it is worth stating in `docs/initial-design-roadmap.md` §2 as a worked example.

Absent a profile, the default is `steal = 1/16, from = principal` — a short grace on the beat, which is the reading
a modern performer defaults to. Defaults go inside; no caller passes this.

### The performance consequence

A point occurrence has zero duration, so `lower_performance` must give it one, taken from the principal or the
previous note per the profile. The principal's performed start or duration moves; its **notated** duration does not.
The test is that the notation golden is unchanged when the profile changes and the MIDI is not.

### Beaming and slurs

A run of grace notes beams together and slurs to its principal by convention. That is `plan.rs`'s job and needs the
`index` to order them — the second consumer of the field, which is what makes it a payload value rather than a
tie-breaker hack.

### The exporters

| | |
| --- | --- |
| MEI | `<graceGrp>` with `<note grace="acc">`, ordered by `index` |
| MusicXML | `<grace slash="yes"/>`; **do not** emit `steal-time-*` — the interpretation is not in the file |
| LilyPond | `\acciaccatura` / `\appoggiatura` per the profile's `steal`, since LilyPond has no neutral form |
| MIDI | the realized notes, at the profile's timing |

LilyPond is the interesting one: it has no way to write a grace note without choosing, so the export *must* pick,
and the choice comes from the profile rather than the source. Say so in `07-backend-contract.md`.

## Target

- `crates/musa-language`: `grace { … }` before a note statement; pitches without durations inside it; recovery,
  formatting (a grace group stays inline).
- `crates/musa-compiler`: `FactKind::Grace`; the `index`; the N2 ordering test that fails without it; the `grace`
  profile settings — which are the second and third settings prompt 62 deferred generalizing for, so the per-mark
  settings table is designed **here**, with three examples rather than one.
- `crates/musa-render`: the four exporters; beaming and the slur to the principal.
- `crates/musa-compiler/src/performance.rs`: the steal, both directions.
- `examples/`: `graces.musa` — the same three-note figure under a Baroque and a Romantic profile, which is the
  fixture that proves the split.
- `docs/initial-design-roadmap.md` §2: the worked example.

## Check

```sh
cargo nextest run --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
cargo run -p musa-cli -- check examples/graces.musa
# order matters — the two must differ:
diff <(cargo run -q -p musa-cli -- kernel examples/graces.musa) \
     <(cargo run -q -p musa-cli -- kernel examples/graces-reordered.musa) && exit 1
# notation is profile-independent, performance is not:
cargo run -p musa-cli -- render examples/graces.musa --to mei > /tmp/a.mei
```

Commit as `Add grace notes`.

## Stop

- No `steal-time` in the source language, and none in MusicXML output.
- No automatic ornament expansion — prompt 70 already refused it, and a written-out mordent is what `grace` is for.
- No grace notes after a note (nachschlag) unless a fixture needs one; it is a different attachment and a different
  span.
- No grace note on a rest.
- No new kernel operation. A point occurrence is D0 as written.
