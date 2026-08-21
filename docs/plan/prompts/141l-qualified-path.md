---
id: 141l
slug: qualified-path
status: done
depends_on: [141g, 141i]
phase: 3
---

# Read the Qualified Path

## Task

`01-surface.md` §1.5 calls explicit qualification "the escape hatch that makes the strictness above affordable", and
`10-traits.md` §6 rests method syntax's whole refusal of a generic receiver on it: "any use that lookup refuses can be
written out." Nothing reads it. [`141g`](141g-raw-lowering.md) wrote every other surface form and left a `PathExpr` as
its own text — `Raw::var("Eq::equal")` — and no name `musa-calculus` holds is spelled with `::`, so *every* qualified
path is `UnknownName` today: a trait method, an enum case, an inherent function alike. Read it, in expressions and in
patterns, and make the operator and index forms desugar to it the way §1.5 says they do.

## Read

- [`../../rules/language/01-surface.md`](../../rules/language/01-surface.md) §1.5 in full — it fixes every decision this
  prompt makes and leaves none of them open. "A `::` path is read left to right, and the capitalization rule §1 already
  fixed decides where the module prefix ends: lowercase segments are modules, the first capitalized segment names a type
  or a trait, and exactly one segment follows it. `std::tonal::TokenKind::PitchLiteral` has one reading." And, four
  paragraphs later, the operator table in its surface spelling: "`x == y` is `Eq::equal(x, y)`, `x < y` is
  `Ord::less(x, y)`, `x + y`, `x - y`, `x * y`, `x / y` are `Add`, `Sub`, `Mul`, `Div`, and `xs[i]` is
  `Index::at(xs, i)`."
- The same document's refusal table, three rows of which this prompt is about: **qualified path** (`a lowercase segment
  after a capitalized one — *a type namespace holds one item*, pointing at the extra segment`), **method call**
  (`x.m(y)` where `x : A` is a parameter — *method lookup needs a concrete type*, **with the `where` or `Trait::m(x, y)`
  as the fix**), and **indexing** (`xs[i] : Option<A>` ⇝ `Index::at(xs, i)`).
- [`../../rules/language/10-traits.md`](../../rules/language/10-traits.md) §5 and §6. §5 is the operator table with the
  trait each row names; §6 is what qualification is *for* — "Qualification always resolves, which is what makes the
  strictness of method syntax affordable."
- [`141g`](141g-raw-lowering.md)'s Design, whose "one direction" rule governs here unchanged: this prompt adds a
  *reading*, and a reading resolves nothing. Its Stop list governs here too.
- [`141i`](141i-constrained-definitions.md), which gave a free `fn` its `where` binder and is why the escape hatch is
  worth opening now. Its own law says the gap in as many words: "§1.5's qualified path has no spelling yet."
- `crates/musa-compiler/src/lower/values.rs` — `value`'s `PathExpr` arm, `application`'s dotted-head split, `operator`,
  `indexing`, `pattern`, and `phase_literal`. Five readings, one of which is right and four of which this prompt
  changes; see Design for which is which.
- `crates/musa-syntax/src/ast.rs`'s `PathExpr::segments`, which already hands the reading exactly what it needs.
  Re-deriving the segments from `node.to_string()` would be the shape of mistake root `AGENTS.md` names: "an adapter
  re-parsing `3/8` out of a token's spelling".
- `crates/musa-calculus/src/class.rs`'s `Classes::method`, `crate::dictionary::method_at`, and `elab::constant`. The
  core's qualified names are dot-separated — `rsplit_once('.')` is the lookup — and `Eq.equal` reaching a trait method
  through `constant` is what `crates/musa-calculus/tests/suite/trait_laws.rs` already proves. This prompt supplies the
  surface half of a mechanism that is finished on the core side.
- [`143`](143-builtin-collapse.md), which declares the traits these spellings name. `Eq`, `Ord`, `Add`, `Sub`, `Mul`,
  `Div`, and `Index` are not in `crate::registry::owned` and are not this prompt's to add.

## Design

**The separator is the whole finding, and it points at the surface rather than the core.** A `where` clause on a `fn` is
admitted and unusable, and the reason is not that `musa-calculus` refuses a generic receiver — §6 says it should, and
`Refusal::MethodOnVariable` is that rule working. The reason is that the repair §6 names has no reading. Two candidate
fixes are therefore wrong before they are written. Teaching `lower/values.rs`'s `application` to tell a trait name from
a receiver would give lowering a scope it does not have and does not need, because the surface already tells them apart
with `::` and `.`. Teaching `elab::method` to fall back to the qualified reading when the receiver names a trait would
give `.` a second meaning that §1.5 assigns to projection — a `docs/rules/` change, so `docs/rules/README.md`'s
amendment procedure and not a repair — and would leave `Eq::equal` broken afterwards. `Same.same(x, y)` stays refused.

**One reading, and §1.5 wrote it.** Segments left to right; drop the leading lowercase ones, which are modules; the
first capitalized segment and the one segment after it are the name. Join them with `.`, which is how `musa-calculus`
spells a qualified name — `Nat.Succ`, `Bool.True`, `Eq.equal` — and hand it through as a variable for the core to
resolve, the same as every other name 141g writes through. Nothing is looked up: which segments are modules is a
question about capitalization, and capitalization is a question about the written text. That is what keeps
[`Lowering`](../../../crates/musa-compiler/src/lower.rs)'s "No scope" true.

**More than one segment after the first capitalized one is refused here.** "Exactly one segment follows it" is a claim
about the written path and about nothing else, so it is the one refusal a reading can raise without becoming a checker.
It needs a code: `Code::QualifiedPath`, with its `musa explain` entry beside the rest.

**The phase's two enumerations are reached by the same path.** `01-surface.md` §1.5's own example is
`std::tonal::TokenKind::PitchLiteral`, and `stdlib/src/adapters/` writes `TokenKind.Comma` twenty times and
`Delimiter.Parentheses` ten. Prompt 142 migrates those spellings; they resolve only if the path reading consults
`phase_literal` where the dotted `NameExpr` reading already does. Two readings of one vocabulary that agreed only by
accident would be the second path `02-core-calculus.md` §5's audit exists to catch, so it is one function asked from
both.

**A pattern is the same path and is read the same way.** The grammar admits `Tying::Untied` in a pattern (see
`parser.rs`'s `at_path_separator_next` arm), and `pattern` reads a flat run of tokens: the head becomes the constructor
and every later identifier becomes a binding, so `Tying::Untied` is today a constructor named `Tying` binding a variable
named `Untied`. It is the same defect one position over and it is repaired with the same reading.

**An operator is a qualified path, and this is what lets it resolve under a `where`.** `x == y` lowers to
`Raw::method(x, "equal")` today, which is §6 method syntax on an exact receiver — so under `where Eq<A>` it is
`MethodOnVariable`, against §5's "an operator resolves only when the concrete head type is known **or a `where` supplies
the dictionary**" and against §1.4's "`x == y` inside `same` is `d.equal(x, y)` for the `d` the caller supplied". §1.5
spells the desugaring `Eq::equal(x, y)`, and `lower.rs`'s own module documentation already claims it — "`x == y` is
`Eq.equal x y`" — while `values.rs` writes something else. The qualified reading makes both cases work for one reason:
`dictionary::method_at` opens the trait's arguments as metavariables and §4 postpones, which a `where` binder discharges
and a known head answers from the global instance. `xs[i]` moves for the same reason and to §1.5's own spelling,
`Index::at(xs, i)`.

**`p up M2` and `p step n` do not move.** Their receivers are concrete and their operations are inherent items in a
type's namespace, which is exactly the case §6's exact-receiver lookup is for. An operator moves because §5 makes it a
*trait* method and a trait method is what a `where` can supply; a transposition is neither.

**The traits do not exist yet, and that costs nothing here.** `crate::registry::owned` declares no `Eq`, `Ord`, `Add`,
or `Index`, and prompt 143 is where the registry collapses behind them. Nothing calls this lowering until 142 wires it,
so writing the spelling §1.5 fixes before the trait it names is declared changes no program's behavior — it changes
which prompt is holding the wrong thing. The laws declare their own trait, as 141i's already do with `Same`.

**A finding this prompt records and does not repair.** `elab::constrained_function_type` names the dictionary binder
`Trait::super_field(&constraint.class)` — the trait's own name — so inside any constrained body the trait is a spellable
*value* and `(Same.same)(x, y)` type-checks today as an ordinary record projection out of it. It works only for a
required method, only under a `where` binder, and it lets source name a binder the core minted; it is a second path to
the qualified reading, reached by a spelling no document offers. Renaming that binder is a `musa-calculus` change with
its own risk surface — `super_field` also names the dictionary *field* a super-constraint occupies, which §1 requires to
be the class's name — so it goes on prompt 142's second-path audit rather than into a prompt about reading the surface.

## Target

- `crates/musa-compiler/src/lower/values.rs`: the `PathExpr` reading, taken from `PathExpr::segments` and doc-commented
  with §1.5's rule before it is implemented; the same reading reached from `pattern`; `phase_literal` asked from both
  spellings; `operator` and `indexing` emitting the qualified path rather than `Raw::method`; `operator_method`'s table
  carrying the trait §5 names for each row.
- `Code::QualifiedPath` in `crates/musa-score/src/diagnose.rs`, with its arm in `musa-project`'s explain mapping, for a
  path with more than one segment after its first capitalized segment.
- `crates/musa-compiler/src/lower/refusals.rs`: `MethodOnVariable` carrying the refusal table's own fix as a `help` —
  the `where` clause, or `Trait::m(x, y)`. This prompt is what makes the second half of that sentence true, so it is the
  prompt that may say it. The core's own message still spells `Trait.method`; 144 owns that wording.
- Laws in `crates/musa-compiler/src/lower/laws.rs`, each lowering something a composer could write and handing it to
  `musa_calculus::check` in a context the law declares: a trait method reached by `Trait::method` under a `where` and at
  a known head; an enum case reached by `Type::Case` in an expression and in a pattern; a module-prefixed path reading
  to the same name as the bare one; an operator checking under a `where` and at a known head; a path with an extra
  segment refused at the node with `Code::QualifiedPath`.
- `docs/plan/code-map/` rows for `musa-compiler`.
- Prompt 142 repaired: its `depends_on` names this prompt, its Read cites it, and its second-path audit carries the
  dictionary-binder finding above.
- Prompt 143 repaired: its Design records that the operator and index readings already write `Eq::equal` and
  `Index::at`, so what it adds is the declarations those spellings name rather than a change to the lowering.

## Check

```sh
cargo build --workspace
cargo nextest run -p musa-calculus -p musa-compiler -p musa-project
cargo clippy --all-targets -p musa-calculus -p musa-compiler -p musa-project -- -D warnings
cargo fmt --check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
python3 scripts/renumber-prompts.py audit
```

Commit as `Read the qualified path`.

## Stop

- Nothing is wired and nothing is deleted. 141g's Stop governs unchanged: `check_piece`, `check_material`, `infer.rs`,
  and every checking path in `core.rs` are untouched, and the corpus still goes through them.
- No `.musa` file changes, in `stdlib/`, `examples/`, or any fixture corpus. The adapters keep writing `TokenKind.Comma`
  until 142 migrates them; this prompt makes `TokenKind::Comma` read to the same literal, and migrating is not reading.
- No trait declared. `Eq`, `Ord`, `Add`, `Sub`, `Mul`, `Div`, and `Index` are prompt 143's, and a trait declared here to
  make a law green would be 143's survey answered in advance by the prompt with no argument for it.
- No grammar change. `musa-syntax` already parses both path positions; a path form it does not admit is not this
  prompt's to add.
- No type-position path. The type grammar has no `::` production, and inventing one is a language change rather than a
  reading.
- No module resolution. Dropping lowercase segments is the capitalization rule §1.5 fixed, not a lookup; what a module
  name reaches is still `use` and `import`'s, and a path whose prefix names no module is the core's `UnknownName`.
- No second meaning for `.`. `Same.same(x, y)` stays §6 method syntax on a receiver named `Same`, and `Same.same` alone
  stays a projection. Changing that is an amendment to `01-surface.md` §1.5, not a repair.
- No message rewriting beyond the one `help` named above. Prompt 144 owns how good a refusal's sentence is, including
  `MethodOnVariable`'s own.
- No dictionary-binder rename in `musa-calculus`. It is recorded in Design and carried to 142's audit.
