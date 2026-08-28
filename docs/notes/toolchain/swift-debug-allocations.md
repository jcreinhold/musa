# Swift allocates in a Debug render block, and it is not the component

**Status: operational. Governs nothing.**

## What surprises you

`apps/musa-audio-unit`'s harness measures the render block with an interposing `malloc` counter and asserts zero.
Building it the obvious way, that assertion fails by a small, perfectly regular amount: two allocations per block of 512
frames, on a component whose Rust side the `musa-au` suite has already proved allocates nothing.

Neither number is the component's.

## What was measured

macOS 26.6.2 (25G83), Xcode 26.6 (17F113), arm64, `-configuration Debug` (`SWIFT_OPTIMIZATION_LEVEL = -Onone`).

An empty Swift loop, inside the armed counter and calling nothing:

```swift
for round in 0..<rounds { sink = sink &+ UInt64(round) }   // rounds = 2000 → 2000 allocations
```

One allocation per iteration. `Range` iteration is a generic conformance, and at `-Onone` nothing specializes it, so
each `next()` goes through boxed existential machinery that reaches the heap. The same loop under `-O` allocates
nothing, which is exactly why this is easy to miss: the number appears in the build you debug in and vanishes in the
build you ship.

Two consequences in this repository:

1. **The render block.** `for index in 0..<buffers.count` over an `AudioBufferList` of two buffers cost two allocations
   per call. It is written as `var channel = 0; while channel < buffers.count { … }` for that reason, and the comment in
   `MusaInstrumentAudioUnit.swift` says so. A render block that is real-time only in Release is not real-time: it is the
   Debug build a developer runs under a host all day.
2. **The rig.** `EventList.first` is `storage.first.map { UnsafePointer($0) }`, and `Optional.map` with a closure costs
   two allocations at `-Onone`. Called *inside* the measured region it read as a component cost; the harness now
   resolves the event-list head before arming the counter.

## How to tell which one you are looking at

Measure a loop that calls nothing, with the same counter, in the same build. If it is nonzero, the rig is the
measurement. Prompt 215 found two such artefacts before it found the truth
([`../research/93-the-audio-unit-shape.md`](../research/93-the-audio-unit-shape.md) §2), and the discipline that comes
out of it is to publish the empty-block baseline beside every number — which `rt.allocation.baseline` does, and
`scripts/check-audio-unit.py` refuses to accept any other number without.
