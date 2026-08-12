# Finite typed process calculus

## 1. Purpose and boundary

This calculus gives a finite studio/instrument description formation rules and a tick transition. It does not give a
signal musical extent and is not a source-language requirement. `StudioSpec` may elaborate into this private IR.

Fix one execution contract `k` containing sample rate, channel/port schemas, tick size, numeric policy, and resource
limits. A port type belongs to that contract; only equal compatible types may be wired.

## 2. Graph syntax

```text
G ::= graph k (I⇒O) { nodes; wires; registers; boundary }

node n : X_n ⇒ Y_n state S_n using op_n
wire     n.out → m.in
register n.out -[q₀]→ m.in
input    I.p → n.in
output   n.out → O.p
```

A node operation is a first-order total transition supplied by the closed processor registry:

```text
step_n : X_n × S_n → Y_n × S_n.
```

It cannot receive or return a function. It has no allocation, lock, I/O, diagnostic, or ambient-state operation. A
foreign processor enters only with a versioned port/state schema, a total transition contract for admitted finite
blocks, and a resource bound.

A register carries a value of its edge type. `q₀` is its explicit initial value. At tick `j`, its target reads the value
stored after tick `j-1`; the source output of tick `j` becomes its stored value for tick `j+1`.

## 3. Formation

Write `dep_G(n,m)` when a same-tick `wire` carries any output of node `n` to any input of node `m`. Registers do not
create same-tick dependencies. Formation requires:

1. node, port, and boundary names are unique;
2. every edge endpoint exists and has equal compatible type;
3. every required node input has exactly one source after fan-in operators are made explicit nodes;
4. every boundary output has exactly one source;
5. all register initial values have their edge types;
6. the whole-node dependency graph `(nodes,dep_G)` is acyclic; and
7. the admitted resource bound covers every node, edge, state value, and buffer before runtime allocation.

The judgment is:

```text
Registry ⊢ G : Graph k I O.
```

A port-level graph being acyclic is insufficient. The schedule must execute the primitive whole-node API, which waits
for all current inputs before producing any output.

## 4. One-tick operational semantics

Let `σ` map every node to state and every register to its stored value. Let `ι:I` be the current boundary input.

1. Make boundary inputs and prior register values available.
2. Choose the canonical topological order of `dep_G`.
3. For each node `n` in that order, collect its complete input tuple `x_n`, then compute `(y_n,s_n')=step_n(x_n,s_n)`
   once.
4. Route `y_n` along same-tick wires and to boundary outputs.
5. After every node has stepped, replace each register with its source output and every node state with `s_n'`.

Write:

```text
G ⊢ (σ,ι) ↦ (σ',o).
```

The canonical order is an implementation-independent tie-break among topological schedules. Independently schedulable
nodes cannot observe scheduling because processors are pure and communicate only through typed edges.

## 5. Denotational semantics

For fixed initial state `σ₀`, the graph denotes the causal stream function obtained by iterating the tick transition:

```text
⟦G,σ₀⟧ : I^ω → O^ω
⟦G,σ₀⟧(ι₀ι₁…) = o₀o₁…
where G⊢(σ_j,ι_j)↦(σ_{j+1},o_j).
```

Only finite prefixes are computed during finite/offline rendering. The denotation is coinductive; the graph and each
prepared render request remain finite.

## 6. Theorems

### P1 — accepted ticks are total and deterministic

If `Registry⊢G:Graph k I O`, the registry transitions satisfy their contracts, `σ` is well typed, and `ι:I`, then one
unique `(σ',o)` satisfies `G⊢(σ,ι)↦(σ',o)`.

*Proof.* The finite dependency DAG has a canonical topological order. Formation makes every node's entire input tuple
available when it is reached. Each total deterministic `step_n` returns one output/state pair. Finite induction over the
order yields one complete next state and boundary output. ∎

### P2 — execution is causal

For fixed graph, parameters, seeds, and initial state, output through tick `j` depends only on input through tick `j`.

*Proof.* Induct on `j`. The current transition reads current boundary inputs, current node states, same-tick predecessor
outputs, and prior registered values only. The induction hypothesis bounds every state/register dependency by the prior
input prefix. ∎

### P3 — every feedback path crosses a register

Every directed graph cycle in the full connection graph contains a register edge.

*Proof.* Removing register edges leaves the acyclic dependency graph required by formation. A cycle containing no
register would remain there, contradiction. ∎

## 7. Host-block independence

The semantic tick is fixed by `k`, not by the size of a caller's render request. Rendering `n+m` ticks at once and
rendering `n` then `m` ticks from the returned state produce the same concatenated outputs by deterministic iteration.
Vectorized or device-block execution must implement this law.

A processor whose feedback delay changes when a caller asks for `512` frames once instead of `128` frames four times
does not implement this specification. Such behavior must be repaired or exposed as a different explicitly fixed tick
contract; it cannot remain ambient.
