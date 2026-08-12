# Tracking where derived results came from

When Musa turns source into a score, a score into gestures, or gestures into MIDI or audio, users need to know where
each result came from. This chapter defines that origin record.

## 1. Stored representations and anchors

Each stored representation has a stable reference for one version:

```text
PresentationRef = (presentation id, version)
```

Its descriptor records:

```text
kind                  source, score, notation, MIDI, audio plan, ...
schema                versioned format description
root                  anchor for the whole representation
generation sites      anchors that can create new material
anchor table          every addressable item in this version
manifest              exact bytes needed to identify the stored result
```

An **anchor** names one addressable item, such as a source expression, score event, MEI element, or process node. Anchor
ids are local to one `PresentationRef`; the pair `(PresentationRef, anchor id)` is globally unambiguous within a
project.

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

`Preserved` covers an ordinary relation such as one source note producing one score event. `Generated` covers material
made at a repeat, template, or algorithmic generation site. It keeps both the source root and the generation site.
`Combined` covers a result made from several inputs, such as a chord label inferred from several notes.

Every anchor in a step must exist, and the pass descriptor must accept the source and target kinds.

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
- Which score events support this analysis label?
- Which tuning and rounding steps produced this MIDI event?
- Which gesture and binding produced this process node?

That is the whole purpose of the graph. It is an origin and loss record, not a universal definition of music.
