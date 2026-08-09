---
id: 86
slug: the-kernel-file-reads
status: done
depends_on: [48, 63, 67, 68]
phase: 3
---

# The Kernel File Reads

## Task

Make a kernel file legible to a person. Prompt 48 made the interchange artifact real and the corpus in
`examples/kernel/` is the deliverable a second implementation is validated against — but its payload label is packed
to the point of being unreadable, so the artifact cannot be checked by eye, and a golden diff cannot be reviewed. This
prompt replaces the label's five-separator packed encoding with a flat, named, quoted token stream, and moves the
corpus to `.musa.kernel` so the extension says whose file it is.

## Read

- `docs/kernel/01-grammar.md` — the interchange syntax. §"Writing is not normalizing" and the naming rule at the foot:
  *"Clear names, no unexplained shorthand (§24): `sequence`/`overlay`/`occurrence`, never `par`/`seq`/`atom`."* That
  rule is what this prompt applies one layer down, and it is what settles the extension.
- `docs/kernel/07-backend-contract.md` §"What a conforming consumer owes" — the three obligations, and the sentence
  after them saying what a consumer does **not** owe: understanding the payload. That is why the payload's spelling is
  free, and why nobody had reason to fix it until now.
- `crates/musa-kernel/src/text.rs` — `TextPayload`, `print`, `parse`, `write_string`. The kernel carries a payload as
  an opaque `"…"`-quoted string escaping `"`, `\` and newline, and never looks inside.
- `crates/musa-compiler/src/factext.rs` — the whole of it. This prompt rewrites it.
- `crates/musa-compiler/src/elaborate.rs` `instantiate` (the `Var`-mark reader) and `mark_of`, whose doc comment
  already concedes the fault: *"a reader counting backslashes is a reader who has stopped reading the music."*

## Design

### Why this is cheaper than it looks

**The label is not hashed.** `Canonical::canonical_key` (`elaborate.rs`) and `TextPayload::to_text` (`factext.rs`) are
two functions for the reason `factext.rs`'s module doc gives: the key is the *equality* serialization and omits the
definition span and the declaration id; the interchange form must reproduce the value exactly and carries them.
`Timeline::semantic_hash` feeds `canonical_key`, never `to_text`. **So this prompt moves zero semantic hashes, zero
normal forms, and zero `insta` snapshots.** The only files that change are the 24 goldens.

**But it is a format change, not a print change.** The label is read back on two paths, and both must move together:
`ScoreFact::from_text`, and `instantiate`, which parses a `Var` mark with `read_step`/`read_span`/`read_scope` on
every `--check`.

### What is actually wrong

Four faults, and the design is four rules that each kill one.

1. **Five separators at five nesting levels** (`|` `@` `;` `:` `,`), none mnemonic.
2. **Positional empty fields you must count.** `tempo@1/4@96@@@@` is seven fields, four of them empty.
3. **Nesting re-escapes.** `join` escapes its separator at every level, so an inner value crossing four levels is
   escaped four times: `examples/kernel/variation.kernel` carried `motif:299\\\\\\\\:311` — eight backslashes for one
   colon.
4. **Redundancy printed in full.** The definition span repeats the source span for every directly authored event; the
   declaration is `0` for every context fact; a duration writes `1/4;1/4;1/4` where one `1/4` says it.

### The form

A label is a **flat whitespace-separated token stream**. Flat is the whole answer to fault 3: there is no nesting, so
nothing is escaped twice. A token is either

- a **bare word** — no whitespace, no `'`, no `\` — used for pitches, ratios, integers, and vocabulary names; or
- a **quoted word** — `'…'` with `\\` and `\'` — used for **every** free-text field, always, even when it would not
  need quoting.

"Always" is the injectivity argument: `mark text '8'` is `Text("8")` and `mark ottava 8` is `Number(8)`, and one rule
keeps them apart without case analysis. Quoting with `'` also means a payload never contains `"`, so the kernel's own
string escape has nothing to double except a literal backslash in composer text — the eight-backslash case becomes
four, and the ordinary case becomes none.

```
label   := scope kind origin
scope   := "piece" | "part" N | "voice" N N
origin  := "[" span ("def" span)? ("#" N)? ("via" step+)? "]"
span    := N ":" N
step    := "motif" span | "repeat" N | "transpose" N N | "stretch" ratio
         | "retrograde" | "invert" quoted | "special" span
```

`[` and `]` delimit themselves, so `[191:198 #4]` is four words without the spaces that would otherwise separate
them — and a bare word therefore contains no bracket either.

`voice 2 11` and not `voice 2.11`, because `a_facts_text_form_writes_no_decimals` asserts a payload contains no `.`
at all — a strictness worth keeping, and a scope index is not worth weakening it for.

Every kind, written out — this table is the specification:

| `FactKind` | Text |
| --- | --- |
| `Note` | `note c4 1/4` · articulations follow as bare words · `free R R` |
| `Rest` | `rest 1/4` · same tail |
| `Mark` | `mark breath` · `mark text 'a note to the player'` · `mark ottava 8` |
| `Grace` | `grace c5 0` · articulations follow |
| `Slur` | `slur` |
| `Phrase` | `phrase 'A'` |
| `Tuplet` | `tuplet 3/2` |
| `Dynamic` | `dynamic sfz` |
| `Hairpin` | `hairpin cres ff 0/1:0/1,1/1:1/1` |
| `Key` | `key c major` |
| `Meter` | `meter 4/4` |
| `Clef` | `clef treble` |
| `Tempo` | `tempo 1/4=96` · `tempo 'Andante'` · `tempo 1/1=60 'rit.' to 30 over 2/1 <shape>` |
| `Section` | `section 'Exposition'` |
| `Harmony` | `harmony 'fmaj7'` |
| `Repeat` | `repeat 4` · `repeat 6 from 4 to 16` |
| `Ending` | `ending 2 pass 3` |
| `Mobile` | `mobile 'a' 'b' order 1 0` |
| `Improvise` | `improvise` · `improvise over 'Dm7 | G7'` |

A duration is `ratio` alone in the common case, and grows only when it must:

```
duration := ratio ("spelled" quoted)? ("tied" ratio+)?
ratio    := N "/" N | N
```

`1/4` for every note whose written value is what it sounds; `1/3 spelled '1/4 ~ 1/12' tied 1/4 1/12` for one that is
not. A whole note is `1`, not `1/1`, which is not cosmetic: the language spells a whole note `1`, and `spelled` elides
exactly when the spelling equals the written ratio, so a `p/q`-only writer would have printed `1/1 spelled '1'` on
every long note in the corpus. A run of ratios ends at the first token that is not one, which is unambiguous because everything that can follow
— an articulation name, `free`, `[` — is not ratio-shaped.

`Progress` keeps its canonical key (`u/d:v/e,…`) unchanged: it is one bare token, it contains no whitespace, and
`read_progress`'s doc comment already argues that for `Progress` the canonical key *is* the text form because it has
no provenance to omit.

### The five elisions, each an iff

Reading reconstructs the value exactly because every elision is a biconditional, not a heuristic:

| Omitted | Exactly when |
| --- | --- |
| `def <span>` | the definition span equals the source span |
| `#<n>` | the declaration id is `0` |
| `via …` | the expansion path is empty |
| `spelled …` | the spelling is the value written `p/q` |
| `tied …` | the pieces are exactly `[value]` |

The origin group itself is never omitted: a span is not optional, so a fact whose source span is `0:0` prints
`[0:0]`. It is also what ends the kind — an articulation run, a mobile's order and a mark's absent argument all stop
at the `[`, which is why no vocabulary word is spelled with a bracket.

### Before and after

```
occurrence "voice@0@0|note@c4@1/4;1/4;1/4@@|191:198|191:198|4|"                  from 0 to 1/4;
occurrence "piece|tempo@1/4@96@@@@|26:50|26:50|0|"                               from 0 to 2;
occurrence "voice@0@0|note@g4@1/4;1/4;1/4@@|299:311|97:104|4|motif:299\\\\\\\\:311" from 0 to 1/4;
```

```
occurrence "voice 0 0 note c4 1/4 [191:198 #4]"                              from 0 to 1/4;
occurrence "piece tempo 1/4=96 [26:50]"                                      from 0 to 2;
occurrence "voice 0 0 note g4 1/4 [299:311 def 97:104 #4 via motif 299:311]" from 0 to 1/4;
```

### What does not change

Scope indices stay indices: `Scope::Voice { part, voice }` carries no names and `to_text` has no score to look them up
in. The `4294967295` shared-binding placeholder stays as it is (`docs/kernel/06`). Occurrence order stays construction
order — within a voice that is already time order, which reads better than N2's global sort, and N2 is what
`--normalized` is for.

### The extension

`.kernel` says nothing about whose file it is, and collides with a word every operating system and compiler already
owns. The repo has two precedents for a suffix musa writes beside a piece, and they agree with each other:
`sonata.musa` → `sonata.musa.recovery` (`autosave.rs`) and `sonata.musa` → `sonata.musa.performance`
(`realization.rs`). A kernel file is the third of exactly that kind, so it is spelled the same way: **`.musa.kernel`**.

`invention.kernel` → `invention.musa.kernel`; the variant marker stays ahead of the suffix, so `canon.normal.kernel`
→ `canon.normal.musa.kernel`.

An abbreviation was considered and rejected on the format's own rule quoted above: `.musak` is a portmanteau rather
than a word, which is the "unexplained shorthand" `01-grammar.md` names; and `.kern` is Humdrum's, a real music format
this one should not be confused with.

**Only the extension moves.** `kernel` also names the grammar's leading keyword (`kernel "Name" {`), the crate, the
CLI verb, and `docs/kernel/`. None of those changes, and a blanket substitution would break the format.

## Target

- `crates/musa-compiler/src/factext.rs`: `to_text`/`from_text` rewritten around one `Words` type that is a writer
  when built up and a reader when split. `join`/`escape`/`split_escaped` go, and with them the six `pub(crate)`
  fragments `elaborate.rs` used to assemble a mark out of — `scope_text`, `span_text`, `step_text`, `read_scope`,
  `read_span`, `read_step`. In their place, two items that say what the caller actually wants:
  `reference_mark(&ReferenceMark) -> String` and `read_reference_mark(&str) -> Option<ReferenceMark>`. Six pieces of
  a format become one named thing, which is the point of putting the format in one module.
- `crates/musa-compiler/src/elaborate.rs`: `mark_of` and `instantiate` become calls to those two, and `mark_of`'s
  doc comment loses the apology.
- `crates/musa-project/src/export.rs`: `ExportRequest::extension()` returns `musa.kernel` — the one authoritative
  spelling; the CLI's `write_artifact` derives every path from it.
- `crates/musa-cli/src/main.rs`: the two usage lines naming `<file.kernel>`.
- `examples/kernel/*.kernel` → `*.musa.kernel`, 24 files, regenerated content.
- `crates/musa-compiler/tests/kernel_interop.rs`: the corpus path, the `format!` that builds a golden's name, the
  `canon.normal` literal, and the two assertions that pin a packed spelling.
- `docs/kernel/06-surface-elaboration.md`: the payload EBNF and its two worked examples.
- `docs/kernel/{01,07,08,09}.md`: the extension, wherever the corpus is named.
- `Makefile`: `make snapshots` sets `UPDATE_KERNEL_GOLDENS=1`, which it does not today — the one command that claims
  to regenerate every golden must actually do so.
- `.gitignore`: `*.performance` beside `*.recovery`.

## Check

```sh
cargo nextest run --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo run -p musa-cli -- kernel examples/invention.musa
```

The laws that must stay green, unchanged in meaning: `a_facts_text_form_round_trips` — keep its corpus and keep its
inversion axis written with every character the format separates on, adapted to the new set;
`a_facts_text_form_writes_no_decimals` — restated over *bare* words rather than the whole line, which is what it was
always about: a tempo marking is allowed to be the word `rit.`, and quoting free text is exactly what lets a law about
numbers stay a law about numbers; `facts_differing_only_in_provenance_have_different_text`;
`printing_and_parsing_an_example_preserves_its_meaning`; `the_kernel_corpus_is_up_to_date` over `*.musa.kernel`.

Two new laws:

- **A label survives the kernel's own quoting.** For every fact in the corpus, `print` a one-occurrence timeline and
  `parse` it back — the property the unit round trip does not cover, because it is the composition of the payload's
  escape with `write_string`'s that produced the eight backslashes.
- **A label has no `"` in it.** That is what keeps the composition from doubling, and it is a property of the writer
  rather than a coincidence of the corpus.

By eye, which is the point of the prompt: `examples/kernel/invention.musa.kernel` can be read aloud.

## Stop

- **No change to `canonical_key`, `Timeline::normalize`, the semantic hash, or normal form.** This is the text form
  only, and the evidence that they are separable is that no `.snap` moves.
- **No `.musa.kernel` → `.musa` direction, and no editor support, highlighting or formatter for kernel files**
  (prompt 48's Stop stands).
- **No payload grammar in `musa-kernel`.** The kernel stays generic in `A` and keeps handing an opaque string to the
  caller's `TextPayload`; a kernel that knew what a note was would be the §12 violation the crate exists to prevent.
- **No renaming of the `kernel` keyword, the `musa-kernel` crate, the `musa kernel` subcommand, or `docs/kernel/`.**
- **No sorting of occurrences.** `--normalized` is where canonical order lives.
- **No new `FactKind`, no new `ExpansionStep`, and no change to what a fact carries.** Same values, spelled so they
  can be read.
