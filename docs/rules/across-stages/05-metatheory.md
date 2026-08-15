# What is proved, implemented, and still open

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
| Type inference terminates and returns a principal type in the two-class Hindley–Milner discipline | proved in outline, `06-proof-outline.md` §2 | absent; prompt 127b implements it |
| Accepted source expressions terminate, and a resource failure cannot change an accepted value | proved in outline, `06-proof-outline.md` §2 | implemented for the current monomorphic core; the inferred core is prompt 127b |
| Storable data excludes a source function at every depth, including inside containers | proved by the admission check, `docs/rules/language/02-core-calculus.md` | absent; prompt 127b |
| `follow`, `together`, `map_payloads` preserve bounds and obey their laws, with unequal durations and multiplicity kept | proved in `docs/rules/kernel/03`–`05` and `10` | implemented and tested at the untagged type; coordinate tags are prompt 127c |
| Versioned exact bytes represent event-track semantic equality exactly (I1) | reviewed as part of K₃.3 | implemented by prompt 155a with delimiter and structured-payload tests |
| Every machine has one total deterministic next step, and machines are causal (M1, M2) | proved in `03-machine-calculus.md` §7 | absent; the current audio graph does not implement these semantics |
| Feedback has a first output and reads only stored data (M3) | proved | absent; the current delay path defers cycle inputs to the previous host block |
| Chain and side-by-side laws (M4, M5) | proved | absent |
| Scheduling emits every boundary exactly once and records every decision (M6) | proved | absent; prompt 151 |
| Scheduling preserves simultaneous placement under an occurrence-local policy (M7, M8) | proved, conditionally | absent |
| One audio frame is the reference step; a valid whole-machine batch changes nothing (R1-batch) | contract stated; the composition rule for feedback-free machines is proved | absent; current modulation runs once per host block |
| Complete origin paths compose without losing intermediate anchors; stage composition grafts and is associative | theorem reviewed as part of K₃.3; the graft rule is stated by prompt 127a | only partial provenance exists today |
| Equal complete preparation arguments give equal results, and collision-checked cache hits are sound (R1, C1) | reviewed as part of K₃.3 | full preparation API and cache are not yet implemented |

The proof-review record for the earlier kernel and identity results is in `docs/notes/research/25`, `30`, `35`, `39`,
`42`, and `45`. The record for the current core calculus is
[`docs/notes/research/core-calculus/`](../../notes/research/core-calculus/README.md), in order, ending at its final
review. Both include failed drafts. The failures matter because they exposed missing assumptions now stated in this
specification — most sharply the two feedback counterexamples that killed the single-`Flow` and port-scheduled designs.

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
because they set the size of prompts 127b–127e and 150–153:

- `crates/musa-kernel` already implements the untagged heart of the event track — exact rational duration, finite
  occurrences, succession by shifting, simultaneity by maximum and multiset union, payload mapping. What it lacks is the
  coordinate tag and the renamed surface.
- `crates/musa-audio`'s per-sample DSP units are close to registered primitives already; the gap is the registry and the
  reference step, not the arithmetic.
- `crates/musa-audio`'s plan does **not** implement the machine semantics. It defers cycle inputs at delay nodes to the
  previous host block, and its modulation path runs once per block, so host block size can affect meaning. That is the
  precise defect `constitution.md` §4 now forbids.

A green test suite for today's code says nothing about a representation that does not yet exist. The code map records
that distinction row by row.

## 4. Evidence required from implementations

Each implementation step must test the premise on which its proof relies:

- byte encoders test empty strings, delimiters, newlines, multiplicity, coordinate tags, version changes, and migration;
- registries reject one id and version paired with two exact descriptors;
- inference tests principal types, and compile-fail tests cover a function hidden in a list, constructor, or abstract
  value; an incomplete call; a recursive term; and a non-exhaustive match;
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
