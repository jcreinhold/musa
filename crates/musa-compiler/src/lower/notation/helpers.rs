//! Diagnostic helpers: provenance and the `not_a_*` refusal builders.

use musa_calculus::{Origin, Raw};

use crate::diagnose::{Code, Diagnostic};
use crate::lower::Lowering;
use crate::origin::{DeclarationId, SourceSpan};

impl Lowering<'_> {
    /// The `Origin` argument a constructed fact carries (§5.7).
    ///
    /// `pub(super)` for one caller outside this module: [`super::values`] reads a
    /// written `play(v, d)` as the four-argument application, and the two
    /// arguments it supplies are these.
    pub(crate) fn provenance_at(&self, origin: Origin, placed: bool, declaration: DeclarationId) -> Raw {
        let span = self.sites.span(origin).unwrap_or_default();
        self.provenance(origin, span, placed, declaration)
    }

    #[expect(
        clippy::unused_self,
        reason = "reads as a sibling of `provenance_at`, which needs the table"
    )]
    pub(crate) fn provenance(&self, origin: Origin, span: SourceSpan, placed: bool, declaration: DeclarationId) -> Raw {
        let written = crate::origin::Origin {
            source_span: if placed { span } else { crate::elaborate::SHARED_ORIGIN },
            definition_span: span,
            declaration,
            // Empty by construction: expansion happened in the phase, and what
            // this module reads is the answer it left behind.
            expansion_path: Vec::new(),
        };
        Raw::lit(origin, crate::registry::origin_literal(written))
    }

    /// A statement §3 forbids inside a value usable at several places.
    ///
    /// *Misplaced* rather than *unsupported*: these are permanent answers, and
    /// the same statement in a score is perfectly legal. A diagnostic that said
    /// "not supported yet" would be a promise nobody intends to keep.
    ///
    /// The sentence names **material** rather than the `music` value it is most
    /// often written in, because a `motif` body and a `repeat` block reach here
    /// too and neither is a `music` value: a message that named one spelling
    /// would be false at two of its three call sites.
    pub(crate) fn misplaced<T>(&mut self, what: &str, span: SourceSpan) -> Option<T> {
        self.refuse(
            Diagnostic::error(Code::Misplaced, format!("{what} belongs to the piece, not to material"))
                .at(span, "this says \"from here onward\"")
                .help("write it in the voice or part this music is used in")
                .note("material is usable at several places, and \"from here onward\" has no unique meaning there"),
        )
    }

    pub(crate) fn not_a_pitch(text: &str, span: SourceSpan) -> Diagnostic {
        Diagnostic::error(Code::NotAValue, format!("`{text}` is not a pitch"))
            .at(span, "expected a pitch")
            .note("a pitch is a letter, an optional `#` or `b`, and an octave: `c4`, `g#5`, `bb3`")
    }

    pub(crate) fn not_a_dynamic(text: &str, span: SourceSpan) -> Diagnostic {
        Diagnostic::error(Code::UnknownWord, format!("`{text}` is not a dynamic marking"))
            .at(span, "unknown marking")
            .help(crate::resolve::suggest(
                text,
                crate::score::DynamicMark::NAMES,
                "markings",
            ))
    }

    pub(crate) fn not_a_count(text: &str, span: SourceSpan) -> Diagnostic {
        Diagnostic::error(Code::NotAValue, format!("`{text}` is not a count")).at(span, "expected a whole number")
    }

    pub(crate) fn out_of_range<T>(&mut self, span: SourceSpan) -> Option<T> {
        self.refuse(
            Diagnostic::error(Code::OutOfRange, "this pitch leaves Musa's exact range")
                .at(span, "the written coordinates do not fit"),
        )
    }
}
