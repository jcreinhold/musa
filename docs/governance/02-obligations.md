# Constitutional obligations

## What this document is

These are non-obvious consequences of the constitution which constrain otherwise plausible designs. They do not choose
surface spelling or Rust layout. Each obligation states what it rules out.

## 1. Cross-presentation conversion is never implicit

**Derives from:** Amendments 2 and 5.

A written pitch, performed pitch trajectory, MIDI note, and oscillator frequency may be related only by a named pass
whose inputs contain the required tuning, context, policy, or instrument data. A common integer or label is not a
coercion.

**Rules out:** structural type equivalence across owners, global conversion instances, and compiler operations which
silently choose tonic, temperament, register, spelling, metre, or transcription policy.

## 2. Launch repertoire does not become global ontology

**Derives from:** Amendment 2.

Common-practice tonal, twelve-tone, and staff-notation domains may be excellent defaults and concrete test cases. They
must remain named theory-owned definitions. A new practice can bypass them and still reach finite gesture and process
presentations through its own typed passes.

**Rules out:** requiring every music to have a key, scale degree, 12-TET pitch class, regular metre, chord, written
score, or Western function label.

## 3. Overlay does not manufacture padding

**Derives from:** Amendment 3.

For timelines `M=(d,E)` and `N=(e,F)`, simultaneous presence is `(max(d,e),E⊎F)`. The shorter presentation is simply
absent after its extent. An author writes a rest only when a notational or analytical presentation needs that fact.

**Rules out:** equal-extent overlay side conditions, automatic rest insertion, and interpreting an entered/withdrawn
voice as malformed temporal structure.

## 4. Metre and expressive timing are payload/context theories

**Derives from:** Amendments 2 and 3.

Pulse layers, bars, tala/metric cycles, conducted plans, groove, rubato, fermatas, accelerandi, and unmeasured regions
may relate to exact placement through named theory/profile passes. None is a mandatory index of every temporal value.

**Rules out:** a global piecewise-constant pulse-set base, a type error for senza misura, and kernel conversion rules
which presume one metrical ontology.

## 5. Equality names its quotient

**Derives from:** Amendment 6.

An admitted payload key may deliberately forget stored presentation detail. It must state that quotient, owner type, and
version. It must be total and deterministic and complete for equality classes; it need not be injective on the raw Rust
struct it intentionally quotients.

**Rules out:** simultaneously claiming that a key is injective on stored values and that it forgets fields, or changing
omitted fields without an equality-schema decision.

## 6. Hash equality is never proof of semantic equality

**Derives from:** Amendment 6.

A digest is a finite index. Unequal digests prove unequal canonical bytes for a deterministic hash computation; equal
digests select candidates which require exact byte or structured equality confirmation whenever correctness depends on
identity.

**Rules out:** hash-only cache hits, digest-owned nominal stamps, and theorems whose converse holds merely “up to a
collision probability.”

## 7. Rendering factorizes through complete execution semantics

**Derives from:** Amendments 4 and 6.

Execution preparation has the conceptual type

```text
prepare_execution :
  Sem_Gesture × Bindings × Seed × Options
  → Result PreparedExecution PrepareError.
```

`Options` contains every choice affecting acceptance or execution. Presentation-only fields reach a separate lineage
pass and cannot affect `PreparedExecution`. Equal complete arguments to a pure deterministic implementation produce an
equal complete result. Equal frames additionally require equal external input histories, initial state, allocation
semantics, and conforming deterministic processors.

**Rules out:** ambient sample-rate/block choices, preparation which observes omitted source spans, and an unconditional
claim of cross-device bit identity.

## 8. Process scheduling is whole-node and feedback is registered

**Derives from:** Amendment 4.

A primitive process transition runs only after all of that node's current-tick inputs are available. The dependency DAG
is therefore over whole nodes, not merely over ports. Every accepted same-tick edge respects a node schedule. A cycle is
legal only when at least one edge reads prior registered state.

**Rules out:** accepting an acyclic port graph which no whole-node API can execute, zero-delay feedback, and caller
block partition as hidden feedback semantics.

## 9. Lineage retains the witnesses needed to form it

**Derives from:** Amendments 1 and 5.

A generated hop retains its generating site/root, qualified source and target anchors, pass identity, region/evidence,
and versioned presentation references. Path composition retains intermediate vertices rather than flattening them into
an uncheckable list of labels. Lineage is a finite set of complete paths; multiplicity belongs in a stable derivation id
when distinct derivations must survive.

**Rules out:** source-less generated edges, composition which erases the root that justified a hop, and claiming both
bag multiplicity and duplicate-edge normalization for one representation.

## 10. Source-language power must have a concrete representation caller

**Derives from:** Amendments 1 and 2.

Nominal data and private constructors have direct callers: plural theory owners. A richer feature—dependent indices,
recursive data, abstract-member functors, first-class worlds, equality proofs—enters only after two concrete operations
cannot be expressed clearly with ordinary total functions, finite data, and typed passes.

**Rules out:** adding CBPV, universes, refinements, or theta-link syntax because the analogy is attractive, and adding a
feature whose only example already assumes the feature.

## 11. Stage rules need not become source terms

**Derives from:** Amendments 3–5.

The temporal algebra, process transition system, and derivation category are all formal calculi. That does not imply one
surface language should internalize all of them. A stage may remain a compiler-owned IR when exposing its terms would
not make musician-facing ideas simpler.

**Rules out:** one universal monoidal operator for chords, phrases, timelines, and DSP, and the inference that “has a
calculus” means “must have source syntax.”

## 12. Real-time execution refines, rather than weakens, the process semantics

**Derives from:** Amendments 1, 4, and 6.

Preallocation, fixed buffers, queueing, and processor specialization may change cost and representation only. They must
implement the declared tick transition. The callback allocates no memory, takes no lock, performs no I/O, logs nothing,
and destroys no large object.

**Rules out:** a fast path with different feedback meaning, best-effort resource failure in the callback, and a cached
plan whose identity omits an execution-affecting option.
