# Studio feedback: choose the tick before claiming the law

**Status: verified implementation probe plus design recommendation; governs nothing.** This note follows the feedback
gap found in [25](25-proof-review.md) and makes it concrete against the current audio implementation.

## 1. What the code does now

**Verified in `crates/musa-audio/src/plan.rs`.** `schedule_order`:

1. finds every `ProcessorSpec::Delay` node which lies on a graph cycle;
2. removes every incoming scheduling dependency to each such node;
3. topologically orders the remaining node graph; and
4. renders each node once per audio block in that order.

The removed edge still names an input buffer. Because its producer runs later in the block, the delay node reads the
contents that producer left at the end of the **previous block**. The delay processor then runs its own sample-by-sample
delay line over that stale block.

Thus a graph cycle has two delays:

```text
authored sample delay + implicit one-block cut delay.
```

The block cut affects the whole delay-node input, including the dry path of its `mix` parameter. This is why the graph
is operationally causal even though an echo output can depend instantaneously on its dry input.

This is not a one-frame guarded recurrence. It is a deterministic block-tick recurrence whose transition internally
processes a vector of frames.

## 2. The current semantics is coherent but block-dependent

Let block size be `b` and the authored delay line contribute `d` frames on the feedback path. The graph scheduler's
cycle recurrence sees at least `b+d` frames of feedback latency, not merely `d`.

### Proposition 33.1 — changing block size can change feedback output

For a nonzero feedback gain and an impulse input, two executions with block sizes `b₁≠b₂` need not produce the same
sample stream, even when sample rate, authored delay parameter, seed, and graph are unchanged.

*Argument.* The first feedback return can occur no earlier than `b+d` under the current cut semantics. Changing `b`
therefore changes the index of that return unless another implementation detail happens to mask it. Subsequent returns
inherit the changed recurrence. Hence frame equality across block partitions does not follow and has direct impulse
counterexamples for suitable `d`, mix, and feedback. ∎

The exact counterexample should be added as a differential test before changing code. The proposition is a consequence
of the inspected schedule, not a claim that every existing effect preset audibly exposes the difference.

## 3. Why “cycle through a Delay node” is not the right static rule

A node-level label is too coarse. A future multi-input/multi-output processor may delay one path and pass another path
through immediately. Treating the whole node as delayed can admit an instantaneous algebraic loop through the
pass-through ports.

Even the current echo has a related distinction:

```text
output = mix(dry_input, delayed_input).
```

Its output is not strictly delayed in its input when dry mix is nonzero. Current execution remains causal only because
the compiler places a register on the **incoming graph edge**, not because `Delay` is strictly causal as a whole.

The static unit must therefore be an instantaneous dependency between individual ports or an explicitly registered edge.
Processor-name pattern matching is not a semantic contract.

## 4. Three honest designs

### A. Explicit block-register feedback

Make the existing operational model source-visible:

```text
feedback_block : BlockDelay b
```

Every accepted cycle crosses an explicit block register. The graph tick is `Block r b`, output is deterministic, and the
extra latency is part of the patch rather than inferred from a neighbouring processor name.

**Advantages:** matches the current scheduler; simple topological block execution; cheap.

**Costs:** sound depends on block size; offline and live renders with different blocks differ; the authored delay time
does not fully describe loop timing; a host with variable callback block size needs another policy.

### B. Frame-causal strongly connected components

Give every processor a conservative instantaneous port-dependency relation and explicit state edges. For each graph
strongly connected component:

1. reject it if its instantaneous per-port dependency graph has a cycle;
2. compute a topological schedule of the instantaneous port operations;
3. execute that schedule once per frame within the host block; and
4. commit delayed state at the declared frame boundary.

The host still renders blocks for efficiency, but block size is batching rather than semantics. An authored one-frame or
`d`-frame delay remains exactly that delay.

**Advantages:** natural DSP feedback meaning; block-partition independence; port-correct validation; no hidden latency.

**Costs:** processor contracts must expose per-port or per-output current-frame computations; SCC execution may inhibit
some block-vectorized optimizations; current whole-node `process(block)` APIs need an internal split or specialized
feedback executor.

### C. Reject feedback until B exists

Require the whole process graph to be a DAG. Delay and reverb remain stateful feed-forward processors, but no output may
route back to an upstream input.

**Advantages:** smallest truthful current semantics; retains fast block execution and partition-independent feed-forward
graphs where processors themselves provide it.

**Costs:** removes a musically and technically useful class of patches.

This is preferable to silently claiming B while implementing A.

## 5. Correct process contract

The K₂ draft's whole-node step plus port-dependency relation is still insufficient. A scheduler cannot evaluate one
output before all inputs merely because a relation says that output ignores some of them if the only executable API is

```text
step_p : State × Frame(AllInputs) → State × Frame(AllOutputs).
```

An honest frame-causal primitive contract needs something like:

```text
output_p,o : State × Frame(Deps(o)) → Value(o)
next_p     : State × Frame(AllInputs) × Frame(AllOutputs) → State
```

or a compiler-owned internal network of atomic port operations with the same information. `Deps(o)` must conservatively
contain every current-frame input which can affect output `o`; state update happens after the current outputs needed to
solve the SCC are known.

For block-register design A, the simpler whole-block step remains adequate because every cut edge supplies a completed
previous-block buffer.

## 6. Recommendation

Choose **B** as the long-term semantics if Musa intends to support authored feedback. It matches how a musician or DSP
author reads a stated delay, makes block size an implementation choice, and gives the type/validation system the right
unit: instantaneous port dependency.

Do not implement B inside the current research turn. First:

1. add an impulse differential test proving the current block-size dependence;
2. document current execution as design A, without calling it a one-frame guarded trace;
3. decide whether prompt 170's block-partition law includes feedback graphs;
4. prototype one feedback SCC with the per-output primitive contract; and
5. compare performance with current block execution before choosing whether B replaces A or feedback is temporarily
   restricted by C.

This does not require a studio term calculus, CBPV, or dependent types. It requires the process IR to state the
operational fact on which causality actually depends.
