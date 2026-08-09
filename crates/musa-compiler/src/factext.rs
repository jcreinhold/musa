//! `ScoreFact`'s interchange text form (docs/kernel/01, prompt 48).
//!
//! The kernel carries payloads as opaque quoted strings (§12); this module is
//! the other half — the one place that says what a musical fact looks like in
//! a `.kernel` file, and the only place that reads one back.
//!
//! **This is not `Canonical::canonical_key`, and it cannot be.** The key is
//! the *equality* serialization (N3): it deliberately omits the definition
//! span and the declaration id, because two facts that differ only in those
//! are the same fact for ordering, equality, and hashing. An interchange form
//! must reproduce the value exactly, so it carries them. One function serving
//! both would have to be the union, which would put provenance detail into
//! semantic equality — a fact would stop being equal to itself compiled from a
//! reformatted source. Two functions, and the reason is the layering, not
//! convenience.
//!
//! The format is `|`-delimited with `@`-delimited subfields; free text
//! (names, chord symbols, spellings) escapes `\`, `|` and `@`. Everything is
//! exact: rationals as `p/q`, never a float, because a consumer that reads a
//! hairpin's shape and rounds it produces different sound from the same file.

use musa_kernel::{Canonical as _, TextPayload};
use num_rational::Ratio;

use crate::elaborate::{FactKind, ScoreFact};
use crate::harmony::ChordSymbol;
use crate::marks::Mark;
use crate::origin::{DeclarationId, ExpansionStep, Interval, Origin, SourceSpan};
use crate::pitch::{PitchClass, WrittenPitch};
use crate::scope::Scope;
use crate::score::{DynamicMark, Mode, NotatedDuration};
use crate::time::MusicalDuration;

impl TextPayload for ScoreFact {
    fn to_text(&self) -> String {
        join(
            &[
                scope_text(self.scope),
                kind_text(&self.kind),
                span_text(self.origin.source_span),
                span_text(self.origin.definition_span),
                self.origin.declaration.0.to_string(),
                path_text(&self.origin.expansion_path),
            ],
            '|',
        )
    }

    fn from_text(text: &str) -> Option<Self> {
        let fields = split_escaped(text, '|');
        let [scope, kind, source, definition, declaration, path] = fields.as_slice() else {
            return None;
        };
        Some(Self {
            scope: read_scope(scope)?,
            kind: read_kind(kind)?,
            origin: Origin {
                source_span: read_span(source)?,
                definition_span: read_span(definition)?,
                declaration: DeclarationId(declaration.parse().ok()?),
                expansion_path: read_path(path)?,
            },
            tied: false,
        })
    }

    fn type_name() -> &'static str {
        "ScoreFact"
    }
}

// The `tied` flag is deliberately absent. It is elaboration-only and false on
// every fact that leaves `elaborate_items` — a tie says two noteheads spell
// one occurrence, and that is resolved before a timeline exists. A file that
// carried it would be describing a state no timeline is ever in.

pub(crate) fn scope_text(scope: Scope) -> String {
    match scope {
        Scope::Piece => "piece".to_owned(),
        Scope::Part { part } => join(&["part".to_owned(), part.to_string()], '@'),
        Scope::Voice { part, voice } => join(&["voice".to_owned(), part.to_string(), voice.to_string()], '@'),
    }
}

pub(crate) fn read_scope(text: &str) -> Option<Scope> {
    let fields = split_escaped(text, '@');
    match fields.as_slice() {
        [tag] if tag == "piece" => Some(Scope::Piece),
        [tag, part] if tag == "part" => Some(Scope::Part {
            part: part.parse().ok()?,
        }),
        [tag, part, voice] if tag == "voice" => Some(Scope::Voice {
            part: part.parse().ok()?,
            voice: voice.parse().ok()?,
        }),
        _ => None,
    }
}

pub(crate) fn span_text(span: SourceSpan) -> String {
    join(&[span.start.to_string(), span.end.to_string()], ':')
}

pub(crate) fn read_span(text: &str) -> Option<SourceSpan> {
    let fields = split_escaped(text, ':');
    let [start, end] = fields.as_slice() else {
        return None;
    };
    Some(SourceSpan::new(start.parse().ok()?, end.parse().ok()?))
}

fn ratio_text(value: Ratio<i64>) -> String {
    format!("{}/{}", value.numer(), value.denom())
}

fn read_ratio(text: &str) -> Option<Ratio<i64>> {
    let (numer, denom) = text.split_once('/')?;
    let numer: i64 = numer.parse().ok()?;
    let denom: i64 = denom.parse().ok()?;
    (denom != 0).then(|| Ratio::new(numer, denom))
}

fn duration_text(duration: &NotatedDuration) -> String {
    let pieces: Vec<String> = duration
        .pieces
        .iter()
        .map(|piece| ratio_text(piece.as_ratio()))
        .collect();
    join(
        &[
            duration.spelling.clone(),
            ratio_text(duration.value.as_ratio()),
            join(&pieces, ','),
        ],
        ';',
    )
}

fn read_duration(text: &str) -> Option<NotatedDuration> {
    let fields = split_escaped(text, ';');
    let [spelling, value, pieces] = fields.as_slice() else {
        return None;
    };
    let pieces: Option<Vec<MusicalDuration>> = split_escaped(pieces, ',')
        .iter()
        .map(|piece| read_ratio(piece).map(MusicalDuration::new))
        .collect();
    Some(NotatedDuration {
        value: MusicalDuration::new(read_ratio(value)?),
        spelling: spelling.clone(),
        pieces: pieces?,
    })
}

/// A free duration, as `least;most` — or empty, which is the common case of a
/// note whose written value is the value it sounds.
fn free_text(free: Option<&crate::score::FreeDuration>) -> String {
    free.map_or_else(String::new, |free| {
        join(
            &[ratio_text(free.least.as_ratio()), ratio_text(free.most.as_ratio())],
            ';',
        )
    })
}

/// Reads what `free_text` wrote for a note that does have a freedom. Absence
/// is the empty field, and the two callers spell that case out: a malformed
/// field is a broken fact rather than a missing one, so the two must not
/// collapse into the same `None`.
fn read_free(text: &str) -> Option<crate::score::FreeDuration> {
    let fields = split_escaped(text, ';');
    let [least, most] = fields.as_slice() else {
        return None;
    };
    Some(crate::score::FreeDuration {
        least: MusicalDuration::new(read_ratio(least)?),
        most: MusicalDuration::new(read_ratio(most)?),
    })
}

fn articulations_text(marks: &[Mark]) -> String {
    let names: Vec<String> = marks.iter().map(|mark| mark.name().to_owned()).collect();
    join(&names, ',')
}

fn read_articulations(text: &str) -> Option<Vec<Mark>> {
    if text.is_empty() {
        return Some(Vec::new());
    }
    split_escaped(text, ',').iter().map(|name| Mark::parse(name)).collect()
}

fn kind_text(kind: &FactKind) -> String {
    let fields: Vec<String> = match kind {
        FactKind::Note {
            pitch,
            duration,
            articulations,
            free,
        } => vec![
            "note".to_owned(),
            pitch.to_string(),
            duration_text(duration),
            articulations_text(articulations),
            free_text(free.as_ref()),
        ],
        FactKind::Rest {
            duration,
            articulations,
            free,
        } => vec![
            "rest".to_owned(),
            duration_text(duration),
            articulations_text(articulations),
            free_text(free.as_ref()),
        ],
        // Three fields rather than two so the absent argument and the empty
        // string stay distinguishable: `mark text ""` is a legal, if odd,
        // direction, and it is not the same fact as `mark breath`.
        FactKind::Mark { mark, argument } => vec![
            "mark".to_owned(),
            mark.name().to_owned(),
            match argument {
                Some(crate::marks::MarkArgument::Text(text)) => format!("t{text}"),
                Some(crate::marks::MarkArgument::Number(number)) => format!("n{number}"),
                None => String::new(),
            },
        ],
        FactKind::Slur => vec!["slur".to_owned()],
        FactKind::Phrase { name } => vec!["phrase".to_owned(), name.clone()],
        FactKind::Tuplet { num, den } => vec!["tuplet".to_owned(), num.to_string(), den.to_string()],
        FactKind::Dynamic { mark } => vec!["dynamic".to_owned(), mark.name().to_owned()],
        FactKind::Hairpin { grows, target, shape } => vec![
            "hairpin".to_owned(),
            if *grows { "cres" } else { "dim" }.to_owned(),
            target.name().to_owned(),
            shape.canonical_key(),
        ],
        FactKind::Key { tonic, mode } => vec![
            "key".to_owned(),
            tonic.to_string(),
            match mode {
                Mode::Major => "major",
                Mode::Minor => "minor",
            }
            .to_owned(),
        ],
        FactKind::Meter { numerator, denominator } => {
            vec!["meter".to_owned(), numerator.to_string(), denominator.to_string()]
        }
        FactKind::Clef { clef } => vec!["clef".to_owned(), clef.name().to_owned()],
        FactKind::Section { name } => vec!["section".to_owned(), name.clone()],
        FactKind::Harmony { symbol } => vec!["harmony".to_owned(), symbol.text.clone()],
        FactKind::Repeat { times } => vec!["repeat".to_owned(), times.to_string()],
        FactKind::Ending { bracket, pass } => {
            vec!["ending".to_owned(), bracket.to_string(), pass.to_string()]
        }
        FactKind::Mobile { fragments, order } => vec![
            "mobile".to_owned(),
            join(fragments, ','),
            join(&order.iter().map(u32::to_string).collect::<Vec<_>>(), ','),
        ],
        FactKind::Improvise { over } => {
            vec!["improvise".to_owned(), over.clone().unwrap_or_default()]
        }
    };
    join(&fields, '@')
}

fn read_kind(text: &str) -> Option<FactKind> {
    let fields = split_escaped(text, '@');
    let tag = fields.first()?.as_str();
    let arg = |index: usize| fields.get(index).map(String::as_str);
    match (tag, fields.len()) {
        ("note", 5) => Some(FactKind::Note {
            pitch: WrittenPitch::parse(arg(1)?)?,
            duration: read_duration(arg(2)?)?,
            articulations: read_articulations(arg(3)?)?,
            free: match arg(4)? {
                "" => None,
                free => Some(read_free(free)?),
            },
        }),
        ("rest", 4) => Some(FactKind::Rest {
            duration: read_duration(arg(1)?)?,
            articulations: read_articulations(arg(2)?)?,
            free: match arg(3)? {
                "" => None,
                free => Some(read_free(free)?),
            },
        }),
        ("mobile", 3) => Some(FactKind::Mobile {
            fragments: split_escaped(arg(1)?, ','),
            order: split_escaped(arg(2)?, ',')
                .iter()
                .map(|index| index.parse().ok())
                .collect::<Option<Vec<u32>>>()?,
        }),
        ("improvise", 2) => Some(FactKind::Improvise {
            over: Some(arg(1)?.to_owned()).filter(|over| !over.is_empty()),
        }),
        ("mark", 3) => Some(FactKind::Mark {
            mark: Mark::parse(arg(1)?)?,
            argument: match arg(2)? {
                "" => None,
                written => Some(match written.split_at_checked(1)? {
                    ("t", text) => crate::marks::MarkArgument::Text(text.to_owned()),
                    ("n", number) => crate::marks::MarkArgument::Number(number.parse().ok()?),
                    _ => return None,
                }),
            },
        }),
        ("slur", 1) => Some(FactKind::Slur),
        ("phrase", 2) => Some(FactKind::Phrase {
            name: arg(1)?.to_owned(),
        }),
        ("tuplet", 3) => Some(FactKind::Tuplet {
            num: arg(1)?.parse().ok()?,
            den: arg(2)?.parse().ok()?,
        }),
        ("dynamic", 2) => Some(FactKind::Dynamic {
            mark: DynamicMark::parse(arg(1)?)?,
        }),
        ("hairpin", 4) => Some(FactKind::Hairpin {
            grows: match arg(1)? {
                "cres" => true,
                "dim" => false,
                _ => return None,
            },
            target: DynamicMark::parse(arg(2)?)?,
            shape: read_progress(arg(3)?)?,
        }),
        ("key", 3) => Some(FactKind::Key {
            tonic: PitchClass::parse(arg(1)?)?,
            mode: match arg(2)? {
                "major" => Mode::Major,
                "minor" => Mode::Minor,
                _ => return None,
            },
        }),
        ("meter", 3) => Some(FactKind::Meter {
            numerator: arg(1)?.parse().ok()?,
            denominator: arg(2)?.parse().ok()?,
        }),
        ("clef", 2) => Some(FactKind::Clef {
            clef: crate::Clef::parse(arg(1)?)?,
        }),
        ("section", 2) => Some(FactKind::Section {
            name: arg(1)?.to_owned(),
        }),
        ("harmony", 2) => Some(FactKind::Harmony {
            symbol: ChordSymbol::parse(arg(1)?)?,
        }),
        ("repeat", 2) => Some(FactKind::Repeat {
            times: arg(1)?.parse().ok()?,
        }),
        ("ending", 3) => Some(FactKind::Ending {
            bracket: arg(1)?.parse().ok()?,
            pass: arg(2)?.parse().ok()?,
        }),
        _ => None,
    }
}

/// A `Progress` from its canonical key (`u/d:v/e,…`).
///
/// The canonical key *is* the text form here, because `Progress` has no
/// provenance to omit: N3's injectivity and the round-trip property coincide,
/// which is exactly the case the prompt asked to be checked before writing a
/// second function.
fn read_progress(text: &str) -> Option<musa_kernel::Progress> {
    let points: Option<Vec<(Ratio<i64>, Ratio<i64>)>> = text
        .split(',')
        .map(|point| {
            let (u, v) = point.split_once(':')?;
            Some((read_ratio(u)?, read_ratio(v)?))
        })
        .collect();
    musa_kernel::Progress::piecewise(points?)
}

fn path_text(steps: &[ExpansionStep]) -> String {
    let steps: Vec<String> = steps.iter().map(step_text).collect();
    join(&steps, ',')
}

pub(crate) fn step_text(step: &ExpansionStep) -> String {
    let fields: Vec<String> = match step {
        ExpansionStep::MotifApplication { call_site } => vec!["motif".to_owned(), span_text(*call_site)],
        ExpansionStep::RepeatIteration(index) => vec!["repeat".to_owned(), index.to_string()],
        ExpansionStep::Transposition(interval) => vec![
            "transpose".to_owned(),
            interval.diatonic_steps.to_string(),
            interval.semitones.to_string(),
        ],
        ExpansionStep::Stretch(factor) => vec!["stretch".to_owned(), ratio_text(*factor)],
        ExpansionStep::Retrograde => vec!["retrograde".to_owned()],
        ExpansionStep::Inversion { axis } => vec!["invert".to_owned(), axis.clone()],
        ExpansionStep::Specialization { override_site } => vec!["special".to_owned(), span_text(*override_site)],
    };
    join(&fields, ':')
}

fn read_path(text: &str) -> Option<Vec<ExpansionStep>> {
    if text.is_empty() {
        return Some(Vec::new());
    }
    split_escaped(text, ',').iter().map(|step| read_step(step)).collect()
}

pub(crate) fn read_step(text: &str) -> Option<ExpansionStep> {
    let fields = split_escaped(text, ':');
    let tag = fields.first()?.as_str();
    let arg = |index: usize| fields.get(index).map(String::as_str);
    match (tag, fields.len()) {
        ("motif", 2) => Some(ExpansionStep::MotifApplication {
            call_site: read_span(arg(1)?)?,
        }),
        ("repeat", 2) => Some(ExpansionStep::RepeatIteration(arg(1)?.parse().ok()?)),
        ("transpose", 3) => Some(ExpansionStep::Transposition(Interval {
            diatonic_steps: arg(1)?.parse().ok()?,
            semitones: arg(2)?.parse().ok()?,
        })),
        ("stretch", 2) => Some(ExpansionStep::Stretch(read_ratio(arg(1)?)?)),
        ("retrograde", 1) => Some(ExpansionStep::Retrograde),
        ("invert", 2) => Some(ExpansionStep::Inversion {
            axis: arg(1)?.to_owned(),
        }),
        ("special", 2) => Some(ExpansionStep::Specialization {
            override_site: read_span(arg(1)?)?,
        }),
        _ => None,
    }
}

/// Join fields with `separator`, escaping it (and the escape character) in
/// each.
///
/// The format nests — a path is inside a field, a step is inside a path — and
/// this is what makes the nesting compose: an inner join's output is one
/// field to the outer join, which escapes it again. Every level unescapes
/// exactly the level it splits, so the layers stay independent and no
/// producer needs to know how deeply it is nested.
pub(crate) fn join(fields: &[String], separator: char) -> String {
    fields
        .iter()
        .map(|field| escape(field, separator))
        .collect::<Vec<_>>()
        .join(&separator.to_string())
}

fn escape(text: &str, separator: char) -> String {
    let mut out = String::with_capacity(text.len());
    for character in text.chars() {
        if character == '\\' || character == separator {
            out.push('\\');
        }
        out.push(character);
    }
    out
}

/// The inverse of [`join`]: split on `separator`, honouring `\` escapes, and
/// unescape each field.
pub(crate) fn split_escaped(text: &str, separator: char) -> Vec<String> {
    let mut fields = vec![String::new()];
    let mut escaped = false;
    for character in text.chars() {
        let Some(current) = fields.last_mut() else {
            break;
        };
        if escaped {
            current.push(character);
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else if character == separator {
            fields.push(String::new());
        } else {
            current.push(character);
        }
    }
    fields
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::score::Mode;
    use crate::time::MusicalDuration;

    fn origin() -> Origin {
        Origin {
            source_span: SourceSpan::new(7, 19),
            definition_span: SourceSpan::new(103, 111),
            declaration: DeclarationId(4),
            expansion_path: vec![
                ExpansionStep::MotifApplication {
                    call_site: SourceSpan::new(7, 19),
                },
                ExpansionStep::RepeatIteration(2),
                ExpansionStep::Transposition(Interval {
                    diatonic_steps: -4,
                    semitones: -7,
                }),
                ExpansionStep::Stretch(Ratio::new(3, 2)),
                ExpansionStep::Retrograde,
                // The axis is free text, and it is written with every
                // character this format separates on.
                ExpansionStep::Inversion {
                    axis: "c4|d4@e4,f4:g4\\h4".to_owned(),
                },
                ExpansionStep::Specialization {
                    override_site: SourceSpan::new(0, 1),
                },
            ],
        }
    }

    fn duration() -> NotatedDuration {
        NotatedDuration {
            value: MusicalDuration::new(Ratio::new(1, 3)),
            spelling: "1/4 ~ 1/12".to_owned(),
            pieces: vec![
                MusicalDuration::new(Ratio::new(1, 4)),
                MusicalDuration::new(Ratio::new(1, 12)),
            ],
        }
    }

    /// A hairpin shape that is multi-segment *and* non-dyadic. `1/3` has no
    /// exact binary expansion, so a text form that went through `f64` would
    /// write `0.3333…` and read back something else — the failure prompt 45
    /// exists to make impossible, made visible here.
    fn awkward_shape() -> musa_kernel::Progress {
        musa_kernel::Progress::piecewise(vec![
            (Ratio::new(0, 1), Ratio::new(0, 1)),
            (Ratio::new(1, 3), Ratio::new(1, 7)),
            (Ratio::new(5, 7), Ratio::new(2, 3)),
            (Ratio::new(1, 1), Ratio::new(1, 1)),
        ])
        .unwrap_or_else(musa_kernel::Progress::linear)
    }

    /// One fact of every kind, each carrying the awkward provenance above —
    /// so a missed escape in any arm fails rather than a lucky one passing.
    fn corpus() -> Option<Vec<ScoreFact>> {
        let kinds = vec![
            FactKind::Note {
                pitch: WrittenPitch::parse("cs5")?,
                duration: duration(),
                articulations: vec![Mark::parse("staccato")?, Mark::parse("accent")?],
                free: None,
            },
            FactKind::Rest {
                duration: duration(),
                articulations: Vec::new(),
                free: Some(crate::score::FreeDuration {
                    least: MusicalDuration::new(Ratio::new(1, 4)),
                    most: MusicalDuration::new(Ratio::new(2, 1)),
                }),
            },
            FactKind::Mobile {
                fragments: vec!["a|name@with,commas".to_owned(), "b".to_owned()],
                order: vec![1, 0],
            },
            FactKind::Improvise {
                over: Some("Dm7 | G7".to_owned()),
            },
            FactKind::Slur,
            FactKind::Phrase {
                name: "a name with|pipes@ats, commas: and \\slashes".to_owned(),
            },
            FactKind::Tuplet { num: 3, den: 2 },
            FactKind::Dynamic { mark: DynamicMark::Sfz },
            FactKind::Hairpin {
                grows: false,
                target: DynamicMark::Ppp,
                shape: awkward_shape(),
            },
            FactKind::Key {
                tonic: PitchClass::parse("bf")?,
                mode: Mode::Minor,
            },
            FactKind::Meter {
                numerator: 7,
                denominator: 8,
            },
            FactKind::Section { name: String::new() },
            FactKind::Harmony {
                symbol: ChordSymbol::parse("fmaj7")?,
            },
            FactKind::Repeat { times: 4 },
            FactKind::Ending { bracket: 2, pass: 3 },
        ];
        let scopes = [Scope::Piece, Scope::Voice { part: 2, voice: 11 }];
        Some(
            kinds
                .into_iter()
                .zip(scopes.into_iter().cycle())
                .map(|(kind, scope)| ScoreFact {
                    scope,
                    kind,
                    origin: origin(),
                    tied: false,
                })
                .collect(),
        )
    }

    /// The round-trip law at the payload layer (N3): the text form is
    /// injective, which is what makes it readable at all.
    #[test]
    fn a_facts_text_form_round_trips() {
        let corpus = corpus();
        assert!(corpus.is_some(), "the corpus itself does not parse");
        for fact in corpus.into_iter().flatten() {
            let text = fact.to_text();
            assert_eq!(
                ScoreFact::from_text(&text).as_ref(),
                Some(&fact),
                "did not round trip: {text}"
            );
        }
    }

    /// No decimal point anywhere: every rational in a payload — durations,
    /// stretch factors, hairpin shapes — is written as `p/q`.
    #[test]
    fn a_facts_text_form_writes_no_decimals() {
        for fact in corpus().into_iter().flatten() {
            let text = fact.to_text();
            assert!(!text.contains('.'), "a rational was written as a decimal: {text}");
        }
    }

    /// Distinct facts have distinct text, including the two that
    /// `canonical_key` deliberately conflates. This is the difference that
    /// made the interchange form a second function rather than the key.
    #[test]
    fn facts_differing_only_in_provenance_have_different_text() {
        let mut a = ScoreFact {
            scope: Scope::Piece,
            kind: FactKind::Slur,
            origin: origin(),
            tied: false,
        };
        let mut b = a.clone();
        b.origin.definition_span = SourceSpan::new(500, 501);
        assert_ne!(a.to_text(), b.to_text(), "the definition span was dropped");
        a.origin.declaration = DeclarationId(9);
        assert_ne!(a.to_text(), b.to_text(), "the declaration was dropped");
    }
}
