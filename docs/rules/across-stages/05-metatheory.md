# What is proved, implemented, and still open

**Status: governing.** What is proved, what the code implements, and what is still open — three claims kept apart.

This chapter keeps three claims separate:

- a mathematical rule may be correct;
- the Rust implementation may or may not implement it; and
- a safe mechanism may still be a poor model of a musical practice.

The review status column below refers to the paper review of the core calculus
(`docs/notes/research/core-calculus/17-final-review.md`), which found no fatal, high, or medium error in the frozen
definitions or conditional theorems. That verdict is about the paper calculus. It says nothing about the compiler or the
audio engine, and the implementation column is deliberately blunt about the gap.

## 1. Reviewed results

| Result | Review status | Implementation status |
| --- | --- | --- |
| ~~Type inference terminates and returns a principal type in the two-class Hindley–Milner discipline~~ — **superseded** by the constitution's prompt-128 amendment, which replaced principal inference with bidirectional elaboration | the outline proof in `../../notes/research/core-calculus/06-proof-outline.md` §2 stands for the discipline it was about; it is no longer a result about Musa | replaced by prompt 169's K1–K20 obligation matrix: decidable conversion and NbE soundness/completeness are implemented and tested |
| Accepted source expressions terminate, and a resource failure cannot change an accepted value | proved in outline, `../../notes/research/core-calculus/06-proof-outline.md` §2 | implemented by the dependent core, its deterministic resource meter, and publish-after-success boundary |
| Storable data excludes a source function at every depth, including inside containers | proved by the admission check, `docs/rules/language/02-core-calculus.md` | implemented as a generated structural constraint, checked again at every payload boundary |
| `follow`, `together`, `map_payloads` preserve bounds and obey their laws, with unequal durations and multiplicity kept | proved in `03-denotational-semantics.md`–`05-normalization.md` and `10-term-calculus.md` of `docs/rules/events/` | implemented and tested at coordinate-indexed `EventTrack<C, A>` |
| Versioned exact bytes represent event-track semantic equality exactly (I1) | reviewed under the K₃.3 integration closure, in this directory's history | implemented by prompt 176a with delimiter and structured-payload tests |
| Every machine has one total deterministic next step, and machines are causal (M1, M2) | proved in `03-machine-calculus.md` §7 | absent; the current audio graph does not implement these semantics |
| Feedback has a first output and reads only stored data (M3) | proved | absent; the current delay path defers cycle inputs to the previous host block |
| Chain and side-by-side laws (M4, M5) | proved | absent |
| Scheduling emits every boundary exactly once and records every decision (M6) | proved | absent; prompt 172 |
| Scheduling preserves simultaneous placement under an occurrence-local policy (M7, M8) | proved, conditionally | absent |
| One audio frame is the reference step; a valid whole-machine batch changes nothing (R1-batch) | contract stated; the composition rule for feedback-free machines is proved | absent; current modulation runs once per host block |
| Complete origin paths compose without losing intermediate anchors; stage composition grafts and is associative | theorem reviewed under the K₃.3 integration closure; the graft rule is stated by prompt 127a | source and adapter provenance are implemented; runtime-stage composition remains prompt 174 work |
| Equal complete preparation arguments give equal results, and collision-checked cache hits are sound (R1, C1) | reviewed under the K₃.3 integration closure, in this directory's history | full preparation API and cache are not yet implemented |

The record for the current core calculus is
[`docs/notes/research/core-calculus/`](../../notes/research/core-calculus/README.md). The earlier event-track and
identity results came out of the K₁, K₂, and K₃ lines, each closed by an adversarial proof review; those reviews and the
candidates they reviewed are in the history of `docs/notes/research/`, deleted once the results they established were
written into this specification. The failed drafts mattered because they exposed missing assumptions now stated here —
most sharply the two feedback counterexamples that killed the single-`Flow` and port-scheduled designs.

## 1a. The core calculus: what is argued and what is tested

`../language/02-core-calculus.md` states a dependency chain — `termination ⟹ normalization ⟹ decidable conversion ⟹
decidable type checking` — and this row exists so that nobody reads it as a claim musa has proved. **Musa does not ship
a formalized metatheory of its core calculus, and this specification does not pretend to one.** What each link rests on
is stated separately, because the three kinds of support are not interchangeable.

| Link in the chain | What supports it here |
| --- | --- |
| Every accepted definition terminates | **Argued**, from structural descent over the case tree (`02-core-calculus.md` §2.4), and **tested** by compile-fail fixtures for calls that do not descend. There is no mechanized proof, and the case-tree substrate is new work at prompt 155 rather than inherited |
| Normalization by evaluation is sound and complete for the core | **Borrowed**, from Abel and Sattler and from Coquand's algorithm (`../language/citations.md` §13.1). The theorem is about the calculus in those papers; that musa's implementation is an instance of it is argued, not proved |
| Conversion is decidable | **Follows** from the two above, and is **tested** the only way an implementation can be: the resource meter turns a non-answer into a stated refusal (`02-core-calculus.md` §4) rather than a hang, so a divergence would surface as an exhaustion rather than as silence |
| Type checking is decidable | **Follows**, with the same standing. Prompt 169's conformance suite is where the claim is exercised over the corpus rather than asserted |
| An elaborated term is well typed | **Tested, and this is the strongest of the four**: the kernel re-checker of prompts 149 and 158 re-checks accepted terms from scratch, so the claim is checked on every program the suite runs rather than argued about |

The honest summary is that musa buys its confidence from a re-checker and a corpus, not from a proof. That is a
defensible position for a music compiler and an indefensible one to be vague about, which is why the row is here.

## 2. Claims this specification does not make

Nothing here proves:

- that one object captures every musical practice;
- that one pitch, metre, chord, key, or function system fits all music;
- that harmonic function is the same as scale degree;
- that a musical motif is a split idempotent;
- that hashes never collide;
- that equal event tracks have equal origin histories;
- that equal machines in the behavioural sense are decidable, or that structural equality implies behavioural equality;
- that scheduling preserves succession under a nonlinear time map or non-additive rounding;
- that any particular optimized batch method satisfies its contract — that is per-primitive evidence;
- that totality implies a real-time deadline;
- that equal prepared machines produce bit-identical output on arbitrary devices; or
- that today’s caller-buffer-based feedback obeys the one-frame rule.

## 3. The implementation is behind the specification, and by how much

The core-calculus review audited the repository at the time it was written and found three things worth repeating,
because they set the size of prompts 127b–127e and 171–174:

- `crates/musa-events` implements the coordinate-indexed event track — exact rational duration, finite occurrences,
  succession by shifting, simultaneity by maximum and multiset union, and payload mapping.
- `crates/musa-dsp`'s per-sample DSP units are close to registered primitives already; the gap is the registry and the
  reference step, not the arithmetic.
- `crates/musa-dsp`'s plan does **not** implement the machine semantics. It defers cycle inputs at delay nodes to the
  previous host block, and its modulation path runs once per block, so host block size can affect meaning. That is the
  precise defect `constitution.md` §4 now forbids.

A green test suite for today's code says nothing about a representation that does not yet exist. The code map records
that distinction row by row.

## 4. Evidence required from implementations

Each implementation step must test the premise on which its proof relies:

- byte encoders test empty strings, delimiters, newlines, multiplicity, coordinate tags, version changes, and migration;
- registries reject one id and version paired with two exact descriptors;
- elaboration tests that checking and inference agree where both apply and that a type parameter no written argument
  determines is reported at the call that left it undetermined, and compile-fail tests cover a function hidden in a
  list, constructor, or abstract value; an incomplete call; a recursive call that does not descend structurally; and a
  non-exhaustive match;
- machines test every constructor, the first feedback output, Boolean negation through a stored delay, causality, and
  the whole-node scheduling counterexample the old graph rules could not handle;
- batching tests every partition of the same requested frames against a plain structural interpreter;
- schedulers test exact-once boundaries, monotonicity, half-open spans, collapsed and point occurrences, handle
  renaming, the occurrence-local `together` law, additive `follow`, and the fixed finished state;
- caches inject a deliberate hash collision and compare complete arguments;
- preparation tests vary each option independently; and
- origin tests retain generation roots, sites, and intermediate anchors, and check graft coverage and associativity.

## 5. What would reopen a decision

The core calculus is closed to new forms. A proposal to add one must record the smallest failing term — a concrete
program the current rules cannot express or cannot make safe — and must show the same need in two materially different
musical uses. An analogy is not evidence, and neither is a feature list from another language.

The two triggers that reopen `constitution.md` §4 are named there and nowhere else.
