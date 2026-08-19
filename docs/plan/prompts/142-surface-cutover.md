---
id: 142
slug: surface-cutover
status: done
depends_on: [136a, 141, 141b, 141c, 141d, 141e, 141f, 141g, 141ga, 141h, 141ha, 141i, 141j, 141k, 141l, 141m, 141n, 141o, 141p, 141r, 141s]
phase: 3
---

# Move the Whole Language Over, Once

## Task

Close the seam. Wire `musa-compiler` to `musa-core`, migrate `stdlib/`, `examples/`, and every fixture corpus onto the
new language, delete contextual `Music` and the rank-1 Hindley–Milner checker along with every superseded checking path,
and discharge `docs/plan/clean-break-ledger.md`. This is the one migration, and it is green at the end rather than in
the middle. It absorbs prompt 127e, whose file stays as a superseded record rather than being repaired.

## Read

- `docs/plan/clean-break-ledger.md` in full — every source spelling, Rust API, serialized form, fixture, and test name
  it names as deleted rather than aliased. This prompt discharges the rows it owns and leaves the machine rows to
  150–153.
- The superseded prompt [127e](127e-source-language-clean-break.md), whose whole Task this prompt absorbs. Everything it
  specifies is this prompt's obligation: contextual `Music` goes, notation becomes ordinary values, and placement comes
  from the enclosing voice's left fold. Absorbing it was right because migrating `stdlib/` twice — once onto the old
  core's replacement, once onto the new language — would have been the same files rewritten for two different reasons
  within ten prompts.
- [`141b`](141b-base-types-and-builtins.md), which gives `musa-core` the base types, literals, and builtin registry
  without which no real Musa program can be elaborated by it.
- [`141e`](141e-compiler-registry.md), which *filled* that registry: the compiler's prelude declarations, the inert
  musical domains as base types, the 92 δ builtins and 14 δ phase builders as `musa_core::Builtin`s, and the sampled
  agreement with the old evaluator that makes them a translation rather than a rewrite. It also names the 28 entries it
  left and why — which is where this prompt's own list of what still has no core spelling comes from. Its accounting law
  is what will fail if this prompt forgets one.
- [`141f`](141f-phase-traversals.md), which registers the two phase traversals and turns the sealed step into a declared
  family with a private constructor. `staff.musa` calls `recurse_syntax` four times and does not compile without it, and
  `run_syntax_step` is a definition here rather than a builtin, so the migration writes `step.run(context)` where the
  old source wrote a call.
- [`141g`](141g-raw-lowering.md), which built the reading half of this prompt's first Target bullet — surface CST to
  `musa_core::Raw`, with the site table that points a refusal back at a span, and laws rather than a caller. It exists
  because this prompt's own Design asks for the three stages in order and an ordering inside one commit is not
  observable; the argument is 141e's, one level up. What is left here is the *wiring*: the passes that call it, the
  readback out of normal forms, and the deletion of what it replaces.
- [`141ga`](141ga-quotation-core.md), which gave a template an inert core shape and moved both quotation forms onto
  δ-rules that call `crate::syntax::instantiate` and `crate::syntax::matched` rather than a second copy of either. It
  exists because 141g's own implementation proved that building a quote out of the phase builders cannot express a
  spread in a separated position without a compiler-generated indexed fold. What is left here is deleting
  `ExprKind::SyntaxQuote` and the checker's own copy of the body walk — and **§1's forgetting rule**, which 141ga's
  pattern side needs and which every phase builder already needed: each of them reads at `Syntax ⟨tokentree⟩`, the core
  has no subtyping, and a `Syntax ⟨expr⟩` in hand is therefore a value with nowhere to go until this prompt says how a
  category is forgotten.
- [`141h`](141h-track-core.md), which gave `EventTrack` its core shape and registered the eight track builtins 141e
  left, on the grounds that they needed a reshape this prompt would perform. The reshape moved there; what stays here is
  deleting contextual `Music` and teaching the source the word.
- [`141ha`](141ha-machine-core.md), which gave `Machine` its core shape and registered §2's eight *grammatical* forms as
  constructors. It split off 141h because a machine reduces to nothing and needs a unit and a product that 141h's track
  does not. Two things stay here rather than there. **`primitive` is unregistered**, and deliberately: §1's ninth row is
  typed by the build-local registry — the written name and version select a descriptor that supplies the step, both
  ports, *and the configuration argument's type* — so it is a different type per registered pair rather than one Π short
  of writable, and only a stage that can perform that lookup can type it. This prompt is that stage; it is counted in
  `registry/rules.rs`'s `UNREGISTERED` until then. **Storability is unchecked**, for the same reason:
  `02-core-calculus.md` §1.2 states it as a constraint, a `Builtin`'s type is a `Term` with no constraint binder, and so
  `Machine ⟨step⟩ (Nat → Nat) Nat` is writable in the core today. Discharging it belongs to the elaboration here that
  has the constraint solver, with `03-machine-calculus.md` §5's preparation refusing what survives. Also here: deleting
  `Type::Machine` and the old checker's machine path.
- [`141i`](141i-constrained-definitions.md), which gave a free `fn`, `record`, and `enum` their `where` clause. The
  migration below needs it: a generic parameter acquires no method without a constraint, and `stdlib/` is full of
  generic definitions that will.
- [`141j`](141j-notation-vocabulary.md) and [`141k`](141k-notation-lowering.md), which are the notation half of this
  prompt's first Target bullet and were split out of it for 141g's reason. 141j registers what a notated block is built
  out of — `sounded`, `follow`, and `nothing` beside `play`, and `Fact` declared as a family mirroring `FactKind` — and
  141k reads the surface into it: a block is a left fold over `follow`, each statement is one call, and pitches resolve
  against the lexical `in scale` before any track value exists. Both exist because starting this prompt found that
  neither the words nor the reading were there: `play` was the one builtin that constructed anything, one `FactKind`
  case out of nineteen, and sequencing had no word at all, because in the deleted contextual-`Music` design placement
  was the evaluator's cursor rather than an operation. The document *structure* those two do not own went to
  [`141p`](141p-piece-structure.md); what is left here of it is `bar`, `senza`, `assert`, and the instance sites, for
  the reasons that prompt's Stop gives.
- [`141l`](141l-qualified-path.md), which read the `::` path 141g left as its own text. The migration below needs it
  twice over: `stdlib/src/adapters/` spells the phase enumerations `TokenKind.Comma` and moves to `TokenKind::Comma`
  here, and every generic definition that acquires a `where` clause reaches its methods by `Trait::method` — the fix
  `01-surface.md` §1.5's refusal table names and the escape hatch `10-traits.md` §6's strictness is affordable because
  of. It also carries a finding for this prompt's second-path audit: `elab::constrained_function_type` names the
  dictionary binder after the trait, so `(Eq.equal)(x, y)` is a spellable projection out of a binder the core minted and
  a second route to the qualified reading.
- [`141m`](141m-rule-refusal.md), which gave a δ-rule a way to say the program is wrong and took `Result` off the
  notation vocabulary. Two things arrive here from it. Its survey table lists the eleven arithmetic, duration, and
  position rules still answering `Result τ Text` and the twenty corpus sites that branch on them, which is one item of
  this migration written out in advance. Its "what §2 still says that is not true" list is the other: `play(chosen,
  1/2)` and `-> EventTrack[WrittenTime, ScoreFact]` are both spellings this prompt makes true or repairs in
  `01-surface.md`.
- [`141n`](141n-top-level-program.md), which gave `musa-core` the thing a pass hands a whole document to: one group of
  named definitions, every signature collected before any body, so a later declaration may be referenced. It was split
  off when starting this prompt found that the core had no door for it — `declare` takes a *data* group, `check` takes
  one term, and `Cx::define`'s binder is nameless on purpose, so the only named binding on offer was `RawShape::Let`,
  which scopes forward only. `examples/neo-riemannian.musa:72` calls the `compose_close` declared at `:151`, and
  `examples/tonal-construction.musa` does the same kind of thing eighteen times, so a nested `let` chain would have
  refused the existing corpus before any of it was migrated. What is left here is building the group out of 141g's items
  and handing it over.
- [`141o`](141o-document-elaboration.md), which walks a whole document: which of the core's four doors each written
  declaration goes through, the order they open in, and the dependency ordering the family groups need for the reason
  141n's definitions needed one. It was split off when starting this prompt found that `crate::lower` had no caller at
  all outside its own laws — thirteen prompts of reading and vocabulary, and nothing had ever handed the new checker a
  real file. Its survey is this prompt's work list and its argument for scope: `stdlib/`'s sixteen non-adapter libraries
  elaborate as one document with exactly two faults left, `Music` and an anonymous product, and both are rows in the
  Target below. What is left here is the *readback*, the four passes, the migration, and the deletions.
- [`141p`](141p-piece-structure.md), which built the other half of that walk: a piece as one raw track, its parts and
  voices numbered the way the projection buckets them, and the context statements read at the scope that gives "from
  here onward" a meaning. It was split off when starting this prompt found that the seven statements 141k refuses — four
  as misplaced, three at a node labelled "a voice to belong to" — are refused for one reason between them, and that the
  reason is document structure rather than a missing statement. Its survey is this prompt's second work list, over
  `examples/` as 141o's is over `stdlib/`. What is left here of the structure is bar lines and the instance sites: a
  `bar`, a `senza`, and an `assert` are checked against barlines a pass resolves, and a `make` mints an expansion path
  `Sites` has no way to number yet. That survey also found the one fault in either corpus that no prompt had written
  down — a notation statement whose argument is a *bound name* rather than a literal — and the Target below now names
  it.
- [`141c`](141c-structural-eliminators.md), which adds the second of §5.8's four families: a compiler-owned operation
  that takes a function argument and rewrites rather than computing. `recurse_syntax`, `run_syntax_step`, and
  `syntax_fold_from_leaves` are registered through it, and `staff.musa` does not compile without them. Its Design also
  says where the eight collection eliminators go — they become library code *in this prompt's migration*, which is why
  they are not registered there.
- [`141d`](141d-finite-constructor-builtins.md), which widens a δ-rule from literals to finite data. Thirty-eight of the
  92 δ-builtins in `BUILTIN_OWNERSHIP` answer an `Option`, a `Result`, or a `List`, and six take a `List`; without it
  the registry this prompt fills cannot be filled. Its `Datum` is the shape those 38 rules are written against here —
  one owned type in both directions, so a rule that is handed an `Option` and answers one transcribes nothing. Reading
  is untyped and writing is realized against the builtin's own result type; a rule here writes `Datum::Case` with the
  qualified constructor name and its fields, and never names a `Constant`.
- `crates/musa-compiler/src/infer.rs` and the checking paths in `core.rs` — what is deleted, and what has to keep
  working because something other than the checker uses it. Read `data.rs` and `registry.rs` beside them: both name
  `crate::infer` too, and `registry::builtins` is live code that types the phase builders through the old `Unifier`.
- `crates/musa-compiler/tests/suite/elaboration_compatibility.rs`, especially
  `existing_language_behavior_matches_the_migration_oracle`, and `tests/fixtures/elaboration-expected-changes.json` —
  the oracle fixes semantic hashes, kernel digests, diagnostic codes, Origin paths, and the MEI/LilyPond/MusicXML/MIDI
  corpora. This is the only prompt permitted to move an entry in it, and every moved entry is argued in the JSON, by
  slug.
- `docs/rules/language/00-semantics.md` §3 — reusable material as an ordinary value, which is the shape notation takes
  after contextual `Music` is gone. Also its paragraph on the expansion phase, beside
  [`02-core-calculus.md`](../../rules/language/02-core-calculus.md) §5.9 and
  [`26-language-design-decision.md`](../../notes/research/language-design-closure/26-language-design-decision.md) §3.1's
  compiler order — the three sentences that say `Σφ` adds to the ordinary environment and takes nothing from it, which
  is what the Design's phase-import section holds `read_adapter_module`'s import refusal against. Cited because the
  refusal was found by implementing this prompt and repaired from the rules rather than from taste.
- `apps/musa-desktop/ui/src/lib/session/generated/` and `apps/musa-desktop/ui/fixtures/`, and
  `docs/notes/toolchain/generated-files.md` — generated files are regenerated by their generator, never edited. A
  migration that hand-edits one has broken the rule the note exists for.
- `~/Code/Idris2/src/Idris/Parser.idr` around the tuple rule, and `src/Idris/Desugar.idr`'s pair desugaring — a mature
  dependently typed implementation deciding the question the anonymous-product bullet below had declined: a comma run is
  folded to the right in the parser and a two-argument `Pair` is applied at desugaring, so the pair a program sees is
  binary at every width. Cited because the repair recorded there takes that answer rather than inventing one.
- The `module-design` skill's audit questions. Deleting a checker is the largest chance this project will get to make
  `core.rs` smaller; taking it requires knowing which boundary the deletion falls along. The boundary is wider than one
  file: the replaced checker is spread over `core.rs`, `elaborate.rs`, `resolve.rs`, and `infer.rs`, and two of those
  four keep working afterwards, so the question the skill asks — what does each file hide once the walk is gone — has to
  be answered per file rather than once.

## Design

**One commit, green at both ends.** The tree compiles and every gate passes before this prompt and after it, and not
necessarily anywhere between. That is the price of a clean break, and the alternative — a compatibility mode, a second
elaborator selected by a flag, a per-file opt-in — costs more, because every later reader has to learn both languages
and decide which one a file is in.

**Order the work so the migration is mechanical.** Elaborate through `musa-core` first and get the existing corpus
passing under the new checker with the old spellings still in place; then migrate spellings; then delete. Mixing the
three makes every failure ambiguous between "the new checker is wrong" and "this file was translated wrong", and there
will be enough failures that the distinction matters. Prompt 141e already took the first slice of this — the registry
the new checker reads names from — for the same reason and one prompt earlier: a table of 117 builtins is a
*translation* with an oracle to check it against, and burying it inside a diff that also moves 11,304 lines of `.musa`
would have made a wrong signature indistinguishable from a wrong migration.

**Contextual `Music` goes, and placement moves to the fold.** A fragment is an `EventTrack[WrittenTime, ScoreFact]`, a
motif is a function returning one, and placement is applied by the enclosing voice's left fold rather than read from an
ambient context. `00-semantics.md` §3 already says this; this prompt makes the source say it too.

  *Repaired during implementation.* The desugaring's grammar and two of its consequences were not stated:

  - **A motif's parameter list is the one a `fn` writes.** The parser read motif parameters in a loop of its own that
    admitted only a bare `Pitch` or `Duration` — no index, no arrow — even though §2's desugaring makes the parameter a
    `fn`'s. `motif_decl` now calls `param_list`, the lowering binds each written annotation (`annotated_lam` rather than
    dropping the type), and tree-sitter follows under the drift law. The old loop's grammar was the by-subtraction shape
    AGENTS.md forbids.
  - **A duration written as a parameter lowers through a new δ word.** `notated_duration : Duration ⟨written⟩ →
    NotatedDuration` (compiler-owned, in neither ownership table, like the quotation words) is inserted by the lowering
    where a duration position holds a term; the rule derives the spelling with `NotatedDuration::spelled`, the one
    honest answer a computed duration has. `Raw::hosted` names it; the sounded duration is the term itself, tuplet
    scaling included.
  - **A header fact's extent is read off the evaluated music, not summed off the tree.** `reach`'s static sum measures a
    `use` as zero because a `use`'s length lives in the material it names, so canon's header facts covered one bar of
    fifteen. A second compiler-owned word, `track_duration : EventTrack ⟨written⟩ ⟨ScoreFact⟩ → Duration ⟨written⟩`,
    answers the evaluated extent, and the header facts are sounded with it. The music is bound once with `Raw::bind` and
    the variable read at both uses, because `Shape::Let` evaluates the bound term once and shares the *value* — an
    earlier cut of this change inlined the music twice, and the doubled charge moved the staff page's budget crossing
    from the evaluation site to an application site (`every_example_elaborates`' pin tracks it).

**The oracle moves once, and every move is argued.** A changed semantic hash, kernel digest, diagnostic code, Origin
path, or rendered corpus file is an entry in `elaboration-expected-changes.json` with a defect and an observation,
naming the prompt by slug. An unargued move is indistinguishable from a regression, and after this prompt there is no
second chance to tell them apart. **A rendered-notation change is a defect until proven otherwise**: the MEI, LilyPond,
MusicXML, and MIDI corpora describe what a musician sees, and a type-checker replacement has no business changing any of
them.

**Delete the second path, and audit for a third.** `infer.rs`'s unifier, the superseded checking arms in `core.rs`,
`names_a_phase_type` and the collision it worked around, and every function that only existed to keep the old
representation working. Then run the second-path audit the core prompts already established: if two code paths can
answer the same question, one of them is going to disagree eventually. 141l's finding is already on the list: the
dictionary binder a `where` introduces is named after its trait, so a constrained body can project a method out of it by
a spelling no document offers, and the audit decides whether that binder should be unspellable the way a desugaring's
own binders are.

**Take the split `core.rs` has been asking for.** 15,017 lines in one file is not a module boundary; it is the absence
of one. The checker's departure is the moment to see what is left and whether it is one thing. Do not restructure
speculatively — the `module-design` rule is that a boundary hides a volatile decision — but do not put the remains back
in one file out of momentum either. `elaborate.rs` asks the same question and gets a different answer: what is left
there after the walk goes is the fact representation and the kernel projection, which are two things and are read by
different callers, so the file has a split in it that `core.rs` may turn out not to.

**Everything downstream moves in this commit**: `stdlib/`, `examples/`, `docs/book/` fixtures, LSP fixtures, desktop
fixtures via their generators, and `editors/tree-sitter-musa`'s corpus. The book's teaching pages quote fixtures and
`scripts/check-docs.sh` holds them to it, so a migrated fixture with an unmigrated page is a failing gate, not a
follow-up.

**`staff.musa` migrates; it is not rewritten.** It moves onto the new language with its structure intact, still
backwards, still without quotation. Prompt 145 rewrites it and measures the result against 2,404 lines, and a partial
rewrite here would destroy that measurement. The same applies to the studio adapter and prompt 146. *Repaired ordering:*
145 now lands immediately after this prompt, ahead of 143 and 144, because the staff budget failures this migration
leaves close with the rewrite and with nothing smaller — 145's **Read** carries the measurement that reordered it.

**The phase stops subtracting the standard library.** *Found during implementation, and written here because the
migration is what made it observable.*

[`core.rs`](../../../crates/musa-compiler/src/core.rs)'s `read_adapter_module_metered` refuses an adapter module that
writes any `import`, on the stated grounds that "expansion runs before ordinary resolution, so an adapter reads its own
declarations and the phase's operations". Two governing sentences say that refusal is wrong, and one note says the
premise behind it is wrong:

- [`00-semantics.md`](../../rules/language/00-semantics.md): "The expansion phase is these judgments in a second
  environment, **not a second language**", under a `Σφ` that **adds** the phase-local types and a separate registry of
  phase operations.
- [`02-core-calculus.md`](../../rules/language/02-core-calculus.md) §5.9: the phase environment "adds three things and
  **takes nothing away**".
- [`26-language-design-decision.md`](../../notes/research/language-design-closure/26-language-design-decision.md) §3.1
  orders a compilation *lex, parse the header, resolve the header's syntax imports **against the already resolved
  package graph**, expand*. The package graph is resolved before expansion; what happens after it is name resolution of
  the importing module's own body. The refusal conflated the two, and `print_value` already takes an
  [`ImportSources`](../../../crates/musa-compiler/src/imports.rs) to prove it.

Until this prompt the subtraction cost nothing, because everything an adapter could want was a builtin. This prompt's
own Target moves the eight collection eliminators into `stdlib/`, and a module that may not import cannot reach library
code — so the phase becomes "ordinary Musa minus the standard library", which is the shape root `AGENTS.md` forbids by
name and Ousterhout ch. 8 prices: a convenience dropped from a layer is paid by every author who writes in it.

The proof that it is not merely inconvenient is `staff.musa`'s printer, and it is a proof rather than an argument. A
printer is checked **twice** — once under the phase and once ordinarily
([`Printer`](../../../crates/musa-compiler/src/core.rs)) — so every declaration it reaches must have a spelling both
readings accept. `dotted(dots: Nat)` counts a `Nat` down. Ordinarily that is
`match dots { Zero -> …, Succ(fewer) -> … }`, and the phase's type language has `Type::Nat` as a primitive with no
constructors, so the phase cannot destructure it. In the phase that is `nat_fold`, and ordinarily `nat_fold` is
`std::nat`'s function, reachable only by an import. **The intersection of the two readings is empty**, and no amount of
rewriting `dotted` changes that, which is why this is a defect in the phase environment rather than a fixture to bend.

What follows: an adapter module may write ordinary `import` statements, resolved from the same `ImportSources` the
compilation already holds and checked under `Reading::Expansion`; a *syntax* import stays refused, for the bootstrap
reason the region check beside it already gives. This is conformance to §5.9 rather than a new language feature, which
is why it lands here and not behind **Stop**'s "no new language feature" — the sentence being deleted is a refusal the
rules never authorized.

## Target

- `musa-compiler` elaborating through `musa-core`: `check_piece`, `check_arguments`, `check_template_voice`, and
  `check_material` calling 141g's and 141k's lowering and `musa_core::check`, with the old checking path deleted. The
  declaration reading is 141g's, the notation reading is 141k's, and the structure around them is 141p's; what this
  delivers is the rest — bar lines, the instance sites templates make, the kernel quote, the tie, the passes that call
  the whole of it, and the readback that turns a normal form into the musical value a consumer receives. A track is a
  literal at `EventTrack ⟨written⟩`, so the readback is a normal form, a literal, and the payload it holds; what costs
  something is `Program`'s shape, which exists to defer contextual instantiation and has nothing left to defer. Seven
  things bar structure, the instance sites, the kernel quote, and the tie forced, recorded here rather than left as
  drift:
  - **A bar's core spelling is its body, and its claim is placed by two terms.** `bar { … }` contributes no occurrence,
    no payload, and no time — `elaborate.rs`'s own `elaborate_bar` already said the kernel's ontology has no bar in it —
    so the braces erase and the term is exactly the fold of what is inside them. What the braces contribute is the
    measure claim, and a fold has no cursor to place it with. So a claim records *the music before it* and *the passage
    itself*, and the readback turns those two terms into a position and a duration. Each enclosing fold prepends what
    stands before the statement the claim came out of, so a bar inside a `repeat` is placed absolutely without any block
    knowing where it stands. The alternative — selecting a passage's occurrences by provenance — needs a step a bar
    deliberately does not mint, because a bar that turned its own contents into an expansion of itself would make a
    composer's notes read as generated in Origin view.
  - **`senza`'s restoring meter is lexical.** It is `meter none`, the body, and the meter that was in force, which is
    three statements' worth of fold and no mechanism. The meter travels down in the reading beside the scale `in scale`
    supplies, seeded from the piece header and updated at each `meter` a block writes. The replaced path asked its
    cursor which change it had passed; that is the same answer by a longer route wherever a meter is written where it is
    read, and this reading refuses `meter` inside reusable material, so there is no other case.
  - **The anonymous product was half-built, not missing, and both halves are `Pair`.** `(a, b)` has lowered since 141g
    and only the type `(A, B)` refused — one construct disagreeing with itself, which is a defect rather than a stage.
    Both halves now read the family 141ha declared for the machine calculus's wiring: `(A, B)` is `Pair A B` and
    `(a, b)` is `Pair.Both a b`, in the expression and in the pattern. A *constructor application* and not a structural
    record, because that is what makes a written product **canonical data** — `Pair.Both a b` is a `musa_core::Datum`
    and a record is not, so a δ-rule and an assertion's argument can read one back. `prelude.rs`'s own note already
    argued the same point one level down: §2's pairs are wiring, field names would be invented here and read by every
    program that matched on one, and two products that named them differently would stop being the same type. **A wider
    product nests to the right**, so `(A, B, C)` is `Pair A (Pair B C)`, `(a, b, c)` is `Pair.Both a (Pair.Both b c)`,
    and a pattern folds the same way. One fold, applied three times, written once in `lower.rs` — the *direction* has to
    agree between the type side, the value side, and the pattern, and three copies of it are three chances for one to
    lean the other way. This closes the second of the two faults 141o's standard-library survey recorded and one of the
    two on the staff adapter.

    *Repaired during implementation.* This bullet said wider products kept the refusal, on the grounds that a choice
    between `(a, (b, c))` and `((a, b), c)` was one "the corpus does not force, since all 36 products it writes are
    pairs". The count was of `stdlib/` and `examples/`, and this prompt migrates the *fixture* corpus too, which forces
    it in the one place the choice cannot be avoided: `quotation_laws` and `syntax_pattern_laws` write four- and
    eight-wide products because a spread's **arity** is what those laws observe, and `11-quotation.md` §2's whole
    sentence about a position's grammar is a statement about how many members arrived. A list cannot say that — it has
    one type and no count — and a record would need field names invented here for a shape whose members are positions.
    So the refusal was subtraction from the language for no gain, which `AGENTS.md`'s "no sublanguage by subtraction"
    names, and the choice it declined to make is one every language with tuple syntax has already made: Idris folds a
    comma run to the right in its parser (`Idris2/src/Idris/Parser.idr:557`) and applies a two-argument `Pair` at
    desugaring (`Desugar.idr:526`), and the pair a program sees is binary either way. Right, not left, for that
    precedent and because it is the direction that makes `(A, B, C)` and `(A, (B, C))` interchangeable in both
    directions — `core_laws::a_wide_product_nests_the_same_way_written_out_by_hand` is where that is checked.
  - **A pattern's holes bind at the scrutinee's category, so the hole readers come in one pair per category.** 141ga
    registered the pattern side at `Syntax ⟨tokentree⟩` — D1 refuses the category-polymorphic signature — and the
    construction side's splice list at `Syntax ⟨expr⟩`, and the two do not meet: a pattern's holes, bound at
    `⟨tokentree⟩`, cannot be spliced into a quote whose list demands `⟨expr⟩`, which is precisely the round trip
    `syntax_pattern_laws::match_after_build_binds_what_the_quote_spliced` writes down. The resolution is 11-quotation
    §4's own sentence, "a pattern is read at the scrutinee's category": beside `quote_hole`/`quote_holes` at
    `⟨tokentree⟩` stand `quote_hole_expr`/`quote_holes_expr` at `⟨expr⟩` — the *literal* index, so §5.8's D1 check
    admits them where the variable-indexed signature was refused — same rules, answering the hole's node at the
    scrutinee's category. The lowering picks the pair from the scrutinee's *written* category: a bare parameter
    annotated `Syntax<Expr>` reads the `⟨expr⟩` pair, anything else the `⟨tokentree⟩` pair, because the lowering runs
    before types exist and the annotation is the category the author stated. A non-variable scrutinee falls to
    `⟨tokentree⟩`, and a hole then spliced into an expression quote earns the refusal §7 already assigns an uncertified
    tree — the categories named, the repair `as_expression` — which is
    `quotation_laws::a_splice_of_the_wrong_category_names_both_categories`'s case, not a new hole in it. The match test
    itself needs no twin: the scrutinee reaches `match_quote`'s `⟨tokentree⟩` parameter through §1's forgetting rule,
    which this prompt supplies. And `INSTANTIATE` discharges the certificate the index claims: the built tree must parse
    as an expression (`syntax::parses_as_expression`, `as_expression`'s own machinery), or the rule does not reduce —
    §1's "a certificate nobody checks is a comment" answered at the one boundary the index exists for, cheaply because
    construction is rare beside matching. 147's round-trip obligation still owns the whole surface.

    *Repaired during implementation.* Not in the sketch at all: the sketch's forgetting rule covers the scrutinee and
    says nothing about the splice, and the hole was discovered by the law above going red — the fixture corpus is where
    this prompt's assumptions get measured, per its own Check. The first repair moved the *splice* list to
    `⟨token-tree⟩` instead, and the corpus refuted it within a run: `syntax_fold_from_leaves` hands a group branch its
    children at `List Answer` — folded *results*, not raw subtrees — so an adapter folding to `Syntax ⟨expr⟩` splices a
    `List (Syntax ⟨expr⟩)`, and §1's acceptance is a rule between two `Syntax` indices that does not reach under the
    `List`. Generalizing acceptance under a type constructor is subtyping by another name, which §1 forbids; moving the
    category choice to the hole binding, where the literal index makes it sayable, is the reading the spec's sentence
    already had.
    One more sentence of 11-quotation went stale the same way, and this prompt conforms the candidate rather than the
    code: §2's "a quote is a checking form" guarded against a *search* — trying each category until one parses — that
    141ga's registry makes impossible, since `instantiate_quote` is fixed at `⟨expr⟩` and a token-tree position is
    reached by forgetting, not by building there. §2 and §7's table now say so (an inferring position infers
    `Syntax<Expr>`; the certificate, not an annotation, is what can fail), and
    `quotation_laws::a_quote_in_an_inferring_position_names_the_annotation` is migrated to pin the new truth. The
    certificate's reporting — a stuck instantiation is currently an expansion refusal, not the parser's words — is
    147's obligation already, named in §8's matrix.

  - **`assert` comes after the anonymous product, not before it.** Its arguments are values the readback has to evaluate
    and hand `crate::assert::Claim::build` in its own six shapes, and one of those six is `within_ranges`'s
    `List<(Pitch, Pitch)>` — the spelling that had no core term until the bullet above gave it one. Reading its
    arguments syntactically to get around that would be a second way to read an expression, which is the second path
    this prompt exists to remove. Four things the implementation forced:
    - **A `Claimed` records the claim it will build, not a built one.** `crate::assert::Claim::build` takes *evaluated*
      arguments and a block being read has none, so the statement holds the registry's own `Predicate` row and a vector
      of `Argued`. Two cases, because `ParamType` has two kinds in it: four shapes are values and stay raw terms, and
      the policy and the rule id are **words** the registry reads rather than terms the language can produce, so they
      are resolved where they stand. `Document::passage` therefore answers the claim beside the passage — one call,
      because whether the notes are gathered depends on which claim was built (`Claim::reads_notes`).
    - **A value argument is annotated, not checked against a second table.** The reading wraps each one in `Raw::annot`
      at the type its shape declares — `Scale`, `ChordClass`, `Nat`, `List (Pair Pitch Pitch)` — so the core refuses a
      wrong argument against the registry's own declaration. The replaced checker carried a parallel `Type` for each
      shape; a second list of expected types beside the registry is exactly the drift this prompt removes. The
      consequence worth stating: an argument's type is checked when the claim is *placed*, not when the block is read,
      because a term has no type until the document it stands in has a context to check it in.
    - **The readback is one function in `crate::registry`, and 141q's predicted promotion was not needed.** 141q's
      Design assigned 142 both a generic pair reading and a promotion of `registry::rules`'s four readers from
      `pub(super)` to the crate. Writing it showed the promotion was an artifact of putting the reading at the *call
      site*: `registry::argument(shape, normal)` lives in `registry.rs` itself, which already sees `rules`, so the three
      existing readers stay where they are and only `rules::halves` is new — the generic pair 141q did ask for, with
      `registry/notation.rs`'s specialized `Pair Nat Nat` reading restated over it. One `pub(crate)` item with one
      caller instead of five with none.
    - **`spell_arguments` and `spell_written` move to `assert.rs`.** The arity sentence is about a `Predicate`'s arity
      and two readings write it while the cutover is in progress; one spelling beside the registry rather than two that
      could describe the same claim differently.
  - **An instance site is a λ, and its provenance is a builtin.** `04-templates-and-modules.md` §1 says expansion is a
    binding and never a rewrite; in a core with λ that sentence *is* the implementation. `make N(a₁, …) as I;` reads
    `N`'s voice body once, at the `Scope::Voice` of the item the site stands at, and wraps it in λs over `N`'s
    parameters applied to the site's argument expressions. λ rather than a chain of `let`s, because a `let` puts the
    first parameter in scope of the second argument — a site inside a template that passed on its own `subject` would
    silently pass the template's parameter of that name instead. `crate::template::Templates` resolves the site
    unchanged, so the generated identity, the collision registry, the arity and kind diagnostics, and the self-instance
    cycle check are the ones already written; what is new on it is `Instance::bound`, the parameters and the argument
    nodes paired, which is the same question `holders`/`bindings` answer for the checker being replaced. Three
    consequences:
    - **The expansion path needs no number `Sites` hands out.** 141p left this to 142 as "a `make` mints an expansion
      path `Sites` has no way to number yet". It does not need one: `Sites` numbers *written nodes*, and the step
      `crate::template` already mints out of the site's structural address is the whole of what a fact must carry.
    - **`instanced : Origin → EventTrack ⟨written⟩ → EventTrack ⟨written⟩` is a ninth track builtin, registered in
      neither ownership table.** A builtin and not a stamp applied while reading, because the facts do not exist until
      the term is evaluated and the ones a function the body calls produced were read in another declaration entirely —
      a reading that stamped as it walked would reach the notes written inside the template and miss those. It prepends
      the origin's expansion path onto every fact's, which is where `ExpansionStep::TemplateInstance` is already
      documented as belonging: everything a transform appends happened *inside* the instance. Only the path is read; a
      fact keeps its own span, because an instance does not relocate the text a composer wrote. It is past both
      ownership tables for `set_note_pitches`'s reason one domain over — a source word for it would let a program claim
      its notes were generated by a template that never made them, which is exactly what Origin view reads.
    - **`Lowering::piece` takes the document's name.** A generated identity is minted in the document's namespace (§2),
      which cannot be read off the tree, so `Document::piece` carries it through. Collecting the templates there also
      reports a template declared twice whether or not a site calls it, which is where that mistake belongs.
    - **What a template body sees is the flat program, and capture is a grammar fact.** The law this replaces
      (`a_template_body_cannot_see_the_site`, now `a_template_body_sees_the_document_and_never_the_making_site`) was
      written for the replaced checker's positional scopes: it put a `let` inside the piece and called that the site.
      Under one program per document the piece's `let` is a document definition, §1's judgment checks the body in the
      ambient context, and the body's seeing it is conformance rather than a leak. The capture the law actually guarded
      is unreachable for a reason one level down: a body's free name could be caught by a making template's binder only
      if a body could hold a `make`, and the grammar does not admit one there — the λ reading's argument-direction care
      (above) is exactly the direction the grammar leaves open. The law now pins both halves of that.
  - **`origin_literal` and `provenance_literal` were one function twice.** Byte-identical bodies at `Origin`, one
    reached by the laws and one by `lower/notation.rs`, with the dead-code expectation on the first hiding the
    duplication. The instance site needed a caller for one of them and found two, so `provenance_literal` is deleted and
    its argument merged into `origin_literal`, whose name matches the base type the way `template_literal` and
    `token_kind_literal` match theirs.
  - **A kernel quote's core spelling is a literal held whole and one builtin.** `kernel EventTrack[WrittenTime,
    ScoreFact] { … }` is read by `crate::lower::kernel`: the reading answers everything the quote can be wrong about —
    the three type words, the body's syntax, a payload that would settle its use's key, meter, or voice, and a name the
    quote does not bind — and hands the checked `musa_kernel::Term` on in a literal at a new base type `KernelTerm`,
    beside `Template` and for `Template`'s reason. Translating the term into core terms instead would mean registering
    raw `scale` and `restrict`, which are the two formers the quote *exists* for (`examples/kernel-splice.musa` writes
    `${stretch(1/2, subject)}` because raw `scale` moves occurrences without renotating payloads), α-renaming the
    quote's binders so a spliced expression cannot be captured, and re-deriving `musa_kernel::evaluate` as core
    reduction — three costs whose only caller would be this one reading. Three consequences:
    - **`spliced : KernelTerm → List (EventTrack ⟨written⟩) → EventTrack ⟨written⟩` is a tenth track builtin, in neither
      ownership table.** Past both for `instantiate_quote`'s reason one stage up: the source already spells the whole
      operation, so a row would invent a second, *called* spelling taking a term no expression can build and a list
      whose order only the reading knows. It binds the *i*th material to the *i*th hole name — `Term::bind`, first hole
      outermost, which is the kernel's own call-by-value sharing — and evaluates. `evaluate` is total on a checked term,
      so the replaced path's "this kernel quote has no extent" diagnostic was unreachable and is not carried forward.
    - **The builtin needs neither an `Origin` nor a `Scope` argument**, because 141k fixed both at *read* time: a free
      `music { … }` reads at `Scope::Piece`, and a kernel quote is an expression. The reading stamps the payloads the
      quote wrote raw, which is also what makes refusing a context-authoritative payload a refusal rather than a silent
      overwrite.
    - **A hole's provenance is `instanced`, not a second builtin.** "These facts were produced inside this expansion" is
      one claim with two sites: an instance site hands it `[TemplateInstance{…}]`, a `${…}` hands it
      `[KernelSplice{at: locus}]`, and prepending is right for both because everything the hole's own expression did
      happened inside the splice. `TRACK_BEYOND`'s doc widens from "minted by `crate::template`" to "minted by the
      reading that resolved the site".
  - **A tie takes two words, and 141j had neither.** `~` is the one surface mark that says something about *two*
    statements, and `sounded` builds one fact — `registry/notation/laws.rs` already recorded the consequence, "`sounded`
    writes no tie: a tie is a property of two facts" — so the reading 141k left had no way to carry one and dropped it.
    `tied` marks the notehead a `~` was written on and `joined` is where a tie stops existing, both registered in
    `registry/notation.rs` past both ownership tables, because `~` already spells the whole idea and a *called* pair
    would let a program mark material it did not write as continuing, or join two noteheads a composer wrote as two.
    `notation::BEYOND` goes 2 → 4 and the operations past both tables 9 → 11. Two consequences:
    - **`joined` is applied at a voice and at nothing smaller.** Both of its refusals are questions about a whole voice:
      a tie at the end of a `repeat` body or a slur continues into whatever follows the block, so a merge inside one
      would report a dangling tie at every nesting level, and "this tie has nothing to tie to" is only answerable where
      there is nothing after. `lower/piece.rs` wraps each voice — the written one and the one a `make` produces —
      unconditionally, because the fold does not look at what it is folding and a `use` can bring in material that ends
      in a tie.
    - **`tied` marks the whole track it is given, not "the last statement of it".** The only thing it is ever applied to
      is one notehead statement: `g4/4 ~` is one `sounded` and `[c4 e4]/2 ~` is a `together` of two, and the written `~`
      is about all of them. A general "tie the end of this block" would be a word for a shape no source writes.
- A notation statement whose argument is a **bound name** read as a term rather than refused. `root/4` in
  `motif turn(root: Pitch)`, `key k;` and `in scale mode` in a `template piece`: 141k's reading folds a pitch, a key,
  and a scale to a *value* while it walks the block, so a parameter — which has no value until an instance site supplies
  one — is reported as though a literal had been misspelled. Five of `examples/`' fifty-four fail this way and both
  template examples do, so the migration cannot land without it. What it costs is that `Fact.Note`, `Fact.Key`, and the
  scale a `step` counts in take an argument that is a `Raw::var`, which makes the enclosing `sounded` a neutral term
  until the site applies it — the ordinary behaviour of a builtin under an unapplied binder (`02-core-calculus.md`
  §5.8), and the reason this is a spelling rather than a language feature. Record in the code map which of the four
  statements each corpus needed. Three things the implementation forced, recorded here rather than left as drift:
  - **`Fact.Key` takes one `Key`, not a tonic and a mode.** A bound key cannot be taken apart by any word this compiler
    registers, so a two-binder constructor could be reached only from a spelled `key g major;`. Narrowing it to one
    binder gives a key statement one meaning rather than two, and `key g major;` reaches the same constructor through
    the same reading. `registry/notation.rs` reads the pair back off the single value.
  - **`pitch_transposed` and `pitchclass_transposed` are new `BUILTIN_OWNERSHIP` rows**, on 141m's refusal channel.
    `p up M3` was an *arm of the replaced checker* (`core.rs`'s `pitch_action`) rather than a row of the table 141e
    translated, so 141e had nothing to carry: an interval moves two coordinates at once and the operation that does it
    had no source word. It needs one here because `(root up M2)/4` in a motif has no literal to move at read time.
    `BUILTIN_OWNERSHIP` goes 117 → 119 and `rules::REGISTERED` 124 → 126.
  - **`step` under a *bound* scale stays refused**, with an `UnsupportedLanguageStage` naming what is missing: a step
    walks a `scale::Frame::around` of the collection, and no registered operation builds one. No corpus file needs it,
    and inventing a builtin for a shape nothing writes would be a knob rather than a capability.
- **The replaced checker deleted, and the eight files it is spread across told apart.** `infer.rs` deleted, `core.rs`'s
  superseded arms deleted, `names_a_phase_type` deleted, second-path audit recorded — and `elaborate.rs` and
  `resolve.rs` carved, which is what the sentence this bullet replaces did not name and the compiler has been reporting
  since the wiring landed. The dead surface is 118 warnings across eight files: 75 in `elaborate.rs`, 22 in `core.rs`,
  14 in `resolve.rs`, 3 in `template.rs`, and one each in `factext.rs`, `marks.rs`, `assert.rs`, and `core_budget.rs`.
  Four things that distribution forces, recorded here rather than discovered halfway through the deletion:
  - **`infer.rs` is not a leaf, and `registry.rs` is why.** Its consumers are `core.rs` (`Type::Var`'s payload,
    `member_types`, `plain`, `plain_one`, `admits`), `data.rs` (`Kind`, `Unifier`, `rebuilt`, `member_types`), and
    `registry.rs` — and the third is *live code the new core depends on*. `registry::builtins` types the phase builders
    by instantiating `SYNTAX_OWNERSHIP`'s scheme with `crate::infer::Unifier` and translating the resulting `Type`
    through `phase_type`, so the registry `musa-core` reads is currently filled by the checker being deleted. Stating
    those signatures in core vocabulary is therefore part of this deletion rather than a consequence of it, and until it
    is done the deletion is not available whatever the warning count says.
  - **`elaborate.rs` and `resolve.rs` are carved, not deleted.** `elaborate_parsed` is still the entry point and now
    orchestrates `crate::document::elaborate`; `FactKind`, `ScoreFact`, and `VoiceTrack` are the fact representation
    `registry/track.rs`, `factext.rs`, `lower/kernel/laws.rs`, and `kernel_text.rs` read; `piece_term` and
    `kernel_normal_form` are the kernel projection `06-surface-elaboration.md` §Sharing was repaired around.
    `resolve.rs` keeps the `Resolver`, the reference index, `lower_header`, `lower_studio`, and the measure and groove
    checks. What goes from both is the middle — the walk that built tracks from a cursor — which is why neither file
    appears on a ledger of deleted concepts and both belong on this list.
  - **A dead item is not automatically a deletion, and `core_budget.rs`'s row is the proof.** `WorkMeter::output` and
    `preflight_output` are unused because nothing in the new lowering charges the million-occurrence limit
    `06-performance.md` fixes: a capability not yet called, not a capability replaced. It stays, and prompt 144
    re-measures it. Every row is sorted into *replaced* or *not yet called* before anything is removed, and that sorting
    is what the second-path audit records — an unsorted deletion silently converts a hole into an intention.
  - **The check is the crate's own warning count, because the ledger cannot be.** `clean-break-ledger.md` names concepts
    and their owning prompts, so §7's grep discharges `Music` and `instantiate_music` and says nothing about the
    eighty-nine private items no document ever named. The Check already states the condition and needs no command added
    to it: `cargo clippy --workspace --all-targets -- -D warnings` promotes `dead_code` to an error, so a surviving
    second path fails the gate. What the audit adds is the reason each row went, which a warning cannot say.
- `primitive` typed against the build-local registry, and `registry/rules.rs`'s `UNREGISTERED` shortened by the row
  141ha left there. It is not one more signature: the name and version select the descriptor that supplies the ports and
  the configuration type, so this is elaboration reading a registry, not a table gaining an entry.
- The storable-port premise discharged where the constraint solver is — a port is `Storable`, and
  `Machine ⟨step⟩ (Nat → Nat) Nat` is refused here rather than left to `03-machine-calculus.md` §5's preparation. Five
  things `03-machine-calculus.md` §2's premises forced, recorded here rather than left as drift:
  - **A base type declares its own storability, and a host signature is the only thing that may demand it.**
    `Base::storable` is a fact the base type carries, `Cx::with_externs` generates the instances from it, and
    `musa_core::requiring_storable` wraps a registered signature in the constraint binder. That is what makes §2's
    `data A` premises *writable*: they appear on `machine(p)`'s two ports, `identity`'s one, and `feedback`'s stored
    value, and on nothing else — `connect`, `beside`, `copy`, `drop`, and `swap` carry no premise and now carry no
    binder. `Storable` stays reserved and generated (`02-core-calculus.md` §1.2): no source may write `impl Storable`,
    and a signature may only *require* it. `registry/machine.rs`'s note that the forms take no constraint binder was
    written before the solver could carry one and is repaired in the same commit.
  - **A constraint nobody wrote still has to land somewhere.** A host's `Storable A` has no span, because the demand is
    part of the compiler rather than part of the program, and a refusal filed at `Origin::UNKNOWN` points at nothing.
    `Constraint::at_use` refiles it at the application that inserted the dictionary — the only place in the *program*
    that has anything to do with it — and a constraint the source did write keeps its own place, because a `where`
    clause is worth pointing at.
  - **The step-tag premise is the one that could not become a constraint, so it is refused by the reading.** A step tag
    is a host notion and `base.rs`'s law is that the core owns the mechanism and the host owns the table, so the core
    signature cannot say that `Machine`'s first argument holds one. `lower/types.rs::machine_type` position-restricts it
    against `registry::is_step_tag`, carrying forward the two messages the replaced checker already wrote. Both halves
    of §2 therefore survive the restatement and they survive it differently: storability as something the signature
    *says*, the step tag as something the surface *reads*.
  - **A conversion mismatch names the two types it was already holding.** `musa-core` gained `show.rs`, a printer in
    core vocabulary — `Machine K A B` and not `Machine<K, A, B>`, because that is what the term is — so `Mismatch` says
    `expected Ratio, found Nat` and `Refusal::UnkeyedConstraint` says which type no key could hold. Prompt 144 still
    owns the rest of the diagnostic surface; what is discharged here is only the part where the stage had computed the
    answer and printed none of it.
  - **§1.3's complete-call rule needed a form of its own, and "declared parameter" turned out not to be a fact about
    types.** `RawShape::Call` is the surface's written argument list, because a spine cannot state the rule: every
    prefix of an iterated application is itself an application and none of them knows it is the last. `Raw::call` is
    used only where an author wrote the list — a reading that builds `play`'s four arguments out of two written ones
    uses `Raw::app`, since an arity sentence about the four would be about the reading rather than about the call. Two
    encodings of the count failed before the third was right, and the failures are the evidence:
    - Counting the explicit binders left standing *after* the last argument refuses `staff_item_fold(…, items)` wherever
      a caller instantiates its generic answer `A` to a function. A result that happens to be a function is not a
      missing argument.
    - Counting the explicit binders of the *head's* type refuses `walked(items)`, where
      `fn walked(items: StaffItem) -> (Position<WrittenTime> -> Result<Realization, Text>)` declares one parameter and
      returns a function. That declaration and a two-parameter one have the same type.
    - What separates them is already in the term: a declared parameter carries the name its author wrote, and a written
      arrow's binder carries `musa_core::ARROW_BINDER` because a Π always binds and nothing in `A → B` refers to the
      argument. `Raw::arrow` is now the constructor for the second, `elab.rs::declared_parameters` stops at it, and
      `Refusal::Underapplied` names the parameters no argument reached — "nothing is given for `by`" rather than a count
      the reader has to go and resolve. A *registered* signature is read the other way, every explicit binder counted,
      because there is no surface declaration beside it to disagree with: the arrow it was built from is its parameter
      list. A parameter an author actually spells `argument` is invisible to the rule, which is a miss and not a wrong
      answer.
  - **An unannotated definition is refused when nothing determines its implicit arguments, and `machine_laws` said
    otherwise.** `let m = identity;` names no ports, so insertion at the definition leaves metavariables nothing can
    solve and §2.1 refuses rather than defaults.
    `machine_laws::an_undecided_machine_is_a_value_and_not_yet_a_projection` still described the replaced checker's
    answer — an open machine kept as "a value with a scheme" — which is HM generalization, and a dependent core has
    none. The law is migrated to the reading `document::laws::an_open_machine_is_refused_until_its_ports_are_written`
    already stated, and the two agreeing is the point: one program cannot be accepted by the elaborator and refused by
    the reading.
- **A block's `follow` is a balanced tree, and this is where the depth budget was found to be reachable at all.** 141k's
  reading says "a block is a left fold over `follow`", and a left fold is a spine as deep as the block is long. The
  evaluator descends it, so `02-core-calculus.md` §4.1's 256 nested levels were reached at some sixty statements:
  `examples/in-c.musa`'s fifty-three-figure voice was refused for nesting, and so was a voice of a hundred plain notes.
  `follow` is associative — `musa_kernel::follow` places each track after the one before and where the brackets fall
  moves no occurrence — so `lower/notation.rs` accumulates into a stack of balanced subtrees whose sizes are powers of
  two, merging equal neighbours the way incrementing a binary counter carries. Depth becomes log₂ n, nothing is rebuilt,
  and every subtree is shared. It also removes a quadratic: each `Claimed` records *the music before it*, and a prefix
  is now the same stack folded down rather than a fresh copy of everything so far. A voice of 260 notes elaborates where
  sixty did not.
  - **What that left standing is a finding, not a stage: a numeral is a tower, and the depth budget measures it.**
    `lower.rs::whole` counts a number up from `Nat.Zero`, because `Nat` is a declared family rather than a base type
    (§5.8) and its literals are therefore constructor applications. So `repeat 384` — the pulse in `examples/in-c.musa`
    — is a term 384 applications deep in *argument* position, and `eval` charges one nesting level per level of it: the
    file is refused at "257 of 256", and would be at any limit a version bump could name, since the depth is the number
    the composer wrote. It is not confined to a repeat count. `stdlib/src/notation/staff.musa` declares
    `Bar(anchor: Nat, …)` and every other node carries an anchor, so an adapter over a real score builds towers as deep
    as the document is long; a source-written `132` in an expression is one 132 deep. Nothing in the corpus matches on
    `Nat.Zero` or `Nat.Succ`, so what the tower buys at these sites is nothing, and what it costs is a depth no program
    can lower. The fix is a *representation* for numerals — the family and its constructors unchanged, a literal that
    splits into `Zero`/`Succ` only where a `match` forces it, which is what every dependent language with a usable `Nat`
    does. That is core work, and this prompt's **Stop** says a migration that wants a language feature records it.
    Recorded here. Reassigning the `Fact` payloads to a base type is *not* the smaller version of it: it would fix the
    repeat count and leave `anchor` alone, and the line between the two would be magnitude rather than meaning.
  - **Three examples still exhaust the step budget, and the measurement they were waiting for split them between two
    prompts.** `diatonic-sequences`, `rule-of-the-octave`, and `staff-page` reach 200,000 reduction steps. §4 sets that
    number as a language-version constant against the *replaced* checker, and this prompt's **Stop** forbids performance
    work and a benchmark rerun on a half-migrated compiler. *Repaired after the measurement this bullet deferred.*
    `staff-page` goes to prompt 145, pulled ahead of 143 and 144 for exactly this: instrumented on the migrated checker,
    the 77-line file spends 1,605,182,361 reduction steps, 1,108,756,085 of them `eval` entries over 2,719 distinct
    source origins, with 18,922,391 δ-unfoldings over 84 distinct definitions — the adapter's backwards reading calling
    through the library. A per-term closedness tally (each distinct term shape walked once for a `Var` anywhere under
    it) found 198,753,114 of those entries, 17.9%, on closed terms, and **zero** at the five hottest sites, all
    closed-looking type annotations in `stdlib/src/notation/staff.musa` whose elaborated terms carry the context
    elaboration glued on. A pointer-keyed closed-term cache therefore caps below a fifth of the spend and never fires
    where the spend is — and since a cache changes what the meter charges, §4 makes one a cost-table version bump
    besides. The spend is the call count, the call count is the adapter's structure, and the structure is 145's.
    `diatonic-sequences` and `rule-of-the-octave` exhaust the same budget through `std::tonal` with no adapter involved;
    they remain 144's measurement, made against a checker 145 has already rewritten the adapter on.
- The eight collection eliminators 141c left out of the registry written as library code: `stdlib/src/nat.musa` holds
  `nat_fold`, `stdlib/src/option.musa` holds `option_fold`, and `stdlib/src/list.musa` holds the other six, each over
  the prelude's own constructors and nothing else. Two things the writing forced, recorded here rather than left as
  drift:
  - **Seven keep their old spelling and `repeat` cannot.** This prompt's own ordering rule says to migrate with the old
    spellings in place, and one of the eight has no old spelling available: `repeat { … }` opens a repeated passage, so
    the word is a statement keyword and `fn repeat` does not parse. It is `repeated` in `std::list`, and the two sites
    that named it — `stdlib/src/adapters/doubled.musa`'s emitted identifier and `examples/doubled.musa`'s header comment
    — say so.
  - **A fold's public argument order is not the order its recursion can be written in.** The corpus writes
    `list_fold_from_start(seed, combine, values)`, and §2.4's measure holds the arguments *before* the recursive
    position fixed, so the list has to come first where the recursion happens. Each public fold is one call into a
    `private fn` that takes its list first; the wrapper is what keeps the corpus unchanged.
- **An adapter module may import**, which is what makes the bullet above reachable from the phase at all. Concretely:
  - `read_adapter_module` takes the compilation's `ImportSources`, loads the module's own import closure through
    `crate::imports::load`, and checks the module against it — the same call the ordinary path makes, so a library that
    does not compile is reported the same way in both, with 141a's `Code::Import` cause naming the file.
  - The `ImportStmt` refusal is deleted and its sentence with it. What stays refused is a **syntax** import, beside the
    `SyntaxRegion` check that is already there and for that check's reason: an adapter written with an adapter puts the
    expansion order back into a cycle, and this pair is the whole of what prevents it.
  - `expand_syntax` and `edit_syntax` take the sources too, and the region cache in `expand.rs` keys on the closure as
    well as on the region — two identical regions under different import graphs are not the same expansion, and a cache
    that said they were would be the second path this prompt's audit exists to find.
  - `run_printer` writes the *adapter's* ordinary imports into the synthesized piece beside `at`'s, so the printer's
    ordinary reading sees exactly the modules its phase reading saw. That is what makes "checked twice" a claim about
    two readings of one program rather than about two programs.
  - `stdlib/src/adapters/staff.musa` and `stdlib/src/adapters/doubled.musa` import `std::nat`, `std::option`, and
    `std::list` as they need them, and keep every call they already write. **This is the migration, not the rewrite**:
    the diff is import lines, and prompt 145's measurement is untouched.
  - The law: a module whose printer names an imported function reads under both readings, and an adapter that writes
    `import syntax …` is refused with a sentence naming the bootstrap.
- `stdlib/`, `examples/`, book fixtures, LSP fixtures, desktop generated fixtures, and the tree-sitter corpus migrated.
- The eleven arithmetic, duration, and position rules 141m left answering `Result τ Text` moved onto the refusal
  channel, and the twenty `match … { Ok(v) -> … }` sites its survey table lists rewritten. 141m could not: one table
  serves two checkers, so narrowing a declared answer there rewrote the corpus, which is this prompt. The criterion is
  141m's and unchanged — `Option` is a musical answer, `Result τ Text` is a diagnostic wearing a value's clothes — and
  what makes it worth doing here rather than never is that 141l routed `x + y` through `Add.add`.
- Contextual `Music` and `ContextualMusic` gone from source, compiler, and documents.
- `docs/plan/clean-break-ledger.md`: every row this prompt owns marked discharged, with the rows left for 150–153 named.
- `tests/fixtures/elaboration-expected-changes.json`: every moved oracle entry, argued.
- `docs/plan/code-map/` rows for every crate that changed, and `docs/rules/language/` repaired wherever the
  implementation proved a specification claim wrong.

## Check

```sh
cargo build --workspace
cargo nextest run --workspace
cargo nextest run --run-ignored all
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
cd editors/tree-sitter-musa && tree-sitter test
pnpm -r check && pnpm -r test
cargo deny check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make lint-ui
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
```

Every example in `examples/` must still compile and render, and every rendered corpus file must be byte-identical unless
its change is argued in `elaboration-expected-changes.json`.

**Two classes of budget exhaustion are expected red at this commit, and they are named rather than silent.** The *staff
class*: every test whose only failure is `reduction steps at 200001 of 200000` out of the staff adapter's expansion —
today `staff_expansion_laws`'s expansion regions, `every_example_elaborates`' staff-page case, and
`the_staff_page_example_compiles_and_renders`. It closes at prompt 145, which lands next and whose Check runs the same
two `nextest` lines with this class green.

    *Repaired during implementation.* The class has one downstream member the failure-mode sentence does not name:
    `musa::cli`'s `wav_export_is_deterministic_for_all_examples`, an ignored test that iterates every example and fails
    at `staff-page.musa` with "cannot export: the piece has never compiled successfully" — the staff class seen one
    stage later, at the export boundary rather than at the budget. Its only failure is the staff adapter's; it goes
    green with the rest of the class at 145. The *tonal class*: the same failure mode out of `diatonic-sequences` and
`rule-of-the-octave`, ordinary `std::tonal` evaluation with no adapter involved. It closes at prompt 144, which measures
it and sets the cost table. This prompt's closing commit lists every red test in both classes by name; anything red
outside them means the prompt is not done.

*Repaired during implementation.* A third class, and it is the same wall the tonal class is. The *pressure class*:
`the_pressure_workloads_compile_and_denote_what_they_claim`'s `core-pressure` case, whose only failure is `nested
evaluation levels at 257 of 256` out of stdlib recursion — `naturals(512)` is `counting_from`, one recursive call per
element. Measured: `range(128)` alone crosses (each recursion level costs two nesting levels — the unfold and the match
— so the wall stands at ~120-deep user recursion), the 64-deep call chain alone does not, and the piece's own voices
never approach it, because a voice fold is `follow` — a δ rule — and charges no depth. This is 144's "linear in the
music" finding in its second instance (the first is `registry/traversal.rs`'s Cons chain, repaired there), and it
carries a consequence the finding's first statement did not: `benches/pipeline.rs` reads this fixture, so the wall makes
a benchmark workload unrunnable — "a benchmark nobody can run is a gate nobody is holding"
(`elaboration_fixture_generators.rs`'s own warning, now true of its neighbor). The same class takes the step wall's
plain-music instance: `large_score_generators::large_score_fixture_is_current` and
`large_score_is_the_size_the_budgets_assume`, whose 1500-event fixture — no adapter, no library recursion, one voice of
ordinary notation — crosses `reduction steps at 200001 of 200000`. A piece is not supposed to need the adapter's
argument to be large, and the desktop's own budgets are measured against this fixture, so this is the charge model's
problem statement in its plainest form: the per-event charge times a real score's size is over the table §4 publishes.
Both close at 144 with the rest of the nesting and cost-table verdict.

Commit as `Move the whole language over, once`.

## Stop

- No compatibility mode, no alias, no deprecation shim, no per-file language selection, no "old syntax still accepted
  with a warning". The ledger's rule is rename, delete, and version-break in the prompt that owns the concept.
- No rewrite of `stdlib/src/adapters/staff.musa` or the studio adapter beyond mechanical migration. Prompts 145 and 146
  own those, and own their measurements.
- No builtin-registry collapse. Prompt 143.
- No re-translation of a signature or a rule 141e already wrote. A disagreement between the two is a defect in one of
  them and is repaired where it is, not worked around here.
- No performance work and no benchmark rerun. Prompt 144 measures the finished checker; measuring a half-migrated one
  would produce a number nobody can act on.
- No hand-edited generated file, under any deadline pressure.
- No new language feature. If the migration wants one, that is a finding and a repair, and it is far better to record it
  than to slip it in under a migration diff.
