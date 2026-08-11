---
id: 84
slug: the-project-is-the-unit
status: done
depends_on: [19, 36, 76]
phase: 3
---

# The Project Is the Unit

## Task

Make the thing a composer opens a **project** rather than a file. `examples/album/` is the fixture roadmap §16 exists to
justify and it cannot be worked with: opening `pieces/01-opening.musa` tells you nothing about the piece beside it, and
opening `library/motifs.musa` is rejected outright by the compiler. This prompt adds `Project` above `ProjectSession` —
the volume above the piece — with a running order the manifest can set, a piece each for the pieces you have opened, and
material that opens without an argument. Nothing about a loose `.musa` file changes: it is a project of one, and it
behaves exactly as it does today.

## Read

- Roadmap §16 — the whole specification: one file grows a directory only when it needs to share something; imports are
  relative, acyclic, local and side-effect-free; no registry, no dependency solver. The sentence this prompt implements
  is the last one: *"An album manifest may specify ordering and shared metadata, but a piece remains independently
  compilable."*
- Prompt 36 — what exists. `project.rs::find` walks up for a `musa.toml` and reads two keys; `imports::closure` reads
  the transitive `use` graph on every recompile. Its repair note is the constraint: **`musa.toml` is metadata and
  nothing else**, and the piece must compile identically with or without it.
- Prompt 19 — `ProjectSession`'s doctrine: *one session, one canonical source*. That stays true. `Project` does not make
  a session hold two documents; it holds two sessions.
- `crates/musa-project/src/session.rs` — `audio: Option<AudioEngine>`, opened lazily the first time something plays.
  That laziness is what makes several sessions affordable, and closing the one you leave is what keeps it true.
- `crates/musa-compiler/src/elaborate.rs` — the `PieceDecl::from_root` gate, and the help line under it that already
  calls a library a legitimate musa file.

## Design

### `Project` is the volume; `ProjectSession` is one piece of it, open

Two nouns, and the crate's `lib.rs` says which is which. `Project` is what the shell holds.

```rust
pub fn open(path: impl AsRef<Path>) -> Result<Self, ProjectError>;   // a piece or a folder
pub fn create(path: impl AsRef<Path>, template: Template) -> Result<Self, ProjectError>;
pub fn from_text(source: impl Into<String>, name: impl Into<String>) -> Self;
pub fn current(&self) -> &ProjectSession;
pub fn current_mut(&mut self) -> &mut ProjectSession;
pub fn show(&mut self, file: &str) -> Result<(), ProjectError>;
pub fn snapshot(&self) -> ProjectSnapshot<'_>;
pub fn save_all(&mut self) -> Result<(), ProjectError>;
```

`Project` is not a facade over `ProjectSession`. It forwards nothing: a caller reaches the document with `current_mut()`
and issues the same `ProjectCommand`s it issues today. What `Project` owns is the set of open pieces, which one is in
hand, and the listing — three things no session can know, and the only three it adds.

**There is always a current piece.** Opening a folder opens the first piece in the running order; a folder with no piece
in it is an error at open time, not an empty state. That kills an entire screen and is musically right — you do not open
a volume in order to look at its cover.

**A piece you leave is left exactly as it was.** `show` keeps the outgoing session — its text, its unsaved flag, and its
undo history — so turning to another piece and back is turning back, not reopening. The one thing it takes away is the
audio device: `show` calls a new `ProjectSession::release_audio`, because only the piece in hand may sound, and a
session that never played never opened a device in the first place.

### Contents, and who spells the words

New `crates/musa-project/src/contents.rs`. It scans `pieces/` when there is one and the root otherwise, and `library/`;
non-recursively, `*.musa` only. Two lists, not a tree — a `library` cannot contain a score, so material and repertoire
are different kinds in the grammar before they are different rows on a page.

The running order is `[project] pieces = [...]` in the manifest when it says, then every unlisted `.musa` file in
filename order. A listed file that is not there is a diagnostic on the project, never a row silently dropped: a contents
page that hides a file is worse than one that admits a gap.

```rust
pub struct ContentsFacts { pub name: String, pub composer: Option<String>,
                           pub pieces: Vec<EntryFacts>, pub material: Vec<EntryFacts> }
pub struct EntryFacts { pub title: String, pub file: String,
                        pub current: bool, pub unsaved: bool, pub used: bool }
```

`title` is the piece's own title, parsed from its header and falling back to the file name — a contents page prints what
the composer wrote, not what the filesystem calls it. `used` marks material the current piece imports, which is
`session.imports()` and costs nothing. Both are computed here because `03-interaction.md` §7 is law: the frontend spells
no facts.

`ProjectSnapshot` gains `contents: Option<&ContentsFacts>` — `None` from `ProjectSession::snapshot`, `Some` from
`Project::snapshot` — and `to_wire` carries it. Recomputed on open, on `show`, and on save; not on every keystroke,
because nothing in it changes on one except `unsaved`, which is read from the sessions each time the facts are built.

### Material is a legitimate file

`elaborate.rs` rejects a `library { … }` root with *"this file declares no piece"*, three lines under a help string that
says a library is one of the two things a musa file may be. Repair it: accept a `Library` root, run the same declaration
collection the import loader runs so that errors *inside* the library are still reported, and return no score without
reporting an error.

That breaks the standing invariant `compiles == score.is_some()`, so the snapshot gains the distinction the language
already has:

```rust
pub enum DocumentKind { Piece, Material }
```

`compiles` then means *well-formed as what it is*, which is what every caller wanted it to mean. A consumer that needs a
score still asks for one and still gets `None`; what changes is that it can now tell "no score yet" from "no score
ever", and prompt 85 routes on exactly that.

### Fixtures, and one repair

- `examples/album/pieces/02-waltz.musa`, the second piece §16's own sketch names, and `pieces = [...]` in
  `examples/album/musa.toml`. A running order with one entry proves nothing.
- Delete `examples/album/pieces/01-opening.musa.recovery`. It is an autosave artifact committed by accident in
  `046f683`; `ProjectSession::open` calls `autosave::take`, so opening the album fixture today offers a stale draft as
  unrecovered crash work.

### Surfaces

- Desktop: `SessionHandle` holds a `Project`. `open_project` accepts a folder as well as a file, and new `show_piece`
  and `save_all` commands turn to another piece and write them all. No `ContentsDto`: `musa-project` owns the wire
  shape, and mirroring `ContentsFacts` into a ts-rs DTO would create the second wire format `session.rs` exists to
  prevent — the interface reads it through `state/snapshot.ts`, as it reads every other part of a snapshot.
  `registry.rs` gains *Open a project* (`Shift+CmdOrCtrl+O`), *Save every piece* (`CmdOrCtrl+Alt+S`), and the Contents
  workspace on `CmdOrCtrl+0` — declared here, `available: false` until prompt 85 implements the screen, which is what
  the flag is for. `⌘0` is the number before the four workspaces because the volume comes before the piece, so *Reset
  zoom* moves to `⇧⌘0` and keeps the shape every other reset in the application has.
- CLI: `musa check <dir>` checks every file the project lists — the running order first, then the material under it. It
  is the same listing the contents page reads, so the two cannot disagree about what is in a project, and a project
  whose libraries are broken does not pass.

## Target

- `crates/musa-project`: `src/project.rs` grown into the `Project` type; new `src/contents.rs`; `DocumentKind` and
  `contents` on `ProjectSnapshot` and its wire; `ProjectSession::release_audio`.
- `crates/musa-compiler`: the `Library` root branch in `elaborate.rs`, and `Compilation` carrying which kind of document
  it compiled.
- `crates/musa`: `check` over a directory; one line of usage.
- `apps/musa-desktop/src-tauri`: `Project` in the session thread, `show_piece` and `save_all`, three registry commands.
- `apps/musa-desktop/ui`: `kind` and `contents` on `ProjectSnapshot`, the bridge calls behind them, and the two new File
  commands in `commands/map.ts`. No screen: prompt 85 owns that.
- `examples/album/`: `pieces/02-waltz.musa`, the manifest's running order, and the deleted `.recovery` file.
- Tests: `crates/musa-project/tests/project_laws.rs`.

## Check

```sh
cargo nextest run --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo run -p musa -- check examples/album
for f in examples/*.musa examples/album/pieces/*.musa examples/album/library/*.musa; do
  cargo run -p musa -- check "$f"
done
```

`project_laws.rs` asserts: a loose file is a project of one; opening a piece inside a project opens the project with
that piece current; the running order is the manifest's, and unlisted files follow it; a piece's edits and undo history
survive a turn away and back; only the piece in hand holds the audio device; material opens with no diagnostic and no
score; `used` names exactly the current piece's imports; and a piece in a project compiles identically to the same
source with no project around it (prompt 36's law, still true).

## Stop

- No multi-piece render, export, or playback orchestration. Prompt 36's Stop stands: each piece renders independently.
- No `assets/`, no package registry, no versioned dependencies, no remote imports (§16, rejected).
- No file creation, renaming, deletion, or moving through the project API. It reads a directory; the filesystem and the
  manifest set what is in one.
- No recursive scan and no watcher. Two directories, read when something happens, not polled.
- No recent-projects list and no persisted last-opened project.
- No screen work: prompt 85 owns every pixel of this.
