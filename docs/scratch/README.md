# Scratch: the search for the musical motive

**Status: scratch. Governs nothing. Not even research-grade yet.** `docs/kernel/` is the governing kernel specification,
`docs/course-correction.md` the governing ontology, `docs/core-boundary.md` the governing boundary decision.
`docs/kernel-hypothesis/` is research that governs nothing and is *upstream* of this directory. This directory is
downstream of all of them and is a working notebook: hard copies of thinking, kept so they can be iterated on and so
that a claim which gets refuted stays visible next to its refutation.

## Why this directory exists

Prompt 126 decided that the core is a calculus of occurrences of any canonical payload, that signals stay out, and — in
§6.7 — that the studio gets no calculus of any kind. The third of those does not follow from the second. §5 proves a
*signal* cannot be an occurrence payload: a sample stream is coinductive, the kernel is inductive and total, and a
stream has no extent. But `StudioGraphSpec` is not a stream. It is a finite, first-order, statically typed description
*of* a stream, and nothing in §5 touches it. The document gave the studio's `none` the same verdict word it gave the
value layer's `none`, for the opposite reason: the value layer has another core underneath
(`docs/language/02-core-calculus.md`), and the studio has none.

That is a local defect with a local fix. But asking "what calculus goes under the studio?" turned out to be the wrong
size of question, because the honest answer to it — *the same one that goes under everything else, if we knew what that
was* — is a question nobody here has answered. Hence the motive.

**That opening question is now answered, and not by the motive.** [08](08-candidate-enriched.md) resolves it from Peyton
Jones's `let`-retention argument: a construct belongs in the core when erasing it would destroy information a later pass
needs, and erasure is staged per construct. The studio needs neither its own calculus nor a merger with the temporal one
— it needs its description to survive, with an equality, as far as the render cache that R1 already presupposes. The
motive question outlived the question that prompted it, which is the usual way of these things.

## The question

Grothendieck's motives are the conjectural universal object through which every cohomology theory factors: Betti, de
Rham, and ℓ-adic realizations all see the same underlying thing in different coefficients. The question this directory
asks is the same one for music, and it is not an analogy but a method — the method in
`~/Code/papers/category-theory/50-examples-for-the-motive/text.md`, which asks for **the smallest description all
realizations agree on. The information that survives every projection.**

Musa's realizations are engraving (MEI, LilyPond, MusicXML), performance, sound, analysis, MIDI, and equational
reasoning. What is the object they all factor through?

The pun is worth stating once and then dropping: a motif is a piece of music's smallest recurring generating idea, and a
motive is a cohomology theory's. `motif` is already a keyword in the language. Nothing follows from the pun; it is not
evidence.

## Reading order

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

## Standard of evidence

Inherited unchanged from `docs/kernel-hypothesis/README.md`, because it is the right standard and because these
documents are meant to be read next to those:

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

- **Converges with `docs/kernel-hypothesis/`**, arrived at independently from OMT before that directory was read: the
  group-action reading of Tₙ/Iₙ (its Atom 5), normal order and prime form as canonicalization-then-orbit (its Atom 6),
  and the metrical-layer reading of hypermeter and metrical dissonance (its Atom 4). Where this directory and that one
  agree, that one has priority and better evidence.
- **New here:** the polarity/CBPV reading ([02](02-candidate-polarity.md)), which is absent there and which answers the
  studio question that started this; the tropical-semiring grading ([03](03-candidate-graded.md)); the observation that
  **L18's side condition is itself the evidence for Atom 4** ([04](04-candidate-fibred.md)), which is an argument for
  that atom from inside the existing kernel's own law list rather than from the repertoire; the claim that Atoms 5 and 6
  together *are* the motive rather than being two more atoms ([05](05-candidate-torsor.md)); and Proposition 7's erasure
  criterion ([08](08-candidate-enriched.md)), which is the only thing here that is ready to change code.
- **Refuted from inside:** [07](07-probe-log.md) P-2 killed [05](05-candidate-torsor.md)'s own D-2 by finding that the
  quotient tower is already built in `crates/musa-compiler/src/scale.rs`, with a test asserting that a square in it
  commutes. Kept because a directory that only records its confirmations is not evidence of anything.
