# Proof review: K₃.3 integration closure

**Status: independent closure review of the frozen K₃.3 corrigendum; governs nothing.** I reviewed
[44](44-k3.3-integration-closure.md) against every open item in the K₃.2 review [42](42-proof-review-k3.2.md) and the
accepted framing/path definitions in [40](40-canonical-framing-bug.md) and [41](41-k3.2-closure.md). This is a closure
review, not a fresh architecture search: the question is whether the added integration premises imply their conclusions
without silently strengthening semantic equality, R1, frame equality, or cache correctness.

## Verdict

- **Decision**: **Correct.**
- **Basis**: I found no exact counterexample to S1, G1/G2, C2, R1-K₃.3, R1-frames, or C1.3 under their stated contracts.
  K₃.3 closes all four issues from review 42: semantic bytes name the payload quotient and encoding version; qualified
  anchors resolve through a well-formed versioned registry; every stored lineage field has exact canonical equality; and
  execution is again typed as a function of `Sem_Gesture`, bindings, seed, and complete options returning the complete
  `Result`.
- **Scope**: “Correct” applies to the mathematical candidate. It does not claim that current Rust implements semantic
  framing, the process IR, lineage paths, artifact registries, preparation factorization, or the cache invariant.
- **Findings**: no fatal, high, or medium defect. Two low implementation clarifications are worth recording before the
  formats are frozen.

## Review-42 closure audit

| Review-42 obligation | K₃.3 result |
| --- | --- |
| Put payload type and quotient version in persisted semantic identity | **Closed.** `owner_type_id`, `quotient_version`, and timeline encoding version precede the framed semantic structure. |
| Make qualified-anchor and pass resolution functional | **Closed.** `RegistryWellFormed` gives unique versioned references, local anchors, pass descriptors, kind agreement, and immutable checking. |
| Give all path fields canonical decidable equality and encoding | **Closed.** `CanonicalData` covers ids, versions, roles, evidence, regions, derivation ids, and schema references; composite records frame variable children. |
| Restore exact execution factorization | **Closed.** `prepare_execution` consumes `Sem_Gesture × Bindings × Seed × Options` and returns the complete `Result`; presentation data reaches only `prepare_lineage`. |

No closure item was discharged by upgrading a digest to equality, requiring lineage equality, claiming content-only
sound equality, or promising cross-platform frames without a conformance premise.

## Theorem attacks

### S1 — versioned semantic framing

**Pass.** At one fixed timeline encoding and payload schema, S1 is F2 with extra fixed header fields. Conversely, if any
of timeline encoding version, stable owner type id, or quotient version differs, the canonically framed header differs.
Unique decoding then prevents the remainder from absorbing, erasing, or shifting that difference.

Adversarial payload keys do not reopen the K₃.1 collision. An empty key, embedded NUL, arbitrary binary bytes, a forged
header spelling, old N5 delimiters, or an entire apparent occurrence record is consumed as exactly the length-framed key
field. It cannot change the schema header or the next occurrence boundary. Equal key bytes under two genuinely different
payload owners remain unequal semantic encodings because `owner_type_id` differs. A changed key contract under one owner
must change `quotient_version`; reusing the header is explicitly an admission violation rather than a second reading of
the same valid schema.

The proof depends on stable ids and versions being nominal canonical data, not finite digests accepted without
confirmation. K₃.3 states that contract and requires complete schema headers in persisted cache keys.

### RegistryWellFormed and G1

**Pass.** `PresentationRef=(PresentationId,ArtifactVersion)` removes the ambiguity from review 42. A finite map has at
most one descriptor at a reference; registry conditions 3–5 supply the descriptor and local anchor/pass entries needed
by an admitted hop. The unique pass descriptor and condition 6 determine the source and target kinds. Root and site
membership are ordinary lookups in that one descriptor.

The previous collision counterexample cannot be formed as an admitted registry:

- distinct presentations with one `PresentationId` but different kinds violate permanent kind ownership;
- changed artifacts using one reference violate the distinct-version rule and exact merge check;
- repeated local `AnchorId` values in two artifact versions remain distinct qualified anchors because the version is in
  the reference; and
- two descriptors at the same reference are impossible in the map and are rejected on registry merge if their exact
  manifests or descriptors differ.

The invariant is an admission contract, not a theorem that globally unique ids arise automatically. That is honest: an
implementation must validate manifests and reject conflicts rather than trust a content digest.

### G2 — category typing and registry growth

**Pass.** Under G1 every vertex and hop formation judgment used by L0–L2.2 has one meaning. Path concatenation retains
qualified versioned vertices and fully formed primitive hops, so it introduces no new registry reference. A well-formed
immutable extension which preserves every existing mapping therefore preserves all old path judgments.

Separately stored lineages are not composed by wishful id agreement. K₃.3 requires construction and validation of a
common merged registry first. If the manifests/descriptors behind an equal presentation reference disagree, composition
is unavailable rather than ambiguous. This is sufficient for a category per fixed well-formed registry; it does not
claim one global category across incompatible registries.

### CanonicalData and C2

**Pass.** For every atomic field, `compare` is a decidable total order on admitted equality classes and encoding
equality coincides with comparison equality. Fixed tags, child length framing, vertex/hop counts, and finite record/list
induction therefore give the same equivalence for hops and paths. Lexicographic path comparison is a decidable total
order; sorting followed by adjacent exact deduplication gives a unique finite-set representative independent of
insertion order.

The repeated-derivation cases behave as claimed:

- byte-identical paths denote the same set member and deduplicate;
- different endpoints, intermediate vertices, passes, roles, regions, evidence, schema ids, or stable derivation ids
  compare differently and survive;
- two operational derivations that must remain distinct but otherwise have identical stored facts require distinct
  `DerivationId` values; and
- insertion order is not treated as identity.

This is equality of derivation artifacts, not a proposition that distinct proofs of the same musical claim are
definitionally equal. K₃.3 explicitly avoids that strengthening.

### R1-K₃.3 — exact execution result

**Pass.** The signature restores the factorization which review 42 required:

```text
prepare_execution :
  Sem_Gesture × Bindings × Seed × Options
  → Result PreparedExecution PrepareError.
```

Presentation fields omitted by the admitted gesture key cannot be observed because no presentation value crosses this
type. `Options` explicitly contains every preparation choice affecting acceptance or execution, including sample rate,
channels, tick/block policy, bounds, and deterministic quality policy. Bindings, options, and other non-scalar arguments
carry versioned admitted equality and complete canonical encodings.

Under equality of all four inputs, a pure deterministic function returns one equal value by substitution. This proves
equality of the **whole** result: either the same canonical failure or equal successful execution. It does not infer
that different options, bindings, seeds, payload schemas, or operation versions should agree. It does not infer lineage
equality. The earlier exact countermodels—observing an omitted derivation field or an ambient 44.1/48 kHz choice—are now
ill typed or violate the complete-options premise.

### R1-frames — runtime premises

**Pass.** R1-frames is conditional on success and on every premise not supplied by source factorization: fixed
allocation semantics, equal external inputs, parameters, seeds, and initial node/register states, plus deterministic
conforming processor transitions. Equal prepared execution then supplies equal process definitions, and the accepted K₃ᴾ
tick induction yields equal outputs.

The theorem does not identify current caller-chunk-sensitive feedback with a fixed block tick. It also does not derive
cross-platform floating-point or device conformance. A current or future runtime which lacks those properties simply
does not satisfy the corollary's premises.

### C1.3 — exact cache hits

**Pass.** `ExecArgs` contains the named operation version; temporal encoding version; gesture owner/quotient schema;
structured gesture semantics; binding schema and value; seed; and option schema and value. Lookup uses the finite digest
only to select candidates. Exact equality of the framed complete arguments confirms a hit.

The insertion invariant then gives

```text
stored_result = operation_version(stored_args),
stored_args = requested_args,
```

so substitution yields recomputation on the request. A digest collision merely adds a rejected candidate. A stale
operation implementation, changed quotient, changed options equality, or changed binding encoding must receive a new
version; reusing a version for changed behavior violates the stated cache admission contract and is not licensed by the
theorem.

C1.3 neither uses hash equality as semantic equality nor caches lineage under the execution key. No hidden R1 or lineage
claim follows.

## Low implementation clarifications

1. **Give `ExecArgs` one named `CanonicalData` record.** The theorem already says “exact framed-argument equality,” and
   its listed fields have canonical/version contracts. Defining the outer field tags, lengths, and `Seed` encoding in
   one record will remove the last opportunity for two implementations to assemble the same complete fields differently.
   This does not change C1.3's proof.

2. **Apply the exact merge-conflict rule to pass descriptors explicitly.** A common merged registry must already satisfy
   unique pass resolution and revalidate every admitted hop, so conflicting `PassId` descriptors cannot produce a
   well-formed common registry. Stating the same “equal id implies equal exact descriptor, otherwise reject” rule for
   passes as for presentation references will make the separate-artifact loading algorithm direct rather than relying on
   final registry revalidation.

Neither clarification supplies a missing mathematical premise used above; both make the eventual storage and merge
formats harder to misimplement.

## Verified, judged, and not checked

### Verified

- I read the full frozen K₃.3 note, all findings in review 42, and the K₃.2 framing and typed-path definitions it
  imports.
- I replayed the delimiter-injection, cross-payload-schema, repeated presentation/anchor id, generated-hop,
  repeated-path, multiple-target-derivation, full-presentation preparation, ambient-options, and digest-collision
  attacks.
- I checked each new theorem against the exact data fields and premises displayed in K₃.3.
- I ran `cargo test -p musa-kernel -q`; all 58 current tests passed.

### Judged

- The **Correct** verdict is a mathematical/specification judgment under the explicitly named ownership, versioning,
  canonical-data, purity, determinism, and conformance contracts.
- I judged the contracts sufficient for the local theorems. Whether a concrete implementation honestly satisfies those
  contracts remains a separate verification obligation.

### Not checked

- Current Rust still uses the refuted N5 display bytes for semantic hashing. K₃.3 explicitly says the proposed framing
  is absent, so passing current tests is not evidence that S1 or C1.3 has been implemented.
- There is no K₃.3 registry, lineage path store, registered process IR, execution factorization, or versioned cache to
  inspect.
- I did not verify uniqueness allocation for stable ids, persistence/migration code, the totality of future canonical
  encoders, processor determinism, or cross-target floating-point behavior.
- Candidate T₂a and cultural adequacy of musical theory packages remain outside K₃.3 exactly as stated.

## Recommendation

Accept K₃.3 as closure of the candidate proof and stop iterating the definitions in this chain. The next work should be
the smallest implementation-backed repair: amend governing N3–N6 and replace the current hash writer with one exact
versioned framed semantic encoder, including the known delimiter-injection regression and arbitrary-byte property tests.
That implementation and its review should precede lineage/process scheduling; it fixes a verified bug in the current
kernel rather than merely prototyping future architecture.

The later registry, lineage, preparation, and process work should each expose the contracts above as validators and law
tests. If an implementation cannot discharge one, repair that implementation contract; no remaining proof finding here
requires another kernel-language redesign.
