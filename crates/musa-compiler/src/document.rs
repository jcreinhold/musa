//! A whole document, elaborated through `musa-core`.
//!
//! [`crate::lower`] reads one declaration and [`crate::registry`] says what the
//! compiler's own vocabulary is. This is the module that puts the two together
//! and walks a *document*: every library it imports, its own lexical root, and
//! the piece or voice being checked.
//!
//! # What a document is, for this module
//!
//! A sequence of [`Source`]s, each a syntax node whose children are
//! declarations, and each carrying the one fact about it that lowering cannot
//! read off the node: whether `02-core-calculus.md` §5.9's phase vocabulary is
//! spellable inside it. Import *order* is the caller's — [`crate::imports`]
//! already answers it, and a module that learned about import graphs would be
//! doing name resolution twice.
//!
//! # The order the three doors are opened in
//!
//! `musa-core` has three that a document walks through: [`musa_core::declare`]
//! for a family group, [`musa_core::declare_trait`], and
//! [`musa_core::declare_program`] for everything a body can be. They are not
//! interchangeable and the order between them is forced:
//!
//! 1. **Families first**, ordered among themselves, because a `data` field is a
//!    type and a type is a family or a base type.
//! 2. **Traits**, whose method types may name any family.
//! 3. **Definitions and instances together**, in one group, because §2.4's
//!    forward reference is a property of the group rather than of a written
//!    order, and because each of the two kinds can name the other: a method body
//!    is an ordinary term that may call any definition, and a definition that
//!    writes `x.m(y)` resolves it at the receiver's head, which is an instance.
//!    Declaring either kind first makes it blind to the other, which is what
//!    [`musa_core::declare_program`] exists to avoid.
//!
//! The one shape this order cannot express is a family whose field names a
//! `record`, since `01-surface.md` §1.2 makes a record a *definition*. That is a
//! real limit and it is stated rather than worked around: the core refuses it by
//! name, at the field, and no document in this repository writes one.
//!
//! # Why the context is rebuilt per document
//!
//! [`crate::registry::owned`] declares the prelude and registers every builtin
//! each time it is called. That is measurable, and it is prompt 144's to
//! measure: sharing one context means deciding what it does about a document
//! that declares a private family of its own, and deciding that before there is
//! a number would be guessing.

#[cfg(test)]
pub(crate) mod laws;

use std::collections::BTreeSet;
use std::sync::Arc;

use musa_core::{Cx, ElabError, Name, Origin, Program, Raw, RawData, RawProgram, RawTopLevel, Term, Visibility};
use musa_language::ast::AstNode as _;
use musa_language::{SyntaxKind, SyntaxNode};

use crate::diagnose::{Code, Diagnostic};
use crate::elaborate::VoiceTrack;
use crate::lower::items::{Declared, Definition, Item};
use crate::lower::notation::{Argued, Claimed};
use crate::lower::{Lowering, Naming, Sites, refusals};
use crate::module::{MemberItem, Modules};
use crate::resolve::Resolver;

/// One node whose children are declarations.
///
/// [`Clone`] because a corpus of sources is read once and elaborated many
/// times, each with a different file added to it: a [`SyntaxNode`] is a
/// refcounted handle, so the copy is a pointer and not a parse.
#[derive(Clone)]
pub(crate) struct Source {
    /// The node itself: a `library`, a document root, a `piece`, or a `voice`.
    pub(crate) root: SyntaxNode,
    /// The document it was written in, when that is not this one.
    ///
    /// Carried because a diagnostic about an imported declaration is stated at
    /// the `import` that pulled it in rather than at a span in a file this one
    /// is not: a span inside a foreign CST is meaningless here, so what a
    /// refusal needs is the path.
    pub(crate) from: Option<String>,
    /// Whether `02-core-calculus.md` §5.9's phase vocabulary is readable here.
    pub(crate) in_phase: bool,
}

impl Source {
    /// A node this document wrote itself, read as ordinary source.
    ///
    /// The two fields the constructor does not take are what a source almost
    /// always is: written here, and out of phase. [`Self::imported`] and
    /// [`Self::in_phase`] name the exceptions, so a caller that has neither
    /// says neither.
    pub(crate) fn own(root: &SyntaxNode) -> Self {
        Self {
            root: root.clone(),
            from: None,
            in_phase: false,
        }
    }

    /// A node the import at `from` supplied.
    pub(crate) fn imported(root: &SyntaxNode, from: &str) -> Self {
        Self {
            from: Some(from.to_owned()),
            ..Self::own(root)
        }
    }

    /// The same source, read where §5.9's phase vocabulary is nameable.
    pub(crate) fn in_phase(mut self) -> Self {
        self.in_phase = true;
        self
    }
}

/// A document's declarations, elaborated, and the context they are in.
///
/// Opaque on purpose, for [`musa_core::declare`]'s reason one level up: a
/// consumer asks this what a name *means* and never what the elaborator did to
/// find out.
pub(crate) struct Document {
    cx: Cx,
    sites: Sites,
    /// Kept so that the context this answers in is the context the definitions
    /// were elaborated into. Nothing reads the handle itself.
    _definitions: Arc<Program>,
    names: Vec<Name>,
    /// This document's modules, kept because a *piece* is read after
    /// elaboration and reads names the same way a declaration does: a voice
    /// that writes `key Away.tonic;` is naming a module member, and a walk
    /// that had forgotten the modules would read it as a projection.
    modules: Modules,
    /// The expansion a root `make` put this document's whole piece behind, when
    /// the file writes one instead of a piece.
    ///
    /// A property of the *document* rather than an argument to
    /// [`Document::piece`], because it is the same answer for every reading this
    /// document can be asked for: the file is the template's body, so everything
    /// in it was produced by the one site that made it.
    standing: Option<crate::origin::Origin>,
}

impl Document {
    /// Every name this document bound, in the order it was written.
    pub(crate) fn names(&self) -> &[Name] {
        &self.names
    }

    /// The normal form of what `name` denotes, with its type.
    ///
    /// One line rather than a second reading, because a name *is* a term: the
    /// declarations this document brought into scope are what make it one, and
    /// resolving it through [`Document::term`] is what makes a use of a
    /// definition mean here exactly what it means in a body.
    ///
    /// # Errors
    ///
    /// [`ElabError`] when `name` is not bound here, or when normalizing it
    /// exhausts the budget.
    pub(crate) fn value(&self, name: &str) -> Result<(Term, Term), ElabError> {
        self.term(&Raw::var(Origin::UNKNOWN, name))
    }

    /// The normal form of `raw` in this document's context, with its type.
    ///
    /// Goes through [`musa_core::infer`] rather than reaching into the group,
    /// which is what makes this the same reading a source term gets: the term is
    /// elaborated, its type is inferred, and the value is normalized *at* that
    /// type, so η applies where the type says it should.
    ///
    /// # Errors
    ///
    /// [`ElabError`] when `raw` does not elaborate here, or when normalizing it
    /// exhausts the budget.
    pub(crate) fn term(&self, raw: &Raw) -> Result<(Term, Term), ElabError> {
        let (term, ty) = musa_core::infer(&self.cx, raw)?;
        let normal = musa_core::normalize(&self.cx, &ty, &term)?;
        Ok((normal, ty))
    }

    /// The written-time track `raw` denotes.
    ///
    /// The readback, and the whole of it. A caller asks this crate for the
    /// *music* a piece is and never for the term that computed it: the normal
    /// form, the literal canonicity promises it is, and the payload that literal
    /// holds are three steps a consumer would otherwise take in the open, and a
    /// consumer that took them could also take a different three.
    ///
    /// # Errors
    ///
    /// [`ElabError`] when `raw` does not elaborate here, when normalizing it
    /// exhausts the budget, or when the normal form does not hold a track —
    /// which is [`crate::registry::read_back`]'s compiler defect rather than a
    /// program's, since the term was checked before it was read.
    pub(crate) fn track(&self, raw: &Raw) -> Result<VoiceTrack, ElabError> {
        let (normal, _) = self.term(raw)?;
        Ok(crate::registry::read_back::<VoiceTrack>(&normal)?.clone())
    }

    /// What `claimed` claims, and where it sits in its voice.
    ///
    /// The other half of the readback, and the reason [`Claimed`] holds terms
    /// rather than numbers. A fold has no cursor, so where a passage *begins* is
    /// how long the music before it lasts — one duration, read the same way the
    /// passage's own is — and the answer is exact rational arithmetic rather
    /// than a running position some walk had to keep correct.
    ///
    /// The claim comes back beside the passage because it is finished here too:
    /// a written `assert` records the *terms* of its arguments, and a term has
    /// no value until this document is elaborated. Building the claim in the
    /// same call is what lets the notes be gathered only for the claims that
    /// read them ([`crate::assert::Claim::reads_notes`]), so a bar pays for its
    /// duration and nothing else. They are relative to the passage, because
    /// that is what a claim is about: `assert voices(4)` counts what sounds
    /// together inside the braces, and where the braces stand is the measure
    /// claim's business.
    ///
    /// # Errors
    ///
    /// [`ElabError`] for [`Self::track`]'s reasons, on either of the two terms
    /// or on any argument, and [`musa_core::Malformed::NotALiteral`] when an
    /// argument's normal form does not hold what its shape declares — which is
    /// this crate's defect rather than a program's, since every argument was
    /// checked at that shape's own type.
    pub(crate) fn passage(
        &self,
        claimed: &Claimed,
    ) -> Result<(crate::assert::Claim, crate::assert::Passage), ElabError> {
        let claim = self.claim(claimed)?;
        let before = self.track(&claimed.before)?;
        let sounding = self.track(&claimed.passage)?;
        let notes = if claim.reads_notes() {
            sounding
                .occurrences()
                .iter()
                .filter_map(|occurrence| {
                    Some(crate::assert::Sounded {
                        pitch: occurrence.payload().pitch_of()?,
                        start: crate::MusicalTime::new(occurrence.span().start().as_ratio()),
                        end: crate::MusicalTime::new(occurrence.span().end().as_ratio()),
                        // The span of the note itself and not of whatever played
                        // it: a note generated from a motif is written in the
                        // motif, and that is where a composer goes to change it.
                        at: occurrence.payload().origin.definition_span,
                    })
                })
                .collect()
        } else {
            Vec::new()
        };
        Ok((
            claim,
            crate::assert::Passage {
                span: claimed.span,
                at: crate::MusicalTime::new(before.duration().as_ratio()),
                extent: crate::MusicalDuration::new(sounding.duration().as_ratio()),
                content_end: claimed.content_end,
                notes,
                noun: claimed.noun,
            },
        ))
    }

    /// The claim `claimed` writes, with its arguments evaluated.
    ///
    /// Each value argument is elaborated exactly the way any other written term
    /// is — the annotation [`crate::lower::notation`] wrapped it in is what
    /// checks it at the shape the registry declares — and then read back into
    /// the host shape by [`crate::registry::argument`]. The words were resolved
    /// where they were written and are copied through.
    ///
    /// # Errors
    ///
    /// As [`Self::passage`].
    fn claim(&self, claimed: &Claimed) -> Result<crate::assert::Claim, ElabError> {
        // One sentence for both failures below, because they are one defect:
        // an argument that was checked at its shape's type and does not read
        // back as that shape means this crate's registration and its reading
        // disagree, which no program can cause and no diagnostic can repair.
        let broken = || ElabError::from(musa_core::Malformed::NotALiteral(claimed.predicate.name.into()));
        let mut arguments = Vec::with_capacity(claimed.arguments.len());
        for (argued, shape) in claimed.arguments.iter().zip(claimed.predicate.parameters) {
            arguments.push(match *argued {
                Argued::Word(ref word) => word.clone(),
                Argued::Value(ref raw) => {
                    let (normal, _) = self.term(raw)?;
                    crate::registry::argument(*shape, &normal).ok_or_else(broken)?
                }
            });
        }
        crate::assert::Claim::build(claimed.predicate.name, arguments).ok_or_else(broken)
    }

    /// Every machine this document's declarations describe, by the name each
    /// was bound to.
    ///
    /// A machine is a *value*, and `03-machine-calculus.md` §2 gives its forms
    /// no reductions, so what a consumer prepares is the description the
    /// declaration built rather than anything the machine computed. The names
    /// are this document's own, in written order, which is the order a studio
    /// reads them in.
    ///
    /// A definition whose type is not a machine's costs one type lookup here
    /// and is never normalized, which is what keeps a document of a hundred
    /// definitions from being evaluated to find the two that are machines. A
    /// machine whose ports nothing decided does not reach this at all: the core
    /// refuses an undetermined implicit argument at the declaration, so
    /// `let m = identity;` is a complaint about the ports and never a machine
    /// with none.
    pub(crate) fn machines(&self) -> Vec<(String, crate::MachineSpec)> {
        self.names
            .iter()
            .filter_map(|name| {
                let (term, ty) = musa_core::infer(&self.cx, &Raw::var(Origin::UNKNOWN, &**name)).ok()?;
                let (step, input, output) = crate::registry::machine_ports(&ty)?;
                let normal = musa_core::normalize(&self.cx, &ty, &term).ok()?;
                let nodes = crate::registry::machine_nodes(&normal)?;
                Some((name.to_string(), crate::MachineSpec::new(step, input, output, nodes)))
            })
            .collect()
    }

    /// The table that turns an [`Origin`] this document minted back into a span.
    pub(crate) fn sites(&self) -> &Sites {
        &self.sites
    }

    /// The piece `node` writes, read into this document's own site table.
    ///
    /// Read *after* elaboration and through the same table, which is what makes
    /// a refusal about a voice restatable at the note that caused it: a piece's
    /// structure names the motifs and fragments its declarations bound, so the
    /// declarations have to be in scope before the structure is a term, and the
    /// origins the structure mints have to be numbered by the table the
    /// declarations were.
    /// `namespace` is the document's own name, which is what a template
    /// instance's generated identity is minted in (`04-templates-and-modules.md`
    /// §2). It cannot be read off the tree, and two documents that made the same
    /// template at the same structural address would otherwise mint one identity
    /// between them.
    pub(crate) fn piece(
        &mut self,
        resolver: &mut Resolver,
        node: &SyntaxNode,
        namespace: &str,
    ) -> Option<crate::lower::piece::Piece> {
        let standing = self.standing.clone();
        Lowering::new(resolver, &mut self.sites)
            .naming(Naming::at_root(&self.modules))
            .piece(node, namespace, standing.as_ref())
    }
}

/// Elaborate every declaration `sources` writes, in one context.
///
/// [`None`] with diagnostics reported when anything was refused. Every refusal
/// is restated at the node it is about, so a caller neither sees an
/// [`ElabError`] nor has to know what an [`Origin`] is.
///
/// `made` is the site that made this document's piece, when the file writes
/// `make N(…) as I;` at its root rather than a piece. Its arguments are
/// definitions of this document like any other — see [`instantiated`] — which is
/// why they arrive here and not at [`Document::piece`].
pub(crate) fn elaborate(
    resolver: &mut Resolver,
    sources: &[Source],
    made: Option<&crate::template::Instance>,
) -> Option<Document> {
    let mut sites = Sites::default();
    let modules = modules_in(resolver, sources);
    let mut read = Read::default();
    for source in sources {
        read.gather(resolver, &mut sites, source, &modules);
        read.barred(resolver, &mut sites, source, &modules);
    }
    read.flatten(resolver, &mut sites, &modules);
    // Here rather than beside the core's own refusals below, because a
    // declaration that could not be read leaves the *list* short: every use of
    // the missing name would be checked against a context that never bound it
    // and reported as an unknown name, burying the refusal already stated at
    // the declaration. [`instantiated`] refuses a document for the same reason
    // one line down.
    if read.refused {
        return None;
    }
    if let Some(instance) = made {
        read.definitions
            .extend(instantiated(resolver, &mut sites, instance, &modules)?);
    }
    let mut cx = match crate::registry::owned() {
        Ok(cx) => cx,
        Err(error) => {
            resolver.report(refusals::restate(&sites, &error));
            return None;
        }
    };
    let mut refused = false;
    for (_, group) in order_families(resolver, read.families)? {
        match musa_core::declare(&cx, &group) {
            Ok(declared) => cx = cx.declaring(&declared),
            Err(error) => {
                resolver.report(refusals::restate(&sites, &error));
                refused = true;
            }
        }
    }
    for raw in &read.classes {
        match musa_core::declare_trait(&cx, raw) {
            Ok(declared) => cx = cx.declaring_class(&declared),
            Err(error) => {
                resolver.report(refusals::restate(&sites, &error));
                refused = true;
            }
        }
    }
    let names: Vec<Name> = read.definitions.iter().map(|held| Arc::clone(&held.name)).collect();
    let program = RawProgram {
        definitions: read.definitions,
        instances: read.instances,
    };
    let declared = match musa_core::declare_program(&cx, &program) {
        Ok(declared) => declared,
        Err(error) => {
            resolver.report(refusals::restate(&sites, &error));
            return None;
        }
    };
    cx = cx.defining(&declared);
    if refused {
        return None;
    }
    Some(Document {
        cx,
        sites,
        _definitions: declared,
        names,
        modules,
        standing: made.map(|instance| crate::lower::expansion(instance.span(), instance.step())),
    })
}

/// What one walk of a document's declarations collected, by which door each
/// goes through.
#[derive(Default)]
struct Read {
    /// Each family group beside the node it was written at, because the node is
    /// what [`order_families`] reads the group's dependencies off.
    families: Vec<(SyntaxNode, RawData)>,
    classes: Vec<musa_core::RawTrait>,
    instances: Vec<musa_core::RawImpl>,
    definitions: Vec<RawTopLevel>,
    /// Whether a declaration this walk was supposed to read was refused, which
    /// is a property of the walk rather than of any one door: what it collected
    /// is no longer everything the sources declare, and a document built from a
    /// short list is worse than none — see [`elaborate`].
    refused: bool,
}

impl Read {
    /// Every declaration written directly inside `source`, lowered and filed.
    ///
    /// Directly inside, and not every descendant: a `fn` written inside a
    /// `piece` is the piece's and is gathered when the piece is a source of its
    /// own, which is what keeps a template's body out of the root's scope.
    fn gather(&mut self, resolver: &mut Resolver, sites: &mut Sites, source: &Source, modules: &Modules) {
        for node in source.root.children() {
            let visibility = visibility_of(&node);
            let item = {
                let lowering = if source.in_phase {
                    Lowering::phase(resolver, sites)
                } else {
                    Lowering::new(resolver, sites)
                };
                lowering.naming(Naming::at_root(modules)).item(&node)
            };
            match item {
                Declared::Item(Item::Data(data)) => self.families.push((node, data)),
                Declared::Item(Item::Class(class)) => self.classes.push(class),
                Declared::Item(Item::Instance(instance)) => self.instances.push(instance),
                Declared::Item(Item::Definition(definition)) => {
                    record(resolver, &node, &definition.name);
                    self.definitions.push(top_level(definition, visibility));
                }
                Declared::Refused => self.refused = true,
                Declared::Elsewhere => {}
            }
        }
    }

    /// Every *named* bar in `source`'s score, as a definition of this document.
    ///
    /// The one declaration that is not written where declarations are written.
    /// `bar refrain { … }` sounds in the voice that holds it and binds a name
    /// any `use` below can answer, so it is both a statement and a declaration —
    /// [`crate::lower::Lowering::bar`] reads the first and
    /// [`crate::lower::Lowering::named_bar`] the second. Beside [`Self::gather`]
    /// rather than inside it for [`Self::flatten`]'s reason: a walk of a source's
    /// children finds the `score`, not the bars three levels under it.
    ///
    /// Only from a `piece`. A document root has the piece among its children and
    /// the piece is a source of its own, so a descendant walk from both would
    /// declare every named bar twice and refuse the document for a collision it
    /// invented.
    ///
    /// An anonymous bar declares nothing and is passed over: it is a measure,
    /// which is ordinary structure, and giving it a name here would be inventing
    /// one the source did not write.
    fn barred(&mut self, resolver: &mut Resolver, sites: &mut Sites, source: &Source, modules: &Modules) {
        if source.root.kind() != SyntaxKind::PieceDecl {
            return;
        }
        for node in source.root.descendants() {
            let Some(name) = musa_language::ast::BarStmt::cast(node.clone()).and_then(|bar| bar.name()) else {
                continue;
            };
            let read = Lowering::new(resolver, sites)
                .naming(Naming::at_root(modules))
                .named_bar(&node, &name);
            match read {
                // Private because a score is not an interface: a bar is written
                // inside a voice, and no `import` reaches in there to name it.
                Some(definition) => self.definitions.push(top_level(definition, Visibility::Private)),
                None => self.refused = true,
            }
        }
    }

    /// Every module member and every functor argument, as definitions of this
    /// document.
    ///
    /// `04-templates-and-modules.md` §4's whole claim, discharged: a module is
    /// a name for a group of declarations and not a thing, so a member is an
    /// *ordinary definition* filed under a qualified name, and an instance is
    /// the same declaration read a second time in a scope where the functor's
    /// parameters name what the site passed. Nothing here is a second kind of
    /// item and nothing crosses into [`musa_core`] that a `let` at a root does
    /// not.
    ///
    /// Filed after [`Self::gather`] rather than during it, because a member is
    /// not written where it is checked: `Modules` has already flattened the
    /// tree, and a walk of a source's children would find the `structure` and
    /// not its members. Order does not otherwise matter —
    /// [`musa_core::declare_program`] computes the dependency order over the
    /// whole list.
    fn flatten(&mut self, resolver: &mut Resolver, sites: &mut Sites, modules: &Modules) {
        for member in modules.members() {
            let written = match &member.item {
                MemberItem::Let(declaration) => declaration.syntax().clone(),
                MemberItem::Function(declaration) => declaration.syntax().clone(),
            };
            let read = Lowering::new(resolver, sites)
                .naming(Naming::inside(modules, &member.scope))
                .item(&written);
            match read {
                Declared::Item(Item::Definition(mut definition)) => {
                    // The one thing the walk cannot know: a member is written
                    // as `tonic` and named `CMajor.tonic`, and for an instance
                    // it is the *functor's* declaration filed under the
                    // instance's name. Public because what a structure hides is
                    // enforced where a name is read (`Modules::resolve`), and a
                    // member the core refused to look up would be hidden from
                    // its own siblings too.
                    definition.name = Name::from(member.name.as_str());
                    record(resolver, &written, &definition.name);
                    self.definitions.push(top_level(definition, Visibility::Public));
                }
                Declared::Item(_) | Declared::Elsewhere => {}
                Declared::Refused => self.refused = true,
            }
        }
        for argument in modules.arguments() {
            let mut lowering = Lowering::new(resolver, sites).naming(Naming::at_root(modules));
            let origin = lowering.origin(&argument.expr);
            match (lowering.ty(&argument.ty), lowering.expr(&argument.expr)) {
                (Some(ty), Some(value)) => self.definitions.push(RawTopLevel {
                    origin,
                    name: Name::from(argument.holder.as_str()),
                    // Private for [`instantiated`]'s reason: the holder is a
                    // name this document minted for a value the site wrote, and
                    // it is unspellable anyway.
                    visibility: Visibility::Private,
                    module: None,
                    ty: Some(ty),
                    value,
                }),
                _ => self.refused = true,
            }
        }
    }
}

/// Everything this document's `signature`, `structure`, `template structure`,
/// and module-`make` declarations mean.
///
/// Empty, and read at no cost, for a document that writes none — which is
/// nearly all of them. See [`Modules::written_in`].
fn modules_in(resolver: &mut Resolver, sources: &[Source]) -> Modules {
    if !Modules::written_in(sources.iter().map(|source| &source.root)) {
        return Modules::default();
    }
    Modules::read(
        resolver,
        sources
            .iter()
            .map(|source| (source.from.as_deref(), source.root.clone())),
    )
}

/// The arguments a root `make` gives its template, as definitions of this
/// document.
///
/// A file that writes `make study(…) as …;` where a piece would *is* the
/// template's body, so the template's parameters are that document's own names
/// and the site's arguments are what they are bound to. `04-templates-and-modules.md`
/// §1's "expansion is a binding, never a rewrite" again, and at this level the
/// binding a core has for it is a top-level definition rather than a λ: a piece
/// is not one term but many — the whole, each voice, and each claim's two — and
/// abstracting every one of them over the same parameters would be the same
/// binding written as many times as a score has parts.
///
/// Private, because a parameter is a name inside this document and an importer
/// that could see it would be reading a name the template declared for itself.
///
/// [`None`] with the refusal reported when an argument or a declared parameter
/// type could not be read; the piece is refused either way, and a half-bound
/// document would report every use of the missing name as well.
fn instantiated(
    resolver: &mut Resolver,
    sites: &mut Sites,
    instance: &crate::template::Instance,
    modules: &Modules,
) -> Option<Vec<RawTopLevel>> {
    let mut lowering = Lowering::new(resolver, sites).naming(Naming::at_root(modules));
    let mut bound = Vec::new();
    let mut whole = true;
    for parameter in instance.bound() {
        let origin = lowering.origin(parameter.argument);
        match (lowering.ty(parameter.ty), lowering.expr(parameter.argument)) {
            (Some(ty), Some(value)) => bound.push(RawTopLevel {
                origin,
                name: Name::from(parameter.name),
                visibility: Visibility::Private,
                module: None,
                ty: Some(ty),
                value,
            }),
            _ => whole = false,
        }
    }
    whole.then_some(bound)
}

/// One lowered definition, as the member of a program `musa-core` reads.
///
/// The visibility comes from the node rather than from the [`Definition`], for
/// the reason `crate::lower::items` gives for leaving it off: a caller has the
/// declaration node in hand when it asks for the item, and a field answering a
/// question its own consumers could not pass on would be a field for nobody.
fn top_level(definition: Definition, visibility: Visibility) -> RawTopLevel {
    RawTopLevel {
        origin: definition.origin,
        name: definition.name,
        visibility,
        module: None,
        ty: definition.ty,
        value: definition.value,
    }
}

/// Tell the reference index that `node` declares `name`.
///
/// The index is what a rename, a reference list, and a go-to-definition are
/// answered from, and it is filled here because here is the one walk that sees
/// every declaration a document has: the reading itself writes a name through
/// for the core to resolve and never learns what it turned out to be
/// (`crate::lower`), so a walk that asked the reading would be asking the one
/// pass that deliberately does not know.
///
/// A motif and a fragment are declared by [`crate::resolve`] instead, under the
/// material kinds, which is why they are passed over here rather than filed
/// under a second kind: one name in two namespaces would make a rename check
/// the wrong collision.
fn record(resolver: &mut Resolver, node: &SyntaxNode, name: &str) {
    let kind = match node.kind() {
        SyntaxKind::FnDecl => crate::resolve::NameKind::Function,
        SyntaxKind::LetDecl | SyntaxKind::RecordDecl => crate::resolve::NameKind::Value,
        _ => return,
    };
    let span =
        crate::resolve::token_span(node, SyntaxKind::Identifier).unwrap_or_else(|| crate::resolve::trimmed_span(node));
    resolver.references.declare(kind, name, span);
}

/// Whether `private` was written on this declaration.
fn visibility_of(node: &SyntaxNode) -> Visibility {
    if node
        .children_with_tokens()
        .filter_map(musa_language::SyntaxElement::into_token)
        .any(|token| token.kind() == SyntaxKind::PrivateKw)
    {
        Visibility::Private
    } else {
        Visibility::Public
    }
}

/// The family groups, ordered so that each is declared after the families its
/// fields name.
///
/// The same analysis [`musa_core::declare_program`] runs over definitions and
/// for the same reason, one door over: a written order is not a dependency
/// order, and `data Chord { root: NoteName; }` may be written above the `data
/// NoteName` it needs.
///
/// # Why the edges come off the syntax and not off the [`RawData`]
///
/// A dependency here is a *type name*, and every type name a declaration writes
/// is an identifier token somewhere under its node — in a field, in an index, in
/// a type parameter's own bound. Reading the tokens finds all of them at once
/// and cannot fall behind a [`Raw`] shape added later; reading the term would
/// mean a second walk of nineteen shapes that has to stay in step with the core.
/// The reading over-approximates — a constructor's own name is a token too — and
/// that is safe in exactly one direction, which is the direction it errs: a
/// spurious edge is a spurious *cycle*, and a cycle is refused rather than
/// silently reordered.
///
/// A cycle between two declarations is refused here rather than in the core,
/// because the core's mutual-recursion door is one group with shared parameters
/// and two written `data` declarations share none.
fn order_families(resolver: &mut Resolver, families: Vec<(SyntaxNode, RawData)>) -> Option<Vec<(SyntaxNode, RawData)>> {
    let edges: Vec<BTreeSet<usize>> = families
        .iter()
        .map(|(node, _)| {
            let named = identifiers(node);
            families
                .iter()
                .enumerate()
                .filter(|&(_, (_, data))| data.families.iter().any(|family| named.contains(family.name.as_ref())))
                .map(|(index, _)| index)
                .collect()
        })
        .collect();
    let mut placed = BTreeSet::new();
    let mut walking = BTreeSet::new();
    let mut order = Vec::with_capacity(families.len());
    for start in 0..families.len() {
        if !visit(start, &edges, &mut walking, &mut placed, &mut order) {
            let names = families
                .iter()
                .enumerate()
                .filter(|(index, _)| walking.contains(index))
                .flat_map(|(_, (_, data))| data.families.iter().map(|family| format!("`{}`", family.name)))
                .collect::<Vec<_>>()
                .join(", ");
            resolver.report(
                Diagnostic::error(Code::DependencyCycle, format!("{names} name each other"))
                    .help("a `data` declaration may not depend on one that depends on it"),
            );
            return None;
        }
    }
    let mut held: Vec<Option<(SyntaxNode, RawData)>> = families.into_iter().map(Some).collect();
    Some(
        order
            .into_iter()
            .filter_map(|index| held.get_mut(index).and_then(Option::take))
            .collect(),
    )
}

/// Depth-first, answering `false` when `node` is on a cycle.
fn visit(
    node: usize,
    edges: &[BTreeSet<usize>],
    walking: &mut BTreeSet<usize>,
    placed: &mut BTreeSet<usize>,
    order: &mut Vec<usize>,
) -> bool {
    if placed.contains(&node) {
        return true;
    }
    if !walking.insert(node) {
        return false;
    }
    for &edge in edges.get(node).into_iter().flatten() {
        if edge != node && !visit(edge, edges, walking, placed, order) {
            return false;
        }
    }
    walking.remove(&node);
    placed.insert(node);
    order.push(node);
    true
}

/// Every identifier written anywhere under `node`.
fn identifiers(node: &SyntaxNode) -> BTreeSet<String> {
    node.descendants_with_tokens()
        .filter_map(musa_language::SyntaxElement::into_token)
        .filter(|token| token.kind() == SyntaxKind::Identifier)
        .map(|token| token.text().to_owned())
        .collect()
}
