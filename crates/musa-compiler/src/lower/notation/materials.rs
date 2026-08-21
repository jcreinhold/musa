//! Materials: `senza`, `mobile`, `arranged`, `declared_material`, and the spoken/sounded families.

use musa_calculus::{Origin, Raw};
use musa_syntax::SyntaxNode;
use musa_syntax::ast::AstNode as _;
use num_rational::Ratio;

use crate::lower::{Lowering, applied, child, is_expr_node, listed, whole};
use musa_score::diagnose::{Code, Diagnostic};
use musa_score::origin::{DeclarationId, SourceSpan};

use super::*;
impl Lowering<'_> {
    /// `senza { … }` — an unmeasured stretch, with the meter put back after it.
    ///
    /// Three statements' worth of fold and no mechanism: `meter none`, the body,
    /// and the meter that was in force. That is what makes it sugar — the
    /// barlines stop for exactly as long as the braces say, and the meter that
    /// resumes is the one that was already there, so there is no second meter to
    /// keep in step with the first.
    ///
    /// The restoring meter is [`Reading::meter`], read lexically. The replaced
    /// path asked its cursor which meter change it had passed, which is the same
    /// answer by a longer route whenever a meter is written where it is read —
    /// and this reading refuses `meter` inside reusable material, so there is no
    /// other case.
    pub(crate) fn senza(
        &mut self,
        node: &SyntaxNode,
        origin: Origin,
        reading: Reading,
        span: SourceSpan,
    ) -> Option<Raw> {
        if !reading.permanent() {
            return self.misplaced("an unmeasured stretch", span);
        }
        let opened = self.sounded_at(
            origin,
            musa_score::Scope::Piece,
            reading.placed,
            reading.declaration,
            metered(origin, musa_score::score::Meter::NONE),
            Ratio::ZERO,
        );
        // Under `meter none` for its whole length, so the body is read with the
        // unmeasured meter in force: a `senza` inside a `senza` restores the one
        // its own braces opened, which is the one that was in force there.
        let body = self.notated(node, reading.metered(musa_score::score::Meter::NONE))?;
        let closed = self.sounded_at(
            origin,
            musa_score::Scope::Piece,
            reading.placed,
            reading.declaration,
            metered(origin, reading.meter),
            Ratio::ZERO,
        );
        Some(applied(
            origin,
            Raw::hosted(origin, "follow"),
            [applied(origin, Raw::hosted(origin, "follow"), [opened, body]), closed],
        ))
    }

    #[expect(
        clippy::arithmetic_side_effects,
        reason = "`Ratio<i64>` addition is exact mathematical arithmetic rather than raw integer ops, the same argument `Lowering::extent` makes; scoped to this function because it is the only arithmetic on it"
    )]
    pub(crate) fn mobile(&mut self, node: &SyntaxNode, origin: Origin, reading: Reading) -> Option<Raw> {
        let statement = musa_syntax::ast::MobileStmt::cast(node.clone())?;
        let span = crate::resolve::trimmed_span(node);
        let names = statement.fragments();
        // One fragment in any order is the fragment. The refusal is the useful
        // part: a `mobile` with one name is almost always a half-finished edit.
        if names.len() < 2 {
            return self.refuse(
                Diagnostic::error(Code::NotAValue, "a mobile arranges at least two fragments")
                    .at(span, format!("this one lists {}", names.len()))
                    .help("write the fragment out instead, or add the ones it is arranged with"),
            );
        }
        let declared = self.declared_material(node);
        let tokens = statement.fragment_tokens();
        let order = self.resolver.decide_order(&self.choice, &names, span);
        let mut played = Vec::with_capacity(order.len());
        let mut over = Ratio::ZERO;
        // Every name is read even after one is refused: a mobile listing two
        // motifs is two mistakes, and stopping at the first would hide the
        // second behind a recompile.
        let mut arranged = true;
        for index in &order {
            let Some(name) = names.get(*index as usize) else {
                continue;
            };
            // The list and its tokens are read off the same tokens, so they
            // cannot disagree about which span this name is.
            let at = tokens.get(*index as usize).map_or(span, crate::resolve::source_span_of);
            let Some(track) = self.arranged(origin, reading, &declared, name, span, at) else {
                arranged = false;
                continue;
            };
            over += declared.get(name.as_str()).map_or(Ratio::ZERO, |held| held.reaches);
            played.push(track);
        }
        if !arranged {
            return None;
        }
        let fact = applied(
            origin,
            Raw::hosted(origin, "Fact.Mobile"),
            [
                listed(
                    origin,
                    names.into_iter().map(|name| plain(origin, "Text", name)).collect(),
                ),
                listed(
                    origin,
                    order.iter().map(|index| whole(origin, u64::from(*index))).collect(),
                ),
            ],
        );
        let marker = self.sounded(origin, reading, fact, over);
        Some(applied(
            origin,
            Raw::hosted(origin, "together"),
            [marker, followed(origin, played)],
        ))
    }

    /// One fragment of a mobile, by name, against what the document declares.
    ///
    /// A mobile arranges *fragments* and nothing else: a motif takes arguments
    /// and a bar is a measure, and neither is material a performance is invited
    /// to reorder. The refusal names what was found, so the mistake is one
    /// sentence rather than a type error one layer down.
    ///
    /// What comes back is what [`Lowering::used`] builds for `use f;`, because
    /// a name in a mobile's list *is* a use of that fragment — played here, in
    /// this voice, and recorded as an expansion so Origin view can say where a
    /// note came from.
    ///
    /// `declared` is handed in rather than looked up. It used to be
    /// `crate::resolve::Resolver::motifs`, which the replaced pass filled while
    /// it walked a piece's header and this reading never does — so under the new
    /// lowering every name in a mobile was refused as undeclared, which is
    /// nineteen refusals for `examples/mobile.musa` alone. The reading was
    /// already walking the document for the extents; asking that one walk what
    /// each name *is* keeps the fix on the side that has the answer, instead of
    /// filling one pass's map from another pass.
    pub(crate) fn arranged(
        &mut self,
        origin: Origin,
        reading: Reading,
        declared: &Declarations,
        name: &str,
        span: SourceSpan,
        at: SourceSpan,
    ) -> Option<Raw> {
        match declared.get(name).map(|held| held.material) {
            Some(crate::resolve::Material::Fragment) => {}
            Some(other) => {
                return self.refuse(
                    Diagnostic::error(
                        Code::Misplaced,
                        format!("`{name}` is a {}, not a fragment", other.word()),
                    )
                    .at(span, "a mobile arranges fragments")
                    .help(format!("declare it as `fragment {name} {{ … }}`")),
                );
            }
            None => {
                let known: Vec<&str> = declared.keys().map(String::as_str).collect();
                let help = crate::resolve::suggest_name(name, &known);
                return self.refuse(
                    Diagnostic::error(Code::UnknownName, format!("cannot find `{name}`"))
                        .at(span, "not declared in this piece")
                        .help(help),
                );
            }
        }
        self.resolver
            .references
            .record_use(crate::resolve::NameKind::Fragment, name, at);
        Some(Self::spoken(origin, reading, span, Raw::var(origin, name)))
    }

    /// The named material this document declares — what each name is, and how
    /// far it reaches.
    ///
    /// Both halves in one walk because a mobile asks both of every name it
    /// lists, and they are answered in the same place: a fragment's extent is
    /// the sum of the statements between its braces, and what makes it a
    /// fragment rather than a motif or a bar is which keyword opened them. A
    /// region's extent is otherwise read off the statements inside its own
    /// braces, and a mobile has none — what it writes are *names*.
    ///
    /// One pass over the document rather than a search per name, so a mobile of
    /// fifty-three figures costs one walk instead of fifty-three.
    pub(crate) fn declared_material(&self, node: &SyntaxNode) -> Declarations {
        use musa_syntax::ast::{BarStmt, FragmentDecl, MotifDecl};
        let Some(root) = node.ancestors().last() else {
            return Declarations::new();
        };
        let mut declared = Declarations::new();
        for held in root.descendants() {
            // A `bar` with no name declares nothing: it is a measure written
            // where it sounds, and only a *named* one is material `use` — or a
            // mobile — could ever reach.
            let named = if let Some(fragment) = FragmentDecl::cast(held.clone()) {
                fragment.name().map(|name| (name, crate::resolve::Material::Fragment))
            } else if let Some(motif) = MotifDecl::cast(held.clone()) {
                motif.name().map(|name| (name, crate::resolve::Material::Motif))
            } else if let Some(bar) = BarStmt::cast(held.clone()) {
                bar.name().map(|name| (name, crate::resolve::Material::Bar))
            } else {
                None
            };
            let Some((name, material)) = named else { continue };
            declared.insert(
                name,
                Reach {
                    material,
                    reaches: self.extent(&held),
                },
            );
        }
        declared
    }

    /// `improvise 8/1 over "Dm7 | G7";` — a frame that sounds as silence.
    pub(crate) fn improvise(&mut self, node: &SyntaxNode, origin: Origin, reading: Reading) -> Option<Raw> {
        let statement = musa_syntax::ast::ImproviseStmt::cast(node.clone())?;
        let span = crate::resolve::trimmed_span(node);
        let (_, held, _) = self.notated_duration(node, span, reading)?;
        let over = statement.over().map(|text| plain(origin, "Text", text));
        let fact = Raw::app(origin, Raw::hosted(origin, "Fact.Improvise"), maybe(origin, over));
        Some(self.sounded_term(origin, reading, fact, held))
    }

    /// `use e;` — the track `e` denotes, in this block's scope, folded on.
    ///
    /// Nothing checks that `e` is a track: §2 says `use e;` "checks that `e` is a
    /// written-time score track", and the check is the core's, because `scoped`'s
    /// signature demands one and the refusal lands at the origin this module gave
    /// the node.
    ///
    /// Two calls, and they are the two things a `use` says about material that
    /// was written somewhere else.
    ///
    /// `scoped` is what makes reusable material reusable. `e` was read at
    /// [`musa_score::Scope::Piece`] — a fragment is "usable at several places" and so
    /// has none of its own — and this is the place, so its facts take the scope
    /// of the block that played them. Here rather than around the whole voice,
    /// because a voice may itself write `key g major;`, which is a piece-scoped
    /// fact deliberately ([`Self::context`]) and would be relabelled into one
    /// voice's private key by a wrapper that could not tell the two apart. A
    /// `use` inside free material relabels `Piece` to `Piece` and costs a
    /// reduction step, which is the price of the rule having no exception.
    ///
    /// `instanced` is what makes it *this* playing of it.
    /// [`musa_score::origin::ExpansionStep::MotifApplication`] is the step Origin view reads to tell
    /// a composer's own notes from material spoken by name, and every consumer of
    /// it — the derivation graph's key, the fact-text spelling, and the
    /// repeat-agreement rule in [`crate::project`], which writes a repeat out
    /// rather than complaining when the repeat came from shared material — asks
    /// that question of a *use site*. The builtin and not a stamp applied while
    /// reading, for its own documented reason: the facts do not exist until the
    /// term is evaluated, and the ones a function `e` calls produced were read in
    /// another declaration entirely.
    ///
    /// Inside `scoped` rather than outside, because relabelling a scope and
    /// recording an expansion commute and the nesting should read the way the
    /// sentence does: this material, played here, belongs to this voice.
    pub(crate) fn used(&mut self, node: &SyntaxNode, origin: Origin, reading: Reading) -> Option<Raw> {
        let called = child(node, is_expr_node)?;
        let material = self.value(&called)?;
        let specialized = self.specialized(node, origin, material)?;
        Some(Self::spoken(
            origin,
            reading,
            crate::resolve::trimmed_span(node),
            specialized,
        ))
    }

    /// `material` with this `use`'s `with { note n = p; }` clause applied, or
    /// `material` unchanged when it wrote none.
    ///
    /// One `respelled` per override, innermost first, so a clause that names
    /// two notes reads as two edits of one occurrence rather than one edit of a
    /// list. Inside [`Lowering::spoken`]'s `instanced` rather than around it,
    /// because the respelling happened *within* this playing of the material
    /// and `04-provenance.md` reads the path outside-in: the note comes out
    /// `motif ▸ specialized`, which is the order a reader asking about it walks.
    ///
    /// **Three refusals here and three in the builtin, and the split is not
    /// arbitrary.** A position of zero, a clause with no pitch, and one note
    /// named twice are properties of the *text*, so they are answered where the
    /// text is. How many notes the occurrence has, whether the one named is a
    /// chord, and whether it is a rest are properties of the material, and the
    /// material is a term until it is evaluated — 141k's fold reported those by
    /// elaborating the body while it walked, and there is nothing to elaborate
    /// here. [`crate::registry::track`]'s `respelled` answers them on 141m's
    /// refusal channel instead.
    pub(crate) fn specialized(&mut self, node: &SyntaxNode, origin: Origin, material: Raw) -> Option<Raw> {
        use musa_syntax::ast::AstNode as _;

        let Some(call) = musa_syntax::ast::UseStmt::cast(node.clone()) else {
            return Some(material);
        };
        let mut named: Vec<u64> = Vec::new();
        let mut specialized = material;
        for each in call.overrides() {
            let at = crate::resolve::trimmed_span(each.syntax());
            let Some(position) = each
                .position()
                .and_then(|text| text.parse::<u64>().ok())
                .filter(|counted| *counted > 0)
            else {
                return self.refuse(
                    Diagnostic::error(Code::OutOfRange, "notes are counted from `note 1`")
                        .at(at, "there is no note 0")
                        .note("the first note of the occurrence is `note 1`"),
                );
            };
            let Some(pitch) = each.pitch().as_deref().and_then(musa_score::pitch::WrittenPitch::parse) else {
                return self.refuse(
                    Diagnostic::error(Code::NotAValue, "this override names no pitch")
                        .at(at, "expected a pitch")
                        .note("`note 2 = f5;` writes `f5` onto the second note"),
                );
            };
            if named.contains(&position) {
                return self.refuse(
                    Diagnostic::error(Code::DuplicateName, format!("note {position} is overridden twice"))
                        .at(at, "the second of two")
                        .note("one note takes one spelling, so one of these two says nothing"),
                );
            }
            named.push(position);
            let step = crate::lower::expansion(
                at,
                musa_score::origin::ExpansionStep::Specialization { override_site: at },
            );
            specialized = applied(
                origin,
                Raw::hosted(origin, "respelled"),
                [
                    Raw::lit(origin, crate::registry::origin_literal(step)),
                    crate::lower::whole(origin, position),
                    plain(origin, "Pitch", pitch),
                    specialized,
                ],
            );
        }
        Some(specialized)
    }

    /// `material`, played here — the two builtins [`Lowering::used`] documents,
    /// with `at` as the call site.
    ///
    /// Its own function because [`Lowering::arranged`] wants the same two: a
    /// name in a mobile's list is material spoken by name at a place, which is
    /// what `use` is.
    pub(crate) fn spoken(origin: Origin, reading: Reading, at: SourceSpan, material: Raw) -> Raw {
        let played = stamped(
            origin,
            at,
            musa_score::origin::ExpansionStep::MotifApplication { call_site: at },
            material,
        );
        applied(
            origin,
            Raw::hosted(origin, "scoped"),
            [scope_of(origin, reading.scope), played],
        )
    }

    /// `sounded(origin, scope, fact, held)`.
    ///
    /// The construction §5.7 requires: every fact carries an origin and a scope,
    /// and neither is something a source line writes or a `fn` pointer invents.
    /// The reading supplies both, which is why the call has four arguments where
    /// the statement had none.
    ///
    /// Nothing to ask any more, and nothing to answer: a length a fact cannot
    /// sound for is refused by the rule at this origin (prompt 141m), so what
    /// comes back is a call and every caller gets one. The `Option` that used to
    /// be here was the `?` this reading wrote, and there is no `?` left to write.
    pub(crate) fn sounded(&self, origin: Origin, reading: Reading, fact: Raw, held: Ratio<i64>) -> Raw {
        self.sounded_at(origin, reading.scope, reading.placed, reading.declaration, fact, held)
    }

    /// The same, with the sounding duration already a term: a duration written
    /// as a parameter is a value the evaluation reads, not one this lowering
    /// can bake.
    pub(crate) fn sounded_term(&self, origin: Origin, reading: Reading, fact: Raw, held: Raw) -> Raw {
        self.sounded_in(origin, reading.scope, reading.placed, reading.declaration, fact, held)
    }

    /// The same, at a scope the reading does not supply.
    ///
    /// Three callers want one: a `clef` written in a voice is the *part's*
    /// clef, a `key` or a `meter` written in a voice is the *piece's*, and the
    /// header facts [`super::piece`] builds belong to the part or the piece that
    /// wrote them rather than to any voice. §5.7 asks which scope a fact is
    /// constructed at, and the answer is not always the scope it was written in.
    pub(crate) fn sounded_at(
        &self,
        origin: Origin,
        scope: musa_score::Scope,
        placed: bool,
        declaration: DeclarationId,
        fact: Raw,
        held: Ratio<i64>,
    ) -> Raw {
        self.sounded_in(origin, scope, placed, declaration, fact, written_duration(origin, held))
    }

    /// The call, with all four arguments in hand.
    ///
    /// `pub(super)` for [`super::piece`]'s header: a header fact's `held` is
    /// the piece's evaluated extent, which is a term, not the ratio a
    /// statement's own written duration gives [`Self::sounded_at`].
    pub(crate) fn sounded_in(
        &self,
        origin: Origin,
        scope: musa_score::Scope,
        placed: bool,
        declaration: DeclarationId,
        fact: Raw,
        held: Raw,
    ) -> Raw {
        applied(
            origin,
            Raw::hosted(origin, "sounded"),
            [
                self.provenance_at(origin, placed, declaration),
                scope_of(origin, scope),
                fact,
                held,
            ],
        )
    }
}
