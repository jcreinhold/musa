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

Prompt 218 added a third and a fourth, both about a component that *emits*:

3. **A block passed down a second Swift frame is copied.** `MusaAuDrive.h` records this in one direction — a Swift
   closure handed to an Objective-C block parameter is copied per call. It holds in the other direction too: a helper
   that takes `AUMIDIOutputEventBlock` as a parameter and hands it to *another* helper pays one allocation per call.
   `MusaProcessorAudioUnit.swift`'s release path builds its packet where the block is already captured, and its comment
   says why.
4. **A Swift closure installed as `midiOutputEventBlock` allocates per call**, so a harness that measures a component
   through one is measuring its own bridging thunk. `MusaAuMidiSink` in `MusaAuDrive.h` is that sink written in
   Objective-C, keeping a bounded record without allocating, for the same reason `MusaAuDriver` is.

## The counter counts the process, not the thread

`AllocProbe.c` interposes `malloc` for everyone. Measuring while the section's own components are still being torn down
— a dozen built and let go, each with a worker still retiring what it prepared — counts their allocations as the render
block's, and the number moves between runs by a hundred either way. `processor.rt.noAllocation` waits before it measures
and publishes an idle driver's number beside the component's; a contaminated run then says so instead of failing
mysteriously.

## How to tell which one you are looking at

Measure a loop that calls nothing, with the same counter, in the same build. If it is nonzero, the rig is the
measurement. Prompt 215 found two such artefacts before it found the truth
([`../research/93-the-audio-unit-shape.md`](../research/93-the-audio-unit-shape.md) §2), and the discipline that comes
out of it is to publish the empty-block baseline beside every number — which `rt.allocation.baseline` does, and
`scripts/check-audio-unit.py` refuses to accept any other number without.
