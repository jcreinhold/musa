---
id: 161
slug: one-declaration-form
status: done
depends_on: [157]
phase: 3
---

# One Declaration Form: `enum` and `record` Become Sugar Over `data`

## Task

The surface has three ways to declare a type — `data`, `record`, `enum` — and the differences between them are not
principled: they are three parsers that grew capabilities at different times. Make `data` the one form, make the other
two shapes of it, and leave exactly one declaration path in the lowerer. Three forms to one, with two spellings kept for
readability rather than for meaning.

## Read

- `docs/rules/language/01-surface.md` §1.2 and §1.3 after prompt 145's repair — both already name this prompt as the one
  that states the rule.
- `crates/musa-compiler/src/lower/items.rs` — `nominal`, `enumeration`, `structural`, the three lowering paths that
  become one, and `variant`/`case`, the two constructor readers.
- `crates/musa-syntax/src/parser/declarations.rs` — `data_decl`, `data_variant`, `record_decl`, `enum_decl`,
  `enum_case`.
- [`57-three-declaration-forms.md`](../../notes/research/language-design-closure/57-three-declaration-forms.md) — the
  capability matrix this prompt is answering, and the corpus measurement behind its Stop.

## Design

**The measurement first.** The three forms differ in five capabilities, and no two of them differ the same way:

|  | index telescope | chosen indices | per-variant `private` | positional fields | named fields |
| --- | --- | --- | --- | --- | --- |
| `data` | ✓ | ✓ | ✗ | ✗ | ✓ |
| `enum` | ✗ | ✗ | ✓ | ✓ | ✓ |
| `record` | ✗ | ✗ | ✗ | ✗ | ✓, one case |

`record` is a shape of `data` already. **`enum` is not**, and that is the finding: it has two capabilities `data` lacks,
so today it is a third form rather than a spelling, and no amount of lowerer tidying makes it sugar.

**One form, by enriching rather than by subtracting.** `data` gains the two — a `private` marker on a variant, and the
positional field form — and then both other spellings are shapes of it. Root `AGENTS.md`'s rule about sublanguages is
the same rule one level up: a form that is "`data` minus two things" is a second abstraction, and the fix is to make the
general form general. Nothing is taken from `enum`; §1.3 keeps every sentence it has.

**Two conveniences, and what each is.** `enum` is `data` with several cases and no index telescope. `record` is `data`
with one case, named after the type, whose fields are named, and no index telescope. Both are refusals rather than
grammar gaps: `enum Vect<A>(n: Nat) { … }` parses the telescope and is refused with *an indexed family is written with
`data`*, where today it says “expected `{`” and leaves the author to guess. That is the one place the grammar still lets
a spelling lie once the data declaration is the union, so it is the one refusal each spelling owes.

**Desugaring, not a second path.** The three CST readers normalize into one description and one builder produces the
`RawData`. There is exactly one declaration path afterwards, and that is the check: a bug in `enum` handling becomes
impossible because there is no `enum` handling — only an `enum`-shaped read of the one declaration.

**Diagnostics keep the spelling the author used.** A refusal about an `enum` says `enum`. The description records which
spelling it came from, for messages only — never for typing.

**The scope repair, recorded.** *Repaired before implementation; note 57 is the measurement.* Three of this prompt's
sentences did not survive contact with the code.

- *"Give `data` index syntax at the surface (prompt 156 gave it to the core)"* — 156 gave it to **both**. `DataIndices`
  and `DataChosen` are in the parser, and `Lowering::index_telescope` and `Lowering::chosen_indices` read them.
- *"`enum` is `data` where every constructor is nullary … refused where a constructor takes a field"* — that contradicts
  `01-surface.md` §1.3, which is governing and which shows `enum Reading<A> { Done(A), Refused { at: NodePath, why: Text
  } }` and `enum Chord { private NamedChord(ChordSymbol, List<Spelling>), … }`. The precedence ladder settles it: the
  rules win, and this Design now states §1.3's rule instead.
- *"`record` … refused with more than one constructor"* — `record_decl` reads fields and nothing else, so there is no
  such program to refuse. The refusal each spelling actually owes is the index telescope, above.

**What this closes.** The complaint that started the thread: three declaration forms whose differences were not
principled. After this they are one form and two shapes of it, and the difference *is* principled — how many cases, and
whether the fields want names.

## Target

- `crates/musa-syntax`: a `private` marker on a `DataVariant`, and the positional field form in one — the two
  capabilities that make `enum` a shape of `data`. An index telescope admitted by `record_decl` and `enum_decl`, so the
  lowerer can refuse it by name.
- `crates/musa-compiler/src/lower/items.rs`: one declaration path. The three readers become shape-normalizers over one
  description; one builder produces the `RawData`; the two telescope refusals; the spelling carried for diagnostics.
- `crates/musa-compiler/src/lower/laws.rs` or `tests/suite/`: that the three spellings of one declaration produce the
  same family, and the two refusals, each naming the spelling it was written with.
- `stdlib/`, `examples/`: unchanged. Nothing in either declares an `enum`, and no `record` wants an index — note 57 §3.
- `docs/rules/language/01-surface.md` §1.2 and §1.3, `docs/book/`: the one form and its two shapes, replacing the two
  paragraphs that promise this prompt will say it.

## Check

```sh
cargo nextest run --workspace
cargo nextest run --workspace --run-ignored all
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
cargo run -p musa -- format --check stdlib/src examples
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
```

`--run-ignored all` carries the 30 staff-budget failures prompt 166 owns; the count must not move.

Commit as `Collapse the declaration forms to one`.

## Stop

- No removal of the `enum` or `record` keywords. This prompt removes a *semantics*, not a spelling — even though note 57
  §3 records that no `.musa` file in the repository declares an `enum`, which is a fact about the corpus before the two
  adapter rewrites and not yet an argument for deleting a word §1.3 specifies at length.
- No change to what a `record` literal, pattern, projection, or `with` means. §1.2's five rules stand.
- No module-layer work — 162.
