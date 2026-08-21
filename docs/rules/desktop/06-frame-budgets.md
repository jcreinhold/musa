# 06 — Frame Budgets

Status: **governing**.

Roadmap §17/§13.2 hold the audio thread to measured guarantees. The interface gets the same treatment: named budgets,
measured on a real workload, checked in the prompt that introduces the surface. "Feels fast" is not a criterion.

## 1. Budgets

Measured on the reference workload — `examples/glass-mountain.musa` for the small case and a generated 100-bar, 4-part
score (`tests/fixtures/large-score.musa`, created at prompt 22) for the large case — on the developer machine, p95 over
20 trials.

| # | Interaction | Budget | Why this number |
| --- | --- | --- | --- |
| B1 | Keystroke → diagnostics updated | **≤ 120 ms** after the 180 ms debounce | Below the threshold where an error feels like it belongs to the keystroke that caused it. |
| B2 | Keystroke → re-engraved score visible, large case | **≤ 400 ms**, with the previous engraving continuously visible | Above this, a composer stops trusting the page and starts waiting for it. |
| B3 | Click → selection halo drawn | **≤ 16 ms** (one frame), no round trip | Selection is the most frequent action in the app. |
| B4 | Selection → inspector fully populated | ≤ 100 ms | The enriching round trip may lag the halo, but not visibly. |
| B5 | Playhead jitter | **< 1 frame** deviation from linear between engine position events | A stepping playhead is worse than none (`03-interaction.md` §4). |
| B6 | App launch → shell painted | ≤ 400 ms | Before the score; the frame must never wait on Verovio. |
| B7 | App launch → score painted, small case | **≤ 1.5 s** cold, ≤ 600 ms warm | The first impression of the whole product. |
| B8 | Zoom step → re-laid-out page | ≤ 250 ms, previous page visible throughout | Zoom is a full Verovio layout at a new page size, so it cannot be a frame; it can be quick enough that the composer reads it as the same page, larger. |
| B9 | Origin view enter/leave | ≤ 120 ms, no layout reflow | It is an ink change; it must cost like one. |
| B10 | Idle CPU with playback stopped | **≈ 0 %** — no polling timers, no rAF loop | An editor that heats a laptop while nothing happens will not be used. |
| B11 | New performance → the new reading drawn (prompt 76) | ≤ 250 ms from the snapshot, previous page visible throughout | Measured apart from B2 on purpose: B2 is what an *edit* costs and its 400 ms includes the 180 ms typing debounce, which a click never pays. Folding the two together would hide a slow redraw behind a wait it does not do. |
| B12 | Choosing a piece already opened this session → its page drawn (prompt 85) | **≤ 400 ms**, previous page visible throughout | Turning to a piece you have already opened is turning back, not reopening: the session is still in memory and nothing has to be read from disk. A piece opened for the first time is B7's cold number by construction and is not asserted here. |

## 2. How they are measured

- The UI ships a `perf` module that records marks for each budgeted interaction via `performance.measure`, behind a
  flag, off in release builds.
- A headless harness (Playwright against `tauri dev`, or against the Vite dev server with a stubbed IPC layer where a
  window is impractical) drives each interaction 20 times on both fixtures and asserts the p95. This runs as part of the
  UI test suite for the prompt that owns the surface, and the budget table above is the assertion table.
- Failures are reported as measurements, not as "slow". A regression that misses B2 by 40 ms says so.
- **One gesture per trial.** The engraver marks `musa:score` for every page that arrives, and that includes the
  neighbour pages the observer renders in the background (`02-engraving.md` §7). A trial must therefore settle before it
  starts and read the first mark *after* its own gesture; otherwise it either ends on the previous trial's background
  page and measures nothing, or inherits the layout that trial walked away from and measures two gestures as one. Both
  were happening in B8 — in strict alternation, so a quarter of every run was double-counted and p95, by construction,
  reported one of the doubles at 267–275 ms. Isolated, the same build measures 117–195 ms per step, p95 184–195 ms. B8
  asserts a floor as well as a ceiling for this reason: a step that measured nothing is not a fast step.
- These are **per-gesture** budgets. Stepping zoom again before the previous layout has finished queues a second full
  Verovio layout behind the first, and the second step costs roughly the sum. That is the honest cost of a gesture the
  composer made while the machine was still working on the last one, and no per-step number covers it; coalescing rapid
  steps would, and is not implemented.

## 3. The structural rules the budgets imply

These are design constraints, not tuning, and they are the reason the budgets are reachable at all:

1. **Verovio in a worker, persistent, cancel-aware, generation-tokened** (`02-engraving.md` §2). B2, B6, B7, B8 are
   unreachable otherwise.
2. **Selection is applied locally from the DOM id** (`03-interaction.md` §2). B3 is unreachable via IPC.
3. **No polling.** Transport position arrives as an engine event; snapshots arrive as an event after `apply`. The
   frontend runs a `requestAnimationFrame` loop **only while playing** and cancels it on stop. B10.
4. **One debounce, one compile.** Keystrokes debounce at 180 ms and produce exactly one `apply_source`; a compile in
   flight is superseded rather than queued.
5. **Page virtualization** (`02-engraving.md` §7) — memory, not latency, but it is what keeps B8 true at page 200.
6. **No `filter`, `backdrop-filter`, or `box-shadow` animation** anywhere. The single leaf shadow is static. These are
   the standard causes of an application that scrolls at 30 fps.

## 4. What the compiler contributes (prompt 127)

B1 and B2 are end-to-end numbers, and the compile inside them is measured separately in
`docs/rules/language/06-elaboration-baseline.md`. Recorded here so a missed budget can be attributed rather than guessed
at:

| workload | compile (P1) | share of B1's 120 ms |
| --- | ---: | ---: |
| `examples/glass-mountain.musa` (the small case) | 159 µs | 0.1% |
| `tests/fixtures/large-score.musa` (the large case) | 4.4 ms | 3.7% |
| an edit in the first bar of the large case | 4.5 ms | 3.8% |
| an edit in the last bar of the large case | 4.5 ms | 3.8% |
| a 16.5 KB document mid-keystroke, recovering | 171 µs | 0.1% |
| the heaviest committed workload (`events-pressure`) | 9.6 ms | 8.0% |

Compilation is therefore not what B1 and B2 spend their time on; engraving is, and the structural rules in §3 are what
keep that true. Two consequences are worth stating because they are what a future miss should be checked against first:

- **The compiler is not incremental, and the measurement says it does not need to be.** An edit in the first bar and an
  edit in the last bar of a 100-bar score cost the same 4.5 ms, which is what a full recompile costs. §5's condition for
  reconsidering that — a measured B1 or B2 failure on a real piece — has not been met.
- **The point query is not a compile.** Asking which name is under the cursor costs 4.6 ns against a compilation the
  session already has. Hover and completion never sit behind the debounce.

## 5. What is deliberately not optimized

Per roadmap §17, no speculative optimization. Compilation stays synchronous inside `ProjectSession` (prompt 19's
decision); a worker-thread or incremental (Salsa) compiler is considered only when B1 or B2 is measured to fail on a
real piece, and it becomes its own prompt with the measurement as its justification.
