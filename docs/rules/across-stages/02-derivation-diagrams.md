# Tracking where derived results came from

When Musa turns source into a score track, a score track into gestures, or gestures into MIDI or a prepared machine,
users need to know where each result came from. This chapter defines that origin record.

## 1. Stored representations and anchors

Each stored representation has a stable reference for one version:

```text
PresentationRef = (presentation id, version)
```

Its descriptor records:

```text
kind                  source, score track, notation, MIDI, machine, ...
schema                versioned format description
root                  anchor for the whole representation
generation sites      anchors that can create new material
anchor table          every addressable item in this version
manifest              exact bytes needed to identify the stored result
```

An **anchor** names one addressable item, such as a source expression, a track occurrence, an MEI element, or a
primitive instance in a machine. Anchor ids are local to one `PresentationRef`; the pair `(PresentationRef, anchor id)`
is globally unambiguous within a project.

Loading two records with the same `PresentationRef` but different descriptors is an error. Musa never guesses which
record is the intended one.

## 2. A conversion record

A compiler pass from representation `S` to representation `T` returns three things:

```text
run_pass(input S)
    -> output T
     + origin paths from S to T
     + a list of losses or approximations
```

It may instead return a diagnostic. The loss list records facts such as “pitch spelling was dropped,” “this timing was
rounded to MIDI ticks,” or “the user selected one alternative.” An empty list does not prove that the pass is
reversible.

Each pass id has one exact descriptor: source kind, target kind, version, and the schemas used for evidence and loss.
Two different descriptors may not share one pass id.

## 3. One step in an origin path

Musa needs three forms of step:

```text
Preserved(source, target, evidence)
Generated(source root, generation site, target, evidence)
Combined(source list, target, evidence)
```

`Preserved` covers an ordinary relation such as one source note producing one track occurrence. `Generated` covers
material made at a repeat, a syntax-adapter expansion, or an algorithmic generation site. It keeps both the source root
and the generation site. `Combined` covers a result made from several inputs, such as a chord label inferred from
several notes.

Every anchor in a step must exist, and the pass descriptor must accept the source and target kinds.

A **reused** result is `Generated`, not `Preserved`: it records both the shared source it instantiates and the site that
instantiated it. This is what makes one shared body usable at several places without the uses becoming
indistinguishable.

## 4. Joining paths

An origin path alternates anchors and conversion steps. Two paths can be joined only when the first path ends at the
exact anchor where the second begins:

```text
p goes from a to b    q goes from b to c
────────────────────────────────────────
q after p goes from a to c
```

Joining keeps the middle anchor `b` and both steps. It does not replace them with a summary label.

**Theorem.** For one valid registry, path joining is associative, and the empty path at an anchor is an identity.

**Proof.** A path is a finite list whose adjacent endpoints match. Joining paths is list concatenation at a shared
endpoint. List concatenation is associative. Concatenating an empty list changes nothing. ∎

## 5. A project’s origin record

The project stores a finite set of complete origin paths in a fixed order. Exact duplicate paths are stored once. If two
otherwise identical derivations must remain distinct, they receive different derivation ids.

The resulting graph connects source, score, notation, analysis, MIDI, gestures, and audio preparation without declaring
those representations equal. It supports questions such as:

- Which source expression produced this engraved note?
- Which track occurrences support this analysis label?
- Which tuning and rounding steps produced this MIDI event?
- Which gesture and binding produced this primitive instance?

That is the whole purpose of the graph. It is an origin and loss record, not a universal definition of music.

## 6. Composing two stages grafts; it does not concatenate

§4 is about **one path**, which is a list, so joining two paths is concatenation at a shared anchor. Composing two
**stages** is a different operation and must not be confused with it.

A stage's derivation is a finite directed graph whose leaves are the anchors of its input representation. Composing a
later stage after an earlier one replaces each leaf of the later graph by the earlier graph rooted at the matching
anchor. Where several results share one input, the grafted subgraph is shared rather than copied; where one result has
several inputs, all of them survive as parents.

Two obligations follow, and both are audited:

- **Coverage.** Every anchor of the composed result has a path to at least one source anchor. A result with no
  derivation is a defect in the pass that produced it, not an acceptable omission.
- **Associativity.** Grafting three stages in either grouping gives the same graph, because grafting acts leaf-wise and
  the leaves of a graft are the leaves of the grafted subgraph.

Flattening this graph into a list of `(source, target)` pairs loses exactly the two things it exists to record: which
intermediate produced which, and which of several inputs a combined result came from. That flattening is the mistake
§4's "does not replace them with a summary label" already forbids, stated at the graph level.

An adapter expansion record — adapter definition and version, use site, input syntax, output syntax, parent expansion —
is a source of `Generated` steps and nothing more. Expansion provenance says how text became an expression; a derivation
says how a musical result came from an input. They have different jobs, and neither proves the other.
