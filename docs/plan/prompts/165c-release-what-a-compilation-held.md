---
id: 165c
slug: release-what-a-compilation-held
status: pending
depends_on: [165b, 166b]
phase: 3
---

> **Written from a measurement taken after prompt 164a**, when a workspace test run was investigated for a memory
> failure. The failure itself was a parser defect and is fixed. What the investigation *also* found is below, and it is
> the part no prompt owned: a compilation's peak footprint is small and a process that does many compilations grows
> anyway.

# Release What a Compilation Held

## Task

A finished compilation leaks its prelude context. **257,208 bytes per `compile` call, exactly, whatever the program is**
— the same figure for a ninety-byte piece, for `tests/fixtures/events-pressure.musa`, and for a piece that imports half
the standard library. Release it.

It is not a high-water-mark artefact and not an allocator declining to return pages: it is live bytes, counted by a
`GlobalAlloc` that adds on `alloc` and subtracts on `dealloc`, read after the `Compilation` and the `SourceDocument`
have both been dropped. A process that compiles grows by a quarter of a megabyte per compilation forever, which is why
`cargo test -p musa-compiler --test suite` peaks at 1.13 GB single-threaded over 503 tests and 1.72 GB over sixteen
threads.

## Read

- **The measurement, and how it was narrowed.** A counting global allocator, live bytes after `drop(compilation)`, one
  workload compiled twelve times in one process:

  | Probe | Live bytes, round 1 → 12 | Per round |
  | --- | --- | --- |
  | `tests/fixtures/events-pressure.musa` | 273,032 → 3,102,320 | **257,208** |
  | a ninety-byte `piece` with one note | 273,032 → 3,102,320 | **257,208** |
  | the same piece importing `std::tonal::harmony` and `std::collections` | 274,759 → 3,103,... | **257,208** |
  | a source that does not parse, so nothing elaborates | 4,940 → 4,940 | **0** |
  | `events-pressure` with `memo_for` returning `None` | 273,032 → 3,102,320 | **257,208** |

  Four things follow and each one closes a door. The leak is *constant in the program*, so it is not the program's
  values. It is *zero when elaboration does not run*, so it is on the elaboration path and not in parsing or lowering.
  It does not move when the δ-unfolding memo is disabled outright, so it is not the memo — which retires the suspicion
  [`165b`](165b-graph-update-and-data-descent.md) would naturally attract. And it does not grow when the program pulls
  in more of the standard library, so it is not the imported context either.

  What is left, and what is the right size, is the prelude that `crate::registry::owned()` builds from scratch on every
  call.

- `crates/musa-compiler/src/registry.rs`'s `owned()` — the whole function. It declares `prelude::phase()`,
  `prelude::musical()`, two `Registry`s, `prelude::methods_in`, and `prelude::collections`, and it is called once per
  compilation from `crates/musa-compiler/src/document.rs:571`.
- `crates/musa-calculus/src/kernel/context.rs` — `Globals(Option<Arc<Tables>>)` and `Tables`, whose `definitions` field
  is a `List<Arc<Defined>>`.
- `crates/musa-calculus/src/kernel/program.rs` — `Defined`, whose `ty` is an `Arc<Value>`.
- `crates/musa-calculus/src/kernel/value.rs` — `Head::Const(_, Globals)`, `Head::Base(_, _, Globals)`,
  `Head::Def(_, Globals, _)`, and `Env`'s own `globals` field, which every `Closure` captures.

  **These four readings are the cycle.** `Arc<Tables>` holds an `Arc<Defined>`; the `Defined`'s type is a `Value`; a
  `Value` that is a `Pi` captures an `Env` in its codomain closure and an `Env` carries a `Globals`; that `Globals` is
  the same `Arc<Tables>`. Every definition whose type is a function type closes the loop, which is nearly all of them.
  `Arc` does not collect cycles, so the prelude's table keeps itself alive after the last outside handle is gone.

- Peyton Jones ch. 17 on storage management is about a machine musa does not have and does not transfer. What does
  transfer is the ordinary observation underneath it: reference counting is exact for acyclic structure and blind to
  cycles, and a design that closes a cycle has chosen a collector it does not have.
- `crates/musa-dsp/tests/suite/rt.rs` — the workspace's existing counting allocator, and its doc comment on why the
  tally is per-thread. Read it before writing a second one.
- [`166b`](166b-per-context-memo-stamp.md), which this prompt now depends on — see **Design**.
- The `rust-performance` skill's workflow, and its rule against optimizing from intuition. Two other structures surfaced
  by the same investigation were measured and dismissed; see **Stop**.

## Design

**Build the prelude once.** `owned()` takes no arguments and answers the same context every time, so the fix that
matches the measurement is to compute it once per process and hand out clones. A `Cx` is a handle over `Arc`s and
cloning one is cheap. The cycle then leaks exactly one prelude for the life of the process — which is not a leak at all,
because a process-lifetime cache is meant to live that long — and the per-compilation cost of rebuilding it goes with
it.

**This is why the prompt depends on [`166b`](166b-per-context-memo-stamp.md) and must not run before it.** A shared
prelude means shared `Value`s, and a shared `Value` carries the δ-unfolding memo cell 165b put on it. Under today's
process-global invalidation stamp, whether a second compilation *hits* a cell the first one filled depends on what other
threads have been solving in between — so caching the prelude would take 166b's non-determinism and multiply it by the
whole prelude. With the stamp scoped to its context first, a cached prelude is safe. Doing these in the other order
would make the budget's acceptance depend on scheduling, which `docs/rules/language/02-core-calculus.md` §4 forbids.

**Do not break the cycle by weakening the back-edge.** `Head::Const`, `Head::Base`, `Head::Def`, and `Env` each carry a
`Globals` deliberately — `context.rs`'s doc comment argues it at length: a closure must resolve against the table it was
*built* under, which is the only table that can be right. A `Weak` there would need a strong owner that outlives every
value built under it, and naming that owner is a larger design than this prompt, with a soundness argument of its own.
Cache first; if a second cyclic table shows up later that a cache cannot answer, that is when the back-edge is worth
re-opening.

**Verify the cache did what the measurement predicted.** The number to beat is 257,208 → 0 per round. If caching drops
it to something small but non-zero, there is a second holder and it should be attributed the same way rather than
rounded off.

## Target

- `crate::registry::owned()`'s prelude built once per process, with the reason written where the cache is — that the
  context is a pure function of the compiler's own tables, and that its `Globals` is a reference cycle, so building a
  second one is both slower and permanent.
- A law that fails if the growth comes back: a counting global allocator, one workload compiled N times in one process,
  asserting that live bytes after the Nth are within a constant of after the first. Live bytes and not RSS — RSS is a
  high-water mark an allocator is free never to return, and it would have called this leak clean.
- A numbered note under `docs/notes/research/language-design-closure/` recording the attribution above: the probe, the
  five narrowing measurements, and the cycle read off the four types.
- `crates/musa-calculus/TRUST.md` updated if anything moved across the trusted boundary.
- The same step counts before and after, shown rather than asserted. `Budget::LANGUAGE`'s note in
  `crates/musa-calculus/src/kernel/budget.rs` records the staff adapter's spend and it must not move.

## Check

```sh
cargo build --workspace
cargo nextest run --workspace
cargo nextest run -p musa-calculus -p musa-compiler --run-ignored all
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo bench -p musa-compiler --bench pipeline -- p1_compile
PATH=/Users/jcreinhold/.cargo/bin:$PATH make docs-check
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
```

Commit as `Release what a compilation held`.

## Stop

- **No change to `crates/musa-calculus/src/kernel/room.rs`.** It was the other suspect and it was measured and
  dismissed: a compile of `examples/in-c.musa` spawns 129–175 scoped threads, one per outermost facade call, each
  reserving `Budget::NESTING × FRAME_CEILING` = 40 MiB of stack. Bypassing every one of them — patching `with_room` to
  take the caller's stack unconditionally — leaves the run at the same 0.15 s and the same 37 MB. The reservations are
  lazily committed and the spawns are lost in the noise. Hoisting the room to one thread per compilation is a real API
  change to a trusted crate for no measured gain, and it needs a workload that shows a cost before it is worth making.
- **No narrowing of the unfold memo and no change to `memo_for`.** It was a suspect and it was measured and dismissed:
  with `memo_for` returning `None` for every head, the leak is byte-for-byte identical. Its invalidation stamp is
  [`166b`](166b-per-context-memo-stamp.md)'s subject and that prompt's Stop already fences it.
- **No change to `Delay`.** It was the reading this prompt was first written around — a forced delay keeps its `env` and
  its `term` alive beside the value it forced to, which is exactly the garbage Peyton Jones ch. 12 §12.4's update exists
  to release. It is still true and it is still not this leak: the leak is constant in the program and delays are not.
  Releasing a forced delay's body is a real change with a real argument and it needs its own prompt and its own
  measurement, taken on a workload where delays are the heap.
- **No budget change, no cost-table change, no step-count change.**
- **No allocator swap.** Reaching for a different global allocator is the intervention the `rust-performance` skill puts
  last for a reason, and it would hide the question rather than answer it.
- **No adapter, stdlib, or example change.**
