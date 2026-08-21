---
id: 127dceb
slug: adapter-module-scope
status: done
depends_on: [127dce, 127dcea]
phase: 3
---

# Give an Adapter Module the Phase Environment It Was Promised

## Task

Prompt 127dc said an adapter definition is "checked and evaluated in prompt 127da's phase environment". The
implementation slices the text of one `let` out of the module and checks it as a bare expression with no symbols, no
modules, and no world, so an adapter has no scope of its own. Make an adapter module a module. Let it spell the phase's
own types. Add the two operations the first trial proved missing: the reader's numeric reading of a token, and equality
on `Text`.

## Read

- Prompt 127dc's **Design**, third paragraph: "An adapter definition is checked and evaluated in prompt 127da's phase
  environment, where the syntax types are in scope; ordinary source is checked in the environment it has now, where they
  are not. One checker, one evaluator, two environments." That sentence is the specification; this prompt is the code
  catching up to it, not a new decision.
- Prompt 127da's phase environment and `SYNTAX_OWNERSHIP`; prompt 127dcb's `expand : Syntax -> Result<Syntax, (Syntax,
  Text)>`; prompt 127dce's three levels and their `let level = …` declaration.
- `crates/musa-compiler/src/expand/mod.rs`: `declaration_of` and `declared_body`, which are the text slice, and every
  caller of them — `level_of`, `editor_of`, and the `print` lookup.
- `crates/musa-compiler/src/phase/mod.rs`: `run_transformer`, which is the splice; `check_and_evaluate_metered` and
  `root_checker`, which are how an ordinary module is checked, and where `Reading::Source` is written in by hand;
  `Reading::Expansion`; `SyntaxOp::instantiate`; `BUILTIN_OWNERSHIP` and the δ-family entries around `ratio_equal`.
- `crates/musa-syntax/src/syntax_kind.rs`: `Integer`, `Rational`, and `Float` — the lexer already keeps `3/8` whole,
  which is the number this prompt hands back rather than re-derives.
- `docs/rules/language/02-core-calculus.md` §5, whose "no syntax value" sentence is about ordinary source and stays
  exactly true here, and §5.8's D1–D4 with Theorem 5, under which `text_equal` is a conservative extension and needs no
  amendment.
- Peyton Jones ch. 3, which enriches the lambda calculus with local definitions and pattern-matching abstractions
  *because* a language written by programmers needs them. Ousterhout ch. 7 (adjacent layers with the same abstraction),
  ch. 8 (pull complexity downwards), and ch. 24's shallow-module and nonobvious-code red flags.
- Root `AGENTS.md`, "No sublanguage by subtraction" and "Hand a consumer what we already computed" — the two standards
  this prompt is the first application of.

## Design

The trial that found this is prompt 127dcfa. Written against the spliced phase, the staff adapter could not scan a
group's children into a pair, so every node had to fold to a *function* of a reader state; it could not turn `4` into a
number or compare two spellings, so the notation it could accept degraded into bracketed forms with the keyword inside
each one. An adapter written that way is unreadable and the notation it reads is not staff notation. That is evidence
about the phase, not about the adapter.

**An adapter module is a module.** It is parsed, checked, and evaluated exactly as a library is, under
`Reading::Expansion` rather than `Reading::Source`. Its `let`, its `fn`, and its `data` are in scope in `expand` and in
`edit`, because they are declarations of the module those operations are declared in. `level` is one of those
declarations and is read the same way. The text slice goes: `declaration_of` and `declared_body` are deleted, and the
phase asks the checked module for a value by name.

**`print` is the one operation that stays outside the checked module, and its reason is the same reason the module has
no imports.** A printer is handed the value the region produced, whose type belongs to the *composer's* package — the
adapter, importing nothing, has no name for it. So `print` cannot be a declaration of a module checked on its own; it
keeps what it has today, read as text and checked at the site that calls it, under `Reading::Foreign`, against the value
it is actually handed. The cost is real and is stated rather than hidden: a printer cannot call its module's other
declarations. Whether a printer should be checked against an interface the adapter can name is prompt 127dcf's question
— that is where the first adapter with a printer worth writing arrives — and it is not this prompt's.

One checker and one evaluator, two readings — which is what 127dc said. The reading still decides two things and only
two: whether the phase registry answers a name, and whether the phase's types have a spelling.

**The phase's types are spellable in an adapter module and nowhere else.** `Syntax`, `NodePath`, and `BindingPath` are
written type names under `Reading::Expansion`, so an adapter can annotate a parameter and can declare `data` that holds
a node. Ordinary source cannot name them, cannot obtain a value of one, and gets the same "cannot find" it gets for any
other unbound type; `02-core-calculus.md` §5's sentence is about that language and remains true word for word. A test
holds the two halves apart.

This is what makes a reader ordinary. `list_fold` checks its step before its list, so a tuple *pattern* against an
unresolved element type is refused — the annotation the checker wants was unwritable, and now it is written. A `data`
the adapter declares is better still: it names its fields, so the module has a reader type instead of a positional
tuple, and projection resolves against a world that is no longer empty.

**`syntax_number(node)` answers with the exact rational the reader already read.** The lexer keeps `3/8` whole as one
`Rational` token and `4` as one `Integer`; an adapter re-deriving either from a token's text would be re-doing work the
compiler has already done, and could not do it at all, since the phase has no `Text`-to-number operation and a finite
table over note values misses `c5(3/8)`. It is a phase operation and not a δ-builtin, in `SYNTAX_OWNERSHIP` beside
`syntax_at`, and it hides the reader's own numeric interpretation of a literal. Its type is `Syntax -> Option<Ratio>`:
optional because a node that is not a numeric token is not a number, and `Ratio` because one operation covering both
kinds is one operation an adapter has to learn.

**`text_equal(left, right)` is an ordinary δ-builtin.** Two texts an adapter was handed cannot be compared at all today,
so a check like "this tie's continuation spells a different pitch" is unwritable — not because it is a privilege, but
because `Text` is the one base type whose equality was never registered. `(Text, Text) -> Bool` satisfies D1 through D4
the way `ratio_equal` does, so §5.8's families are unchanged and Theorem 5 covers it. Ordinary source gets it too; there
was never a reason it should not have.

**What does not change.** `expand`'s type, the refusal that carries a node, anchors, the three levels, the fixed
compiler order, expansion records and their charging, type blindness, and the ban on emitting imports, modules, or
declarations. An adapter module still contains no adapter region, so termination is still structural. An adapter that
imports another module is refused with a sentence saying so rather than silently ignored; whether the phase ever
resolves imports is a later question and not this prompt's.

## Target

- Adapter modules checked and evaluated as modules under `Reading::Expansion`: `Reading` threaded through the module
  pipeline instead of written in by hand, the operations resolved by name from the checked module, and
  `declaration_of`/`declared_body` and the splice in `run_transformer` gone.
- `Syntax`, `NodePath`, and `BindingPath` spellable as type names under that reading only.
- `syntax_number : Syntax -> Option<Ratio>` in the phase registry, with its hidden-information entry.
- `text_equal : (Text, Text) -> Bool` in the δ registry, with its hidden-information entry, and its row in
  `docs/book/src/reference/language.md`'s operations table.
- The bundled fixture adapter rewritten to use a module-level helper and a declared `data`, so the new scope is
  exercised by the phase's own executable fixture rather than only by a test.
- Tests: an adapter whose `expand` calls a sibling `fn`; one that destructures a `data` it declares; one that annotates
  a parameter `Syntax`; ordinary source refused for naming `Syntax` and for calling `syntax_number`; `syntax_number` on
  an integer token, on a rational token, and on a node that is neither; `text_equal` on equal and unequal texts and in
  ordinary source; an adapter that imports, refused with its sentence; and every diagnostic inside an adapter module
  landing in that module rather than in the composer's file.

## Check

```sh
cargo nextest run -p musa-syntax -p musa-compiler -p musa-project -p musa-lsp
cargo clippy --all-targets -p musa-syntax -p musa-compiler -p musa-project -p musa-lsp -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
```

Commit as `Give an adapter module the phase environment it was promised`.

## Stop

- No change to `expand`'s type, to the refusal, to anchors, or to the three levels.
- No change to how `print` is checked — it stays at its call site, under `Reading::Foreign`.
- No syntax value, syntax type name, or phase operation reachable from ordinary source.
- No import resolution for adapter modules — refuse and say so.
- No staff adapter, no studio adapter, and no work on prompt 127dcfa's fourteen items.
- No general macro system, no adapter-generated declarations, and no adapter recursion.
- No change to `docs/rules/`.
