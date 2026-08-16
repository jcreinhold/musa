---
id: 141g
slug: raw-lowering
status: in-progress
depends_on: [136a, 137, 139, 140, 141e, 141f]
phase: 3
---

# Read the Surface as a Raw Term

## Task

Write the half of elaboration that knows about `.musa`: a lossless CST in, a [`musa_core::Raw`] out, and a table that
turns a [`musa_core::Origin`] back into a span when a refusal comes home. Nothing is wired to it and nothing is deleted
— the laws beside it are its caller, exactly as 141e's registry and 141f's `run_syntax_step` were theirs until something
called them.

Prompt 142 said this in its own Design and then asked for it in the same commit as the migration:

> Elaborate through `musa-core` first and get the existing corpus passing under the new checker with the old spellings
> still in place; then migrate spellings; then delete. Mixing the three makes every failure ambiguous between "the new
> checker is wrong" and "this file was translated wrong".

An ordering inside one commit is not observable. This prompt makes the first of the three a thing that can be wrong on
its own.

## Read

- [`141e`](141e-compiler-registry.md)'s Design, which is this prompt's argument one prompt earlier and about the other
  table: "a table of 117 builtins is a *translation* with an oracle to check it against, and burying it inside a diff
  that also moves 11,304 lines of `.musa` would have made a wrong signature indistinguishable from a wrong migration."
  Reading the surface is the same kind of thing — a translation, with `musa-core`'s own acceptance as its oracle — and
  it is larger than the builtin table by every measure.
- [`142`](142-surface-cutover.md) in full, whose Target this prompt takes the first bullet of and whose **Stop** list
  governs here too: no compatibility mode, no second elaborator selected by a flag, no new language feature.
- `crates/musa-compiler/src/core.rs`'s `Checker` — every surface form that has to be readable, and `lower_type`,
  `lower_signature`, `check_pattern`, `quote_template`, and `music_expression` in particular. It is what is being
  *replaced*, so it is the list of what must still be readable, not the design to copy: unification, implicit insertion,
  coverage, and scheme instantiation leave with it.
- [`../../rules/language/02-core-calculus.md`](../../rules/language/02-core-calculus.md) §2, and its sentence "A
  projection, a variable, and a literal infer." That decides the one question the surface cannot leave open — see
  Design.
- [`../../rules/language/01-surface.md`](../../rules/language/01-surface.md) §1 and §7 for the forms, and
  [`10-traits.md`](../../rules/language/10-traits.md) §5 and §6 for what `x == y` and `x.m(y)` are spellings *of*.
  `musa_core::Raw`'s own module documentation says the core never learns an operator table; this is the module that
  therefore has to hold one.
- [`141ga`](141ga-quotation-core.md), which owns both quotation forms. They left this prompt on the evidence its own
  implementation produced — see Design — and everything else about reading the surface stayed.
- `musa_core::Refusal`'s variants and `musa_core::PathStep`. A refusal is what a composer will see, so the mapping to
  [`crate::diagnose::Diagnostic`] is part of reading the surface rather than a later polish. Prompt 144 owns *how good*
  the message is; this prompt owns that there is one, at a span.
- The `module-design` skill's audit questions. This module's whole claim is that it is small because the core does the
  hard part; a lowering that grew a type of its own would have failed that claim.

## Design

**One direction, and the core has the other.** Lowering reads, desugars, and numbers. It does not resolve a name to an
index, unify, insert an implicit argument, check coverage, check positivity, or check termination — every one of those
is `musa-core`'s, and a second implementation beside it is the second path `02-core-calculus.md` §5's audit exists to
catch. The visible consequence is that an unknown name is *written through* as a variable rather than refused here: the
core is what holds the context, so the core is what answers `UnknownName`, at the origin this module gave it.

**A literal infers, so the surface node decides its domain.** §2 says so in as many words, and
[`musa_core::RawShape::Lit`] carries the base type it inhabits. So `Integer` is a `Nat`, `Rational` is a `Ratio`, a
`PitchLiteral` in expression position is a `Pitch`, and the pitch-*class* that `01-surface.md` §7 calls "checked in its
expected domain" is read from the form that supplies it — `chord c# minor` is a `ChordExpr`, and the node is what says
`NoteName`, not a type flowing in. Where the old checker read an `Integer` as a `Duration` because a `Duration` was
expected, the new reading is a `Nat` and the conversion is written. **That is a source change, and prompt 142's
migration owns it**; this prompt owns only that the reading is the node's and not a type expectation's, because a
lowering that took an expected type would be checking, and checking is what was moved.

**Origins are a table here, not a span in the core.** `musa-core` is a leaf that must not learn what a file is, and an
[`musa_core::Origin`] is one word so that every term can afford one. `Sites` is the caller's half: it hands out a number
per surface node and answers with the span again when an `ElabError` arrives. A refusal about a *registered signature*
carries `Origin::UNKNOWN` by construction and therefore has nowhere of its own to point; it says so rather than pointing
at node one.

**Declarations lower too, and that is where the module boundary is.** A `data`, `record`, `enum`, `trait`, or `impl`
declaration becomes a `RawData`, `RawTrait`, or `RawImpl`, and a `let` or `fn` becomes a name, a raw type, and a raw
value. Reading a *declaration* and reading an *expression* share the CST walk and the site table and nothing else, so
they are two functions in one module rather than two modules: the information they share is what would have to be passed
between them.

**Quotation is not here, and the reason is a measurement rather than a boundary.** A quote reads as nested
`syntax_group`/`syntax_token` calls over `syntax_built(here, q, i)` paths right up to the form the feature exists for: a
`$..xs` spread in a **separated** position mints one comma between every pair of elements it ends up with, so the
commas' paths are `children.len() + k` for a `k` nothing knows until the adapter runs. Writing that as a lowered term
needs a compiler-generated indexed fold — the `callN` boilerplate prompt 131 abolished, reintroduced one level down —
and it would compute derived identity a second time beside `crate::syntax::build`, which is the second path
`02-core-calculus.md` §5's audit exists to catch. So a template is *data*, its core shape is an inert base type, and
[`141ga`](141ga-quotation-core.md) is where it and both quotation forms land. This prompt reads every other surface
form, and refuses a quote at the node with a diagnostic that says which prompt owns it.

**Laws are the caller, and the oracle is the core.** The property this module can have is that what it writes, the core
accepts — so every law lowers something real and hands it to `musa_core::check` in [`crate::registry::owned`]'s context.
That is checkable now, needs nothing migrated, and is exactly the property the cutover will rely on. Where an answer is
also *computable*, the law normalizes it and compares against the old evaluator, which is still present and is the
second oracle 142 will remove.

## Target

- `crates/musa-compiler/src/lower.rs`: `Sites`, the type lowering, the expression lowering, the pattern lowering, and
  the declaration lowering, each doc-commented with its invariants before its implementation. `pub(crate)` reaches no
  further than the laws and prompt 142 need.
- The literal readers for every base type the surface writes, sharing [`crate::registry`]'s own literal writers so that
  a lowered `Duration<WrittenTime>` and a registered signature's are one expression rather than two that agree today.
- `musa_core::ElabError` restated as a [`crate::diagnose::Diagnostic`] at the span `Sites` numbered, with a code per
  refusal family and the `PathStep` trail as the diagnostic's cause.
- Laws in `crates/musa-compiler/src/lower/laws.rs`: every written base type, constructor, and arrow lowers to a type the
  core accepts; every surface expression form lowers to a term the core checks at its written type; a pattern column
  reaches `match` in the order the arm wrote it; a refusal points at the node that caused it. Beside the module rather
  than in `tests/suite/` for 141e's reason — `Raw`, `Sites`, and `registry::owned` are all private to this crate.
- `docs/plan/code-map/` rows for `musa-compiler`.
- Prompt 142 repaired: its first Target bullet becomes wiring what this built, its Read cites this prompt, and its
  `depends_on` names it.
- Prompt 141ga written, on the evidence above, and cited from this prompt's Design and from 142.

## Check

```sh
cargo build --workspace
cargo nextest run -p musa-core -p musa-compiler
cargo clippy --all-targets -p musa-core -p musa-compiler -- -D warnings
cargo fmt --check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
python3 scripts/renumber-prompts.py audit
```

Commit as `Read the surface as a raw term`.

## Stop

- Nothing is wired and nothing is deleted. `check_piece`, `check_material`, `infer.rs`, and every checking path in
  `core.rs` are untouched, and the corpus still goes through them. Prompt 142 is the cutover and this is not it.
- No `.musa` file changes, in `stdlib/`, `examples/`, or any fixture corpus. A reading that needs the source migrated to
  be worth having is a reading with no oracle.
- No second checker. If a form seems to need an expected type to lower, that is a finding about the form and a repair,
  not a parameter to thread — the one exception the core itself names is `Annot`, which is a raw term and not a
  mechanism.
- No new language feature and no grammar change. `musa-language` parses what it parses; a form it does not admit is
  prompt 142's problem or nobody's.
- No track or machine spelling. `EventTrack` and `Machine` have no core shape until 141h gives them one, and lowering a
  word that denotes nothing would be work thrown away.
- No quotation. `QuoteExpr` and `QuotePattern` are refused at the node with the prompt that owns them named; 141ga is
  where a template becomes a core value, and half a quotation is worse than none.
