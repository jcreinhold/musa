# A tracing test that only fails under `cargo test`

## What it looks like when it goes wrong

`crates/musa-project/tests/suite/logging_laws.rs` passes under `cargo nextest run` and under `cargo test --
--test-threads=1`, and fails perhaps one run in three under plain `cargo test`. This matters because `cargo insta test
--workspace --unreferenced=reject` — which several prompt **Check** blocks run — uses cargo test's threaded runner. The
red it produces names a law about logging, so the natural next move is to go and debug the compiler, and the compiler is
not what broke.

The report looks like a pipeline that stopped emitting:

```text
---- logging_laws::a_compilation_opens_one_compile_span_naming_its_document stdout ----
assertion `left == right` failed: one compilation, one span: [
    Opened { name: "render_notation", fields: [("target", "Mei")] },
]
  left: 0
 right: 1
```

Read that carefully before assuming a leak: the `render_notation` span is *the test's own*, because
`ProjectSession::from_text` compiles and then renders MEI on one thread. Nothing foreign arrived. The `compile` span,
one call earlier on that same thread, was never built at all.

## Why

`tracing` decides whether to build a span in two steps, and only the second one asks a subscriber. The first is a
process-wide cache: one `Interest` per `info_span!` site in the source, consulted by the macro before any dispatch
happens. `Interest::never` there means the span is skipped with no subscriber consulted, on every thread.

Who fills that cache in is the problem. From `tracing-core-0.1.36/src/callsite.rs`:

```rust
pub(super) fn rebuilder(&self) -> Rebuilder<'_> {
    if self.has_just_one.load(Ordering::SeqCst) {
        return Rebuilder::JustOne;
    }
    Rebuilder::Read(LOCKED_DISPATCHERS.read().unwrap())
}
```

and `Rebuilder::JustOne` resolves through `dispatcher::get_default` — **the current thread's** subscriber. So while at
most one subscriber is registered, the first thread anywhere in the binary to reach a callsite decides that callsite's
`Interest` for the whole process, using whatever subscriber *that* thread happens to have.

Under a threaded runner the other 124 tests in `musa-project`'s suite are compiling and rendering the whole time, on
threads with no subscriber. `NoSubscriber` answers `Interest::never`. Whichever of them reaches `compile` first writes
that answer down on behalf of the process, and the listening law then watches a compilation open no span. Which
callsites get poisoned is a race, which is why the failure moves between the three laws and why the captured list is a
different subset each time.

`with_default` cannot defend against this. It scopes a subscriber to one thread; the cache it loses to is global, and
the thread that poisons it is not the one running the law.

## What was measured

macOS 15 (Darwin 25.5.0), arm64, 12 cores; `tracing` 0.1.44, `tracing-core` 0.1.36, `tracing-subscriber` 0.3.23. Each
row is whole-suite runs of `cargo test -p musa-project --test suite`.

| Arrangement | Failures |
| --- | --- |
| `with_default` per law, as originally written | 3 / 15 |
| …plus a mutex serialising the three laws against each other | 14 / 40 |
| …plus `tracing::callsite::rebuild_interest_cache()` inside the listening scope | 13 / 40 |
| one permanent global subscriber, capture in a thread-local | **0 / 50** |

The two middle rows are the fixes that suggest themselves, and both are worth knowing are dead ends. Serialising the
laws addresses the wrong contention — the interference is a sibling test in another file, not a sibling law — and it
made things *worse* by spreading the three laws further across the suite's run. Forcing a rebuild repairs the cache at
one instant, and any other thread's first touch of a callsite re-poisons it a moment later.

Filtering the suite to the file alone gives 0 / 30, which is the confirmation that nothing in the file is at fault.
Pairing the file with one other module at a time shows the interference is not one culprit but every module that
compiles a document: `session_laws` 7 / 8, `wire_laws` 6 / 8, `studio_laws` 5 / 8, `analysis_session_laws` 4 / 8, down
to `editing_laws` and `library_laws` at 0 / 8.

## The rule

**A test that asserts on emitted spans must not install its subscriber with `with_default`.** Install one global
subscriber for the test binary, once, and let it stay; then a law's question — which spans did *this* thread open — is a
thread-local, which is what it always was. `logging_laws.rs` does this in `listen()` and `OPENED_HERE`, and the reason
is written at both.

Two consequences worth knowing before adding a fourth law there:

- The global subscriber is unfiltered and permanent, so every callsite in the binary answers "yes" and every test's
  spans are built and then discarded. That is deliberate: a uniform answer is what makes the cache's contents
  independent of which thread got there first.
- `musa_project::Logging::install` can no longer win in that binary. That is not a loss —
  `installing_a_second_time_reports_that_it_did_not` is a law about the *second* install, and an already-claimed global
  is exactly the embedding-application case it exists to describe.

Under `cargo nextest run` none of this is load-bearing: a process per test means one subscriber and one thread, and the
cache has no one to race with. The arrangement is for the runner that does not give each test a process.
