---
id: 127d
slug: machine-values
status: done
depends_on: [127b, 127ca]
phase: 3
---

# Add Finite Machine Values to the Source Core

## Task

Add `Primitive<K,A,B>` and `Machine<K,A,B>` as finite storable source values. Source evaluation builds machine
descriptions; it never runs their open-ended histories.

## Read

- The prompt-127a machine specification and research `05-selected-calculus.md` §5 and `06-proof-outline.md` §4.
- Current source type/value representation, exact codecs, studio declarations, audio processor descriptors, and crate
  dependency direction.
- The module-design skill; compare a public evaluator value with a narrow exact `MachineSpec` projection consumed by
  `musa-dsp`.

## Design

Admit exactly these forms: registered primitive, identity, `connect`, `beside`, `feedback(initial,m)`, `copy`, `drop`,
and `swap`. Ports and feedback values must be storable data. There is no public `lift` from a source function and no
unguarded loop.

The checker uses ordinary type equality to connect ports and the nominal step tag `K` to prevent unlike step meanings
from connecting. A machine value is finite, exact, and contains primitive id, version, port schemas, and storable
configuration; it contains no source closure or running state.

Keep the evaluator's `Value` private. Expose one immutable, exact, caller-oriented `MachineSpec` projection only because
`musa-dsp` is its named consumer. Compare this boundary against adding a new crate; do not add a crate unless two
independent current consumers need the same stable representation and the new crate hides more than it exposes.

The build-local primitive descriptor table rejects one id/version paired with unequal schema or configuration codec.
Actual start state and step code remain absent until prompt 171.

## Target

- Source types, values, inference rules, exact encoding, constructors, diagnostics, and compiler projection.
- Small complete paper/source programs for chains, side-by-side paths, initialized feedback, copy/drop/swap, and type
  mismatches.
- Law tests over a finite test primitive family; rejection tests for hidden closures, wrong step tags, bad ports,
  uninitialized feedback, and attempted `lift`.
- Public-surface comparison and module-design audit.

## Check

```sh
cargo nextest run -p musa-compiler -p musa-project
cargo clippy --all-targets -p musa-compiler -p musa-project -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
bash .agents/skills/module-design/scripts/audit-module.sh crates/musa-compiler
```

Commit as `Add finite machine values to the source core`.

## Stop

- No machine execution, audio preparation, scheduler, DSP primitive, arbitrary graph, public function lifting, general
  feedback, dynamic plug-in, or behavioral-equality decision.
- No public core AST or public mutable machine tree.
