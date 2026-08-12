# What the inter-universal analogy earns

**Status: conceptual sieve; governs nothing.** The user suggested thinking inter-universally, in the spirit of IUT and
Θ-links, instead of insisting on a musical motive. This note reads that analogy as a method for avoiding false
identifications. It does not claim that Musa instantiates Inter-universal Teichmüller theory.

## 1. What the sources actually emphasize

I read the local transcriptions of:

- `arithmetic-geometry/anabelian-and-iut/mathematics-of-mutually-alien-copies`, especially the introduction, §2, and §3;
- `a-panoramic-overview-of-inter-universal-teichmuller-theory/04-inter-universality-and-anabelian-geometry.md`; and
- the IUT-I introduction and log/Θ-link overview where the surrounding terminology needed checking.

The load-bearing pattern, abstracted very cautiously, is not “everything is a world”. It is:

1. make distinct copies rather than identify them through a convenient shared carrier;
2. recognize that a cross-copy assignment may fail to preserve the full native structure on either side;
3. disable labels or operations which would make an invalid identity look available;
4. transport only reconstructible or invariant structure across the boundary; and
5. keep track of the indeterminacy introduced by that transport.

In the cited IUT exposition, the specific obstruction concerns assignments incompatible with ring/scheme structure and
the resulting need to treat certain groups without the field-automorphism labeling apparatus. Musa has no analogous
arithmetic theorem, Galois reconstruction, or log-theta lattice. Any stronger identification would be wordplay.

## 2. The Musa analogue is representation independence

Musa's corresponding danger is familiar and concrete:

```text
written note ≠ performed gesture ≠ scheduled event ≠ signal contribution
notated duration ≠ performed duration ≠ frame interval
part ≠ instrument ≠ mixer track
pitch spelling ≠ frequency
```

They may share integers, timestamps, labels, or source ancestry in an implementation. Those shared carriers do not
license equality. The PL-theoretic mechanism which naturally enforces the prohibition is ordinary abstraction:

- nominal types prevent two theories or stages from being identified by representation shape;
- private constructors let each theory own its invariants;
- typed passes state exactly which direction is implemented;
- loss evidence states what the pass forgets or approximates; and
- lineage records which identified output parts derive from which identified input parts.

This is closer to abstract data types and representation independence than to a new dependent universe hierarchy.
Candidate T₂a's `A.Intent` and `B.Intent` are “mutually alien” in the only operational sense Musa needs: equal layouts
do not let values cross. A translator must be named and typed.

## 3. Why no `world` form follows

A first-class world would earn its place only if the language needed to quantify over native theories while preserving
different judgments or conversion laws. The current examples need static owners, not world-valued terms:

```text
CommonPractice.WrittenPitch
Karnatak.SvaraIntent
Performance.Gesture
Audio.PreparedExecution
```

These paths already disable accidental identification. Their modules may use different definitions internally. A generic
`world W` binder adds a new kinding, substitution, and equality problem without simplifying chord construction, phrase
constraints, preparation, or DSP graphs.

**Escalation test.** Add world-polymorphic syntax only after two concrete operations must quantify over theory owners
and cannot be expressed by an ordinary signature plus an explicitly exported translator. None of the notebook's examples
currently passes that test.

## 4. Why no Θ-link form follows

A Musa pass has the ordinary shape

```text
Pass S T E = S → Result {
  value   : T,
  lineage : Lineage S T,
  losses  : List E
} Diagnostic.
```

The source and target remain distinct. `lineage` is not an equality proof, and `losses` prevents an approximation from
masquerading as an isomorphism. Pass composition composes functions, concatenates loss evidence, and composes typed
lineage paths.

This already expresses the useful “link” discipline. A dedicated `theta_link` term would be justified only if several
passes shared a new generic formation/composition law not expressible this way. The examined score→gesture,
gesture→plan, plan→runtime, engraving, and analysis examples do not.

The word **link** remains useful in prose for a correspondence which deliberately preserves less than the native
structures. It should not become syntax merely because the analogy suggested the word.

## 5. The actual inter-presentation object

The coherent project artifact is the finite derivation diagram proposed in
[32](32-the-coherent-object-is-the-diagram.md): native presentations at nodes, typed passes at edges, and explicit
lineage/loss evidence on derived edges. No node is the universal musical object. No edge licenses all operations from
one node on the other.

For one realized project:

```text
source
  ──elaborate──▶ contextual music
  ──instantiate/close──▶ temporal facts
  ──interpret profile──▶ gestures
  ──prepare──▶ finite process plan
  ──allocate/execute──▶ observed audio history

temporal facts ──engrave──▶ notation plan ──render──▶ MEI/PDF
temporal facts ──analyze under theory T──▶ claims with evidence
```

The diagram is not a common quotient. It remembers which structures exist at which node and which comparisons were
actually constructed. The modest universal property is the free category of recorded derivation paths: any assignment of
primitive passes to composable maps extends uniquely to paths. That theorem organizes provenance; it does not explain
music.

## 6. A useful noncommuting square

The analogy earns its keep when a tempting square should **not** commute. Let `forget_spelling` map written pitch to a
chromatic coordinate, and let `transpose_written` and `transpose_sounding` be operations owned by their respective
theories. Without a declared interval interpretation and tuning context, there is no theorem

```text
forget_spelling ∘ transpose_written
  = transpose_sounding ∘ forget_spelling.
```

In some packages and contexts the owner can prove such a law. In others—microtonal accidentals, adaptive just
intonation, notation with contextual inflection, or a transposition which preserves fingering rather than frequency—the
two sides may not even have the same intended operation. The type system should demand the bridge data instead of
installing the square globally.

The same applies to time:

```text
quantize ∘ warp  ?=  process_frames ∘ quantize.
```

Rounding, collision policy, and state make this a proved pass-specific property or an explicit discrepancy, never a
kernel conversion rule.

## 7. Design consequences

The inter-universal reading supports the following concrete decisions:

1. Keep K₃'s stage-specific kernels rather than one universal value calculus.
2. Adopt nominal theory-owned data before dependent indices or world polymorphism.
3. Require all cross-stage operations to name their source/target types, admitted equality, loss, and lineage behavior.
4. State commuting laws locally per pass and package; do not infer them from shared vocabulary such as pitch, duration,
   key, or signal.
5. Preserve intermediate qualified anchors in lineage paths, as K₃.2 does. Erasing the intermediate “labeling apparatus”
   repeats exactly the identification error the analogy warns against.
6. Keep runtime audio histories out of finite derivation identity. The finite plan and its anchors cross the boundary;
   observations are indexed by execution state and time.

## 8. Verdict

The analogy is constructive as a **sieve against illicit transport**. It is not a candidate calculus. Its positive PL
content is nominal abstraction plus explicit, evidence-bearing passes. Those definitions are already smaller and more
checkable than worlds or Θ-links, and they make the intended distinction simple: things remain alien until a named pass
relates them, and the pass carries only the structure it proves.

