//! `ScoreFact`'s interchange text form (docs/kernel/01).
//!
//! The kernel carries payloads as opaque quoted strings (§12); this module is
//! the other half — the one place that says what a musical fact looks like in
//! a `.musa.kernel` file, and the only place that reads one back.
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
//! # The form
//!
//! A label is a **flat, whitespace-separated stream of words**, and flat is
//! load-bearing. The form this replaced nested five separators five deep and
//! escaped each level again at the next, so one colon inside a motif call
//! reached the file as eight backslashes. Nothing here nests, so nothing is
//! escaped twice.
//!
//! A word is either **bare** — no whitespace, no `'`, no `\`, no bracket —
//! used for pitches, ratios, integers and vocabulary names; or **quoted** —
//! `'…'`, escaping `\` and `'` — used for *every* free-text field, always,
//! even where quoting would not be needed. Always, because that is what keeps
//! `mark text '8'` and `mark ottava 8` apart without case analysis, and
//! because a payload that never contains `"` gives the kernel's own string
//! escape nothing to double.
//!
//! ```text
//! label := scope kind origin
//! scope := "piece" | "part" N | "voice" N N
//! origin := "[" span ("def" span)? ("#" N)? ("via" step+)? "]"
//! span := N ":" N
//! ```
//!
//! Provenance elides only what reading can reconstruct exactly, and every
//! elision is a biconditional rather than a guess: `def` is written iff the
//! definition span differs from the source span, `#n` iff the declaration is
//! not zero, `via` iff the expansion path is non-empty. The span itself is
//! never elided, so `[0:0]` is written as it stands.
//!
//! Everything is exact: rationals as `p/q` — or `p`, when the denominator is
//! one — and never a float, because a consumer that reads a hairpin's shape
//! and rounds it produces different sound from the same file.

use musa_kernel::{Canonical as _, PayloadText, TextPayload};
use num_rational::Ratio;

use crate::elaborate::{FactKind, ScoreFact};
use crate::harmony::ChordSymbol;
use crate::marks::{Mark, MarkArgument};
use crate::origin::{DeclarationId, ExpansionStep, Interval, Origin, SourceSpan};
use crate::pitch::{PitchClass, WrittenPitch};
use crate::scope::Scope;
use crate::score::{DynamicMark, FreeDuration, Metronome, Mode, NotatedDuration, Ramp};
use crate::time::MusicalDuration;

impl PayloadText for ScoreFact {
    fn to_text(&self) -> String {
        let mut words = Words::default();
        write_scope(&mut words, self.scope);
        write_kind(&mut words, &self.kind);
        write_origin(&mut words, &self.origin);
        words.finish()
    }

    fn from_text(text: &str) -> Option<Self> {
        let mut words = Words::split(text)?;
        let scope = take_scope(&mut words)?;
        let kind = take_kind(&mut words)?;
        let origin = take_origin(&mut words)?;
        words.end().then_some(Self {
            scope,
            kind,
            origin,
            tied: false,
        })
    }
}

impl TextPayload for ScoreFact {
    fn type_name() -> &'static str {
        "ScoreFact"
    }
}

// The `tied` flag is deliberately absent. It is elaboration-only and false on
// every fact that leaves `elaborate_items` — a tie says two noteheads spell
// one occurrence, and that is resolved before a timeline exists. A file that
// carried it would be describing a state no timeline is ever in.

/// What a reference's mark rewrites when its body is instantiated (E-Mark).
///
/// The same word stream as a label, so there is one tokenizer and one escape
/// rule in this crate rather than two that must agree:
/// `depth N ("origin" span)? ("scope" scope)? ("via" step+)?`.
pub(crate) struct ReferenceMark {
    /// Where in the expansion path the steps belong.
    pub(crate) depth: usize,
    /// The steps to insert there.
    pub(crate) steps: Vec<ExpansionStep>,
    /// The span to give payloads that were elaborated for sharing, if any.
    pub(crate) origin: Option<SourceSpan>,
    /// The scope to give them, if any.
    pub(crate) scope: Option<Scope>,
}

/// Write a reference mark. See [`ReferenceMark`] for the shape.
pub(crate) fn reference_mark(mark: &ReferenceMark) -> String {
    let mut words = Words::default();
    words.word("depth");
    words.word(mark.depth.to_string());
    if let Some(span) = mark.origin {
        words.word("origin");
        words.word(span_word(span));
    }
    if let Some(scope) = mark.scope {
        words.word("scope");
        write_scope(&mut words, scope);
    }
    if !mark.steps.is_empty() {
        words.word("via");
        for step in &mark.steps {
            write_step(&mut words, step);
        }
    }
    words.finish()
}

/// Read what [`reference_mark`] wrote, or `None` if it was not written by it.
pub(crate) fn read_reference_mark(text: &str) -> Option<ReferenceMark> {
    let mut words = Words::split(text)?;
    if !words.keyword("depth") {
        return None;
    }
    let depth = words.integer()?;
    let origin = if words.keyword("origin") {
        Some(words.span()?)
    } else {
        None
    };
    let scope = if words.keyword("scope") {
        Some(take_scope(&mut words)?)
    } else {
        None
    };
    let steps = if words.keyword("via") {
        take_steps(&mut words)?
    } else {
        Vec::new()
    };
    words.end().then_some(ReferenceMark {
        depth,
        steps,
        origin,
        scope,
    })
}

// ---------------------------------------------------------------------------
// Words: the writer, the reader, and the one escape rule
// ---------------------------------------------------------------------------

/// A word and whether it was written quoted.
///
/// The flag is not decoration: `mark text '8'` and `mark ottava 8` are
/// different facts whose only difference is this bit.
#[derive(Clone, Debug)]
struct Word {
    text: String,
    quoted: bool,
}

/// A payload as a sequence of words — a writer when built up, a reader when
/// split, and the same escape rule read in both directions.
#[derive(Default)]
struct Words {
    words: Vec<Word>,
    at: usize,
}

/// `[` and `]` delimit themselves, so `[191:198 #4]` reads as four words
/// without the spaces that would otherwise be needed to separate them.
const PUNCTUATION: [char; 2] = ['[', ']'];

impl Words {
    /// Append a bare word: no whitespace, no `'`, no `\`, no bracket.
    fn word(&mut self, word: impl Into<String>) {
        self.words.push(Word {
            text: word.into(),
            quoted: false,
        });
    }

    /// Append a free-text word, quoted whether or not it needs to be.
    fn text(&mut self, text: &str) {
        self.words.push(Word {
            text: text.to_owned(),
            quoted: true,
        });
    }

    /// The words as one line, with a space between each pair except where a
    /// bracket sits — `[191:198]`, not `[ 191:198 ]`.
    fn finish(&self) -> String {
        let mut out = String::new();
        // Also true at the start, where there is nothing to separate from.
        let mut after_open = true;
        for word in &self.words {
            let bracket = |text| !word.quoted && word.text == text;
            if !after_open && !bracket("]") {
                out.push(' ');
            }
            after_open = bracket("[");
            if word.quoted {
                out.push('\'');
                for character in word.text.chars() {
                    if character == '\\' || character == '\'' {
                        out.push('\\');
                    }
                    out.push(character);
                }
                out.push('\'');
            } else {
                out.push_str(&word.text);
            }
        }
        out
    }

    /// Split a payload back into words, or `None` if a quote is unterminated.
    fn split(text: &str) -> Option<Self> {
        let mut words = Vec::new();
        let mut characters = text.chars().peekable();
        while let Some(&character) = characters.peek() {
            if character.is_whitespace() {
                characters.next();
            } else if character == '\'' {
                characters.next();
                let mut quoted = String::new();
                loop {
                    match characters.next()? {
                        '\'' => break,
                        '\\' => quoted.push(characters.next()?),
                        character => quoted.push(character),
                    }
                }
                words.push(Word {
                    text: quoted,
                    quoted: true,
                });
            } else if PUNCTUATION.contains(&character) {
                characters.next();
                words.push(Word {
                    text: character.to_string(),
                    quoted: false,
                });
            } else {
                let mut bare = String::new();
                while let Some(&character) = characters.peek() {
                    if character.is_whitespace() || character == '\'' || PUNCTUATION.contains(&character) {
                        break;
                    }
                    bare.push(character);
                    characters.next();
                }
                words.push(Word {
                    text: bare,
                    quoted: false,
                });
            }
        }
        Some(Self { words, at: 0 })
    }

    /// The next word if it is bare, without consuming it.
    fn peek(&self) -> Option<&str> {
        self.words
            .get(self.at)
            .filter(|word| !word.quoted)
            .map(|word| word.text.as_str())
    }

    /// Whether the next word is quoted — free text rather than vocabulary.
    fn peek_text(&self) -> bool {
        self.words.get(self.at).is_some_and(|word| word.quoted)
    }

    /// Step past the word just peeked at.
    fn skip(&mut self) {
        self.at = self.at.saturating_add(1);
    }

    /// Consume the next word if it is bare and equal to `word`.
    fn keyword(&mut self, word: &str) -> bool {
        let matched = self.peek() == Some(word);
        if matched {
            self.skip();
        }
        matched
    }

    /// Consume the next word, which must be bare.
    fn bare(&mut self) -> Option<String> {
        let word = self.peek()?.to_owned();
        self.skip();
        Some(word)
    }

    /// Consume the next word, which must be quoted.
    fn quoted(&mut self) -> Option<String> {
        let word = self.words.get(self.at).filter(|word| word.quoted)?.text.clone();
        self.skip();
        Some(word)
    }

    fn integer<T: std::str::FromStr>(&mut self) -> Option<T> {
        self.bare()?.parse().ok()
    }

    fn ratio(&mut self) -> Option<Ratio<i64>> {
        read_ratio(&self.bare()?)
    }

    fn span(&mut self) -> Option<SourceSpan> {
        read_span(&self.bare()?)
    }

    /// A pair written `p/q` and kept unreduced — a meter and a tuplet ratio
    /// both mean the two numbers as written, not the fraction they make.
    fn pair(&mut self) -> Option<(u32, u32)> {
        let (left, right) = self
            .bare()?
            .split_once('/')
            .map(|(l, r)| (l.to_owned(), r.to_owned()))?;
        Some((left.parse().ok()?, right.parse().ok()?))
    }

    fn end(&self) -> bool {
        self.at == self.words.len()
    }
}

fn span_word(span: SourceSpan) -> String {
    format!("{}:{}", span.start, span.end)
}

fn read_span(text: &str) -> Option<SourceSpan> {
    let (start, end) = text.split_once(':')?;
    Some(SourceSpan::new(start.parse().ok()?, end.parse().ok()?))
}

/// A rational as `p/q`, or as `p` when the denominator is one — which is how
/// the language writes a whole note, and so what a duration's spelling has to
/// be compared against for the `spelled` elision to fire.
fn ratio_text(value: Ratio<i64>) -> String {
    if *value.denom() == 1 {
        value.numer().to_string()
    } else {
        format!("{}/{}", value.numer(), value.denom())
    }
}

fn read_ratio(text: &str) -> Option<Ratio<i64>> {
    match text.split_once('/') {
        Some((numer, denom)) => {
            let numer: i64 = numer.parse().ok()?;
            let denom: i64 = denom.parse().ok()?;
            (denom != 0).then(|| Ratio::new(numer, denom))
        }
        None => Some(Ratio::from_integer(text.parse().ok()?)),
    }
}

// ---------------------------------------------------------------------------
// Scope, origin, expansion steps
// ---------------------------------------------------------------------------

fn write_scope(words: &mut Words, scope: Scope) {
    match scope {
        Scope::Piece => words.word("piece"),
        Scope::Part { part } => {
            words.word("part");
            words.word(part.to_string());
        }
        // Two words rather than `part.voice`: a payload contains no decimal
        // point anywhere, which is what keeps every rational in it exact, and
        // a scope index is not worth weakening that to.
        Scope::Voice { part, voice } => {
            words.word("voice");
            words.word(part.to_string());
            words.word(voice.to_string());
        }
    }
}

fn take_scope(words: &mut Words) -> Option<Scope> {
    match words.bare()?.as_str() {
        "piece" => Some(Scope::Piece),
        "part" => Some(Scope::Part { part: words.integer()? }),
        "voice" => Some(Scope::Voice {
            part: words.integer()?,
            voice: words.integer()?,
        }),
        _ => None,
    }
}

fn write_origin(words: &mut Words, origin: &Origin) {
    words.word("[");
    words.word(span_word(origin.source_span));
    if origin.definition_span != origin.source_span {
        words.word("def");
        words.word(span_word(origin.definition_span));
    }
    if origin.declaration.0 != 0 {
        words.word(format!("#{}", origin.declaration.0));
    }
    if !origin.expansion_path.is_empty() {
        words.word("via");
        for step in &origin.expansion_path {
            write_step(words, step);
        }
    }
    words.word("]");
}

fn take_origin(words: &mut Words) -> Option<Origin> {
    if !words.keyword("[") {
        return None;
    }
    let source_span = words.span()?;
    let definition_span = if words.keyword("def") {
        words.span()?
    } else {
        source_span
    };
    let declaration = match words.peek().filter(|word| word.starts_with('#')) {
        Some(_) => DeclarationId(words.bare()?[1..].parse().ok()?),
        None => DeclarationId(0),
    };
    let expansion_path = if words.keyword("via") {
        take_steps(words)?
    } else {
        Vec::new()
    };
    words.keyword("]").then_some(Origin {
        source_span,
        definition_span,
        declaration,
        expansion_path,
    })
}

/// The words an expansion step can begin with, and so the words that continue
/// a `via` run. A run ends at the first word that is not one of these — `]`
/// in a label, and the end of the stream in a reference mark.
const STEP_TAGS: [&str; 11] = [
    "template",
    "motif",
    "scale",
    "assert",
    "repeat",
    "transpose",
    "stretch",
    "retrograde",
    "invert",
    "map-pitches",
    "special",
];

fn write_step(words: &mut Words, step: &ExpansionStep) {
    match step {
        ExpansionStep::TemplateInstance {
            template,
            alias,
            site,
            identity,
        } => {
            words.word("template");
            words.text(template);
            words.text(alias);
            words.word(span_word(*site));
            words.text(identity);
        }
        ExpansionStep::MotifApplication { call_site } => {
            words.word("motif");
            words.word(span_word(*call_site));
        }
        ExpansionStep::RepeatIteration(index) => {
            words.word("repeat");
            words.word(index.to_string());
        }
        ExpansionStep::Transposition(interval) => {
            words.word("transpose");
            words.word(interval.diatonic_steps.to_string());
            words.word(interval.semitones.to_string());
        }
        ExpansionStep::Stretch(factor) => {
            words.word("stretch");
            words.word(ratio_text(*factor));
        }
        ExpansionStep::Retrograde => words.word("retrograde"),
        ExpansionStep::Inversion { axis } => {
            words.word("invert");
            words.text(axis);
        }
        ExpansionStep::MapNotePitches => words.word("map-pitches"),
        ExpansionStep::ScaleContext { scale } => {
            words.word("scale");
            words.text(scale);
        }
        ExpansionStep::Assertion { claim } => {
            words.word("assert");
            words.text(claim);
        }
        ExpansionStep::Specialization { override_site } => {
            words.word("special");
            words.word(span_word(*override_site));
        }
    }
}

fn take_steps(words: &mut Words) -> Option<Vec<ExpansionStep>> {
    let mut steps = Vec::new();
    while words.peek().is_some_and(|word| STEP_TAGS.contains(&word)) {
        steps.push(take_step(words)?);
    }
    Some(steps)
}

fn take_step(words: &mut Words) -> Option<ExpansionStep> {
    match words.bare()?.as_str() {
        "template" => Some(ExpansionStep::TemplateInstance {
            template: words.quoted()?,
            alias: words.quoted()?,
            site: words.span()?,
            identity: words.quoted()?,
        }),
        "motif" => Some(ExpansionStep::MotifApplication {
            call_site: words.span()?,
        }),
        "repeat" => Some(ExpansionStep::RepeatIteration(words.integer()?)),
        "transpose" => Some(ExpansionStep::Transposition(Interval {
            diatonic_steps: words.integer()?,
            semitones: words.integer()?,
        })),
        "stretch" => Some(ExpansionStep::Stretch(words.ratio()?)),
        "retrograde" => Some(ExpansionStep::Retrograde),
        "invert" => Some(ExpansionStep::Inversion { axis: words.quoted()? }),
        "map-pitches" => Some(ExpansionStep::MapNotePitches),
        "scale" => Some(ExpansionStep::ScaleContext { scale: words.quoted()? }),
        "assert" => Some(ExpansionStep::Assertion { claim: words.quoted()? }),
        "special" => Some(ExpansionStep::Specialization {
            override_site: words.span()?,
        }),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Durations and articulations
// ---------------------------------------------------------------------------

/// `1/4`, and longer only where it must be:
/// `1/3 spelled '1/4 ~ 1/12' tied 1/4 1/12`.
fn write_duration(words: &mut Words, duration: &NotatedDuration) {
    let value = ratio_text(duration.value.as_ratio());
    words.word(value.clone());
    if duration.spelling != value {
        words.word("spelled");
        words.text(&duration.spelling);
    }
    if duration.pieces.as_slice() != [duration.value] {
        words.word("tied");
        for piece in &duration.pieces {
            words.word(ratio_text(piece.as_ratio()));
        }
    }
}

fn take_duration(words: &mut Words) -> Option<NotatedDuration> {
    let value = MusicalDuration::new(words.ratio()?);
    let spelling = if words.keyword("spelled") {
        words.quoted()?
    } else {
        ratio_text(value.as_ratio())
    };
    let pieces = if words.keyword("tied") {
        let mut pieces = Vec::new();
        while words.peek().and_then(read_ratio).is_some() {
            pieces.push(MusicalDuration::new(words.ratio()?));
        }
        pieces
    } else {
        vec![value]
    };
    Some(NotatedDuration {
        value,
        spelling,
        pieces,
    })
}

/// Articulation names, bare, in order. The run ends at `free` or at the `[`
/// that opens the origin, neither of which is a mark name.
fn write_articulations(words: &mut Words, marks: &[Mark]) {
    for mark in marks {
        words.word(mark.name());
    }
}

fn take_articulations(words: &mut Words) -> Option<Vec<Mark>> {
    let mut marks = Vec::new();
    while let Some(word) = words.peek() {
        if word == "free" || word == "[" {
            break;
        }
        marks.push(Mark::parse(word)?);
        words.skip();
    }
    Some(marks)
}

/// A note's freedom to be held, as `free least most` — absent for a note whose
/// written value is the value it sounds, which is nearly all of them.
fn write_free(words: &mut Words, free: Option<&FreeDuration>) {
    if let Some(free) = free {
        words.word("free");
        words.word(ratio_text(free.least.as_ratio()));
        words.word(ratio_text(free.most.as_ratio()));
    }
}

/// Reads what [`write_free`] wrote, after the caller has taken the `free` that
/// says there is one — so absence is the caller's `if`, and a malformed
/// freedom is a broken fact rather than a missing one.
fn take_free(words: &mut Words) -> Option<FreeDuration> {
    Some(FreeDuration {
        least: MusicalDuration::new(words.ratio()?),
        most: MusicalDuration::new(words.ratio()?),
    })
}

// ---------------------------------------------------------------------------
// The kinds
// ---------------------------------------------------------------------------

fn write_kind(words: &mut Words, kind: &FactKind) {
    match kind {
        FactKind::Note {
            pitch,
            duration,
            articulations,
            free,
        } => {
            words.word("note");
            words.word(pitch.to_string());
            write_duration(words, duration);
            write_articulations(words, articulations);
            write_free(words, free.as_ref());
        }
        FactKind::Rest {
            duration,
            articulations,
            free,
        } => {
            words.word("rest");
            write_duration(words, duration);
            write_articulations(words, articulations);
            write_free(words, free.as_ref());
        }
        // The argument's absence is its absence: `mark breath` has no third
        // word. `mark text ''` is a legal, if odd, direction and is a
        // different fact, which is why free text is always quoted and a
        // number never is.
        FactKind::Mark { mark, argument } => {
            words.word("mark");
            words.word(mark.name());
            match argument {
                Some(MarkArgument::Text(text)) => words.text(text),
                Some(MarkArgument::Number(number)) => words.word(number.to_string()),
                None => {}
            }
        }
        FactKind::Grace {
            pitch,
            articulations,
            index,
        } => {
            words.word("grace");
            words.word(pitch.to_string());
            words.word(index.to_string());
            write_articulations(words, articulations);
        }
        FactKind::Slur => words.word("slur"),
        FactKind::Phrase { name } => {
            words.word("phrase");
            words.text(name);
        }
        FactKind::Tuplet { num, den } => {
            words.word("tuplet");
            words.word(format!("{num}/{den}"));
        }
        FactKind::Dynamic { mark } => {
            words.word("dynamic");
            words.word(mark.name());
        }
        FactKind::Hairpin { grows, target, shape } => {
            words.word("hairpin");
            words.word(if *grows { "cres" } else { "dim" });
            words.word(target.name());
            words.word(shape.canonical_key());
        }
        FactKind::Key { tonic, mode } => {
            words.word("key");
            words.word(tonic.to_string());
            words.word(match mode {
                Mode::Major => "major",
                Mode::Minor => "minor",
            });
        }
        FactKind::Meter { numerator, denominator } => {
            words.word("meter");
            words.word(format!("{numerator}/{denominator}"));
        }
        FactKind::Clef { clef } => {
            words.word("clef");
            words.word(clef.name());
        }
        // A marking is a metronome mark, a word, or both, it may be gradual,
        // and a gradual one may print without saying where it arrives. Each
        // part is named, so a marking that has none of them is one word.
        FactKind::Tempo { metronome, text, ramp } => {
            words.word("tempo");
            if let Some(mark) = metronome {
                words.word(format!("{}={}", ratio_text(mark.beat), mark.bpm));
            }
            if let Some(text) = text {
                words.text(text);
            }
            if let Some(ramp) = ramp {
                if let Some(bpm) = ramp.to {
                    words.word("to");
                    words.word(bpm.to_string());
                }
                words.word("over");
                words.word(ratio_text(ramp.over.as_ratio()));
                words.word(ramp.shape.canonical_key());
            }
        }
        FactKind::Section { name } => {
            words.word("section");
            words.text(name);
        }
        FactKind::Harmony { symbol } => {
            words.word("harmony");
            words.text(symbol.text());
        }
        FactKind::Repeat { times, range } => {
            words.word("repeat");
            words.word(times.to_string());
            if let Some((least, most)) = range {
                words.word("from");
                words.word(least.to_string());
                words.word("to");
                words.word(most.to_string());
            }
        }
        FactKind::Ending { bracket, pass } => {
            words.word("ending");
            words.word(bracket.to_string());
            words.word("pass");
            words.word(pass.to_string());
        }
        FactKind::Mobile { fragments, order } => {
            words.word("mobile");
            for fragment in fragments {
                words.text(fragment);
            }
            words.word("order");
            for index in order {
                words.word(index.to_string());
            }
        }
        FactKind::Improvise { over } => {
            words.word("improvise");
            if let Some(over) = over {
                words.word("over");
                words.text(over);
            }
        }
    }
}

fn take_kind(words: &mut Words) -> Option<FactKind> {
    match words.bare()?.as_str() {
        "note" => Some(FactKind::Note {
            pitch: WrittenPitch::parse(&words.bare()?)?,
            duration: take_duration(words)?,
            articulations: take_articulations(words)?,
            free: if words.keyword("free") {
                Some(take_free(words)?)
            } else {
                None
            },
        }),
        "rest" => Some(FactKind::Rest {
            duration: take_duration(words)?,
            articulations: take_articulations(words)?,
            free: if words.keyword("free") {
                Some(take_free(words)?)
            } else {
                None
            },
        }),
        // A quoted argument is text, a bare one is a number, and the `[` that
        // opens the origin is neither: that is the whole of what "free text is
        // always quoted" buys, and it is why `mark text '8'` and
        // `mark ottava 8` need no case analysis to tell apart.
        "mark" => {
            let mark = Mark::parse(&words.bare()?)?;
            let argument = if words.peek_text() {
                Some(MarkArgument::Text(words.quoted()?))
            } else if words.peek().is_some_and(|word| word != "[") {
                Some(MarkArgument::Number(words.integer()?))
            } else {
                None
            };
            Some(FactKind::Mark { mark, argument })
        }
        "grace" => Some(FactKind::Grace {
            pitch: WrittenPitch::parse(&words.bare()?)?,
            index: words.integer()?,
            articulations: take_articulations(words)?,
        }),
        "slur" => Some(FactKind::Slur),
        "phrase" => Some(FactKind::Phrase { name: words.quoted()? }),
        "tuplet" => {
            let (num, den) = words.pair()?;
            Some(FactKind::Tuplet { num, den })
        }
        "dynamic" => Some(FactKind::Dynamic {
            mark: DynamicMark::parse(&words.bare()?)?,
        }),
        "hairpin" => Some(FactKind::Hairpin {
            grows: match words.bare()?.as_str() {
                "cres" => true,
                "dim" => false,
                _ => return None,
            },
            target: DynamicMark::parse(&words.bare()?)?,
            shape: read_progress(&words.bare()?)?,
        }),
        "key" => Some(FactKind::Key {
            tonic: PitchClass::parse(&words.bare()?)?,
            mode: match words.bare()?.as_str() {
                "major" => Mode::Major,
                "minor" => Mode::Minor,
                _ => return None,
            },
        }),
        "meter" => {
            let (numerator, denominator) = words.pair()?;
            Some(FactKind::Meter { numerator, denominator })
        }
        "clef" => Some(FactKind::Clef {
            clef: crate::Clef::parse(&words.bare()?)?,
        }),
        "tempo" => take_tempo(words),
        "section" => Some(FactKind::Section { name: words.quoted()? }),
        "harmony" => Some(FactKind::Harmony {
            symbol: ChordSymbol::parse(&words.quoted()?)?,
        }),
        "repeat" => Some(FactKind::Repeat {
            times: words.integer()?,
            range: if words.keyword("from") {
                let least = words.integer()?;
                if !words.keyword("to") {
                    return None;
                }
                Some((least, words.integer()?))
            } else {
                None
            },
        }),
        "ending" => Some(FactKind::Ending {
            bracket: words.integer()?,
            pass: {
                if !words.keyword("pass") {
                    return None;
                }
                words.integer()?
            },
        }),
        "mobile" => {
            let mut fragments = Vec::new();
            while words.peek_text() {
                fragments.push(words.quoted()?);
            }
            if !words.keyword("order") {
                return None;
            }
            let mut order = Vec::new();
            while words.peek().is_some_and(|word| word != "[") {
                order.push(words.integer()?);
            }
            Some(FactKind::Mobile { fragments, order })
        }
        "improvise" => Some(FactKind::Improvise {
            over: if words.keyword("over") {
                Some(words.quoted()?)
            } else {
                None
            },
        }),
        _ => None,
    }
}

fn take_tempo(words: &mut Words) -> Option<FactKind> {
    let metronome = match words.peek().filter(|word| word.contains('=')) {
        Some(_) => {
            let word = words.bare()?;
            let (beat, bpm) = word.split_once('=')?;
            Some(Metronome {
                beat: read_ratio(beat)?,
                bpm: bpm.parse().ok()?,
            })
        }
        None => None,
    };
    let text = if words.peek_text() { Some(words.quoted()?) } else { None };
    let to = if words.keyword("to") {
        Some(words.integer()?)
    } else {
        None
    };
    let ramp = if words.keyword("over") {
        Some(Ramp {
            to,
            over: MusicalDuration::new(words.ratio()?),
            shape: read_progress(&words.bare()?)?,
        })
    } else if to.is_some() {
        // `to` without `over` is a ramp that arrives nowhere in no time. The
        // writer cannot produce it, so reading it is reading a broken file.
        return None;
    } else {
        None
    };
    Some(FactKind::Tempo { metronome, text, ramp })
}

/// A `Progress` from its canonical key (`u/d:v/e,…`).
///
/// The canonical key *is* the text form here, because `Progress` has no
/// provenance to omit: N3's injectivity and the round-trip property coincide,
/// and it is one bare word with no whitespace in it.
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

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

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
                // character this format gives a meaning to.
                ExpansionStep::Inversion {
                    axis: "c4 d4 'e4' [f4] \\g4".to_owned(),
                },
                ExpansionStep::MapNotePitches,
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
    /// write `0.3333…` and read back something else — the failure this test
    /// makes visible.
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
                pitch: WrittenPitch::parse("c#5")?,
                duration: duration(),
                articulations: vec![Mark::parse("staccato")?, Mark::parse("accent")?],
                free: None,
            },
            FactKind::Rest {
                duration: duration(),
                articulations: Vec::new(),
                free: Some(FreeDuration {
                    least: MusicalDuration::new(Ratio::new(1, 4)),
                    most: MusicalDuration::new(Ratio::new(2, 1)),
                }),
            },
            // A whole note: its spelling is `1`, which is what `ratio_text`
            // writes, so `spelled` elides — the case a `p/q`-only writer would
            // have printed in full on every long note in the corpus.
            FactKind::Note {
                pitch: WrittenPitch::parse("c4")?,
                duration: NotatedDuration {
                    value: MusicalDuration::new(Ratio::new(1, 1)),
                    spelling: "1".to_owned(),
                    pieces: vec![MusicalDuration::new(Ratio::new(1, 1))],
                },
                articulations: Vec::new(),
                free: None,
            },
            FactKind::Mark {
                mark: Mark::parse("text")?,
                argument: Some(MarkArgument::Text("8".to_owned())),
            },
            FactKind::Mark {
                mark: Mark::parse("ottava")?,
                argument: Some(MarkArgument::Number(8)),
            },
            FactKind::Mark {
                mark: Mark::parse("breath")?,
                argument: None,
            },
            FactKind::Grace {
                pitch: WrittenPitch::parse("d5")?,
                articulations: vec![Mark::parse("accent")?],
                index: 1,
            },
            FactKind::Mobile {
                fragments: vec!["a name with 'quotes' and \\slashes".to_owned(), "b".to_owned()],
                order: vec![1, 0],
            },
            FactKind::Improvise {
                over: Some("Dm7 | G7".to_owned()),
            },
            FactKind::Improvise { over: None },
            FactKind::Slur,
            FactKind::Phrase {
                name: "a name with 'quotes', brackets [and] \\slashes".to_owned(),
            },
            FactKind::Tuplet { num: 3, den: 2 },
            FactKind::Dynamic { mark: DynamicMark::Sfz },
            FactKind::Hairpin {
                grows: false,
                target: DynamicMark::Ppp,
                shape: awkward_shape(),
            },
            FactKind::Key {
                tonic: PitchClass::parse("bb")?,
                mode: Mode::Minor,
            },
            FactKind::Meter {
                numerator: 7,
                denominator: 8,
            },
            FactKind::Tempo {
                metronome: Some(Metronome {
                    beat: Ratio::new(1, 1),
                    bpm: 60,
                }),
                text: Some("rit.".to_owned()),
                ramp: Some(Ramp {
                    to: Some(30),
                    over: MusicalDuration::new(Ratio::new(2, 1)),
                    shape: awkward_shape(),
                }),
            },
            FactKind::Tempo {
                metronome: None,
                text: Some("a tempo".to_owned()),
                ramp: None,
            },
            FactKind::Clef {
                clef: crate::Clef::parse("treble")?,
            },
            FactKind::Section { name: String::new() },
            FactKind::Harmony {
                symbol: ChordSymbol::parse("fmaj7")?,
            },
            FactKind::Repeat { times: 4, range: None },
            FactKind::Repeat {
                times: 6,
                range: Some((4, 16)),
            },
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

    /// The controlled traversal is total and identity-preserving on one fact
    /// of every kind. Together with its wildcard-free match, this makes a new
    /// fact variant fail both compilation and this corpus obligation until a
    /// pitch policy is chosen.
    #[test]
    fn every_fact_kind_has_an_explicit_pitch_mapping_policy() {
        let corpus = corpus();
        assert!(corpus.is_some(), "the corpus itself does not parse");
        for fact in corpus.into_iter().flatten() {
            assert_eq!(
                crate::elaborate::map_note_pitch_fact(&fact, Some).as_ref(),
                Some(&fact),
                "identity pitch mapping changed {fact:?}"
            );
        }
    }

    /// No decimal point in any bare word: every rational in a payload —
    /// durations, stretch factors, hairpin shapes, scope indices — is written
    /// as `p/q` or as a whole number, and never as `0.333`.
    ///
    /// Bare words rather than the whole line, because a tempo marking is
    /// allowed to be the word `rit.` — free text is quoted, so the two cannot
    /// be confused, and quoting is what lets this stay a law about numbers.
    #[test]
    fn a_facts_text_form_writes_no_decimals() {
        for fact in corpus().into_iter().flatten() {
            let text = fact.to_text();
            let words = Words::split(&text).expect("what we wrote splits");
            for word in &words.words {
                assert!(
                    word.quoted || !word.text.contains('.'),
                    "a rational was written as a decimal: {text}"
                );
            }
        }
    }

    /// No double quote anywhere, which is a property of the writer rather
    /// than of the corpus: the kernel wraps a payload in `"…"` and escapes
    /// `"` and `\` inside it, so a payload that quoted with `"` would have
    /// every free-text field escaped twice — which is how the form this
    /// replaced reached eight backslashes for one colon.
    #[test]
    fn a_facts_text_form_writes_no_double_quotes() {
        for fact in corpus().into_iter().flatten() {
            let text = fact.to_text();
            assert!(!text.contains('"'), "a payload would be escaped twice: {text}");
        }
    }

    /// The composition the unit round trip does not cover: a fact written
    /// into a one-occurrence timeline, printed as kernel text, parsed back.
    #[test]
    fn a_label_survives_the_kernels_own_quoting() {
        use musa_kernel::{Beat, Occurrence, Span, Term, timeline};
        for fact in corpus().into_iter().flatten() {
            let extent = Beat::new(Ratio::new(1, 4));
            let span = Span::new(Beat::from_integer(0), extent).expect("0 to 1/4 is a span");
            let body = timeline(extent, vec![Occurrence::new(span, fact.clone())]).expect("one occurrence");
            let printed = musa_kernel::print("round-trip", &Term::literal(body), &[]);
            let parsed = musa_kernel::parse::<ScoreFact>(&printed)
                .expect("what we printed parses")
                .into_term();
            let read = parsed
                .into_literal()
                .ok()
                .and_then(|body| body.occurrences().first().map(|one| one.payload().clone()));
            assert_eq!(read.as_ref(), Some(&fact), "did not survive printing: {printed}");
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

    /// A reference mark is the same word stream, and reads back the same way.
    #[test]
    fn a_reference_mark_round_trips() {
        let cases = [
            ReferenceMark {
                depth: 0,
                steps: origin().expansion_path,
                origin: Some(SourceSpan::new(708, 720)),
                scope: Some(Scope::Voice { part: 0, voice: 0 }),
            },
            ReferenceMark {
                depth: 3,
                steps: Vec::new(),
                origin: None,
                scope: None,
            },
        ];
        for mark in cases {
            let text = reference_mark(&mark);
            let read = read_reference_mark(&text).expect("a mark we wrote reads back");
            assert_eq!(read.depth, mark.depth, "{text}");
            assert_eq!(read.steps, mark.steps, "{text}");
            assert_eq!(read.origin, mark.origin, "{text}");
            assert_eq!(read.scope, mark.scope, "{text}");
        }
    }
}
