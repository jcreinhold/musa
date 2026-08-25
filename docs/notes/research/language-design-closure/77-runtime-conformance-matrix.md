# 77. Runtime conformance after the dependent core

**Status: governs nothing.** This is prompt 174's audit record. It imports prompt 169's K1–K20 matrix and note 67's
adapter theorem. Its first premise is therefore K14's output: a kernel-rechecked, finite event term whose payload type
has passed `Storable` and whose owner supplies a versioned canonical encoding.

`scripts/check-core-calculus-conformance.sh` runs every row below. A passing test is a falsifier that found no defect in
its stated corpus, not a mechanized proof of the universal claim. The proof argument after the table states the owner
contracts on which the verdict remains conditional.

## Scope and comparison

For a coordinate `C` and admitted payload `A`, an event track is the exact pair `(d,E)` from the governing event rules.
For registered step kind `K` and storable ports `A,B`, a machine description denotes the initialized deterministic
transition `S × A → S × B` supplied by its closed primitive registry. Structural equality means equality of finite
descriptions. Behavioral equality means equality of output histories for every input history; this audit never claims a
decision procedure for the latter.

Scheduling compares an exact source boundary with one exact physical time, then with one frame under a complete
rounding/collapse policy. Audio comparison is equality of the interleaved sample history, including bit equality where
the tests say byte equality. A real-time deadline is not among these comparisons.

## Executable matrix

| Row | Obligation and implementation owner | Executable falsifier and what failure means |
| --- | --- | --- |
| **R1** | Event construction enforces `0 ≤ start ≤ end ≤ d`; coordinates remain type-separated and support is half-open. `musa-events::{time,occurrence,track}`. | Bounds, negative-duration, half-open-support, and coordinate-identity laws. A failure admits an invalid span, conflates coordinates, or observes a boundary twice. |
| **R2** | `follow` is translated multiset union with additive duration; `together` is multiset union with maximum duration. Multiplicity is retained. | L1–L6 plus the non-idempotence and duplicate-occurrence counterexamples. A failure changes duration, placement, or multiplicity. |
| **R3** | Construction is normalization; semantic equality and stable identity use the complete canonical payload key, coordinate, duration, spans, and multiplicity. Gesture identity encodes every provenance constructor explicitly and length-frames arbitrary source strings. | Canonical-order, schema/multiplicity framing, arbitrary-delimiter, and gesture-framing laws. A failure makes equality order-sensitive or permits an encoding collision. |
| **R4** | A finite machine prepares only when its tree, ports, stored data, primitive descriptor, depth, state, and work bounds validate against one unique versioned registry. | Registry-uniqueness and malformed-structure/stored-data controls. A failure admits an unregistered, ill-typed, cyclic/reused, or unpriced description. |
| **R5** | Every prepared machine has one initialized state and one total deterministic next step on a value of its checked input port. Output prefixes are causal. | Per-constructor steps, primitive registration witnesses, dynamic-port refusal, deterministic-history, and future-input controls. A failure makes a checked step partial, nondeterministic, or acausal. |
| **R6** | Feedback reads its explicit initial value on step zero and commits new state after the current output. `connect` consumes the current left output; `beside` retains both outputs and never mixes. | Feedback trace, connect/beside equations, connect reassociation, and the stale-whole-node counterexample. A failure changes first output, delays a chain, or invents mixing. |
| **R7** | A checked schedule records every exact source/physical/frame decision under one complete versioned policy and rejects missing, conflicting, reversed, or over-budget maps. | Exact-decision determinism, collapse controls, malformed-map controls, and adversarial preparation bounds. A failure hides a choice or lets resource policy change an answer. |
| **R8** | Every positive boundary is emitted exactly once, with end before begin at one frame; point occurrences use the declared point convention. A source reaches a fixed allocation-free `Finished` state. | Exact-once/half-open ordering, point policy, countdown/seek, and ambient-silence laws. A failure duplicates a boundary, changes same-frame order, or grows after completion. |
| **R9** | Handles are opaque and occurrence-local. Successful overlay scheduling preserves duplicate occurrences; schedule merging injects the two handle namespaces. | Duplicate-overlay/merge and debug-hygiene controls. A failure aliases two occurrences or leaks canonical payload spelling through a handle. |
| **R10** | Succession is preserved only under an additive time map and additive frame conversion, by delaying the second schedule. No unconditional nonlinear theorem is claimed. | The additive-follow differential law. A failure breaks the theorem under its premises; cases outside those premises are deliberately unclaimed. |
| **R11** | Audio preparation consumes exact gesture tracks directly through R7–R10, checks the complete private primitive registry, layout, tuning, memory, and worst-step work, then allocates all state before running. | Production preparation refusals, NaN controls, registry witness coverage, and resource-limit controls. A failure admits unsupported or unpriced callback state. |
| **R12** | One audio step applies the current event batch before producing exactly one stereo frame. Host partitioning is repeated application of that step; random behavior depends only on the explicit render seed. | 64/256 partition oracle, effect/envelope/modulation/feedback laws, explicit-seed law, and old-whole-node counterexample. A failure makes callback size or an implicit seed part of musical meaning. |
| **R13** | Offline and live harnesses call the same step. The callback performs no allocation, lock, I/O, or logging, and replaced plans retire on the control side. | Shared-step, allocation, logging, retirement, and migration-oracle laws. A failure introduces a second interpreter or violates an instrumented RT obligation. Deadlines remain an owner contract, not a theorem. |
| **R14** | Cross-stage programs preserve the already-proved K14 value through exact gestures, scheduling, and repeated machine steps; derivation grafting covers reuse, shares common inputs, and is associative. | The six complete-program witnesses, derivation coverage/reuse/grafting laws, backend oracle, and exact gesture provenance. A failure loses an origin, requires a new core form, or makes stage grouping observable. |

## Proof architecture

The top-level statement is intentionally conditional.

**Theorem (implemented runtime cutover).** Assume K1–K20 and note 67. Assume each registered source builtin satisfies
its declared total/partial contract, each registered machine primitive satisfies its declared initial-state and total
step contract, each canonical payload key is total and complete for its advertised quotient, and any future optimized
whole-machine batch satisfies exact state-and-output equality with repeated one-frame steps. Then every successfully
prepared notation-led program has the event-track, scheduling, and audio-history meaning stated by the governing rules.
The same machine rules give a meaning to successfully prepared audio-led programs without requiring a score.

The proof has four milestones.

**Proposition 1 (event representation).** Every admitted event term evaluates to one bounded normalized event track, and
`follow`, `together`, observation, and payload mapping have the stated denotations.

**Proof.** K14 supplies a well-formed finite term. Induction on that term uses the constructor equations. `track` checks
every basis occurrence against the ambient duration; `follow` translates the second occurrence multiset by the first
duration; `together` unions the multisets without padding; and mapping leaves spans untouched. The flat representation
is already the normal form. Sorting by exact start, end, and the framed payload key therefore computes the canonical
representative. R1–R3 exercise each induction case and the counterexamples to idempotence, closed support, and
coordinate erasure. ∎

**Proposition 2 (machine execution).** A prepared finite description and an input history determine one causal output
history by repeated next step.

**Proof.** Preparation validates a finite tree and supplies each leaf's registered initial state. Structural induction
on that tree gives one step: primitives use their owner contract; wiring constructors are total rearrangements;
`connect` substitutes the current left output into the right input; `beside` takes the product of the two steps; and
feedback reads the stored value before replacing it. Thus one tree step is total and deterministic. Induction on the
input-history length gives a unique output history, while the one-step equation shows that its prefix depends only on
the corresponding input prefix. R4–R6 exercise every constructor, including the first feedback output and regrouped
connection. This proves no decidability result for behavioral equality. ∎

**Proposition 3 (checked scheduling).** Under a complete policy and a finite monotone time map answering every required
boundary, scheduling yields one finite deterministic decision record and one finite event source. Overlay is preserved
under the occurrence-local policy; succession is preserved under the additive premises of R10.

**Proof.** Enumerate the finite occurrences and their boundaries. Exact map lookup precedes the policy's single
physical-to-frame conversion, so every boundary receives one recorded decision. Canonical occurrence order and the
policy's complete kind order make collisions deterministic. Fresh occurrence handles distinguish multiplicity, and merge
injects left and right handles into disjoint namespaces. The overlay result follows because duplicate occurrences
receive equal decisions without receiving equal handles. For succession, additivity maps a translated boundary `d+s` to
the scheduled duration of `d` plus the image of `s`; delaying the second finite table therefore gives the combined
table. Without that hypothesis the equation is not used. Exhausting the table changes the source to its fixed `Finished`
variant. R7–R10 contain negative controls for every premise. ∎

**Proposition 4 (one-frame audio and composition).** Prepared audio's output under any host partition is the history
obtained by repeatedly applying the one-frame reference step, with the scheduled batch delivered before each output.
Offline and live execution agree when given the same prepared value and commands.

**Proof.** `PreparedAudio::step` is the reference transition. `render` partitions a mutable host slice into stereo
frames and calls that transition once for each frame; it contains no batch semantic shortcut. Induction on the number of
frames proves equality with repeated step, and a second induction over host slices proves partition independence. The
offline renderer and playback callback both call this operation, so their only remaining differences are explicit
transport commands. R11–R13 compare partitions, seeds, harnesses, and the instrumented RT path. Derivation composition
is independent of execution: its graft operation shares matching inputs and its unit laws make three-stage grouping
equal; the R14 witnesses keep the gesture's full `Origin` payload through scheduling. ∎

The theorem follows by composing Propositions 1–4 at their typed boundaries. No proposition introduces a universal
`Music` object, a second evaluator, a second frame schedule, or an implicit dictionary. Pattern unification remains
exactly the scoped Miller fragment imported through K10–K11: distinct bound-variable spines may be inverted, repeated or
out-of-scope spines postpone or refuse, and the runtime audit adds no unification rule.

## Complete programs and limits

| Route | Executed witness | What it establishes | What it does not claim |
| --- | --- | --- | --- |
| Tonal construction | `examples/tonal-construction.musa` through compile, exact gestures, preparation, and audio steps | Harmony and voicing remain distinct source values and reach finite audio. | Adequacy of a culture-specific harmonic theory. |
| Flexible time | `examples/chant.musa` through the same chain | Exact occurrence time and sound do not require meter. | A unique performance of chant. |
| Phrase-led transcription | A finite `TranscriptionCandidate` track with a nonempty loss statement | Ambiguity and loss can remain data rather than hidden choice. | A production transcription algorithm or uniqueness. |
| Ensemble tuning | `examples/in-c.musa` prepared at concert A = 432 Hz | Tuning configures the instrument edge without rewriting written pitch. | Adequacy for any ensemble's tuning practice or acoustic model. |
| Live protocol | A finite checked counter-machine description stepped 10,000 times | Finite source construction does not impose a fixed runtime end. | A network/device protocol or deadline. |
| Audio-led | Paired microphone/synth frame inputs, separate gain effects, explicit mixer | Input, side-by-side processing, and mixing need no score or universal link. | Microphone I/O, a production synthesizer frontend, or plug-in hosting. |

The literature is used at its actual strength. The selected calculus supplies the two-value architecture and conditional
scheduling theorem; its proof outline supplies the induction routes and owner assumptions; the final review supplies the
counterexamples and explicitly withholds theory adequacy, performance uniqueness, and RT deadlines. Peyton Jones's
pattern discipline is imported through K10–K11 rather than reimplemented here. Ousterhout's information-hiding test is
why the graph, render plan, primitive state, handles, and frame schedule remain private.
