---
id: 164
slug: builtin-collapse
status: pending
depends_on: [156, 166]
phase: 3
---

> **Reinstated and repaired by notes [`51`](../../notes/research/language-design-closure/51-the-terseness-audit.md) and
> [`52`](../../notes/research/language-design-closure/52-the-musical-algebra.md).** Note 50 superseded this prompt on
> the reasoning that the collapse was its phase 3. The collapse survives, with a larger target: the seventeen builtins
> hardcoded to the modulus 12 (`pc12_*`, `row12_*`) collapse onto the index of 142d, and the operations onto the traits
> of 142e. That is 12% of a 139-entry registry, and it is the count note 51 §3 uses as the evidence for the index.

# Collapse the Builtin Registry Behind Methods and Namespaces

## Task

The compiler owns 122 source operations and 18 phase operations, and a large fraction of them exist only because the
language had no way to overload a name. It has one now. Collapse `nat_add`/`ratio_add`/`duration_add` behind `+`,
`text_equal` behind `==`, `duration_of`/`position_of` behind `Duration::of`/`Position::of`, and every other entry whose
ownership entry no longer names anything the compiler actually hides. Record what shrank and what did not, and why.

**The mechanism is a namespace, not a trait.** The trait system was measured away at [143](143-one-theory-amendment.md)
and deleted at [146](146-delete-the-trait-system.md), and [`10-traits.md`](../../rules/language/10-traits.md) is retired
with no successor. What stands in its place is `01-surface.md` §1.5: an operator is surface syntax for a named function
— `x == y` is `x.equal(y)`, which is `Nat::equal(x, y)` at `x : Nat` — resolved by type-directed disambiguation against
the definitions of that name in scope. So there is no `Eq`, no instance, and no dictionary; there is
`impl Nat { fn equal(…) { … } }` beside the `impl Interval { fn compose(…) }` that `stdlib/src/pitch.musa` already
writes. An earlier statement of this prompt was written before 146 and named the traits; the collapse it asks for is
unchanged and the vehicle is this one.

**The seventeen modulus-12 entries are the largest single case, and they are collapsed here rather than at 142e.** That
prompt named the algebra; this one supplies the carrier it acts on. `Pc12`, `PcSet12`, and `Row12` become `Pc(n)`,
`PcSet(n)`, and `Row(n)` over 142d's index and 142f's declaration form; `Group<Ti>`, `Action<Pc(n), Ti>`, and
`Torsor<Pc(n), Ic(n)>` are `stdlib/src/algebra.musa`'s records at those carriers — ordinary values, since 143 made a
structure a record rather than a trait; and `orbit` and `stabilizer` — ordinary functions of a finite action, taking the
modulus as the number it is — replace `row12_symmetries`, `row12_forms`, and `row12_matrix`. Note 52 §2.3 is the
argument that those three builtins are one question asked three times.

*Repaired ordering:* this prompt now follows the staff rewrite — prompt 166 was pulled ahead of it and of 144 when the
step-budget measurement said the rewrite could not wait for the cost table. That is the order prompt 140 already assumed
("prompt 164 removes what is dead after 145"): the collapse audits a registry the rewritten adapter calls.

*And the frontmatter now says so.* Until prompt 162a's run this prompt named only 156 while 166 named this one, so the
two prompts each claimed to follow the other and the selector took whichever number was smaller. The prose above and
166's own Read — the step-budget measurement, in full — are the record of which way the repair went, and `depends_on`
has been made to agree with them. The consequence is checkable rather than editorial: the Check below asserts a green
staff budget class, which is 166's to deliver.

## Read

- `crates/musa-compiler/src/phase/mod.rs`'s `BUILTIN_OWNERSHIP` and `SYNTAX_OWNERSHIP` — **every entry's
  `hidden_information` field**, one at a time. That field is the argument for the operation existing, and this prompt is
  the audit that field was written for: an entry whose hidden information is "arithmetic on two numbers" was never
  hiding anything, and an entry hiding the build-local registry or a private representation still is.
- `docs/rules/language/00-semantics.md`'s compiler-ownership paragraph — "an operation may be a builtin only when it
  needs source-aware provenance, direct core construction, a registered primitive's private state, or the private finite
  representation and work budget needed to preserve total evaluation." That is the four-way test each entry faces.
- `docs/rules/language/01-surface.md` §1.5's operator paragraph — the operator table's successor, since
  [`10-traits.md`](../../rules/language/10-traits.md) is retired — together with prompt
  [137a](137a-operators-and-methods.md)'s implementation and [146](146-delete-the-trait-system.md)'s replacement of
  instance lookup by type-directed disambiguation. The replacements have to exist and be as fast, or this is a
  regression dressed as a cleanup. §1.5's two guard rules are load-bearing here: an operator resolves only when the
  expected type or the head argument's type is known, and an operation that can fail keeps its failing shape, so
  `ratio_div`'s `Result` survives becoming `/`.
- `docs/rules/language/03-musical-domains.md` §5, the governing statement of the indexed domains this prompt supplies:
  `Cyclic(n)`, `Pc(n)`, `Ic(n)`, `Row(n)`, `Icv(n)`, and what each means. `pc12` is `Pc(12)` and `Row12` is `Row(12)`;
  the §4 definitions are unchanged and are the `n = 12` reading of these.
- [`141l`](141l-qualified-path.md), which read `::` and moved the operator and index lowerings onto §1.5's qualified
  desugaring. Its Stop refused to declare what those spellings name on the grounds that doing so would answer this
  prompt's survey in advance; this is the prompt that answers it.
- Prompt [138](138-typed-syntax.md)'s registry survey, which already marked phase entries for deletion once their
  arguments became typed.
- `docs/rules/language/02-core-calculus.md` §5.8's four builtin families — collapsing entries must not change how many
  families there are, and an operation that moves from the compiler to `stdlib/` leaves the registry rather than moving
  between families.
- [`52-the-musical-algebra.md`](../../notes/research/language-design-closure/52-the-musical-algebra.md) §2.3 and §5, and
  `docs/rules/language/03-musical-domains.md` §5. §2.3 is the observation that a set class, Messiaen's modes, and a
  row's symmetries are orbit and stabilizer and nothing else; §5 is what the index buys, counted. §5's falsifier — the
  post-tonal rewrite for arbitrary n, substantially shorter than today's 213 lines plus 17 builtins — is this prompt's
  to pass or to fail.
- The `rust-performance` skill's workflow, and `docs/rules/language/06-elaboration-baseline.md`'s P1/P2 baseline. A
  dictionary indirection where there used to be a direct call is exactly the kind of change that is invisible in a
  microbenchmark and visible in a pipeline.

## Design

**The test is the ownership field, not the name.** An entry survives when it hides something a library could not: a
private representation, the build-local registry, source-aware provenance, direct core construction, or the work budget.
It goes when its hidden information turns out to be "how to add two numbers". Go through all 140 entries and record the
verdict for each — this is a survey with an answer per row, not a sweep that deletes what is easy.

**The spellings already exist; what is missing is what they name.** Prompt [141l](141l-qualified-path.md) moved the
operator and index readings onto `01-surface.md` §1.5's own desugaring and read the `::` path that `Duration::of` is
written with. So none of this prompt's work is a change to `musa-compiler`'s lowering: `equal`, `less`, `add`, `sub`,
`mul`, `div`, and `at` are the names those readings already write, and at a `Nat` receiver nothing declares any of them
— `1 == 1` today is `no method 'equal' for 'Nat'`. What this prompt adds is the `impl` blocks that declare them on each
carrier, and a collapsed entry is measured by an operator that resolves rather than by a table that shrank.

**Some entries move to `stdlib/` rather than disappearing.** An operation that a library can now express belongs in the
library, written in Musa, where it can be read and improved. Say which moved, and check that the moved version is
covered by the same laws the builtin was.

**Measure the ones that get slower, and expect a different cause from the one this prompt first named.** There is no
dictionary to project through: 146 resolves a method to a named function at elaboration time, so `a + b` at a known
`Nat` elaborates to the call `nat_add(a, b)` already was, wrapped in whatever λ the `impl` body puts around the builtin.
What is left to cost is that λ and the disambiguation walk, and
[note 60](../../notes/research/language-design-closure/60-the-staff-rewrite-measured.md) §3 is the standing warning that
a wrapper an author cannot see is charged to every call that reaches it. Report P1 and P2 against
`06-elaboration-baseline.md`'s baseline under its 10% gate; a regression that this prompt causes is this prompt's to fix
or to argue, not prompt 165's to inherit.

**Say what did not shrink.** A survey that reports only the wins is not evidence. The entries that survived, and the
reason each survived, are the more useful half of the output, because they are the list a future reader will check
before proposing a new builtin.

**The privacy audit runs again.** Deleting a builtin can widen what source can observe. Every removed entry gets the
same check the core prompts applied: does anything now reachable from source reveal a private representation, a registry
decision, or a budget?

## Target

- The collapsed registries, with the operator, method, and namespace forms carrying the load.
- A per-entry survey — in `docs/plan/code-map/` or a note it links, whichever the code map's own structure wants —
  recording for each of the 140 entries whether it was kept, replaced, or moved to `stdlib/`, and the hidden information
  that decided it.
- `stdlib/` gaining the operations that left the compiler, with their laws.
- `stdlib/src/post_tonal/` rewritten over `Pc(n)`: `Group<Ti>`, `Action<Pc(n), Ti>`, and `orbit`/`stabilizer` as
  ordinary functions, with the modulus reaching them as the number it is.
- `examples/`: the two fixtures the Check names.
- P1/P2 measurements against `06-elaboration-baseline.md`'s baseline, and any mitigation applied, measured.
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

**The staff class is green**, because [165b](165b-graph-update-and-data-descent.md) and then 166 ran first; the thirty
staff budget failures the suite carried through 162a are those two prompts' to close between them, and a red one here
means this prompt was started out of order. `cargo nextest run --workspace` is green outright.

What `--run-ignored all` still shows is four rows, measured at prompt 166's commit and **all four
[165](165-diagnostics-and-performance.md)'s**, whose Target already names each of them:

| Test | What it reads |
| --- | --- |
| `musa::cli wav_export_is_deterministic_for_all_examples` | every example, and it dies on `examples/diatonic-sequences.musa` |
| `large_score_generators::*` (2) | `tests/fixtures/large-score.musa`, 1500 events, no adapter |
| `elaboration_fixture_generators::the_pressure_workloads_compile_and_denote_what_they_claim` | `core-pressure.musa`, no adapter |

An earlier statement of this paragraph named "the tonal budget class — `diatonic-sequences` and `rule-of-the-octave` …
and nothing else". Half of that is wrong twice over: `rule-of-the-octave` compiles, and the two `large_score` rows and
`core-pressure` are neither tonal nor closed. Prompt 166's Check carries the same correction and the same measurement.

`cargo insta test --workspace --unreferenced=reject` is red for `diatonic-sequences` too, and for one more reason that
is [166b](166b-per-context-memo-stamp.md)'s: it runs libtest rather than nextest — one process, many threads — where the
δ-unfolding memo's process-global invalidation stamp makes a compilation's step count depend on what else the process is
compiling ([note 60](../../notes/research/language-design-closure/60-the-staff-rewrite-measured.md) §6). Neither red is
this prompt's, and a *new* red under either runner is.

The oracle stays fixed: a collapse that changes a semantic hash, a diagnostic code, or a rendered corpus file has
changed behaviour, and behaviour changes belonged to prompt 142.

And the musical check the moved half brings with it: `orbit` under `Action<Pc(n), Ti>` at modulus 12, applied to the
committed set-class fixtures, reproduces `102-set-class-and-prime-form.md`'s prime forms, and `stabilizer` applied to
the whole-tone and octatonic collections reproduces `106-collections.md`'s modes of limited transposition. Both as
fixtures in `examples/`, both cited by `make docs-check`'s theory-citation pass.

Commit as `Collapse the builtin registry behind methods and namespaces`.

## Stop

- No new builtin. This prompt only removes and moves — `orbit` and `stabilizer` arrive in `stdlib/`, written in Musa, or
  the three they replace have not actually left.
- No change to the four builtin families of §5.8, and no fifth registry.
- No behaviour change. Same values, same diagnostics, same rendered output.
- No deletion of an entry whose ownership field still names something a library cannot express, however tempting the
  symmetry.
- No performance work beyond mitigating a regression this prompt caused. Prompt 165 owns the checker's budget.
- No adapter rewrite. Prompts 166 and 167.
