---
id: 64
slug: meter-changes
status: pending
depends_on: [61, 63]
phase: 3
---

# Mid-Piece Meter

## Task

A composer can write a second `meter`. The bar count restarts, the engraver draws the time signature, and MEI,
LilyPond and MusicXML all say so. This is the prompt prompt 57 deferred to by name — *"`MeterMap` becomes a map, and
every exporter learns to write the change"* — and it is also what makes the bar length `bar 5/4 { … }` and the
pickup measure expressible, because both are meter facts and now have somewhere to live.

Prompts 61 and 63 built the whole machine. This prompt writes the grammar, populates it, and turns on the
`is_constant` fast path's other branch.

## Read

- Prompt 57 §"Irregular lengths are deferred, and the reason is honest" — the debt this pays.
- Prompt 61 `bars.rs` — `BarLines::uniform` is the only constructor; this prompt adds the other one.
- Prompt 63 `context.rs` and `scope.rs` — `ContextTrack`, the inheritance table, and the motif-body invariant.
- `crates/musa-render/src/plan.rs` — `Fold` (:584) and `plan_staff`'s measure walk (:820).
- `crates/musa-compiler/src/elaborate.rs` — `Share` and `bind_anonymous`; the bar-length check (:1680).

## Design

### The grammar

`meter` becomes a voice item, legal wherever a note is, taking effect from where it is written:

```musa
voice right {
    bar { c5 1/4; d5 1/4; e5 1/4; f5 1/4; }
    meter 3/4;
    bar { g5 1/4; a5 1/4; b5 1/4; }
}
```

Written **at the cursor**, not as `meter 3/4 at 9:1;`. The `at` form was drafted and is rejected here: a measure
coordinate is a position in bars, and where the bars fall is what the meter determines, so `meter 7/8 at 9:1` is a
fixpoint. It is well-founded only as an ascending fold and only if every change lands on a barline — which is a
solvable problem, and an entirely unnecessary one, because the composer is already writing at a place in the voice.
Say the thing where it happens.

The piece-level `meter` in the header stays and means "from the beginning".

### A change must land on a barline

Otherwise `BarLines::time_of` is partial in a way no caller can act on, and the engraver has to invent a bar that
is neither length. The diagnostic is prompt 56's `does-not-add-up` again, because it is the same mistake:

```
  × a meter change must land on a barline
    ╭─[examples/broken/meter-mid-bar.musa:8:9]
  8 │         meter 3/4;
    ·         ─────┬────
    ·              ╰── this is 1/4 into measure 5
    ╰────
  help: add 3/4 before it, or move it after the next bar
```

### Meter resolution becomes its own pass

`BarLines` is needed by the bar-length check, by `check_tuplets`, and by the meter-change-on-a-barline check itself
— so the meters must be resolved before any of them run, in an **ascending fold** over the voice. That is a new pass
in `elaborate.rs`, ordered before sections, chords and tempo, and it is the reason prompt 61 insisted `at`/`time_of`
be inverses.

### The two hazards prompts 61 and 63 wrote down, now live

**`Share`.** A motif body elaborated once and referenced twice sits under two meters, so its bar-length check has
two answers. The invariant from prompt 63 becomes an error here: a `meter` statement inside a motif or bar body is
rejected, with a diagnostic that says why — a motif is material, and where the barlines fall is a property of the
place it is used, not of the material.

**`Fold`.** Prompt 58's repeat prints once and plays twice, so the sounding pass crosses a meter change the written
pass crosses once. Notation walks folded `BarLines`; performance and `facts.rs` walk unfolded ones. Prompt 61 built
both instances; this prompt is where they stop being equal, so the test that proves they differ belongs here.

### The exporters

- **MEI**: `<scoreDef>` with a new `<meterSig>` at the measure that starts the change.
- **LilyPond**: `\time 3/4` at the measure.
- **MusicXML**: `<time>` inside the measure's `<attributes>`.
- **MIDI**: a time-signature meta event. This is exact — SMF has one, and it belongs on the tempo track.

Each backend's constant case must produce byte-identical output to before, which is what `is_constant` is for.

## Target

- `crates/musa-language`: `meter` as a voice item — one `SyntaxKind`, one AST wrapper, one `VoiceItem` variant, the
  recovery set, and the formatter's blank-line rule (a meter change is a paragraph break, like a section).
- `crates/musa-compiler`: the meter-resolution pass; `BarLines::from_changes`; the barline and motif-body
  diagnostics; `bar 5/4 { … }` and the pickup, which are now expressible and must be accepted.
- `crates/musa-render`: the four exporters above; `plan.rs` walks `BarLines::measures()`.
- `examples/`: `changing-meter.musa` — a folk tune alternating 7/8 and 4/4, which is the case this exists for, plus
  a pickup in `twinkle.musa`. `examples/broken/meter-mid-bar.musa`.
- `docs/initial-design-roadmap.md` §7.2; prompt 57's deferral struck with a pointer here.

## Check

```sh
cargo nextest run --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
cargo run -p musa-cli -- check examples/changing-meter.musa
cargo run -p musa-cli -- render examples/changing-meter.musa --to lilypond | grep -c '\\time'   # 2 or more
cargo run -p musa-cli -- check examples/broken/meter-mid-bar.musa
git diff --stat -- crates/*/tests/snapshots    # only fixtures that gained a meter change
```

Commit as `Let the meter change mid-piece`.

## Stop

- No key or clef change — prompt 65. One kind at a time, because each has its own inheritance rule and its own
  exporter shape, and a combined prompt would hide which one broke.
- No polymeter. A meter written in a voice applies to the piece from that point; per-voice meter is prompt 75, and
  it needs its own evidence.
- No `senza misura` — prompt 74.
- No automatic meter inference from bar lengths. A bar declares what it claims to be; musa does not guess.
- No mid-bar meter change, even where a backend could express it.
