# The events vocabulary amendment: retiring *kernel*

**Status: record.** Governs nothing. This page keeps visible the argument for retiring the word *kernel*, so that a
later reader who finds `musa-events`, `docs/rules/events/`, `kernel T { … }`, or `% musa-events-3` in this directory's
earlier pages, in the prompt records, or in a stored file knows why they no longer match `docs/rules/`.

Documents 00–22 in this directory, and `docs/notes/research/kernel-hypothesis/`, are **not** edited to match it. They
are the record of how the calculus was chosen, and rewriting them would falsify that record. The mapping in §5 is how to
read them.

## 1. What prompted it

Nothing about the calculus. The name was tested against Ousterhout, *A Philosophy of Software Design*, ch. 14, whose
consistency test has three parts: always use the common name for a given purpose, never use that name for anything else,
and keep the purpose narrow enough that all uses behave the same. *Kernel* failed the second part five times over. In
one workspace it named:

1. the crate `musa-events`;
2. the governing directory `docs/rules/events/`;
3. the surface-language keyword that opens a core quotation, `events EventTrack[WrittenTime, ScoreFact] { … }`;
4. the interchange file format — the `.musa.events` extension and the `% musa-events-3` marker;
5. the CLI subcommand `musa kernel`;

and, in prose, 6. the value itself — 258 uses of "the event track" across `docs/rules/` and `docs/plan/`.

Only the sixth is a thing. The other five are places the word had leaked to, and each one taught a reader that *kernel*
was the name of something slightly different from what the last one meant.

## 2. The constitution already spoke the right vocabulary

The decisive evidence is that this drift never reached the top of the ladder. `constitution.md` and `obligations.md` do
not contain the word at all. They say **event track**, and `§3` and `§4` call it one of the two core values. The type is
`EventTrack`, the basis constructor is `event`, and `00-purpose.md` is titled "Purpose of the Event-Track Core".

That same file had already confessed the defect, in its sixth line:

> The directory is still called `kernel/` because that is what the crate is called and what forty prompts of citations
> point at.

A name kept because of what points at it, rather than because of what it describes, is the "Hard to Pick Name" red flag
(ch. 14.6) recorded in the specification itself. So this amendment does not introduce a vocabulary; it finishes
propagating the one the constitution has used since prompt 12.

## 3. *Kernel* was also the third "core"

The second cost was collision rather than leakage. By the time of this amendment the workspace had three things a reader
could reasonably call the core:

- the **event track** — finite, exact, temporal, `docs/rules/events/`;
- the **machine** — the other core value, `docs/rules/across-stages/03-machine-calculus.md`;
- the **term core** — the dependently typed calculus in `musa-calculus`, reached through `musa_compiler::core`.

*Kernel* and *core* were being used interchangeably for all three. Naming the first one after what it actually holds
removes it from the contest, and leaves the ambiguity of *core* confined to the two that the constitution genuinely
declares to be core values.

## 4. Why `events`, and what it was chosen against

- **`time`** — rejected. It names the coordinate, not the value; an event track is a finite multiset of occurrences
  *indexed by* exact time. It would also collide with `mod time` inside the very crate it renamed, which is the
  crate-shadows-its-own-module defect.
- **`track`** — rejected. `AGENTS.md`'s layer-separation law reserves the word: *voice ≠ mixer track*. It is also
  already a reserved keyword in this grammar (`docs/rules/events/01-grammar.md` §2), so the quotation keyword could not
  have been spelled that way.
- **`core`** — rejected for the reason in §3, and because `musa_compiler::core` already means the term core.
- **`events`** — chosen. It is the word the type, the basis constructor, and the constitution already use, and nothing
  else in the workspace claims it.

## 5. The replacement rule, in plain language

The value is an **event track**. Where a document needs an adjective it is **event-track** (an event-track term, the
event-track core). Where a name must qualify the interchange language, the quotation form, or the crate, the qualifier
is **events**. *Kernel* is retired, except when quoting outside work — Lean 4's kernel keeps its own name.

| Was | Is |
| --- | --- |
| `musa-events`, `musa_events` | `musa-events`, `musa_events` |
| `docs/rules/events/` | `docs/rules/events/` |
| `events EventTrack[…] { … }` | `events EventTrack[…] { … }` |
| `.musa.events`, `% musa-events-3` | `.musa.events`, `% musa-events-3` |
| `musa kernel` | `musa events` |
| `examples/events/`, `examples/events-splice.musa` | `examples/events/`, `examples/events-splice.musa` |
| `EventsError`, `EventsTokenClass`, `EventsQuote`, `EventsSplice`, `EventsHole` | `EventsError`, `EventsTokenClass`, `EventsQuote`, `EventsSplice`, `EventsHole` |
| `EVENTS_MARKER`, `events_classify`, `events_bindings`, `events_keyword_doc`, `check_events` | `EVENTS_MARKER`, `events_classify`, `events_bindings`, `events_keyword_doc`, `check_events` |
| "the event track", "the event-track" | "the event track" |

## 6. Which current examples no longer work

Every source that spells the quotation keyword or the interchange format:

- `examples/events-splice.musa`, whose `events EventTrack[WrittenTime, ScoreFact] { … }` no longer lexes;
- the committed corpus `examples/events/*.musa.events` — ten files, each beginning `% musa-events-3`;
- every `.musa.events` a user has exported, for the same reason;
- the `musa kernel` invocations in `docs/book/`.

All of them are renamed and rewritten in the same commit that changes the lexer, which is what keeps `examples/` an
executable specification rather than a set of demos.

## 7. How stored files and public APIs migrate

**Clean break, no alias.** `docs/plan/clean-break-ledger.md` is the standing policy, and it is the right one here for a
reason particular to this change: a lexer that still accepted `kernel` as a deprecated spelling of `events` would be two
names for one thing, which is the entire defect being repaired. A file beginning `% musa-events-3` is now rejected with
the diagnostic that already exists for an unrecognized marker, and the version counter is advanced to `musa-events-3`
rather than restarted, so that an old file is recognizably *older* and not merely foreign.

The public Rust surface changes by rename only — no signature, no behaviour, and no denotation moves. `musa-events`
exports exactly what `musa-events` exported.

## 8. What the six requirements are answered against

`constitution.md` and `obligations.md` are **not** amended by this change; they never used the word. Under
`docs/rules/README.md` the pages that do change are amendable in the ordinary way — a repair committed before the code.
The six requirements are answered here anyway, because the change reaches the surface language and a reader a year from
now should not have to reconstruct why: the reason is §1–§3, the broken examples are §6, the replacement rule is §5, the
specification and code map are updated in the same commit, the migration is §7, and this file is the record.
