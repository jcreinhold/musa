# 10 — The Kernel Term Calculus

**Status: governing** (graduated at prompt 48). Prompt 47 built the `Term` type and its evaluator, prompt 48 gave it a
text form (`01-grammar.md`) and a second producer/consumer; prompt 49 makes elaboration emit terms. Nothing in this
document was revised on the way through implementation — the two repairs prompt 48 made were to `01-grammar.md`'s
payload syntax and to `05-normalization.md`'s claim about N5, neither of which is a claim this document makes.

The kernel has been an algebra of *values*: you build a `Timeline` and the building is gone. This document adds a syntax
whose meanings are those same values — no new semantic domain, no new operation, no new equality. What it buys is three
things values cannot express, listed next.

## Why a calculus at all (the scope rule)

A term language is worth adding **only** for what values cannot express:

1. **Sharing.** A canon states its subject once and uses it four times. As a value, "four times" means four timelines in
   memory and four copies in every downstream walk; the fact that they are the *same* material is lost the moment the
   value exists. `let` records it.
2. **Deferred observation.** Asking what sounds in bars 40–44 of a piece should not require building bars 1–39. A term
   can be restricted before it is evaluated; a value cannot, because building it *is* evaluating it.
3. **Interchange.** A second implementation, a visualizer, or a test oracle needs a syntax to read. `01-grammar.md` has
   promised one since prompt 08; this document is the semantics that promise needs before a parser is honest.

Everything else is out of scope. This is not an invitation to binders-in-general, functions, application, recursion, or
computation. Concretely, the acceptance test for any proposed term form, applying §34's rule:

> A form belongs in the calculus only if it serves sharing, deferred observation, or interchange, **and** its meaning is
> a timeline `03-denotational-semantics.md` already defines.

A form that introduces a meaning `03` does not define is not a term — it is a proposal to change the kernel, and it goes
through `08-open-questions.md` and §34 like any other.

Terms are finite. Every term has a denotation in `(d, E)`. There is no term whose evaluation can fail to terminate,
because there is nothing to recur through (T4).

## The terms

```text
t, u ::= timeline d { (s, e, a)* }     % literal                      — D1
       | seq t₁ … tₙ                   % temporal succession          — D2
       | over t₁ … tₙ                  % simultaneous presence        — D3
       | scale r t                     % time scaling, r ∈ ℚ>0        — D5
       | restrict [i, j) t             % observation                  — D6
       | let x = t in u                % sharing; x scopes over u
       | x                             % reference
       | x @ m                         % marked reference                — T6
```

`d`, `s`, `e`, `i`, `j` are exact rationals; `r` is a positive exact rational; `a` is a payload value (`01-grammar.md`);
`x` ranges over names; `m` is an opaque **mark**, a string the kernel never interprets.

Six forms and a reference. Each of the first five is exactly one of `03`'s definitions, so the calculus adds no meaning;
`let` and `x` add sharing, which is a statement about *structure*, not about meaning (T2).

### The mark on a reference (prompt 49)

A marked reference `x @ m` denotes the same timeline as `x`, with the consumer's payload map applied once to the
instantiated copy. It exists because sharing and provenance pull in opposite directions and one of them was going to
lose.

Concretely: `repeat 3 { … }` elaborates to a `let` and three references, which is the whole saving — but every
occurrence of the third repetition must still carry `RepeatIteration(2)` in its origin, and the occurrences inside the
`let` are stated once and cannot each carry a different one. Either the payloads distinguish the uses and there is no
sharing, or the *references* do. The mark is the reference doing it.

Three constraints make this narrow enough to be worth having, and they are the whole of the addition:

1. **The mark is opaque.** It is a string; the kernel neither reads it nor gives it meaning. §12 is untouched — this is
   the same discipline payloads already live under (`01-grammar.md`).
2. **It selects a payload map, and nothing else.** Evaluation applies `Timeline(f)` (D7) to the instantiated value,
   where `f` is chosen by the consumer from `m`. Spans, extent, occurrence count and order are untouched, because D7
   already guarantees that (L9–L12). This is not a new operation: `map` is still not a term — no function is written
   down, and the consumer that owns the payload chooses the map, exactly as it chooses what the payload text means.
3. **An unmarked reference is the identity case.** `x` is `x @ m` with the identity map, so E-Var stays as it was and a
   consumer with no marks (every consumer but `musa-compiler`) is unaffected.

What this buys, and it is the point: the interchange file both *shares* and reproduces the compiled snapshot's
provenance byte for byte. Without it, prompt 49 has to choose, and either choice loses something the project already
promised — `06-surface-elaboration.md` records that the choice was faced.

### `shift` is sugar, and stays sugar

A delay is the single most common thing an interchange file will say — a canon's second voice enters after two bars —
and writing it as a sequence with an empty timeline obscures the intent:

```text
shift d t   ≝   seq (timeline d { }) t          % d ≥ 0
```

This is D8's `delay` under another name, and D8 is already derived. It is therefore specified here as **sugar with a
stated expansion**: a reader may write `shift`, and a well-formedness checker, evaluator, or printer may expand it
before doing anything else. Sugar with a mechanical expansion costs nothing semantically — there is one meaning, and it
is the sequence's. A *primitive* `shift` would cost something: another form for every consumer to match on, another case
in every proof, and a second way to say what `seq` already says. Under §34 that is not a bargain, and D4's striking at
prompt 37 is the precedent — an operation that only restates another is not part of the basis.

Printers write the expansion, not the sugar, so canonical text stays unique (N5). `shift` is an input convenience.

### `map f` is not a term

D7 defines payload mapping, and it is a real kernel operation. It is nevertheless **not** a term, and the asymmetry is
deliberate.

A term `map f t` requires `f` to be nameable. Naming a function means a syntax for functions, which means application,
which means the door to general computation is open — the exact line `00-purpose.md` draws ("not a general-purpose
programming language"). And the payload domain is where this would hurt most: `01-grammar.md`'s payload schemas are
first-order and boring on purpose (§24), and a function *over* payloads is the first thing that would need not to be.

So: **the calculus is one of temporal structure.** Payload transformation happens above it, during elaboration, exactly
as it does today — transposition applies eagerly and the timeline it produces already carries transposed payloads
(`06-surface-elaboration.md`). Terms carry already-mapped payloads. `Timeline(f)` remains available as a *function on
values* (D7, L9–L12) for the code that has an `f` in hand; what does not exist is a way to write `f` down in a file.

The cost is real and worth naming: an interchange file cannot say "this section is that section, transposed", only "this
section is these notes". It can still say "this section *is* that section" (`let`), which is the sharing case that
motivated the calculus. If a future consumer genuinely needs transposition-preserving interchange, the answer is a
*payload-level* concept — a named interval in the payload schema — not a function in the term language. Recorded here so
the next person does not reopen it by accident.

## Static semantics

Well-formedness is `02-static-semantics.md` K7 (scoping and term shape), which this document adds. In summary, a term is
well-formed when: every name is bound by an enclosing `let`; `let` does not shadow; `scale`'s factor is positive;
`restrict`'s window is ordered; `seq` and `over` have at least one argument, all of the same payload type; and every
literal satisfies K1. A well-formed **closed** term (no free names) is the input to evaluation.

## Evaluation

Evaluation is a relation between an environment, a term, and a value:

```text
ρ ⊢ t ⇓ (d, E)
```

where `ρ` maps names to values `(d, E)`. `ρ(x)` is the value bound to `x`; `ρ[x ↦ v]` extends it.

```text
                                   ∀i. 0 ≤ sᵢ ≤ eᵢ ≤ d
(E-Timeline)  ───────────────────────────────────────────────────────────────
              ρ ⊢ timeline d { (sᵢ, eᵢ, aᵢ)* } ⇓ (d, { (sᵢ, eᵢ, aᵢ) })


              ρ ⊢ t₁ ⇓ v₁   …   ρ ⊢ tₙ ⇓ vₙ
(E-Seq)       ─────────────────────────────────────
              ρ ⊢ seq t₁ … tₙ ⇓ v₁ ; … ; vₙ                            (D2)


              ρ ⊢ t₁ ⇓ v₁   …   ρ ⊢ tₙ ⇓ vₙ
(E-Over)      ─────────────────────────────────────
              ρ ⊢ over t₁ … tₙ ⇓ v₁ ⊕ … ⊕ vₙ                           (D3)


              ρ ⊢ t ⇓ v        r ∈ ℚ>0
(E-Scale)     ─────────────────────────────
              ρ ⊢ scale r t ⇓ scale_r(v)                               (D5)


              ρ ⊢ t ⇓ v        i ≤ j
(E-Restrict)  ─────────────────────────────────────
              ρ ⊢ restrict [i, j) t ⇓ restrict_{[i,j)}(v)              (D6)


              ρ ⊢ t ⇓ v        ρ[x ↦ v] ⊢ u ⇓ w
(E-Let)       ────────────────────────────────────
              ρ ⊢ let x = t in u ⇓ w


                   x ∈ dom(ρ)
(E-Var)       ──────────────────────
              ρ ⊢ x ⇓ ρ(x)


                   x ∈ dom(ρ)        f = φ(m)
(E-Mark)      ──────────────────────────────────
              ρ ⊢ x @ m ⇓ Timeline(f)(ρ(x))                            (D7)
```

E-Mark is parameterized by the consumer's `φ`, a function from marks to payload maps, fixed for one evaluation.
`φ(m) = id` for every `m` recovers E-Var, which is why an evaluator with no marks needs no `φ` at all.

Three properties of these rules are load-bearing and easy to lose:

- **`let` is call-by-value.** `t` is evaluated once, when the binding is made, and its *value* is bound. This is what
  makes sharing a cost saving rather than a duplication in disguise, and it is safe precisely because there is no
  effect, no failure, and no divergence to observe (T4).
- **`;` and `⊕` are the operations of `03`, not new ones.** E-Seq and E-Over do not define anything; they hand off. If a
  reading of these rules disagrees with `03`, `03` is right.
- **`restrict` in a term evaluates its argument first.** That is the *specification*; it is not the implementation
  strategy. T5 is what licenses an implementation to push the restriction inward and build less, and prompt 50 is where
  that is measured and only then done.

`restrict` deserves one more line, because D6 makes it an *observation* with both spans. Evaluating a `restrict` term
yields the timeline of visible occurrences, with whole spans preserved as D6 requires — restriction never rewrites where
an occurrence began, in a term any more than in a value.

## Theorems

These are the theorems that make this a calculus rather than a file format. Each names the property test that implements
it (prompt 47, `crates/musa-kernel/tests/terms.rs`), in the style `04-algebraic-laws.md` uses. `⟦t⟧` abbreviates the `v`
with `∅ ⊢ t ⇓ v`, for closed well-formed `t`; equality is semantic equality (N4).

- **T1 — the constructors are a homomorphism.** For all closed well-formed terms,

  ```text
  ⟦seq t u⟧ = ⟦t⟧ ; ⟦u⟧          ⟦over t u⟧ = ⟦t⟧ ⊕ ⟦u⟧
  ⟦scale r t⟧ = scale_r(⟦t⟧)      ⟦restrict I t⟧ = restrict_I(⟦t⟧)
  ```

  Consequence, and the reason to state it: **L1–L18 are laws about terms too, by transport.** `seq (seq t u) v` and
  `seq t (seq u v)` denote equal timelines because L1 says so; the calculus inherits the algebra rather than needing
  its own. Test: `term_constructors_are_a_homomorphism`.

- **T2 — `let` is transparent.** `⟦let x = t in u⟧ = ⟦u[t/x]⟧`, where `u[t/x]` is capture-avoiding substitution (K7's
  no-shadowing rule makes capture impossible, so substitution is textual). Sharing changes cost, never meaning: a term
  with `let` and its expansion are indistinguishable in the semantics, which is exactly why an implementation is free to
  choose either. Test: `let_is_transparent`.

- **T3 — evaluation is normalization.** For closed well-formed `t`, `normalize(⟦t⟧)` is the canonical form (N1–N2), and
  for any `t`, `u`:

  ```text
  ⟦t⟧ ≡ ⟦u⟧   ⟺   canonical(⟦t⟧) = canonical(⟦u⟧)   ⟺   hash(⟦t⟧) = hash(⟦u⟧)   (up to N6's collision bound)
  ```

  This is the theorem that stops the term language from becoming a second notion of equality. There is one equality in
  this kernel; terms do not get their own, and two terms are equal exactly when the timelines they denote are.
  Test: `evaluation_agrees_with_normalization`.

- **T4 — totality.** Every closed well-formed term evaluates, in finitely many steps, to a value. There is no diverging
  term: there is no recursion (K4, Q5), no fixpoint, no application, and every constructor takes a finite number of
  finite arguments, so evaluation is structural recursion over a finite tree. Evaluation is also *deterministic* — the
  rules are syntax-directed, one per form. Test: `every_well_formed_term_evaluates`.

- **T5 — observation commutes with sharing.** For any window `I` and any closed well-formed `let x = t in u`:

  ```text
  restrict I (let x = t in u)  =  let x = t in restrict I u
  ```

  Immediate from T2 and T1, but stated separately because it is the law that makes deferred observation *sound*: it
  says an implementation may push a restriction through a binding without changing the answer, which is the enabling
  step for observing a window without evaluating the whole. Prompt 50 depends on it. If it could not be stated
  cleanly, prompt 50 would have no foundation and should be struck; it can, and the reason is that `let` binds a value
  rather than a computation. Test: `restriction_commutes_with_sharing`.

  Note what T5 does **not** say. It does not say `restrict I (seq t u) = seq (restrict I t) (restrict I u)` — that is
  false, because `seq` translates its second argument and a window in the composite names different material than the
  same window in the parts. The push-inward rule for `seq` has to translate the window, and stating it is prompt 50's
  job, with the measurement that justifies doing it at all.

- **T6 — instantiation preserves the denotation up to payloads.** For any `φ`, any closed well-formed `let x = t in u`,
  and `u'` the term `u` with every mark erased:

  ```text
  spans(⟦let x = t in u⟧_φ)  =  spans(⟦let x = t in u'⟧)
  ```

  and the two agree occurrence-for-occurrence in canonical order, differing only in payloads. Marks therefore cannot
  change *when* anything sounds, only what a payload says about itself — which is exactly the latitude provenance
  needs and the only latitude it gets. Immediate from D7's L9–L12 (mapping preserves support and distributes over `;`
  and `⊕`), stated separately because it is the property prompt 49's "provenance must be byte-identical" rests on.
  Test: `a_mark_changes_payloads_and_nothing_else`.

  The converse warning: T6 does **not** say marks preserve semantic equality (N4), and they must not — two references
  to one body marked differently denote timelines that are *deliberately* unequal, because their occurrences carry
  different provenance. That is the whole reason the mark exists.

### What the suite checks beyond the five theorems

- `the_algebra_transports_to_terms` and `synchronized_interchange_holds_of_terms` — L1, L4, L5 and L18 asked at the term
  level. They are T1's consequence, and they are tested rather than argued because a disagreement between the calculus
  and the algebra would be a bug in *this document*, not in the code.
- `shift_denotes_its_stated_expansion` — the sugar's expansion, so `shift` cannot quietly become a primitive.
- `ill_formed_terms_are_rejected` — the two rules `check` answers, plus the four the constructors make unrepresentable.

## What is deliberately absent

Recorded with reasons, so that each stays absent for a reason rather than by omission:

- **No functions, no application, no lambda.** The door to general computation, and the line `00-purpose.md` draws. The
  calculus has binders (`let`) but no abstractions — a distinction worth keeping sharp, since `let` here is a name for a
  *value*, not a parameter.
- **No recursion, no fixpoint, no cyclic references.** Q5 keeps recursion a surface-language question, and K4 already
  requires the reference graph to be acyclic. Recursion would also destroy T4, which every consumer relies on.
- **No `Pattern` type, no loops, no infinite terms.** Q1 stays open, and §18 puts patterns *above* the finite kernel as
  producers of finite observations. A calculus makes patterns look tractable; that is a reason for suspicion, not for
  settling Q1 in passing.
- **No `join`, no nested timelines.** §16: `Timeline[Timeline[A]]` has no canonical flattening, and a syntax for it
  would smuggle in a choice among begin-at-onset, stretch-to-fit, crop, and repeat.
- **No conditionals, no arithmetic on terms.** Both are computation. Rationals appear in terms as *literals*; nothing
  computes them.
- **No `map`.** See above — payload transformation is not temporal structure.
- **No `reverse`.** Prompt 34 established that retrograde is elaboration reading material backwards and needs no kernel
  primitive. A calculus is not new evidence, and this document does not reopen it.
- **No queries as term forms.** `covering` and `prevailing` (D10–D11) are questions asked of a value, not ways of
  building one: a term denotes a timeline, and neither query does. They apply to `⟦t⟧` like any other observation, and
  giving them syntax would mean a term language with two kinds of result.
- **No curve form.** Continuous shape landed one prompt before this one as a payload *value* (`Progress`, §32 Q4
  resolved), which is why `a` in a literal already carries it and the grammar above needs no `curve` production. This is
  the scope rule paying off exactly once, on the first construct that tested it: a form that would have added a meaning
  `03` does not define was not needed, because `03` was extended with a value instead of the calculus with a form.
- **No binary format, no versioning, no schema negotiation.** One text form, prompt 48.

## Relationship to the other documents

| Document | Relationship |
| --- | --- |
| `00-purpose.md` | The calculus lives under its "not a general-purpose programming language" line; the scope rule above is that line applied to terms. |
| `01-grammar.md` | The concrete syntax *of these terms*. This document is the semantics; that one is the notation, and prompt 48 implements it. |
| `02-static-semantics.md` | K7 states well-formedness for terms. |
| `03-denotational-semantics.md` | Every term's meaning. The calculus adds no operation to D1–D12. |
| `04-algebraic-laws.md` | L1–L19 transport to terms through T1; L24 needs no transport, being a fact about payloads. L9–L12 are what T6 rests on. |
| `05-normalization.md` | N1–N6 apply to the *values* terms evaluate to. T3 is what ties the two together. |
| `08-open-questions.md` | Q6's trigger; Q1, Q2, Q5, Q9 stay open. Q4 is **resolved** — see below. |
