---
id: 162d
slug: or-patterns
status: in-progress
depends_on: [162c]
phase: 3
---

# One Arm for Several Constructors

## Task

`match written { Bass | Tenor -> true, Treble | Alto -> false, }` is refused: `|` is not a pattern. So an arm that
answers the same thing for four constructors is written four times, and the corpus does exactly that —
`stdlib/src/adapters/staff.musa` and `stdlib/src/notation/staff.musa` both carry runs of arms whose bodies are
character-for-character identical. Add alternation to the pattern grammar, and amend
[`01-surface.md`](../../rules/language/01-surface.md) §1 to say so, because §1's list of what patterns cover does not
include it.

## Read

- `docs/rules/language/01-surface.md` §1's pattern paragraph — "Patterns cover booleans, naturals and other literal
  domains, empty/cons lists, products, enum constructors, and record fields." Alternation is not in the list, so this
  prompt amends the list. §1 also fixes what stays out — no guards, no conditional equations — and the amendment must
  leave those exactly as they are.
- `docs/rules/language/02-core-calculus.md` §6.2 — the case tree. An alternation is not a new elimination: it is one
  body reached from several branches, which is what a case tree's methods already are. What §6.2 has to gain is the
  sentence saying so, and the rule that the alternatives bind the same names at the same types.
- `docs/rules/README.md`'s amendment procedure. `docs/rules/language/` is candidate until prompt
  [193](193-language-conformance.md), so an amendment here is the ordinary kind and not a constitutional one — but it is
  still an amendment and this prompt's Task is to make it, which is why the Task says so first.
- `crates/musa-syntax/src/parser/patterns.rs`, and prompt [162c](162c-nested-patterns-parse.md), which this builds on:
  an alternative is a pattern, so alternation composes with nesting and `Wrap(Loud(n) | Quiet)` has to mean something or
  be refused for a stated reason.
- Peyton Jones ch. 5, *The Semantics of Pattern Matching*, §5.2's match compiler: the variable rule, the constructor
  rule, and the mixture rule. An alternation is the mixture rule with one right-hand side shared, and ch. 5 is where the
  binding condition below comes from. §5.4.1 is the per-leaf elaboration cost this prompt inherits and does not pay.
- `crates/musa-calculus/src/elaboration/case.rs`'s module documentation, and prompt
  [165](165-diagnostics-and-performance.md)'s Finding A — which prompt owns hoisting an arm's body out of the leaves it
  reaches, and the condition under which that is sound.
- `stdlib/src/adapters/staff.musa`'s `said_of`, `form_span`, and `form_refusal` — the measured callers, so the Check can
  name lines that become fewer. `clef_word` and `spelling_word` are *not* among them: their arms answer four different
  words, so alternation has nothing to say about them, and `stdlib/src/notation/staff.musa` carries no run of identical
  arms at all.

## Design

**Alternatives bind the same names at the same types, or the arm is refused.** `Loud(n) | Quiet` is refused because one
alternative binds `n` and the other does not, and the refusal says which name and which alternative lacks it. This is
Peyton Jones ch. 5's condition and it is not negotiable: the body is one expression and it is checked once, so the scope
it is checked in has to be the same whichever alternative matched.

**The common case binds nothing, and that is the case to make cheap.** `Bass | Tenor` is four constructors of an
enumeration with no fields, which is where the corpus's duplication actually is. It must cost one arm and one body.

**It is not a new core term.** An alternation compiles to a case tree whose methods for several constructors are the
same method, which §6.2's tree can already express. So `02-core-calculus.md` gains a sentence and the core gains no form
— the same shape prompt [161](161-one-declaration-form.md) used for `enum` and `record`.

**The arm is written once; its body is still elaborated once per leaf, and that is prompt 165's.**
`crates/musa-calculus/src/elaboration/case.rs`'s module documentation already states the standing cost — "An arm's body
is still elaborated once per leaf it reaches, which is Peyton Jones §5.4.1's remaining cost \[…\] prompt 165 owns the
change" — and prompt [165](165-diagnostics-and-performance.md)'s Finding A is the hoist that answers it, under the
condition 136b proved. An alternation reaches several leaves exactly the way a variable pattern in a split column
already does, so it inherits that cost rather than earning a second, differently-conditioned answer here; sharing one
body across leaves *here* would be 165 implemented early and under a weaker argument. What this prompt owes instead is
that an alternation is no worse than the arms it replaces: `Bass | Tenor -> e` emits the term `Bass -> e, Tenor -> e`
emits, which is a law below. Prompt [162f](162f-lazy-methods.md) is unaffected either way — it forces the method the
target chose and no other, whether or not two methods are one term.

**Exhaustiveness is unchanged.** An arm covering four constructors covers four constructors. Coverage counting is §6.2's
and the amendment says nothing new about it.

## Target

- `|` between patterns, at every position a pattern stands, including inside a nested one.
- The binding condition, with a refusal that names the name and the alternative.
- An alternation adds no core form and no method a separate arm would not have produced: `Bass | Tenor -> e` emits the
  term `Bass -> e, Tenor -> e` emits.
- `docs/rules/language/01-surface.md` §1 and `02-core-calculus.md` §6.2 amended, each saying what it now admits and what
  it still refuses.
- `editors/tree-sitter-musa` reads it, with a corpus entry.
- Laws: an alternation answers what the several arms answered *and emits the same term*; a binding disagreement is
  refused; an alternation of all a family's constructors is exhaustive; a nested alternation parses.
- One `stdlib/` caller converted as the evidence — `stdlib/src/adapters/staff.musa`'s `said_of`, whose four arms bind
  one name out of four constructors of different arities — with the line count before and after in the commit message.

## Check

```sh
cargo build --workspace
cargo nextest run -p musa-syntax -p musa-calculus -p musa-compiler
cargo nextest run --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo insta test -p musa-syntax -p musa-compiler --unreferenced=reject
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
```

```sh
printf 'library {\n  data Clef { Treble, Bass, Alto, Tenor }\n  fn low(c: Clef) -> Bool { match c { Bass | Tenor -> true, Treble | Alto -> false, } }\n}\n' > /tmp/alt.musa
cargo run -q -p musa -- check /tmp/alt.musa
```

`cargo nextest run --run-ignored all` and `cargo insta test --workspace` carry the reds prompt
[164](164-builtin-collapse.md)'s Check enumerates and [166b](166b-per-context-memo-stamp.md) owns; a *new* red under
either is this prompt's.

Commit as `Let one arm answer for several constructors`.

## Stop

- No guards. `Bass | Tenor if …` is the conditional equation §1 refuses, and alternation is not the door for it.
- No alternation in a *definition's* parameter list. There is still no pattern on the left of a definition.
- No `..` and no wildcard constructor set. `_` already discards.
- No sweep of `stdlib/`. One converted caller is the evidence; everything else in the adapters belongs to prompts
  [166](166-staff-rewrite.md) and [167](167-studio-rewrite.md).
- No change to what exhaustiveness means or to any diagnostic §6.2 names.
