---
id: 129
slug: dependent-core-spec
status: pending
depends_on: [128]
phase: 3
---

# Specify the Dependent Core

## Task

Rewrite `docs/rules/language/02-core-calculus.md` as the specification of the core prompt 128 admitted: a small
dependently typed calculus with universes, Π, dependent records, inductive families, an identity type, definitional
equality by normalization-by-evaluation, bidirectional elaboration with metavariables, and a checked well-founded
termination rule. Every metatheoretic obligation the old core carried is either re-derived against the new core or
recorded as superseded with a reason.

## Read

- `docs/rules/language/02-core-calculus.md` in full — this prompt rewrites it, and the parts that survive have to
  survive *as written*, not as a paraphrase. §1.1's storable-data predicate, §4's resource acceptance and
  budget-independence law, §5.7's track-construction safety, §5.9's expansion phase and its law 11, §6.1's two-stage
  boundary, and §7's provenance are the load-bearing paragraphs.
- `docs/notes/research/language-design-closure/42-dependent-core-decision.md` (written by prompt 128) — the decision
  this prompt writes down. Where the record and this specification disagree, the record is the argument and this
  document is the contract; a disagreement is a defect in one of them.
- `docs/rules/constitution.md` §9 as amended, and `docs/rules/across-stages/05-metatheory.md` §1 — the principal-type
  claim §1.1 cites is one of the claims that changes.
- `docs/rules/kernel/12-payload-admission.md` — storable data is a kernel-facing predicate, not a language-internal
  convenience, so whatever replaces the `d` variable class still has to answer the kernel.
- Peyton Jones ch. 8 and ch. 9 for what a type checker is obliged to do and how it is written down, and ch. 5 for the
  semantics of pattern matching — §6.2's flat-pattern rule is the one this prompt overturns, and ch. 5 is where the
  case-tree compilation that replaces it comes from.
- `docs/notes/research/language-design-closure/39-totality-and-structural-abstraction.md` §11.2 — the cost list, item by
  item, is the outline of what this document must actually contain.

## Design

**Universes.** A predicative hierarchy `Type l` with `Type l : Type (l+1)` and no `Type : Type`. No cumulativity in the
core: cumulativity buys convenience at the cost of a subtyping relation inside conversion, and conversion is about to
carry dependent match and records already. Levels are a core sort with `0`, `succ`, and `max`; the elaborator solves
level metavariables against that constraint set and the surface never writes a level. State the consistency consequence
plainly — a hierarchy is what keeps the checker from proving everything, and a total language whose type theory is
inconsistent is not total in any useful sense.

**Π, and where plicity lives.** The core has one function type. Implicit arguments are an *elaboration* notion: the
surface marks a binder implicit, the elaborator inserts a metavariable at each use, and what reaches the core is an
ordinary Π. Nothing downstream — traits, `Syntax<Cat>`, `Vec A n` — needs a second binder form, and keeping plicity out
of the core keeps conversion from having to know about it.

**Dependent records are primitive, not Σ sugar.** Named fields with η. Two reasons, both concrete: trait dictionaries
are records, and coherence arguments are much easier to state when two dictionaries for the same instance are
convertible by η rather than by a chain of projections; and a Σ-encoded record loses its field names, so every error
message about a missing field would name `fst`/`snd` instead. Σ remains derivable and is not separately primitive.

**Inductive families.** Parameters and indices are distinguished. Strict positivity is checked on the declaration group,
including through the function-argument positions the old §5.6 already refused. The core's elimination form is the
generated dependent recursor; surface `match` elaborates through a case tree to recursors, which is where coverage is
decided. **This replaces §6.2.** Flat patterns were right for a one-level evaluator and are wrong here: a dependent
match that cannot look through two constructors cannot express the thing indices exist for, and forcing the author to
write the nesting by hand is precisely the `staff.musa` failure one level down. Say so in the rewritten section rather
than deleting the old rule silently.

**Identity, and the K decision.** `Id A x y` with `refl` and the dependent eliminator `J`. Uniqueness of identity proofs
is **admitted**: this is a set-level theory with no higher-inductive or univalent ambitions, and index unification for
families like `Vec A n` wants constructor injectivity together with K. Admitting K is a real commitment — write it down
as one, with what it forecloses, so prompt 132 can falsify it on a program rather than discover it during prompt 135.

**Conversion by NbE.** Definitional equality is β, η at Π and at records, δ for definitions, and ι on recursors applied
to constructors. It is decided by evaluating both sides to a semantic value and quoting back to a normal form.
Decidability rests on totality, and totality now rests on the termination checker, so the dependency runs
`termination → normalization → decidable conversion → type checking`. State that chain explicitly: it is the reason
prompt 128 kept totality rather than a matter of taste.

**Bidirectional elaboration.** Check and infer modes, the two rules that switch between them, where metavariables are
created and where they are solved, pattern-fragment (Miller) unification and the exact restriction that makes it
decidable and most-general, and postponement for constraints blocked on an unsolved meta. Public signatures are still
annotated; what changed is that a signature may mention a value.

**Termination is a checked measure.** Every recursive definition presents a measure into a well-founded order, and
structural decrease is the special case where the measure is the subterm order and the elaborator supplies it. Nothing
is opaque to the checker and there is no `partial`. The soundness obligation is stated as an obligation: an accepted
definition denotes a total function.

**What survives, restated rather than assumed.**

- **§1.1's storable-data predicate survives; its `d` variable class does not.** In a language with traits, "this type
  variable ranges only over storable data" is a constraint — `Storable A` — and writing it that way is strictly better:
  it composes, it appears in signatures the author already reads, and its failures are ordinary instance errors. Its
  instances are *generated from the declaration group and never written by hand*, so the predicate remains a structural
  fact about a type rather than something a user can assert. The list of what only storable data may be — payload, port
  type, feedback value, primitive configuration, foreign argument — is unchanged. `SyntaxStep` stays excluded, and
  `Syntax<Cat>` inherits the same treatment when prompt 138 defines it.
- **§4's budget rule is restated for a checker that evaluates.** Conversion can now diverge in cost, not in principle,
  so a budget bounds type checking too. The law becomes: a budget may end a check or an evaluation with a stated
  exhaustion diagnostic, and may never silently accept, silently reject, or change an accepted program's value.
  Exhaustion is a third outcome, named as one — pretending it is a rejection would make acceptance depend on a resource
  limit.
- **§5.7 (track construction) and §5.9 (the expansion phase, including law 11) are obligations to re-derive**, not
  assumptions to carry. Prompt 148 owes the proofs; this document owes their statements against the new core.
- **§6.1's two-stage boundary is unchanged.** This language and the event-track term calculus stay two stages.
- **§7's provenance grows.** Every core term records the surface node it was elaborated from, because a dependent
  checker reports failures in terms of normal forms the author never wrote, and because prompt 131's quotation needs a
  term's origin to be a fact rather than a reconstruction.

**§5's obligation matrix is rewritten, not extended.** The new list: NbE soundness and completeness, decidability of
conversion, type preservation, canonicity for the closed storable-data types, strong normalization, strict positivity
implying consistency, coverage completeness, termination soundness, and the two carried-forward obligations above. Each
entry names what discharges it and which prompt owes it — 133 through 137 for the mechanism, 148 for the audit.

## Target

- `docs/rules/language/02-core-calculus.md`, rewritten: §1 syntax (universes, Π, records, families, `Id`), §1.1 storable
  data as the `Storable` constraint with its generated instances, §2 static semantics as bidirectional elaboration with
  metavariables, §3 dynamic semantics as NbE with the conversion algorithm, §4 resource acceptance with the
  three-outcome budget law, §5 the rewritten obligation matrix with per-prompt ownership, §6 the implementation boundary
  with §6.2 replaced by case-tree compilation and the replacement argued, §7 provenance on elaborated terms.
- `docs/rules/language/citations.md`: every new theoretical claim — NbE, pattern unification, positivity, coverage,
  well-founded recursion, K — with the chapter or paper it comes from. A claim without a citation is the defect this
  file exists to prevent.
- `docs/rules/language/README.md`'s document map row for `02-core-calculus.md`, if its one-line contract no longer
  describes the file.
- No other `docs/rules/language/` file changes. 130 and 131 own the surface, traits, and quotation.

## Check

```sh
python3 scripts/renumber-prompts.py audit
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
/Users/jcreinhold/.cargo/bin/mdwright check docs/rules docs/plan
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
git diff --check
```

Commit as `Specify the dependent core`.

## Stop

- No code, no grammar, no fixture.
- No surface syntax. This document says what the core *is*; a reader who wants to know what an author types is reading
  the wrong file, and 130 is where that is answered.
- No traits, no `Syntax<Cat>`, no collection library. They are library code over this core and belong to 130, 131, and
  141 — a core document that specifies them has admitted they are not library code.
- No cumulativity, no universe polymorphism beyond level metavariables, no `partial`, no general recursion, no CBPV, no
  coinduction. Each is a separate amendment.
- No proof. Stating an obligation is this prompt's job; discharging it is prompt 148's.
