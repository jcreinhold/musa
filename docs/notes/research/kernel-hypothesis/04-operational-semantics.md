# Operational semantics and typing rules

The kernel calculus under the hypothesis: its terms, its typing judgments, its evaluation, and the theorems that must
survive. The shape is `docs/rules/kernel/10-term-calculus.md` with two forms added and one theorem newly at risk.

The design constraint throughout is the one that document already fixed and that Amendment IV of `00-constitution.md`
restates: **no abstraction, no recursion, no function space.** Every addition below is checked against T4 (totality)
before it is proposed, and the check is the reason `act` is phrased as a group action rather than a map.

---

## 1. Signatures

The calculus is parametric in a **payload signature**

```text
Σ = (A, G, ·)
```

where `A` is a set of payload values, `G` is a group, and `·` is an action `G × A → A`. The kernel knows nothing else
about `A` — not that it is pitch, not that it has an ordering, not that it is finite. `Σ` is supplied by the consumer
exactly as payloads are supplied today (`docs/rules/kernel/01-grammar.md`).

The **time group** `T = ℚ>0 ⋉ ℚ` is fixed and built in: scaling by a positive rational and translation. `scale` and
`shift` are its generators. Retrograde extends `T` to allow negative scaling; whether it does is Q-H.

---

## 2. Terms

```text
t, u ::= timeline d { (s, e, a)* }      % literal                            — E1
       | seq t₁ … tₙ                    % succession                         — E2
       | over t₁ … tₙ                    % simultaneity                       — E3
       | alt t₁ … tₙ                     % alternative                        — E4   (new)
       | act g t                         % group action, g ∈ G ∪ T            — E5   (new)
       | restrict [i, j) t              % observation                        — E6
       | let x = t in u                 % sharing
       | x  |  x @ m                    % reference, marked reference
```

Two forms are added and one is *removed by absorption*: `scale r t` becomes `act r t` for `r` in the time group, so the
form count goes from seven to eight rather than nine. `shift` remains sugar with the expansion
`shift d t ≝ seq (timeline d { }) t`, unchanged.

`alt` is the only genuinely new *meaning*. Everything else is bookkeeping.

---

## 3. Typing

The kernel is monomorphic: there is one type of composition, `Music<Σ>`, and the judgments are about well-formedness.
Contexts `Γ` bind names to that type.

```text
──────────────────────  (T-Var)
Γ, x : Music<Σ> ⊢ x : Music<Σ>


Γ, x : Music<Σ> ⊢ x : Music<Σ>      m a mark
──────────────────────────────────  (T-Mark)
Γ, x : Music<Σ> ⊢ x @ m : Music<Σ>


d ∈ ℚ≥0     ∀i. 0 ≤ sᵢ ≤ eᵢ ≤ d     ∀i. aᵢ ∈ A
────────────────────────────────────────────────  (T-Timeline)
Γ ⊢ timeline d { (sᵢ, eᵢ, aᵢ)ⁿ } : Music<Σ>


Γ ⊢ tᵢ : Music<Σ>   (1 ≤ i ≤ n)          Γ ⊢ tᵢ : Music<Σ>   (1 ≤ i ≤ n)
──────────────────────────────  (T-Seq)  ──────────────────────────────  (T-Over)
Γ ⊢ seq t₁ … tₙ : Music<Σ>               Γ ⊢ over t₁ … tₙ : Music<Σ>


Γ ⊢ tᵢ : Music<Σ>   (1 ≤ i ≤ n)     n ≥ 1
──────────────────────────────────────────  (T-Alt)
Γ ⊢ alt t₁ … tₙ : Music<Σ>


Γ ⊢ t : Music<Σ>     g ∈ G ⊎ T
────────────────────────────────  (T-Act)
Γ ⊢ act g t : Music<Σ>


Γ ⊢ t : Music<Σ>     i, j ∈ ℚ,  i ≤ j        Γ ⊢ t : Music<Σ>    Γ, x : Music<Σ> ⊢ u : Music<Σ>
──────────────────────────────────────  (T-Restrict)  ─────────────────────────────────────────  (T-Let)
Γ ⊢ restrict [i, j) t : Music<Σ>                       Γ ⊢ let x = t in u : Music<Σ>
```

Three observations about what these rules deliberately do *not* say.

- **No type depends on a musical value.** `Music<Σ>` is indexed by a signature, which is data supplied by the consumer,
  and by nothing else. This is Amendment 1 of `~/Code/kan/docs/rules/constitution.md` applied: dependency is over an
  *admissible index domain*, and a piece of music is not one. Making `Music` depend on its extent, its voice count, or
  its style is the seductive move that `03-claims-and-styles.md` §5 rejects, and the typing rules are where the
  rejection is enforced.
- **`act` does not check that `g` is meaningful for the payload.** It cannot: the kernel does not know what `A` is.
  Well-typedness of `g` is `Σ`'s obligation, which is the same discipline payload values already live under.
- **`alt` requires at least one branch.** `alt` with zero branches would be a piece with no hearings, which is not
  silence — silence is `timeline d { }`, which has one hearing containing nothing. The distinction is real and the rule
  is where it is made.

---

## 4. Evaluation

Values are the denotations of `02-denotational-semantics.md` §1: pairs `(𝔈, d)`. Evaluation is big-step, `ρ ⊢ t ⇓ v`,
with `ρ` an environment mapping names to values.

```text
ρ ⊢ x ⇓ ρ(x)                                            (E-Var)
ρ ⊢ x @ m ⇓ Payload(f_m)(ρ(x))                          (E-Mark)
ρ ⊢ timeline d { … } ⇓ E1(…)                            (E-Timeline)
ρ ⊢ tᵢ ⇓ vᵢ   ⟹   ρ ⊢ seq t₁ … tₙ ⇓ v₁ ⊳ … ⊳ vₙ         (E-Seq)
ρ ⊢ tᵢ ⇓ vᵢ   ⟹   ρ ⊢ over t₁ … tₙ ⇓ v₁ ⊕ … ⊕ vₙ        (E-Over)
ρ ⊢ tᵢ ⇓ vᵢ   ⟹   ρ ⊢ alt t₁ … tₙ ⇓ v₁ ⊻ … ⊻ vₙ         (E-Alt)
ρ ⊢ t ⇓ v     ⟹   ρ ⊢ act g t ⇓ g · v                    (E-Act)
ρ ⊢ t ⇓ v     ⟹   ρ ⊢ restrict [i,j) t ⇓ v ↾ [i,j)      (E-Restrict)
ρ ⊢ t ⇓ v, ρ[x ↦ v] ⊢ u ⇓ w   ⟹   ρ ⊢ let x = t in u ⇓ w (E-Let)
```

where `⊳`, `⊕`, `⊻`, `g · –`, and `↾` are E2–E6 of `02-denotational-semantics.md`. Every rule is a direct reading of one
clause of the denotation, which is what keeps T3 true.

### 4.1 The duplication problem, and how sharing survives it

`02-denotational-semantics.md` E2 duplicates the continuation when the first operand has conflict. Naively this makes
`let x = alt a b in seq x (seq x x)` exponential, which would break T5 — observation commutes with sharing — in spirit
if not in letter, and would certainly break the performance budgets in `docs/rules/kernel/09-performance.md`.

The resolution is that the *value* need not be a materialized event structure. Represent a value as a **branch-indexed
structure**: a set of events each tagged with the branch choices it is contingent on, exactly the way a BDD represents
an exponential function in polynomial space. Sequential composition after a branching operand adds one tag; it does not
copy events. Configurations are enumerated on demand by `hearings`, and a consumer that asks for all of them of course
pays for all of them — but that is the consumer asking for exponentially many performances, which is a request, not an
accident.

> **Obligation.** The representation must satisfy: `hearings(v)` enumerates exactly `𝒞max(𝔈(v))`, and `|v|` is linear in
> the size of the term. This is the one place where the hypothesis has a non-obvious implementation requirement, and it
> should be prototyped before anything else is committed to.

---

## 5. Theorems

Six theorems must survive; here is the status of each.

**T1 — the constructors are a homomorphism.** Survives. E-Seq, E-Over, E-Alt, E-Act each evaluate to the denotation of
their arguments' denotations, by construction of the rules.

**T2 — `let` is transparent.** `⟦let x = t in u⟧ = ⟦u[t/x]⟧`. Survives, with one subtlety worth naming: after
substitution, two occurrences of `t` become two *disjoint* copies of its events, whereas before substitution they were
one value used twice. Under E-Let, `over x x` overlays a value with itself, and E3 takes a disjoint union, so the two
events are distinct — which is what X1 (overlay is not idempotent) already required. The theorem holds because both
sides produce isomorphic structures; it would fail if events had identities that survived copying, and that is the
reason `01-atoms.md` §8.3 insists events have no names.

**T3 — evaluation is normalization.** Survives, conditionally, and this is the honest weak point. Canonical form is now
up to *isomorphism of labelled event structures*, not equality of sorted multisets. Deciding it is:

- **Decidable**, since structures are finite.
- **Cheap in practice**, because the labels are rich: sort by `(start, end, payload)`, then refine each class by the
  labels of predecessors and successors to a fixpoint. Two events tie only if they have identical spans, identical
  payloads, and identical neighbourhoods, which for real music means genuine unisons in symmetric positions.
- **Not cheap in the worst case.** Residual ties require search, and the problem is graph-isomorphism-shaped.

Compared with the multiset denotation's linear-time canonical form, this is a real regression. It is bounded by the fact
that `|E|` is the note count of a piece and ties are rare, but it should be measured against
`docs/rules/kernel/09-performance.md`'s budgets before adoption, not after. Recorded as Q-I.

**T4 — totality.** Survives, and this is the theorem the whole design was shaped around. There is no abstraction, no
recursion, and no `fix`. `act` is total because a group action is a total function. `alt` is total because it is a
disjoint union of finite structures. The induction on term structure that proves T4 today goes through with two more
cases and no new ideas. Amendment IV's insistence on actions rather than maps is exactly this proof staying short.

**T5 — observation commutes with sharing.** Survives given §4.1's obligation. Restriction is an observation that touches
only visible spans, so it commutes with branch tags.

**T6 — instantiation preserves the denotation up to payloads.** Survives. Marks select payload maps, and payload maps
touch neither `≤` nor `#` nor timings.

**T7 — the action is a homomorphism (new).** For all `g` and all closed `t, u`:

```text
g · (v ⊳ w) = (g · v) ⊳ (g · w)      g · (v ⊕ w) = (g · v) ⊕ (g · w)      g · (v ⊻ w) = (g · v) ⊻ (g · w)
1 · v = v                             g · (h · v) = (gh) · v
```

For payload-group elements these are immediate, since the action touches only `λ`'s third component. For time-group
elements the first two are L14 and L15 restated, and the `⊻` case is new. This theorem is what makes a transformational
analysis — "this passage is `L` then `P` applied to that one" — a claim the kernel can check rather than a comment.

---

## 6. What is still deliberately absent

Unchanged from `docs/rules/kernel/10-term-calculus.md`, and worth restating because the additions above make the
temptation stronger, not weaker:

- **No abstraction and no application.** `act` exists so that `map` does not have to. If a consumer needs an arbitrary
  payload function, it applies one *outside* the kernel, to the value, as `Payload(f)` already allows.
- **No recursion.** Q1 in `docs/rules/kernel/08-open-questions.md` (infinite and live patterns) stays open and stays
  out.
- **No conditional.** A branch on a payload value would make evaluation depend on `A`, which the signature discipline
  forbids. `alt` is a *nondeterministic* branch that commits to nothing, which is a different thing and is why it can be
  added without this consequence.
- **No `join`.** X3 stands: nothing flattens `Music<Music<Σ>>`.
