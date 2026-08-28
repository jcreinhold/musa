# 10 — Audio performance closure

Status: **descriptive measurement record for prompts 191 and 211**. The semantic contracts remain
[`08-performance-and-sound.md`](08-performance-and-sound.md),
[`../events/07-backend-contract.md`](../events/07-backend-contract.md), and the across-stage machine calculus. A timing
cannot weaken them.

## Method and machine

Measurements were taken on 2026-08-25 on an Apple M4 Pro (12 cores, 24 GB), arm64 macOS 26.6.1, rustc/cargo 1.98.0.
Divan reports release-build medians and allocator activity. UI numbers are p95 over 20 Playwright trials against the
stubbed Vite shell. Callback distribution times 1,000 consecutive 128-frame calls with `Instant`; its timer overhead is
therefore included. Scheduler noise and thermal state make the timing rows local evidence. Allocation counts and checked
semantic results are exact.

The fixed corpus identities were:

| input | SHA-256 |
| --- | --- |
| `tests/fixtures/audio-bridge.musa` | `3b1e90b251fb08a509610d9ad16bfa90b1080722b706c291a4b72e05b09dfa32` |
| `stdlib/src/sound/instrument.musa` | `d465410563aa172ffb3ff7a8350b9c03c1dbd825e43bfcbb5240c757ac3f83f8` |
| `stdlib/src/sound/sample.musa` | `21fb91ca1e336aa32c4ce0c62503b840061ea16caf948914a6b434855b444d17` |

The benchmark owns generated source for the other two workloads, so source construction is outside the timed closure and
cannot silently drift independently of the harness:

| workload | scale |
| --- | --- |
| audio bridge | 12 gestures, 254,400 output frames, standard profile and instrument, routing/effects |
| dense native mix | 8 isolated instances, 512 gestures, 144,000 output frames |
| large sample map | 128 key/round-robin regions, 64 voices, one shared 2,048-frame verified WAV |

## Stage profile

`cargo bench -p musa-compiler -- p1_compile p2_elaborate` separates source compilation from the DSP projection. The
selected medians are:

| stage | workload | median | maximum live | allocations / allocated bytes |
| --- | --- | ---: | ---: | ---: |
| P1 compile | audio bridge | 5.361 ms | 168 B | 4 / 168 B |
| P2 elaborate | audio bridge | 5.276 ms | 49.73 KB | 4,354 / 203.4 KB |
| P1 compile | large score | 115.0 ms | 168 B | 4 / 168 B |
| P2 elaborate | large score | 134.3 ms | 5.101 MB | 232,545 / 12.54 MB |
| P1 compile | events pressure | 9.692 ms | 168 B | 4 / 168 B |
| P2 elaborate | events pressure | 10.15 ms | 496.9 KB | 38,695 / 2.143 MB |

The P1 seam reuses the benchmark's prepared fixture and reports only retained work inside `compile`; P2 is the row that
shows elaboration allocation. Neither row is relabelled as source-profile cost. `interpret_performance_source` measures
the already-compiled score's source-declared profile/control realization separately: **11.71 ms** median, 143 maximum
live allocations/24.31 KB, and 609 total allocations/54.51 KB.

Preparation and rendering then measure different closures:

| operation | median | observed range | maximum live | allocations / allocated bytes |
| --- | ---: | ---: | ---: | ---: |
| prepare audio bridge | 407 µs | 394.2–487.4 µs | 519 / 179.9 KB | 1,022 / 203.7 KB |
| prepare dense native mix | 24.97 ms | 24.81–25.38 ms | 10,748 / 1.079 MB | 30,461 / 2.529 MB |
| prepare 128-region sample map | 23.91 µs | 21.54–39.33 µs | 134 / 82.32 KB | 135 / 82.34 KB |
| render audio bridge | 45.73 ms | 45.34–46.09 ms | 2 / 2.129 MB | 2 / 65.53 KB |
| render dense native mix | 38.62 ms | 37.97–39.30 ms | 2 / 2.129 MB | 2 / 65.53 KB |

The two render allocations are the offline output collector. The native graph, sample runtime, media reader, playback
install/retire path, and their destruction handoff retain dedicated zero-allocation/zero-lock laws. The dense callback
distribution measured p95 42.875 µs, maximum 67.334 µs, deadline 2.667 ms, and 0/1,000 misses. Existing sampler laws
exercise round-robin, release, pedal, choke, selection, high polyphony, and deterministic stealing without publishing
voice allocator state merely so a report can count it.

## Comparison and the one intervention

Prompt 93 rendered 240,000 frames in 18.26 ms. The current comparable render has 254,400 frames and takes 45.73 ms: 2.36
times the normalized time per frame. This is a material expected change, not noise. Prompts 171–180 replaced
host-block-defined modulation and feedback with the governing one-frame step, added exact source profile/control
mapping, and isolated each part's instrument instance. Those are semantic repairs; the old block loop is not a valid
optimization target.

The measured dense loop did reveal one true no-op: for almost every frame, each lane passed an empty event slice to a
function that scanned the full processor schedule. Returning immediately for an empty slice changed the audio-bridge
median from 46.99 to 45.76 ms (−2.6%) with allocation counts and output bytes unchanged. Repeated runs after the change
landed at 45.73–46.17 ms. No batching, SIMD, parallelism, layout exposure, or alternate evaluator was introduced.

## R1 and conditional frames

`audio_performance_laws` states each pair and premise in executable form:

| experiment | held equal | varied | outcome |
| --- | --- | --- | --- |
| R1 repeat | gesture tracks, instrument/machine binding, studio, seed, every option | nothing | decisions, extent, format, and the first 2,048 frame bit patterns equal |
| lineage separation | performed gesture tracks and all execution arguments | title position, source offset, meter, key, lineage | lineage differs; preparation observation is bit-equal |
| option axes | tracks and bindings | tail, tuning, sample rate, rounding, resource bound, maximum extent one at a time | each changes its named result or stable refusal only |
| binding axis | gesture track and options | source instrument gain | frames differ |
| conditional frames | allocation/start state, input history, parameters, seed, primitive implementation | host partition | exact `f32::to_bits` equality for native controls, feedback, sampler, and media laws |

Exact bits are promised here because every compared run uses the same target, primitive implementation, summation order,
and one-frame step. The language does not generalize that result across targets or substitute processors; those
comparisons require the processor's named tolerance.

There is currently **no prepared-execution cache**. Every edit recompiles and prepares a fresh machine, while checked
standard vocabulary uses a separate process-local source projection cache that is not a preparation result. Therefore
cold and warm execution preparation are identical operations, eviction cannot affect them, and no digest candidate can
false-hit. The measured 24.97 ms preparation does not justify adding mutable cache state. If one is later measured
necessary, `ExecArgs` must frame the operation version, exact gesture bytes/schema, machine and instrument bindings,
locked assets/packages, seed, format, schedule/batching policy, bounds, and options; candidate lookup must inject the
governing deliberate-collision law before shipping.

Preloading also meets every current workload and its explicit memory bounds. Streaming is not admitted. Malformed SFZ,
SoundFont, WAV, package, digest, and resource-limit cases remain outside timed success samples and inside their bounded
law suites.

## Stem taps beside the mix

Prompt 211 required the choice between one traversal and one render per output to be measured rather than argued.
Measured on 2026-08-28 on the same Apple M4 Pro (12 cores, 24 GB), arm64 macOS 26.6.2, rustc/cargo 1.98.0, by
`cargo bench -p musa-dsp --bench stem_render`. The benchmark owns its generated source, so the workload cannot drift
independently of the harness:

| workload | scale |
| --- | --- |
| routed mix | 8 parts, 4 shared returns, 512 gestures, 12 taps, 144,000 output frames |

| operation | median | observed range | maximum live | allocations / allocated bytes |
| --- | ---: | ---: | ---: | ---: |
| render the mix alone | 38.69 ms | 38.38–39.21 ms | 2 / 2.129 MB | 2 / 65.53 KB |
| render the mix and 12 stems in one traversal | 46.03 ms | 45.12–46.62 ms | 29 / 14.97 MB | 29 / 14.97 MB |
| render 13 outputs, one pass each | 504.4 ms | 503.1–508.1 ms | 2 / 14.91 MB | 26 / 851.9 KB |

One traversal costs **19% more than the mix alone** and **10.9 times less than a pass per output**, because the graph is
walked once for thirteen reads of it rather than thirteen times for one read each. That settles the design's "measure
before choosing" in favour of the single traversal, which is also the shape that makes alignment structural: a tap is
read from the buffer the same frame wrote, so no stem can drift from the master or from another stem.

Memory is the output itself and nothing else. Thirteen stereo f32 outputs of 144,000 frames are 14.976 MB, and the
measured 14.97 MB maximum live is that number: the collectors are reserved at their final size, the per-tap scratch is
one stereo frame, and no decoded asset is stored a second time. The separate-pass shape holds the same audio in less
transient allocation but thirteen independent graph states, which is what its 26 allocations and 10.9× time are.

The 29 allocations are the thirteen output collectors, their thirteen `Vec` headers, and the tap list; the render itself
still allocates nothing per frame. The callback path is untouched — `step_with_taps` is `step` plus one read per tap of
a buffer that frame already wrote — and the mix it returns is byte-for-byte the mix `render_offline` produces, which is
a law rather than an observation (`suite::stem_tap_laws`).

## Interface and ownership result

The prompt 191 Playwright run passed 191/191 tests. Its measured p95 values were B1 2 ms, B2 253 ms (2 ms stubbed round
trip, 151 ms engraving), B3 0 ms, B4 1 ms, B6 44 ms, B7 215 ms, B8 186 ms, B9 1 ms, B11 15 ms, and B12 14 ms. Sound and
Mix edits use B1's same source→snapshot path and their screen laws prove that controls and sends rewrite `.musa`.

The optimization added no public item. `musa-dsp` still exposes one opaque `PreparedAudio` operation/result boundary;
processor schedules, automation slots, decoded PCM, allocator state, and callback timing remain private. The checked
standard-library projections remain read-only and retain their exact source bytes. The module audit is therefore a
surface check, not a reason to expose a benchmark seam.

Reproduction commands are the prompt's Check plus:

```sh
cargo bench -p musa-compiler -- p1_compile p2_elaborate
cargo bench -p musa-dsp --bench audio_bridge
cargo bench -p musa-dsp --bench sampler
cargo bench -p musa-dsp --bench stem_render
cargo nextest run -p musa-dsp audio_performance_laws
```
