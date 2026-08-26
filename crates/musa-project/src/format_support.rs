//! Read-only facts owned by the registered foreign-format adapters.
//!
//! These records do not describe Musa declarations. They report only the
//! private host boundary: which versioned adapter recognizes an immutable
//! asset, what it translates, and which sound-changing input it refuses.

/// The support contract for one registered foreign sound format.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FormatSupportFacts {
    /// Stable adapter identity participating in asset identity.
    pub adapter: &'static str,
    /// Human-readable foreign format name.
    pub format: &'static str,
    /// Exact source contract produced by successful translation.
    pub result: &'static str,
    /// Compact summary of supported translation behavior.
    pub summary: &'static str,
    /// Names accepted by the adapter where a finite vocabulary exists.
    pub accepted_names: &'static [&'static str],
    /// Named refusal classes; every encountered unsupported name is also
    /// repeated verbatim in the adapter diagnostic.
    pub refused: &'static [&'static str],
    /// Governing external specification URL.
    pub specification: &'static str,
}

const SFZ: FormatSupportFacts = FormatSupportFacts {
    adapter: "sfz@1",
    format: "SFZ",
    result: "std::sound::sample::SampleMapArtifact",
    summary: "global/group/region inheritance; root-contained WAV regions; key and expression ranges; exact pitch, gain, pan, loops, envelopes, release behavior, sustain state, choke, and sequence selection",
    accepted_names: crate::sfz::SUPPORTED_OPCODES,
    refused: &[
        "unknown headers and sound-changing opcodes",
        "include/define directives and substitutions",
        "scripts, generators, and non-WAV sample codecs",
        "controller bands that do not translate exactly to typed sustain state",
    ],
    specification: "https://sfzformat.com/opcodes/",
};

const SF2: FormatSupportFacts = FormatSupportFacts {
    adapter: "sf2@1",
    format: "SoundFont 2.04",
    result: "std::sound::sample::SampleMapArtifact",
    summary: "bounded RIFF/Hydra validation; explicit preset selection; exact global/local zone combination; embedded 16/24-bit samples; key and expression layers; tuning, attenuation, pan, envelopes, loops, low-pass, stereo links, exclusive classes, and admitted note-on modulators",
    accepted_names: &[
        "address offsets",
        "pan",
        "initialAttenuation",
        "volume envelope",
        "keyRange",
        "velRange",
        "coarseTune",
        "fineTune",
        "sampleID",
        "sampleModes",
        "exclusiveClass",
        "initialFilterFc",
        "initialFilterQ",
        "scaleTuning=100",
    ],
    refused: &[
        "active LFO and modulation-envelope generators",
        "effects sends and forced key/velocity",
        "ambient MIDI controllers, pressure, pitch wheel, linked modulators, and unsupported destinations",
        "ROM or non-reciprocal linked samples",
    ],
    specification: "https://musescore.org/sites/musescore.org/files/2023-01/sfspec24.pdf",
};

/// Look up the registered adapter support for an authored asset address.
///
/// The address is inspected only to select a registered foreign adapter. The
/// editor does not parse the foreign file or infer its musical declaration.
#[must_use]
pub fn format_support(asset: &str) -> Option<&'static FormatSupportFacts> {
    let (base, fragment) = asset
        .rsplit_once('#')
        .map_or((asset, None), |(base, fragment)| (base, Some(fragment)));
    let extension = std::path::Path::new(base).extension()?.to_str()?;
    if extension.eq_ignore_ascii_case("sfz") && fragment.is_none() {
        Some(&SFZ)
    } else if extension.eq_ignore_ascii_case("sf2")
        && fragment.is_some_and(|fragment| fragment.starts_with("preset=") || fragment.starts_with("preset-name="))
    {
        Some(&SF2)
    } else {
        None
    }
}

/// Every registered foreign sound-format contract, in stable documentation order.
#[must_use]
pub const fn format_supports() -> &'static [FormatSupportFacts] {
    &[SFZ, SF2]
}
