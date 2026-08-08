---
id: 65
slug: key-and-clef-changes
status: pending
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
cargo insta test --workspace --unreferenced=reject
cd apps/musa-desktop/ui && npm test
cargo run -p musa-cli -- render examples/modulation.musa --to lilypond | grep -c '\\key'   # 3 or more
cargo run -p musa-cli -- render examples/clef-change.musa --to musicxml | grep -c '<clef'  # 2 or more
```

Commit as `Let the key and the clef change mid-piece`.

## Stop

- No transposing instruments. A part written in E♭ is a real feature and a separate one: it needs written-versus-
  sounding pitch throughout, which is §2's largest unimplemented row and deserves its own prompt.
- No key inference, no modulation analysis, no Roman numerals. Deferred with the theory libraries (README).
- No cautionary or courtesy accidental policy.
- No octave-transposing clefs (`treble_8`) unless a fixture needs one.
- No polytonality — a second key in a *voice* scope at the same time as another is prompt 75's shape, not this
  prompt's.
