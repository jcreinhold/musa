---
id: 141o
slug: document-elaboration
status: done
depends_on: [141g, 141i, 141k, 141l, 141n]
phase: 3
---

# Elaborate a Whole Document

## Task

Thirteen prompts built a reading, a vocabulary, and four doors into `musa-calculus`, and
[`crate::lower`](../../../crates/musa-compiler/src/lower.rs) has no caller outside its own laws. Nothing walks a
*document*: nothing decides which of [`musa_calculus::declare`](../../../crates/musa-calculus/src/lib.rs),
`declare_trait`, `declare_impl`, and `declare_program` a written declaration goes through, nothing orders the family
groups, and so nothing has ever handed the new checker a real file.

Write that walk, and use it to say what the corpus still needs.

## Read

- [`141g`](141g-raw-lowering.md)'s `Item` — four variants "because `musa-calculus` has four doors … and this is the type
  that says which one a written declaration goes through. A caller matches once and calls". This prompt is that caller.
- [`141n`](141n-top-level-program.md)'s `declare_program` and `Cx::defining`, and its Design's first paragraph: the
  program is a group and the group is the door. What it left is "building the group out of 141g's items and handing it
  over", which is one of this prompt's two halves.
- [`141e`](141e-compiler-registry.md)'s [`registry::owned`](../../../crates/musa-compiler/src/registry.rs) — the context
  every document is elaborated in, already assembled in the four stages its module documentation writes out. This prompt
  adds nothing to it and calls it once per document.
- [`141k`](141k-notation-lowering.md) and [`141l`](141l-qualified-path.md), whose readings the survey below exercises
  for the first time on real files rather than on written-out laws.
- [`142`](142-surface-cutover.md)'s Design, in particular **"Order the work so the migration is mechanical"** —
  elaborate through `musa-calculus` first and get the corpus passing "with the old spellings still in place; then
  migrate spellings; then delete", because "mixing the three makes every failure ambiguous between 'the new checker is
  wrong' and 'this file was translated wrong'". This prompt is the first of those three and nothing else. Its own
  precedent is named there: 141e took the registry slice one prompt earlier for the same reason, since "a table of 117
  builtins is a *translation* with an oracle to check it against, and burying it inside a diff that also moves 11,304
  lines of `.musa` would have made a wrong signature indistinguishable from a wrong migration". A walk that has never
  been run on a real file is the same wager one level up.
- `crates/musa-compiler/src/core/mod.rs`'s `check_piece`, `check_arguments`, `check_template_voice`, and
  `check_material`, and the `data_owners` and `module_owners` helpers beside them. They are the shape of the argument
  this walk takes — a document is its imports, its own root, and the piece or voice being checked — and they are what
  142 replaces. Read them for the *argument*, not for the checker.
- Peyton Jones **ch. 6 §6.2.8** again, for the half 141n did not need. Dependency analysis is not only about
  definitions: a `data Chord { root: NoteName; }` written above the `data NoteName` it names is the same problem with
  the same answer, and the core cannot solve it because its own mutual-recursion door is one group with shared
  parameters — which two written `data` declarations do not have.
- The `module-design` skill's audit questions. This is a new boundary, so the public surface is stated before it is
  implemented and each item on it has a caller in this prompt.

## Design

**A document is a list of sources, and a source is a node plus one bit.** The node is whatever holds declarations as its
children — a `library`, a document root, a `piece`, a `voice` — and the bit is whether `02-core-calculus.md` §5.9's
phase vocabulary is spellable inside it, which is the one fact lowering cannot read off the node. Import *order* is not
this module's: `crate::imports` already answers it, and a walk that learned about import graphs would be doing name
resolution a second time.

**The order the four doors open in is forced, and it is written down rather than discovered.** Families first, because a
field is a type; traits next, because a method type may name any family; the definitions as one group, because §2.4's
forward reference is a property of the group and not of a written order; instances last, because an `impl`'s method
bodies are ordinary terms that may call any definition. The one shape this cannot express is a family whose field names
a `record`, since `01-surface.md` §1.2 makes a record a definition — stated as a limit, refused by the core at the
field, and written by nothing in this repository.

**Family groups are ordered by their written identifiers, not by their raw terms.** Every type name a `data` declaration
depends on is an identifier token somewhere under its node, so reading the tokens finds all of them — fields, indices, a
parameter's own bound — in one pass that cannot fall behind a `Raw` shape added later. It over-approximates, and only in
the safe direction: a spurious edge is a spurious *cycle*, and a cycle is refused rather than silently reordered.

**The survey is the deliverable, not a diagnostic aid.** Elaborating the standard library through the new checker and
recording exactly what it still refuses is what makes 142's migration mechanical: every remaining fault is either a
spelling the migration owns or a defect in one of the thirteen prompts, and until the walk exists nobody can tell which.
The recorded list is a law, the way `registry::rules::UNREGISTERED` is a law — a file that starts failing for a new
reason fails the build, and a file that stops failing has to be struck from the list in the commit that fixed it.

**No caller.** 142 owns the readback out of normal forms, the four passes, and the deletion of what they replace, and
this prompt stops at the door for 141g's reason one level up: a walk introduced in the same commit that deletes the old
checker would make a wrong walk indistinguishable from a wrong migration.

## Target

- `crates/musa-compiler/src/document.rs`: `Source`, an opaque `Document`, and `elaborate`, each doc-commented with the
  rule it implements. `Document` answers two questions and no others — what a name denotes as a normal form at its own
  type, and where an origin was written — because those are the two 142's readback needs and nothing else has a caller.
- Dependency ordering over the family groups, with a cycle refused under `Code::DependencyCycle` naming the declarations
  on it.
- Every refusal restated through 141g's `refusals::restate`, so a caller never sees an `ElabError` and never learns what
  an `Origin` is.
- Laws beside the module: ordinary definitions elaborate and read back; a body may name a definition written after it; a
  `data` declaration may be written after the one that names it; two that name each other are refused; the phase
  vocabulary is readable in a phase source and not in an ordinary one.
- **The survey**, as a law: `stdlib/`'s sixteen non-adapter libraries elaborated as one document, and each adapter
  elaborated in phase scope, with the remaining faults recorded exactly and each one named as something
  [`142`](142-surface-cutover.md)'s Target already owns.
- A `docs/plan/code-map/spec-to-implementation-map.md` row for the walk.

## Check

```sh
cargo build --workspace
cargo nextest run -p musa-compiler
cargo clippy --all-targets -p musa-compiler -- -D warnings
cargo fmt --check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
python3 scripts/renumber-prompts.py audit
```

Commit as `Elaborate a whole document`.

## Stop

- **No caller and no deletion.** `core.rs`, `infer.rs`, and the four passes are untouched; 142 owns all of them.
- **No readback.** A normal form is answered as a `Term`; turning one into a score fact is 142's.
- **No migration.** Not one line of `.musa` changes. The survey *records* what the corpus needs and repairs nothing —
  that is the whole of why it is worth having.
- No import resolution, no package loading, no module system. A caller supplies the sources in order.
- No `examples/`. They are pieces rather than libraries, so surveying one means walking a `piece` and its templates,
  which is document *structure* and belongs to 142 with the rest of it.
- No performance work. The context is rebuilt per document and 144 measures it.
- No new language feature. If the survey wants one, that is a finding for 142 and it is far better recorded than slipped
  in here.
