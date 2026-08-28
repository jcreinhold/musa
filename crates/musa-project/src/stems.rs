//! Aligned stems beside the master mix.
//!
//! A stem here is a *tap*: a read of a route the studio already declared,
//! taken during the same frame the master was mixed from. There is no second
//! mixer model, no stem declaration in `.musa`, and no mute or solo. What a
//! part output or a named bus carried is what the file contains.
//!
//! **They are aligned, not additive.** Every file begins at project frame
//! zero and ends at the same frame, including declared release and effect
//! tails. Summing them does not reproduce the master and this module never
//! says it does: a send duplicates signal, a return may share nonlinear
//! processing, and the master's own limiter cannot be reconstructed from
//! what reached it. [`StemSet::routes`] reports the declared edges so a
//! reader can see the projection instead of guessing at it.
//!
//! **Names are for reading, not for identity.** A file name is derived from
//! a display name, and display names are not unique, not case-distinct on
//! every filesystem, and not always writable as a path. So the naming rule
//! below is deliberately conservative: a name that is not plainly writable,
//! or that would share a file name with another of its kind, is disambiguated
//! by a suffix derived from its stable semantic identity. The authoritative
//! mapping from file to route is [`StemFile::part`] and [`StemFile::kind`],
//! never the spelling of the file name.

use musa_score::ScoreSnapshot;

use crate::error::ProjectError;

/// Which output one file carries.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StemKind {
    /// The mix, identical to the WAV export.
    Master,
    /// One part's own instrument chain, before any bus it sends to.
    Part,
    /// One named bus or return, after its own chain.
    Bus,
}

/// Whether a declared edge carries the whole signal or a share of it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StemRouteKind {
    /// The signal goes there.
    Route,
    /// A share of the signal goes there as well.
    Send,
}

/// One declared routing edge of the studio the stems were tapped from.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct StemRoute {
    kind: StemRouteKind,
    source: String,
    destination: String,
}

impl StemRoute {
    /// Whether the edge is a route or a send.
    pub const fn kind(&self) -> StemRouteKind {
        self.kind
    }

    /// The part or bus the signal leaves.
    pub fn source(&self) -> &str {
        &self.source
    }

    /// The bus it arrives at, or `master`.
    pub fn destination(&self) -> &str {
        &self.destination
    }
}

/// One rendered output and the name it should be written under.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StemFile {
    kind: StemKind,
    part: String,
    file_name: String,
    bytes: Vec<u8>,
}

impl StemFile {
    /// Which output this is.
    pub const fn kind(&self) -> StemKind {
        self.kind
    }

    /// The part or bus name the source wrote. Empty for the master.
    pub fn part(&self) -> &str {
        &self.part
    }

    /// The file name to write it under, within its kind's directory. Stable
    /// for equal inputs and free of path separators, control characters, and
    /// the characters Windows reserves.
    pub fn file_name(&self) -> &str {
        &self.file_name
    }

    /// A 32-bit floating-point stereo WAV of the whole aligned extent.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// A master mix and every stem beside it, all of one length.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StemSet {
    sample_rate: u32,
    frames: u64,
    files: Vec<StemFile>,
    routes: Vec<StemRoute>,
}

impl StemSet {
    /// Samples per second, the same for every file.
    pub const fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Frames in every file. All of them start at project frame zero and run
    /// to this frame, silence and tails included.
    pub const fn frames(&self) -> u64 {
        self.frames
    }

    /// The master first, then one file per part, then one per named bus.
    pub fn files(&self) -> &[StemFile] {
        &self.files
    }

    /// The declared routing edges the taps sit on.
    pub fn routes(&self) -> &[StemRoute] {
        &self.routes
    }
}

/// Render the master and every declared tap as aligned WAV files.
///
/// One preparation and one traversal produce all of them, so the stems are
/// aligned by construction rather than by a later correction, and the master
/// is byte-for-byte the one [`crate::ExportRequest::Wav`] produces.
///
/// # Errors
/// [`ProjectError::Performance`] if lowering, audio preparation, or WAV
/// encoding fails.
pub(crate) fn to_stems(
    score: &ScoreSnapshot,
    studio: &musa_dsp::StudioExecution,
    assets: &crate::assets::AssetInventory,
) -> Result<StemSet, ProjectError> {
    let mut prepared = crate::playback::build(score, studio, assets)?;
    let routes = prepared
        .routes()
        .iter()
        .map(|route| StemRoute {
            kind: match route.kind() {
                musa_dsp::AudioRouteKind::Route => StemRouteKind::Route,
                musa_dsp::AudioRouteKind::Send => StemRouteKind::Send,
            },
            source: route.source().to_owned(),
            destination: route.destination().to_owned(),
        })
        .collect();
    let rendered = musa_dsp::render_offline_multitrack(&mut prepared);
    let kinds = rendered
        .stems()
        .iter()
        .map(|stem| match stem.tap().role() {
            musa_dsp::AudioTapRole::Part => StemKind::Part,
            musa_dsp::AudioTapRole::Bus => StemKind::Bus,
        })
        .collect::<Vec<_>>();
    let names = file_names(
        &kinds,
        &rendered
            .stems()
            .iter()
            .map(|stem| stem.tap().name())
            .collect::<Vec<_>>(),
    );
    let mut files = Vec::with_capacity(rendered.stems().len().saturating_add(1));
    files.push(StemFile {
        kind: StemKind::Master,
        part: String::new(),
        file_name: MASTER_FILE.to_owned(),
        bytes: crate::playback::wav_bytes(rendered.master())?,
    });
    for ((stem, kind), file_name) in rendered.stems().iter().zip(kinds).zip(names) {
        files.push(StemFile {
            kind,
            part: stem.tap().name().to_owned(),
            file_name,
            bytes: crate::playback::wav_bytes(stem.audio())?,
        });
    }
    Ok(StemSet {
        sample_rate: rendered.master().sample_rate(),
        frames: rendered.frames(),
        files,
        routes,
    })
}

/// What the mix is written as. The kind, not the name, says what it is.
const MASTER_FILE: &str = "mix.wav";

/// Longest sanitized display name kept in a file name, before any suffix.
const NAME_BUDGET: usize = 64;

/// Deterministic file names for one rendered set.
///
/// A name is kept as written only when it is *plainly writable* — nonempty,
/// within the budget, and made only of ASCII letters, digits, `-`, and `_` —
/// and only when nothing else of its kind claims the same name once ASCII
/// case is folded away. Anything else is disambiguated with a suffix derived
/// from the kind and the exact display name, which is why two names that a
/// filesystem would merge cannot become one file.
fn file_names(kinds: &[StemKind], names: &[&str]) -> Vec<String> {
    let mut chosen: Vec<String> = kinds
        .iter()
        .zip(names)
        .map(|(kind, name)| {
            if plainly_writable(name) {
                (*name).to_owned()
            } else {
                suffixed(&sanitize(name), *kind, name)
            }
        })
        .collect();
    // Two parts may be written `Horn` and `horn`, and a filesystem that folds
    // case would keep one file. Both are then named by identity instead. The
    // kind is deliberately not part of this key: a consumer that writes every
    // file into one directory must not lose one, and the kind is reported
    // beside the file rather than spelled into it.
    let mut folded: Vec<String> = chosen.iter().map(|name| name.to_ascii_lowercase()).collect();
    // Decided from the names as they stand, before any of them is replaced:
    // renaming as we walk would disambiguate whichever came first and leave
    // the other alone, which is an answer that depends on the walk.
    let colliding = folded
        .iter()
        .map(|key| folded.iter().filter(|other| *other == key).count() > 1)
        .collect::<Vec<_>>();
    for index in 0..chosen.len() {
        if colliding.get(index) != Some(&true) {
            continue;
        }
        let Some((kind, name)) = kinds.get(index).zip(names.get(index)) else {
            continue;
        };
        let replacement = suffixed(&sanitize(name), *kind, name);
        if let Some(slot) = folded.get_mut(index) {
            *slot = replacement.to_ascii_lowercase();
        }
        if let Some(slot) = chosen.get_mut(index) {
            *slot = replacement;
        }
    }
    // A 64-bit identity digest is not a proof of distinctness. If two of them
    // still meet, the position in the set separates them, and the position is
    // as deterministic as the routing it came from.
    for index in 0..chosen.len() {
        let shared = folded
            .iter()
            .enumerate()
            .any(|(other, key)| other < index && folded.get(index) == Some(key));
        if !shared {
            continue;
        }
        let Some(slot) = chosen.get_mut(index) else { continue };
        *slot = format!("{slot}-{index}");
        if let Some(key) = folded.get_mut(index) {
            *key = slot.to_ascii_lowercase();
        }
    }
    chosen.into_iter().map(|name| format!("{name}.wav")).collect()
}

/// Whether a display name may stand as a file name unchanged.
fn plainly_writable(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= NAME_BUDGET
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
}

/// The writable residue of a display name: every other byte becomes `_`, and
/// runs of them collapse so a name of punctuation does not become a wall of
/// underscores. May be empty, which the suffix then carries alone.
fn sanitize(name: &str) -> String {
    let mut out = String::with_capacity(name.len().min(NAME_BUDGET));
    let mut gap = false;
    for character in name.chars() {
        if character.is_ascii_alphanumeric() || character == '-' || character == '_' {
            if out.len() >= NAME_BUDGET {
                break;
            }
            out.push(character);
            gap = false;
        } else if !gap && !out.is_empty() {
            out.push('_');
            gap = true;
        }
    }
    while out.ends_with('_') {
        out.pop();
    }
    out
}

/// A sanitized name plus the identity digest that makes it unambiguous.
fn suffixed(sanitized: &str, kind: StemKind, name: &str) -> String {
    let digest = identity(kind, name);
    if sanitized.is_empty() {
        format!("{digest:016x}")
    } else {
        format!("{sanitized}-{digest:016x}")
    }
}

/// FNV-1a over the kind and the exact display bytes. Stable across runs,
/// platforms, and releases, which is what makes a file name reproducible.
fn identity(kind: StemKind, name: &str) -> u64 {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    let tag = match kind {
        StemKind::Master => b'm',
        StemKind::Part => b'p',
        StemKind::Bus => b'b',
    };
    let mut hash = OFFSET;
    for byte in std::iter::once(tag).chain(name.bytes()) {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(PRIME);
    }
    hash
}

/// What a display name becomes when it has to be a file.
///
/// These are unit tests rather than fixtures because `.musa` identifiers are
/// ASCII, so a name with a slash or a combining mark cannot reach this
/// function from source today. The function is still total over what a future
/// display name could hold, and that totality is what is checked here.
#[cfg(test)]
#[expect(
    clippy::indexing_slicing,
    reason = "a fixture too short to index is the test failing"
)]
mod naming_laws {
    use super::{StemKind, file_names};

    fn parts(names: &[&str]) -> Vec<String> {
        file_names(&vec![StemKind::Part; names.len()], names)
    }

    #[test]
    fn a_plain_name_is_kept_as_written() {
        assert_eq!(
            parts(&["horn", "double_bass", "vln-1"]),
            ["horn.wav", "double_bass.wav", "vln-1.wav"]
        );
    }

    #[test]
    fn a_name_a_path_cannot_hold_is_replaced_rather_than_escaped() {
        let named = parts(&["a/b"]);
        assert_eq!(named.len(), 1);
        let name = &named[0];
        assert!(name.starts_with("a_b-"), "{name}");
        assert!(!name.contains('/'), "{name}");
    }

    #[test]
    fn a_name_outside_ascii_never_reaches_the_filesystem_to_be_normalized_there() {
        // Two names a normalizing filesystem could merge must not both be
        // written under their own spelling. Neither is: both are named by
        // identity, and their identities differ because their bytes do.
        let composed = "vi\u{0301}ola";
        let precomposed = "v\u{00ed}ola";
        let named = file_names(&[StemKind::Part, StemKind::Part], &[composed, precomposed]);
        assert!(named[0].is_ascii() && named[1].is_ascii(), "{named:?}");
        assert_ne!(named[0], named[1]);
    }

    #[test]
    fn two_names_one_filesystem_would_fold_together_are_both_kept() {
        let named = parts(&["Horn", "horn"]);
        assert_ne!(named[0].to_ascii_lowercase(), named[1].to_ascii_lowercase());
        assert!(
            named
                .iter()
                .all(|name| name.starts_with("Horn-") || name.starts_with("horn-")),
            "{named:?}"
        );
    }

    #[test]
    fn a_part_and_a_bus_of_one_name_are_two_files() {
        let named = file_names(&[StemKind::Part, StemKind::Bus], &["hall", "hall"]);
        assert_ne!(named[0], named[1]);
    }

    #[test]
    fn a_name_with_nothing_writable_in_it_is_named_by_identity_alone() {
        let named = parts(&["", "···"]);
        assert_ne!(named[0], named[1]);
        assert!(
            named.iter().all(|name| name.len() == "0123456789abcdef.wav".len()),
            "{named:?}"
        );
    }

    #[test]
    fn a_name_longer_than_the_budget_is_cut_and_then_disambiguated() {
        let long = "a".repeat(200);
        let other = format!("{long}b");
        let named = parts(&[&long, &other]);
        assert_ne!(named[0], named[1]);
        assert!(named.iter().all(|name| name.len() < 100), "{named:?}");
    }

    #[test]
    fn the_same_names_produce_the_same_files_every_time() {
        let names = ["Horn", "horn", "a/b", "hall", ""];
        assert_eq!(parts(&names), parts(&names));
    }
}
