//! `kernel EventTrack[WrittenTime, ScoreFact] { … }` read as a raw core term.
//!
//! `01-surface.md` §7's typed antiquotation: the one place a composer writes the
//! assembly the compiler normally writes for them, and the only surface form
//! whose interior is not this language. The parser recognizes the braces and
//! reads the `${…}` holes; everything between them is `musa-kernel`'s grammar,
//! so what this module does with the text is hand it to the kernel's own reader.
//!
//! # What is decided here, and what is left for the core
//!
//! Everything a quote can be wrong *about itself* is decided here, once, while
//! the source is still in reach: that the three type words name the one
//! instantiation this build has, that the body parses, that no payload written
//! in it settles a question its use is entitled to settle, that every `${…}`
//! stands where the term expects material, and that the term is closed. None of
//! those needs a value, and all of them want a span.
//!
//! What is left is the one thing that does need values — binding each hole to
//! the material the host spliced there and evaluating the assembled term — and
//! that is `spliced`, the third operation [`crate::registry::track`] registers
//! past both ownership tables.
//!
//! # Why the term is held whole rather than translated
//!
//! The kernel has seven term formers and the core spells three of them:
//! `follow`, `together`, and `shift`. The two that are left are the reason a
//! quote exists at all. A raw `scale` moves every occurrence and leaves every
//! written value alone, and a raw `restrict` observes a window without
//! shortening the work it looks at — neither is a notated operation, and
//! `examples/kernel-splice.musa` spends a paragraph on why the diminution in it
//! is written `${stretch(1/2, subject)}` rather than `scale by 1/2`. Translating
//! the term into core terms would mean registering both as operations past both
//! tables whose only caller is this reading, α-renaming the quote's own binders
//! so a spliced expression cannot be captured by one, and re-deriving
//! [`musa_kernel::evaluate`] as core reduction.
//!
//! So the term rides in a literal, which is [`crate::registry::template_literal`]'s
//! answer one stage down and for the same reason: a quote's body is inert, no
//! program takes one apart, and two of them agree exactly when the host says the
//! terms do.
//!
//! # Provenance
//!
//! The facts written raw in the quote are made *here*, so they are stamped here:
//! the quote's span, the piece's scope — a quote is an expression, and
//! `01-surface.md` §2's rule that a value usable at several places has no voice
//! of its own is what [`super::notation::Reading::free`] already says about
//! `music { … }` — and one [`ExpansionStep::KernelSplice`] at locus zero, so
//! Origin view can say that a note with no surface statement behind it came from
//! here.
//!
//! The facts a hole *splices* were made by the expression the host wrote, which
//! this module never walks past. Their step is the hole's own locus, and it is
//! prepended by `instanced` — the same builtin an instance site uses, because it
//! is the same claim: these facts were produced inside this expansion.

#[cfg(test)]
mod laws;

use musa_calculus::{Origin, Raw};
use musa_kernel::{Duration, Term, WrittenTime};
use musa_language::ast::AstNode as _;
use musa_language::{SyntaxElement, SyntaxKind, SyntaxNode};
use num_rational::Ratio;

use super::{Lowering, applied, expansion, listed};
use crate::diagnose::{Code, Diagnostic};
use crate::elaborate::ScoreFact;
use crate::origin::{ExpansionStep, SourceSpan};

/// The term a quote's body denotes, at the one instantiation this build has.
type Quoted = Term<WrittenTime, ScoreFact>;

impl Lowering<'_> {
    /// One kernel quote, as the `spliced` call it denotes.
    ///
    /// The five checks run in an order chosen so each is answerable on its own:
    /// the type words before the text (a quote at the wrong payload is not a
    /// term this build could read), the text before the payloads (there are no
    /// payloads until it parses), closure before the holes (it needs the hole
    /// *names*, which substitution already fixed, and none of their material),
    /// and the holes last, all of them, so a quote with two misplaced `${…}`
    /// reports two.
    pub(super) fn kernel_quote(&mut self, node: &SyntaxNode, origin: Origin) -> Option<Raw> {
        let span = crate::resolve::trimmed_span(node);
        let quote = musa_language::ast::KernelQuote::cast(node.clone())?;
        self.instantiation(&quote)?;

        let text = quote_text(node);
        let (body_start, body_end) = quote.body_span()?;
        let holes = quote.holes();
        // Fresh names, chosen so that nothing in the quoted text can be one:
        // capture is prevented by construction rather than diagnosed after the
        // fact.
        let mut stem = "splice".to_owned();
        while text.contains(&stem) {
            stem.push('_');
        }
        let named: Vec<String> = (0..holes.len()).map(|index| format!("{stem}{index}")).collect();
        let base = u32::from(node.text_range().start());
        let (source, seams) = substitute_holes(&text, base, body_start, body_end, &stem, &holes);
        let mut term = match musa_kernel::parse_expression::<WrittenTime, ScoreFact>(&source) {
            Ok(term) => term,
            Err(error) => {
                return self.refuse(
                    Diagnostic::error(Code::Syntax, "this kernel quote is not well formed").at(
                        quote_error_span(&seams, &error, body_start, body_end),
                        error.to_string(),
                    ),
                );
            }
        };
        self.settles_nothing(&term, span)?;
        self.stands_alone(&term, &named, span)?;

        let mut material = Vec::with_capacity(holes.len());
        let mut whole = true;
        for (hole, name) in holes.iter().zip(&named) {
            match self.hole(&term, hole, name, span) {
                Some(read) => material.push(read),
                None => whole = false,
            }
        }
        if !whole {
            return None;
        }

        // The quote's own writing, at the quote's own zero. Built once and
        // cloned: every raw payload carries the same answer, because they were
        // all written in the same place.
        let written = expansion(span, ExpansionStep::KernelSplice { at: Ratio::new(0, 1) });
        term.map_payloads(&mut |fact| {
            fact.scope = crate::Scope::Piece;
            fact.origin = written.clone();
        });
        Some(applied(
            origin,
            Raw::hosted(origin, "spliced"),
            [
                Raw::lit(origin, crate::registry::kernel_literal(term, named)),
                listed(origin, material),
            ],
        ))
    }

    /// The three type words, checked against what this build can mean.
    ///
    /// This language's own words rather than the kernel's, which is why they are
    /// read here and not by [`musa_kernel::parse_expression`]. The coordinate is
    /// checked before the payload because it is the stronger claim: a track in
    /// performed time is not a score, whatever its payloads say, and nothing
    /// converts one coordinate into another.
    fn instantiation(&mut self, quote: &musa_language::ast::KernelQuote) -> Option<()> {
        if let Some((constructor, at)) = quote.constructor()
            && constructor != "EventTrack"
        {
            return self.refuse(
                Diagnostic::error(
                    Code::UnknownName,
                    format!("`{constructor}` is not a kernel type constructor"),
                )
                .at(SourceSpan::new(at.0, at.1), "expected `EventTrack`")
                .note("a quote writes one composition expression, and a composition is an event track"),
            );
        }
        if let Some((coordinate, at)) = quote.coordinate()
            && coordinate != "WrittenTime"
        {
            return self.refuse(
                Diagnostic::error(
                    Code::UnknownName,
                    format!("a score quote cannot be written in `{coordinate}`"),
                )
                .at(SourceSpan::new(at.0, at.1), "expected `WrittenTime`")
                .help("write `WrittenTime`, the coordinate notation is written in")
                .note("nothing converts one coordinate into another; performance derives its own"),
            );
        }
        let (payload, at) = quote.payload_type()?;
        if payload != "ScoreFact" {
            return self.refuse(
                Diagnostic::error(
                    Code::UnsupportedPayload,
                    format!("this build has no meaning for `{payload}` payloads"),
                )
                .at(SourceSpan::new(at.0, at.1), "no payload type by this name")
                .help("write `ScoreFact`, the payload a musa score is made of")
                .note("the kernel is parametric in its payload; this compiler implements one"),
            );
        }
        Some(())
    }

    /// Refuse a quote whose raw payloads would settle what its use settles.
    ///
    /// A reusable value may read the context it is used in but never change it,
    /// so a payload that wrote a key, a meter, a tempo or a clef is refused —
    /// and so is one that named a scope or an origin, because the use decides
    /// which voice material lands in and this module decides where it came
    /// from. Both are stamped a few lines below, over whatever was written, and
    /// refusing first is what keeps the overwriting from being silent.
    fn settles_nothing(&mut self, term: &Quoted, span: SourceSpan) -> Option<()> {
        let mut authority = Vec::new();
        term.for_each_payload(&mut |fact| {
            if let Some(what) = context_authority(&fact.kind) {
                authority.push(what);
            }
            if fact.scope != crate::elaborate::SHARED_SCOPE {
                authority.push("a voice of its own");
            }
            if fact.origin.source_span != crate::elaborate::SHARED_ORIGIN {
                authority.push("an origin of its own");
            }
        });
        let Some(what) = authority.first() else {
            return Some(());
        };
        self.refuse(
            Diagnostic::error(Code::Misplaced, format!("a quote cannot carry {what}"))
                .at(span, "this material would settle what its use is entitled to settle")
                .help("write what the material *is*; the use supplies where it goes and where it came from")
                .note("reusable music may read the context supplied at each use, but it cannot change it"),
        )
    }

    /// Refuse a quote that names something it does not bind.
    ///
    /// Asked with the holes bound to empty material — the shape `spliced` will
    /// build — so that a free name in a quote is a complaint about the quote
    /// rather than about the use that first reached it.
    fn stands_alone(&mut self, term: &Quoted, named: &[String], span: SourceSpan) -> Option<()> {
        let mut closed = term.clone();
        for name in named.iter().rev() {
            closed = Term::bind(name.clone(), Term::literal(musa_kernel::empty(Duration::ZERO)), closed);
        }
        if let Err(error) = closed.check() {
            return self.refuse(
                Diagnostic::error(Code::UnknownName, "this kernel quote does not stand on its own")
                    .at(span, error.to_string())
                    .note(
                        "a quote is closed: every name it uses is one it binds, and `${…}` is how the outside gets in",
                    ),
            );
        }
        Some(())
    }

    /// One `${…}`, as the material `spliced` will bind to its name.
    ///
    /// The locus is read off the term rather than counted here, because where a
    /// hole sits is a property of the composition around it: a `follow` adds the
    /// durations before it, a `together` leaves it alone, a `shift` translates
    /// it. [`Term::locus`] is that reading, and the refusal is what a `${…}`
    /// somewhere no material can stand gets.
    fn hole(
        &mut self,
        term: &Quoted,
        hole: &musa_language::ast::KernelHole,
        name: &str,
        span: SourceSpan,
    ) -> Option<Raw> {
        let (start, end) = hole.span();
        let Some(locus) = term.locus(name) else {
            return self.refuse(
                Diagnostic::error(Code::Misplaced, "nothing is spliced here")
                    .at(
                        SourceSpan::new(start, end),
                        "this hole is not in a position that names material",
                    )
                    .help("write `${…}` where the term expects a composition")
                    .note("a hole stands for material, so it stands where material does"),
            );
        };
        let expression = hole.expr()?;
        let origin = self.origin(&expression);
        let read = self.value(&expression)?;
        Some(applied(
            origin,
            Raw::hosted(origin, "instanced"),
            [
                Raw::lit(
                    origin,
                    crate::registry::origin_literal(expansion(
                        span,
                        ExpansionStep::KernelSplice { at: locus.as_ratio() },
                    )),
                ),
                read,
            ],
        ))
    }
}

/// The quote's own text with its comments blanked out, byte for byte.
///
/// A quote is written in a `.musa` file, so it is commented the way the rest of
/// the file is — `//` and `/* */`, which the lexer already reads as trivia here.
/// The kernel's alphabet has no `//` and Musa's has no `%`, so there is exactly
/// one comment syntax inside a quote and it is the host's.
///
/// Blanked rather than removed: every offset in what comes back is still the
/// offset it has in the document, which is what lets a complaint from the
/// kernel's reader point at the character it stopped on. Newlines survive so the
/// line a complaint lands on is the line it was written on.
pub(crate) fn quote_text(node: &SyntaxNode) -> String {
    let mut text = node.text().to_string();
    let base = usize::from(node.text_range().start());
    for token in node.descendants_with_tokens().filter_map(SyntaxElement::into_token) {
        if !matches!(token.kind(), SyntaxKind::LineComment | SyntaxKind::BlockComment) {
            continue;
        }
        let start = usize::from(token.text_range().start()).saturating_sub(base);
        let end = usize::from(token.text_range().end()).saturating_sub(base);
        // Byte-wise, and only ASCII bytes are written: a newline is never part
        // of a multi-byte sequence, so the string stays valid UTF-8 and stays
        // exactly as long as it was.
        // SAFETY-BY-CONSTRUCTION: `blanked` is the same length as the range it
        // replaces, so no later token's offsets move.
        let Some(comment) = text.get(start..end) else {
            continue;
        };
        let blanked: String = comment
            .bytes()
            .map(|byte| if byte == b'\n' { '\n' } else { ' ' })
            .collect();
        text.replace_range(start..end, &blanked);
    }
    text
}

/// The quoted body with each hole replaced by its fresh name, and the map back.
///
/// The map is a list of `(offset in the substituted text, offset in the
/// document)` at each seam, which is what turns a kernel parse error into a
/// place in the composer's file. Without it every complaint about a quote would
/// point at the whole quote.
pub(crate) fn substitute_holes(
    text: &str,
    base: u32,
    body_start: u32,
    body_end: u32,
    stem: &str,
    holes: &[musa_language::ast::KernelHole],
) -> (String, Vec<(usize, u32)>) {
    let relative = |absolute: u32| usize::try_from(absolute.saturating_sub(base)).unwrap_or_default();
    let mut source = String::with_capacity(text.len());
    let mut spans = Vec::new();
    let mut at = relative(body_start);
    for (index, hole) in holes.iter().enumerate() {
        let (start, end) = hole.span();
        let (start, end) = (relative(start), relative(end));
        let Some(before) = text.get(at..start) else {
            continue;
        };
        spans.push((
            source.len(),
            body_start.saturating_add(u32::try_from(at).unwrap_or_default()),
        ));
        source.push_str(before);
        source.push(' ');
        source.push_str(stem);
        source.push_str(&index.to_string());
        source.push(' ');
        at = end;
    }
    if let Some(rest) = text.get(at..relative(body_end)) {
        spans.push((source.len(), base.saturating_add(u32::try_from(at).unwrap_or_default())));
        source.push_str(rest);
    }
    (source, spans)
}

/// Where a kernel parse error lands in the document.
pub(crate) fn quote_error_span(
    spans: &[(usize, u32)],
    error: &musa_kernel::KernelError,
    body_start: u32,
    body_end: u32,
) -> SourceSpan {
    let musa_kernel::KernelError::Parse { offset, .. } = error else {
        return SourceSpan::new(body_start, body_end);
    };
    let Some((seam, document)) = spans.iter().rev().find(|(seam, _)| seam <= offset) else {
        return SourceSpan::new(body_start, body_end);
    };
    let at = document.saturating_add(u32::try_from(offset.saturating_sub(*seam)).unwrap_or_default());
    SourceSpan::new(at, at.saturating_add(1).min(body_end))
}

/// What a fact would take authority over, if it is one of the four that can.
///
/// Key, meter, tempo and clef are *context*: they hold from where they are
/// written until they are written again, so a value carrying one would change
/// its caller's context from inside — the very thing a reusable `music` value
/// must not do (`docs/rules/language/00-semantics.md`, contextual closure).
pub(crate) fn context_authority(kind: &crate::elaborate::FactKind) -> Option<&'static str> {
    match kind {
        crate::elaborate::FactKind::Key { .. } => Some("a key"),
        crate::elaborate::FactKind::Meter { .. } => Some("a meter"),
        crate::elaborate::FactKind::Tempo { .. } => Some("a tempo"),
        crate::elaborate::FactKind::Clef { .. } => Some("a clef"),
        crate::elaborate::FactKind::Note { .. }
        | crate::elaborate::FactKind::Rest { .. }
        | crate::elaborate::FactKind::Mark { .. }
        | crate::elaborate::FactKind::Grace { .. }
        | crate::elaborate::FactKind::Slur
        | crate::elaborate::FactKind::Phrase { .. }
        | crate::elaborate::FactKind::Tuplet { .. }
        | crate::elaborate::FactKind::Dynamic { .. }
        | crate::elaborate::FactKind::Hairpin { .. }
        | crate::elaborate::FactKind::Section { .. }
        | crate::elaborate::FactKind::Harmony { .. }
        | crate::elaborate::FactKind::Repeat { .. }
        | crate::elaborate::FactKind::Mobile { .. }
        | crate::elaborate::FactKind::Improvise { .. }
        | crate::elaborate::FactKind::Ending { .. } => None,
    }
}
