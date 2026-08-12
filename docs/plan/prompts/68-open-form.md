---
id: 68
slug: open-form
status: done
depends_on: [67]
phase: 3
---

# Open Form

## Task

The three remaining kinds of written freedom, on prompt 67's mechanism: a **mobile form** whose sections may be played
in any order, a **free duration** the performer chooses from a range, and an **improvisation region** with a notated
frame and unnotated contents. Each is a `Decision` variant that already exists, a `FactKind` the page can print, and no
kernel change.

The acceptance test is repertoire, not syntax: *In C*, Klavierstück XI, and a jazz chart's solo section must each be
writable, and the corpus is where they are.

## Read

- `docs/rules/kernel/11-realization.md` — the repertoire it names is what this prompt must deliver.
- Prompt 67 — `Decision::{Order, Duration}` exist and have no producer. This prompt is the producer.
- Prompt 58 §"the timeline holds every pass; the page prints the instruction once" — the rule for how a freedom appears
  on the page.
- `docs/rules/kernel/08-open-questions.md` rows 7–10 — the falsification corpus. Adding pieces to it is part of this
  prompt.

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

`fragment` is a named body like a motif — prompt 57 already established the namespace and its diagnostics, so this adds
a `Material` tag rather than a namespace.

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

*In C* is fifty-three figures, each repeated as many times as the player likes, entered when the player likes. It needs
prompt 67's ranged repeat **and** this prompt's fragments, and it needs them per player — which is the case that will
show whether `ChoicePath` is right, because fifty-three fragments in each of several parts is exactly where an
ordinal-based identity would fall over.

Write it. If it cannot be written, this prompt's design is wrong and the repair goes in `11-realization.md` first.

### Mobile form and the ordering decision

`Decision::Order(Vec<u32>)` is a permutation of the fragment list. Drawn per path from the seed by Fisher–Yates over a
per-path derived stream, which is deterministic and needs no `rand`. Klavierstück XI's 19 fragments give 19! orderings —
the number that killed `choose` — and here it costs nineteen `u32`s.

### Free duration and where the *notated* value goes

`g4 1/4 to 2/1` is written as a quarter and may be held to a double whole. The **notated** duration is the minimum; the
**performed** duration is the decision. That is §2's row exactly — notated duration ≠ performed duration — and it is why
this does not need a new time representation. The engraver draws a quarter with a bracket; the performance plan uses the
drawn value.

### Improvisation is a frame, not a hole

`improvise 8/1 over "Dm7 | G7 | Cmaj7"` occupies eight whole notes of real time so everything after it lands where it
should. It sounds as silence, because musa does not improvise, and the chord text is prompt 35's harmony payload, reused
rather than reinvented. A future prompt could sound a comping pattern; this one must not, because "musa invents notes"
is a product decision, not a feature.

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
- `docs/rules/kernel/08-open-questions.md`: rows 7–10 marked proven, or the reason they are not.
- `docs/rules/kernel/11-realization.md`: graduated from candidate to governing.

## Repairs made while implementing

Six places where the design above was wrong or underspecified. Each is repaired here so a later prompt reads the truth
rather than the plan.

**A free duration is a field on the note, not a fact beside it.** The Design table proposes
`FactKind::FreeDuration { min, max }` "on the note". There is no such position: a fact is an occurrence over a span, so
a second occurrence sharing the note's span would have to be matched back to its note by coincidence of extent — and two
free-duration notes at the same onset in different voices would be indistinguishable. The freedom is a property *of* the
note, so it is `free: Option<FreeDuration>` on `FactKind::Note` and `FactKind::Rest`. `Mobile` and `Improvise` remain
facts as designed, because those genuinely are regions rather than properties.

**The stored notated value is the decision, not the minimum.** The Design says "the notated duration is the minimum; the
performed duration is the decision", then says "the performance plan uses the drawn value". Both cannot hold: if the
occurrence spans the minimum, every note after a held note lands too early. What is stored is the decided duration — so
the timeline is right — and `FreeDuration { least, most }` carries the written range alongside it, from which the
engraver draws `least` with a bracket. §2's row is kept, with the *written* value in the payload rather than in the
span.

**A voice is not bar-checked after a free duration.** `resolve.rs::check_measure_sanity` would report every bar after a
held note as mismeasured. How far such a voice reaches is the performance's answer, so the complaint would be about the
freedom rather than about a mistake: a voice with any `free` event is skipped from that check.

**A ranged repeat inside shared material does not warn.** Prompt 67's repeat-agreement check reports a repeat whose
passes disagree, because a disagreeing repeat cannot be one barline. *In C*'s fifty-three figures are exactly that and
are not a defect — shared material stands at places that have nothing to do with each other, so there was never a
system-crossing barline to lose. `project.rs::agreed_repeats` now writes such a repeat out silently when its
`expansion_path` contains a `MotifApplication`. A ranged repeat among a voice's own items still warns.

**`in-c.musa` is one voice, not "fifty-three fragments in each of several parts".** The Design names the several-parts
case as the one that tests `ChoicePath`. It does not: a site written among a voice's own items has its ordinal counted
per voice, so the k-th site in every voice is *one* site by construction — several parts would test nothing that one
part does not. What does test the identity is the fifty-three-way edit, which
`editing_one_figure_leaves_the_other_fifty_two_decisions_alone` checks directly. The example is written as one voice of
fifty-three fragments plus a pulse, and the figures are original rather than Riley's, because *In C* (1964) is in
copyright; the file's header says so.

**`ExportArtifact` became a struct.** A lossy export must say what it dropped, so a rendered artifact has to carry
warnings. The alternative — a second `export_with_warnings` method — is a shallow module, and changing `export`'s return
type touches fourteen call sites. Only `session.rs` constructed the variants, so `ExportArtifact` is now a struct of a
body and its warnings, with `text`/`bytes`/`warn` constructors.

## Check

```sh
cargo nextest run --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
cargo run -p musa -- check examples/in-c.musa
diff <(cargo run -q -p musa -- kernel examples/mobile.musa --seed 7) \
     <(cargo run -q -p musa -- kernel examples/mobile.musa --seed 7)     # stable
cargo run -p musa -- render examples/changes.musa --to musicxml 2>&1 | grep -i "improvis"  # the warning
grep -L "Status: candidate" docs/rules/kernel/11-realization.md
```

Commit as `Add open form`.

## Stop

- No performer-timed entry, no cueing between players, no real-time coordination. *In C*'s players enter "when they
  like"; musa compiles one plausible reading and says so.
- No generated improvisation. Silence and a frame.
- No graphic or text-only scores. A page musa cannot engrave is not a page musa should accept.
- No probability weights on a mobile's orderings.
- No new kernel operation, still.
