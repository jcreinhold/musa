# Scratch: the search for the musical motive

**Status: research. Governs nothing.** `docs/rules/` holds the decisions this work fed into, `docs/rules/events/` the
governing kernel specification, `docs/rules/across-stages/` the cross-stage rules distilled from files 19–57 below.
[`kernel-hypothesis/`](kernel-hypothesis/README.md) is the earlier research line, *upstream* of the numbered files here.
This directory is downstream of all of them and is a working notebook: hard copies of thinking, kept so they can be
iterated on and so that a claim which gets refuted stays visible next to its refutation.

## Why this directory exists

Prompt 126 decided that the core is a calculus of occurrences of any canonical payload, that signals stay out, and — in
§6.7 — that the studio gets no calculus of any kind. The third of those does not follow from the second. §5 proves a
*signal* cannot be an occurrence payload: a sample stream is coinductive, the kernel is inductive and total, and a
stream has no extent. But `StudioGraphSpec` is not a stream. It is a finite, first-order, statically typed description
*of* a stream, and nothing in §5 touches it. The document gave the studio's `none` the same verdict word it gave the
value layer's `none`, for the opposite reason: the value layer has another core underneath
(`docs/rules/language/02-core-calculus.md`), and the studio has none.

That is a local defect with a local fix. But asking "what calculus goes under the studio?" turned out to be the wrong
size of question, because the honest answer to it — *the same one that goes under everything else, if we knew what that
was* — is a question nobody here has answered. Hence the motive.

**That opening question is now answered, and not by the motive.** [08](08-candidate-enriched.md) resolves it from Peyton
Jones's `let`-retention argument: a construct belongs in the core when erasing it would destroy information a later pass
needs, and erasure is staged per construct. The studio needs neither its own calculus nor a merger with the temporal one
— it needs its description to survive, with an equality, as far as the render cache that R1 already presupposes. The
motive question outlived the question that prompted it, which is the usual way of these things.

**Reopened in [11](11-candidate-relational-presentation.md).** The erasure criterion still answers the cache question,
but it does not answer how a score, performance profile, studio binding, signal graph, and sound can form one coherent
artifact. E8's consumer-count argument threw away the links among realizations and then concluded that nothing remained.
Files 11–17 retain that refutation and search for a compositional, non-identifying replacement.

## The question

Grothendieck's motives are the conjectural universal object through which every cohomology theory factors: Betti, de
Rham, and ℓ-adic realizations all see the same underlying thing in different coefficients. The question this directory
asks is the same one for music, and it is not an analogy but a method — the method in
`~/Code/papers/category-theory/50-examples-for-the-motive/text.md`, which asks for **the smallest description all
realizations agree on. The information that survives every projection.**

[11](11-candidate-relational-presentation.md) proves that this formulation is too forgetful: separate projections do not
determine the relation among them. The revised question is therefore: **what smallest compositional presentation retains
each realization's native structure and the coherent links among them without identifying their universes?**

Musa's realizations are engraving (MEI, LilyPond, MusicXML), performance, sound, analysis, MIDI, and equational
reasoning. What is the object they all factor through?

The pun is worth stating once and then dropping: a motif is a piece of music's smallest recurring generating idea, and a
motive is a cohomology theory's. `motif` is already a keyword in the language. Nothing follows from the pun; it is not
evidence.

## Reading order

Files 00–18 are the motive search, in sequence. Files 19 onward are four later lines of inquiry, summarized after the
table — they are the ones `docs/rules/across-stages/` and `docs/rules/` were actually distilled from, and they are
appended to as the work continues, so they are indexed by line rather than one row per file.

| File | What it holds |
| --- | --- |
| [00-the-motive-question.md](00-the-motive-question.md) | The question stated precisely enough to be answered wrongly, and the three tests any answer must pass |
| [01-realizations-and-residue.md](01-realizations-and-residue.md) | The empirical core: musical examples run through every realization, and what survives all of them |
| [02-candidate-polarity.md](02-candidate-polarity.md) | Candidate P — call-by-push-value: the score is a value, the signal is a computation |
| [03-candidate-graded.md](03-candidate-graded.md) | Candidate G — duration as a grade in the tropical semiring |
| [04-candidate-fibred.md](04-candidate-fibred.md) | Candidate F — timelines fibred over metrical contexts; L18's side condition as a symptom |
| [05-candidate-torsor.md](05-candidate-torsor.md) | Candidate T — the payload torsor and its quotient tower, which is where the motive most likely is |
| [06-sieve.md](06-sieve.md) | All candidates through the five-example sieve; what fell out, what needed side conditions |
| [07-probe-log.md](07-probe-log.md) | Probes run against the codebase, with the claims they damaged — including the one that refuted T's own prediction |
| [08-candidate-enriched.md](08-candidate-enriched.md) | Candidate E — SPJ's enriched core and staged erasure; not a motive, but the criterion that settles the studio question |
| [09-the-proposal.md](09-the-proposal.md) | **The synthesis** — indexed call-by-push-value over two index domains: typing rules, operational semantics, how dependent it needs to be, and the work order |
| [10-external-review.md](10-external-review.md) | External review of 00–09 — claims that failed, claims that survived, and the decision not to execute 09 step 1 |
| [11-candidate-relational-presentation.md](11-candidate-relational-presentation.md) | Candidate R — reopens the common-residue premise; a useful relational coordination substrate which fails the emergence test on its own |
| [12-candidate-process-worlds.md](12-candidate-process-worlds.md) | Candidate S — native worlds as symmetric monoidal process theories; chords and processor graphs from parallel and serial composition |
| [13-candidate-modules.md](13-candidate-modules.md) | Candidate M — modules/profunctors as non-identifying inter-world links, with functors as the representable special case |
| [14-candidate-temporal-locality.md](14-candidate-temporal-locality.md) | Candidate L — temporal worlds vary over observation regions; restriction, support, gluing, and multiple clocks replace equal active extents |
| [15-candidate-frames.md](15-candidate-frames.md) | Candidate F₂ — context-indexed worlds; pitch collections, transposition, and conditional key families from free constructions and frame actions |
| [16-candidate-fibred-equipment.md](16-candidate-fibred-equipment.md) | Candidate E₂ — synthesis as context-indexed monoidal process worlds joined by modules, with a small dependently sorted diagram language |
| [17-sieve-and-prototype.md](17-sieve-and-prototype.md) | Comparative sieve, the smallest surviving conjecture, and a fixed-signature prototype plan designed to falsify it |
| [18-minimal-recommendation.md](18-minimal-recommendation.md) | End of the motive search — begin with one multi-sorted compositional syntax; worlds, links, dependency, choice, and guarded computation must earn admission through failed examples |

### After the motive search

Four lines, each following the same shape: a candidate, its metatheory, then an adversarial proof review that is kept
whether or not it was kind to the candidate.

| Files | Line | What came of it |
| --- | --- | --- |
| [19](19-domain-obligations.md) | **Domain obligations** — what a second kernel would have to earn before it exists | The standing precondition on all three lines below |
| [20](20-candidate-staged-algebras.md)–[27](27-lineage-is-the-link.md) | **K₁** — a total metalanguage with staged deep algebras, its semantics, encodings, five-case sieve, metatheory, and proof review; then candidates E₃ and H, and the turn to lineage | K₁ fell; [27](27-lineage-is-the-link.md) is where "lineage is the link" replaced the search for a common residue |
| [28](28-candidate-k2.md)–[33](33-studio-feedback-semantics.md) | **K₂** — the smallest coherent core, its metatheory and proof review; theory modules as candidate T₂; the derivation diagram as the coherent object; studio feedback | [32](32-the-coherent-object-is-the-diagram.md) is the load-bearing result: the coherent object is the derivation diagram, not a musical motive. It is what `docs/rules/across-stages/02-derivation-diagrams.md` became |
| [34](34-candidate-k3-stratified-kernels.md)–[45](45-proof-review-k3.3.md) | **K₃** — three small kernels and one coherence discipline, closed in three passes (K₃.1 framing, K₃.2 semantic framing and typed lineage paths, K₃.3 integration), each with its proof review; plus [40](40-canonical-framing-bug.md), a governing bug, and [43](43-what-the-iut-analogy-earns.md) | The reviewed source of `docs/rules/across-stages/` and of the identity rules in `docs/rules/`. `docs/rules/across-stages/05-metatheory.md` §1 cites 25/30/35/39/42/45 as the proof-review record |
| [36](36-theory-module-paper-prototypes.md)–[37](37-calculus-of-theory-modules.md), [46](46-proof-review-t2a.md)–[66](66-proof-review-t2j.md) | **T₂** — theory modules, from paper prototypes and the T₂a nominal calculus through source closure, packages, and caching | The source-calculus results remain live. The package and cache line stops at [66](66-proof-review-t2j.md): its graph repairs passed, but it still used hashes as identity and confused package dependencies with module imports. The next work returns to the language and musical cases instead of opening T₂k |

### Package-design stop

The package and cache line has consumed enough work. Review [66](66-proof-review-t2j.md) found two exact errors, but
neither blocks the source language:

- a package dependency edge is not a source module import; and
- exact source bytes decide equality, while a hash only finds possible matches.

Prompt 162 can apply those rules when Musa implements exact Git source packages. Until then, language research assumes
one finite resolved package graph per build and fresh private type identities inside that build. Registries, version
solving, persistent compiled identities, and compiled-artifact caches are outside the current design closure. A measured
need in a working package system may reopen them later.

### Descriptions and music as played

Later language work exposed a more basic problem: Musa's build order had been mistaken for a fact about music.
[`descriptions-and-music/`](descriptions-and-music/README.md) starts again from three documented examples. Its first
idea is that notation, studio setups, and audio references can all describe the same musical event. A performance is one
event that matches those descriptions, with a record explaining the match.

That idea remains useful for comparison, but it failed as an execution core: it does not build passages, schedule
events, connect instruments, or run audio. The refutation remains in the next research line.

### Core calculus

[`core-calculus/`](core-calculus/README.md) asks the smaller execution question: what finite values must a total source
program build so that written music and audio can meet without being called the same thing?

It rejects one universal flow and selects three parts:

- a small pure language with inferred types;
- `EventTrack<C,A>` for finitely many events placed in a finite length; and
- `Machine<K,A,B>` for a finite deterministic machine that may keep taking steps.

The scheduler is the checked connection. It turns a finite event track into a machine that emits event batches one audio
frame at a time. The line contains five audits and their repairs; the final review accepts the paper calculus under its
stated primitive, scheduling, and batching contracts. It does not yet change Musa's rules or code.

## Standard of evidence

Inherited unchanged from [`kernel-hypothesis/README.md`](kernel-hypothesis/README.md), because it is the right standard
and because these documents are meant to be read next to those:

- **Cited** — a claim about music theory, with an Open Music Theory chapter given by filename.
- **Derived** — a mathematical consequence of stated definitions, as a numbered proposition.
- **Judged** — a design choice the evidence leaves open, saying what it chooses against.

Two further rules are imported from `~/Code/proofs/.claude/skills/rethink-math/`, because this directory is exactly the
kind of work they govern:

- **Precise enough to be wrong.** A candidate that no example could falsify is not written down here.
- **Report what fell out, and what required side conditions.** A side condition is the sign that the definition is at
  the wrong level. This is the sharpest tool in the directory and [04](04-candidate-fibred.md) is built on it.

## What is new here versus what converges

Honesty about provenance, because most of this directory's value is in the two or three genuinely new claims:

- **Converges with [`kernel-hypothesis/`](kernel-hypothesis/README.md)**, arrived at independently from OMT before that
  directory was read: the group-action reading of Tₙ/Iₙ (its Atom 5), normal order and prime form as
  canonicalization-then-orbit (its Atom 6), and the metrical-layer reading of hypermeter and metrical dissonance (its
  Atom 4). Where this directory and that one agree, that one has priority and better evidence.
- **New here:** the polarity/CBPV reading ([02](02-candidate-polarity.md)), which is absent there and which answers the
  studio question that started this; the tropical-semiring grading ([03](03-candidate-graded.md)); the observation that
  **L18's side condition is itself the evidence for Atom 4** ([04](04-candidate-fibred.md)), which is an argument for
  that atom from inside the existing kernel's own law list rather than from the repertoire; the claim that Atoms 5 and 6
  together *are* the motive rather than being two more atoms ([05](05-candidate-torsor.md)); and Proposition 7's erasure
  criterion ([08](08-candidate-enriched.md)), which is the only thing here that is ready to change code.
- **Refuted from inside:** [07](07-probe-log.md) P-2 killed [05](05-candidate-torsor.md)'s own D-2 by finding that the
  quotient tower is already built in `crates/musa-compiler/src/scale.rs`, with a test asserting that a square in it
  commutes. Kept because a directory that only records its confirmations is not evidence of anything.
