# Assets, packages, and recorded media

**Status: candidate.** External sound, reproducible assets, and pinned packages, without weakening source authority.

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

Source names an asset only where a musical declaration consumes it, such as an instrument's established `from` clause.
There is no parallel `asset` declaration and no host-authored source default:

```musa
instrument solo_strings from "assets/solo-violin.sfz"
    conforms note_instrument;
```

Physical policy and attribution are stated once in the project manifest under the same logical path:

```toml
[assets."assets/solo-violin.sfz"]
kind = "sfz"
adapter = "sfz@1"
max_bytes = 67108864
license = "CC-BY-4.0"
source = "https://example.org/solo-violin"
```

`kind` and the versioned `adapter` identity are required; the byte limit, license expression, and attribution text are
optional. The generated lock repeats the path, kind, adapter, and exact byte length and adds the `sha256:` raw-byte
digest. `musa assets lock <project>` is the only operation that writes this observation. `musa assets list <project>`
reports the ordered facts, and `musa assets verify <project>` checks them without writing. Ordinary checking, playback,
and export never repair the lock implicitly.

Local paths resolve relative to the declaring file's package root after lexical normalization. Escaping that root,
following a symlink outside it, or relying on case-fold collisions is rejected. Absolute user-machine paths may be used
only by an explicit non-reproducible scratch command and cannot produce a release artifact.

## 2. Exact-pinned packages without a solver

The roadmap's relative-import-only rule remains governing until prompt 183. This candidate extends it in prompts 182–183
with a locked asset/package closure and a fetch layer, not a package ecosystem or dependency solver. Project syntax is:

```toml
[packages.orchestra]
git = "https://example.org/musa/orchestra.git"
rev = "sha1:8f42000000000000000000000000000000000000"
```

Source imports a package module through the same module-path syntax used for `std`; the manifest alias is its first
segment:

```musa
import orchestra::instruments::strings;
```

There is no quoted `pkg:` module-import form. `pkg:orchestra/solo-violin.sfz` is instead the logical address of package
data where an existing source declaration expects an asset path, such as `instrument … from "…"`.

A remote package is the package shape `04-templates-and-modules.md` fixes — `musa.toml`, a source root, and a `mod` tree
— fetched by exact pin rather than bundled. Prompt 183 adds the fetch layer and the lockfile and no second notion of
what a package is.

Lock format 2 retains the root `[assets."path"]` records from §1 and adds `package_roots`, an ordered `packages` array,
and an ordered file array under each package. A root alias maps to a content-addressed node locator. Each node records
package name, Git URL, exact algorithm-tagged revision, declared source root, exact alias-to-node dependency edges, a
framed tree digest, and the complete canonical sequence of `(path, byte length, SHA-256 digest)` file facts. The cache
may use the node or tree digest to find a candidate, but verification compares the complete descriptor, edge map, file
fact sequence, and raw bytes. Digest equality alone is never package equality.

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
binary layout leaks into an instrument signature. Prompts 182 and 184–186 specify supported subsets, licensing metadata,
streaming/preload budgets, decoding, and conformance fixtures.

### 3.1 `sfz@1` support matrix

`sfz@1` is a deliberately small, version-labelled adapter. “Supported” means exact translation into the ordinary
`std::sound::sample` declarations and deterministic native execution; it never means that the union of SFZ v1, SFZ v2,
ARIA, LinuxSampler, and player extensions is one language. A sound-changing name outside this table is an error.
Harmless labels are warnings only when this table says so.

| Input | Published family | `sfz@1` policy |
| --- | --- | --- |
| `<region>`, `<group>` | SFZ v1 | supported, with region over group inheritance |
| `<global>` | SFZ v2 | supported as the one file-wide inheritance level used by common v1-compatible libraries |
| every other header | SFZ v2 / extension | error by header name |
| `sample` | SFZ v1 | supported for a relative, root-contained WAV path; generators and other codecs are errors |
| `key`, `lokey`, `hikey`, `pitch_keycenter` | SFZ v1 | supported for integer keys 0–127; named-note octave conventions and `pitch_keycenter=sample` are errors |
| `lovel`, `hivel` | SFZ v1 | supported; inclusive 0–127 values map exactly to normalized gesture expression |
| `tune`, `transpose` | SFZ v1 | supported as exact cents and semitones; explicit `pitch_keytrack` is accepted only at its default 100 |
| `volume`, `pan` | SFZ v1 | supported as exact decimal dB and the exact `[-100,100]` pan coordinate; dB becomes linear only at the DSP edge |
| `offset`, `end`, `loop_start`, `loop_end`, `loop_mode` | SFZ v1 | supported; inclusive SFZ `end`/`loop_end` positions become checked half-open native ends by exact `+1`; `no_loop`, `one_shot`, `loop_continuous`, and `loop_sustain` remain distinct |
| `loop_type` | SFZ v2 | `forward` and `alternate` are supported and labelled v2; other values are errors |
| `ampeg_attack`, `ampeg_decay`, `ampeg_sustain`, `ampeg_release` | SFZ v1 | supported with exact written times/level and the documented SFZ-v1 linear-attack, convex-decay/release family |
| `trigger` | SFZ v1 | `attack`, `release`, `release_key`, `first`, and `legato` are supported and remain distinct |
| `locc64`, `hicc64` | SFZ v1 | supported only for the full range, `0..63` pedal-up range, or `64..127` pedal-down range; narrower MIDI-controller bands are errors because the native predicate is typed sustain state, not a leaked CC number |
| `group`, `off_by`, `off_mode` | SFZ v1 | supported for nonnegative 32-bit groups and `fast`/`normal`; zero means no choke, following the common v1 player convention |
| `seq_length`, `seq_position` | SFZ v1 | supported for positive values with position at most length; counters are per prepared instrument and selection group |
| `global_label`, `group_label`, `region_label` | extension metadata | warning and retained in the imported support summary; no sound effect |
| `#include`, `#define`, `$` substitution, script/generator opcodes | extension | error in `sfz@1`; no file is opened or text executed implicitly |

Within one header, the last occurrence of a supported opcode wins. A new `<group>` starts from `<global>` and a new
`<region>` starts from the current group. Missing values use the published defaults fixed by the adapter version, not a
host player's changing defaults. In particular: full key/expression range, root key 60, zero tune/transpose/volume/pan
and offset, whole-sample end, no explicit loop, zero attack/decay, full sustain, 1 ms release, attack trigger, no choke,
and a one-position sequence. `key=n` writes `lokey=n`, `hikey=n`, and `pitch_keycenter=n` at that point, so later
individual assignments override the shorthand and an earlier one does not.

All applicable regions layer. Sequence filtering happens before layering; it does not collapse unrelated microphone or
velocity layers into one winner. Release regions require the note's prepared attack token. `release` follows sustain
deferral, while `release_key` follows physical note-off and ignores sustain. `first` and `legato` read the typed Musa
phrase relation rather than ambient MIDI-note state. A newly started region with group `g` stops existing regions whose
`off_by` is `g`, using each stopped region's `off_mode`.

Because SFZ declares neither a whole-instrument voice pool nor a separate native selection policy, `sfz@1` emits a
64-voice `SampleMap` and the map's per-selection-group round-robin policy. Regions outside a nonzero sequence group
still layer; the policy does not turn unrelated regions into alternatives. A product may refuse that fixed pool under
its explicit sampler limits, but it may not silently shrink the checked map.

SFZ-v1 decay and release curves are not identically implemented by maintained players. `sfz@1` fixes the public
SFZ-format reference equation (`exp(-8t/T)`, clamped at the target/terminal level) as adapter semantics so live and
offline rendering cannot depend on an installed player. This choice, the accepted opcode table, path interpretation, and
every default are part of adapter identity; changing one requires `sfz@2`.

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
beat support and an opaque `AssetRef`; it is not a note. The event track applies only its ordinary temporal laws and
remains opaque to the media reference and fit policy.

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
Core transforms may move, copy, or restrict that point; they do not stretch or reverse audio. Physical seconds, raw or
decoded samples, and fixed-media duration never enter `Term[ScoreFact]` or `EventTrack[WrittenTime, ScoreFact]`. The
asset table beside the score snapshot supplies `L` to performance preparation. This is the honest exception to
“everything has a beat duration,” useful for field recording and fixed-media/timeline practice (OMT
`098-twentieth-century-rhythmic-techniques.md`).

### Temporal action table

Core operations act only on occurrence support; the media reader remains forward unless source explicitly chooses a
playback-rate sign in a later specification.

| Operation | Note-driven sample | Musical clip | Fixed-media cue |
| --- | --- | --- | --- |
| follow/shift | note gesture moves with its note | beat interval translates; source phase anchor translates | point onset translates; `L` unchanged |
| together | independent note voices | both media machines coexist | both media machines coexist |
| repeat | notes and deterministic realization choices duplicate with Origin | interval and phase mapping duplicate per iteration | point duplicates; each copy lasts `L` seconds |
| stretch by `r>0` | note support and gesture timing scale; pitch is unchanged | beat support scales by `r`; policy is reapplied to the new support | onset scales; physical duration and playback rate are unchanged |
| restrict `[i,j)` | ordinary note restriction policy | support intersects the window while retaining the original source phase, so a middle restriction does not restart | point survives iff its onset is in the window; a surviving recording is not cropped |
| retrograde in duration `d` | note supports relocate by the score law | interval relocates and source still plays forward | point moves from `b` to `d-b`; audio is not reversed |
| pitch transpose/invert | written note pitch changes before sample selection | no effect | no effect |

For fixed media, every listed temporal operation can change only `b₀`; substituting the new onset into `end=θ(b₀)+L`
proves physical-duration invariance. For a `by rate` clip, normalized phase depends only on the affine beat coordinate
in its transformed interval, proving that it reaches 0 and 1 at the transformed endpoints. Restriction must therefore
retain the original affine phase anchor; resetting it would violate the restriction law by changing audible content that
remained inside the window.

## 5. Routing recorded media

Sampled instruments produce their assigned part machine. Beat-fitted and fixed-media declarations produce named media
machines. They enter only the mix graph:

```musa
studio {
    route pulse -> master;
    room field_room { reverb(room: 0.74, damping: 0.6, mix: 1); }
    send harbor -> field_room at -18 dB;
    route harbor -> master;
    route field_room -> master;
}
```

A media machine has no `PartId` unless it is a note-driven instrument. The mix cannot send it note gestures, retune it,
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

Prompts 182–188 implement these laws; prompts 189–192 measure preparation cost and audit deterministic artifacts.
