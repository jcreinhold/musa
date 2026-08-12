//! The request handlers: one module per request kind, each a lookup into the
//! snapshot's facts plus a coordinate conversion.
//!
//! The law of this directory is `docs/rules/desktop/03-interaction.md` §7: a
//! handler restates what the session computed — it never derives a musical
//! fact from the text itself. A handler that cannot answer from facts answers
//! `None`.
//!
//! Facts describe the last *valid* compile (roadmap §14.7), so while the
//! source is mid-edit their spans refer to the text that compiled. Handlers
//! treat a match as best-effort — the same bargain every client makes with
//! its own debounce — and never as a reason to panic.

pub(crate) mod analysis;
pub(crate) mod bundled;
pub(crate) mod call;
pub(crate) mod code_action;
pub(crate) mod completion;
pub(crate) mod definition;
pub(crate) mod diagnostics;
pub(crate) mod folding;
pub(crate) mod formatting;
pub(crate) mod hover;
pub(crate) mod items;
pub(crate) mod names;
pub(crate) mod semantic_tokens;
pub(crate) mod signature_help;
pub(crate) mod symbols;

use musa_project::Fraction;

/// An exact rational the way a person reads it: `3` or `3/8`, never `0.375`.
fn fraction(fraction: &Fraction) -> String {
    if fraction.denominator == 1 {
        fraction.numerator.to_string()
    } else {
        format!("{}/{}", fraction.numerator, fraction.denominator)
    }
}
