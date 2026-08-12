# Rules for audio process graphs

This chapter defines two things: when an audio graph is valid, and what happens during one audio step. The graph is a
private compiler form. Composers do not need to write it.

## 1. Fixed settings for one prepared graph

Before Musa builds a graph, it fixes an execution contract `k`. The contract includes:

- sample rate;
- input, output, and channel types;
- the number of frames in one semantic step;
- numeric rules, including non-finite values; and
- memory and processor limits.

Only ports with the same compatible type may be connected. These settings are explicit inputs to audio preparation, not
global values read later by the callback.

## 2. Nodes, wires, and stored delays

A graph contains nodes, ordinary wires, and registers:

```text
node n : input type X -> output type Y, with state S, using processor op
wire n.output -> m.input
register n.output -[initial value]-> m.input
```

An ordinary wire carries a value made during the current step. A register carries a value from the previous step. At
step zero it supplies its declared initial value. After each step, it stores its source node’s new output.

This distinction gives feedback a clear meaning. In a delay loop, the processor reads yesterday’s value while producing
today’s. A loop made only of ordinary wires would need a value before the same value had been computed, so it is
rejected.

Each processor supplies a finite deterministic function:

```text
step_n(current inputs, current state) -> (current outputs, next state)
```

The function is first-order: it does not receive or return another function. It does not allocate, lock, perform I/O, or
read hidden global state.

## 3. When a graph is valid

Write `n -> m` when an ordinary wire carries any output of node `n` to any input of node `m`. Register edges do not
count because they carry a value from the previous step.

A graph is valid only if:

1. node, port, and boundary names are unique;
2. every wire endpoint exists and its types match;
3. every required node input has exactly one source, unless an explicit mixer node performs fan-in;
4. every graph output has exactly one source;
5. every register’s initial value has the correct type;
6. the whole-node graph formed by ordinary wires has no cycle; and
7. the resource bound includes every node, edge, state value, and buffer.

The formal statement is:

```text
processor registry ⊢ G : Graph(k, inputs, outputs)
```

The cycle check is over whole nodes. Checking ports alone is unsound because the processor API waits for all of a node’s
inputs before it can produce any output.

## 4. One audio step

Let `σ` contain every node state and register value. Let `ι` be the current graph input. One step runs as follows:

1. Make `ι` and the old register values available.
2. Order the nodes so every ordinary-wire source comes before its target. Use a fixed tie-break when several orders are
   possible.
3. In that order, collect all inputs for each node and call its processor once.
4. Send node outputs along ordinary wires and to graph outputs.
5. After every node has run, commit all next node states and new register values.

We write the result as:

```text
G ⊢ (σ, ι) -> (σ', o)
```

Here `σ'` is the next state and `o` is the current output.

## 5. The stream produced by a graph

Start with initial state `σ₀` and an input sequence `ι₀, ι₁, ...`. Repeating the one-step rule produces `o₀, o₁, ...`:

```text
(σ₀, ι₀) -> (σ₁, o₀)
(σ₁, ι₁) -> (σ₂, o₁)
...
```

The graph and each offline render request are finite. The possible input and output histories may be unbounded.

## 6. Basic theorems

The following results explain why the validity rules are necessary.

**Theorem P1: one step has one result.** Suppose the graph is valid, every processor obeys its stated total
deterministic contract, and the input and state have the right types. Then one audio step returns exactly one next state
and output.

**Proof.** A finite acyclic node graph has a fixed topological order. When the scheduler reaches a node, every ordinary
input has been made by an earlier node and every register input was available at the start. The node’s processor returns
one result. Induction over the finite node order yields one complete next state and graph output. ∎

**Theorem P2: the graph is causal.** Output through step `j` depends only on input through step `j`.

**Proof.** At step `j`, a node reads the current graph input, outputs of earlier nodes in the same step, current node
state, and register values from step `j - 1`. By induction, the stored state and register values depend only on earlier
inputs. No rule reads a future input. ∎

**Theorem P3: every feedback loop contains a register.**

**Proof.** Remove the register edges. The validity check says the remaining ordinary-wire graph has no cycle. Any cycle
in the full graph must therefore use at least one removed register edge. ∎

## 7. Caller buffer size must not change the result

The execution contract fixes the semantic step size. It does not change when a device or offline caller asks for a
larger buffer.

Running `n + m` steps in one request must give the same samples and final state as running `n` steps, keeping the state,
and then running `m` more. This follows by repeated use of Theorem P1.

A feedback processor that behaves differently for one 512-frame request and four 128-frame requests does not satisfy
this specification. The implementation must either fix it or declare a different fixed semantic step as part of `k`.
