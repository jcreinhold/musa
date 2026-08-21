---
id: 136
slug: records-and-enums
status: done
depends_on: [135]
phase: 3
---

# Give the Language Records and Namespaced Enums

## Task

Add the first two surface forms of the new language: structural records with field projection and `with` update along a
path, and nominal enums whose constructors live in their type's namespace. Grammar, CST, formatter, and tree-sitter in
`musa-syntax`; elaboration into the core in `musa-calculus`. This is where `data Pending`'s eight-field destructure dies
and where the `Untied` collision that forced `names_a_phase_type` into the compiler stops being possible.

## Read

- `docs/rules/language/01-surface.md` §1 as rewritten by prompt 130 — the record and enum grammar, their desugarings,
  and the corpus rows that say what is accepted and what is refused.
- Prompt [127dcfac](127dcfac-record-update.md) — single-field, single-level `with` on nominal data already landed, so
  read what it built before widening it. The parser, formatter, and CST work it did is the pattern to follow.
- `stdlib/src/adapters/staff.musa`'s `data Pending` and every site that destructures it, and the `Tying`/`Tie`
  constructor collision together with `names_a_phase_type` in `crates/musa-compiler/src/phase/mod.rs`. Those are the two
  concrete programs this prompt has to improve, and prompt 166 measures whether it did.
- `crates/musa-syntax/src/{keywords.rs, parser.rs, highlight.rs}` and the formatter — where a new keyword actually
  enters the language, and what else has to move with it.
- `editors/tree-sitter-musa` and the drift law that binds it to the real lexer. A new keyword that does not reach the
  grammar is drift, and the drift test is in the Check for that reason.
- `docs/rules/style-guide.md` — naming rules are prompt 137's, but read what a rule costs before writing new syntax that
  will need one.

## Design

**Where each half lands, and why the compiler is not wired up yet.** The grammar, CST, formatter, and highlighting go
into `musa-syntax`; the elaboration of records and enums into core terms goes into `musa-calculus`, over the raw term
prompt 134 introduced. `musa-compiler` connects the two exactly once, in prompt 142. Until then a `record` declaration
parses, formats, and highlights, and then fails resolution with the existing unknown-declaration diagnostic. That is
deliberate: wiring a second checking path through the old compiler and then deleting it in 142 would mean building the
migration twice and would put a second elaborator in the tree for six prompts. This same split applies to prompts
137–141, and 142's Design says how the seam closes.

**Records are structural, with named fields and η from the core.** `01-surface.md` §1.2 is explicit: a record *is* its
fields, two declarations with the same fields at the same types denote one type, and an author who wants two quantities
kept apart declares one-case enums instead. The declared name is what diagnostics say and nothing more. Declaration,
construction by field name, projection `p.field`, and update along a path: `p with { region.anchor = a }`. The path form
is the widening 127dcfac deferred, and `Pending` is why: eight fields, read one at a time, updated one at a time,
destructured in full every time. Two rules worth stating in the grammar rather than discovering in review — an update
names a path, not an expression, so `p with { f(x).g = y }` is a syntax error rather than a puzzle; and an update of a
field that does not exist names the record type and its actual fields.

**Enums are nominal sums with namespaced constructors.** `TokenKind::PitchLiteral`. Two enums in one module may share a
constructor spelling, which is the whole point: the printer splice in prompt 127dcfb failed because the staff adapter's
`Untied` and the staff package's `Untied` were one name, and the compiler still carries `names_a_phase_type` as the
scar. A bare constructor is still accepted where the expected type is known — that is 134's check direction doing its
job, not an inference heuristic — and ambiguity between two enums in check position is an error naming both.

**`data` does not fork into three declarations.** Records and enums are the two shapes `data` was being used for, and
they are two spellings an author chooses between rather than three unrelated declarations to learn. They do not
elaborate the same way, and §1.2's last bullet is why: a `record` becomes a core dependent record type, so there is no
family, no recursor, and no positivity question, while an `enum` generates its own family under §1.1's positivity check.
That asymmetry is the *reason* the surface distinction is worth having — it is what makes records structural and enums
nominal — so the prompt states one surface idea with two elaborations rather than one elaboration with two skins.

**The formatter decides the layout before the first program is written.** Records and enums are declarations authors
read far more often than they write; `musa format` owns their shape, the round-trip and idempotence tests cover them,
and `format_check_passes_on_the_standard_library` stays green. A formatter that has no opinion about a new form is how
`.musa` files start disagreeing with each other.

**Laws.** Projection after construction is the field. `with` at a path changes that field and nothing else — the
locality law 127dcfac stated, now at depth. Two records with the same fields are convertible (core η). An enum's
constructors are distinct and its match is exhaustive. Parsing round-trips losslessly and formatting is idempotent.
Namespaced constructors with the same spelling in different enums do not collide, tested by the program that used to
collide.

## Target

- `musa-syntax`: grammar, CST nodes, typed AST wrappers, formatter, highlighting, and completion for record and enum
  declarations, path update, and namespaced constructor paths.
- `editors/tree-sitter-musa`: grammar and queries, with the drift test green.
- `musa-calculus`: elaboration of records and enums into core records and families, and their laws.
- `crates/musa-syntax/tests/suite/` and `crates/musa-calculus/tests/suite/` cases, including the collision program.
- `docs/plan/code-map/` rows for both crates.
- No `stdlib/` or `examples/` change: nothing is migrated until 142.

## Check

```sh
cargo build --workspace
cargo nextest run -p musa-syntax -p musa-calculus
cargo nextest run --workspace
cargo clippy --all-targets -p musa-syntax -p musa-calculus -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
cd editors/tree-sitter-musa && tree-sitter test
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
```

Commit as `Give the language records and namespaced enums`.

## Stop

- No traits, no impls, no operators, no method syntax, no `where`. Prompt 137.
- No `musa-compiler` wire-up, no migration of `stdlib/` or `examples/`, and no second checking path. Prompt 142.
- No anonymous records, no row polymorphism, no structural subtyping, no field punning, no update-by-function
  (`p with { n = f(n) }` is not this prompt's form and needs its own argument).
- No deletion of `data`, and no deletion of `names_a_phase_type` — the workaround comes out in 142 with the code that
  made it necessary, and removing it before the collision is actually gone would be removing a guard on a live bug.
- No style-guide rule; prompt 137 adds naming rules with their diagnostics.
