---
id: 203
slug: daw-export-bundles
status: pending
depends_on: [202]
phase: 4
---

# Export Portable Logic and GarageBand Bundles

## Task

Package Musa's existing open exports and prompt 202's aligned audio into one deterministic, inspectable directory for
Logic Pro or GarageBand. Give CLI and desktop users a single export action while keeping every constituent artifact and
loss explicit.

## Read

- Prompts 19, 28, 32, 43, 67, 182–183, 189, 201–202; the project export/cache facade and desktop Export submenu.
- Apple's current Logic MIDI/MusicXML import and GarageBand audio/MIDI import guides cited by prompt 201. Confirm file
  and track behavior against current primary documentation when executing this prompt.
- The current Standard MIDI, MusicXML, WAV, provenance, semantic identity, asset lock, and atomic-file-writing code.

## Design

Add one project facade such as `export_daw_bundle(target, destination, options) -> DawExportReport`; names may follow
the established project vocabulary, but the operation must hide compilation, realization, scheduling, constituent
exports, hashing, staging, and cleanup. It writes a sibling temporary directory and atomically installs the completed
bundle without partially replacing an existing destination.

The version-1 bundle contains:

- `score.mid` and `performance.mid`, kept distinct exactly as roadmap §12.5 requires;
- `score.musicxml` for Logic's notation route, with its absence in the GarageBand profile stated rather than disguised;
- `audio/mix.wav`, `audio/parts/*.wav`, and `audio/returns/*.wav` from one complete render argument record;
- `musa-manifest.json`, a deterministic schema carrying the complete identities/options, target, stable part/control
  mappings, file digests, alignment/tap facts, import guidance, and machine-readable losses; and
- an origin sidecar mapping exported MIDI tracks/events, MusicXML ids where available, and stem identities back to Musa
  origins without making that sidecar canonical.

The target profile chooses only documented packaging and guidance. It does not change musical realization to suit a
host. Logic users are told to choose MusicXML for editable notation or performance MIDI for production—not import both
as though they were one track. GarageBand receives the MIDI/audio forms it documents. Both profiles report polytempo,
polymeter, tuning, channel, controller, and notation losses from the producing exporters.

Use stable ordered JSON and content digests. A repeated export from equal complete inputs is byte-identical except for
filesystem metadata outside the bundle. Do not put wall-clock timestamps, absolute source paths, usernames, or random
UUIDs in files. The source and lock closure are identified, not copied into the bundle unless the user separately asks
for an archive in a later prompt.

CLI: `musa export --to daw --profile logic|garageband --output DIR` (adapt spelling to the established command parser).
Desktop: one Export-submenu item with a target choice, destination picker, progress/cancel behavior, and a completion
report listing files and losses. The UI forwards project facts and does not rebuild the manifest in TypeScript.

## Target

- Project/CLI/desktop DAW-bundle export with versioned Rust/IPC report types and generated TypeScript bindings.
- Golden bundle fixtures for a solo score, a routed multipart score, controlled performance, media/tails, and every
  significant loss class; parse every emitted MIDI/MusicXML/JSON/WAV artifact in tests.
- Atomic replacement/cancellation/error cleanup, cache correctness, deterministic byte laws, and path-adversary tests.
- A concise Logic Pro and GarageBand import how-to linked from the export completion state.

## Check

```sh
cargo nextest run -p musa-notation -p musa-project -p musa -p musa-desktop
cargo clippy --all-targets -p musa-notation -p musa-project -p musa -p musa-desktop -- -D warnings
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
cargo insta test --workspace --unreferenced=reject
cd apps/musa-desktop/ui && npx pnpm run check && npx pnpm run test:unit
```

Commit as `Export portable DAW bundles`.

## Stop

- No `.logicx`/`.band` generation, DAW launch/automation, import/round-trip, network upload, or project archive.
- No new MIDI, MusicXML, or audio semantics; repair the owning exporter if a constituent artifact is wrong.
- No live MIDI, synchronization, Audio Unit, recording, waveform editing, or mastering.
