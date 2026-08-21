---
id: 48
slug: events-interop
status: done
depends_on: [47]
phase: 3
---

# The Kernel as an Interchange Format

## Task

Give the term calculus its text form and a second producer and consumer, which is the condition Q6 set for building a
parser at all: `musa events <file.musa>` prints a piece as events text, `musa events --check <file.events>` reads it
back, and a corpus of `.event track` fixtures becomes an executable specification that a future second implementation
could be tested against. Round-tripping is the acceptance test: print, parse, evaluate, and get the same canonical form
and the same semantic hash.

This graduates `docs/rules/events/10-term-calculus.md` from candidate to governing.

## Read

- `docs/rules/events/01-grammar.md` (the text form, repaired at prompt 46), `10-term-calculus.md`, `05-normalization.md`
  N5–N6 (canonical serialization and hash — the printer's output for a *normalized* term must remain exactly N5's, which
  is what today's goldens contain).
- `docs/rules/events/08-open-questions.md` Q6 — quote its condition in the prompt's commit message: the second
  producer/consumer now exists.
- `crates/musa/src/main.rs` (command dispatch and help text), `crates/musa-project/src/export.rs` (how exports are
  routed today — events text is an export, and should not grow a parallel path).
- `crates/musa-syntax/src/lexer.rs` — read it before writing a lexer. Events text is *not* musa source and must not
  share its lexer; the question to answer explicitly is whether anything is genuinely shared (rational literal parsing,
  probably) or whether sharing would couple two languages that change independently.

## Design

### Two properties, and the layer each belongs to

- **Printing a normalized term is N5**, byte-identical to what `events_normal_form` emits today. Existing goldens do not
  change.
- **Printing an un-normalized term** preserves `let`, `seq`, `over`, `shift`, and `scale` structure. This is the new
  capability, and it is what makes the format worth having: a canon prints as a `let` and two `shift`s rather than as a
  thousand occurrences.

The printer takes a term; normalization stays a separate operation the caller may apply first. Do not add a "normalize
while printing" flag — that complects two decisions the caller can make in sequence.

### Payloads at the boundary

The event track is generic in `A`; a file is not. The format needs a payload syntax, and the honest options are:

- **A** — the format is generic over a payload *text* the event track neither writes nor reads, with the compiler
  supplying a `ScoreFact` printer/parser. The event track stays payload-opaque (§12), which is the invariant this crate
  exists to hold.
- **B** — the event track defines a payload grammar. Simpler files, but the event track now knows what a note is, and
  §12 dies.

Take **A**. It means one trait pair — a payload writer and a payload reader — parameterizing the printer and parser, and
it means `ScoreFact`'s text form is specified in `docs/rules/events/06-surface-elaboration.md` where the payload is
defined, not in the grammar document. The cost is one indirection; the alternative is the event track learning music
theory.

**`Progress` is the payload text form's hardest case, and the reason prompt 45 exists.** A hairpin's shape must survive
the round trip exactly — rational breakpoints, no float anywhere — because a second implementation that reads this file
and guesses the shape produces different sound from the same artifact. That is the difference between an interchange
format and a lossy dump, and it is why the shape had to be in the denotation rather than in `performance.rs`. Give
`Progress` a text form in this prompt's payload grammar, and make one of the round-trip fixtures a multi-segment,
non-dyadic curve so an accidental float conversion fails the test rather than rounding quietly.

State in `07-backend-contract.md` what a conforming consumer owes: it must honour the printed shape, and it chooses its
own sampling (prompt 45's distinction). A file that pins the shape and leaves sampling free is exactly as normative as
this format can honestly be.

`Canonical` already produces a payload key. Check before adding anything whether the canonical key *is* the payload text
— if it is injective and parseable, one function serves both, and N3's injectivity requirement is exactly the round-trip
property. If it is not parseable (today's `format!("{}|{}|…")` may not be), decide deliberately between making it
parseable and having two functions, and record the choice.

### The round-trip law

```text
for every fixture:  parse(print(t)) evaluates to a timeline with
                    the same canonical form and semantic hash as t
```

as a property test over generated terms *and* as a test over every `examples/*.musa` compiled to a term. The second is
the one that catches payload-escaping bugs, because real payloads contain the characters a generator will not think of
(names with spaces, chord symbols, provenance paths).

### The CLI

```sh
musa events <file.musa>                  # print the piece as events text
musa events <file.musa> --normalized     # print its normal form (N5)
musa events --check <file.events>        # parse, check, evaluate; report violations
```

Route it through `musa-project`'s export path like every other target rather than reaching into the compiler from the
CLI. Update the help text; `musa render --to plan` and friends are unchanged.

### The fixture corpus

`examples/events/` holds the printed form of each `examples/*.musa`, committed as goldens, regenerated by the same
`insta` mechanism as the other backends. These are the artifacts a second implementation would be validated against, so
each gets a one-line header comment naming the source fixture and the events spec version. That corpus, not the CLI, is
what Q6 was waiting for.

## Target

- `crates/musa-events/src/text.rs` (new): printer and parser over the payload trait pair; `EventsError` gains parse
  positions.
- `crates/musa-compiler`: `ScoreFact`'s payload text form; the piece-to-term entry point (still `#[doc(hidden)]` until
  prompt 49 makes terms the elaboration output).
- `crates/musa-project/src/export.rs`, `crates/musa/src/main.rs`: the `events` target and help text.
- `examples/events/*.event track`: goldens for every `examples/*.musa`.
- `docs/rules/events/01-grammar.md`: the implemented grammar, candidate banner lifted from `10-term-calculus.md`;
  `08-open-questions.md`: **Q6 resolved**.
- `docs/rules/events/09-pipeline-baseline.md`: a row only if printing lands on a measured path (it should not).

## Check

```sh
cargo nextest run -p musa-events -p musa-compiler -p musa-project
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
for f in examples/*.musa; do cargo run -p musa -- events "$f" > /tmp/k.event track && cargo run -p musa -- event track --check /tmp/k.event track; done
diff <(cargo run -p musa -- event track examples/canon.musa --normalized) examples/events/canon.normal.event track
grep -L "Status: candidate" docs/rules/events/10-term-calculus.md
```

Commit as `Add the events interchange format`.

## Repairs made while implementing

**Two functions, not one — the canonical key cannot be the payload text.** The prompt asked for this to be checked
first. `ScoreFact::canonical_key` is the N3 *equality* serialization and deliberately omits `definition_span` and
`declaration`, because two facts differing only in those are the same fact for ordering, equality, and hashing. That
quotient is what makes the semantic hash semantic, and it is exactly what an interchange form must not do. Making the
key parseable would have meant widening it, which would have moved every golden and every stored hash and made a fact
stop being equal to itself compiled from a reformatted source. So `TextPayload::to_text` is a second function,
`canonical_key` is untouched, and nothing downstream moved. Where a payload has no provenance to quotient — `Progress` —
the two coincide and one function serves, which is the case the prompt predicted.

**"N5 is a strict subset of the grammar" was false, and `--normalized` prints interchange text.** N5 writes the N3 key
bare (unquoted, unparseable) and has no version header; it is not a file. Both `01-grammar.md` and `05-normalization.md`
claimed otherwise and are repaired. `musa events --normalized` therefore prints the *interchange* spelling of the normal
form — a single flat `timeline` inside an events file — which `--check` accepts, where N5 bytes never could.
`events_normal_form` and its snapshots are byte-identical to before, which is the sense in which the prompt's "existing
goldens do not change" held.

**`01-grammar.md`'s payload declaration was struck.** The document specified `payload Note { letter: text; … }` and
record-shaped payload values. That contradicts the prompt's own choice of option A: an event track that reads a payload
record knows what a note is, and §12 dies. A payload is now an opaque quoted string, the type name in `Timeline[<T>]`
exists only so a reader can *refuse* a file it does not own, and `ScoreFact`'s form is specified in
`06-surface-elaboration.md` where the payload lives.

**The corpus is plain files, not insta snapshots.** The prompt asked for "the same insta mechanism as the other
backends". A `.snap` wraps its payload in a YAML preamble, and `examples/events/*.event track` exists to be read *as
event track text* by a second implementation. The test owns the corpus and regenerates it under
`UPDATE_EVENTS_GOLDENS=1`, which gives the same staleness guarantee without making the artifact unreadable.

**The version header is required, not decorative.** `%` begins a comment, so a file missing its header would have parsed
as a valid file of unknown vintage. `parse` checks the first line before the lexer sees it. This is what makes "refuse a
version you do not know" a rule a consumer can actually follow.

**Nothing is shared with the surface lexer.** The prompt asked for the question to be answered explicitly. Events text
has a different comment syntax, different keywords, no pitch or duration literals, and payloads that are opaque strings;
the only overlap is reading `p/q`, which is four lines of `split_once` and `parse`. Sharing it would couple two
languages that change independently for no saving.

**The term a piece prints is an `over` of literals, not yet a `let`.** `elaborate_voice` produces timelines, so that is
honestly what there is to print until prompt 49. The overlay structure does survive printing, and the round-trip law is
testable now rather than after the structure lands.

## Stop

- No binary format, no compression, no schema version negotiation. A version *string* in the header is enough.
- No `.event track` → `.musa` direction, ever. The source is canonical (AGENTS.md); an events file is a projection, and
  reconstructing source from it would create the second editable representation this project forbids.
- No import of events files into pieces. That is not interop, it is a second surface language.
- No editor support, syntax highlighting, or formatter for `.event track`.
