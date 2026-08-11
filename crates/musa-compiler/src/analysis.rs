//! Musical analysis: observing a compiled score without changing it.
//!
//! `docs/language/05-verification.md` divides theory work three ways. A
//! **constructor invariant** is what a value must satisfy to exist. An
//! **assertion** proves a decidable claim a composer wrote down
//! ([`crate::assert`]). An **analysis** is the third thing: it reads a
//! finished score and reports what it saw, it may be ambiguous, and its
//! findings do not become facts merely because an algorithm selected one.
//!
//! # An analysis is an abstract interpretation
//!
//! Every kind admitted here states three things, in the shape Peyton Jones
//! (1987) §22.1 gives a program analysis — an abstract domain, an abstract
//! reading of the concrete object, and a stated relationship between the two:
//!
//! 1. **The abstract domain** its findings live in.
//! 2. **The abstraction map** from the concrete score projection — that is,
//!    from [`ScoreSnapshot`] — into that domain.
//! 3. **The soundness claim**: what a finding licenses a reader to conclude,
//!    and what it does not.
//!
//! This is not decoration. It is what makes [`Standing`] mean something: a
//! [`Standing::Candidate`] is precisely a concrete reading the abstraction
//! cannot separate from another, and a [`Standing::Conflict`] is precisely two
//! readings the abstraction says cannot both hold. Without the map those words
//! are severity labels chosen by feel, and a finding with no stated
//! relationship to the score is exactly the false claim
//! `docs/language/03-musical-domains.md` §5 forbids. An analysis whose
//! soundness claim cannot be written must not ship.
//!
//! # What this module may not do
//!
//! It never constructs or rewrites music, never emits a compiler diagnostic,
//! and never touches the source. [`analyze`] borrows a snapshot and returns a
//! value; there is no path from here back into the resolver. That is the
//! reason the boundary is one function rather than a compiler pass: an
//! analysis that could report *into* the compilation would be a lint with
//! extra steps, and prompt 83 already drew that line.
//!
//! # Why one function and not a crate
//!
//! The alternative was a public `musa-analysis` crate owning the algorithms.
//! It is rejected for the reason a public elaboration API is rejected: its
//! only caller would be this crate — every consumer reaches analysis through
//! `musa-project` — while its public surface would be segmenters, candidate
//! graphs, and theory indexes, which are pass details that prompt 118 and 119
//! will change. The boundary here is one request, one report, and one
//! function; everything that computes a finding stays private.

use crate::harmony::ChordSymbol;
use crate::origin::SourceSpan;
use crate::pitch::WrittenPitch;
use crate::scope::Scope;
use crate::score::{EventId, Key, Meter, PartId, ScoreSnapshot, VoiceId};
use crate::time::{MusicalDuration, MusicalTime};

/// Which analysis to run.
///
/// A closed list, for the reason [`crate::assert`]'s claim family is one: each
/// kind owes an abstract domain, an abstraction map, and a soundness claim,
/// and a kind supplied by a caller could not owe anything.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AnalysisKind {
    /// The score's own statements, and nothing read into them.
    Facts,
}

impl AnalysisKind {
    /// Every kind, in the order a listing prints them.
    pub const ALL: [Self; 1] = [Self::Facts];

    /// How the kind is written on a command line.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Facts => "facts",
        }
    }

    /// The kind written as `name`, or `None` if none is.
    pub fn named(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.as_str() == name)
    }

    /// One line saying what the analysis does, carried by every report so a
    /// reader never has to guess which algorithm produced it.
    pub fn method(self) -> &'static str {
        match self {
            Self::Facts => {
                "reads the score's own statements: what sounds, what rests, what is written above the staff, and what is in force"
            }
        }
    }

    /// What the analysis takes for granted, one sentence each.
    ///
    /// No kind may leave this empty: an empty list is itself a claim — that
    /// the reading depends on nothing — so every kind states at least the
    /// boundary of what it read.
    pub fn assumptions(self) -> &'static [&'static str] {
        match self {
            Self::Facts => &[
                "the score compiled: an analysis reads a finished snapshot, never half-resolved source",
                "written spelling is kept: `d#4` is reported as D-sharp and never as E-flat",
                "nothing outside the requested scope and window was read, and nothing about it is claimed",
            ],
        }
    }
}

/// Which music to read.
///
/// Named rather than numbered, because whoever is asking is a person at a
/// command line or an interface panel and `PartId(1)` is not something anyone
/// wrote down. A name nothing answers to is [`AnalysisError::NoSuchPart`],
/// which is the point of naming: a typo is refused rather than silently
/// analyzing the whole piece.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AnalysisScope {
    /// Every part and every voice.
    Score,
    /// One part, by the name its declaration gives it.
    Part(String),
    /// One voice of one part, both by name.
    Voice {
        /// The part's name.
        part: String,
        /// The voice's name within that part.
        voice: String,
    },
}

/// What to observe, where, and how far.
///
/// Immutable once built. The window is half-open — `from` is read and `to` is
/// not — because that is the only convention under which two adjacent windows
/// tile a piece without reporting the note on the seam twice.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnalysisRequest {
    kind: AnalysisKind,
    scope: AnalysisScope,
    window: Option<(MusicalTime, MusicalTime)>,
}

impl AnalysisRequest {
    /// The whole score, from beginning to end.
    ///
    /// The defaults live here because every caller that does not narrow its
    /// request wants exactly them, and a three-argument constructor two of
    /// whose arguments are almost always "all of it" is a constructor that
    /// makes its callers say nothing three times.
    pub fn new(kind: AnalysisKind) -> Self {
        Self {
            kind,
            scope: AnalysisScope::Score,
            window: None,
        }
    }

    /// Read only this part or voice.
    #[must_use]
    pub fn scoped(mut self, scope: AnalysisScope) -> Self {
        self.scope = scope;
        self
    }

    /// Read only `[from, to)`, in whole notes from the start of the piece.
    #[must_use]
    pub fn within(mut self, from: MusicalTime, to: MusicalTime) -> Self {
        self.window = Some((from, to));
        self
    }
}

/// A request no score could answer.
///
/// Every case is something the caller can fix by asking differently, which is
/// the test for belonging here at all: an empty result is not an error, and a
/// window containing no notes reports nothing rather than failing.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum AnalysisError {
    /// The score has no part of that name.
    #[error("no part named `{name}`; this score has {}", spell(.available))]
    NoSuchPart {
        /// The name that was asked for.
        name: String,
        /// The names there are, in score order.
        available: Vec<String>,
    },
    /// The part exists and has no voice of that name.
    #[error("part `{part}` has no voice named `{name}`; it has {}", spell(.available))]
    NoSuchVoice {
        /// The part that was found.
        part: String,
        /// The voice name that was asked for.
        name: String,
        /// The voice names that part has, in score order.
        available: Vec<String>,
    },
    /// The window ends before it starts, or is a single instant.
    #[error("a window from {from} to {to} contains no music")]
    EmptyWindow {
        /// Where it was asked to start.
        from: MusicalTime,
        /// Where it was asked to stop.
        to: MusicalTime,
    },
}

/// `` `a` ``, `` `b` ``, and `` `c` `` — or `none`, which is a thing a score's
/// part list can be.
fn spell(names: &[String]) -> String {
    match names {
        [] => "none".to_owned(),
        [only] => format!("`{only}`"),
        [rest @ .., last] => {
            let mut out = rest
                .iter()
                .map(|name| format!("`{name}`"))
                .collect::<Vec<_>>()
                .join(", ");
            out.push_str(" and `");
            out.push_str(last);
            out.push('`');
            out
        }
    }
}

/// How firmly a finding is held.
///
/// These are not severities. Each is a position of the finding relative to the
/// analysis's own abstraction, and each is meaningful only because that
/// abstraction was written down.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Standing {
    /// The abstraction determines this: every concrete score the report is
    /// consistent with agrees about it.
    Fact,
    /// Several concrete readings survive the abstraction and this is one of
    /// them. A candidate is never reported alone — the readings it competes
    /// with are in the same report — and the order they appear in is the
    /// analysis's stated deterministic ranking, never a probability.
    Candidate,
    /// Two readings the abstraction says cannot both hold, reported together
    /// because choosing between them is the reader's business.
    Conflict,
}

impl Standing {
    /// The word a report prints.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Fact => "fact",
            Self::Candidate => "candidate",
            Self::Conflict => "conflict",
        }
    }
}

/// What one finding says, in the domain its analysis reasons about.
///
/// Typed rather than a sentence: a consumer that wants to draw the pitch on a
/// staff, or hand it to the next analysis, must not have to parse English back
/// out of a string. Composing the sentence is the renderer's job.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Observation {
    /// A written pitch sounding over an exact span.
    Sounding {
        /// The spelled pitch. Spelling is kept: `d#4` is not `eb4`.
        pitch: WrittenPitch,
        /// Where it starts, in whole notes from the piece start.
        onset: MusicalTime,
        /// How long it sounds.
        extent: MusicalDuration,
    },
    /// A rest: nothing sounding, over an exact span.
    Silence {
        /// Where the silence starts.
        onset: MusicalTime,
        /// How long it lasts.
        extent: MusicalDuration,
    },
    /// A chord symbol written above the staff.
    ///
    /// Recorded, never interpreted — `crate::harmony`'s standing rule. The
    /// finding says the symbol is written there. It says nothing about whether
    /// the notes underneath agree with it.
    Written {
        /// The symbol as the source spells it.
        symbol: ChordSymbol,
        /// Where it is written, in whole notes from the piece start.
        at: MusicalTime,
    },
    /// A key in force from an instant.
    KeyInForce {
        /// The key.
        key: Key,
        /// Where it takes force.
        from: MusicalTime,
    },
    /// A meter in force from an instant.
    MeterInForce {
        /// The meter.
        meter: Meter,
        /// Where it takes force.
        from: MusicalTime,
    },
}

impl Observation {
    /// Where in the piece the observation is anchored — the first key every
    /// report is ordered by.
    pub fn at(&self) -> MusicalTime {
        match *self {
            Self::Sounding { onset, .. } | Self::Silence { onset, .. } => onset,
            Self::Written { at, .. } => at,
            Self::KeyInForce { from, .. } | Self::MeterInForce { from, .. } => from,
        }
    }

    /// The tie-break within one instant, so a key and the note under it print
    /// in a fixed order rather than in whatever order the reading produced.
    fn rank(&self) -> u8 {
        match *self {
            Self::MeterInForce { .. } => 0,
            Self::KeyInForce { .. } => 1,
            Self::Written { .. } => 2,
            Self::Sounding { .. } => 3,
            Self::Silence { .. } => 4,
        }
    }
}

/// Where in the score a finding can be seen.
///
/// Three shapes rather than one optional span, because the three answer
/// genuinely different questions. An event has an identity a caller can select
/// by; an annotation has source text and no identity of its own; and a context
/// has neither, because a context track records where a value *takes force*,
/// which is a place in the piece rather than a place in a file. Collapsing
/// them into `Option<SourceSpan>` would make every consumer rediscover which
/// case it was looking at.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Evidence {
    /// A score event, and the source that spells it.
    Event {
        /// Which part it is in.
        part: PartId,
        /// Which voice of that part.
        voice: VoiceId,
        /// The event's snapshot-local identity.
        id: EventId,
        /// The statement that spells it — the note inside the motif when the
        /// event came from one, so the reference points at editable text.
        span: SourceSpan,
    },
    /// Something written above the staff, and its source.
    Annotation {
        /// Where the annotation is written.
        span: SourceSpan,
    },
    /// A value in force from an instant, with no statement to point at.
    InForce {
        /// The scope reading it, since a key in force in one voice may have
        /// been written for the piece.
        scope: Scope,
        /// Where the value takes force.
        from: MusicalTime,
    },
}

/// One thing an analysis saw.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnalysisFinding {
    code: &'static str,
    standing: Standing,
    observation: Observation,
    evidence: Evidence,
}

impl AnalysisFinding {
    /// The stable code, for a consumer that filters or groups without matching
    /// on prose: `sounding-pitch`, `key-in-force`.
    pub fn code(&self) -> &'static str {
        self.code
    }

    /// How firmly the finding is held.
    pub fn standing(&self) -> Standing {
        self.standing
    }

    /// What was seen.
    pub fn observation(&self) -> &Observation {
        &self.observation
    }

    /// Where it can be seen in the score.
    pub fn evidence(&self) -> &Evidence {
        &self.evidence
    }
}

/// What an analysis saw, with the method and assumptions that produced it.
///
/// Immutable: a report records one reading, and a consumer able to add a
/// finding to it would be publishing its own conclusions under the analysis's
/// name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnalysisReport {
    kind: AnalysisKind,
    findings: Vec<AnalysisFinding>,
}

impl AnalysisReport {
    /// Which analysis produced this.
    pub fn kind(&self) -> AnalysisKind {
        self.kind
    }

    /// One line saying what it does.
    pub fn method(&self) -> &'static str {
        self.kind.method()
    }

    /// What it took for granted.
    pub fn assumptions(&self) -> &'static [&'static str] {
        self.kind.assumptions()
    }

    /// Everything it saw, in a deterministic order: by instant, then by the
    /// fixed rank of the observation kind, then by part, voice, and event.
    ///
    /// The order belongs to the report and not to a renderer, because "two
    /// runs of one analysis on one score produce the same bytes" has to be a
    /// property of [`analyze`] for any caller to diff two reports at all.
    pub fn findings(&self) -> &[AnalysisFinding] {
        &self.findings
    }

    /// Put the findings in that order and drop exact repeats.
    ///
    /// A repeat is possible because context tracks are read per voice and two
    /// voices of one part see the same piece-wide key. Two voices *sounding*
    /// the same pitch are not repeats — their evidence differs — so
    /// deduplication is on the whole finding and never on the observation
    /// alone. Applied at the one construction site, so no kind can forget it.
    fn canonical(mut self) -> Self {
        self.findings.sort_by_key(order);
        self.findings.dedup();
        self
    }
}

/// Read `snapshot` the way `request` asks, and report what is there.
///
/// This is the whole boundary. Segmenters, indexes, candidate graphs, and
/// theory values stay behind it: a caller supplies a request and receives a
/// report, and nothing about how the reading was computed is observable.
///
/// The snapshot is borrowed and never mutated — an analysis cannot change a
/// score, and that is a property of this signature rather than a promise in
/// prose.
///
/// # Errors
/// [`AnalysisError`] when the request names a part or a voice this score does
/// not have, or a window that contains no music. An empty *answer* is not an
/// error: a window with nothing in it is a report with no findings.
pub fn analyze(snapshot: &ScoreSnapshot, request: &AnalysisRequest) -> Result<AnalysisReport, AnalysisError> {
    let lanes = select(snapshot, &request.scope)?;
    let window = window_of(request)?;
    let findings = match request.kind {
        AnalysisKind::Facts => facts::observe(snapshot, &lanes, &request.scope, window),
    };
    Ok(AnalysisReport {
        kind: request.kind,
        findings,
    }
    .canonical())
}

/// The half-open window a request selects, or the error saying it selects
/// nothing.
fn window_of(request: &AnalysisRequest) -> Result<Option<(MusicalTime, MusicalTime)>, AnalysisError> {
    match request.window {
        Some((from, to)) if to <= from => Err(AnalysisError::EmptyWindow { from, to }),
        other => Ok(other),
    }
}

/// One voice an analysis reads, resolved from the names the request used.
struct Lane {
    part: PartId,
    voice: VoiceId,
}

impl Lane {
    /// The scope this lane reads context tracks through, so a key written for
    /// the piece and one written for the part both answer correctly.
    fn scope(&self) -> Scope {
        Scope::Voice {
            part: self.part.0,
            voice: self.voice.0,
        }
    }
}

/// Resolve the request's names into the voices it selects, in score order.
///
/// The lookup lives here rather than in each kind because every kind asks the
/// same question about scope, and because a misspelled part is a mistake about
/// the request rather than about the music.
fn select(snapshot: &ScoreSnapshot, scope: &AnalysisScope) -> Result<Vec<Lane>, AnalysisError> {
    let lanes = |id: PartId, part: &crate::score::Part| -> Vec<Lane> {
        part.voices().map(|(voice, _)| Lane { part: id, voice }).collect()
    };
    let named = |wanted: &str| {
        snapshot
            .parts()
            .iter()
            .find(|(_, part)| part.name() == wanted)
            .ok_or_else(|| AnalysisError::NoSuchPart {
                name: wanted.to_owned(),
                available: snapshot
                    .parts()
                    .iter()
                    .map(|(_, part)| part.name().to_owned())
                    .collect(),
            })
    };
    match *scope {
        AnalysisScope::Score => Ok(snapshot.parts().iter().flat_map(|(id, part)| lanes(id, part)).collect()),
        AnalysisScope::Part(ref name) => {
            let (id, part) = named(name)?;
            Ok(lanes(id, part))
        }
        AnalysisScope::Voice { ref part, ref voice } => {
            let (id, found) = named(part)?;
            let wanted = found
                .voices()
                .map(|(candidate, _)| candidate)
                .find(|candidate| found.voice_name(*candidate) == Some(voice.as_str()))
                .ok_or_else(|| AnalysisError::NoSuchVoice {
                    part: part.clone(),
                    name: voice.clone(),
                    available: found
                        .voices()
                        .filter_map(|(candidate, _)| found.voice_name(candidate))
                        .map(str::to_owned)
                        .collect(),
                })?;
            Ok(vec![Lane {
                part: id,
                voice: wanted,
            }])
        }
    }
}

/// Whether `at` falls in the half-open window, with `None` meaning the whole
/// piece.
fn inside(window: Option<(MusicalTime, MusicalTime)>, at: MusicalTime) -> bool {
    match window {
        None => true,
        Some((from, to)) => at >= from && at < to,
    }
}

/// The total order every report is in.
fn order(finding: &AnalysisFinding) -> (MusicalTime, u8, u32, u32, u64, u32) {
    let (part, voice, id, span) = match finding.evidence {
        Evidence::Event { part, voice, id, span } => (part.0, voice.0, id.0, span.start),
        Evidence::Annotation { span } => (u32::MAX, u32::MAX, u64::MAX, span.start),
        Evidence::InForce { .. } => (u32::MAX, u32::MAX, u64::MAX, u32::MAX),
    };
    (
        finding.observation.at(),
        finding.observation.rank(),
        part,
        voice,
        id,
        span,
    )
}

/// The `facts` kind.
///
/// **Abstract domain.** The finite set of *pointed statements* of a score: a
/// sounding written pitch with its exact span, a silence with its exact span,
/// a chord symbol at an instant, and a key or meter in force from an instant —
/// each paired with where in the score it can be seen.
///
/// **Abstraction map.** α restricts the snapshot to the requested lanes and
/// window and reads every surviving score event, harmony annotation, and key
/// and meter stretch into exactly one element of that set. A chord event
/// becomes one `Sounding` per tone, because a chord at this layer is
/// simultaneous notes and nothing more.
///
/// **Soundness.** α is *exact on what it reports*: for every finding there is
/// a statement of the score with precisely those coordinates, and every
/// statement of the score inside the scope and window has a finding.
/// Therefore every finding is a [`Standing::Fact`] — the abstraction separates
/// every pair of concrete scores differing on anything it reports, so no two
/// readings survive it and there is nothing to be a candidate about.
///
/// What a finding licenses is exactly "the score states this here". What it
/// does *not* license is any claim about material the request excluded: γ of a
/// report is every score agreeing with it inside the scope and window, and
/// that set is not a singleton. A reader concluding "the piece is in C major"
/// from one `KeyInForce` finding over one bar has read something the map does
/// not say.
mod facts {
    use super::{AnalysisFinding, AnalysisScope, Evidence, Lane, Observation, Standing, inside};
    use crate::score::{ScoreEventKind, ScoreSnapshot};
    use crate::time::MusicalTime;

    pub(super) fn observe(
        snapshot: &ScoreSnapshot,
        lanes: &[Lane],
        scope: &AnalysisScope,
        window: Option<(MusicalTime, MusicalTime)>,
    ) -> Vec<AnalysisFinding> {
        let mut found = Vec::new();
        for lane in lanes {
            let Some(voice) = snapshot.parts().get(lane.part).and_then(|part| part.voice(lane.voice)) else {
                continue;
            };
            for event in voice.events() {
                if !inside(window, event.onset) {
                    continue;
                }
                let evidence = Evidence::Event {
                    part: lane.part,
                    voice: lane.voice,
                    id: event.id,
                    span: event.origin.definition_span,
                };
                let extent = event.notated_duration.value;
                match event.kind {
                    ScoreEventKind::Note { pitch } => found.push(AnalysisFinding {
                        code: "sounding-pitch",
                        standing: Standing::Fact,
                        observation: Observation::Sounding {
                            pitch,
                            onset: event.onset,
                            extent,
                        },
                        evidence,
                    }),
                    // A chord is simultaneous notes at this layer, so it is
                    // that many findings and not one with a list inside: the
                    // next analysis asks about pitches, not about noteheads.
                    ScoreEventKind::Chord { ref pitches } => {
                        found.extend(pitches.iter().map(|pitch| AnalysisFinding {
                            code: "sounding-pitch",
                            standing: Standing::Fact,
                            observation: Observation::Sounding {
                                pitch: *pitch,
                                onset: event.onset,
                                extent,
                            },
                            evidence: evidence.clone(),
                        }));
                    }
                    ScoreEventKind::Rest => found.push(AnalysisFinding {
                        code: "silence",
                        standing: Standing::Fact,
                        observation: Observation::Silence {
                            onset: event.onset,
                            extent,
                        },
                        evidence,
                    }),
                }
            }
            for (from, key) in snapshot.keys().changes(lane.scope()) {
                if inside(window, from) {
                    found.push(AnalysisFinding {
                        code: "key-in-force",
                        standing: Standing::Fact,
                        observation: Observation::KeyInForce { key: *key, from },
                        evidence: Evidence::InForce {
                            scope: lane.scope(),
                            from,
                        },
                    });
                }
            }
            for (from, meter) in snapshot.meters().changes(lane.scope()) {
                if inside(window, from) {
                    found.push(AnalysisFinding {
                        code: "meter-in-force",
                        standing: Standing::Fact,
                        observation: Observation::MeterInForce { meter: *meter, from },
                        evidence: Evidence::InForce {
                            scope: lane.scope(),
                            from,
                        },
                    });
                }
            }
        }
        // A chord symbol belongs to the piece, not to a staff: it says what
        // the whole texture is doing there. So it is read when the request is
        // about the piece, and left out when the request narrowed to one part
        // or voice — reporting it there would attribute a piece-wide statement
        // to music that did not make it.
        if matches!(*scope, AnalysisScope::Score) {
            found.extend(
                snapshot
                    .annotations()
                    .harmony()
                    .iter()
                    .filter(|mark| inside(window, mark.at))
                    .map(|mark| AnalysisFinding {
                        code: "written-harmony",
                        standing: Standing::Fact,
                        observation: Observation::Written {
                            symbol: mark.symbol.clone(),
                            at: mark.at,
                        },
                        evidence: Evidence::Annotation {
                            span: mark.origin.source_span,
                        },
                    }),
            );
        }
        found
    }
}
