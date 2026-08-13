---
id: 127db
slug: derivation-graph
status: done
depends_on: [127da]
phase: 3
---

# Build the Finite Derivation Graph

## Task

Implement the origin record `docs/rules/across-stages/02-derivation-diagrams.md` already governs: a finite acyclic
derivation graph in which a reused result keeps both the source it instantiates and the site that instantiated it, and a
combined result keeps all of its inputs. Make track construction record into it. This closes the third and fourth
blockers together, because they are one object.

## Read

- `docs/notes/research/language-design-closure/37-final-blocker.md` §§3–4 — mapping shared music lost each reuse site,
  and a list-shaped path could not express combined ancestry.
- `docs/rules/across-stages/02-derivation-diagrams.md` in full; §3 fixes the three step forms, §4 fixes path joining, §6
  fixes stage composition as grafting and forbids flattening. `04-identity-and-realization.md` for what an anchor must
  survive.
- `crates/musa-compiler/src/origin.rs` — the current `Origin`, `ExpansionStep`, and `ChoicePath`, which are the closest
  existing thing and are a list.
- `crates/musa-kernel/src/track.rs` and the event-track construction path in `crates/musa-compiler/src/elaborate.rs`.

## Design

A derivation is a finite directed acyclic graph whose leaves are anchors of an input representation, not a list. The
three step forms are `Preserved`, `Generated(source root, generation site, …)`, and `Combined(source list, …)`, and each
carries the evidence its pass descriptor declares.

Two consequences the current list cannot express, and which this prompt exists to deliver:

- **Reuse is `Generated`, not `Preserved`.** One shared phrase used at two sites gives two results with two derivations
  that agree on the shared root and differ in the generation site. Unfolding a use appends both before any later
  transformation step, so a later map cannot make the two copies indistinguishable.
- **Composition grafts.** Composing a later stage after an earlier one replaces each leaf of the later graph by the
  earlier graph rooted at the matching anchor. Where several results share an input the grafted subgraph is shared;
  where one result has several inputs all of them survive as parents. This is not list concatenation, and there is no
  helper that flattens the graph into `(source, target)` pairs — §6 says what such a flattening loses.

Two obligations are audited rather than asserted: **coverage** (every anchor of a composed result reaches at least one
source anchor; a result with no derivation is a defect in the pass that produced it) and **associativity** (grafting
three stages in either grouping gives the same graph). State both as law tests over generated graphs, not as prose.

Wire it where facts are made: event-track construction, `follow`, `together`, payload mapping, and repeat or reuse
expansion each append their own leaves. Adapter expansion records are a source of `Generated` steps and are added in
prompt 127dc; nothing here anticipates them beyond leaving the step form open.

Anchor ids stay local to one `PresentationRef` and build-local. Nothing in this prompt promises an identity across
builds, and nothing stores a derivation in a cache keyed without its pass descriptor version.

## Target

- The finite derivation-graph type, its builder, and the three step forms, with anchors, pass descriptors, and evidence.
- Grafting composition, with sharing on common inputs and all parents retained on a combined result.
- Event-track construction, succession, overlay, payload mapping, and reuse recording their own leaves.
- One narrow public accessor with a real consumer, following the `Compilation::machine` precedent.
- Law tests: coverage; associativity of grafting under both groupings; two uses of one shared body giving two distinct
  derivations that agree on the root; a combined result keeping every parent; refusal of a cyclic or dangling anchor;
  and exact identity of a graph under its own encoding.
- Public-surface comparison and module-design audit.

## Check

```sh
cargo nextest run -p musa-kernel -p musa-compiler -p musa-project
cargo clippy --all-targets -p musa-kernel -p musa-compiler -p musa-project -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
bash .agents/skills/module-design/scripts/audit-module.sh crates/musa-compiler
```

Commit as `Record derivations as a finite grafted graph`.

## Stop

- No adapter phase, adapter import, or expansion record — prompt 127dc adds them as a `Generated` source.
- No flattening of the graph to source/target pairs, and no summary label replacing an intermediate anchor.
- No machine, audio, or derivation type inside `musa-kernel` beyond what a track occurrence already carries; the kernel
  stays a leaf.
- No cross-build identity, package registry, or persistent derivation cache.
- No notation or studio migration.
