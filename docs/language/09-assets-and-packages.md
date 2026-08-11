# Assets, packages, and recorded media

External sound does not weaken source authority. A reproducible Musa project is the source plus an immutable, locked
build closure. Filesystem paths are authoring addresses; content identities are compilation facts.

## 1. Project build closure

For root project `P`, the build closure is:

```text
Closure(P) = {
  normalized Musa source documents,
  exact package graph,
  raw asset identities and declared adapter options,
  compiler/standard-library version,
  target-relevant deterministic render options
}
```

Every entry has a canonical path within its owning project/package and a cryptographic content digest. An `AssetId` is
`(digest algorithm, raw-byte digest, media kind, adapter-version, canonical adapter options)`. Hashing raw bytes catches
same-name changes; adapter identity catches semantic changes in decoding or mapping. Decoded caches are disposable and
verified against `AssetId`.

`musa.lock` records the exact package commits, package tree digests, transitive edges, asset digests, and adapter
versions. It is committed source-adjacent build metadata, generated deterministically and never edited by the sound UI.
A missing or mismatched locked object is an error, not an invitation to use whatever exists at the path.

Local paths resolve relative to the declaring file's package root after lexical normalization. Escaping that root,
following a symlink outside it, or relying on case-fold collisions is rejected. Absolute user-machine paths may be used
only by an explicit non-reproducible scratch command and cannot produce a release artifact.

## 2. Exact-pinned packages without a solver

The roadmap's relative-import-only rule remains governing until prompt 142. This candidate extends it in prompts 131–132
with a fetch layer, not a package ecosystem or dependency solver. Project syntax is:

```toml
[packages]
orchestra = { git = "https://example.org/musa/orchestra.git", rev = "sha1:8f42000000000000000000000000000000000000" }
```

Source resolves a package path explicitly:

```musa
import "pkg:orchestra/instruments/strings.musa";
```

A remote package is the package shape `04-templates-and-modules.md` fixes — `musa.toml`, a source root, and a `mod` tree
— fetched by exact pin rather than bundled. Prompt 132 adds the fetch layer and the lockfile and no second notion of
what a package is.

`rev` is a full immutable commit object ID with explicit `sha1:` or `sha256:` algorithm. Branches, tags, version ranges,
“latest,” registries, and implicit network lookup are rejected. `musa fetch` materializes and verifies the exact tree in
a content-addressed project cache and writes `musa.lock`. Ordinary `build`, `check`, `play`, and `render` are offline
operations over the lock and cache. They never fetch.

Packages may name their own exact-pinned packages. Resolution is graph collection, not constraint solving: two edges
using the same package name/URL at different commits are a conflict diagnostic unless the root assigns distinct aliases.
Cycles are rejected. Git submodules, executable build scripts, and native plugins are outside the initial package
format. Package Musa code has the same totality and resource rules as local code. The initial fetch command supports
public sources only; authenticated/private sources produce an explicit unsupported diagnostic rather than inspecting
credential files, environment secrets, browser state, or an ambient agent.

This design gives online reuse, auditable revision pinning, and offline builds while retaining the roadmap's “no
registry and no generalized version solver” decision.

## 3. Sampled instruments are note-driven

A sampled instrument implements an instrument signature. Score pitch, note identity, expression, emphasis, sustain,
techniques, and release choose and drive sample regions:

```musa
instrument solo_strings from "pkg:orchestra/solo-violin.sfz"
    conforms note_instrument;
```

The semantic object is a checked `SampleMap`: immutable audio regions plus predicates over typed gestures, selection
priority, pitch/root mapping, loop/release behavior, and deterministic variation state. The runtime carries per-voice
sample position/rate/state and obeys the same allocation, stealing, and RT laws as a synthesized implementation. For
score playback, any stochastic/round-robin choice becomes a selection token derived on the control side from the
realization seed and stable gesture identity and is recorded in realization provenance. Live input uses the same
control-side token service with a stable input ordinal. The audio callback consumes tokens and never draws randomness.

SFZ and SoundFont are interchange adapters, not Musa ontology. SFZ's text regions and opcodes compile into the supported
`SampleMap` subset; every unsupported opcode is diagnosed by name and policy. SoundFont's banks/presets compile through
a separate adapter to the same internal contract. Neither format's global defaults, MIDI numbering, modulation IDs, or
binary layout leaks into an instrument signature. Prompts 133–135 specify supported subsets, licensing metadata,
streaming/preload budgets, decoding, and conformance fixtures.

## 4. Three distinct recorded-media semantics

The same WAV bytes may participate in three different typed declarations. Musa never infers which one from file type.

### Note-driven sample

An `instrument ... from` region sounds because a note gesture selects it. Its onset, pitch/rate, release, and expression
come from the part lane. It is swappable with another conforming note instrument.

### Beat-fitted clip

A clip is media placed in performed musical time:

```musa
clip pulse from "assets/pulse.wav" fit 4/1 by rate;
cue pulse at 9:1;
```

The declaration records source duration `L` and target musical duration `D`. For `by rate`, at performed beat `b` within
cue onset `b₀`, normalized source phase is `(b-b₀)/D`; tempo changes alter its physical derivative, so the whole asset
remains fitted to the beat interval and pitch follows resampling rate. `by loop` instead plays at natural rate, restarts
as needed, and truncates at the beat end. `by crop` plays once at natural rate, truncating whichever of asset or beat
support outlasts the other and leaving any remaining support silent. There is no pitch-preserving warp promise. Loop
count is explicit: `cue pulse at 9:1 repeat 4;`. The cue elaborates to an interval `ScoreFact::MusicalClip` with exact
beat support and an opaque `AssetRef`; it is not a note. The kernel applies only its ordinary temporal laws and remains
opaque to the media reference and fit policy.

### Fixed-media cue

Fixed media is a recording placed at a musical onset but retaining physical duration:

```musa
fixed_media harbor from "assets/harbor.wav";
cue harbor at 17:1;
```

If the resolved onset beat is `b₀`, the tempo map is `θ : Beat → Second`, and decoded duration is `L` seconds, then:

```text
start(harbor) = θ(b₀)
end(harbor)   = θ(b₀) + L
```

A later tempo edit may move the onset in seconds but cannot stretch the recording or manufacture a musical end beat. The
cue elaborates to a point `ScoreFact::FixedMediaCue` containing Origin, an opaque `AssetRef`, and playback settings.
Kernel transforms may move, copy, or restrict that point; they do not stretch or reverse audio. Physical seconds, raw or
decoded samples, and fixed-media extent never enter `Term[ScoreFact]` or `Timeline[ScoreFact]`. The asset table beside
the score snapshot supplies `L` to performance preparation. This is the honest exception to “everything has a beat
duration,” useful for field recording and fixed-media/timeline practice (OMT
`098-twentieth-century-rhythmic-techniques.md`).

### Temporal action table

Kernel operations act only on occurrence support; the media reader remains forward unless source explicitly chooses a
playback-rate sign in a later specification.

| Operation | Note-driven sample | Musical clip | Fixed-media cue |
| --- | --- | --- | --- |
| sequence/shift | note gesture moves with its note | beat interval translates; source phase anchor translates | point onset translates; `L` unchanged |
| overlay | independent note voices | both media signals coexist | both media signals coexist |
| repeat | notes and deterministic realization choices duplicate with Origin | interval and phase mapping duplicate per iteration | point duplicates; each copy lasts `L` seconds |
| stretch by `r>0` | note support and gesture timing scale; pitch is unchanged | beat support scales by `r`; policy is reapplied to the new support | onset scales; physical duration and playback rate are unchanged |
| restrict `[i,j)` | ordinary note restriction policy | support intersects the window while retaining the original source phase, so a middle restriction does not restart | point survives iff its onset is in the window; a surviving recording is not cropped |
| retrograde in extent `d` | note supports relocate by the score law | interval relocates and source still plays forward | point moves from `b` to `d-b`; audio is not reversed |
| pitch transpose/invert | written note pitch changes before sample selection | no effect | no effect |

For fixed media, every listed temporal operation can change only `b₀`; substituting the new onset into `end=θ(b₀)+L`
proves physical-duration invariance. For a `by rate` clip, normalized phase depends only on the affine beat coordinate
in its transformed interval, proving that it reaches 0 and 1 at the transformed endpoints. Restriction must therefore
retain the original affine phase anchor; resetting it would violate the restriction law by changing audible content that
remained inside the window.

## 5. Routing recorded media

Sampled instruments produce their assigned part signal. Beat-fitted and fixed-media declarations produce named media
signals. They enter only the mix graph:

```musa
studio {
    route pulse -> master;
    room field_room { decay: 1.2 s; }
    send harbor -> field_room at -18 dB;
    route harbor -> master;
    route field_room -> master;
}
```

A media signal has no `PartId` unless it is a note-driven instrument. The mix cannot send it note gestures, retune it,
or expose its waveform as score data. Editing, trimming, denoising, and comping remain external; Musa may select an
immutable region by exact sample bounds declared in the asset adapter.

## 6. Offline and real-time preparation

All packages are resolved, assets verified, formats parsed, sample regions indexed, and required media preloaded or
bound to bounded lock-free streaming buffers before playback. Capacity and missing-data errors occur on the control
side. A stream underrun follows one documented engine diagnostic/recovery policy; it never performs file I/O on the
callback to catch up.

Offline rendering uses the same prepared instruments, media lanes, mix graph, and render operation as live playback. The
lockfile, adapter algorithms, realization choices, sample rate, and deterministic render options are sufficient to
reproduce promised offline bytes. Platform decoders that cannot meet this contract are converted to a canonical cached
PCM representation during fetch/preparation and identified in the closure.

## 7. Security, licensing, and diagnostics

Packages are data and total Musa source: no fetched code executes outside the compiler's language semantics. Diagnostics
name package alias, commit, canonical path, digest, and importing span. Asset declarations may carry SPDX license and
attribution metadata; release tooling reports missing metadata but does not invent legal conclusions.

The required errors include: unlocked package, unavailable offline object, digest mismatch, path escape, import cycle,
alias/revision conflict, unsupported sample-map feature without declared fallback, undecodable media, channel/rate
contract failure, and preparation-budget excess. None degrades silently to the default oscillator or silence.

## 8. Reproducibility laws

1. **Closure determinism:** collecting the same root files and lock yields the same ordered closure.
2. **Content addressability:** changing any source/asset byte changes its corresponding identity and semantic cache key.
3. **Path irrelevance:** relocating a complete project without changing canonical internal paths preserves the closure.
4. **Offline closure:** after successful `fetch`, every semantic build and render preparation succeeds with networking
   disabled or fails only for an input/environment condition also present online.
5. **Adapter determinism:** one locked adapter and options compile identical bytes to an equivalent internal map/media.
6. **Media distinction:** tempo transformation changes beat-fitted physical playback but not fixed-media duration;
   changing instrument assignment affects note-driven samples but not clip or fixed-media lanes.

Prompts 131–137 implement these laws; prompts 140–142 measure preparation cost and audit deterministic artifacts.
