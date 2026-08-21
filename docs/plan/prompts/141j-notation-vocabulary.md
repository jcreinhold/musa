---
id: 141j
slug: notation-vocabulary
status: done
depends_on: [141b, 141d, 141e, 141h]
phase: 3
---

# Give Notation Its Core Words

## Task

Prompt 141h registered the eight track words `BUILTIN_OWNERSHIP` names, and exactly one of them *constructs*: `play`,
which makes `Note` facts out of a `Voicing`. `crate::elaborate::FactKind` has nineteen cases, and the other eighteen —
rest, mark, grace, slur, phrase, tuplet, dynamic, hairpin, key, meter, clef, tempo, section, harmony, repeat, mobile,
improvise, ending — have no core spelling at all. Neither does *sequencing*: a voice puts one fragment after another,
and there is no word for it. Register the vocabulary a notated block is built out of, so that prompt 141k has something
to lower into.

This is a gap rather than a decision. `docs/plan/prompts/README.md` states the kernel's semantic core as "`empty`,
`event`, `follow`, `together`, `map_payloads`, `duration` over exact rational ambient time", and `00-semantics.md` §3
gives the two source equations in the kernel's own words — `use m; use n; ⇒ follow(m, n)` and
`voices m and n ⇒ together(m, n)`. `together` is registered. `follow` and `empty` are not, because in the deleted
contextual-`Music` design sequencing was the evaluator's *cursor* rather than an operation: placement was read from an
ambient context, so nothing ever had to name the act of placing. Deleting that context is prompt 142's job, and it
cannot be done while the replacement has no word.

## Read

- [`../../rules/language/00-semantics.md`](../../rules/language/00-semantics.md) §2's fragment judgment and §3 in full.
  §3 is the specification this prompt supplies the words for: a fragment is a value of type
  `EventTrack[WrittenTime, ScoreFact]`, placement is applied by the enclosing voice's left fold, and composition "is
  therefore the core's own operations, with no separate equation set to maintain". §3 also fixes what a block may *not*
  contain — a key, meter, tempo, or clef change, a part or voice declaration, a profile, an instrument, a mix route, or
  an import — "because 'from here onward' has no unique meaning in a value usable at several places". That sentence
  decides this prompt's visibility question below.
- [`141h`](141h-track-core.md), whose Design argued that the eight track builtins needed a reshape and took it. Its
  `SPELLINGS` array, its `TRACK_BEYOND` array, and the argument attached to the second: `set_note_pitches` is registered
  and is deliberately *not* a source word, because "a source word for it would be the control removed". This prompt has
  three more operations in that position and the same reason.
- [`141e`](141e-compiler-registry.md) — the registry assembled in four stages, `BUILTIN_OWNERSHIP`, and the accounting
  law that counts what is registered against what the ownership tables name. That law is what fails if this prompt adds
  a word and forgets to say where it came from.
- [`141b`](141b-base-types-and-builtins.md) §5.8's four families and the rule that decides declaration from
  registration: a base type is *inert* and opaque, a declared family is finite data with nothing hidden behind it. The
  `Scope` paragraph in `crates/musa-compiler/src/prelude.rs` is the worked example — "finite data with three cases and
  nothing hidden behind them", declared, while an `Origin` beside it is registered.
- [`141d`](141d-finite-constructor-builtins.md) — `Datum::Case`, which is how a δ-rule *reads* a declared family's
  constructor. A rule that takes a `Fact` argument reads it as a `Datum::Case` with its fields in declaration order, and
  writes nothing back: this prompt's constructor answers a track, not a fact.
- `crates/musa-compiler/src/elaborate.rs` — `FactKind`'s nineteen cases and their payload types, and `ScoreFact`'s three
  fields beside the kind (`scope`, `origin`, `tied`). The mirroring rule below is stated against this enum, so read what
  each case actually carries before deciding it is one core case.
- `crates/musa-compiler/src/registry/track.rs` — `builtins`, `PLAY`, `track_type`, `Provenance`, and `scope_of`. The new
  words are written the same way, out of the same helpers, and `play` is not rewritten.
- `crates/musa-compiler/src/chord.rs` — `Voicing` and `VoicingError::Empty`, "a chord with no notes is a rest, not a
  voicing". That is why silence cannot be `play` with an empty voicing and needs a case of its own.
- `crates/musa-kernel`'s facade: `empty`, `track`, `follow`, `together`, `duration`. The rules this prompt registers are
  translations of kernel operations, not new arithmetic, and `follow`'s duration additivity and associativity are
  already proved there (`../../rules/kernel/04-algebraic-laws.md`).

## Design

**Three registrations and one declaration, and each is the smallest thing that lets a block be built.**

```text
Fact                                                        a declared family, nineteen cases
sounded  : Origin → Scope → Fact → Duration ⟨written⟩ → Result (EventTrack ⟨written⟩) Text
follow   : EventTrack ⟨written⟩ → EventTrack ⟨written⟩ → EventTrack ⟨written⟩
nothing  : EventTrack ⟨written⟩
```

**`Fact` is declared, not registered, and it mirrors `FactKind` one for one.** It is finite data whose cases are the
score's own vocabulary; nothing is hidden behind them, and a later analysis pass wants to `match` on one, which a base
type cannot offer. Its fields are the payload types the registry already carries as inert domains — a `PitchClass`, a
`DynamicMark`, a `ChordSymbol`, a `Clef` — and a payload type this prompt finds unregistered is registered here, named
in the Target rather than discovered. **One for one is the rule and it is checkable**: every `FactKind` case has exactly
one `Fact` case with the same name and the same fields in the same order, and conversely. State it as a law that walks
both, so that a nineteenth kind added later fails a test rather than silently becoming unwritable notation.

**`sounded` is the constructor, and `play` stays exactly as it is.** They are not two spellings of one operation: `play`
takes a `Voicing` and answers a track of *several* simultaneous `Note` facts, which is what a chord is, while `sounded`
puts *one* fact over `[0, held]`. Prompt 141e wrote `play` against §5.7's construction clause and prompt 142's Stop
forbids re-translating what it wrote; a `sounded` that tried to absorb it would be that re-translation. Both take an
`Origin` and a `Scope` for the same reason — §5.7 requires every constructed fact to carry both, and neither is
something a `fn` pointer can invent.

**All three are beyond the ownership tables, and that is a visibility decision rather than an omission.** `00-semantics`
§3 says a block may not contain a key, meter, tempo, or clef change; a source word for `sounded` would hand every
program the ability to put a `Key` fact in the middle of a voice, and the rule that forbids it would become a check
somewhere instead of a property of the vocabulary. That is `set_note_pitches`'s argument one domain over, and the same
`TRACK_BEYOND` list records it. `follow` and `nothing` are beyond the tables for a weaker reason and should say so:
`use` is the source spelling of sequencing (`01-surface.md` §2) and `01-surface.md` §3's worked programs write
`together`, `shift`, and `map_note_pitches` and never write `follow`, so giving sequencing a second source spelling is a
surface change with its own evidence, and surface changes are not this prompt's.

**Sequencing is a word rather than an encoding.** `follow(m, n)` is expressible as `together(m, shift(duration(m), n))`,
and that expression is the reason to register `follow` rather than the reason not to: it needs `duration`, and a
`duration : EventTrack ⟨written⟩ → Duration ⟨written⟩` would hand the source a query it has never had — a program could
branch on how long a fragment is, which makes a track's extent part of every caller's control flow. `follow` is the
kernel's own word, `musa_kernel::follow` already exists and is already proved associative and duration-additive, and
registering it costs one rule that calls it.

**`nothing` is `follow`'s identity and it is a value, not a rule.** An empty `music { }` block, a voice with no
statements, and the seed of the left fold are all the same track: duration zero, no occurrences. It is
`musa_kernel::empty` and nothing more.

*Repaired during implementation.* This paragraph asked for `Builtin::constructor` — 141ha's shape, "whose reduction
shape is none at all" — or a zero-argument δ-rule, and neither works, for the same reason in two spellings. A δ-rule
fires "at the moment the last argument arrives" (`02-core-calculus.md` §5.8), so a rule of no arguments never fires; a
`Builtin::constructor` has no reduction by design, so `follow(nothing, t)` would hand `follow`'s rule a rigid neutral
where it expects canonical data and the spine would block forever. Both shapes say *operation*, and `nothing` is not
one. Nor is it a `Definition` like `run_syntax_step`, which needs one only because a λ has no inferable type: a literal
at a base type carries its own, so a `Definition` here would write a type the value already states. So `nothing` is a
**literal** at `EventTrack ⟨written⟩` that prompt 141k's lowering embeds directly, and the accounting law counts two
registrations here rather than three.

**Agreement is the check, as it was in 141e.** Each new rule is sampled against the code that builds the same fact in
`elaborate.rs` today, by a second hand-written path, so a broken encoding cannot make agreement pass. `follow`'s law is
the kernel's: durations add, and `follow(nothing, t)` and `follow(t, nothing)` are `t`.

**Rejected: nineteen constructor builtins, one per fact kind.** It would make each notation statement one call rather
than a call with a constructed argument, and it would put nineteen entries in a table whose accounting law is the thing
that keeps the registry honest. The `Fact` family says the same thing once, in the language's own data, and a `match`
over it is available to the analyses of `07-analysis.md` rather than being a shape only this compiler can read.

**Rejected: a `Fact`-valued builtin per kind and a `sounded` that takes a base-typed `Fact`.** Registering `Fact` as an
inert base type would make it unmatched and unmatchable, which is exactly the "opaque values" 141b's rule refuses when
the data is finite. `Scope` settled this question already and this is the same answer.

## Target

- `crates/musa-compiler/src/prelude.rs`: `Fact` declared, one case per `FactKind` case, doc-commented with the mirroring
  rule and with what each field's payload type is.
- `crates/musa-compiler/src/registry/`: `sounded` and `follow` registered against the types above and `nothing` written
  as a literal at the same track type, in `track.rs` beside the eight or in a `notation.rs` next to it, whichever leaves
  `track.rs` a module that can still be named in one sentence.
- `TRACK_BEYOND` (or its equivalent) naming all three operations that no ownership table names, each with the sentence
  that says why it has no source word.
- Any payload type `Fact` needs and the registry lacks, registered as an inert base type and counted.
- The accounting law extended: registered builtins are exactly the ownership tables' entries plus the named
  beyond-the-table set, and the `Fact`/`FactKind` mirroring law walking both directions.
- Agreement laws: one per new rule, against the fact construction in `elaborate.rs`, plus `follow`'s two identities and
  its duration additivity.
- `docs/plan/code-map/spec-to-implementation-map.md`: the `musa-compiler` registry row records the four beyond-the-table
  operations and the declared `Fact`.

## Check

```sh
cargo build --workspace
cargo nextest run -p musa-calculus -p musa-compiler
cargo clippy --all-targets -p musa-calculus -p musa-compiler -- -D warnings
cargo fmt --check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
python3 scripts/renumber-prompts.py audit
```

Commit as `Give notation its core words`.

## Stop

- No lowering. Nothing here reads a `.musa` node; prompt 141k is the reading and these laws are this prompt's only
  caller, exactly as 141e, 141h, and 141ha leave their registrations.
- No source word for any of the four. That is a surface change, and `01-surface.md` is not amended by an implementation
  prompt.
- No `duration` builtin, and no query that answers a property of a track. The argument above is the whole reason
  `follow` exists as a word.
- No change to `play`, to the eight, or to any signature 141e already wrote. A disagreement with one of them is a defect
  in that one and is repaired where it is.
- No readback, no wiring, no deletion of the old evaluator's fact construction — the agreement laws need it, and 142
  deletes it.
- No new `FactKind` case, and no reshaping of an existing one. If the mirroring law proves a case unwritable in the
  core, that is a finding to record, and repairing it is a repair of this prompt rather than a widening of it.
