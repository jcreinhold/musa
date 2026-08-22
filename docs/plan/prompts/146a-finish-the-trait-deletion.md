---
id: 146a
slug: finish-the-trait-deletion
status: in-progress
depends_on: [146]
phase: 3
---

# Finish Deleting the Trait System: the Surface Specification and the Dead Codes

## Task

Prompt 146 deleted the trait mechanism — `class.rs`, `dictionary.rs`, the prelude's trait region, the `trait` keyword,
and the `where` clause — and put type-directed disambiguation in its place. Three things it described survived it: the
surface specification still writes `trait`, `where`, and instance lookup into its normative grammar and its conformance
table; twenty diagnostic codes with no producer anywhere in the workspace are still listed by `musa explain`; and the
long-form explanations of the three *live* method diagnostics still explain method resolution in terms of instances and
dictionaries. Repair the specification, then delete the residue.

This is a repair found late, which is why it is numbered where it belongs and executed after 154. It carries the rank it
would have had, not the one it was written at.

## Read

- `docs/rules/README.md`, "Changing a decision" — the six requirements bind `constitution.md` and `obligations.md`; the
  other pages, `language/` among them, are amendable "in the ordinary way — a prompt that repairs them, committed before
  the code changes". This prompt is that sentence: the rules commit lands first and the code commit second.
- `docs/rules/language/01-surface.md` §1 (the grammar block), §1.4, §1.5, and the conformance table at the end. §1.4 is
  written in the future tense about a prompt that has run, and §1.5's opening sentence describes the mechanism 146
  deleted, two paragraphs after §1.4 says it is gone.
- `crates/musa-syntax/src/keywords.rs` — the evidence that the grammar is stale, and the drift law's own side of it.
  There is no `trait` keyword and no `where` keyword. `IMPL`'s doc says what the surviving form is: "`impl T { … }`
  opens `T`'s namespace … The block is a prefix and nothing more — there is no dispatch."
- `crates/musa-calculus/src/elaboration/namespace.rs`, the module doc in full — what replaced instance lookup, in the
  words this prompt's diagnostics should be written in: a namespace is a dotted name, `impl Pitch { fn act(…) }`
  declares `Pitch.act`, and `Pitch::act(p, i)`, `p.act(i)` and the operator spelling are three ways to write one lookup.
- `crates/musa-score/src/diagnose.rs` — `code_table!`, which generates `as_str` and `ALL` from one table, so a row
  deleted there is a code gone from `musa explain` and from `Code::parse`.
- `crates/musa-project/src/diagnostic.rs`'s `explain`, and its two laws: `every_code_has_an_explanation` and
  `a_listed_code_is_a_code_explain_answers_about`. They hold the table and the explanations together in one direction
  each; neither can see that a code has no producer.
- `docs/plan/prompts/165-diagnostics-and-performance.md` — the prompt that makes the *surviving* codes good. This one
  only removes the ones that cannot be reached; it does not touch a message 165 will rewrite.

## Design

**The count, taken the way 146's was.** Every variant of `musa_score::Code` was extracted from the enum and grepped for
as `Code::<name>` across `crates/`, excluding the two files that define the code and explain it. **Twenty have no
producer.** Nineteen are the trait system's: `ReservedClass`, `HeadlessClass`, `ConstrainedField`, `SuperClass`,
`ConstrainedInstance`, `ConstrainedData`, `DuplicateMethod`, `ClassArity`, `HandWrittenStorable`, `BlanketInstance`,
`DuplicateInstance`, `OrphanInstance`, `DerivedMethod`, `NoSuchMethod`, `MissingMethod`, `UnresolvedInstance`,
`UnconstrainedVariable`, `UnkeyedConstraint`, and `DuplicateConstraint`. No pending prompt names any of them; the one
mention anywhere in `docs/plan/prompts/` is 142e's, which is `done` and historical.

**The twentieth is `SpliceCategory`, and it is dead for a different reason.** Quotation is live, and its category rule
is enforced — but as an ordinary conversion failure. `quotation_laws::a_splice_of_the_wrong_category_names_both_
categories` asserts exactly that: the report names `` `TokenTree` `` and `` `Expr` ``, which is a type mismatch between
two applications of `Syntax`, not a code of its own. Prompt 159 makes `Syntax` a family indexed by `Cat`, which is the
direction that keeps it a type mismatch. The code is deleted with the other nineteen and the *rule* it explained is
untouched.

**Two Storable codes go with them, and storability does not.** `ReservedClass` and `HandWrittenStorable` policed a
`trait Storable` nobody can declare now and an `impl Storable` there is no form to write. The structural rule is
enforced by `Refusal::NotStorable`, which is live and keeps its own code: what is deleted is the pair of reports about a
mechanism, never the rule about a type.

**The three live method codes keep their codes and lose their prose.** `MethodOnVariable`, `NoMethodForType` and
`AmbiguousMethod` are produced by `elab/infer.rs` and `elab/name.rs` today, and all three explanations describe traits,
dictionaries and coherence. `AmbiguousMethod` is the one that is wrong about *what it reports*, not only about why: it
is not "two traits declare a method alike" but a bare member spelling that the expected type did not narrow to one
namespace — and it also fires when narrowing comes back **empty**, which the old text cannot account for at all.
`RedundantNamePrefix` is a fourth: it exempts "a trait instance", where `lint.rs` exempts an `impl` whose head is not a
plain type name.

**What the specification says after the repair.** The normative grammar loses `trait`, `trait-item`, the trait-headed
`impl`, `where-clause` and `constraint`, and `inherent` is renamed to `impl` because it is the only `impl` there is —
`visibility? "impl" type "{" function* "}"`, which is what `declarations.rs`'s `impl_decl` reads. §1.4 becomes a record
of a completed deletion rather than a notice of a pending one. §1.5's opening resolves a method in a namespace. The
conformance table loses the seven trait rows, and the three rows that describe *live* features in trait vocabulary — the
operator at a known head, indexing, and the method call — are restated in the vocabulary of §1.5's disambiguation.

**What is deliberately not done here.** No message text changes for a surviving code beyond the four above; 165 owns
that. No new law that greps for a producer per code: the check below catches it for this prompt, and a permanent law
wants a home that can see the whole workspace, which no single crate's test binary can.

## Target

Two commits, in this order, because `docs/rules/README.md` requires the rules repair to land before the code changes.

1. `docs/rules/language/01-surface.md`: the grammar block, §1.3's type list, the type-parameter paragraph, the `private`
   paragraph, §1.4 in full, §1.5's opening and its first bullet, `T::x`'s sentence and the path-reading rule, and the
   conformance table's trait rows — deleted where the form is gone, restated where the feature is live.
2. `crates/musa-score/src/diagnose.rs`: twenty rows out of the `code_table!` table, and the stale doc comments on
   `NoMethodForType`, `AmbiguousMethod` and `QualifiedPath` repaired. `crates/musa-project/src/diagnostic.rs`: the
   twenty explanations deleted, and the four rewritten.

## Check

```sh
cargo nextest run -p musa-score -p musa-project    # or cargo test
cargo clippy --all-targets -p musa-score -p musa-project -- -D warnings
make fmt-check
make docs-check
```

And the count that motivated the prompt, which must come back empty:

```sh
python3 - <<'PY'
import re, subprocess, pathlib
src = pathlib.Path("crates/musa-score/src/diagnose.rs").read_text()
body = re.search(r"pub enum Code \{(.*?)\n\}", src, re.S).group(1)
variants = re.findall(r"^    ([A-Z][A-Za-z0-9]*),$", body, re.M)
out = subprocess.run(["grep","-rn","--include=*.rs","-o",r"Code::[A-Za-z0-9]*","crates"],
                     capture_output=True, text=True).stdout
used = {line.split("Code::")[1] for line in out.splitlines()
        if "musa-score/src/diagnose.rs" not in line and "musa-project/src/diagnostic.rs" not in line}
print([v for v in variants if v not in used] or "every code has a producer")
PY
```

## Stop

- No change to a surviving code's message or explanation beyond the four this prompt names. Prompt 165 owns the rest.
- No new diagnostic code, and no renaming of one that survives.
- No change to `02-core-calculus.md`, whose account of the core is not what went stale here.
- No permanent law enumerating producers. The check above is this prompt's; a law needs a home that sees every crate.
