# 62. The prelude leak, attributed

**Status: governs nothing.** `../../../plan/prompts/165c-release-what-a-compilation-held.md` is the prompt. This page is
the measurement that found the leak, the four probes that narrowed it, and the two suspects the same investigation
cleared. Taken on the tree prompt 165c lands on, on an arm64 macOS host, debug builds unless a row says otherwise.

## 1. Why RSS could not answer

The question began as "does the compiler leak", and the first three hours of it were spent on the wrong instrument. Peak
resident set size is a *high-water* mark: an allocator that frees a page is under no obligation to give it back to the
kernel, and the next allocation is happy to reuse it. So RSS cannot tell retention from reuse, and on this compiler it
reported clean:

| Workload | Peak RSS |
| --- | ---: |
| `musa check examples/in-c.musa` — the largest single example | 35 MB |
| `musa check examples/*.musa` — fifty-seven of them, one process | 45 MB |

Fifty-seven compilations for ten megabytes over the largest one reads as a compiler that releases what it takes. It was
not: every one of those compilations leaked a quarter of a megabyte, and the leaked pages were being reused for the next
compilation's live data, which is exactly what RSS cannot see.

The instrument that could is a `GlobalAlloc` that adds `layout.size()` on `alloc` and subtracts it on `dealloc`, read
after the `Compilation` and the `SourceDocument` have both been dropped. That is *live bytes*, and it is what
`crates/musa-compiler/tests/suite/retention_laws.rs` now counts.

## 2. The measurement

One workload, compiled twelve times in one process, live bytes read after each:

| Probe | Round 1 | Round 12 | Per round |
| --- | ---: | ---: | ---: |
| `tests/fixtures/events-pressure.musa` | 273,032 | 3,102,320 | **257,208** |
| a ninety-byte `piece` with one note | 273,032 | 3,102,320 | **257,208** |
| the same piece, importing `std::tonal::harmony` and `std::collections` | 274,759 | 3,103,... | **257,208** |
| a source that does not parse, so nothing elaborates | 4,940 | 4,940 | **0** |
| `events-pressure` with `memo_for` returning `None` for every head | 273,032 | 3,102,320 | **257,208** |

Perfectly linear, and each row closes a door:

- **Constant in the program.** The same figure to the byte for ninety bytes of source and for the events pressure
  fixture, so it is not the program's own values.
- **Zero when elaboration does not run.** So it is on the elaboration path and not in parsing, lowering, or diagnostics.
- **Unmoved by more of the standard library.** So it is not the imported context.
- **Unmoved by disabling the δ-unfolding memo outright.** So it is not the memo, which is the suspicion prompt `165b`
  would otherwise attract and which `../../../plan/prompts/166b-per-context-memo-stamp.md` was already looking at.

What is left, and what is the right size, is the prelude that `crate::registry::owned()` built from scratch on every
call.

## 3. The cycle

`Arc` is exact for acyclic structure and blind to cycles, and the prelude's context is a cycle. Four readings, each one
line of a type definition:

1. `kernel/context.rs` — `Globals(Option<Arc<Tables>>)`, and `Tables.definitions: List<Arc<Defined>>`.
2. `kernel/program.rs` — `Defined.ty: Arc<Value>`.
3. `kernel/value.rs` — `Form::Pi { codomain: Closure, .. }`, and a `Closure` captures an `Env`.
4. `kernel/value.rs` — `Env.globals: Globals`, the same `Arc<Tables>` the definition was declared into.

Every definition whose type is a function type closes the loop, which is nearly all of them. `Head::Const`,
`Head::Base`, and `Head::Def` each carry a `Globals` as well, so the loop closes several other ways too.

None of that is a mistake. `context.rs`'s own doc comment argues the back-edge at length: a closure must resolve against
the table it was *built* under, which is the only table that can be right, and threading it as a parameter would still
leave a closure applicable under some other one. The design is deliberate and the cycle is its price. What was a mistake
was paying that price once per compilation.

## 4. The fix, and why it is the small one

`registry::owned()` takes no arguments and is a pure function of the compiler's own tables, so it is built once per
process behind a `LazyLock` and cloned. The cycle then retains exactly one prelude for the life of the process, which is
not a leak — a prelude is meant to live that long.

The alternative was to weaken the back-edge to a `Weak<Tables>`, and it was not taken. It needs a strong owner that
outlives every value built under the table, naming that owner is a design with a soundness argument of its own, and it
would be re-opening a decision `context.rs` already made for stated reasons. Cache first.

**It depended on `166b` and could not have landed before it.** A shared prelude means shared `Value`s, and a shared
`Value` carries the memo cell prompt `165b` put on it. Under the process-global invalidation stamp `166b` removed,
whether a second compilation *hit* a cell the first one filled depended on what other threads had been solving in
between — so caching the prelude would have taken that non-determinism and multiplied it by the whole prelude. With the
stamp scoped to its run, a cell one compilation fills is never a hit for another, whatever the two are doing.

## 5. After

| Measurement | Before | After |
| --- | ---: | ---: |
| Live bytes retained per `compile` call | 257,208 | **0** |
| `cargo test -p musa-compiler --test suite`, peak RSS | 1.72 GB | **651 MB** |
| `musa check examples/*.musa` (release), wall clock | 1.30 s | **0.93 s** |
| `musa check examples/in-c.musa` (release), wall clock | 0.15 s | 0.15 s |

The single-file row is the honest one: a lone compilation still pays for one prelude, because someone has to. What the
cache buys is that the fifty-seventh does not.

Step counts do not move, and not by luck: a `Cx` carries a `Budget`, which is a limit, and not a `Meter`, which is the
counter. The meter is minted per facade call, so what building the prelude spent was never charged to a caller either
way. `Budget::LANGUAGE`'s note in `kernel/budget.rs` does not move.

## 6. Two suspects the same investigation cleared

Recorded so that a later reader does not re-open them for free.

**The stack room.** `kernel/room.rs` gives each outermost facade call a scoped thread of `Budget::NESTING ×
FRAME_CEILING` = 40 MiB, and the compiler calls the facade in loops: a compile of `examples/in-c.musa` spawns
**129–175** of them, one per `declare`, `check`, or `infer`. It reads like a problem and it is not one. Patching
`with_room` to take the caller's stack unconditionally — every spawn and every reservation gone — leaves the same run at
the same 0.15 s and the same 37 MB. The reservations are lazily committed and never touched deeply, and a thread spawn
is microseconds. Hoisting the room to one thread per compilation is a real API change to a trusted crate for no measured
gain.

**The forced delay.** `kernel/value.rs`'s `Delay { env, term, forced }` never releases `env` or `term` after `fill`
writes `forced`, which is exactly the garbage Peyton Jones ch. 12 §12.4's update rule exists to release: an `Env` is a
list of shared `Value`s, and those values' spines hold further delays holding their own environments, so the retention
chains. That reading is still true. It is still not this leak — this leak is constant in the program and delays are not,
and there is exactly one construction site (`eval.rs`'s `Frame::Argument` arm, guarded by `family::delays_next`), so
delays exist at recursor method positions and nowhere else. Releasing a forced delay's body is a real change with a real
argument, and it wants its own prompt and its own workload, one where delays are the heap.
