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
| B1 | Keystroke → diagnostics updated | **≤ 120 ms** after the 100 ms debounce | Below the threshold where an error feels like it belongs to the keystroke that caused it. |
| B2 | Keystroke → re-engraved score visible, large case | **≤ 400 ms**, with the previous engraving continuously visible | Above this, a composer stops trusting the page and starts waiting for it. |
| B3 | Click → selection halo drawn | **≤ 16 ms** (one frame), no round trip | Selection is the most frequent action in the app. |
| B4 | Selection → inspector fully populated | ≤ 100 ms | The enriching round trip may lag the halo, but not visibly. |
| B5 | Playhead jitter | **< 1 frame** deviation from linear between engine position events | A stepping playhead is worse than none (`03-interaction.md` §4). |
| B6 | App launch → shell painted | ≤ 400 ms | Before the score; the frame must never wait on Verovio. |
| B7 | App launch → score painted, small case | **≤ 1.5 s** cold, ≤ 600 ms warm | The first impression of the whole product. |
| B8 | Zoom step → re-laid-out page | ≤ 250 ms, previous page visible throughout | Zoom is a full Verovio layout at a new page size, so it cannot be a frame; it can be quick enough that the composer reads it as the same page, larger. |
| B9 | Origin view enter/leave | ≤ 120 ms, no layout reflow | It is an ink change; it must cost like one. |
| B10 | Idle CPU with playback stopped | **≈ 0 %** — no polling timers, no rAF loop | An editor that heats a laptop while nothing happens will not be used. |
| B11 | New performance → the new reading drawn (prompt 76) | ≤ 250 ms from the snapshot, previous page visible throughout | Measured apart from B2 on purpose: B2 is what an *edit* costs and its 400 ms includes the 100 ms typing debounce, which a click never pays. Folding the two together would hide a slow redraw behind a wait it does not do. |
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
4. **One debounce, one compile.** Keystrokes debounce at 100 ms and produce exactly one `apply_source`; a compile in
   flight is superseded rather than queued.
5. **Page virtualization** (`02-engraving.md` §7) — memory, not latency, but it is what keeps B8 true at page 200.
6. **No `filter`, `backdrop-filter`, or `box-shadow` animation** anywhere. The single leaf shadow is static. These are
   the standard causes of an application that scrolls at 30 fps.

## 4. What the compiler contributes (prompt 165)

B1 and B2 are end-to-end numbers, and the compile inside them is measured separately in
`docs/rules/language/06-elaboration-baseline.md`. Recorded here so a missed budget can be attributed rather than guessed
at:

| workload | compile (P1) | share of B1's 120 ms |
| --- | ---: | ---: |
| `examples/glass-mountain.musa` (the small case) | 0.92 ms | 0.8% |
| `tests/fixtures/large-score.musa` (the large case) | 110.9 ms | 92.4% |
| the heaviest non-large committed workload (`events-pressure`) | 7.27 ms | 6.1% |

The dependent checker makes compilation the binding half of B1 on the large case. Prompt 165 measured the full score at
272 ms after its new cost table first admitted it; preserving the balanced prefix tree's duration decomposition brought
that to 110.9 ms without omitting any bar claim. The UI budget run first measured B2 at p95 350 ms with a 180 ms
debounce, a 2 ms stubbed round trip, and 169 ms engraving. Composed with the real compile, that was 461 ms and therefore
a real B2 failure. The debounce is now 100 ms. The final twenty-trial run measured B1's post-debounce frontend round
trip at **2 ms** and B2 at **249 ms** p95, including a 2 ms stubbed round trip and 149 ms engraving. Substituting the
real 110.9 ms compile for the stub gives **360 ms**, below B2's 400 ms; the compiler itself remains below B1's 120 ms.
The previous engraving remains visible throughout.

Two consequences are worth stating because they are what a future miss should be checked against first:

- **The compiler is still not incremental, but it has spent the available large-case margin.** A further measured B1
  regression is now the condition for a worker-thread or incremental compiler; the old claim that compilation was only
  3.7% of B1 is retired by this measurement.
- **The point query is not a compile.** Asking which name is under the cursor costs 4.6 ns against a compilation the
  session already has. Hover and completion never sit behind the debounce.

## 5. What is deliberately not optimized

Per roadmap §17, no speculative optimization. Compilation stays synchronous inside `ProjectSession` (prompt 19's
decision); a worker-thread or incremental (Salsa) compiler is considered only when B1 or B2 is measured to fail on a
real piece, and it becomes its own prompt with the measurement as its justification.

## 6. Prompt 191 sound and mix closure

The prompt 191 run on the same machine measured B1 at **2 ms** after the debounce and B2 at **253 ms** p95, split into 2
ms of stubbed round trip and 151 ms of engraving. The current real large-score P1 median is **115 ms**, so substituting
it gives 368 ms from keystroke through visible engraving, still inside B2. Sound and Mix source edits use that same
one-debounce/one-snapshot path; their screen laws additionally prove that an instrument choice, exposed control, or send
rewrites source rather than a UI-owned graph.

Audio has a physical deadline rather than another perceptual number: a 128-frame callback at 48 kHz has 2.667 ms. The
eight-instance, 512-gesture workload measured p95 **42.875 µs**, maximum **67.334 µs**, and zero misses in 1,000
consecutive blocks. The complete method, scale variables, allocation measurements, and comparison are recorded in
[`../language/10-audio-performance.md`](../language/10-audio-performance.md). This is local evidence, not a portable
absolute threshold; the invariant remains that every callback completes before its device deadline without allocation,
locking, I/O, logging, or large destruction.
