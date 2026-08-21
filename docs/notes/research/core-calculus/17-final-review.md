# Final review of the core calculus

**Reviewed files:**

- `05-selected-calculus.md` at SHA-256 `14a60536f1325598b225140ba0130f7310e57948267e02ae25a349e4b6ee54aa`;
- `06-proof-outline.md` at SHA-256 `7d52fd57637f02f2186cfd6a480a061412be36aeae824967d8ea5ca26cf18031`.

## Verdict

**Correct under the stated contracts.** This verdict concerns the paper calculus, not the current compiler or audio
engine. I found no fatal, high, or medium error in the frozen definitions or conditional theorems.

The design is small enough to recommend:

1. a pure total language evaluates finite source programs;
2. `EventTrack<C,A>` stores finitely many placed events in a finite length; and
3. `Machine<K,A,B>` stores a finite deterministic machine that can run for any number of steps.

Audio is part of the core through `Machine`, its step rule, its preparation check, and the checked scheduler from event
tracks. Audio is not forced into the shape of a finite score.

## What was verified

### The source language

The displayed source language has no recursion or partial calls. Hindley–Milner inference needs only one extra static
mark: whether a type is ordinary source value or storable data. That mark blocks a source function, even one hidden in a
list or constructor, from entering a track, machine, primitive call, or feedback value.

Under A1 and A3, the standard termination proof applies. Foreign source primitives remain trusted: each must be total,
first order, and data-only. This is an explicit boundary, not a theorem about arbitrary host code.

The resource error also has a precise place. `run`, `done`, and `failed` retain the source result type. A fixed integer
charge and a left-to-right next step leave no choice between success and failure.

### Finite event tracks

The `follow` and `together` rules preserve occurrence bounds. `follow` adds lengths and shifts the second track.
`together` takes the greater length and keeps both multisets. It does not pad a shorter voice and does not turn absence
into a rest.

The algebra follows from exact rational addition, maximum, translation, and multiset union. Duplicate events remain
duplicates. Half-open positive spans make the end-frame rule exact, while point events remain possible.

### Running machines

Every machine form has a concrete state, start state, and next-step rule. `connect` runs its left machine and then its
right machine in the same step. `beside` keeps two paths apart. An audio mixer is therefore an explicit unit, not a
hidden meaning of parallel placement.

Feedback reads a stored value from the prior step and takes an explicit initial value. The Boolean-negation and
missing-first-value counterexamples from the rejected drafts no longer apply.

Primitive ids and versions select one exact build-local registration. Structural equality therefore has one meaning
inside a build. Behavioral equality remains a mathematical relation; the type checker and cache need not decide it.

### The track-to-audio boundary

Scheduling is a finite checked algorithm. It maps only the input track's finite boundary set, records rounding and
collision choices, rejects order reversal, and creates a bounded table. Its running source reaches a fixed `Finished`
state after the last batch, so it does not grow a counter forever.

Private event handles remain distinct when two scheduled sources are merged. The merger tags left and right handles
before sorting. Exact duplicate events receive equal time decisions but different handles.

The overlay theorem now has the premises it needs: all three schedules succeed, the policy treats each occurrence on its
own, equal copies get equal decisions, and merging only orders messages. Under those premises, scheduling preserves
simultaneous placement up to a consistent renaming of private handles. A handle-respecting instrument then produces the
same audio history.

There is no false general theorem for succession. Nonlinear time maps and frame rounding can break it. The document
claims preservation only when time and frame conversion are additive.

### Audio steps and batches

One audio step means one sample frame. That fixes the reference sound. A host may process a block only when its whole
batch operation equals repeated frame steps.

The earlier false feedback proof is gone. Child batches compose through feedback-free chain and side-by-side wiring. A
feedback machine stays frame by frame unless its own whole-machine batch has separate evidence.

## Attempts to break it

| Attack | Result |
| --- | --- |
| Hide a looping closure inside a nominal value returned by a primitive | Rejected because every primitive argument and result must be storable data, recursively. |
| Put a source function in a generic event payload | Rejected by the data mark on the payload type variable. |
| Ask feedback for its output before a first value exists | Rejected because feedback reads the explicit initial value at step zero. |
| Change sound by changing host block size | Forbidden by the one-frame reference rule; batching is conditional on exact agreement. |
| Give two separately scheduled events the same handle | The merger injects left and right handles into disjoint sets. |
| Move the second of two exact duplicate events | Forbidden for a policy used by the overlay theorem. |
| Reverse two score boundaries while keeping each note's duration positive | Rejected by the finite monotonicity check. |
| Let a finished score source grow memory during an endless run | Prevented by the fixed `Finished` state. |
| Reseed a primitive by regrouping a machine tree | Prevented because seeds are explicit primitive configuration. |
| Treat a mixer as generic parallel composition | Prevented because `beside` returns a pair; mixing is an explicit primitive. |

## Musical cases

The core does not encode one theory of music. It gives theory packages two neutral tools: placed events and running
machines.

| Case | Where it fits |
| --- | --- |
| Tonal score | Notes, spelling, keys, chords, voicing, scale degree, and harmonic function remain separate library data and functions. Written facts occupy an event track. |
| Flexible or unmeasured time | The track stores exact local positions without meter. A checked time map handles swing, rubato, fermatas, and gradual tempo change. Meter and grouping remain library data. |
| Phrase-led performance | A phrase type may leave timing or ornament open. An interpretation function returns a performed gesture track and records choices or loss. |
| Ensemble-led tuning | Ensemble, tuning, and acoustic targets remain package data. They configure instrument machines; the core does not reduce them to twelve-tone pitch or one tonic. |
| Interactive live music | A finite protocol builds a machine with live input. The source finishes; the machine may keep stepping until the people stop. |
| Audio-led composition | A microphone, synthesizer, sampler, or effect can begin the program. A score is optional. Side-by-side paths and an explicit mixer cover retained acoustic and electronic sound. |

These are representation routes, not claims that a package adequately describes Karnatak music, gamelan, bomba, or any
other practice. Such a claim still needs a qualified practitioner.

## What the algebra says

The useful common structure is composition, not identity.

- Event tracks have two operations: put passages one after another, or place their events in the same span.
- Machines have two operations: wire one unit into the next, or run two units side by side.
- Scheduling preserves side-by-side musical placement only under the stated local policy.
- A time map may preserve succession under stronger additive conditions.

Mathematicians call the machine laws a symmetric monoidal category. Nothing in that name says a score is a sound. It
says only that wiring can be regrouped without changing the result. The checked scheduler supplies the narrower bridge
between placed musical events and a running event source.

This is more useful than a universal “motive,” `Description`, `world`, or `link` term. Those ideas may help compare
representations, but the examples did not force them into the execution core.

## Current code audit

**Verified at repository commit `06da1ee6f96e`.**

- `crates/musa-kernel/src/timeline.rs` already implements the untagged heart of `EventTrack`: exact rational length,
  finite occurrences, sequence by shifting, overlay by maximum and multiset union, and payload mapping.
- `crates/musa-dsp/src/effects.rs` contains stateful per-sample DSP units that can become registered machine primitives.
- `crates/musa-dsp/src/plan.rs` does **not** yet implement the candidate machine semantics. It defers cycle inputs at
  delay nodes to the previous host block, and its modulation path runs once per block. Host block size can therefore
  affect meaning. The candidate instead requires one-frame reference behavior and separately checked batching.

The focused current tests passed: 123 tests across `musa-kernel` and `musa-dsp`. That shows the existing code remains
healthy. It does not prove conformance to this proposal.

## Judgments

- The split between `EventTrack` and `Machine` is necessary. One stores a finite sparse object; the other describes an
  open-ended run. Draft A's single `Flow` failed on succession, mixing, and first-step feedback.
- Both types belong in one core language. This is the direct connection to audio that the earlier language closure lost.
- Dependent types, CBPV, first-class signals, and type-directed macros would add rules without removing a side condition
  in two distinct musical cases. They should stay out of the first core.
- Notes, chords, keys, meters, tuning systems, and instruments should remain libraries or registered primitives. The
  core should not choose a musical culture.
- `EventTrack`, `length`, `Machine`, and `step` are clearer names than `Timeline`, `extent`, `process graph`, and
  `tick`.

## Not checked

- The proof outline is not a mechanized proof and does not spell out every standard lambda-calculus case.
- No theory package has yet been implemented against the candidate.
- No practitioner has reviewed a culture-specific package.
- No current compiler pass, lineage pass, or audio plan has been proved to preserve these rules.
- Real-time deadlines remain engineering contracts checked at preparation. Totality alone does not prove speed.
- Persistent package identity and compiled caches remain outside this effort.

## Recommendation

Adopt this as the language-design candidate and stop adding core forms. The next work should be concrete:

1. write one complete tonal package and one audio-led program on paper against these exact types;
2. define the private typed core syntax and evaluator that implement §2;
3. define the primitive registry and the one-frame machine interpreter;
4. test a flattened audio plan against repeated structural frame steps; and
5. only then propose changes to `docs/rules/` and implementation prompts.

If either complete program needs a new core feature, record the smallest failing term. Do not reopen the calculus on an
analogy alone.
