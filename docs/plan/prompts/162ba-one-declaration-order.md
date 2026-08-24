---
id: 162ba
slug: one-declaration-order
status: done
depends_on: [141o, 142f]
phase: 3
---

# Order a Document's Declarations by One Analysis, Not by Kind

## Task

The index expression a constructor chooses cannot name anything the document declares. `data Vect<A>(n: Nat) { Nil :
(0), Cons(m: Nat, head: A, tail: Vect<A>(m)) : (m + 1), }` — the example
[`02-core-calculus.md`](../../rules/language/02-core-calculus.md) §1.1 introduces indexed families with — is not
writable in source:

```sh
$ cat ix.musa
library {
    impl Nat {
        fn add(left: Nat, right: Nat) -> Nat { nat_add(left, right) }
    }
    let b: Nat = 1 + 2;
    data Vect<A>(n: Nat) {
        Nil : (0),
        Cons(m: Nat, head: A, tail: Vect<A>(m)) : (m + 1),
    }
}
$ musa check ix.musa
  × no method `add` for `Nat`
   ╭─[ix.musa:8:47]
 8 │         Cons(m: Nat, head: A, tail: Vect<A>(m)) : (m + 1),
   ·                                                    ──┬──
```

The term-level `1 + 2` two lines above resolves through the same `impl Nat`. Nothing about the index is different — an
index is an ordinary term (§1) and `chosen_indices` lowers it with the ordinary `Lowering::expr` — so the two are the
same call reaching two different contexts. The plain form says so with less noise: `fn bump(subject: Peano) -> Peano {
Succ(subject) }` beside `There(q: Peano) : (bump(q))` reports `cannot find bump`.

Make the order a document declares things in one dependency analysis over all of them, and delete the order fixed by
kind.

## Read

- [`141o`](141o-document-elaboration.md)'s Design, third paragraph, which is what this prompt repairs: "**The order the
  four doors open in is forced, and it is written down rather than discovered.** Families first, because a field is a
  type; … the definitions as one group … instances last." True of fields and false of indices, and 141o could not have
  known: it landed before [`142c`](142c-index-amendment.md) amended the calculus and before
  [`142f`](142f-writable-index.md) made an indexed type writable at all. Its fourth paragraph — the token-level reading
  of a `data` node's dependencies — is repaired with it, for the reason under **Design** below.
- The same Design's stated limit: "The one shape this cannot express is a family whose field names a `record`, since
  `01-surface.md` §1.2 makes a record a definition — stated as a limit, refused by the core at the field, and written by
  nothing in this repository." It is the same defect one kind over, and it is the evidence that ordering by kind was the
  mistake rather than the index being a special case.
- [`02-core-calculus.md`](../../rules/language/02-core-calculus.md) §1, *Dependency is what Π means*: "a family's index
  may be any term of the index's type — a call, a projection, a value the program computed". And §1.1's `Vec` example,
  which writes `n + 1` in a constructor's result. The specification is not silent here and the code disagrees with it,
  which is the condition root [`AGENTS.md`](../../../AGENTS.md) names: "either the code is wrong or the document needs a
  deliberate repair — never let them drift silently." The code is wrong.
- [`01-surface.md`](../../rules/language/01-surface.md) §1.5, for why `m + 1` is a method call at all: an operator *is*
  a named function, `x + y` is `x.add(y)`, resolved at the head of the receiver's type. That reading is correct and this
  prompt does not touch it. What was missing is a context in which the lookup can succeed.
- [`143`](143-one-theory-amendment.md) and [`151`](151-delete-the-index-stratum.md), so the repair that is *not* wanted
  is named before it is proposed: there is no index stratum and an index has no arithmetic of its own. 143 refused a
  solver the checker consults rather than implements, 151 deleted the one that existed, and re-introducing an
  index-level `+` would be that class again. An index is an ordinary term and resolves like one.
- `crates/musa-calculus/src/elaboration/declare_program.rs` — `graph`, `Written`, `free`, `ordering`, `Held`. The
  analysis this prompt widens already exists, is precise, and reads the `Raw` shapes under a binder stack. `Held` is a
  one-variant enum, which is the shape it was left in.
- Peyton Jones **ch. 6 §6.2.8** and **ch. 8**, cited by 141o and by `kernel/program.rs` for the definitions' half:
  dependency analysis sorts declarations "into minimal groups" and has to run *before* type-checking. The half neither
  citation drew is that "declarations" is not "definitions" — a group's members are whatever can name each other, and in
  this language that is both kinds.

## Design

**One analysis, because the dependency runs both ways.** A `data` field is a type, so a definition may have to wait for
a family; a `data` index is a term, so a family may have to wait for a definition. Neither kind can come first, and a
caller holding two lists can join them only by guessing. So the two travel in one `RawProgram` and one `declare_program`
call computes the order over both — which is the same answer §2.4 already gave for definitions and instances, and the
same answer 141r gave when it deleted the fourth door for making every definition blind to its own document's instances.

**The core owns the reading, and the token pass goes.** 141o read a `data` node's dependencies off its identifier
tokens, arguing that a term walk "would mean a second walk of nineteen shapes that has to stay in step with the core".
The argument was right about the cost and wrong about who pays it: the core *has* that walk — `free`, precise, under a
binder stack — and the second copy was the CST pass. It also stops being safe at this width. Over-approximation was
harmless while the graph held only type names; a graph that holds every definition's name too turns a `record` field
called `count` and a `fn count` into a spurious edge, and a spurious edge is a spurious cycle, and a cycle is refused.
So the edges come off the `Raw` shapes, and `Written` grows one method that walks a `RawData` under the binders
`declare` opens: the families, the group's parameters, each family's indices, then each constructor's fields, with the
chosen indices read under the fields and not under the index telescope — which is the scope `declare` gives them.

**A group binds more than one name.** A definition binds one; a `data` group binds one per family and one per
constructor, twice for a constructor because `01-surface.md` §1.3 lets a use site write `Succ` or `Peano.Succ`. So the
graph's table is name-to-node rather than node-to-name, and the family groups take the first positions in an index space
the definitions continue.

**A cycle is refused wherever it runs, and it is refused once.** `Refusal::DefinitionCycle` says *declarations* rather
than *definitions* now, and `document.rs`'s own `Code::DependencyCycle` — a second cycle check that could not see the
definitions' half — is deleted. The help sentence moves to `refusals::restate`, where the surface knows the repair the
core cannot name: two families that need each other are one `data` declaration, and two definitions that need each other
are refused outright, because §2.4 admits recursion through the measure rather than through the graph.

**One refusal ends the walk.** Previously a bad `data` set a flag and the walk continued, so two bad `data` declarations
were two diagnostics, while a bad definition returned at once. One group means one behaviour, and it is the
definitions': the first refusal returns. That is a real loss of one diagnostic in one shape and it is the honest price
of the group — a declaration after a refused one is being checked against a context that is missing what it was supposed
to hold, and the second report would be about the hole rather than about the program.

**`Program` carries the families it declared.** Not a second value beside it: the order the two kinds were elaborated in
is the program's own, and a caller putting the families in with a second call could only put them in a different one.
`Cx::defining` brings both into scope, and `Program`'s field is `pub(crate)`, so no caller outside the crate changes.

**Not in scope, and named so it is not mistaken for a gap.** `Nat.add` still does not exist in the prelude, so `ix.musa`
above passes only because it writes its own `impl Nat`. Giving `+` a meaning with no `impl` in the document is prompt
[`164`](164-builtin-collapse.md)'s, and it is a *different* defect that this one uncovers rather than causes.

## Target

- `crates/musa-calculus/src/elaboration/raw.rs`: `RawGroup` — a `RawData` beside the module it was written in, for the
  reason `RawTopLevel::module` already gives one word over — and `RawProgram::families`.
- `crates/musa-calculus/src/elaboration/declare_program.rs`: `Held::Family`; `graph` over both kinds in one index space,
  with a name-to-node table; `Written::walk_data`; the elaboration loop dispatching a group to `elaboration::declare` at
  the position the order puts it in.
- `crates/musa-calculus/src/kernel/program.rs`: `Program::families`, `pub(crate)`.
- `crates/musa-calculus/src/lib.rs`: `declare_metered` deleted. Its one caller was the document walk's family loop, and
  a public item with no caller is a public item this repository does not keep. `declare` stays — a law that declares one
  group and asks the kernel about it is asking about the declaration and not about a document.
- `crates/musa-calculus/src/kernel/context.rs`: `Cx::defining` brings the families into scope with the definitions.
- `crates/musa-calculus/src/elaboration/refuse.rs`: `DefinitionCycle` says *declarations*.
- `crates/musa-compiler/src/document.rs`: `Read::families`; `record`s filed as the definitions they are; one
  `declare_program_metered` call. `TypeDecl`, `order_types`, `visit`, and `identifiers` deleted.
- `crates/musa-compiler/src/lower/refusals.rs`: the cycle's help sentence, which the core cannot write.
- Laws in `crates/musa-compiler/src/document/laws.rs`: a chosen index calls a definition; a chosen index calls a method
  of this document, written as §1.1's `Vect`; a `data` field names a `record`, which 141o recorded as a limit; a family
  and a definition that name each other are refused naming both.
- `docs/plan/code-map/spec-to-implementation-map.md`: both rows repaired — the §2.4 door and the document walk.

## Check

```sh
cargo build --workspace
cargo nextest run -p musa-calculus -p musa-compiler
cargo nextest run --workspace
cargo clippy --all-targets --workspace -- -D warnings
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
python3 scripts/renumber-prompts.py audit
```

And the shape the prompt exists for, which no law can state because it is a claim about the CLI:

```sh
cargo run -q -p musa -- check ix.musa   # the Task's file: ok
```

Commit as `Order a document's declarations by one analysis, not by kind`.

## Stop

- **No index arithmetic.** An index is an ordinary term and resolves through the ordinary namespaces. 143 and 151 are
  the reason, and a `+` that meant something else inside `(…)` would be the stratum they deleted, wearing a smaller hat.
- **No prelude vocabulary.** `Nat.add`, `Nat.less`, and the rest of the operator table are
  [`164`](164-builtin-collapse.md)'s. A document that writes no `impl Nat` still reports `no method add for Nat`, and
  that is 164's row to strike.
- **No change to method resolution.** `01-surface.md` §1.5's reading — head of the receiver's type, one name — is
  untouched. This prompt changes which definitions are in scope when it runs, and nothing about how it runs.
- **No new cycle policy.** Two definitions that name each other are refused exactly as before; two families that name
  each other are refused exactly as before. What is new is only that one analysis sees both.
- **No performance work.** The context is still rebuilt per document and prompt 165 still owns measuring it.
