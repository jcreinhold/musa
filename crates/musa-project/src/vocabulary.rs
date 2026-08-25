//! The checked edition-pinned studio vocabulary shared by application readers.
//!
//! Compilation and exact decoding happen once per process. The cached value is
//! an opaque projection of ordinary bundled Musa source; it is not a second
//! construction API and carries the complete checked source identity.

use std::sync::OnceLock;

static VOCABULARY: OnceLock<Result<musa_dsp::StudioVocabulary, String>> = OnceLock::new();

/// Read the standard studio vocabulary compiled from `std::sound::catalogue`.
///
/// # Errors
///
/// Returns a stable description when the bundled declaration tree fails to
/// check or its exact artifact disagrees with the consumer schema. Such a
/// failure is a build defect and is held so repeated editor requests do not
/// repeatedly compile it.
pub fn standard_studio_vocabulary() -> Result<&'static musa_dsp::StudioVocabulary, &'static str> {
    VOCABULARY
        .get_or_init(|| {
            let checked = musa_compiler::checked_standard_studio_vocabulary().map_err(|diagnostics| {
                diagnostics
                    .iter()
                    .map(|diagnostic| diagnostic.message.as_str())
                    .collect::<Vec<_>>()
                    .join("; ")
            })?;
            musa_dsp::decode_studio_vocabulary(&checked).map_err(|error| error.to_string())
        })
        .as_ref()
        .map_err(String::as_str)
}
