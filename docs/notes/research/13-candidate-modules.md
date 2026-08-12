# Candidate M — functors are the representable links

**Status: candidate component. Governs nothing.** [12](12-candidate-process-worlds.md) gives each native world an
internal compositional structure. This note asks for the weakest link between worlds which still respects those internal
maps.

**One-line claim.** A notation–performance or performance–audio link should be a module (profunctor), not generally a
functor and never an identification. A functor is recovered exactly as a representable module. Thus deterministic
realization falls out as a special case of correspondence rather than being forced on every link.

The local source uses “module,” “profunctor,” “distributor,” and “correspondence” as synonyms and develops categories,
functors, modules, and module maps as a virtual equipment
([Elements of ∞-Category Theory, chapters 7–8](../../../../papers/category-theory/elements-of-infinity-category-theory/part-ii/chapter-8-the-calculus-of-modules.md)).
The candidate below is the ordinary 1-categorical fragment. Nothing in Musa currently requires the ∞-categorical
generality of that source.

---

## 1. The level audit

Candidate R used a relation `R ⊆ Ob(C) × Ob(D)` between the objects of two worlds. This is a shadow. It forgets what
happens when either object is transformed by a native map.

For example, suppose a written passage `n` is coherent with an audio object `a`. If `f : n′ → n` is a notation
transformation and `g : a → a′` an audio transformation, the theory should say how the witness linking `n` and `a`
changes along `f` and `g`. A bare relation has no such action. Adding each action as an unrelated predicate would make
naturality a convention rather than structure.

The exact object is therefore a family of witness sets indexed by both categories, contravariant in the source world and
covariant in the target world.

---

## 2. Modules and their elements

Let `C` and `D` be small categories.

**Definition 13.1 (module).** A module `P : C ⇸ D` is a functor

`P : Cᵒᵖ × D → Set`.

For objects `c ∈ C` and `d ∈ D`, an element `p ∈ P(c,d)` is a coherence witness linking `c` to `d`. For maps
`f : c′ → c` and `g : d → d′`, functoriality supplies a transported witness

`P(f,g)(p) ∈ P(c′,d′)`.

Identity and composition laws for this action are not optional musical axioms. They are the functor laws of `P`.

This variance matches use: precomposing the source object and postcomposing the target object transport an existing
link. The opposite orientation is represented by a module in the opposite direction.

### 2.1 The profile example

Let `N` be a category of written articulations and notation-preserving transformations, and `G` a category of
performance gestures and gesture transformations. A profile-dependent module `P_profile : N ⇸ G` assigns to
`(staccato, gesture)` the set of profile choices and derivations under which the gesture realizes the mark.

There may be several such witnesses, or none. Transforming the notation or gesture transports a witness only when the
profile theory defines the required action. This records more structure than the set-theoretic relation in [11] without
claiming a unique gesture.

---

## 3. Functors as representable modules

Let `F : C → D` be a functor.

**Definition 13.2 (right-represented module).** The module represented by `F` is

`F∗(c,d) = D(Fc,d)`.

Its elements are maps from the deterministic interpretation `Fc` to the compared target `d`. Contravariance in `c` is
precomposition by `Ff`; covariance in `d` is postcomposition.

Thus a functor does not compete with modules. It embeds into them. A link is functional precisely when it is suitably
representable; representability is a property to establish, not a grammar choice imposed on every realization.

### 3.1 Composition

For modules `P : C ⇸ D` and `Q : D ⇸ E`, their composite, when the indicated coend exists, is

`(Q ⊙ P)(c,e) = ∫^{d ∈ D} P(c,d) × Q(d,e)`.

Concretely, an element is represented by an intermediate object `d` and a pair of witnesses, modulo the relation which
moves a `D`-map from the right action on `P` to the left action on `Q`. In discrete categories this reduces to ordinary
relational composition: choose an intermediate object and existentially hide it.

**Proposition 13.3 (represented links compose as functors do).** Let `F : C → D` and `G : D → E`. There is a natural
isomorphism of modules

`G∗ ⊙ F∗ ≅ (G ∘ F)∗`.

**Proof.** At `(c,e)`, the left side is

`∫^{d ∈ D} D(Fc,d) × E(Gd,e)`.

Define a map to `E(GFc,e)` by sending a representative `(u : Fc → d, v : Gd → e)` to `v ∘ G(u)`. If `h : d → d′`, the
coend identifies `(h ∘ u,v)` with `(u,v ∘ G(h))`; both representatives map to `v ∘ G(h) ∘ G(u)`, so the map is well
defined.

Conversely, send `w : GFc → e` to the coend class represented by `(id_Fc : Fc → Fc, w : GFc → e)`. The first composite
is the identity because `w ∘ G(id_Fc) = w`. For a representative `(u,v)`, the reverse composite is represented by
`(id_Fc, v ∘ G(u))`, which the coend relation identifies with `(u,v)`. These maps are natural in `c` and `e` because
they are defined using only functorial action and composition. Hence they form the claimed natural isomorphism. ∎

This proposition is the precise sense in which ordinary realization functors “fall out.” General modules compose even
when no deterministic intermediate realization exists; represented ones reduce to ordinary functor composition.

---

## 4. Parallel composition

Suppose `C` and `D` are symmetric monoidal process worlds. A bare module does not yet say that independent realization
witnesses combine in parallel.

**Definition 13.4 (lax monoidal module).** A lax monoidal module is a module `P : C ⇸ D` together with coherent maps

```text
P(c,d) × P(c′,d′) → P(c ⊗ c′, d ⊗ d′)
1 → P(I_C, I_D),
```

natural in all four variables and satisfying the associativity, unit, and symmetry diagrams for a lax monoidal functor
`Cᵒᵖ × D → Set`.

The comparison need not be invertible. Two independently realized tones may combine to a valid realization of their
parallel configuration, while a realization of the whole may contain coupling, room response, or voice-leading
information not decomposable into independent realizations.

**Corollary 13.5 (parallel witnesses compose).** If `p ∈ P(c,d)` and `q ∈ P(c′,d′)`, a lax monoidal module supplies a
canonical witness `p ⊗ q ∈ P(c ⊗ c′,d ⊗ d′)`.

**Proof.** Apply the binary lax monoidal comparison map to `(p,q)`. ∎

This is stronger than declaring a predicate `realizes(c,d)`: it makes the relation between chord construction and
parallel audio construction functorial data subject to coherence laws.

---

## 5. A minimal term discipline

The internal language need not expose coends or ∞-categories. It can distinguish four judgments:

```text
w world
A object @ w
f : A → B @ w
P : w ⇸ v
p : P[A,B]
```

Native maps have identity, serial composition, and tensor from Candidate S. Link witnesses have:

- left and right action by native maps;
- parallel tensor when the module is declared lax monoidal;
- multi-link composition with intermediate objects hidden;
- module maps as coherent transformations between link families.

A typed hypergraph remains a plausible concrete normal form: native directed boxes live inside worlds; link boxes cross
world boundaries; hidden intermediate vertices implement module composition. The categorical semantics now tells the
normalizer which graph rewrites are structural and which apparent rewrites would illegally identify worlds.

---

## 6. Operational semantics

A represented module has a functor behind it and can inherit a deterministic implementation. A general module is a
specification or query space:

- synthesis chooses or computes a target and witness from a source;
- analysis searches in the opposite direction;
- score following updates a family or distribution of witnesses;
- an editor may retain the witness as provenance without executing it.

Executability is therefore a property or selected orientation of a module, not part of the definition of link. This
prevents denotational many-valuedness from infecting deterministic audio scheduling while avoiding the opposite error of
declaring every musical interpretation functional.

---

## 7. IUT analogy, with a stop rule

The useful analogy is structural:

- each world retains its own objects, maps, and equalities;
- a cross-world module is not an equality of objects;
- maps on either side act on the link without becoming the same map;
- represented links recover functors when preservation is genuinely present.

The stop rule is equally important. No claim is made that Hodge theaters are categories of notation or audio, that a
Theta-link is a profunctor, or that IUT proves this choice. The local module calculus, not IUT terminology, supplies the
mathematical definition.

---

## 8. Emergence audit

| Intended concept | Does it fall out? | Reason |
| --- | --- | --- |
| Many-valued realization | **Yes** | several elements of `P(c,d)` or several target objects |
| Deterministic realization functor | **Yes** | represented module |
| Composition through an intermediate world | **Yes** | coend/module composition |
| Composition of deterministic realizations | **Yes** | Proposition 13.3 |
| Parallel realization of a chord/process bank | **Yes, with one declared property** | lax monoidal module |
| Change on either native side | **Yes** | functorial left/right action |
| Key | **No** | requires a context/frame theory |
| Temporal entrance, rubato, and tail | **No** | requires temporal locality and support |
| Efficient normalization | **Unsettled** | coend quotient and module equations need a concrete finite presentation |

**Verdict.** Candidate M repairs Candidate R's vacuity: a link must be natural under the native maps and, when declared
monoidal, coherent with parallel composition. It also repairs Candidate S's overconstraint: functors are the
representable links, not the only links. It still does not explain the temporal and contextual indices over which the
worlds vary. Those are separate candidates, not fields to add opportunistically to `P`.
