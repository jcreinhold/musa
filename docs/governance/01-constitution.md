# The Musa constitution

## What belongs here

This document defines Musa at the level below syntax and above implementation. A claim belongs here only when removing
it changes what the project is. The exact carrier types, inference algorithms, storage formats, and Rust modules are
downstream.

## Amendment 1: the source is the canonical editable presentation

A Musa project has one editable authority: its source and locked package inputs. Engraving, analysis, performance
gestures, prepared audio, waveforms, and interface state are derived presentations. A structured editor changes source;
it does not maintain a second independently editable score model.

### What follows

- Every derived presentation must be reproducible from identified source/package inputs and explicit realization
  choices.
- Selection, diagnostics, and navigation from a derived presentation require a recorded route back to source.
- A cache may accelerate derivation but cannot become an editable authority.

### What this leaves open

- concrete source syntax and formatting;
- which derived artifacts are persisted;
- whether a derivation is eager, incremental, or cached.

## Amendment 2: musical meanings are plural and representation-owned

Musa does not have one universal `Music`, `Pitch`, `Key`, `Chord`, `Meter`, or `Gesture` which every practice must use.
A musical theory or realization stage owns its carriers, constructors, observations, and equality. Equal machine
representations do not identify values owned by different theories or stages.

The built-in Western launch domains are one package of useful definitions, not the ontology of music. A practice may
instead be phrase-primary, gesture-primary, timbre-primary, cyclic, oral, notationally sparse, or use no global pitch or
metrical coordinate.

### What follows

- Cross-theory and cross-stage movement is named and typed.
- A package may hide representations behind nominal carriers and total operations.
- No compiler quotient tower is presumed to extend to every repertoire.

### What this leaves open

- the first source syntax for nominal ownership;
- which packages ship in the standard library;
- whether a later concrete caller earns dependent or generative types.

## Amendment 3: finite symbolic time is exact ambient placement

The temporal kernel describes a finite value as an exact nonnegative rational extent together with a finite multiset of
typed occurrences supported inside that extent. Temporal succession translates the later value. Simultaneous presence
unions occurrence multisets and takes the maximum extent.

Extent is ambient space, not the sum of event durations. Uncovered space is silence by absence. A notated rest is a
payload fact when notation needs one; it is not the kernel's unit and is never inserted merely to equalize extents.

### What follows

- Unequal-duration voices overlay directly.
- Exact symbolic time contains no sample, frame, second, or floating-point quantity.
- Multiplicity is retained: equal occurrences do not collapse.
- The temporal kernel remains payload-opaque and musically neutral.

### What this leaves open

- the payload theory;
- tempo, groove, rubato, and tuning interpretations;
- whether a later live-language extension has a separate coinductive semantics.

## Amendment 4: finite process descriptions and running behavior are different stages

A studio or instrument graph is a finite typed description of a process. Its execution is state evolution over physical
time. An audio history is a potentially unbounded observation of that execution. None of these is a temporal-kernel
payload, and none is identified with the others.

The finite process description has its own formation and operational semantics. Feedback is admitted only through an
explicit state/register boundary. The existence of that process calculus does not give a graph musical extent or put a
signal in the temporal kernel.

### What follows

- Preparation crosses from exact finite musical intent to a finite executable process definition.
- Physical units and processor state begin at that boundary.
- Process cycles require an explicit delay/register; a combinational dependency cycle is rejected.

### What this leaves open

- concrete processor vocabulary and buffer layout;
- tick size and vectorization strategy, provided the specified transition is preserved;
- live device and plug-in integrations.

## Amendment 5: coherence is a typed derivation diagram, not a common quotient

A project is coherent when its presentations are connected by recorded typed derivations. A derivation names its source
and target presentation kinds, the pass which related them, anchor lineage, and any loss or approximation. It is not an
equality proof between native presentations.

There is no required universal musical object through which engraving, performance, sound, analysis, and MIDI factor.
The finite diagram of presentations and passes is the common project artifact. Composition is path composition; it
retains intermediate identities and evidence.

### What follows

- A tempting commuting square is a local theorem of named passes, never a global conversion rule.
- Lossy transcription, tuning, MIDI, and rendering stay coherent by recording loss rather than pretending to be
  isomorphisms.
- The useful content of the inter-universal analogy is non-identification until a named bridge is supplied. It does not
  introduce `world` or `theta_link` syntax.

### What this leaves open

- which passes persist full lineage;
- compression of derivation data which can be proved information-preserving;
- later higher categorical structure justified by actual pass laws.

## Amendment 6: every presentation owns exact, versioned identity

Semantic equality belongs to a presentation and an admitted schema. Its canonical encoding is complete for that equality
and explicitly versioned. Display text is not identity. A finite digest locates candidate identity records; it does not
replace exact equality.

Presentation equality, derivation equality, execution equality, and observational audio equality are separate relations.
A specification may relate them by a theorem with explicit premises, but no shared hash or field layout silently
identifies them.

### What follows

- Changing what an equality observes changes its schema version.
- Variable-size canonical data is uniquely framed.
- Cache hits confirm exact complete arguments after digest lookup.

### What this leaves open

- digest algorithms used only as indexes;
- migration policy between explicitly versioned equalities;
- presentation-specific quotient choices.
