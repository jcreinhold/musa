---
id: 127dcfac
slug: record-update
status: done
depends_on: [127dcfab]
phase: 3
---

# Let a Record Be Rebuilt by Naming Only What Changed

## Task

`stdlib/src/adapters/staff.musa` declares `data Pending` with seven fields and then writes seven `holding_*` functions
whose entire body is a match that destructures all seven and a constructor that names all seven, one of which differs.
The file constructs a `Pending` twenty-four times. Nothing about that is the adapter's problem: the language can build a
nominal record and take it apart, and has no way to say "this one, with that field replaced."

This prompt adds immutable update of named fields of a nominal record. It is constructor elaboration, and it changes
nothing about what a record means.

## Read

- `docs/rules/language/02-core-calculus.md` §1 and §5 — nominal constructors, projection, and the exhaustive `match`
  that is a record's only eliminator today; and §5.6's generated fold, which update must not disturb.
- `docs/rules/language/01-surface.md` — the constructor and pattern grammar this prompt extends.
- `docs/notes/research/language-design-closure/39-totality-and-structural-abstraction.md` §4.1, which fixes the
  five-part semantic contract below and rules out row polymorphism, width subtyping, lenses, mutation, and structural
  records; and §3.3, whose level audit keeps this at constructor elaboration rather than a record calculus.
- Prompt [127ac](127ac-nominal-data.md) — nominal declaration, its generated fold, and the exact identity and
  storability rules an updated value must still satisfy.
- Prompt [127dcec](127dcec-construction-charges.md) — a value is charged where it is constructed. An update constructs
  one record, and the meter must say so.
- Ousterhout ch. 8 — the cost of a missing convenience is paid once per author rather than once by the implementation.
  Seven near-identical functions in one standard-library file is that cost, itemized.
- `stdlib/src/adapters/staff.musa`: `data Pending` at line 274, and `holding_body`, `holding_length`, `holding_dots`,
  `holding_tie`, `holding_numbers`, `holding_voices`, and `holding_word` at lines 896–1003.
- `crates/musa-syntax/src/parser.rs`, `ast.rs`, `formatter.rs`, and `syntax_kind.rs`; and
  `editors/tree-sitter-musa/grammar.js` with its queries, held to the lexer by the drift law in root `AGENTS.md`.

## Design

**The semantic contract, in five parts.** For an update of a nominal record:

- the subject is evaluated exactly once;
- each field's right-hand side is evaluated exactly once, left to right, against the surrounding lexical environment and
  not against the subject's fields;
- every named field belongs to the subject's nominal record type;
- a field named twice is rejected with a diagnostic that points at both mentions; and
- the result has the subject's nominal type, with every unmentioned field carried over unchanged.

The second part is the one that has to be stated: an update is not a `let` over the old fields, so
`p with { n = n + 1 }` reads the *surrounding* `n`, not the field. An author who wants the old field writes it —
`p with { n = p.n + 1 }` — and that reading is decidable at a glance, which is the point.

**It elaborates to a constructor application.** The core is unchanged: an update of a record with fields `f₁…fₙ` becomes
the nominal constructor applied to the mentioned right-hand sides and to projections of the subject for the rest, with
the subject bound once so it is not re-evaluated. No new core term, typing rule, reduction, or normalization case. The
charge is one record construction, exactly as prompt 127dcec requires, and the elaboration must not smuggle in a second
one by duplicating the subject.

**Refused: row polymorphism or structural records.** Both would make "a record with at least these fields" a type, which
is a different language. The update here is nominal: the type is known from the subject, the field set is fixed by the
declaration, and an unknown field name is a resolution error naming the declaration.

**Refused: lenses or a generated per-field setter.** A generated `with_dots` per field is the `holding_*` pattern moved
from the library into the compiler, and it multiplies with the field count instead of disappearing. A lens library needs
composition over a type this language cannot yet write.

**Refused: mutation.** Nothing about this is assignment; the subject is unchanged and the result is a new value.

**The spelling is tested, not assumed.** `p with { dots = more }` reads as an expression, keeps `{ … }` a block
delimiter, and does not collide with the constructor form. It is a candidate: this prompt writes the corpus both ways
against the formatter and the tree-sitter grammar before fixing it in `01-surface.md`, and reports if a different
spelling parses and formats better.

**The measurement is the deliverable, not the claim.** `stdlib/src/adapters/staff.musa` has seven `holding_*` functions
and twenty-four `Pending` constructions today. The prompt records both counts before and after, so the reduction is
evidence rather than assertion, and so that prompts 127dcfae and 127dcfag can attribute later improvements correctly.

## Target

- `docs/rules/language/01-surface.md` — the update form, its five-part contract as **Design** states it, and the
  elaboration written out.
- `docs/rules/language/02-core-calculus.md` — a sentence in §5 recording that update is surface elaboration to the
  nominal constructor and adds no core term. §5 is otherwise unchanged and the prompt states that it is.
- `crates/musa-syntax/` — `syntax_kind.rs`, `lexer.rs`, `parser.rs`, and `ast.rs` gain the form; `formatter.rs` lays it
  out, including the one-field and many-field cases.
- `crates/musa-compiler/` — elaboration with the subject bound once; resolution of each field against the declaration;
  the duplicate-field diagnostic naming both mentions; the unknown-field diagnostic naming the declaration; and a source
  map that points a field's type error at that field's right-hand side.
- `editors/tree-sitter-musa/grammar.js` and `queries/` — the form and its highlighting.
- `stdlib/src/adapters/staff.musa` — the seven `holding_*` functions collapse to updates at their call sites, or to
  bodies one line long where a named function still earns its name. `holding_length`'s deliberate use of `faulting` is
  behavior and is preserved.
- Tests in `crates/musa-syntax` and `crates/musa-compiler`: the subject is evaluated once and each right-hand side once,
  in order, proved by the meter rather than by inspection; a right-hand side reads the surrounding binding and not the
  field; an unmentioned field is carried over; duplicate and unknown fields are rejected at their own spans; updating a
  non-record and updating across two nominal types are rejected; the result's exact identity matches the equivalent full
  construction; and the charge is one construction.
- A short note under `docs/notes/research/core-calculus/` with the before/after counts of `holding_*` functions and
  `Pending` constructions, and its entry in that directory's `README.md`.

## Check

```sh
cargo nextest run -p musa-syntax -p musa-compiler -p musa-project -p musa-lsp
cargo clippy --all-targets -p musa-syntax -p musa-compiler -p musa-project -p musa-lsp -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
cd editors/tree-sitter-musa && tree-sitter test
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
```

Commit as `Let a record be rebuilt by naming only what changed`.

## Stop

- No row polymorphism, width subtyping, structural records, or "record with at least these fields" type.
- No lenses, no generated per-field setters, no field-name values, and no first-class field access as a function.
- No mutation, no assignment, and no reference type.
- No update of a sum's variant, no update through an `Option`, and no nested-path update (`p.a.b = …`). A nested update
  is written as nested updates.
- No new core term, typing rule, reduction, normalization case, or cost-table entry.
- No change to `data` declaration, its generated fold, exact identity, or the storable-data admission rules.
- No adapter behavior change. The staff expansion produces the same items and its fixtures stay byte-identical.
