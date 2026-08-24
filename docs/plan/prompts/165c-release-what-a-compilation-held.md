---
id: 165c
slug: release-what-a-compilation-held
status: pending
depends_on: [165b]
phase: 3
---

> **Written from a measurement taken after prompt 164a**, when a workspace test run was investigated for a memory
> failure. The failure itself was a parser defect and is fixed. What the investigation *also* found is below, and it is
> the part no prompt owned: a compilation's peak footprint is small and a process that does many compilations grows
> anyway.

# Release What a Compilation Held

## Task

Find what a finished compilation keeps alive, and release it. One compilation costs about 2.6 MB live; a process that
runs several hundred of them grows by roughly 2 MB apiece and never gives it back. Nothing in `docs/rules/` is violated
by that — it is not a correctness bug — but it is the difference between a desktop session that stays flat over an
afternoon of edits and one that does not, and `docs/rules/desktop/06-frame-budgets.md` assumes the first.

**Attribute before fixing.** This prompt's first half is measurement, and its Design names a leading hypothesis rather
than a conclusion, because the hypothesis is a reading of the code and not a profile.

## Read

- The measurements this prompt exists for, reproducible from the repo root against a release build:

  | What | Peak RSS | Note |
  | --- | --- | --- |
  | `musa check examples/in-c.musa` | 35 MB | the largest single example |
  | `musa check examples/*.musa` (57 files, one process) | 45 MB | released and reused — the CLI shows no growth |
  | `cargo test -p musa-compiler --test suite -- --test-threads=1` | 1.13 GB | 503 tests, ~2 MB apiece over a ~90 MB base |
  | `cargo test -p musa-compiler --test suite` | 1.72 GB | the same, sixteen threads |
  | `cargo bench -p musa-compiler --bench pipeline -- p1_compile` | 838 MB | divan reports 2.619 MB / 35,750 allocations *live* per iteration |

  The CLI row and the bench row disagree, and reconciling them is the first piece of work: the CLI's examples are small
  and the bench's workloads are the pressure fixtures, so the difference may be size rather than retention. Establish
  which before changing anything.

- `crates/musa-calculus/src/kernel/value.rs` — `Delay`, `Neutral::unfolded`, `memo_for`, and `loosen`. `loosen` and its
  worklist are what make dropping a deep value iterative rather than recursive, so the drop path is already deliberate
  and is the right place to read from.
- `crates/musa-calculus/src/kernel/eval.rs` — `demanded` and `Frame::Forcing`, the two places a delay is filled.
- Peyton Jones ch. 12 §12.4, the update rule: a shared redex's root is overwritten with its result, and the point of
  overwriting rather than annotating is that the redex's environment becomes garbage. Ch. 17 on storage management is
  about a machine musa does not have and does not transfer; §12.4's *reason* does.
- `crates/musa-compiler/benches/pipeline.rs` — the workloads, and `divan`'s allocation counters, which measure live
  bytes at an iteration's peak and therefore cannot see retention across iterations. Whatever measures this has to
  measure something else.
- The `rust-performance` skill's workflow, and its rule against optimizing from intuition. Two of the three structures
  this investigation surfaced turned out to cost nothing measurable; see **Stop**.

## Design

**The leading hypothesis is that a forced delay never releases its environment.**

```rust
pub(crate) struct Delay { env: Env, term: Term, forced: OnceLock<Value> }
```

`fill` writes `forced` and leaves `env` and `term` in place for the delay's whole life. An `Env` is a persistent list of
shared `Value`s, and a `Value`'s spine holds further delays holding their own environments, so the retention chains: a
single live delay can pin the whole local scope it was built under, long after the only thing anyone will ever read from
it is `forced`. That is precisely the garbage §12.4's update exists to release.

It is a hypothesis and not a finding because only one site constructs one — `eval.rs`'s `Frame::Argument` arm, guarded
by `family::delays_next` — so delays exist at recursor method positions and nowhere else. Whether that is enough of the
heap to explain 2 MB a compilation is a question for a measurement, not for a reading.

**If it is confirmed, the shape of the fix is to make the body droppable.** `parts()` hands out `&Env` and `&Term`
today, so releasing them means the body moves behind something a reader can take from — a `Mutex<Option<(Env, Term)>>`
beside the `OnceLock`, taken under the lock in `demanded`, or the two collapsed into one cell that holds *either* the
body or the value. Two threads may force one delay (`fill`'s doc comment says so and totality is why it is allowed), so
whichever shape is chosen has to keep that safe: a loser that finds the body already taken reads the value instead.

**If it is not confirmed, say so and follow the measurement.** A prompt that names a hypothesis is not a prompt that
mandates it. The other structures read during the same investigation and *not* selected are in **Stop**, with the
numbers that dismissed them, so a later reader does not re-open them for free.

**No budget change and no cost-table change.** Releasing memory must not change what a program costs in steps: the
budget is acceptance (`docs/rules/language/02-core-calculus.md` §4) and this prompt is about bytes. If a candidate fix
would change a step count, it is the wrong fix.

## Target

- A measurement that attributes the growth, recorded in `docs/notes/research/language-design-closure/` as a numbered
  note: what was measured, with what command, on what machine, and what fraction of the growth each attributed structure
  accounts for.
- The attributed retention released, in the crate that owns it.
- A law that fails if it comes back — a test that runs one workload N times in one process and asserts the process's
  live bytes after the Nth are within a constant factor of after the first. Live bytes and not RSS: RSS is a high-water
  mark that an allocator is free never to return, which is why the CLI row above proves less than it looks like it does.
  `crates/musa-dsp`'s `CountingAllocator` is the existing instrument of this kind in the workspace and is the thing to
  read before writing a second one.
- `crates/musa-calculus/TRUST.md` updated if anything moved across the trusted boundary.
- The same step counts before and after, shown rather than asserted: `Budget::LANGUAGE`'s note in
  `crates/musa-calculus/src/kernel/budget.rs` records the staff adapter's spend, and it must not move.

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
- **No narrowing of the unfold memo and no change to `memo_for`.** Its invalidation stamp is
  [`166b`](166b-per-context-memo-stamp.md)'s subject and that prompt's Stop already fences it. A memo cell holds a value
  no longer reachable through a stamp that has moved on, which is retention of the same family — but it is retention
  *within* a live value, so it goes away when the value does, and the two prompts must not both edit the same field.
- **No budget change, no cost-table change, no step-count change.**
- **No allocator swap.** Reaching for a different global allocator is the intervention the `rust-performance` skill puts
  last for a reason, and it would hide the question rather than answer it.
- **No adapter, stdlib, or example change.**
