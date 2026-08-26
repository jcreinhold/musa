---
id: 219
slug: daw-integration-closure
status: pending
depends_on: [212, 214, 217, 218]
phase: 4
---

# Close Logic and GarageBand Integration Against Real Hosts

## Task

Audit the complete DAW boundary against governing identity/loss laws, measured real-time budgets, reproducible host
fixtures, and the current documented behavior of Logic Pro and GarageBand. Finish the user path from a Musa project to
open files, live MIDI, synchronized transport, and Audio Units without claiming an unsupported round trip.

## Read

- Prompts 210–218 and every completion report/repair they produced; prompts 43, 67, 173–174, 191–193.
- Current Apple primary documentation cited by prompt 210 plus the SDK/Xcode/host versions recorded by prompt 215.
- All bundle manifests/losses, CoreMIDI schedules/sync state, AU state/parameter/bus mappings, project cache identities,
  RT instrumentation, and the book's export/playback/sound paths.

## Design

Build one conformance matrix with rows for source/lock identity, seed/profile, score/performance MIDI, MusicXML, master
and every stem tap, frame alignment/tails, CoreMIDI part/channel mapping, clock authority, AU instrument frames,
parameter automation, output buses, MIDI Processor scheduling, provenance, state restoration, and every declared loss.
Columns name Logic Pro, GarageBand, automated host harness, offline/native reference, and unsupported-by-design. A green
cell links evidence; a documented limitation is not colored green by omission.

Use checked-in small projects covering multipart routing and returns, tempo/meter changes, polytempo/polymeter,
microtonal/control losses, deterministic choice, sampled assets, media tails, renamed/colliding identities, seek/loop,
offline bounce, and source/package/asset invalidation. Generated open artifacts may be fixtures; proprietary host
documents may not be committed or treated as golden truth.

Automate every layer the repository controls: parse constituent files, compare identities/digests, run a fake CoreMIDI
clock/endpoint, instantiate both AU components, render and compare frames/events, save/restore state, and validate with
`auval`. When current Logic Pro or GarageBand is installed, execute a versioned manual protocol and record observations;
the automated host harness remains the CI contract so commercial app availability cannot make the suite dishonest.

Measure bundle throughput/peak memory, live MIDI jitter/drop behavior, sync drift/relock, AU preparation, render
max/p95/deadline misses, state size/restore latency, parameter event density, bus count, and plan retirement. Preserve
prompt 191's method and the no-allocation/no-lock/no-I/O/no-log callback proof across Swift and Rust. Optimize only a
measured failure and update the governing budget or compatibility table only with evidence.

Teach three honest workflows: import open files/stems into Logic or GarageBand; connect live CoreMIDI with one
authority; and use the Music Device (plus Logic-only MIDI Processor) in a host. Each page states what remains canonical,
how to recover the exact Musa source/lock identity, where DAW automation lives, and which changes do not round-trip.

## Target

- Complete conformance/performance/compatibility reports and generated fixture corpus, with every prompt-210 law traced
  to an automated or explicitly manual check.
- Focused correctness/performance repairs within prompts 211–218's existing boundaries; repair a governing conflict or
  public-boundary change before implementing it.
- Diátaxis-aligned book tutorial, Logic/GarageBand how-tos, conceptual DAW-boundary explanation, reference pages for
  CLI/bundle schema/losses/MIDI sync/AU state and parameters, and troubleshooting.
- Final code-map, roadmap, prompt overview, app navigation, installer-development notes, and compatibility table
  updates.

## Check

```sh
cargo nextest run --workspace --run-ignored all
cargo clippy --workspace --all-targets -- -D warnings
xcodebuild -project apps/musa-audio-unit/MusaAudioUnit.xcodeproj -scheme MusaAudioUnit -configuration Release CODE_SIGNING_ALLOWED=NO test
scripts/check-daw-integration.sh
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
make docs-check
cargo deny check
cargo insta test --workspace --unreferenced=reject
cd apps/musa-desktop/ui && npx pnpm run check && npx pnpm run test:unit
python3 scripts/renumber-prompts.py audit
```

Commit as `Close Logic and GarageBand integration`.

## Stop

- No publishing, signing/notarization credentials, App Store submission, installer release, or automatic DAW launch.
- No `.logicx`/`.band` writer, DAW import/round-trip, source mutation from automation, recording, waveform editor,
  mastering suite, or third-party plug-in hosting.
- No claim about an untested host/version and no weakening a governing law to turn a red matrix cell green.
