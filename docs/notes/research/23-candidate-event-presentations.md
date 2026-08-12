# Candidate E₃ — finite event presentations before metric realization

**Status: partially refuted candidate; governs nothing.** The hardest case in [22](22-encodings-and-sieve.md) exposed
three structures that ordinary lists and timelines encode only by convention:

- causal prerequisite without a fixed duration;
- concurrent independence rather than accidental simultaneity; and
- incompatible alternatives whose choice remains unresolved.

Winskel's event structures were designed to separate exactly these. The full theory includes infinite structures,
general enabling, recursion, synchronization algebras, games, and strategies. Candidate E₃ takes only a finite prime
fragment and asks whether it makes Musa's intended examples simpler.

The immediate question is:

> Does a finite labelled event structure deserve to be a deep source value beside `Timeline`, or can domain packages
> continue to define ordinary graph data without losing important generic laws?

---

## 1. Definition

For `A : Data`, a finite event presentation is

```text
EventPlan A = (E, ≤, #, ℓ)
```

where:

- `E` is a finite set of stable event identities;
- `≤` is a partial order on `E`, called causal dependency;
- `#` is a symmetric, irreflexive conflict relation hereditary along causality:

  ```text
  e # f ∧ f ≤ g  ⇒  e # g;
  ```

- `ℓ : E → A` labels each event with domain data.

A **configuration** is a subset `x ⊆ E` which is:

1. down-closed: `e∈x ∧ d≤e ⇒ d∈x`;
2. conflict-free: no `e,f∈x` satisfy `e#f`.

Two events are **concurrent** when they are incomparable under `≤` and not in conflict. This is logical independence,
not equal onset.

This is the finite prime-event-structure presentation used in the newer event-structure literature. It is slightly more
restrictive than the general `(E,Con,⊢)` definition in
`~/Code/papers/logic-and-computation/semantics/event-structures/text.md`; the restriction buys a small validator and a
canonical finite form.

## 2. Formation and public operations

```text
event      : ∀ A. A → EventPlan A
before     : ∀ A. EventPlan A × EventPlan A → EventPlan A
beside     : ∀ A. EventPlan A × EventPlan A → EventPlan A
choose     : ∀ A. EventPlan A × EventPlan A → EventPlan A
map_event  : ∀ A B. (A → B) × EventPlan A → EventPlan B
configs    : ∀ A. EventPlan A → FiniteSet (Configuration A)
```

All binary constructors first take a tagged disjoint union, preserving event identity.

- `before(P,Q)` inherits each side's order and conflict and adds `p≤q` for every `p∈P`, `q∈Q`.
- `beside(P,Q)` inherits each side and adds no cross relation.
- `choose(P,Q)` inherits each side and adds `p#q` for every cross pair, with hereditary closure.
- `map_event(f,P)` changes only labels.

Typing is ordinary:

```text
Δ;Γ ⊢ P : EventPlan A    Δ;Γ ⊢ Q : EventPlan A
──────────────────────────────────────────────── Before
Δ;Γ ⊢ before(P,Q) : EventPlan A

Δ;Γ ⊢ f : A → B    Δ;Γ ⊢ P : EventPlan A
────────────────────────────────────────────── Map-Event
Δ;Γ ⊢ map_event(f,P) : EventPlan B
```

Construction cannot make an ill-formed plan. A raw interchange parser validates finiteness, acyclicity, irreflexivity,
symmetry, and hereditary conflict before constructing the opaque value.

### 2.1 Refutation: `before` is not total on prime event structures

The definition above is wrong when the left plan contains a choice. Let

```text
P = choose(event(a),event(b))
Q = event(c).
```

The proposed `before(P,Q)` adds both `a≤c` and `b≤c`, while `a#b`. Thus the causal past of `c` contains a conflict. No
configuration can contain `c`, because down-closure would require both incompatible causes. The constructor promised a
sequence “choose `a` or `b`, then do `c`” and instead made `c` unreachable.

There are three honest repairs, none definitionally free:

1. restrict `before(P,Q)` to conflict-free `P`, which fails the ordinary choice-then-continuation example;
2. duplicate `Q` once per compatible maximal branch of `P`, retaining which copy followed which branch; this can be
   exponentially large and makes intensional identity choices load-bearing; or
3. move from prime event structures to the general `(E,Con,⊢)` presentation, where distinct consistent sets can enable a
   continuation. That requires stable/disjunctive-enabling choices and a larger theory.

The rest of this note records what the prime fragment would buy *if* this hole were repaired, but §9's prior verdict
that it has earned a prototype is withdrawn. Candidate E₃ has not yet found its smallest composition law.

## 3. Denotation and normal form

The denotation of a plan is its finite family of labelled configurations, including event identity and causal order
restricted to each configuration. Equality of configuration *sets alone* would be too coarse: distinct intensional
choices can have the same visible labels, and event-structure semantics deliberately retains that distinction.

The normal form is:

1. a canonical topological ordering refined by stable identity;
2. the transitively reduced causal graph;
3. the minimal conflict pairs whose hereditary closure gives `#`;
4. canonical label bytes; and
5. versioned event identities/provenance.

Semantic equality quotients construction association and tag spelling but not duplicate alternatives with identical
labels. Choice is therefore non-idempotent at the intensional level.

## 4. Laws and non-laws

Up to canonical renaming of tagged event identities:

```text
before(before(P,Q),R)  ≅ before(P,before(Q,R))             E1
beside(beside(P,Q),R)  ≅ beside(P,beside(Q,R))             E2
beside(P,Q)            ≅ beside(Q,P)                       E3
choose(choose(P,Q),R)  ≅ choose(P,choose(Q,R))             E4
choose(P,Q)            ≅ choose(Q,P)                       E5
map_event(id,P)        = P                                  E6
map_event(g∘f,P)       = map_event(g,map_event(f,P))        E7
```

No general distributivity is asserted. In particular, sharing an initial event before a choice is not the same
intensional plan as duplicating that event into both branches:

```text
before(P,choose(Q,R)) ≠ choose(before(P,Q),before(P,R))
```

The configuration families may have similarly labelled runs, but provenance and causal identity differ. This mirrors the
temporal kernel's refusal to distribute sequence over overlay.

## 5. Timed realization

A chosen configuration `x` becomes metric only through a schedule:

```text
Schedule c P = { σ : events(P) ⇀ Interval c |
                   dom(σ) is a configuration x,
                   d < e in x ⇒ end(σ(d)) ≤ start(σ(e)) }
```

The strict relation `d<e` imposes succession; concurrent events may overlap, coincide, or occur in either order. A
schedule carries a finite ambient extent bounding every interval.

```text
realize : ∀ c A. EventPlan A × Schedule c P → Timeline c A
```

maps each scheduled event to one occurrence with its label. The plan does not infer a schedule, tempo, meter, or
duration.

**Proposition E8 — realization is well formed.** Every valid schedule realizes to a well-formed timeline.

*Argument.* The schedule's ambient bound gives `0≤start≤end≤extent` for each event. Its domain is finite because `E` is
finite. These are exactly the timeline formation conditions. ∎

**Proposition E9 — realization preserves conflict exclusion.** No realized timeline contains both events from a conflict
pair.

*Argument.* The schedule domain is a configuration and therefore conflict-free. ∎

Realization does not reflect concurrency: two incomparable events can be scheduled sequentially, and two events with no
causal relation may happen to share an onset. The plan and timeline are different information.

## 6. Examples that improve

### 6.1 Ossia and mobile form

An ossia is `choose(main,alternative)` until a realization selects a configuration. A mobile section uses `beside` for
independent modules plus causal edges for cues. No fake durations are needed before performance.

### 6.2 Improvisational pathways

A finite phrase grammar can unfold to a finite plan for a bounded section. Alternative continuations are conflict;
prerequisites are causality; independently available responses are concurrent. The plan does not claim that the grammar
defines a rāga or maqām—it is only a reusable carrier for one finite set of possibilities.

### 6.3 Performer interaction

A finite protocol fragment can distinguish a dancer move, the response it enables, and independent ensemble actions. The
game/strategy literature shows how input/output polarity and receptivity can extend this to open interactive systems,
but E₃ does not import that apparatus yet.

### 6.4 Orchestration choices

Alternative instrument assignments or optional doublings can remain unresolved choices. A selected configuration plus
orchestration-specific schedule realizes to gesture facts and bindings.

## 7. Examples that do not improve

### 7.1 Fixed score passages

A fully scheduled passage already has more information in `Timeline`. Reconstructing a causal order from numeric spans
is not canonical. Wrapping every ordinary score in `EventPlan` adds ceremony without removing a side condition.

### 7.2 Chords, keys, rāgas, and maqāms

The plan provides no musical classification. It can carry the relevant domain labels and relations, but it does not make
those theories fall out.

### 7.3 Audio processors

An event plan describes possible event occurrences, not a causal state machine from input histories to output histories.
A filter, oscillator, or reverb is not a finite configuration of events. `Process` remains distinct.

### 7.4 Notation/audio correspondence

A timed realization connects one configuration to a timeline, but engraving and audio still need theory-specific
interpretation and preparation. E₃ is not the common carrier of every realization.

## 8. Dependent-looking schedules without dependent type conversion

The notation `Schedule c P` appears to index a type by the plan value `P`. Putting arbitrary plans into definitional
type equality would reintroduce the DML problem K₁ removed.

The implementation should instead use a sealed existential witness:

```text
data Scheduled c A = exists PId. Scheduled {
    plan     : PlanRef PId A,
    schedule : CheckedSchedule PId c
}

schedule : EventPlan A × RawSchedule c
         → Result (Scheduled c A) ScheduleError
```

`PId` is a fresh nominal identity created by validation. A checked schedule cannot be paired with another plan, but the
type checker compares only nominal identities. The validator—not reduction of an event graph inside a type—discharges
the domain constraints.

This is the same pattern as an ensemble module exporting pitches valid for that ensemble. Generativity handles stable
dependencies; `Result` handles dynamic ones.

## 9. Sieve result

| Test | Result |
| --- | --- |
| Empty | empty plan has configuration `∅`; natural |
| One event | `event(a)`; natural |
| Composition | `before`, `beside`, and `choose` have distinct laws; natural |
| Dependency | nominal `PlanRef` avoids arbitrary value indices; natural but adds identity machinery |
| Hard combined | a bounded interactive choice schedules into a timeline; unbounded open interaction still needs `Process` or strategies |

**Judgment, revised by §2.1.** E₃ has two genuine callers: mobile/ossia form and finite phrase/interaction
presentations. But its attractive four-constructor API is not closed: the first composition after a choice fails. It
does **not** yet earn a prototype deep module, primitive surface syntax, replacement of `Timeline`, or the title
“motive.”

The next paper experiment is to compare general stable enabling with explicit branch duplication on the single term
`before(choose(a,b),c)`, then on a mobile section with rejoining paths. No implementation should begin until one account
has a simple associative composition law and preserves choice provenance.

## 10. Source provenance

The mathematical definitions are adapted from:

- `~/Code/papers/logic-and-computation/semantics/event-structures/text.md`, especially §§1.1 and 2.1–2.4, for
  consistency, enabling, configurations, morphisms, synchronization, and the warning that configurations do not recover
  all intensional event-structure data;
- `~/Code/papers/logic-and-computation/semantics/games-and-strategies-as-event-structures/text.md`, especially §§1–2,
  for the prime `(E,≤,Con)` presentation, concurrency as consistent incomparability, intensional nondeterminism, and
  interaction as synchronization followed by hiding.

E₃ intentionally omits recursive event structures, general synchronization algebras, polarity, strategies, copycat,
receptivity, courtesy, and bicategorical composition. Each would need a current Musa caller and its own proof before
admission.
