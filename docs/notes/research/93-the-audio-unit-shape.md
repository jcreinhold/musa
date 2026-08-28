# The Audio Unit shape

**Status: governs nothing.** The measurement behind prompt 215, taken before any production code was coupled to Xcode.
[`docs/rules/across-stages/06-daw-boundary.md`](../../rules/across-stages/06-daw-boundary.md) §3–§7 states the boundary;
[`92-the-daw-boundary.md`](92-the-daw-boundary.md) says why it reads the way it does. This page says whether it can be
built at all on the machine it has to run on, and what the system refused.

Every claim here is **Measured** unless it says otherwise: it is the output of
[`scripts/check-audio-unit-trial.sh`](../../../scripts/check-audio-unit-trial.sh) on the machine and versions in §1,
reproducible by running that script. A claim the trial could not settle says `unsupported` and says why.

## 1. What was measured, and on what

|  |  |
| --- | --- |
| Host | arm64 macOS 26.6.2, build 25G83 |
| Toolchain | Xcode 26.6 (17F113), SDK `macosx26.5` |
| Trial | [`apps/musa-audio-unit-trial/`](../../../apps/musa-audio-unit-trial/), six targets, no Musa runtime linked |
| Command | `scripts/check-audio-unit-trial.sh` |
| Result | 38 findings passed, 1 unsupported, `auval` validated both components |

The trial is a containing app, two extensions, a shared framework, a command-line host harness, and an allocation probe.
It renders a fixed-point sawtooth from a 128-entry increment table and schedules eight fixed occurrences. It contains no
compiler, no DSP graph, and no rational time: it is there to falsify the *shape*, and a real synthesizer inside it would
only have made the failures ambiguous.

Two components, because §3 names two crossings that the system treats differently:

| Component | Type | Subtype | Manufacturer | What it proves |
| --- | --- | --- | --- | --- |
| Trial Instrument | `aumu` | `musi` | `Musa` | a Music Device driven by host MIDI |
| Trial Processor | `aumi` | `musp` | `Musa` | a MIDI Processor selecting from a finite schedule |

Both are ad-hoc signed (`codesign -f -s -`), registered with `pluginkit -a`, validated with `auval -v`, driven by the
harness, and deregistered by the script's `EXIT` trap. Nothing is left installed.

## 2. The render block

**Measured.** The block allocates nothing, and that number is only trustworthy because the rig was proved first.

| Finding | Result |
| --- | --- |
| `rt.allocation.hostBaseline` | 0 allocations across 2000 calls of an *empty* block |
| `rt.allocation` | 0 across 2000 blocks of 512 frames |
| `rt.allocation.events` | 0 across 2000 blocks carrying a MIDI event and a parameter ramp |
| `rt.timing` | worst 19.5 µs, mean 9.0 µs, against a 10666.7 µs budget |

Getting to zero took two corrections, and both are the finding rather than the anecdote:

1. **The harness's own buffer-list setup was inside the measured region**, and read as three allocations per block. A
   render measurement that includes the host's preparation is measuring the host.
2. **Calling an `AUInternalRenderBlock` from Swift copies the block on every call** — one allocation per call, and it
   showed up even for a block whose body was empty, which is what made it identifiable. The rig now holds the block in
   an Objective-C object (`MusaTrialDriver`) that copies it once at `init`.

*The consequence for production.* Any allocation instrumentation must publish its own baseline against an empty block
before it publishes a number for real work, or the first two allocations it reports will be its own. And the language
boundary is part of the render path's cost: an RT contract stated in terms of "the Swift code allocates nothing" is not
the same statement as "the block allocates nothing".

Three further render facts, each a rule §4 and §7 already imply and now has evidence for:

- `render.blockPartition` — one block of 1024 frames is sample-identical to four of 256. This is Theorem R1-batch
  holding at the host boundary, measured rather than assumed.
- `render.hostSuppliesNoMemory` — a host may hand the block an `AudioBufferList` with null `mData` and expect the
  component's own buffers. Preallocating those in `allocateRenderResources` is mandatory, not an optimization.
- `render.noncontiguousSampleTime`, `render.reset`, `render.offline`, `render.maximumFrames` — a jump to sample time
  9876543, a loop and a reset, an offline flag, and a drop from 2048 frames to 32 all produce the expected frames.

## 3. State, parameters, and buses

**Measured.**

- `state.keepsHostClassInfo` — **`fullState` may be extended but not replaced.** An override that returns only Musa's
  own keys fails `auval` with `Class Data does not have required field:<type> == componentType`. The working shape
  starts from `super.fullState`, merges Musa's keys onto it, and calls `super` in the setter before restoring.
- `state.documentCarriesClosure` / `state.noAssetBytes` — a property-list-safe document snapshot carrying version, ABI
  version, project/source/lock identity, source closure, selected declaration, asset *digests*, and the parameter
  address table survives the round trip. Asset bytes stay out.
- `state.refusesForeignClosure` — a component handed state made from another source identity refuses it and says both
  identities, rather than rendering something plausible.
- `state.crossesTheProcessBoundary` — all of that survives `.loadOutOfProcess`.
- `param.tree.projection`, `param.ramp`, `param.tree.replacement` — a two-parameter tree, a sample-accurate ramp, and a
  live replacement of the tree with a three-parameter one all behave.
- `bus.count`, `bus.multipleOutputs` — two output buses are negotiated and rendered, bus 1 frame-for-frame half of bus
  0.

`instantiate.outOfProcess`, `instantiate.relaunch`, and `instantiate.afterTermination` all pass: a second instance comes
up, and a fresh extension process comes up after the first is killed with `pkill`, with its parameter tree and buses
intact. The trial's `instantiate.inProcess` finding constructs the `AUAudioUnit` subclass **directly**, in the harness
process; it is not evidence that an *extension* can be loaded in-process, and no such evidence was taken.

## 4. The App Group refusal

**Measured, and negative.** This is the one thing the system would not do.

|  |  |
| --- | --- |
| A non-sandboxed process writes and reads `~/Library/Group Containers/group.dev.musa.audiounittrial/trial-asset.txt` | works (`sandbox.appGroup`) |
| A sandboxed extension calls `containerURL(forSecurityApplicationGroupIdentifier:)` and reports the name | works (`sandbox.extensionNamesAssetStore`) |
| A sandboxed extension **reads a file inside that container** | the component never reaches the host again |

The third row is precise about its failure. It is not an error, an exception, or a refusal the component can report:
`AVAudioUnit.instantiate(with:options:[.loadOutOfProcess])` simply never calls back — a 20-second deadline expires, no
crash report is written, and `log show` has nothing. Three variants were tried: the read in the component's initializer
(host never receives the component at all), the same read made `lazy` and triggered by `fullStateForDocument` (state
request returns nothing), and `FileManager.contents(atPath:)` in place of `String(contentsOf:encoding:)` (identical).
Merely *naming* the container is safe in every variant, which is what isolates the read as the cause.

**Judged, on that evidence.** A real App Group identifier is team-prefixed and provisioned by Apple; ad-hoc signing
cannot carry one, so this is a distribution-gated capability rather than a broken API. Two consequences, and the second
is the one that changes the design:

1. Any check that must pass on a machine without a Developer Team cannot depend on an extension reading its App Group.
   The trial reports `sandbox.extensionReadsAsset` as `unsupported` and does not attempt the read.
2. **A component should reach its assets through what the host restored, not through a container it goes looking for.**
   That is the better design independently of provisioning: a container lookup is the component asserting where the
   truth lives, and D1 says the host's restored document state is what names it. Prompts 216–218 were amended to say
   assets arrive by restored identity and host-granted access, with an App Group as an optional convenience behind a
   Development Team rather than the mechanism the contract rests on.

There is a third, blunter consequence for the component's initializer: **it must not touch the filesystem.** The eager
variant of this read did not fail the component, it made the component undiscoverable. Whatever a production initializer
wants to know, it learns lazily or from restored state.

## 5. What the trial did not settle

Stated plainly rather than left to be assumed:

- No Logic Pro or GarageBand observation. `auval` and the harness are the whole of the automated evidence; a claim about
  a specific host's workflow is not in this page and prompts must not cite this page for one.
- No Thread Sanitizer run, and no Audio Workgroup measurement. The allocation probe and the timing trace are what was
  taken.
- No in-process extension loading, no notarization, no distribution, and no Rust across the boundary — the trial's C ABI
  is a hand-written stand-in for the shape, not the production header.

## 6. The architecture this accepts

**Judged**, on §2–§4. Unchanged from the boundary's §3–§7 in substance, and now with the parts that were guesses
replaced:

- Compile, resolve, decode, and prepare on a control worker; publish an immutable prepared plan through a small
  versioned C ABI; consume only preallocated state and already-delivered render events in the block; retire old plans
  off-thread. A wait-free generation counter with release/acquire ordering is enough for the publication, and it stays
  importable from Swift.
- The Music Device is MIDI-driven and never seeks a composition. The MIDI Processor owns a finite random-access schedule
  and selects from it by host position: `processor.seek.doesNotReplay` measures the difference — from the beginning the
  search starts at index 0, and after a seek to beat 2 it starts at index 4, with no replay in between.
- The component's document state is the closure identity, and the component refuses a foreign one. Assets come by digest
  and by host-granted access.
- The block's budget is not tight. 19.5 µs worst against 10.7 ms leaves the whole of the real work still to be paid for,
  which is the useful reading of that number rather than a claim of speed.
