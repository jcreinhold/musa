---
id: 65
slug: key-and-clef-changes
status: done
depends_on: [63, 64]
phase: 3
---

# Modulation and Clef Change

## Task

A piece can modulate, and a part can change clef. Both are one-line grammar additions on top of prompt 63's
machinery — but each carries a real decision the other does not, which is why they share a prompt rather than being
folded into 64: key is the kind with `Latest` inheritance, and clef is the kind that changes **mid-measure** and
whose spelling the MIDI speller and the engraver both read.

## Read

- Prompt 63 §"Inheritance is per kind" — the table, and the viola counterexample that motivates `Latest` for key.
- `crates/musa-project/src/session.rs` (:387) — `self.valid.score.key()`, the live wrong answer this fixes.
- `crates/musa-compiler/src/spell.rs` and `apps/musa-desktop/ui/src/lib/session/generated/spellings.json` — spelling
  is keyed on the key, and a modulation changes it mid-piece.
- `crates/musa-compiler/src/resolve.rs::part_metadata` (:701) and prompt 63's duplicate-declaration diagnostic,
  which this prompt turns from an error into a legal second declaration when it carries a position.

## Design

### The grammar

```musa
key d minor;          // at the top of a voice, or anywhere in it
clef bass;            // likewise
```

Both are voice items, both take effect where written, exactly like prompt 64's `meter`. The header forms stay and
mean "from the beginning".

### Key: `Latest`, and what that means concretely

A key fact in any enclosing scope is a candidate; the one with the greatest start time not after the point in
question wins, regardless of which scope wrote it. So a part that opens in D minor against a piece in F major keeps
D minor; when the piece modulates to A major at bar 60, the part follows — because the piece's fact is *later*, not
because it is *outer*.

That is the rule the viola case demands, and the reason it cannot be "innermost wins". Write the test as the
counterexample: three facts, two scopes, and an assertion at four times.

### Clef: `Override`, and it may change mid-measure

A part's clef is the part's business, so no piece-level clef reaches it — but unlike key and meter, a clef change is
*ordinary* mid-measure and every backend supports it there. Do not add a barline check; the engraver places the
small clef before the note it affects, which is what `plan.rs` needs to learn.

This is the only one of the four context kinds whose change is not a barline event, and stating that here is what
stops prompt 74 from assuming they all are.

### The MIDI speller stops being wrong

`session.rs` spells an incoming MIDI note against the piece's key. With a `ContextTrack` it spells against the key
in force **at the caret** — `key_at(caret)`, one call. This is the prompt where prompt 63's "two real callers"
claim is paid, and it is a behaviour fix a user can feel: entering a C♯ after a modulation to D major currently
produces D♭ in a piece whose header says F.

### The exporters

| | Key | Clef |
| --- | --- | --- |
| MEI | `<keySig>` in a mid-piece `<scoreDef>` | `<clef>` in `<staffDef>`, or inline before the note |
| LilyPond | `\key d \minor` | `\clef bass` |
| MusicXML | `<key>` in `<attributes>` | `<clef>` in `<attributes>`, or mid-measure |
| MIDI | key-signature meta event | nothing — MIDI has no clef, and that is not a loss |

### Cautionary accidentals are not this prompt

A modulation raises the question of whether the engraver prints a courtesy natural at the change. Verovio does what
it does; musa does not second-guess it here. If it turns out to be wrong, it is an engraving prompt with a
screenshot golden, not a semantics prompt.

## Target

- `crates/musa-language`: `key` and `clef` as voice items; recovery; the formatter's paragraph-break rule.
- `crates/musa-compiler`: `FactKind::Key` and `FactKind::Clef` produced at position; the `Latest` and `Override`
  resolutions from prompt 63's table, each with its counterexample test; spelling reads `key_at`.
- `crates/musa-render`: the seven exporter rows above; `plan.rs` places a mid-measure clef.
- `crates/musa-project/src/session.rs`: the speller reads the key at the caret.
- `examples/`: `modulation.musa` — one piece, three keys, and a viola part that starts in a fourth and follows the
  third; `clef-change.musa` — a cello line crossing into treble.
- `apps/musa-desktop`: the inspector shows the key and clef **in force at the selection**, not the piece's.

## Check

```sh
cargo nextest run --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cd apps/musa-desktop/ui && npm test
cargo run -p musa-cli -- check examples/modulation.musa
cargo run -p musa-cli -- check examples/clef-change.musa
cargo run -p musa-cli -- render examples/modulation.musa --to lilypond -o - | grep -c '\\key'   # 3 or more
cargo run -p musa-cli -- render examples/clef-change.musa --to musicxml -o - | grep -c '<clef'  # 2 or more
```

Commit as `Let the key and the clef change mid-piece`.

## Repairs made while implementing

**No new `SyntaxKind` and no new statement, again.** `KeyStmt` and `ClefStmt` already existed — the header writes one
and the part writes the other — so `key` and `clef` in a voice are the *same nodes in a different place*: two arms in
`voice_items`, two entries in `VOICE_RECOVERY`, two `VoiceItem` variants. As at prompt 64, a second kind meaning "key,
but over here" would have been a second spelling of one idea.

**No formatter rule.** Same finding as prompt 64: the formatter preserves the blank lines the composer wrote and never
inserts one, for sections or for anything else. Both examples format to canonical form untouched.

**An unnamed `bar { … }` is not material, and this prompt is what forced the distinction.** Prompt 64 drew one line —
a context statement is legal among a voice's own items and nowhere else — and swept `repeat`, `slur`, `tuplet`, motif
bodies *and* `bar` bodies onto the far side of it. But a clef change is mid-measure by definition, and a measure is a
`bar`, so that line made the prompt's central feature inexpressible. The real rule is reachability, not syntax: a body
reached exactly once, from the voice, through single-use containers only, still *is* that place. An unnamed bar is such
a container; a named bar can be answered from another voice, so it is material like a motif. Prompt 64's
`a_change_inside_material_is_refused` was repaired accordingly: it now uses `repeat` for the material case, and the
mid-bar `meter` it used to check is refused by the barline rule instead — which is the rule that actually applies to
it, and a better diagnostic than the one it used to get.

**The viola in a fourth key is not shipped, and the reason is this prompt's own Stop.** The Target asks
`modulation.musa` for "a viola part that starts in a fourth and follows the third". A part carrying a different key
signature from the piece while sounding the written pitches is a **transposing instrument**, which the Stop defers by
name, and there is no part-level `key` statement to write it with. Adding one would have shipped a fixture that means
something musa cannot yet say. The `Latest` rule it was meant to demonstrate is still proved, by the counterexample
test the Target actually asks for (`context.rs::a_key_written_for_a_part_still_follows_the_pieces_modulation`), and
`modulation.musa` is one piece in three keys — which is what a modulation is.

**Where the context is carried in the plan, and why the two kinds differ there too.** `MeasurePlan` gained
`key_signature()` — the key this measure *prints*, `None` when it inherits — exactly mirroring `time_signature()`,
because a modulation is a barline event. The clef is not, so it is `MeasurePlan::clefs()`: a list of
`ClefChange { onset_in_measure, clef }`. It sits on the measure rather than on a `NotatedItem` because a clef belongs
to the **staff** and both voices of a staff read it; putting it on an item would have forced the plan to pick a voice
arbitrarily. Each backend writes it into its first lane, before the first item at or after the onset — the same three
lines three times, which is cheaper than three backends disagreeing about which note the small clef precedes.

**Per-event key and clef in the wire snapshot, rather than a change list the frontend searches.** The inspector must
show what is in force *at the selection*, and finding the latest change at or before a note is reasoning about musical
time, which `03-interaction.md` §7 puts on the core's side of the line. So `EventFacts` carries `key` and `clef`
directly. Measured, not assumed: B2 held at 339 ms of its 400 ms budget (341 ms before), so the extra two strings per
event cost nothing worth trading the layer boundary for.

**The MIDI speller now takes a caret, and the caret had to be plumbed to reach it.** `midi_entry` takes an engraved
event id and spells against `key_at` in that event's **part** at that event's onset. `Job::Midi` grew the caret
alongside the listening flag, `listen_to_midi` grew a parameter, and `App.svelte` re-sends it whenever the caret moves
while entry is on. This is prompt 63's "two real callers" claim paid, and it is the behaviour fix a composer can feel:
a C♯ entered after a modulation to D major used to come out D♭.

**Every `plan__*.snap` moved and none of them changed meaning**, for the same reason as at prompt 64: `MeasurePlan`
gained two fields whose `Debug` rendering is the snapshot. `modulation` and `clef_change` were added to the `LilyPond`,
MEI, `MusicXML` and kernel-golden corpora, because a feature with no golden is a feature nothing defends.

**`cargo insta test --workspace --unreferenced=reject` is not in the Check.** `cargo-insta` is not installed here; the
same repair was recorded at prompts 61, 62 and 64. `cargo nextest run --workspace` runs the same assertions.

## Stop

- No transposing instruments. A part written in E♭ is a real feature and a separate one: it needs written-versus-
  sounding pitch throughout, which is §2's largest unimplemented row and deserves its own prompt.
- No key inference, no modulation analysis, no Roman numerals. Deferred with the theory libraries (README).
- No cautionary or courtesy accidental policy.
- No octave-transposing clefs (`treble_8`) unless a fixture needs one.
- No polytonality — a second key in a *voice* scope at the same time as another is prompt 75's shape, not this
  prompt's.
