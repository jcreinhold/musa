# Bundle format

What `musa render <piece> --to daw` writes, exactly. The how-to for using one is
[Import a Musa bundle into Logic Pro or GarageBand](../how-to/daw-bundle.md); this page is the schema.

A bundle is an ordinary directory. Nothing in it is compressed, encrypted, or proprietary, and every file in it is a
format that outlives this project.

## Layout

```text
<bundle>/
  score.mid              the written reading
  performance.mid        the played reading
  score.musicxml         the notation — Logic profile only
  audio/
    mix.wav              the rendered master
    parts/<part>.wav     one stem per part output
    returns/<bus>.wav    one stem per named bus
  musa-manifest.json     what this bundle is
  musa-origins.json      which note came from which source event
```

Audio is 32-bit float WAV at the render's own sample rate. Every audio file begins at project frame zero and is exactly
as long as the mix, so they line up with no offset — including the tail after the last written note.

**The stems do not sum to the mix.** A send duplicates signal, a return may share nonlinear processing, and the master's
limiting cannot be reconstructed from what reached it. The manifest's `routes` say where signal actually went.

## `musa-manifest.json`

```json
{
  "musaManifest": 1,
  "profile": "logic",
  "piece": "examples/glass-mountain.musa",
  "canonical": "The `.musa` source is the work. …",
  "identity": { "music": "d79499c5…", "assets": "e3b0c442…" },
  "render": {
    "sampleRate": 48000, "channels": 2, "bitsPerSample": 32,
    "sampleFormat": "float", "renderSeed": 1297437505, "frames": 856000
  },
  "midi": { "ticksPerQuarter": 480, "score": "score.mid", "performance": "performance.mid" },
  "notation": "score.musicxml",
  "mix": "audio/mix.wav",
  "parts":   [{ "name": "violin", "midiTrack": 1, "midiChannel": 0, "stem": "audio/parts/violin.wav" }],
  "returns": [{ "name": "hall", "stem": "audio/returns/hall.wav" }],
  "routes":  [{ "kind": "route", "source": "violin", "destination": "master" }],
  "additive": "The stems do not sum to the mix. …",
  "files":   [{ "path": "score.mid", "bytes": 226, "sha256": "be66d07e…" }],
  "guidance": ["Choose one route. …"],
  "losses":  [{ "kind": "tuning", "message": "…" }]
}
```

| Field | What it is |
| --- | --- |
| `musaManifest` | schema version. A reader that does not know this number should refuse rather than guess |
| `profile` | `logic` or `garageband` — which packaging this is |
| `piece` | the source path as given to the exporter, for a person to recognize |
| `canonical` | one sentence stating that the source is the work and this is a one-way reading |
| `identity.music` | the exact identity of the compiled music. Same music, same string; changed music, changed string |
| `identity.assets` | the identity of the locked asset closure the render used |
| `render` | the format, the realization seed, and the exact frame count every audio file has |
| `midi` | the division and the two readings' filenames |
| `notation` | the notation file, or absent with a `notation` loss saying why |
| `parts` | one entry per part: its name, its MIDI track, its MIDI channel, and its stem |
| `returns` | one entry per named bus that got its own stem |
| `routes` | where signal went, which is why the stems do not sum |
| `files` | every file in the bundle, with its byte count and SHA-256 |
| `guidance` | sentences for a person about how to import this without making two copies of one piece |
| `losses` | everything this reading could not carry — see [Losses](losses.md) |

Two things are deliberately *not* in it: a timestamp, and any path outside the bundle. An export is reproducible, so two
exports of one source are byte-for-byte identical, and a manifest that recorded when or where it was made would break
that for no benefit.

## `musa-origins.json`

The provenance sidecar. It answers "which source event produced this note?" for every note in both MIDI files.

```json
{
  "musaOrigins": 1,
  "canonical": "A reading of this bundle, not of the work. …",
  "midi": [
    {
      "path": "score.mid",
      "tracks": [{ "track": 1, "part": "violin", "channel": 0 }],
      "notes":  [{ "track": 1, "tick": 0, "key": 76, "event": 0 }]
    }
  ],
  "notation": { … },
  "stems": { … }
}
```

`event` indexes the piece's own event track, which is what makes the answer exact rather than approximate. Track 0
carries the conductor information and belongs to no part, which is why its `part` and `channel` are `null`.

This file is rewritten by the next export. Nothing reads it back into Musa; it exists so that a rendering can be traced
to its source six months later.

## Verifying a bundle

Every file's digest is in the manifest, so a bundle checks itself:

```bash
python3 - <<'EOF'
import hashlib, json, pathlib, sys
root = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else ".")
manifest = json.loads((root / "musa-manifest.json").read_text())
for entry in manifest["files"]:
    digest = hashlib.sha256((root / entry["path"]).read_bytes()).hexdigest()
    print(("ok  " if digest == entry["sha256"] else "BAD "), entry["path"])
EOF
```
