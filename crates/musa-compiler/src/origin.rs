//! Provenance (roadmap §9): why an expanded event exists. Every
//! `ScoreEvent` traces to a source span and a declaration, plus the path of
//! expansion steps that produced it (empty for directly authored notes until
//! prompt 06).

use num_rational::Ratio;
use serde::{Deserialize, Serialize};

/// A byte span in the source document.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceSpan {
    /// First byte of the span.
    pub start: u32,
    /// One past the last byte of the span.
    pub end: u32,
}

impl SourceSpan {
    /// Create a span from start and end byte offsets.
    pub const fn new(start: u32, end: u32) -> Self {
        Self { start, end }
    }
}

/// The declaration an event originates from: an ordinal over the piece's
/// declarations in source order. Stable within one compilation; not a
/// permanent project identity.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct DeclarationId(pub u32);

/// Why an event exists (roadmap §9).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Origin {
    /// The source span that (transitively) produced the event.
    pub source_span: SourceSpan,
    /// The span of the statement that literally spells this event: the note
    /// inside the `motif` body for a generated event, and the same as
    /// `source_span` for an authored one. Editing the definition edits here;
    /// `source_span` is where the event *came from*, this is what wrote it.
    pub definition_span: SourceSpan,
    /// The enclosing declaration.
    pub declaration: DeclarationId,
    /// The expansion steps from declaration to event; empty for directly
    /// authored notes.
    pub expansion_path: Vec<ExpansionStep>,
}

/// One step in an expansion path.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExpansionStep {
    /// A motif was applied at this call site.
    MotifApplication {
        /// The span of the `use` statement.
        call_site: SourceSpan,
    },
    /// A `repeat` iteration (0-based).
    RepeatIteration(u32),
    /// A transposition by an interval.
    Transposition(Interval),
    /// A time stretch by an exact factor.
    Stretch(Ratio<i64>),
    /// A retrograde.
    Retrograde,
    /// An inversion about an axis pitch, as the source spells it (`c5`).
    Inversion {
        /// The axis the block was mirrored about.
        axis: String,
    },
    /// A checked `pitch -> pitch` function was applied to sounding pitches.
    MapNotePitches,
    /// A lexical scale was in force here, as the source spells it
    /// (`scale c dorian`). It changes generative pitch coordinates only; it
    /// is never a key signature.
    ScaleContext {
        /// The scale the enclosing `in scale` named.
        scale: String,
    },
    /// A declaration template was expanded at an instance site.
    ///
    /// The first step of any event a `make` produced, and the reason Origin
    /// can answer "which instance is this" and not only "which template".
    TemplateInstance {
        /// The template's name, as the site calls it.
        template: String,
        /// The `as` name the site gives what it made.
        alias: String,
        /// The span of the `make` statement.
        site: SourceSpan,
        /// The generated declaration's stable identity, as hex.
        identity: String,
    },
    /// An `assert` was in force here, and the claim it made held.
    ///
    /// Provenance and nothing more: an assertion adds no occurrence, no
    /// payload, and no time, so this step is the whole of what it leaves
    /// behind. It is what lets Origin answer "what did this passage promise",
    /// and — because changing only Origin cannot change `≈facts` — it is also
    /// the proof that a claim which holds changed nothing.
    Assertion {
        /// The claim's name, as the source spells it.
        claim: String,
    },
    /// One note of a motif occurrence was respelled by a `with` clause.
    Specialization {
        /// The span of the override that respelled it.
        override_site: SourceSpan,
    },
}

/// A signed musical interval (roadmap §5.4): diatonic steps plus semitones,
/// with direction baked into the sign. `P5` up is `(4, 7)`; down is
/// `(-4, -7)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Interval {
    /// Signed diatonic steps (a fifth is 4).
    pub diatonic_steps: i64,
    /// Signed semitones (a perfect fifth is 7).
    pub semitones: i64,
}

impl Interval {
    /// The zero interval.
    pub const ZERO: Self = Self {
        diatonic_steps: 0,
        semitones: 0,
    };

    /// Parse a simple or compound interval literal with a direction.
    ///
    /// Perfect-class sizes (unisons, fourths, fifths, and their compounds)
    /// accept `P`, repeated `A`, or repeated `d`. Major-class sizes accept
    /// `M`, `m`, repeated `A`, or repeated `d`. The repetitions are not a
    /// semantic bound: `AAA4` and `ddd10` are ordinary integer pairs.
    pub fn parse(text: &str, down: bool) -> Option<Self> {
        let quality_end = text.find(|character: char| character.is_ascii_digit())?;
        let (quality, size_text) = text.split_at(quality_end);
        if quality.is_empty() {
            return None;
        }
        let size: i64 = size_text.parse().ok()?;
        if size == 0 {
            return None;
        }
        let steps = size.checked_sub(1)?;
        let simple = steps.rem_euclid(7);
        let octaves = steps.div_euclid(7);
        let natural_simple = *[0_i64, 2, 4, 5, 7, 9, 11].get(usize::try_from(simple).ok()?)?;
        let natural = octaves.checked_mul(12)?.checked_add(natural_simple)?;
        let perfect_class = matches!(simple, 0 | 3 | 4);
        let alteration = match quality {
            "P" if perfect_class => 0,
            "M" if !perfect_class => 0,
            "m" if !perfect_class => -1,
            "dim" => {
                if perfect_class {
                    -1
                } else {
                    -2
                }
            }
            quality if quality.bytes().all(|byte| byte == b'A') => i64::try_from(quality.len()).ok()?,
            quality if quality.bytes().all(|byte| byte == b'd') => {
                let degrees = i64::try_from(quality.len()).ok()?;
                if perfect_class {
                    degrees.checked_neg()?
                } else {
                    degrees.checked_add(1)?.checked_neg()?
                }
            }
            _ => return None,
        };
        let semitones = natural.checked_add(alteration)?;
        let sign = if down { -1 } else { 1 };
        Some(Self {
            diatonic_steps: steps.checked_mul(sign)?,
            semitones: semitones.checked_mul(sign)?,
        })
    }

    /// Compose two written intervals componentwise.
    pub fn compose(self, other: Self) -> Option<Self> {
        Some(Self {
            diatonic_steps: self.diatonic_steps.checked_add(other.diatonic_steps)?,
            semitones: self.semitones.checked_add(other.semitones)?,
        })
    }

    /// The interval which undoes this interval.
    pub fn inverse(self) -> Option<Self> {
        Some(Self {
            diatonic_steps: self.diatonic_steps.checked_neg()?,
            semitones: self.semitones.checked_neg()?,
        })
    }
}

impl std::fmt::Display for Interval {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let down = self.diatonic_steps.is_negative() || (self.diatonic_steps == 0 && self.semitones.is_negative());
        let Some(steps) = (if down {
            self.diatonic_steps.checked_neg()
        } else {
            Some(self.diatonic_steps)
        }) else {
            return write!(out, "({}, {})", self.diatonic_steps, self.semitones);
        };
        let Some(semitones) = (if down {
            self.semitones.checked_neg()
        } else {
            Some(self.semitones)
        }) else {
            return write!(out, "({}, {})", self.diatonic_steps, self.semitones);
        };
        let Some(size) = steps.checked_add(1) else {
            return write!(out, "({}, {})", self.diatonic_steps, self.semitones);
        };
        let simple = steps.rem_euclid(7);
        let natural = steps.div_euclid(7).checked_mul(12).and_then(|octaves| {
            [0_i64, 2, 4, 5, 7, 9, 11]
                .get(usize::try_from(simple).ok()?)
                .and_then(|simple| octaves.checked_add(*simple))
        });
        let Some(alteration) = natural.and_then(|natural| semitones.checked_sub(natural)) else {
            return write!(out, "({}, {})", self.diatonic_steps, self.semitones);
        };
        let perfect_class = matches!(simple, 0 | 3 | 4);
        let quality = match (perfect_class, alteration) {
            (true, 0) => Some("P".to_owned()),
            (false, 0) => Some("M".to_owned()),
            (false, -1) => Some("m".to_owned()),
            (_, positive) if positive > 0 => usize::try_from(positive).ok().map(|count| "A".repeat(count)),
            (true, negative) => usize::try_from(negative.unsigned_abs()).ok().map(|count| {
                if count == 1 {
                    "dim".to_owned()
                } else {
                    "d".repeat(count)
                }
            }),
            (false, negative) => negative
                .checked_neg()
                .and_then(|magnitude| magnitude.checked_sub(1))
                .and_then(|degrees| usize::try_from(degrees).ok())
                .map(|count| {
                    if count == 1 {
                        "dim".to_owned()
                    } else {
                        "d".repeat(count)
                    }
                }),
        };
        let Some(quality) = quality else {
            return write!(out, "({}, {})", self.diatonic_steps, self.semitones);
        };
        if down {
            write!(out, "down {quality}{size}")
        } else {
            write!(out, "{quality}{size}")
        }
    }
}

/// The stable name of a place where the piece leaves a decision open.
///
/// A realization has to make the *same* decision again after the composer
/// edits an unrelated bar; otherwise every keystroke re-rolls the performance
/// and the page flickers with music nobody wrote. That rules out the two
/// obvious names (`docs/kernel/11-realization.md`): a source span, because
/// reformatting would re-roll everything, and a [`DeclarationId`], because
/// inserting a declaration renumbers everything after it.
///
/// So a path is made of **names**: the motif or the bar a site sits in, ending
/// in an ordinal among its unnamed siblings. Insert a bar above and the paths
/// below are unchanged, because names do not shift.
///
/// A site written among a voice's own items has **no name above it** — its
/// path is just its ordinal. That is not an omission: prompt 57's rule is that
/// a repeat barline crosses the system, so a repeat the page can draw is one
/// repeat of the whole piece, written once in each voice that sounds under it.
/// Giving it a per-voice path would decide it several times over and the
/// voices would come apart. The same argument prompt 64 made for `meter`,
/// arriving at the same answer. A freedom that really is one player's — *In
/// C*'s — is a different construct and belongs to prompt 68.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ChoicePath(Vec<ChoiceStep>);

/// One name along a [`ChoicePath`].
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ChoiceStep {
    /// A motif, by the name it was declared under.
    Motif(Box<str>),
    /// A fragment, by name.
    Fragment(Box<str>),
    /// A named bar.
    Bar(Box<str>),
    /// Which site this is among the unnamed siblings in the innermost named
    /// thing.
    ///
    /// The weak point, stated rather than hidden: insert a third site between
    /// two unnamed ones and the second's ordinal shifts, so its decision
    /// changes. A composer who cares names the bar; the interface shows which
    /// path each decision belongs to, so the shift is visible rather than
    /// mysterious.
    Ordinal(u32),
}

impl ChoicePath {
    /// Extend a path by one step, leaving the original alone.
    ///
    /// Paths are built downward through elaboration, where the enclosing
    /// names are known and the site is not yet, so every construction is
    /// "this path, plus where I am".
    #[must_use]
    pub fn then(&self, step: ChoiceStep) -> Self {
        let mut steps = self.0.clone();
        steps.push(step);
        Self(steps)
    }

    /// The bytes this path is, for hashing and for display.
    ///
    /// Injective: two different paths encode to two different byte strings.
    /// Each step is written with its kind letter, its length, and its text, so
    /// no concatenation of names can imitate a different splitting of them —
    /// the same requirement N3 places on payload keys, and it gets the same
    /// test.
    #[must_use]
    pub fn canonical(&self) -> String {
        let mut text = String::new();
        for step in &self.0 {
            let (letter, name) = match step {
                ChoiceStep::Motif(name) => ('m', name.to_string()),
                ChoiceStep::Fragment(name) => ('f', name.to_string()),
                ChoiceStep::Bar(name) => ('b', name.to_string()),
                ChoiceStep::Ordinal(index) => ('#', index.to_string()),
            };
            text.push(letter);
            text.push_str(&name.len().to_string());
            text.push(':');
            text.push_str(&name);
        }
        text
    }

    /// Read back what [`Self::canonical`] wrote, or `None` if the text is not
    /// a path.
    ///
    /// A pin is stored as its path, so a realization that survives a session
    /// (prompt 76) is a set of these strings. Parsing the injective encoding
    /// rather than adding a second one is what makes "the file says what the
    /// digest hashed" true by construction.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let mut steps = Vec::new();
        let mut rest = text;
        while !rest.is_empty() {
            let (letter, tail) = rest.split_at_checked(1)?;
            let (length, tail) = tail.split_once(':')?;
            let length: usize = length.parse().ok()?;
            let (name, tail) = tail.split_at_checked(length)?;
            steps.push(match letter {
                "m" => ChoiceStep::Motif(name.into()),
                "f" => ChoiceStep::Fragment(name.into()),
                "b" => ChoiceStep::Bar(name.into()),
                "#" => ChoiceStep::Ordinal(name.parse().ok()?),
                _ => return None,
            });
            rest = tail;
        }
        Some(Self(steps))
    }

    /// The steps, outermost first — what an interface prints as a trail.
    pub fn steps(&self) -> &[ChoiceStep] {
        &self.0
    }

    /// The path as a sentence: `the fill, first choice`.
    ///
    /// `docs/interface/` §states and voice is the rule here — the interface
    /// says what happened, not what the machine did, and a composer should be
    /// able to use open form without learning the word "realization". So no
    /// `#2`, no `Ordinal`, and no brackets. A path with no name above it is
    /// just its ordinal, which is the honest reading: the piece asked its
    /// third question and this is the answer.
    #[must_use]
    pub fn describe(&self) -> String {
        let mut named: Vec<&str> = Vec::new();
        let mut ordinal = None;
        for step in &self.0 {
            match step {
                ChoiceStep::Motif(name) | ChoiceStep::Fragment(name) | ChoiceStep::Bar(name) => named.push(name),
                ChoiceStep::Ordinal(index) => ordinal = Some(*index),
            }
        }
        let place = named.join(", in ");
        let which = ordinal.map(nth);
        match (place.is_empty(), which) {
            (true, Some(which)) => format!("the {which} choice"),
            (true, None) => "this choice".to_owned(),
            (false, Some(which)) => format!("{place}, {which} choice"),
            (false, None) => place,
        }
    }
}

/// `first`, `second`, … and plain numbering past the point where the words
/// stop helping a reader place a site at a glance.
fn nth(index: u32) -> String {
    const WORDS: [&str; 9] = [
        "first", "second", "third", "fourth", "fifth", "sixth", "seventh", "eighth", "ninth",
    ];
    WORDS
        .get(index as usize)
        .map_or_else(|| format!("#{}", index.saturating_add(1)), |word| (*word).to_owned())
}

impl std::fmt::Display for ChoicePath {
    /// `fill ▸ #2` — the trail a person reads, as opposed to the bytes a
    /// digest reads.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut first = true;
        for step in &self.0 {
            if !first {
                formatter.write_str(" ▸ ")?;
            }
            first = false;
            match step {
                ChoiceStep::Motif(name) | ChoiceStep::Fragment(name) | ChoiceStep::Bar(name) => {
                    formatter.write_str(name)?;
                }
                ChoiceStep::Ordinal(index) => write!(formatter, "#{index}")?,
            }
        }
        Ok(())
    }
}
