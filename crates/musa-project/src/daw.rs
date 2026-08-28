//! One directory a workstation can read (`docs/rules/across-stages/06-daw-boundary.md`).
//!
//! The bundle is a *projection*, never a second copy of the work. Rule D1
//! holds: nothing here comes back into `.musa`, and the manifest says so in
//! as many words. What the bundle adds over the individual exports is that
//! every artifact in it was produced from **one** compile, one render
//! argument record, and one identity — so a MIDI track, a stem, and a
//! notation file in the same directory are readings of the same piece rather
//! than three exports taken at three moments.
//!
//! **The profile chooses packaging, never music.** A Logic bundle and a
//! `GarageBand` bundle of the same piece hold the same MIDI, the same audio,
//! and the same manifest facts; they differ in whether `MusicXML` is present
//! and in what the guidance says. No realization decision is made to suit a
//! host — §9 of the boundary forbids it, and a host-shaped realization would
//! be a second work.
//!
//! **Deterministic by construction.** Repeated exports of equal inputs
//! produce equal bytes: no wall-clock time, no absolute path, no user name,
//! no random identifier reaches a file. What varies between two machines is
//! the directory the bundle was asked for, which is not in it.
//!
//! **Installed whole or not at all.** The bundle is built in a sibling
//! temporary directory and moved into place in one step. An error, at any
//! point, leaves the destination exactly as it was.

use std::path::{Path, PathBuf};

use sha2::{Digest as _, Sha256};

use crate::error::ProjectError;

/// The bundle schema this module writes and prompt 213 onward carries.
const BUNDLE_VERSION: u64 = 1;

/// Which workstation the packaging and guidance are for.
///
/// Not a rendering choice. The profile decides which files are present and
/// what the guidance says; the music is the same either way.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DawProfile {
    /// Logic Pro: notation *or* production, chosen by the user.
    Logic,
    /// `GarageBand`: the MIDI and audio forms its own documentation names.
    GarageBand,
}

impl DawProfile {
    /// The word the manifest and the CLI both use.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Logic => "logic",
            Self::GarageBand => "garageband",
        }
    }

    /// Whether this profile's bundle carries the notation route.
    const fn carries_musicxml(self) -> bool {
        matches!(self, Self::Logic)
    }
}

/// What the caller asked for, beyond where to put it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct DawExportOptions {
    /// Which workstation to package for.
    pub profile: DawProfile,
    /// Whether an existing destination may be replaced.
    ///
    /// Replacement is still all-or-nothing: the old directory is moved aside,
    /// the new one is moved into place, and only then is the old one removed.
    /// A failure at any step leaves the original where it was.
    pub replace: bool,
}

impl DawExportOptions {
    /// A bundle for one profile, refusing to touch a destination that exists.
    pub const fn new(profile: DawProfile) -> Self {
        Self {
            profile,
            replace: false,
        }
    }

    /// The same, permitted to replace a bundle already at the destination.
    #[must_use]
    pub const fn replacing(mut self) -> Self {
        self.replace = true;
        self
    }
}

/// One file the bundle contains.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DawFile {
    path: String,
    bytes: u64,
    digest: String,
}

impl DawFile {
    /// Where it sits inside the bundle, with `/` separators whatever the
    /// host filesystem uses.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Its size.
    pub const fn bytes(&self) -> u64 {
        self.bytes
    }

    /// Its SHA-256, lowercase hex — the same digest the manifest carries.
    pub fn digest(&self) -> &str {
        &self.digest
    }
}

/// One thing the bundle could not say, and what kind of thing it was.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DawLoss {
    kind: String,
    message: String,
}

impl DawLoss {
    /// A stable word for the class of loss: `polytempo`, `polymeter`,
    /// `tuning`, `channel`, `controller`, or `notation`.
    pub fn kind(&self) -> &str {
        &self.kind
    }

    /// What was lost, in the words the producing exporter used.
    pub fn message(&self) -> &str {
        &self.message
    }

    fn new(kind: &str, message: impl Into<String>) -> Self {
        Self {
            kind: kind.to_owned(),
            message: message.into(),
        }
    }
}

/// What one export produced.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DawExportReport {
    version: u64,
    profile: DawProfile,
    destination: PathBuf,
    files: Vec<DawFile>,
    losses: Vec<DawLoss>,
}

impl DawExportReport {
    /// The bundle schema version, which prompt 213 onward carries forward.
    pub const fn version(&self) -> u64 {
        self.version
    }

    /// Which workstation it was packaged for.
    pub const fn profile(&self) -> DawProfile {
        self.profile
    }

    /// The directory now holding it.
    pub fn destination(&self) -> &Path {
        &self.destination
    }

    /// Every file written, in bundle order, each with its digest.
    pub fn files(&self) -> &[DawFile] {
        &self.files
    }

    /// Everything the target formats could not say, once per kind.
    pub fn losses(&self) -> &[DawLoss] {
        &self.losses
    }
}

/// Build one bundle in memory: relative path to bytes, in bundle order.
struct Staged {
    files: Vec<(String, Vec<u8>)>,
}

impl Staged {
    const fn new() -> Self {
        Self { files: Vec::new() }
    }

    fn add(&mut self, path: impl Into<String>, bytes: Vec<u8>) {
        self.files.push((path.into(), bytes));
    }

    /// The manifest's file table: path, size, digest, in bundle order.
    fn table(&self) -> Vec<DawFile> {
        self.files
            .iter()
            .map(|(path, bytes)| DawFile {
                path: path.clone(),
                bytes: bytes.len() as u64,
                digest: hex(&Sha256::digest(bytes)),
            })
            .collect()
    }
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(
        String::with_capacity(bytes.len().saturating_mul(2)),
        |mut text, byte| {
            // The write cannot fail: a `String` is an infallible sink.
            let _written = write!(text, "{byte:02x}");
            text
        },
    )
}

/// Produce one bundle and install it at `destination`.
///
/// Everything is built before anything is written: the compile is already
/// done, and the MIDI, notation, and audio here all read that one compile.
fn build(
    name: &str,
    valid: &crate::snapshot::ValidArtifacts,
    assets: &crate::assets::AssetInventory,
    options: DawExportOptions,
) -> Result<(Staged, Vec<DawLoss>), ProjectError> {
    let score = &valid.score;
    let gestures =
        musa_compiler::lower_gestures(score).map_err(|error| ProjectError::Performance(error.to_string()))?;
    let midi_options = musa_notation::MidiOptions::default();
    let render = |mode| {
        musa_notation::render_midi_with_origins(&gestures, &musa_notation::MidiOptions { mode, ..midi_options })
            .map_err(|error| ProjectError::Notation(error.to_string()))
    };
    let written = render(musa_notation::MidiMode::Score)?;
    let played = render(musa_notation::MidiMode::Performance)?;
    let stems = crate::stems::to_stems(score, &valid.studio_execution, assets)?;

    let mut losses = crate::playback::midi_losses(score, &gestures)
        .into_iter()
        .map(|(kind, said)| DawLoss::new(kind, said))
        .collect::<Vec<_>>();
    losses.push(DawLoss::new(
        "tuning",
        format!(
            "the audio was rendered at A = {} Hz; neither a Standard MIDI File nor a WAV states a \
             tuning, so a workstation will play the MIDI at whatever its own instruments are tuned to",
            musa_score::PerformanceOptions::default().tuning.concert_a
        ),
    ));
    losses.push(DawLoss::new(
        "controller",
        "the MIDI files carry notes, tempo, meter, and key and no continuous controllers: the \
         dynamics, articulation, and profile shaping of this piece are in the rendered audio, not in \
         the MIDI",
    ));
    if let Some(shared) = shared_channel(written.tracks()) {
        losses.push(DawLoss::new(
            "channel",
            format!(
                "this piece has more parts than MIDI has channels, so `{shared}` shares a channel with \
                 another part; a workstation reading channels rather than tracks will merge them"
            ),
        ));
    }

    let mut staged = Staged::new();
    staged.add("score.mid", written.bytes().to_vec());
    staged.add("performance.mid", played.bytes().to_vec());
    if options.profile.carries_musicxml() {
        let notation = musa_notation::render_notation(
            score,
            musa_notation::NotationTarget::MusicXml,
            &musa_notation::NotationOptions::default(),
        )
        .map_err(|error| ProjectError::Notation(error.to_string()))?;
        losses.extend(
            notation
                .warnings()
                .iter()
                .map(|warning| DawLoss::new("notation", warning.clone())),
        );
        staged.add("score.musicxml", notation.text().as_bytes().to_vec());
    } else {
        losses.push(DawLoss::new(
            "notation",
            "GarageBand does not document MusicXML import, so this bundle contains no notation file; \
             export the piece for Logic, or use `musa render --to musicxml`, if you want one",
        ));
    }
    for file in stems.files() {
        staged.add(stem_path(file), file.bytes().to_vec());
    }

    let manifest = manifest(name, valid, assets, options, &stems, &written, &staged, &losses)?;
    staged.add("musa-manifest.json", manifest);
    let origins = origins(options, &stems, &written, &played)?;
    staged.add("musa-origins.json", origins);
    Ok((staged, losses))
}

/// Where one stem sits in the bundle. The kind chooses the directory, so two
/// routes of one name are two files even on a filesystem that folds case.
fn stem_path(file: &crate::StemFile) -> String {
    match file.kind() {
        crate::StemKind::Master => format!("audio/{}", file.file_name()),
        crate::StemKind::Part => format!("audio/parts/{}", file.file_name()),
        crate::StemKind::Bus => format!("audio/returns/{}", file.file_name()),
    }
}

/// The first part whose channel another part already has, if any.
fn shared_channel(tracks: &[musa_notation::MidiTrack]) -> Option<&str> {
    let mut seen: Vec<u8> = Vec::new();
    for track in tracks {
        // The tempo track has no channel; it is not a part sharing one.
        let Some(channel) = track.channel else { continue };
        if seen.contains(&channel) {
            return track.part.as_deref();
        }
        seen.push(channel);
    }
    None
}

/// The manifest: what this bundle is, what produced it, and what it lost.
///
/// The file table deliberately excludes the manifest and the origin sidecar.
/// A file cannot carry its own digest, and a table that listed one of them
/// and not the other would be a table with a rule nobody could state.
#[expect(clippy::too_many_arguments, reason = "a manifest is the record of every argument")]
fn manifest(
    name: &str,
    valid: &crate::snapshot::ValidArtifacts,
    assets: &crate::assets::AssetInventory,
    options: DawExportOptions,
    stems: &crate::StemSet,
    written: &musa_notation::RenderedMidi,
    staged: &Staged,
    losses: &[DawLoss],
) -> Result<Vec<u8>, ProjectError> {
    use serde_json::{Map, Value, json};

    let stem_of = |kind: crate::StemKind, part: &str| {
        stems
            .files()
            .iter()
            .find(|file| file.kind() == kind && file.part() == part)
            .map(stem_path)
    };
    let parts = written
        .tracks()
        .iter()
        .filter_map(|track| {
            let part = track.part.as_deref()?;
            Some(json!({
                "name": part,
                "midiTrack": track.index,
                "midiChannel": track.channel,
                "stem": stem_of(crate::StemKind::Part, part),
            }))
        })
        .collect::<Vec<_>>();
    let returns = stems
        .files()
        .iter()
        .filter(|file| file.kind() == crate::StemKind::Bus)
        .map(|file| json!({ "name": file.part(), "stem": stem_path(file) }))
        .collect::<Vec<_>>();
    let routes = stems
        .routes()
        .iter()
        .map(|route| {
            json!({
                "kind": match route.kind() {
                    crate::StemRouteKind::Route => "route",
                    crate::StemRouteKind::Send => "send",
                },
                "source": route.source(),
                "destination": route.destination(),
            })
        })
        .collect::<Vec<_>>();
    let files = staged
        .table()
        .into_iter()
        .map(|file| json!({ "path": file.path, "bytes": file.bytes, "sha256": file.digest }))
        .collect::<Vec<_>>();

    let mut document = Map::new();
    document.insert("musaManifest".to_owned(), json!(BUNDLE_VERSION));
    document.insert("profile".to_owned(), json!(options.profile.name()));
    document.insert("piece".to_owned(), json!(name));
    document.insert(
        "canonical".to_owned(),
        json!(
            "The `.musa` source is the work. This bundle is a one-way reading of it and nothing here \
             converts back (docs/rules/across-stages/06-daw-boundary.md, Rule D1)."
        ),
    );
    document.insert(
        "identity".to_owned(),
        json!({
            "music": valid.identity.to_string(),
            "assets": hex(&assets.identity()),
        }),
    );
    document.insert(
        "render".to_owned(),
        json!({
            "sampleRate": stems.sample_rate(),
            "channels": 2,
            "bitsPerSample": 32,
            "sampleFormat": "float",
            "renderSeed": crate::playback::RENDER_SEED,
            "frames": stems.frames(),
        }),
    );
    document.insert(
        "midi".to_owned(),
        json!({
            "ticksPerQuarter": musa_notation::MidiOptions::default().ticks_per_quarter,
            "score": "score.mid",
            "performance": "performance.mid",
        }),
    );
    document.insert(
        "notation".to_owned(),
        json!(if options.profile.carries_musicxml() {
            Value::String("score.musicxml".to_owned())
        } else {
            Value::Null
        }),
    );
    document.insert("mix".to_owned(), json!("audio/mix.wav"));
    document.insert("parts".to_owned(), json!(parts));
    document.insert("returns".to_owned(), json!(returns));
    document.insert("routes".to_owned(), json!(routes));
    document.insert(
        "additive".to_owned(),
        json!(
            "The stems do not sum to the mix. A send duplicates signal, a return may share nonlinear \
             processing, and the master limiter cannot be reconstructed from what reached it. The \
             routes above say where the signal went."
        ),
    );
    document.insert("files".to_owned(), json!(files));
    document.insert("guidance".to_owned(), json!(guidance(options.profile)));
    document.insert(
        "losses".to_owned(),
        json!(
            losses
                .iter()
                .map(|loss| json!({ "kind": loss.kind, "message": loss.message }))
                .collect::<Vec<_>>()
        ),
    );
    json_bytes(&Value::Object(document))
}

/// How to bring this bundle into the workstation it was packaged for.
///
/// Claims here stay at the strength `06-daw-boundary.md` §8 records: what
/// Apple's `GarageBand` page states was read directly, and the Logic rows are
/// cited at the strength of the guide's own titles. Nothing here asserts a
/// behaviour this project has not read or measured.
fn guidance(profile: DawProfile) -> Vec<&'static str> {
    match profile {
        DawProfile::Logic => vec![
            "Choose one route. `score.musicxml` is the editable notation; `performance.mid` is the \
             played reading for production. Importing both makes two copies of one piece.",
            "`score.mid` is the neutral reading: written durations, one velocity. Import it instead of \
             `performance.mid` if you want the notes without an interpretation.",
            "`audio/` holds the rendered mix and its stems. Every file starts at project frame zero and \
             is the same length, so they line up with no offset.",
            "Logic's import behaviour here is cited from its guide's titles rather than measured; \
             prompt 215 replaces that with a measurement.",
        ],
        DawProfile::GarageBand => vec![
            "GarageBand imports MIDI by dragging the file into the Tracks area, where it \"appears on \
             one or more software instrument tracks\"; choose the instrument for each in the Library.",
            "`performance.mid` is the played reading and `score.mid` the written one. Import one.",
            "`audio/` holds the rendered mix and its stems, all starting at frame zero and all the same \
             length. GarageBand's documented audio formats include WAV.",
            "GarageBand does not document MusicXML import, so this bundle has no notation file.",
        ],
    }
}

/// The origin sidecar: which exported thing came from which source event.
///
/// Explicitly not canonical, and it says so: it is a reading of the bundle,
/// useful for pointing a report at source, and it is regenerated by the next
/// export rather than maintained.
fn origins(
    options: DawExportOptions,
    stems: &crate::StemSet,
    written: &musa_notation::RenderedMidi,
    played: &musa_notation::RenderedMidi,
) -> Result<Vec<u8>, ProjectError> {
    use serde_json::{Map, Value, json};

    let file = |path: &str, rendered: &musa_notation::RenderedMidi| {
        json!({
            "path": path,
            "tracks": rendered
                .tracks()
                .iter()
                .map(|track| json!({
                    "track": track.index,
                    "part": track.part,
                    "channel": track.channel,
                }))
                .collect::<Vec<_>>(),
            "notes": rendered
                .origins()
                .iter()
                .map(|origin| json!({
                    "track": origin.track,
                    "tick": origin.tick,
                    "key": origin.key,
                    "event": origin.event.0,
                }))
                .collect::<Vec<_>>(),
        })
    };

    let mut document = Map::new();
    document.insert("musaOrigins".to_owned(), json!(BUNDLE_VERSION));
    document.insert(
        "canonical".to_owned(),
        json!(
            "A reading of this bundle, not of the work. The `.musa` source is canonical; this file is \
             rewritten by the next export and nothing should be edited here."
        ),
    );
    document.insert(
        "midi".to_owned(),
        json!(vec![file("score.mid", written), file("performance.mid", played)]),
    );
    document.insert(
        "notation".to_owned(),
        json!(if options.profile.carries_musicxml() {
            json!({
                "path": "score.musicxml",
                "ids": Value::Array(Vec::new()),
                "note": "musa's MusicXML export writes no element ids, so no note in it can be named \
                         from here. The MIDI origins below cover the same events.",
            })
        } else {
            Value::Null
        }),
    );
    document.insert(
        "stems".to_owned(),
        json!(
            stems
                .files()
                .iter()
                .map(|stem| json!({
                    "path": stem_path(stem),
                    "kind": match stem.kind() {
                        crate::StemKind::Master => "master",
                        crate::StemKind::Part => "part",
                        crate::StemKind::Bus => "bus",
                    },
                    "name": stem.part(),
                }))
                .collect::<Vec<_>>()
        ),
    );
    json_bytes(&Value::Object(document))
}

/// Pretty JSON with a trailing newline, so a bundle can be read and diffed.
/// Key order is the order it was built in, which is the schema's order.
fn json_bytes(document: &serde_json::Value) -> Result<Vec<u8>, ProjectError> {
    let mut bytes =
        serde_json::to_vec_pretty(document).map_err(|error| ProjectError::Performance(error.to_string()))?;
    bytes.push(b'\n');
    Ok(bytes)
}

/// Build the bundle and install it whole.
///
/// The staging directory is a sibling of the destination, so the move into
/// place is a rename within one filesystem rather than a copy that can fail
/// halfway. Nothing is written at the destination until every artifact
/// exists.
pub(crate) fn export_bundle(
    name: &str,
    valid: &crate::snapshot::ValidArtifacts,
    assets: &crate::assets::AssetInventory,
    options: DawExportOptions,
    destination: &Path,
) -> Result<DawExportReport, ProjectError> {
    let parent = destination
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf);
    if !parent.is_dir() {
        return Err(ProjectError::Assets(format!(
            "`{}` is not a directory this export can write beside",
            parent.display()
        )));
    }
    let occupied = destination.symlink_metadata().is_ok();
    if occupied && !options.replace {
        return Err(ProjectError::Assets(format!(
            "`{}` already exists; ask for a replacement or choose another name",
            destination.display()
        )));
    }

    let (staged, losses) = build(name, valid, assets, options)?;
    let files = staged.table();

    // Staged beside the destination and moved in one step: an interrupted
    // export leaves a temporary directory the operating system will collect,
    // never a half-written bundle somebody might read as a whole one.
    let staging = tempfile::tempdir_in(&parent).map_err(|error| ProjectError::io(parent.display(), error))?;
    for (path, bytes) in &staged.files {
        let file = staging.path().join(path);
        if let Some(directory) = file.parent() {
            std::fs::create_dir_all(directory).map_err(|error| ProjectError::io(directory.display(), error))?;
        }
        std::fs::write(&file, bytes).map_err(|error| ProjectError::io(file.display(), error))?;
    }
    install(staging, destination, &parent, occupied)?;

    Ok(DawExportReport {
        version: BUNDLE_VERSION,
        profile: options.profile,
        destination: destination.to_path_buf(),
        files,
        losses,
    })
}

/// Move a finished staging directory into place, replacing what is there.
///
/// The old destination is moved aside first and removed last, so a failure
/// of the second rename can put it back. What this must never do is delete
/// a good bundle and then fail to install its replacement.
fn install(staging: tempfile::TempDir, destination: &Path, parent: &Path, occupied: bool) -> Result<(), ProjectError> {
    let staged = staging.keep();
    let displaced = if occupied {
        let aside = tempfile::Builder::new()
            .prefix(".musa-export-replaced-")
            .tempdir_in(parent)
            .map_err(|error| ProjectError::io(parent.display(), error))?
            .keep();
        // `tempdir_in` made the directory; the rename needs the name free.
        discard(std::fs::remove_dir(&aside), "clear the aside name");
        if let Err(error) = std::fs::rename(destination, &aside) {
            discard(std::fs::remove_dir_all(&staged), "remove the staged bundle");
            return Err(ProjectError::io(destination.display(), error));
        }
        Some(aside)
    } else {
        None
    };
    if let Err(error) = std::fs::rename(&staged, destination) {
        discard(std::fs::remove_dir_all(&staged), "remove the staged bundle");
        if let Some(aside) = displaced {
            discard(std::fs::rename(&aside, destination), "put the old bundle back");
        }
        return Err(ProjectError::io(destination.display(), error));
    }
    if let Some(aside) = displaced {
        discard(std::fs::remove_dir_all(&aside), "remove the replaced bundle");
    }
    Ok(())
}

/// A cleanup step that must not mask the failure it is cleaning up after.
///
/// Reported rather than dropped: a temporary directory left behind is worth
/// a line in the log, and worth nothing in the error the caller sees.
fn discard(outcome: std::io::Result<()>, what: &str) {
    if let Err(error) = outcome {
        tracing::debug!(%error, "could not {what}");
    }
}
