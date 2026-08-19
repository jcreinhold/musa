---
id: 143
slug: builtin-collapse
status: superseded
depends_on: [145]
phase: 3
---

> Superseded by the course correction: `docs/notes/research/language-design-closure/50-the-course-correction-audit.md`. Its builtin collapse is phase 3's retarget of the registry onto the surviving core.

# Collapse the Builtin Registry Behind Traits and Namespaces

## Task

The compiler owns 117 source operations and 14 phase operations, and a large fraction of them exist only because the
language had no way to overload a name. Traits, methods, and type namespaces exist now. Collapse
`nat_add`/`ratio_add`/`duration_add` into `Add`, `text_equal` into `==`, `duration_of`/`position_of` into
`Duration::of`/`Position::of`, and every other entry whose ownership entry no longer names anything the compiler
actually hides. Record what shrank and what did not, and why.

*Repaired ordering:* this prompt now follows the staff rewrite — prompt 145 was pulled ahead of it and of 144 when the
step-budget measurement said the rewrite could not wait for the cost table. That is the order prompt 140 already assumed
("prompt 143 removes what is dead after 145"): the collapse audits a registry the rewritten adapter calls.

## Read

- `crates/musa-compiler/src/core.rs`'s `BUILTIN_OWNERSHIP` and `SYNTAX_OWNERSHIP` — **every entry's `hidden_information`
  field**, one at a time. That field is the argument for the operation existing, and this prompt is the audit that field
  was written for: an entry whose hidden information is "arithmetic on two numbers" was never hiding anything, and an
  entry hiding the build-local registry or a private representation still is.
- `docs/rules/language/00-semantics.md`'s compiler-ownership paragraph — "an operation may be a builtin only when it
  needs source-aware provenance, direct core construction, a registered primitive's private state, or the private finite
  representation and work budget needed to preserve total evaluation." That is the four-way test each entry faces.
- `docs/rules/language/10-traits.md`'s operator table, and prompt [137a](137a-operators-and-methods.md)'s implementation
  — the replacements have to exist and be as fast, or this is a regression dressed as a cleanup.
- [`141l`](141l-qualified-path.md), which read `::` and moved the operator and index lowerings onto §1.5's qualified
  desugaring. Its Stop refused to declare the traits those spellings name on the grounds that doing so would answer this
  prompt's survey in advance; this is the prompt that answers it.
- Prompt [138](138-typed-syntax.md)'s registry survey, which already marked phase entries for deletion once their
  arguments became typed.
- `docs/rules/language/02-core-calculus.md` §5.8's four builtin families — collapsing entries must not change how many
  families there are, and an operation that moves from the compiler to `stdlib/` leaves the registry rather than moving
  between families.
- The `rust-performance` skill's workflow, and `docs/rules/language/06-performance.md`'s P1/P2 baseline. A dictionary
  indirection where there used to be a direct call is exactly the kind of change that is invisible in a microbenchmark
  and visible in a pipeline.

## Design

**The test is the ownership field, not the name.** An entry survives when it hides something a library could not: a
private representation, the build-local registry, source-aware provenance, direct core construction, or the work budget.
It goes when its hidden information turns out to be "how to add two numbers". Go through all 131 entries and record the
verdict for each — this is a survey with an answer per row, not a sweep that deletes what is easy.

**The spellings already exist; what is missing is what they name.** Prompt [141l](141l-qualified-path.md) moved the
operator and index readings onto `01-surface.md` §1.5's own desugaring — `x == y` is lowered as `Eq::equal(x, y)` and
`xs[i]` as `Index::at(xs, i)` — and read the `::` path that `Duration::of` is written with. So none of this prompt's
work is a change to `musa-compiler`'s lowering: `Eq`, `Ord`, `Add`, `Sub`, `Mul`, `Div`, and `Index` are names those
readings already write and nothing in `crate::registry::owned` declares. What this prompt adds is the declarations and
the instances, and a collapsed entry is measured by an operator that resolves rather than by a table that shrank.

**Some entries move to `stdlib/` rather than disappearing.** An operation that a library can now express belongs in the
library, written in Musa, where it can be read and improved. Say which moved, and check that the moved version is
covered by the same laws the builtin was.

**Measure the ones that get slower.** Replacing a direct builtin call with a dictionary projection is the standard cost
of this design, and the standard mitigation — resolving a known-concrete instance at elaboration time to a direct call —
is worth doing where the measurement says so and not before. Report P1 and P2 against `06-performance.md`'s baseline
under its 10% gate; a regression that this prompt causes is this prompt's to fix or to argue, not prompt 144's to
inherit.

**Say what did not shrink.** A survey that reports only the wins is not evidence. The entries that survived, and the
reason each survived, are the more useful half of the output, because they are the list a future reader will check
before proposing a new builtin.

**The privacy audit runs again.** Deleting a builtin can widen what source can observe. Every removed entry gets the
same check the core prompts applied: does anything now reachable from source reveal a private representation, a registry
decision, or a budget?

## Target

- The collapsed registries, with the operator, method, and namespace forms carrying the load.
- A per-entry survey — in `docs/plan/code-map/` or a note it links, whichever the code map's own structure wants —
  recording for each of the 131 entries whether it was kept, replaced, or moved to `stdlib/`, and the hidden information
  that decided it.
- `stdlib/` gaining the operations that left the compiler, with their laws.
- P1/P2 measurements against `06-performance.md`'s baseline, and any mitigation applied, measured.
- The privacy audit, recorded.
- `docs/rules/language/` repaired wherever it named an operation that no longer exists.

## Check

```sh
cargo build --workspace
cargo nextest run --workspace
cargo nextest run --run-ignored all
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
cargo bench -p musa-compiler -- p1_compile p2_elaborate
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
```

The two `nextest` lines still show the tonal budget class red — `diatonic-sequences` and `rule-of-the-octave`, named in
prompt 142's Check and closed at 144 — and nothing else. The staff class is green since 145.

The oracle stays fixed: a collapse that changes a semantic hash, a diagnostic code, or a rendered corpus file has
changed behaviour, and behaviour changes belonged to prompt 142.

Commit as `Collapse the builtin registry behind traits and namespaces`.

## Stop

- No new builtin. This prompt only removes and moves.
- No change to the four builtin families of §5.8, and no fifth registry.
- No behaviour change. Same values, same diagnostics, same rendered output.
- No deletion of an entry whose ownership field still names something a library cannot express, however tempting the
  symmetry.
- No performance work beyond mitigating a regression this prompt caused. Prompt 144 owns the checker's budget.
- No adapter rewrite. Prompts 145 and 146.
