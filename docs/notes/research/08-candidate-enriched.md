# Candidate E — the enriched core, and staged erasure

**Source.** Peyton Jones, *The Implementation of Functional Programming Languages*, ch. 3 (§3.1, §3.2, **§3.2.4**), ch.
6 (§6.2.2, **§6.5**). The second of the two main references named for this work, and the one that turns out to bear
directly on the question the directory opened with.

**One-line claim.** The studio dilemma — "no calculus" versus "unify the two cores" — is a false one. SPJ's compilers
answer a structurally identical question with a third option that is neither, and it is the option that matches what
musa already needs.

---

## 1. The construction

SPJ does not translate the surface language into the small core directly. He interposes an **enriched lambda calculus**:
a strict superset of the ordinary calculus, with four extra constructs (`let`/`letrec`, pattern-matching lambdas, the
fat bar `[]`, and `case`), chosen so that the front translation is "little more than a change of syntax". All the real
work happens as *transformations within the enriched language*, ending in the small core.

Two reasons are given for the interposition (§3.1), and both apply to musa unchanged:

1. The surface language "is designed to be a language for programmers, not compilers", and lacks what a
   transformation-based compiler needs.
2. Front ends are largely syntactic variants of one another, so keeping the transformations inside the enriched language
   makes them reusable across surfaces — change only the front translation.

That is the course correction's own architecture, arrived at independently: a surface language that elaborates into a
small kernel. What musa does *not* currently have written down is the middle language and the discipline governing what
is allowed to live in it.

## 2. §3.2.4, which is the whole point

Having built the machinery to erase `let` into `(λv.E) B`, SPJ then argues **against doing it**:

> …is using a sledgehammer (lambda abstraction) to crack a nut (let-expressions). The lambda abstraction `(λv.E)` could
> be applied to many arguments, but it is in fact only ever applied to one, namely `B`.

Three reasons follow. The first is the one that matters here:

> …it is not possible to type-check the program once it has been transformed into the ordinary lambda calculus, but the
> addition of simple `let(rec)`-expressions is sufficient to solve the problem.

And §6.5 states the resulting discipline in one sentence:

> This is essential for type-checking, though it can be transformed into the ordinary lambda calculus **after that**.

> **Proposition 7 (cited).** A construct earns its place in the core not by being primitive, and not by being
> irreducible, but because **erasing it destroys information a later pass needs**. The core an analysis runs on and the
> core an evaluator runs on need not be the same core. Erasure is *staged*: each construct survives exactly as far as
> its last consumer.

This is a criterion, and criteria are what `docs/core-boundary.md` §6 is short of. §6's eight forbids are stated as
positions. Proposition 7 is a test: *name the pass that needs the structure, or erase it.*

## 3. What it says about the studio

`docs/core-boundary.md` §6.7 says there is no calculus under the studio. [01](01-realizations-and-residue.md) E8 and
Proposition 1 say a patch is not part of the motive — five of six realizations extract nothing from it. Both are about
the **denotation**. Proposition 7 is about the **term language**, and the two do not conflict:

| Question | Answer | From |
| --- | --- | --- |
| Is a patch part of the object every realization factors through? | **No.** It is coefficient data of one realization. | Prop 1 |
| Does the patch's *description* belong in the one term language? | **Yes, until its last consumer.** | Prop 7 |

So the dilemma that opened this directory was malformed. "Unify the two cores" read as *one algebra containing both
timelines and signal graphs* is wrong for the reason Prop 1 gives. Read instead as **one term language, in which the
studio's finite description is an ordinary term with the ordinary properties, erased into the render plan at the point
where nothing downstream reads it again** — it is right, and it is what SPJ's compilers do with `let`.

> **Derived (D-1).** `docs/core-boundary.md` §6.7 is defensible under the reading "no *second temporal* calculus" and
> indefensible under the reading "no structure". The defect identified at the top of this directory stands, and Prop 7
> supplies the criterion that replaces §6.7's assertion with a test: *name the pass that needs the structure, or erase
> it.*

**D-1 originally named the wrong object, and [07](07-probe-log.md) P-3 corrects it.** The first draft claimed R1's cache
key `semantic_hash(M) ⊕ B ⊕ s` is uncomputable because `StudioGraphSpec` lacks `PartialEq` and `Hash`. But R1 quantifies
over *instrument bindings* `B`, not over the studio patch graph — `docs/core-boundary.md`'s own ledger row for prompt
158 says "`PartId` on the gesture payload is what B already implies" — and `B` does not exist yet, since prompts 156,
157, and 158 are all `pending`. So there is no present defect there; there is a **design constraint on prompts 157 and
158**, which is cheaper to satisfy at introduction than to retrofit: whatever `B` becomes must be comparable and
hashable, because R1's cache is only well-defined if it is.

Where Prop 7 *does* bite is the seam this directory noticed at the outset and then walked past:
`musa_compiler::StudioSpec` and `musa_audio::StudioGraphSpec` are two independent representations of the same graph.
Prop 7 asks the question that settles it — name the pass that needs the second one's structure, or let the first survive
to that pass. That question is open and does not depend on anything else in this directory.

## 4. What else this changes

**~~D-2. `docs/rules/language/` gets its missing middle.~~ Withdrawn — the middle is already named, and the repo has
already considered and rejected this exact import.** `docs/rules/language/02-core-calculus.md` §6.1 cites Peyton Jones
1987 §3.1 by name, states that musa is *deliberately not* the enriched-calculus arrangement, and gives the reason: the
elaboration core has lambdas, higher-order functions, products, and folds; the kernel has six forms and no abstraction
at all — so the kernel is not this calculus with the sugar removed, and no simplifying transformation between them
exists. What connects them is **evaluation applied twice** (functions eliminated, then sharing eliminated), with
`Term[ScoreFact]` as the stage boundary.

That is a better architecture than the one D-2 proposed importing, and it is load-bearing: §6.1 notes that totality is
therefore proved *twice, separately*, so a change to either calculus leaves the other's proof intact. §6.2 then applies
Prop 7's test explicitly — nested patterns are refused because the pattern-match compiler (SPJ chs. 4–6) "does not earn
its place" under the §34 rule. **The repo was already running this criterion and already citing this book.** D-3 below
guessed that from prompt 37; §6.1 and §6.2 confirm it outright. What Prop 7 adds is not a new idea to musa but a name
for one it holds, and §6.7 remains the one place the test was applied by assertion instead of by naming the consumer.

**D-3. It explains prompt 37 in retrospect.** `extend` was struck for want of a caller, which is Proposition 7's test
applied correctly — no consumer, no construct. [04](04-candidate-fibred.md) §3 records the cost (L18's proviso) without
disputing the method. So the repo has *already been running* Prop 7; it just has not written it down, and §6.7 is what
happens when the same test is applied by assertion instead of by naming the consumer.

**D-4. It bounds the ambition of this directory.** SPJ's core is small and the enriched calculus is where the mess
lives, deliberately. A motive-hunt that keeps *adding* to the kernel is going the wrong way; the candidates that survive
should mostly change what an object *is* ([04](04-candidate-fibred.md)) or name structure already present
([05](05-candidate-torsor.md)), not add constructors.

## 5. Gaps

**G-1. Prop 7 is a criterion for the term language and says nothing about the motive.** It settles a boundary dispute.
It does not tell us what all six realizations factor through, and must not be mistaken for progress on that question.

**G-2. Time is absent.** The book has no notion of duration, metre, or simultaneity; `⊕` has no analogue in it. It
constrains the *shape* of musa's compiler and contributes nothing to the base ([04](04-candidate-fibred.md)) or the
payload ([05](05-candidate-torsor.md)).

**G-3. Laziness is doing work in SPJ's arguments that musa may not want.** Several of the book's transformation choices
turn on non-strict semantics. The `let`-retention argument (§3.2.4) does not — it is about arity and information loss —
but the surrounding chapters should not be imported wholesale.

## 6. Five-example sieve

| # | Case | Verdict |
| --- | --- | --- |
| 1 | trivial: a construct with one consumer | passes — survives to that consumer, then erased |
| 2 | one operation: `let` in SPJ's own setting | passes — this is the worked case |
| 3 | composition: two passes needing the same structure | passes — erasure point is the later of the two |
| 4 | dependency: the render cache needing `B`'s identity | passes — but as a constraint on prompts 157/158, not a present defect (P-3) |
| 5 | hardest: a construct whose only consumer is *outside* the compiler — a UI that edits patches structurally | **run, and the premise was false** ([07](07-probe-log.md) P-3). The UI displays `StudioFacts`, a projection, and emits `StudioEdit` → `TextEdit`s into the source; `StudioGraphSpec` crosses no crate boundary at all. The editor consumes facts and text, not term structure, so the erasure point does not move. The facts layer is what absorbs the pressure Prop 7 would otherwise put on every boundary — worth noting as a pattern, not just a relief. |

## 7. Verdict

**Not a candidate for the motive at all — and the most immediately useful document in the directory.** It contributes no
account of the base, the payload, or the morphisms. What it contributes is the criterion that decides which constructs
live in the core, and under that criterion the studio question resolves without needing the motive question answered
first.

Case 5 has since been run and closed ([07](07-probe-log.md) P-3): the editor is not a consumer of the studio's term
structure, so the erasure point does not move. What remains is one well-posed question, which needs no further research
to *ask* and none of this directory's other machinery to *answer*: `musa_compiler::StudioSpec` and
`musa_audio::StudioGraphSpec` are two representations of one graph — name the pass that needs the second, or let the
first survive to it.
