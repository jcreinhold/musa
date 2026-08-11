---
id: 134
slug: reproducible-assets
status: pending
depends_on: [85, 92, 128, 130]
phase: 4
---

# Assets Are Immutable Build Inputs

> **Contingent on prompt 125.** The core-boundary decision may repair this prompt's Design, fold it into another, or
> replace it. Read `docs/core-boundary.md` first.

## Task

Give Musa projects a secure, provenance-preserving asset model for samples and recordings before a decoder is added. The
canonical build input becomes source plus manifest/lock plus content-addressed assets; editable intent remains in
source, while external bytes are named, typed, verified, and loaded only on the control side.

## Read

- `docs/language/09-assets-and-packages.md`; roadmap §§10.6, 13.2, 16, 18 Phase 4; prompt 84 project ownership.
- Current `Project`, import closure, `Compilation`/`ValidArtifacts`, semantic hash, export cache, CLI/project error
  model, and every path-opening call.
- Repository dependency policy before selecting an audio metadata/decoder crate. A new dependency requires a deliberate
  roadmap repair and license/security review in the prompt commit.

## Design

An `AssetRef` has logical project/package-relative identity, declared kind, verified content digest, and source span.
Resolved asset facts expose only metadata required by project, compiler, UI, and audio preparation; raw bytes/decoder
objects stay behind the project/audio asset store. Source semantic identity remains score-only where promised; audio
artifact identity additionally includes asset digests, relevant decoder/format version, realization seed, and render
options.

Resolution is rooted: project assets stay within the project root; package assets stay within that locked package.
Reject traversal, escaping symlinks, kind mismatches, digest mismatch, missing files, unsupported encodings, and
unreasonable declared sizes with actionable diagnostics. Compilation/checking may inspect metadata but never decode on
the audio thread. All I/O, validation, decode/preload, replacement, and retirement happen on the control side.

Record optional license/source metadata and surface it to users, but do not pretend the compiler can determine legal
permission. Define invalidation: a changed asset digest invalidates prepared audio and audio exports, not kernel or
notation artifacts.

## Target

- Manifest/source asset declarations and project-owned resolver/store with immutable verified identities.
- CLI check/list/verify behavior and project/LSP/UI facts for status, kind, origin, digest, and license metadata.
- Hash/cache invalidation laws and path/symlink/adversarial-size tests using temporary project roots.
- No sample or clip playback yet; focused fixtures use tiny deterministic assets only.

## Check

```sh
cargo nextest run -p musa-compiler -p musa-project -p musa -p musa-lsp
cargo clippy --all-targets -p musa-compiler -p musa-project -p musa -p musa-lsp -- -D warnings
cargo fmt --check
cargo deny check
cargo insta test --workspace --unreferenced=reject
```

Commit as `Make assets reproducible build inputs`.

## Stop

- No implicit download, embedded credentials, license adjudication, sample playback, or waveform editor.
- No raw asset bytes in `ScoreSnapshot`, kernel terms, compiler diagnostics, or desktop IPC.
- No cache key based only on path or modification time.
