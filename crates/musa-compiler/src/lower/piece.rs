//! A written piece, as the track it denotes.
//!
//! [`super::notation`] is `00-semantics.md` §3's first composition equation —
//! `follow` over a block's statements. This module is the second one, over a
//! score's voices:
//!
//! ```text
//! together(v₁, together(v₂, … together(vₙ, context)))
//! ```
//!
//! and that is the whole of what a piece is. There is no third combinator and
//! no pass: a part is not a thing in the term, it is the *scope* its voices'
//! facts are constructed at.
//!
//! # What the structure is for
//!
//! Scope, and nothing else. [`super::notation`] refuses seven statements: four
//! because "from here onward" has no unique meaning in a value usable at
//! several places, and three because bar structure needs a voice to belong to.
//! Both refusals are about one missing thing, and this module supplies it by
//! widening the reading rather than by widening the statement table — reading a
//! voice's body is `notated(node, Reading::at(Scope::Voice { … }))` and nothing
//! else. Everything the fold already knew it still knows.
//!
//! # The numbering is positional and counts every item
//!
//! A part's id is a running count over the score; a voice's is its position
//! among the part's items, and an instance site is an item like any other.
//! `PartDecl::items` says why the grammar keeps them interleaved — "a `make`
//! between two voices makes a voice *there*".
//!
//! # An instance is a binding, and its provenance is a builtin
//!
//! `make N(…) as I;` reads `N`'s body once, at the scope of the voice the site
//! stands in, and applies λs over its parameters to the argument expressions the
//! site wrote. That is `04-templates-and-modules.md` §1's "expansion is a
//! binding, never a rewrite", written in the core's own λ. What the site adds is
//! provenance, and that is [`crate::registry`]'s `instanced` rather than
//! anything this walk does: the facts do not exist until the term is evaluated,
//! and the ones a function the body calls produced were read in another
//! declaration entirely.
//!
//! # A region is the longest voice
//!
//! [`super::notation::extent`] sums, because a block is a fold and a fold's
//! extent is a sum. What a header fact covers is `03-denotational-semantics.md`
//! D3's `max(d, e)` over the parts instead, so it is computed from the voices
//! rather than by calling the same function one level up.
//!
//! # Where a context statement lands
//!
//! A header's facts cover the piece; a context statement written among a
//! voice's items is a **point** where the music reached it. Both are the
//! replaced path's rules unchanged, including the two that look like
//! exceptions: a `meter`, a `key`, or a `tempo` written in one voice is the
//! *piece's* from there — per-voice meter is polymeter, which is a part's own
//! header — and a `clef` is the *part's*, because the reader whose hands change
//! staff is one player.

#[cfg(test)]
mod laws;

use musa_calculus::{Origin, Raw};
use musa_language::SyntaxNode;
use musa_language::ast::AstNode as _;

use super::notation::{Context, Reading};
use super::{Lowering, applied, expansion};
use musa_score::diagnose::{Code, Diagnostic};

/// A piece, read.
///
/// The track is the whole answer; the parts are what a consumer needs to bucket
/// a projection of it back into the score it came from. Nothing else, because
/// nothing else has a caller: names and ids are what `Scope` is written out of,
/// and every other question about a piece is answered by the term.
pub(crate) struct Piece {
    /// The piece as one written-time track.
    pub(crate) track: Raw,
    /// Its parts, in written order.
    pub(crate) parts: Vec<Part>,
}

/// One part, and the voices it numbered.
pub(crate) struct Part {
    /// The id every fact of this part is constructed at.
    pub(crate) id: u32,
    /// What the score calls it.
    pub(crate) name: String,
    /// Where the name is written, when the part has one to point at.
    ///
    /// The reference index and the lint pass are about *text*, and a name
    /// without a place in it is a name neither can say anything about.
    pub(crate) name_span: Option<musa_score::origin::SourceSpan>,
    /// The meter this part counts its own barlines in, when it states one.
    ///
    /// Carried rather than re-read, because `Meter` inherits by `Override`
    /// (`musa_score::scope`): a part that states 7/8 does not hear the piece's
    /// changes, so a claim written in it is proved against bars nothing else
    /// in the score knows about. [`crate::resolve::part_facts`] answered this
    /// while the part was being read, and asking the CST a second time would
    /// be the re-derivation `AGENTS.md` names.
    pub(crate) meter: Option<musa_score::Meter>,
    /// The performance profile the part names, and where it names it.
    ///
    /// Read here and *checked* by the caller, because the two questions have
    /// two owners: what the part says is written in the part, and whether the
    /// piece declares a profile by that name is the snapshot's to answer —
    /// [`crate::resolve::lower_header`] has not necessarily filled it when a
    /// part is walked, and a reading that had to be handed the declared set to
    /// answer "what does this part say" would be answering two questions at
    /// once. It is not a fact either: a clef and a polymeter enter the piece's
    /// context track, and a profile is metadata about how the notation is
    /// played (roadmap §2 — a dynamic marking is not a number of decibels).
    pub(crate) profile: Option<(String, musa_score::origin::SourceSpan)>,
    /// Its voices, in written order.
    pub(crate) voices: Vec<Voice>,
}

/// One voice of one part.
pub(crate) struct Voice {
    /// The id within its part, which is its position among the part's items.
    pub(crate) id: u32,
    /// What the part calls it.
    pub(crate) name: String,
    /// Where the name is written — a voice's own identifier, or the `as` name
    /// at the site that made it. See [`Part::name_span`].
    pub(crate) name_span: Option<musa_score::origin::SourceSpan>,
    /// What this voice alone sounds, as a term.
    ///
    /// Beside [`Piece::track`] rather than instead of it, because the two answer
    /// different questions and a consumer needs both: a projection is *lanes*,
    /// one per voice, and the piece's own identity is the whole simultaneity.
    /// Deriving either from the other would mean taking a normal form apart by
    /// scope, which is reading provenance back out of a value that was built to
    /// carry it forward.
    pub(crate) track: Raw,
    /// The claims written over passages of this voice, in source order.
    ///
    /// Per voice rather than per piece because a claim is proved against the
    /// barlines *its own part* counts by, and a polymetric piece has more than
    /// one set of them. Each one carries its own two terms, so a claim is placed
    /// by reading, not by a position this walk would have had to keep.
    pub(crate) claims: Vec<super::notation::Claimed>,
}

impl Lowering<'_> {
    /// `piece "…" { … }` — its score, its context, and the track they make.
    ///
    /// [`None`] when anything in it was refused, for [`Lowering::music`]'s
    /// reason one level up: a piece missing a voice is a different piece, not a
    /// shorter one. Every part and every voice is read even after one is
    /// refused, so a score with three bad voices reports three diagnostics.
    ///
    /// `standing` is the expansion a `make … as …;` at the document's root put
    /// this whole piece behind, when the file writes one instead of a piece. It
    /// is applied per *voice* and not to [`Piece::track`], because a voice is
    /// what a made piece produces: a context fact keeps the empty path the
    /// replaced walk gave it, since a header states what it states wherever the
    /// piece was made.
    pub(crate) fn piece(
        &mut self,
        node: &SyntaxNode,
        namespace: &str,
        standing: Option<&musa_score::origin::Origin>,
    ) -> Option<Piece> {
        let declaration = musa_language::ast::PieceDecl::cast(node.clone())?;
        let origin = self.origin(node);
        let mut whole = true;
        let mut instances = Instances::of(self.resolver, node, namespace);
        let mut context = self.header(&declaration, &mut whole);
        // The meter and the collection every voice starts in. Read off the
        // header rather than carried out of [`Lowering::header`], because a
        // header states three facts and only two of them are a *reading
        // context*: a `senza` asks what meter to put back and a `step` asks what
        // collection to count in, while nothing asks about the tempo the piece
        // opened with.
        let opening = declaration
            .meter()
            .and_then(|statement| crate::resolve::parse_meter(&statement))
            .unwrap_or_default();
        let suggested = declaration
            .key()
            .and_then(|statement| crate::resolve::parse_key(&statement));
        let mut parts: Vec<Part> = Vec::new();
        let mut tracks = Vec::new();
        for written in declaration.score().map(|score| score.parts()).unwrap_or_default() {
            let name = written.name().unwrap_or_default();
            let span = crate::resolve::trimmed_span(written.syntax());
            if parts.iter().any(|part| part.name == name) {
                self.repeated(
                    &mut whole,
                    format!("this score already has a part called `{name}`"),
                    span,
                );
                continue;
            }
            // Numbered only once the name is accepted, so a refused part costs
            // no id and the surviving ones keep the numbers they would have had.
            let id = u32::try_from(parts.len()).unwrap_or(u32::MAX);
            let (stated, counted) = self.part_context(&written, id, &mut whole);
            context.extend(stated);
            let mut voices: Vec<Voice> = Vec::new();
            for (index, item) in written.items().into_iter().enumerate() {
                let voice = u32::try_from(index).unwrap_or(u32::MAX);
                // Both kinds of item name a voice before anything of it is
                // read: a written one by its own identifier, an instance by
                // the `as` name at the site. Asked here rather than in either
                // branch because "this part already has a voice called that"
                // is one question about both.
                let (voice_name, span, named_at) = match item {
                    musa_language::ast::PartItem::Voice(ref held) => (
                        held.name().unwrap_or_default(),
                        crate::resolve::trimmed_span(held.syntax()),
                        crate::resolve::token_span(held.syntax(), musa_language::SyntaxKind::Identifier),
                    ),
                    musa_language::ast::PartItem::Make(ref site) => (
                        site.alias().unwrap_or_default(),
                        crate::resolve::trimmed_span(site.syntax()),
                        Some(crate::resolve::trimmed_span(site.syntax())),
                    ),
                };
                if voices.iter().any(|earlier| earlier.name == voice_name) {
                    self.repeated(
                        &mut whole,
                        format!("part `{name}` already has a voice called `{voice_name}`"),
                        span,
                    );
                    continue;
                }
                // Numbered here, where a voice begins: everything read under
                // this reading — the voice's own statements and every block
                // nested in them — carries the voice as the declaration that
                // wrote it, which is what `#n` in a printed fact names.
                let entered = self.sites.declaring();
                let read_under = Reading::at(musa_score::Scope::Voice { part: id, voice }, entered).metered(opening);
                let held_at = match suggested {
                    Some(key) => read_under.keyed(key),
                    None => read_under,
                };
                let read = match item {
                    musa_language::ast::PartItem::Voice(held) => {
                        self.plain(&held, voice, voice_name, named_at, held_at)
                    }
                    musa_language::ast::PartItem::Make(site) => self.made(
                        &mut instances,
                        &site,
                        &format!("score/part[{name}]/{index}"),
                        voice,
                        named_at,
                        held_at,
                    ),
                };
                let Some(mut read) = read else {
                    whole = false;
                    continue;
                };
                if let Some(made) = standing {
                    let at = self.origin(node);
                    read.track = applied(
                        at,
                        Raw::hosted(at, "instanced"),
                        [Raw::lit(at, crate::registry::origin_literal(made.clone())), read.track],
                    );
                }
                tracks.push(read.track.clone());
                voices.push(read);
            }
            parts.push(Part {
                id,
                name_span: crate::resolve::token_span(written.syntax(), musa_language::SyntaxKind::Identifier),
                name,
                meter: counted,
                profile: written.profile().map(|statement| {
                    (
                        statement.name().unwrap_or_default(),
                        crate::resolve::trimmed_span(statement.syntax()),
                    )
                }),
                voices,
            });
        }
        // The header's facts cover the piece, and the piece's extent is a
        // fact about its *music*: a voice whose length arrives through a `use`
        // is as long as the material it names, which the written tree does not
        // say, so the number is read off the evaluated tracks
        // (`track_duration`) rather than summed off the source the way one
        // statement's own length is.
        let bound = self.mint("music");
        let music = simultaneous(origin, tracks);
        let extent = applied(
            origin,
            Raw::hosted(origin, "track_duration"),
            [Raw::var(origin, bound.as_str())],
        );
        let laid: Vec<Raw> = context
            .into_iter()
            // Placed: a header stands at exactly one place in the piece, so its
            // facts carry the span they were written at rather than the shared
            // placeholder a body usable at several places has to carry.
            //
            // Under no declaration, which is the honest answer rather than a
            // gap: `tempo 1/4 = 96;` is the piece speaking, and a header is
            // written in no motif, no bar, and no voice.
            .map(|(scope, said, fact)| {
                self.sounded_in(
                    said,
                    scope,
                    true,
                    musa_score::origin::DeclarationId::default(),
                    fact,
                    extent.clone(),
                )
            })
            .collect();
        // One `let`, not two inlines of the same term: `Shape::Let` evaluates
        // the bound term once and shares the *value*, so the music is folded
        // a single time whether or not the header stated anything.
        let track = Raw::bind(
            origin,
            bound.clone(),
            music,
            simultaneous(
                origin,
                vec![Raw::var(origin, bound.as_str()), simultaneous(origin, laid)],
            ),
        );
        whole.then(|| Piece { track, parts })
    }

    /// A name the score already used, at the declaration that used it again.
    ///
    /// Two declarations of one name is always an error here for the reason
    /// `crate::resolve` gives for motifs: musa has no shadowing. What is
    /// specific to a part and a voice is that the name is also how a projection
    /// is read back, so a repeat would make two lanes indistinguishable.
    fn repeated(&mut self, whole: &mut bool, said: String, span: musa_score::origin::SourceSpan) {
        *whole = false;
        self.refuse::<()>(
            Diagnostic::error(Code::DuplicateName, said)
                .at(span, "declared again here")
                .help("rename one of them, or delete this declaration"),
        );
    }

    /// The facts a piece's own header states, each with the scope it is about
    /// and the origin of the statement that said it.
    ///
    /// The origin travels beside the fact rather than being the piece's,
    /// because `sounded` writes it into the constructed fact and a composer
    /// asking where the 7/8 came from wants the line that says `meter 7/8`.
    ///
    /// The meter is there whether or not anybody wrote one, because an
    /// unwritten meter is still a meter and 4/4 governs the piece either way.
    /// The key and the tempo are not: a page with no marking on it is a page
    /// with no marking on it, and the 120 a performance falls back to is the
    /// performance layer's default rather than something the piece said.
    fn header(
        &mut self,
        declaration: &musa_language::ast::PieceDecl,
        whole: &mut bool,
    ) -> Vec<(musa_score::Scope, Origin, Raw)> {
        let mut said = Vec::new();
        match declaration.meter() {
            Some(statement) => match self.stated(statement.syntax(), Context::Meter) {
                Some((origin, fact)) => said.push((musa_score::Scope::Piece, origin, fact)),
                None => *whole = false,
            },
            // Unwritten, so [`Origin::UNKNOWN`]: the table hands out no number
            // for it and `Sites::span` answers [`None`], which is what
            // `provenance_at` already reads as "nowhere of its own to point".
            None => said.push((musa_score::Scope::Piece, Origin::UNKNOWN, unmeasured(Origin::UNKNOWN))),
        }
        for (statement, which) in [
            declaration.tempo().map(|held| (held.syntax().clone(), Context::Tempo)),
            declaration.key().map(|held| (held.syntax().clone(), Context::Key)),
        ]
        .into_iter()
        .flatten()
        {
            match self.stated(&statement, which) {
                Some((origin, fact)) => said.push((musa_score::Scope::Piece, origin, fact)),
                None => *whole = false,
            }
        }
        said
    }

    /// The facts a part's own header states, at [`musa_score::Scope::Part`].
    ///
    /// Read through [`crate::resolve::part_facts`] rather than off the nodes,
    /// because that reading already exists and already carries the two refusals
    /// this one would otherwise have to invent — a second `clef` is two answers
    /// to one question, and an unreadable meter is not a meter. What is left
    /// here is writing the three values it answers as the raw terms
    /// `registry::notation` reads back, which is `AGENTS.md`'s "hand a consumer
    /// what we already computed" at the one place it applies in this module.
    ///
    /// A part that complained answers nothing: the piece is refused either way,
    /// so a half-read context is a term nobody will look at.
    fn part_context(
        &mut self,
        part: &musa_language::ast::PartDecl,
        id: u32,
        whole: &mut bool,
    ) -> (Vec<(musa_score::Scope, Origin, Raw)>, Option<musa_score::Meter>) {
        let Some(facts) = self.heard(|resolver| crate::resolve::part_facts(resolver, part)) else {
            *whole = false;
            return (Vec::new(), None);
        };
        let counted = facts.meter.map(|(meter, _)| meter);
        let scope = musa_score::Scope::Part { part: id };
        let mut said = Vec::new();
        if let Some((clef, span)) = facts.clef {
            let origin = self.sites.at(span);
            said.push((scope, origin, super::notation::clefed(origin, clef)));
        }
        if let Some((meter, span)) = facts.meter {
            let origin = self.sites.at(span);
            said.push((scope, origin, super::notation::metered(origin, meter)));
        }
        if let Some((marking, span)) = facts.tempo {
            let origin = self.sites.at(span);
            said.push((scope, origin, super::notation::tempo(origin, &marking)));
        }
        (said, counted)
    }

    /// One header statement, as the fact it states and the origin that states it.
    fn stated(&mut self, node: &SyntaxNode, which: Context) -> Option<(Origin, Raw)> {
        let origin = self.origin(node);
        let fact = self.fact(node, origin, crate::resolve::trimmed_span(node), which)?;
        Some((origin, fact))
    }

    /// A voice written out among a part's items.
    ///
    /// [`None`] when its body was refused, and when it takes parameters: a voice
    /// with parameters is a template, and a template standing among a part's
    /// items is one declared in the wrong place rather than an instance of
    /// anything.
    fn plain(
        &mut self,
        held: &musa_language::ast::VoiceDecl,
        voice: u32,
        name: String,
        name_span: Option<musa_score::origin::SourceSpan>,
        reading: Reading,
    ) -> Option<Voice> {
        if held.is_template() {
            return self.refuse(
                Diagnostic::error(Code::Misplaced, "a voice with parameters needs `template`")
                    .at(
                        crate::resolve::trimmed_span(held.syntax()),
                        "this voice takes parameters",
                    )
                    .help("write it as `template voice …` at the top of the file, and `make` it here"),
            );
        }
        self.restart_sites();
        let track = self.notated(held.syntax(), reading)?;
        Some(Voice {
            id: voice,
            name,
            name_span,
            track: joined(self.origin(held.syntax()), track),
            // Taken here, where the voice they were written in is still the
            // thing being read: the walk is shared across the score, so leaving
            // them would give the next voice this one's bars.
            claims: self.claimed(),
        })
    }

    /// A `make` standing where a voice would, as the voice it makes.
    ///
    /// Expansion is a **binding** and never a rewrite
    /// (`04-templates-and-modules.md` §1), and in a core with λ that sentence is
    /// the implementation: the template's body is read once, at the scope of the
    /// voice the site stands in, and its parameters become λs applied to the
    /// argument expressions the site wrote. λ rather than `let` because every
    /// argument then elaborates in the scope the *site* stands in — a chain of
    /// `let`s would put the first parameter in scope of the second argument, so
    /// a site inside a template that passed on its own `subject` would silently
    /// pass the template's parameter of that name instead. No syntax is copied
    /// and no span moves, so a refusal inside a template body points at the text
    /// the author wrote once, however many instances there are.
    ///
    /// The reading needs no re-scoping afterwards for the same reason: a
    /// `Reading` carries the scope down, and the one handed here is the voice's
    /// own. What the site adds beyond the binding is provenance, and that is
    /// `instanced` rather than anything this walk could do — the facts do not
    /// exist until the term is evaluated, and the ones a function the body calls
    /// produced were read in another declaration entirely.
    fn made(
        &mut self,
        instances: &mut Instances,
        site: &musa_language::ast::MakeStmt,
        path: &str,
        voice: u32,
        name_span: Option<musa_score::origin::SourceSpan>,
        reading: Reading,
    ) -> Option<Voice> {
        let instance = instances.templates.instance(
            self.resolver,
            site,
            path,
            crate::template::Kind::Voice,
            instances.enclosing.as_deref(),
            &instances.namespace,
        )?;
        let held = instance.voice()?;
        let at = self.sites.at(instance.span());
        self.restart_sites();
        let read = self.notated(held.syntax(), reading);
        let claims = self.claimed();
        // Every parameter is read even after one is refused, for the reason the
        // score walk reads every voice: a site with two bad arguments is two
        // mistakes.
        let mut whole = read.is_some();
        let mut bound = Vec::new();
        for parameter in instance.bound() {
            let origin = self.origin(parameter.argument);
            match (self.ty(parameter.ty), self.expr(parameter.argument)) {
                (Some(ty), Some(argument)) => bound.push((parameter.name.to_owned(), origin, ty, argument)),
                _ => whole = false,
            }
        }
        let stamped = applied(
            at,
            Raw::hosted(at, "instanced"),
            [
                Raw::lit(
                    at,
                    crate::registry::origin_literal(expansion(instance.span(), instance.step())),
                ),
                read?,
            ],
        );
        let abstracted = bound.iter().rev().fold(stamped, |body, (name, origin, ty, _)| {
            Raw::annotated_lam(*origin, name.clone(), ty.clone(), body)
        });
        let track = bound
            .into_iter()
            .fold(abstracted, |function, (_, origin, _, argument)| {
                Raw::app(origin, function, argument)
            });
        whole.then(|| Voice {
            id: voice,
            name: instance.alias().to_owned(),
            name_span,
            track: joined(at, track),
            claims,
        })
    }
}

/// `joined(track)` — the voice's tied noteheads read as the sounds they spell.
///
/// Applied here, at a voice, and at no smaller thing, because both halves of
/// what `joined` answers are questions about a whole voice. A tie at the end of
/// a repeat body or a slur continues into whatever follows the block, so a
/// merge done inside one would report a dangling tie at every nesting level; and
/// "this tie has nothing to tie to" is only true at the end of a voice, which is
/// the one place there is nothing after.
///
/// Unconditionally, rather than only when the voice writes a `~`: the reading
/// folds statements without looking at them, a `use` can bring in material that
/// ends in a tie, and the rule answers its argument unchanged when no fact is
/// marked.
fn joined(origin: Origin, track: Raw) -> Raw {
    Raw::app(origin, Raw::hosted(origin, "joined"), track)
}

/// What every `make` in one piece is resolved against.
///
/// One per piece, because all three are the same for every site and none of
/// them can be read off a part item: the templates a document declares stand at
/// its lexical root, the namespace a generated identity is minted in is the
/// document's own name, and the template whose body this piece is — when it is
/// one — is the one name a site inside it may not write.
struct Instances {
    templates: crate::template::Templates,
    namespace: String,
    enclosing: Option<String>,
}

impl Instances {
    /// The templates `node`'s document declares, and the two facts about `node`
    /// every site inside it is resolved with.
    ///
    /// Collected whether or not the piece contains a site, because a template
    /// declared twice is reported here and that is where the mistake is: the
    /// second declaration is the error and a site that calls it is innocent.
    fn of(resolver: &mut crate::resolve::Resolver, node: &SyntaxNode, namespace: &str) -> Self {
        let root = node.ancestors().last().unwrap_or_else(|| node.clone());
        Self {
            templates: crate::template::Templates::collect(resolver, &root),
            namespace: namespace.to_owned(),
            enclosing: node
                .parent()
                .and_then(musa_language::ast::TemplateDecl::cast)
                .and_then(|template| template.name()),
        }
    }
}

/// `together` over `tracks`, right-nested, seeded with the empty track.
///
/// Right-nested rather than folded from the left because `together` is
/// associative and commutative (L4, L5), so the shape is free and the one that
/// reads as "this voice, beside all the rest" is the one written.
fn simultaneous(origin: Origin, tracks: Vec<Raw>) -> Raw {
    tracks
        .into_iter()
        .rev()
        .fold(Raw::lit(origin, crate::registry::empty_track()), |rest, track| {
            applied(origin, Raw::hosted(origin, "together"), [track, rest])
        })
}

impl Lowering<'_> {}

/// `Fact.Meter 4 4`, for a piece that wrote none.
fn unmeasured(origin: Origin) -> Raw {
    let meter = musa_score::score::Meter::default();
    applied(
        origin,
        Raw::hosted(origin, "Fact.Meter"),
        [
            super::whole(origin, u64::from(meter.numerator())),
            super::whole(origin, u64::from(meter.denominator())),
        ],
    )
}
