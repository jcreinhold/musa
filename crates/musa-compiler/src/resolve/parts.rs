#![allow(clippy::arithmetic_side_effects)]
use musa_language::ast::{AstNode as _, KeyStmt, TempoStmt};
use musa_language::{SyntaxElement, SyntaxKind, SyntaxNode};
use num_rational::Ratio;

use crate::diagnose::{Code, Diagnostic};
use crate::origin::SourceSpan;
use crate::pitch::PitchClass;
use crate::score::{Clef, Key, Meter, Mode, NotatedDuration};
use crate::time::MusicalDuration;

use super::{Resolver, suggest, token_span, token_text, trimmed_span};

/// The three things a part states about itself, before the reading shapes
/// them.
///
/// The profile a part names is deliberately not among them: a clef, a
/// polymeter, and a polytempo are *facts* that enter the piece's context
/// track, and a profile is metadata the snapshot carries. The reading that
/// wants the facts should not have to supply the declared profiles to get
/// them — see [`crate::lower::piece::Part::profile`], which carries the
/// written name to the one caller that can check it.
pub(crate) struct PartFacts {
    /// The clef the part is read in.
    pub(crate) clef: Option<(Clef, SourceSpan)>,
    /// The part's own meter — polymeter, when it differs from the piece's.
    pub(crate) meter: Option<(Meter, SourceSpan)>,
    /// The part's own tempo — polytempo.
    pub(crate) tempo: Option<(Marking, SourceSpan)>,
}

/// The part-level facts both semantic paths read the same way.
pub(crate) fn part_facts(resolver: &mut Resolver, part: &musa_language::ast::PartDecl) -> PartFacts {
    let mut clef: Option<(Clef, SourceSpan)> = None;
    for node in part.syntax().children() {
        if node.kind() != SyntaxKind::ClefStmt {
            continue;
        }
        let written = token_text(&node, SyntaxKind::Identifier).unwrap_or_default();
        let span = trimmed_span(&node);
        match Clef::parse(&written) {
            // A part reads in one clef until the grammar can say where a
            // second one starts, so a second declaration is two answers to
            // one question rather than a change of clef. Last-wins was
            // silent, which meant the composer found out by looking at the
            // page.
            Some(parsed) => match clef {
                Some((_, first)) => resolver.report(
                    Diagnostic::error(Code::DuplicateName, "this part already says what clef it is in")
                        .at(span, "declared again here")
                        .also(first, "first declared here")
                        .help("write one `clef` per part, and `clef bass;` in a voice where it changes"),
                ),
                None => clef = Some((parsed, span)),
            },
            None => resolver.report(
                Diagnostic::error(Code::UnknownWord, format!("`{written}` is not a clef"))
                    .at(
                        token_span(&node, SyntaxKind::Identifier).unwrap_or_else(|| trimmed_span(&node)),
                        "not a clef musa reads",
                    )
                    .help(suggest(&written, Clef::NAMES, "clefs")),
            ),
        }
    }
    PartFacts {
        clef,
        meter: part.meter().and_then(|stmt| {
            let span = trimmed_span(stmt.syntax());
            match parse_meter(&stmt) {
                Some(meter) => Some((meter, span)),
                None => {
                    resolver.error(
                        Code::NotAValue,
                        "this meter cannot be read",
                        span,
                        "expected `4/4`, or `none`",
                    );
                    None
                }
            }
        }),
        tempo: part
            .tempo()
            .map(|stmt| (tempo_marking(resolver, &stmt), trimmed_span(stmt.syntax()))),
    }
}

/// What one `tempo` statement says, before either path shapes it.
///
/// Three answers rather than a [`crate::elaborate::FactKind`] because the two
/// readings need them in two shapes — one builds the fact directly, the other
/// writes each as an argument of `Fact.Tempo` — and turning the finished fact
/// back into its parts would be a second reading of what this already knows.
pub(crate) struct Marking {
    /// The metronome mark, when the statement carries one.
    pub(crate) metronome: Option<crate::score::Metronome>,
    /// The printed words, when the statement carries any.
    pub(crate) text: Option<String>,
    /// How the marking arrives, when it is gradual.
    pub(crate) ramp: Option<crate::score::Ramp>,
}

impl Marking {}

/// The same reading, stopping one step earlier.
pub(crate) fn tempo_marking(resolver: &mut Resolver, tempo: &TempoStmt) -> Marking {
    let syntax = tempo.syntax();
    let text = tempo.text();
    let metronome = tempo.has_metronome().then(|| {
        let beat = tempo
            .beat()
            .and_then(|text| parse_ratio(&text))
            // `quarter` (or another beat name) resolves to 1/4 for now.
            .unwrap_or_else(|| Ratio::new(1, 4));
        let bpm = tempo
            .bpm()
            .and_then(|text| text.parse::<u32>().ok())
            .unwrap_or_else(|| {
                resolver.report(
                    Diagnostic::error(Code::NotAValue, "this tempo has no speed")
                        .at(trimmed_span(syntax), "expected a number")
                        .help("write `tempo quarter = 72;`"),
                );
                120
            });
        crate::score::Metronome { beat, bpm }
    });
    // No check that the marking says *something*: the grammar refuses
    // `tempo;` outright, and a file with a syntax error never reaches
    // elaboration. A diagnostic here would be one nothing could produce.
    Marking {
        metronome,
        ramp: tempo_ramp(resolver, tempo, text.is_some()),
        text,
    }
}

/// The gradual half of a tempo marking, when it has one.
///
/// Two halves that only mean something together: `to` says where the change
/// arrives, `over` says how far it reaches. One without the other is refused
/// rather than guessed at, because both guesses would be wrong — a reach with
/// no destination is not a change, and a destination with no reach is a jump
/// already written more simply without the word `to`.
fn tempo_ramp(resolver: &mut Resolver, tempo: &TempoStmt, printed: bool) -> Option<crate::score::Ramp> {
    let syntax = tempo.syntax();
    let arrives = tempo.ramp_to().and_then(|text| text.parse::<u32>().ok());
    let Some(over) = tempo.over().and_then(|text| parse_ratio(&text)) else {
        if arrives.is_some() {
            resolver.report(
                Diagnostic::error(Code::Misplaced, "this gradual tempo change has no reach")
                    .at(trimmed_span(syntax), "`to` without `over`")
                    .help("say how far it takes, like `tempo quarter = 120 to 60 over 4/1;`")
                    .note("a change with no reach is a change at a point, written without `to`"),
            );
        }
        return None;
    };
    if arrives.is_none() && !printed {
        resolver.report(
            Diagnostic::error(Code::Misplaced, "this gradual tempo change goes nowhere")
                .at(trimmed_span(syntax), "`over` with neither a destination nor a word")
                .help("write where it arrives (`to 60`) or what to print (`\"rit.\"`)"),
        );
        return None;
    }
    Some(crate::score::Ramp {
        to: arrives,
        over: crate::time::MusicalDuration::new(over),
        // The grammar writes no shape, so every ramp is a straight line — in
        // seconds per beat, which is where the evenness a listener hears
        // lives. The value is in the track rather than invented during
        // lowering, so a second implementation integrates the same curve.
        shape: musa_kernel::Progress::linear(),
    })
}

pub(crate) fn parse_meter(meter: &musa_language::ast::MeterStmt) -> Option<Meter> {
    if meter.is_unmeasured() {
        return Some(Meter::NONE);
    }
    let text = meter.value()?;
    let (numerator, denominator) = text.split_once('/')?;
    let (numerator, denominator): (u32, u32) = (numerator.parse().ok()?, denominator.parse().ok()?);
    // `4/0` is not a meter and `0/4` is spelled `none`: a fraction here has to
    // name real measures, or the barlines would fall nowhere by accident
    // rather than on purpose.
    if numerator == 0 || denominator == 0 {
        return None;
    }
    Some(Meter::new(numerator, denominator))
}

pub(crate) fn parse_key(key: &KeyStmt) -> Option<Key> {
    let syntax = key.syntax();
    // The tonic is a node, not a token: `bb` is one identifier and `g#` is
    // two tokens, and only the node knows it is one pitch class either way.
    let tonic = PitchClass::parse(
        syntax
            .children()
            .find(|child| child.kind() == SyntaxKind::PitchClass)?
            .text()
            .to_string()
            .trim(),
    )?;
    let mut identifiers = syntax
        .children_with_tokens()
        .filter_map(SyntaxElement::into_token)
        .filter(|token| token.kind() == SyntaxKind::Identifier);
    let mode = match identifiers.next()?.text() {
        "major" => Mode::Major,
        "minor" => Mode::Minor,
        _ => return None,
    };
    Some(Key::new(tonic, mode))
}

pub(crate) fn parse_ratio(text: &str) -> Option<Ratio<i64>> {
    let (numerator, denominator) = text.split_once('/')?;
    Some(Ratio::new(numerator.parse().ok()?, denominator.parse().ok()?))
}

/// Parse a statement's duration (`1/2`, `3/8`, `1`, `/4`, `/4.`) into value +
/// spelling.
///
/// It reads the statement's `Duration` node rather than its first numeral,
/// which is what makes the short form safe: the `4` in `c4/4` is a numeral
/// like any other, and a search for one would find the octave and call every
/// quarter a whole note without ever failing.
///
/// The short form is spelled out — `/4.` records `3/8` — because that spelling
/// reaches diagnostics, the desktop inspector and every kernel golden, and one
/// duration must not arrive there under two names.
pub(crate) fn parse_duration(node: &SyntaxNode) -> Option<NotatedDuration> {
    let text = musa_language::ast::Duration::of(node)?.value()?;
    let value = if text.contains('/') {
        parse_ratio(&text)?
    } else {
        Ratio::from_integer(text.parse().ok()?)
    };
    Some(NotatedDuration::single(MusicalDuration::new(value), text))
}
