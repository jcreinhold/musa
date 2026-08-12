# Proof review: K₃.2 semantic framing and typed paths

**Status: independent review of the frozen K₃.2 repair; governs nothing.** I reviewed [40](40-canonical-framing-bug.md)
and [41](41-k3.2-closure.md) as fixed documents against the K₃.1 counterexamples in [39](39-proof-review-k3.1.md), the
governing temporal equality/hash contract, and the current generic kernel implementation. I separately checked theorem
validity and implementation status: a candidate theorem can be correct even though current Rust still implements the
refuted N5 hashing scheme.

## Verdict

- **Decision**: **Incomplete.**
- **What closed**: F1–F3 and T2.2 are correct for the explicitly assumed uniquely decodable framing. Arbitrary payload
  bytes cannot escape a length-delimited field. L0, L0.1, L1.2, and L2.2 are correct for a fixed **well-formed**
  artifact registry: complete primitive hops retain generated anchors, repeated vertices do not break concatenation,
  finite-set identity works, and target coverage composes.
- **What remains**: K₃.2 does not state enough registry well-formedness to make qualified-anchor resolution functional,
  and it does not define canonical equality for every field needed by a “finite canonical set” of paths. More
  importantly, §5's displayed R1 drops both the semantic projection and `Options` from the reviewed K₃ signature. The
  imported factorization proof therefore does not prove the displayed law.
- **Why not Incorrect**: I found no counterexample to the new framing or typed-path theorem after adding hypotheses the
  prose clearly intends. The failures are missing formation/integration hypotheses and a regressed R1 signature, not a
  defect in length framing or path concatenation.

## Findings

### High

1. **The displayed R1 no longer has the signature that made K₃'s proof valid.**
   - **Location**: [41 §5](41-k3.2-closure.md) versus [34 §6](34-candidate-k3-stratified-kernels.md) and
     [39](39-proof-review-k3.1.md).
   - **Type**: lost factorization premise / accidental strengthening.
   - **Problem**: the reviewed K₃ operation was conceptually

     ```text
     prepare_execution : sem(M) × Bindings × Seed × Options
                       → Result PreparedExecution PrepareError.
     ```

     That type made semantic congruence immediate: execution could not inspect fields omitted by `sem(M)`. K₃.2 instead
     displays

     ```text
     M ≡_T N
     ────────────────────────────────────────────────────
     prepare_execution(M,B,s)=prepare_execution(N,B,s).
     ```

     The full presentations `M,N` replace their semantic forms, `Options` disappears, and the result/error boundary is
     no longer visible.

     There are two exact countermodels to derivability from the surrounding definitions:

     1. Let payloads be `(sound,derivation)` with `key(sound,derivation)=sound`. Timelines differing only in
        `derivation` are K₃.2-equal. A deterministic function on the **full** timeline can return the omitted derivation
        byte, so the displayed conclusion fails. Calling the equation a law can forbid that implementation, but the old
        substitution proof no longer establishes it.
     2. For a real audio preparation operation, equal `M,B,s` prepared at 44.1 kHz and 48 kHz, or with different block
        options, may produce different execution specifications. The earlier theorem required equal options. Omitting
        that argument either makes options illicit ambient state or strengthens the law to option independence.
   - **Why it matters**: §5 says R1 remains split and §1 says no surviving result is strengthened. The displayed rule is
     precisely where the semantic/presentation split must remain visible.
   - **Repair**: restore the exact factorization and equality premises:

     ```text
     prepare_execution : Sem_Gesture × Bindings × Seed × Options
                       → Result PreparedExecution PrepareError

     Sem(M)=Sem(N)  B=B'  s=s'  O=O'
     ─────────────────────────────────────────────────────────
     prepare_execution(Sem(M),B,s,O)
       = prepare_execution(Sem(N),B',s',O').
     ```

     Then retain the already-stated runtime premises for frame equality. Presentation data reaches only lineage.

### Medium

1. **The artifact registry lacks the uniqueness conditions used by every qualified-anchor judgment.**
   - **Location**: [41 §§2–4](41-k3.2-closure.md).
   - **Type**: missing formation hypothesis.
   - **Problem**: the note gives every presentation an `id(P)` but never requires `id` to be injective in the fixed
     registry. “A qualified anchor resolves in `ᶜ`” and “belongs to `S`” are used as though resolution were a function.

     Take distinct presentations `P,Q∈ᶜ` with the same presentation id and the same local anchor id `a`, but different
     kinds, roots, or site membership. The byte pair `(id,a)` then resolves to both. A pass-kind check has two possible
     source kinds; a `Generated` hop may be valid through `P` and invalid through `Q`; and the reflexive path `[(id,a)]`
     belongs to both nominal objects. The path data no longer determines the typing derivation claimed by L0 and L1.2.
   - **Repair**: define `RegistryWellFormed(ᶜ)` and make it a premise of L0–L2. At minimum:

     - presentation ids are unique and permanently bind one presentation kind and artifact version;
     - local anchor ids are unique within a presentation;
     - roots and sites belong to that presentation's anchor set;
     - pass declarations resolve uniquely and name source/target kinds; and
     - every qualified reference in a checked lineage resolves in the immutable registry.

     With these conditions, a fixed finite registry is enough for the category theorem.

2. **“Finite canonical set of paths” is not yet a complete canonical data specification.**
   - **Location**: [41 §§2–4](41-k3.2-closure.md).
   - **Type**: missing decidable equality/encoding hypotheses.
   - **Problem**: presentation and anchor ids have canonical encodings, and evidence is called canonical, but `PassId`,
     `RegionMap`, presentation kinds, and any `DerivationId` are not given canonical equality/order/encoding obligations
     here. Mathematical finite sets need only equality as a proposition, so L1.2's abstract category argument survives.
     An executable canonical set, cache key, or separately stored lineage needs decidable equality and a total order or
     injective encoding for every nested field.
   - **Repair**: put every hop and path field under one finite canonical-encoding record, including an explicit framing
     rule for evidence and region data. Define path equality structurally on that record and set normalization as sort
     plus exact deduplication. This is an implementation-facing completion, not a new categorical idea.

### Low

1. **The stored semantic encoding needs a payload-schema discriminator when bytes cross a typed boundary.** F2 is
   correctly stated for one fixed `A`, so no theorem fails. A persisted or shared cache may compare encodings from two
   payload types whose keys and timelines happen to have identical bytes. Either include the canonical payload schema id
   in the domain-separated tag or make it a mandatory outer component of every cache key. “Fixed semantic-schema
   version” should identify both the timeline encoding and `key_A`'s quotient/version.

2. **The byte grammar remains abstract, by design.** “One canonical spelling” alone would not make adjacent variable
   integers self-delimiting, but [40 §3](40-canonical-framing-bug.md) additionally requires unique decodability and says
   a self-delimiting varint/fixed scheme must be chosen. F2 is correct under that stated premise. The Rust repair still
   needs to choose and test one exact encoding before stored hashes have a specification.

## Attack results

### F1 — structured equality

**Pass.** `Sem_A` is a finite extent plus a sorted finite list of exact rationals and byte strings. Rational equality,
byte equality, list length, and componentwise equality are decidable. Sorting by the lexicographic total order retains
duplicate triples, so it does not quotient multiplicity.

The revised N3 definition is also coherent: `key_A` defines the equality admitted by the parametric kernel. It need not
be injective on deliberately omitted Rust storage fields. Any payload owner claiming a prior domain equality must prove
soundness and completeness for that equality.

### F2 and T2.2 — adversarial framing

**Pass under the explicitly stated unique-decoding premise.** Let a payload key be empty, contain NUL bytes, contain
every old N5 keyword and delimiter, contain newlines, or itself look like a length prefix and following occurrence. The
decoder first reads the self-delimiting record fields, then reads exactly the declared number of key bytes. Key contents
cannot alter the next record boundary. The occurrence count distinguishes an empty timeline from one with an empty key
and retains repeated equal occurrences.

Canonical rationals and self-delimiting natural encodings make the structured field boundaries unique. Thus equal framed
bytes decode to equal extents and equal ordered triples; equal structures encode componentwise to equal bytes. The K₃.1
delimiter-injection example no longer collides under this encoding.

Evaluation and finite normalization are unchanged from the governing temporal proof, so T2.2 correctly joins that result
to F1/F2.

### F3 and C1.2 — hash/cache boundary

**Pass, conditionally.** Equal semantic forms have equal injective encodings and hence equal hashes. The finite hash
converse remains explicitly unclaimed. C1.2 requires a candidate selected by the digest to pass exact comparison of the
complete structured arguments or injective framed bytes at the same operation and semantic-schema version. Together with
insertion of the actual deterministic result, substitution proves the hit equals recomputation.

This cache result does not prove lineage equality or rendering from content-only equality. It remains correct only if
“complete arguments” includes bindings, seed, options, operation version, and payload schema, and only if execution is
factored through `Sem` as restored above.

### L0 and L0.1 — repeated anchors and units

**Pass for a well-formed registry.** Concatenation is `V_p ++ tail(V_q)` and `H_p ++ H_q`. It preserves the
vertex/hop-length invariant and every old adjacency. Empty-hop reflexive paths are harmless endpoint cases.

Repeated anchors do not break associativity. For example, paths with vertex lists

```text
p = [a,b,a]
q = [a,c,a]
r = [a,d]
```

produce `[a,b,a,c,a,d]` under both associations. Concatenating `[a]` on either side drops its sole duplicated boundary
vertex and adds no hop, so reflexive paths are units.

### L1.2 — path-set identity, associativity, and coverage

**Pass for a well-formed registry and structural path equality.** For `L:S→T`, each path has exactly one matching
reflexive path at its first anchor and exactly one at its last anchor; concatenation returns the same full path. Set
construction adds no extra member, so both identity laws hold.

Three-way composition enumerates the same triples of composable paths under both associations, and L0.1 gives identical
concatenated path data. Exact deduplication removes the same complete duplicates.

For target `u` of `U`, every `q∈M:T→U` chosen by M's coverage begins at a qualified anchor `t` of `T`; L's coverage of
that exact qualified anchor supplies at least one `p`. Therefore `q⋆p` covers `u`. If several paths cover `t`,
composition retains all distinct concatenated paths and exact deduplication still leaves at least one.

### L2.2 — generated hops

**Pass for a well-formed registry.** The K₃.1 counterexample now composes as the full path

```text
(id(S),s)
  --Preserved--> (id(T),root_T)
  --Generated--> (id(U),u).
```

The generated hop's own `from` remains `(id(T),root_T)`; it is never retested against outer source `(id(S),s)`. Site
anchors and rule evidence survive for the same reason. Repeated appearances of a root/site elsewhere in the path do not
change the hop-local check.

## Is the fixed registry enough?

Mathematically, yes—after adding `RegistryWellFormed(ᶜ)`. A category is obtained for each fixed registry. Every path and
lineage is finite even though cycles allow infinitely many possible paths, and composition never introduces an
unregistered vertex.

For separately stored artifacts, “fixed” must be made operational. A lineage artifact needs a manifest or
content-addressed references that close over every presentation, anchor table, pass declaration, region schema, and
evidence schema used by its paths. Loading two separately stored lineages for composition must construct and validate a
common immutable registry. Globally unique/content-bound presentation ids make inclusion into a larger registry preserve
old paths. Reusing an id for changed anchor data would invalidate qualification and must be rejected as a version error.

This storage mechanism is absent, but that is implementation absence rather than a refutation of the per-registry
category theorem.

## Verified, judged, and not checked

### Verified

- I read both frozen K₃.2 documents, the K₃.1 review, the governing N3–N6 text, and the current `Canonical`,
  `semantic_eq`, `write_canonical`, and `semantic_hash` implementations.
- I checked the framing against empty and delimiter-bearing keys, list multiplicity, and the exact K₃.1 collision.
- I expanded path concatenation for identities, repeated anchors, three operands, generated hops, and multiple target
  derivations.
- I ran `cargo test -p musa-kernel -q`; all 58 current tests passed.

### Judged

- The F and L theorem verdicts are mathematical judgments about the proposed definitions, not claims that Rust already
  realizes them.
- I classify the candidate as **Incomplete** because the core repairs are correct, while registry formation,
  canonical-path data, and the exact R1 factorization still need specification.

### Not checked

- Current Rust has not implemented `encode_sem`; it still hashes current N5 display bytes. The 58 passing tests do not
  discharge F2/T2.2 or the adversarial framing regression.
- No K₃.2 artifact registry, hop/path lineage store, preparation factorization, or versioned cache exists to inspect.
- I did not check cross-target floating-point/device conformance; K₃.2 correctly leaves it as an explicit R1 premise.

## Recommendation

Keep both K₃.2 repairs. Before calling closure complete:

1. restore `prepare_execution(Sem(M),Bindings,Seed,Options)→Result` exactly;
2. define and require `RegistryWellFormed(ᶜ)` with injective content-bound presentation ids;
3. give every hop/path field canonical decidable equality and framing; and
4. include payload-schema identity in every persisted semantic/cache key.

Then repair governing N3–N6 and the Rust semantic-hash writer together, with the K₃.1 injection as a regression and
arbitrary-byte property tests. The typed-path lineage model is ready for implementation only after the registry manifest
and canonical path record are fixed; its category proof does not need another conceptual replacement.
