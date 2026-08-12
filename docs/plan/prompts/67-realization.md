---
id: 67
slug: realization
status: done
depends_on: [66]
phase: 3
---

# Realizations

## Task

Implement prompt 66's mechanism: `Realization` as a compile parameter, `ChoicePath` as a decision's stable name,
per-path seed derivation, and one surface feature to prove it — `repeat 4 to 16 { … }`, a range of counts, chosen at
compile time. `musa check --seed 42` and `musa render --seed 42` reproduce a performance exactly; the same source with a
different seed produces a different, equally valid one.

One feature, chosen because it is the smallest thing that exercises every part of the mechanism and because it is what a
dance producer writes on the first line of a track.

## Read

- `docs/rules/kernel/11-realization.md` (prompt 66) — all of it. This prompt implements that document and nothing else.
- `crates/musa-kernel/src/hash.rs` — FNV-1a 128, the workspace's one stable digest. Do not add a second.
- `crates/musa-compiler/src/origin.rs` — `DeclarationId`, `Origin`, `expansion_path`. `ChoicePath` goes here, because it
  is provenance.
- `crates/musa-compiler/src/lib.rs` — `CompileOptions`, and how `compile` is called from `musa-project`.
- Prompt 43 — the semantic hash. A realization is an input to it, and this prompt must say how.

## Design

### The surface

```musa
repeat 4 to 16 { kick 1/4; kick 1/4; }     // between 4 and 16 times, chosen
```

`repeat n { … }` is unchanged and remains exact. The ranged form is the aleatory one, and the count it resolves to is a
decision with a path.

### The types

```rust
// crates/musa-compiler/src/realize.rs

/// Which performance to compile. Two compiles with equal sources and equal
/// realizations produce equal timelines, equal normal forms, and equal hashes.
pub struct Realization { seed: u64, overrides: BTreeMap<ChoicePath, Decision> }

/// What was decided at one site.
pub enum Decision { Count(u32), Order(Vec<u32>), Duration(Ratio<i64>) }

impl Realization {
    /// The realization every existing piece gets: seed zero, no overrides.
    /// A determinate piece never consults it, so this is not "a default
    /// performance" — it is the absence of a question.
    pub fn deterministic() -> Self;
    pub fn seeded(seed: u64) -> Self;
    /// Pin one site, leaving the rest to the seed.
    pub fn pinned(self, path: ChoicePath, decision: Decision) -> Self;
    /// Every decision actually taken, in path order — what the page shows and
    /// what a `.kernel` header records.
    pub fn taken(&self) -> impl Iterator<Item = (&ChoicePath, &Decision)>;
}
```

`Decision` carries all three shapes now, though only `Count` has a producer, because `taken()` and the `.kernel` header
are serialization surfaces and a third variant added later would move the format. `Order` and `Duration` arrive with
prompt 68; the enum is the one place where anticipating them costs nothing and not anticipating them costs a format
revision.

### `ChoicePath` is a path of names

```rust
pub struct ChoicePath(Vec<ChoiceStep>);
pub enum ChoiceStep { Part(Box<str>), Voice(Box<str>), Motif(Box<str>), Bar(Box<str>), Ordinal(u32) }
```

Built during elaboration from the enclosing named constructs, with `Ordinal` distinguishing sibling sites inside the
innermost named one. Inserting a bar above does not renumber a path below it, because names do not shift — that is the
entire reason this is not a `DeclarationId` and not a span.

`Ordinal` is the weak point and the document says so: two unnamed sibling sites in one voice, and inserting a third
between them shifts the second. The mitigation is that the composer who cares can name the bar, and prompt 76 shows
which path each decision belongs to so the shift is visible rather than mysterious.

### Derivation is per path, not a stream

```rust
fn draw(seed: u64, path: &ChoicePath) -> u128   // fnv1a_128(seed ‖ path.canonical())
```

A sequential PRNG stream would re-roll every later decision when a site is inserted. Per-path derivation means inserting
a site changes exactly that site. `path.canonical()` is a byte encoding with the same injectivity requirement N3 places
on payload keys, and it gets the same test.

### The realization is part of semantic identity

The hash covers the *timeline*, and the timeline already differs between realizations — so nothing needs to change, and
this prompt must **verify** that rather than assume it: two seeds producing different counts must produce different
hashes, and the same seed must produce the same hash across processes.

`.kernel` files need **no grammar change**. The header is a `%` comment line and `text.rs::parse` already treats later
`%` lines as trivia:

```
% musa-kernel 1
% realization seed=42 pins=0
```

`--check` reads it, reports it, and refuses nothing; the file is still exactly one timeline.

### The CLI

```sh
musa check  <file.musa> --seed 42
musa render <file.musa> --to mei --seed 42
musa kernel <file.musa> --seed 42
```

Absent, the realization is `deterministic()`. A piece with no aleatory construct is bit-identical under every seed, and
that is a test: run the whole `examples/` corpus under three seeds and diff.

## Target

- `crates/musa-compiler/src/realize.rs` (new): `Realization`, `Decision`, `draw`; `CompileOptions::realization`.
- `crates/musa-compiler/src/origin.rs`: `ChoicePath`, `ChoiceStep`, `canonical`.
- `crates/musa-language`: `repeat n to m { … }` — the range in the existing `repeat` statement.
- `crates/musa-compiler/src/elaborate.rs`: the path is threaded through elaboration; the ranged repeat draws its count
  and records the decision.
- `crates/musa-kernel/src/text.rs`: the realization header line, written and read.
- `crates/musa-project`, `crates/musa`: `--seed` on `check`, `render`, `kernel`.
- `examples/`: `loop-lengths.musa` — a four-bar house pattern whose fills repeat a variable number of times.
- `crates/musa-compiler/tests/realize.rs`: same seed → same hash; different seed → different hash; determinate pieces
  are seed-invariant across the corpus; `ChoicePath::canonical` is injective; inserting a named bar above a site does
  not change that site's decision.

## Check

```sh
cargo nextest run --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
for s in 1 2 3; do for f in examples/*.musa; do cargo run -q -p musa -- kernel "$f" --seed $s; done; done > /tmp/a
# determinate pieces identical under every seed:
diff <(cargo run -q -p musa -- kernel examples/canon.musa --seed 1) \
     <(cargo run -q -p musa -- kernel examples/canon.musa --seed 999)
cargo run -p musa -- check examples/loop-lengths.musa --seed 42
```

Commit as `Add realizations`.

## Repairs made while implementing

**A decision site among a voice's own items belongs to the piece, not to the voice.** The Design says a `ChoicePath` is
"part, voice, motif, bar, and an ordinal"; building the first ranged repeat showed that reading is wrong. Prompt 57
already refuses to *draw* a repeat whose voices disagree — "a repeat barline crosses the system, so it can only be drawn
where the whole system repeats" — so a per-voice path would decide one repeat three times, the voices would come apart,
and the engraver would quietly write the passage out. The path of a site written at `Place::Voice` is therefore its
ordinal alone, counted from zero in each voice, so the k-th such site in every voice is one site. Same argument as
prompt 64's for `meter`, same answer. `ChoiceStep` loses `Part` and `Voice` — a variant with no producer is dead surface
— and prompt 68 adds a per-voice step with the construct that needs it, since *In C* is per-player by definition and no
barline could span its players.

**A shared body's decisions belong to the material.** A motif or named bar is elaborated once and shared between call
sites (prompt 49), so its inner `ChoicePath` prefix is reset to the material's own name rather than inherited from the
caller — exactly as the provenance `path` already is. A ranged repeat inside a motif therefore takes one count per
motif. This is prompt 66's T2 sharing question answered by construction rather than by a rule, and it is the reading T2
forces once sharing is load-bearing.

**A determinate piece's `.kernel` file has no realization header at all.** The Design shows the header written
unconditionally, which would have made every existing golden depend on a seed it never reads and broken the Check's own
`diff` of `canon.musa` under two seeds. The header is written only when the piece actually decided something: every
realization produces the determinate file, so naming one would be a claim the file does not need.

**A backwards range is refused rather than compiled.** `repeat 6 to 2` is a mistake about the music; treating it as zero
passes would hide it in silence.

**`musa_kernel::print` takes note lines; `musa_kernel::notes` reads them.** The Design says `.kernel` needs no grammar
change and that is true, but "written and read" still needs an API, and the kernel must not learn what a realization is
in order to carry the sentence. A note is an opaque `%` line; the compiler decides what goes in one.

## Stop

- No `choose`, no kernel change, no term form. Prompt 66 settled it.
- No mobile form, no free durations, no improvisation regions — prompt 68. `Decision` has the variants; nothing produces
  them.
- No interface. Prompt 76.
- No random pitch, no random dynamics, no generative anything. The mechanism resolves *written* freedoms; it does not
  invent music.
- No global RNG, no `rand` dependency. FNV-1a and a counter is the whole of it, and it must be reproducible across
  platforms and processes.
