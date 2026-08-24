//! A whole document, elaborated through `musa-calculus`.
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
//! # There is one door, and the order behind it is computed
//!
//! Every declaration a document writes — its `data` groups, its `record`s, its
//! definitions, and what its `impl` blocks leave behind — goes through
//! [`musa_calculus::declare_program`] in one call, and the order they are
//! elaborated in is the dependency order that call computes.
//!
//! It was three doors in a forced order until prompt 162ba, and the order was
//! wrong rather than merely coarse. Families first, "because a `data` field is a
//! type", is true and is only half of it: a field is a type, but an *index* is a
//! term. `02-core-calculus.md` §1 makes a family's index "any term of the
//! index's type — a call, a projection, a value the program computed", so
//! `data Vect<A>(n : Nat) { Cons(…) : (m + 1) }` names `Nat.add` and a walk that
//! had declared no definition yet could only report it as a name nobody wrote.
//! The dependency runs both ways, so neither kind can come first and the
//! question is not one an order fixed by kind can answer.
//!
//! Two things fall out of asking it once. The limit 141o recorded — "a family
//! whose field names a `record`", since `01-surface.md` §1.2 makes a record a
//! definition — is gone, because it was a symptom of the ordering rather than a
//! fact about the language. And a cycle is refused by
//! [`musa_calculus::Refusal::DefinitionCycle`] wherever it runs, rather than by
//! two cycle checks that could not see each other's half.
//!
//! # Why the context is rebuilt per document
//!
//! [`crate::registry::owned`] declares the prelude and registers every builtin
//! each time it is called. That is measurable, and it is prompt 165's to
//! measure: sharing one context means deciding what it does about a document
//! that declares a private family of its own, and deciding that before there is
//! a number would be guessing.

#[cfg(test)]
pub(crate) mod laws;

use std::sync::Arc;

use musa_calculus::{Cx, ElabError, ModuleId, Name, Origin, Program, Raw, RawProgram, RawTopLevel, Term, Visibility};
use musa_syntax::ast::AstNode as _;
use musa_syntax::{SyntaxKind, SyntaxNode};

use crate::elaborate::VoiceTrack;
use crate::lower::items::{Declared, Definition, Item};
use crate::lower::notation::{Argued, Claimed};
use crate::lower::{Lowering, Naming, Sites, refusals};
use crate::resolve::Resolver;
use musa_score::diagnose::{Code, Diagnostic};

/// One node whose children are declarations.
///
/// [`Clone`] because a corpus of sources is read once and elaborated many
/// times, each with a different file added to it: a [`SyntaxNode`] is a
/// refcounted handle, so the copy is a pointer and not a parse.
#[derive(Clone)]
pub(crate) struct Source {
    /// The node itself: a `library`, a document root, a `piece`, or a `voice`.
    pub(crate) root: SyntaxNode,
    /// The `import` that supplied it, when this document did not write it.
    ///
    /// Carried because a diagnostic about an imported declaration is stated at
    /// the `import` that pulled it in rather than at a span in a file this one
    /// is not: a span inside a foreign CST is meaningless here, so what a
    /// refusal needs is the statement.
    pub(crate) from: Option<Import>,
    /// Whether `02-core-calculus.md` §5.9's phase vocabulary is readable here.
    pub(crate) in_phase: bool,
    /// Declarations this reading leaves to somebody else. See [`Self::without`].
    left: Vec<String>,
}

/// The `import` statement one source arrived through.
///
/// [`crate::imports::Imported`] said the same three things while the loader
/// held the libraries; this is the owned copy, because a [`Source`] outlives
/// the borrow and a corpus is elaborated many times.
#[derive(Clone)]
pub(crate) struct Import {
    /// The file the text came from.
    pub(crate) path: String,
    /// What the importer wrote after `as`, on the rare import that wrote one.
    ///
    /// `01-surface.md` §1: "Importing two modules that export the same name is
    /// an error naming both; `import p::q as alias;` resolves it by qualifying
    /// that one, so an alias is required exactly at a real conflict and absent
    /// otherwise." So this is [`None`] on nearly every import, and where it is
    /// [`Some`] it is the whole of what the alias does — [`Read::gather`] files
    /// the library's definitions under `alias.name`, and
    /// [`crate::lower::Naming`] is how a use site reaches them.
    pub(crate) alias: Option<String>,
    /// The statement itself, which is the one span in this document that is
    /// about that library.
    pub(crate) at: musa_score::origin::SourceSpan,
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
            left: Vec::new(),
        }
    }

    /// A node the import `from` supplied.
    pub(crate) fn imported(root: &SyntaxNode, from: crate::imports::Imported<'_>) -> Self {
        Self {
            from: Some(Import {
                path: from.path.to_owned(),
                alias: from.qualifier.map(str::to_owned),
                at: from.at,
            }),
            ..Self::own(root)
        }
    }

    /// The same source, read where §5.9's phase vocabulary is nameable.
    pub(crate) fn in_phase(mut self) -> Self {
        self.in_phase = true;
        self
    }

    /// The same source, minus the declarations `left` names.
    ///
    /// One caller, and it is the expansion phase. An adapter's `print` is read
    /// where it is *run* rather than with its module — its parameter is the
    /// package's own type, which the phase has no name for — and a helper only
    /// `print` reaches goes with it ([`crate::phase::Printer`]). So the phase asks
    /// for the module without those lines.
    ///
    /// By name and not by rebuilding the node, because the node is where the
    /// author wrote it: a library re-spelled without those declarations would
    /// move every span after the first one, and a refusal about the module would
    /// then point into the wrong line of the adapter's own file.
    pub(crate) fn without(mut self, left: impl IntoIterator<Item = String>) -> Self {
        self.left.extend(left);
        self
    }

    /// Whether this reading was told to leave `node` to somebody else.
    fn leaves(&self, node: &SyntaxNode) -> bool {
        !self.left.is_empty() && bound_name(node).is_some_and(|name| self.left.contains(&name))
    }
}

/// A document's declarations, elaborated, and the context they are in.
///
/// Opaque on purpose, for [`musa_calculus::declare`]'s reason one level up: a
/// consumer asks this what a name *means* and never what the elaborator did to
/// find out.
pub(crate) struct Document {
    cx: Cx,
    sites: Sites,
    /// Kept so that the context this answers in is the context the definitions
    /// were elaborated into.
    ///
    /// [`audit`] reads it, and nothing else does: a caller asks this document
    /// what a name *means*, and the program is the elaborator's own record of
    /// how it found out. The kernel is the one reader entitled to that record,
    /// because re-deriving it is the whole of what `TRUST.md` claims.
    #[cfg_attr(not(test), expect(dead_code, reason = "the audit that reads it is a test build's"))]
    definitions: Arc<Program>,
    names: Vec<Name>,
    /// What elaborating the declarations charged.
    ///
    /// Kept because one caller has a budget of its own to answer for: the
    /// expansion phase charges `26-language-design-decision.md` §3.5's four
    /// counters, and reading an adapter *module* is the checking half of them.
    /// A number estimated on that side would be a second opinion about work the
    /// core already counted exactly, so the reading reports what it spent and
    /// the phase adds it up — see [`crate::phase::PhaseWork`].
    spend: musa_calculus::Spend,
}

impl Document {
    /// Every name this document bound, in the order it was written.
    pub(crate) fn names(&self) -> &[Name] {
        &self.names
    }

    /// What elaborating this document's declarations charged.
    ///
    /// The sum over the families, traits, definitions and instances it holds.
    /// Not the *piece*: a piece is read afterwards and through
    /// [`Document::piece`], so a caller that wants both adds two numbers rather
    /// than reading one that quietly means whichever happened first.
    pub(crate) const fn spend(&self) -> musa_calculus::Spend {
        self.spend
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
    /// Goes through [`musa_calculus::infer`] rather than reaching into the group,
    /// which is what makes this the same reading a source term gets: the term is
    /// elaborated, its type is inferred, and the value is normalized *at* that
    /// type, so η applies where the type says it should.
    ///
    /// # Errors
    ///
    /// [`ElabError`] when `raw` does not elaborate here, or when normalizing it
    /// exhausts the budget.
    pub(crate) fn term(&self, raw: &Raw) -> Result<(Term, Term), ElabError> {
        Ok(self.term_metered(raw)?.0)
    }

    /// The same, and what reading it charged.
    ///
    /// For [`crate::phase::AdapterModule`], which has a budget of its own:
    /// `26-language-design-decision.md` §3.5 gives the expansion phase four
    /// counters and two of them are this reading's work. A phase that estimated
    /// them would be keeping a second opinion about work the core already
    /// counted exactly.
    ///
    /// # Errors
    ///
    /// As [`Self::term`].
    pub(crate) fn term_metered(&self, raw: &Raw) -> Result<((Term, Term), musa_calculus::Spend), ElabError> {
        let ((term, ty), elaborating) = musa_calculus::infer_metered(&self.cx, raw)?;
        let (normal, normalizing) = musa_calculus::normalize_metered(&self.cx, &ty, &term)?;
        Ok(((normal, ty), elaborating.and(normalizing)))
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
    /// read them ([`musa_score::assert::Claim::reads_notes`]), so a bar pays for its
    /// duration and nothing else. They are relative to the passage, because
    /// that is what a claim is about: `assert voices(4)` counts what sounds
    /// together inside the braces, and where the braces stand is the measure
    /// claim's business.
    ///
    /// # Errors
    ///
    /// [`ElabError`] for [`Self::track`]'s reasons, on either of the two terms
    /// or on any argument, and [`musa_calculus::Malformed::NotALiteral`] when an
    /// argument's normal form does not hold what its shape declares — which is
    /// this crate's defect rather than a program's, since every argument was
    /// checked at that shape's own type.
    pub(crate) fn passage(
        &self,
        claimed: &Claimed,
    ) -> Result<(musa_score::assert::Claim, musa_score::assert::Passage), ElabError> {
        let claim = self.claim(claimed)?;
        let before = self.track(&claimed.before)?;
        let sounding = self.track(&claimed.passage)?;
        let notes = if claim.reads_notes() {
            sounding
                .occurrences()
                .iter()
                .filter_map(|occurrence| {
                    Some(musa_score::assert::Sounded {
                        pitch: occurrence.payload().pitch_of()?,
                        start: musa_score::MusicalTime::new(occurrence.span().start().as_ratio()),
                        end: musa_score::MusicalTime::new(occurrence.span().end().as_ratio()),
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
            musa_score::assert::Passage {
                span: claimed.span,
                at: musa_score::MusicalTime::new(before.duration().as_ratio()),
                extent: musa_score::MusicalDuration::new(sounding.duration().as_ratio()),
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
    fn claim(&self, claimed: &Claimed) -> Result<musa_score::assert::Claim, ElabError> {
        // One sentence for both failures below, because they are one defect:
        // an argument that was checked at its shape's type and does not read
        // back as that shape means this crate's registration and its reading
        // disagree, which no program can cause and no diagnostic can repair.
        let broken = || ElabError::from(musa_calculus::Malformed::NotALiteral(claimed.predicate.name.into()));
        let mut arguments = Vec::with_capacity(claimed.arguments.len());
        for (argued, shape) in claimed.arguments.iter().zip(claimed.predicate.parameters) {
            arguments.push(match *argued {
                Argued::Word(ref word) => word.clone(),
                Argued::Value(ref raw) => {
                    let (normal, _) = self.term(raw)?;
                    crate::registry::argument(&self.cx, *shape, &normal).ok_or_else(broken)?
                }
            });
        }
        musa_score::assert::Claim::build(claimed.predicate.name, arguments).ok_or_else(broken)
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
    pub(crate) fn machines(&self) -> Vec<(String, musa_score::MachineSpec)> {
        self.names
            .iter()
            .filter_map(|name| {
                let (term, ty) = musa_calculus::infer(&self.cx, &Raw::var(Origin::UNKNOWN, &**name)).ok()?;
                let (step, input, output) = crate::registry::machine_ports(&ty)?;
                let normal = musa_calculus::normalize(&self.cx, &ty, &term).ok()?;
                let nodes = crate::registry::machine_nodes(&self.cx, &normal)?;
                Some((
                    name.to_string(),
                    musa_score::MachineSpec::new(step, input, output, nodes),
                ))
            })
            .collect()
    }

    /// The context this document's terms were elaborated in.
    ///
    /// Exposed because a term names what it means and the context decides it
    /// (`02-core-calculus.md` §6): a caller reading a normal form back as data
    /// has to bring the table the term was written under.
    pub(crate) const fn cx(&self) -> &musa_calculus::Cx {
        &self.cx
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
    pub(crate) fn piece(&mut self, resolver: &mut Resolver, node: &SyntaxNode) -> Option<crate::lower::piece::Piece> {
        Lowering::new(resolver, &mut self.sites).piece(node)
    }
}

/// Elaborate every declaration `sources` writes, in one context.
///
/// [`None`] with diagnostics reported when anything was refused. Every refusal
/// is restated at the node it is about, so a caller neither sees an
/// [`ElabError`] nor has to know what an [`Origin`] is.
/// Every definition this document elaborated, read again by the kernel.
///
/// `musa-calculus`'s [`recheck_program`](musa_calculus::recheck_program) closed
/// over the whole core in prompt 158, and this is where the standard library and
/// `examples/` meet it: the crate that owns the pass is a leaf with no parser,
/// so the corpus it most wants to be stated over can only be read here.
///
/// Loud rather than a diagnostic, and only in test builds. Loud because a
/// disagreement is *this crate's* defect and there is no repair an author could
/// make — `TRUST.md` says elaboration is untrusted precisely so that a term it
/// built and the kernel rejects is a bug report about us. Test builds only
/// because it re-derives every definition a second time, and the release
/// compiler already pays for the per-declaration audit
/// `elaboration::declare_program` runs behind `debug_assertions`.
#[cfg(test)]
pub(crate) fn audit(document: &Document) {
    if let Err(fault) = musa_calculus::recheck_program(&document.cx, &document.definitions) {
        #[expect(clippy::panic, reason = "a gate that cannot fail loudly is not a gate")]
        {
            panic!("the kernel rejects what elaboration built for this document: {fault}");
        }
    }
}

/// The module number each file this document reads from was given.
///
/// `01-surface.md` §1.3 scopes `private` to "the module that declares it", and a
/// Musa module is a *file*: `mod tonal;` names `tonal.musa`. So the unit here is
/// the file, and the numbers are private to one elaboration — nothing persists
/// one, nothing compares two documents' numbers, and no hash, snapshot, or
/// export ever sees one. [`crate::prelude::IMPORTED`] states where they start
/// and which three are reserved.
///
/// # Why the key is the file and not the [`Source`]
///
/// A document is walked as several sources of which two are one file: the
/// lexical root and the `piece` node inside it are both written in the document
/// being compiled — see [`crate::elaborate`], which pushes `Source::own(root)`
/// and `Source::own(piece)` — and a number keyed on the source would make a
/// voice unable to name a `private` declaration three lines above the piece it
/// is in. Both take [`crate::prelude::SOURCE`], which is also where the context
/// stands, because the file being compiled is the one viewer.
#[derive(Default)]
struct Files {
    /// Each imported path beside its number, in the order they were read.
    ///
    /// A list rather than a map: a document imports a handful of files, and the
    /// same path asked for twice has to answer the same number — two `Source`s
    /// of one library are one module however they were reached.
    imported: Vec<(String, ModuleId)>,
}

impl Files {
    /// The module `source` was written in, minting a number for a file this
    /// walk has not seen before.
    fn of(&mut self, sites: &mut Sites, source: &Source) -> ModuleId {
        let Some(import) = &source.from else {
            return crate::prelude::SOURCE;
        };
        if let Some((_, module)) = self.imported.iter().find(|(path, _)| *path == import.path) {
            return *module;
        }
        // Saturating rather than wrapping to [`crate::prelude::SOURCE`], on a
        // count no document reaches: a file that shared the home file's number
        // would be *more* visible than it should be, and a hiding rule may only
        // ever fail closed.
        let next = ModuleId::new(
            u32::try_from(self.imported.len())
                .ok()
                .and_then(|read| read.checked_add(crate::prelude::IMPORTED))
                .unwrap_or(u32::MAX),
        );
        self.imported.push((import.path.clone(), next));
        // So a refusal can say which file the name is maintained in: the core
        // carries the number and only this side knows what it stands for.
        sites.in_file(next, &import.path);
        next
    }
}

pub(crate) fn elaborate(resolver: &mut Resolver, sources: &[Source]) -> Option<Document> {
    let mut sites = Sites::default();
    let aliases: Vec<String> = sources
        .iter()
        .filter_map(|source| source.from.as_ref()?.alias.clone())
        .collect();
    let mut read = Read::default();
    // The names this file wrote itself, picked out as they are read: every
    // source's declarations go into one flat list, and [`named_once`] is about
    // the ones a span in *this* document can point at. See its doc for why the
    // imported ones are a different question.
    let mut here: Vec<(Name, Origin)> = Vec::new();
    // And the names a *flat* import brought in, beside the statement that
    // brought them, which is what [`imports_agree`] compares. An aliased import
    // is not on the list because it cannot collide: its names all begin with a
    // qualifier `as` took out of circulation.
    let mut brought: Vec<(Name, &Import)> = Vec::new();
    let mut files = Files::default();
    for source in sources {
        let already = read.definitions.len();
        let numbered = sites.counted();
        let module = files.of(&mut sites, source);
        read.gather(resolver, &mut sites, source, module, &aliases);
        read.barred(resolver, &mut sites, source, module);
        // Which file this source's sites came out of, recorded now because this
        // is the walk that knows. A refusal about one of them is restated at the
        // `import` below rather than at a span in a document nobody here has —
        // see `refusals::restate`.
        if let Some(import) = &source.from {
            sites.imported(numbered, &import.path, import.at);
        }
        let named = read.definitions.iter().skip(already);
        match &source.from {
            None => here.extend(named.map(|held| (Arc::clone(&held.name), held.origin))),
            Some(import) if import.alias.is_none() => brought.extend(
                named
                    .filter(|held| held.visibility == Visibility::Public)
                    .map(|held| (Arc::clone(&held.name), import)),
            ),
            Some(_) => {}
        }
    }
    // Here rather than beside the core's own refusals below, because a
    // declaration that could not be read leaves the *list* short: every use of
    // the missing name would be checked against a context that never bound it
    // and reported as an unknown name, burying the refusal already stated at
    // the declaration.
    if read.refused {
        return None;
    }
    // Standing *somewhere*, which is what makes the phase's `private` bite: the
    // compiler's own context stands nowhere and is inside every module, and a
    // document elaborated in it could mint the sealed step. See
    // [`crate::prelude::SOURCE`].
    let mut cx = match crate::registry::owned() {
        Ok(cx) => cx.in_module(crate::prelude::SOURCE),
        Err(error) => {
            resolver.report(refusals::restate(&sites, &error));
            return None;
        }
    };
    // `05-adapters.md` §5.9 keeps the phase's vocabulary and the piece's apart,
    // so `run_syntax_step` is in scope for a reading that holds a phase source
    // and unknown everywhere else. It is a definition rather than a builtin
    // because a projection out of a declared family is what the language
    // already writes — see [`crate::prelude::expansion`].
    if sources.iter().any(|source| source.in_phase) {
        match crate::prelude::expansion(&cx) {
            Ok(widened) => cx = widened,
            Err(error) => {
                resolver.report(refusals::restate(&sites, &error));
                return None;
            }
        }
    }
    let mut spend = musa_calculus::Spend::default();
    let names: Vec<Name> = read.definitions.iter().map(|held| Arc::clone(&held.name)).collect();
    let mut refused = !named_once(resolver, &sites, &here);
    refused |= !imports_agree(resolver, &brought);
    let program = RawProgram {
        families: read.families,
        definitions: read.definitions,
    };
    let declared = match musa_calculus::declare_program_metered(&cx, &program) {
        Ok((declared, spent)) => {
            spend = spend.and(spent);
            declared
        }
        Err(error) => {
            resolver.report(refusals::restate(&sites, &error));
            return None;
        }
    };
    cx = cx.defining(&declared);
    documented(resolver, &cx, read.declaring);
    if refused {
        return None;
    }
    Some(Document {
        cx,
        sites,
        definitions: declared,
        names,
        spend,
    })
}

/// That no two of `written` bind one name, reporting each repeat.
///
/// The document is the scope, so this is the one place the whole list exists at
/// once and the only place the question can be asked. `declare_program` cannot
/// ask it: a definition is a global name rather than a binder, and the core's
/// duplicate refusals — `DuplicateField`, `DuplicateCase`, `DuplicateMethod`,
/// `DuplicateInstance` — are each about a *declaration's own* parts, where the
/// second entry has a first entry to be compared against. Handing it two
/// definitions of `value` would leave it deciding which document meant which,
/// which is a question about sources it does not have.
///
/// Returns whether the list was clean, and reports every repeat rather than the
/// first: two names written twice are two mistakes, and an author who fixed one
/// and recompiled to find the other would be paying for this function's
/// convenience.
///
/// # Why an import's names are not in this list
///
/// They *are* in the same flat namespace — `01-surface.md` §1 puts imported
/// definitions there deliberately, so a score writes `numeral_chord(home, five)`
/// and not a qualified path — so a piece that writes `let repeated = …` beside
/// `import std::list;` really has two things called `repeated`. But that is a
/// different sentence with a different repair. §1 says two imports exporting one
/// name is "an error naming both", resolved by `import p::q as alias;`, and both
/// halves of that need what this function does not have: the *paths*. A span
/// inside a foreign CST means nothing against this document's text — see
/// [`Source::from`], which carries the path for exactly that reason — so the
/// second label of an import collision cannot be a span at all.
///
/// So an import's names are [`imports_agree`]'s to compare, and what is checked
/// here is what this document's own text can be pointed at for: two
/// declarations a reader can see at once. The remaining case — a local name
/// that collides with an imported one — is neither function's, because §1 does
/// not say what it means. Two imports colliding is stated there and `as` is its
/// repair; a local name shadowing an import is a question the surface
/// specification has not answered, and answering it here would be inventing a
/// rule rather than enforcing one.
fn named_once(resolver: &mut Resolver, sites: &Sites, written: &[(Name, Origin)]) -> bool {
    let mut seen: Vec<&(Name, Origin)> = Vec::with_capacity(written.len());
    let mut clean = true;
    for held in written {
        let (name, at) = (&held.0, held.1);
        if let Some(&&(_, previous)) = seen.iter().find(|(taken, _)| *taken == *name) {
            clean = false;
            let said = Diagnostic::error(
                Code::DuplicateName,
                format!("`{name}` is declared twice in this document"),
            );
            let said = match sites.span(at) {
                Some(span) => said.at(span, "declared again here"),
                None => said,
            };
            resolver.report(
                said.maybe_also(sites.span(previous), "first declared here")
                    .help("give one of them another name")
                    .note("musa has no shadowing: a name means one thing everywhere the piece can see it"),
            );
        } else {
            seen.push(held);
        }
    }
    clean
}

/// That no two flat imports export one name, reporting each clash.
///
/// `01-surface.md` §1, exactly: "Importing two modules that export the same
/// name is an error naming both; `import p::q as alias;` resolves it by
/// qualifying that one, so an alias is required exactly at a real conflict and
/// absent otherwise." So the message names both files, the help offers `as`,
/// and both labels are `import` statements — the two spans in this document
/// that are about those files, and the two lines the author will edit.
///
/// Not the same question as [`named_once`], and worth its own function for the
/// reason its message shows: a repeat inside one document is repaired by
/// renaming a declaration, and a clash between two imports is repaired by
/// qualifying one of the *statements*. Neither party did anything wrong, which
/// is why the second label reads "and here" rather than "first declared here".
///
/// Only the public names, and only the unaliased imports. A `private`
/// declaration in a library is not exported, so two libraries that each keep a
/// helper called `step` are nothing to each other; and an aliased import's
/// names all begin with a qualifier `as` took out of circulation, so it has
/// already answered the question this asks.
fn imports_agree(resolver: &mut Resolver, brought: &[(Name, &Import)]) -> bool {
    let mut seen: Vec<&(Name, &Import)> = Vec::with_capacity(brought.len());
    let mut clean = true;
    for held in brought {
        let (name, from) = (&held.0, held.1);
        match seen.iter().find(|(taken, _)| *taken == *name) {
            // The same file reached twice is one import, not two: a library
            // imported by two others is loaded once and its names arrive once.
            Some((_, first)) if first.path == from.path => {}
            Some((_, first)) => {
                clean = false;
                resolver.report(
                    Diagnostic::error(
                        Code::DuplicateName,
                        format!("`{}` and `{}` both declare `{name}`", first.path, from.path),
                    )
                    .at(from.at, "the second of two imports declaring it")
                    .also(first.at, "and here")
                    .help(format!(
                        "qualify one of them: `as` binds it under a name of its own, so `{name}` means one thing again"
                    )),
                );
            }
            None => seen.push(held),
        }
    }
    clean
}

/// What one walk of a document's declarations collected, by which door each
/// goes through.
#[derive(Default)]
struct Read {
    /// Each `data` group, beside the module it was written in.
    families: Vec<musa_calculus::RawGroup>,
    /// Every definition, `record`s among them: a record declares a type and is
    /// written as a definition (`01-surface.md` §1.2), and there is no longer a
    /// door it has to be sorted into before elaboration begins.
    definitions: Vec<RawTopLevel>,
    /// Each definition's declaration beside the name it bound, held until the
    /// document is elaborated. See [`documented`].
    declaring: Vec<Declaring>,
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
    ///
    /// An `import … as alias;` files what it supplies under `alias.name`, which
    /// is the whole of what an alias does. It reaches only the *definitions*: a
    /// `data`, a `trait`, and an `impl` are still declared flat, because
    /// `01-surface.md` §1 gives the alias one job — resolving a collision
    /// between two modules exporting the same name — and the collision the same
    /// section reports is between names in "the current **flat** value
    /// namespace". Qualifying a type as well would be a namespace rule nothing
    /// asks for and no source writes.
    fn gather(
        &mut self,
        resolver: &mut Resolver,
        sites: &mut Sites,
        source: &Source,
        module: ModuleId,
        aliases: &[String],
    ) {
        let alias = source.from.as_ref().and_then(|from| from.alias.as_deref());
        for node in source.root.children() {
            if source.leaves(&node) {
                continue;
            }
            let visibility = visibility_of(&node);
            let item = {
                let lowering = if source.in_phase {
                    Lowering::phase(resolver, sites)
                } else {
                    Lowering::new(resolver, sites)
                };
                let lowering = if source.from.is_some() {
                    lowering.elsewhere()
                } else {
                    lowering
                };
                lowering.naming(Naming::under(aliases)).item(&node)
            };
            match item {
                Declared::Item(Item::Data(data)) => {
                    for family in &data.families {
                        record(
                            resolver,
                            &node,
                            &family.name,
                            source.from.as_ref().map(|from| from.path.as_str()),
                        );
                        self.declaring
                            .push(Declaring::named(&node, &family.name, data.origin, source));
                    }
                    self.families.push(musa_calculus::RawGroup {
                        data,
                        module: Some(module),
                    });
                }
                // An `impl`'s definitions are filed flat and never under an
                // alias, for the reason the paragraph above gives: the block
                // names a *type's* namespace, which the importing document does
                // not get to rename.
                Declared::Item(Item::Namespace(members)) => {
                    for (written, definition) in members {
                        record(
                            resolver,
                            &written,
                            &definition.name,
                            source.from.as_ref().map(|from| from.path.as_str()),
                        );
                        self.declaring.push(Declaring::at(&written, &definition, source));
                        self.definitions.push(top_level(definition, visibility, module));
                    }
                }
                Declared::Item(Item::Definition(mut definition)) => {
                    if let Some(alias) = alias {
                        definition.name = Name::from(format!("{alias}{}{}", crate::lower::DOT, definition.name));
                    }
                    record(
                        resolver,
                        &node,
                        &definition.name,
                        source.from.as_ref().map(|from| from.path.as_str()),
                    );
                    self.declaring.push(Declaring::at(&node, &definition, source));
                    self.definitions.push(top_level(definition, visibility, module));
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
    /// rather than inside it, because a walk of a source's children finds the
    /// `score`, not the bars three levels under it.
    ///
    /// Only from a `piece`. A document root has the piece among its children and
    /// the piece is a source of its own, so a descendant walk from both would
    /// declare every named bar twice and refuse the document for a collision it
    /// invented.
    ///
    /// An anonymous bar declares nothing and is passed over: it is a measure,
    /// which is ordinary structure, and giving it a name here would be inventing
    /// one the source did not write.
    fn barred(&mut self, resolver: &mut Resolver, sites: &mut Sites, source: &Source, module: ModuleId) {
        if source.root.kind() != SyntaxKind::PieceDecl {
            return;
        }
        for node in source.root.descendants() {
            let Some(name) = musa_syntax::ast::BarStmt::cast(node.clone()).and_then(|bar| bar.name()) else {
                continue;
            };
            let read = Lowering::new(resolver, sites).named_bar(&node, &name);
            match read {
                // Private because a score is not an interface: a bar is written
                // inside a voice, and no `import` reaches in there to name it.
                Some(definition) => {
                    self.declaring.push(Declaring::at(&node, &definition, source));
                    self.definitions
                        .push(top_level(definition, Visibility::Private, module));
                }
                None => self.refused = true,
            }
        }
    }
}

/// One lowered definition, as the member of a program `musa-calculus` reads.
///
/// The visibility comes from the node rather than from the [`Definition`], for
/// the reason `crate::lower::items` gives for leaving it off: a caller has the
/// declaration node in hand when it asks for the item, and a field answering a
/// question its own consumers could not pass on would be a field for nobody.
/// The module comes from the *walk*, for the mirrored reason: a definition does
/// not know which file it was read out of and the walk over the sources does.
///
/// `01-surface.md` §1.3: "A marked declaration is nameable from a sibling
/// definition in its own module and from nowhere else." The module written here
/// is the whole of what makes that sentence true — a [`RawTopLevel`] carrying
/// [`None`] is `visibility.rs`'s "written nowhere in particular", which hides
/// from nobody, and that is what every source definition was until 162a.
fn top_level(definition: Definition, visibility: Visibility, module: ModuleId) -> RawTopLevel {
    RawTopLevel {
        origin: definition.origin,
        name: definition.name,
        visibility,
        module: Some(module),
        ty: definition.ty,
        value: definition.value,
    }
}

/// One declaration, held from the walk that read it until the document can say
/// what it means.
///
/// [`crate::lower::documented`] states what a reader is told, and every field it
/// needs but the type is a property of the *declaration* — the word, the name,
/// the parameters, the comment above it, where it is written. Only the type of a
/// declaration that wrote none has to wait for elaboration, so the record is
/// built once, afterwards, rather than built early and patched.
struct Declaring {
    node: SyntaxNode,
    /// The name the flat namespace holds it by, which for a module member is
    /// `CMajor.tonic` and not the `tonic` the declaration wrote.
    name: Name,
    origin: Origin,
    /// The document it is written in, `None` for the one being compiled.
    uri: Option<String>,
}

impl Declaring {
    /// One declaration, as this walk found it in `source`.
    fn at(node: &SyntaxNode, definition: &Definition, source: &Source) -> Self {
        Self::named(node, &definition.name, definition.origin, source)
    }

    /// The same, for a declaration that names a *type* rather than a value.
    ///
    /// A `record` is a one-constructor family after prompt 157, so what it
    /// declares no longer arrives here as a [`Definition`] — and a reader who
    /// hovers `Group` is asking the question they always were. The type comes
    /// off the family constant by the same inference every other declaration's
    /// does, so the record a reader is shown is the one they were shown before.
    fn named(node: &SyntaxNode, name: &Name, origin: Origin, source: &Source) -> Self {
        Self {
            node: node.clone(),
            name: Arc::clone(name),
            origin,
            uri: source.from.as_ref().map(|from| from.path.clone()),
        }
    }
}

/// Tell the editor what each declaration means, now that the document knows.
///
/// The type of a declaration that wrote none is asked for by *inferring its own
/// name*: `cx` holds every definition this document declared, so the type a use
/// of the name would have is the type the declaration ended up with. Nothing is
/// re-elaborated — the name is one node, and what comes back is what
/// [`musa_calculus::declare_program`] already settled.
///
/// A name that will not infer is passed over silently. It cannot be a name this
/// document failed to declare — the walk only holds declarations the core
/// accepted — so a refusal here would be about a document that is already being
/// refused for the reason that produced it.
fn documented(resolver: &mut Resolver, cx: &Cx, declaring: Vec<Declaring>) {
    for held in declaring {
        let inferred = || {
            let (_, ty) = musa_calculus::infer(cx, &Raw::var(held.origin, &*held.name)).ok()?;
            crate::lower::documented::spelled(&ty)
        };
        if let Some(item) = crate::lower::documented::documented(&held.node, &held.name, held.uri.as_deref(), inferred)
        {
            resolver.references.document(item);
        }
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
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "the dispatch is over `SyntaxKind`'s three hundred variants, of which three carry a name an editor indexes; writing the other variants out would hide the reading rather than check it"
)]
fn record(resolver: &mut Resolver, node: &SyntaxNode, name: &str, source: Option<&str>) {
    let kind = match node.kind() {
        SyntaxKind::FnDecl => crate::resolve::NameKind::Function,
        SyntaxKind::LetDecl | SyntaxKind::RecordDecl => crate::resolve::NameKind::Value,
        _ => return,
    };
    let span =
        crate::resolve::token_span(node, SyntaxKind::Identifier).unwrap_or_else(|| crate::resolve::trimmed_span(node));
    match source {
        None => resolver.references.declare(kind, name, span),
        // The span belongs to the imported document's text, which nothing in
        // this document's coordinates can point at: the name is recorded as
        // spelled elsewhere, and go-to-definition opens *that* URI.
        Some(uri) => resolver.references.declare_external(kind, name, uri, span),
    }
}

/// The name a declaration is written under, before anything lowers it.
///
/// Its *first* name and not everything it binds: a `data` binds its
/// constructors too, and [`Source::without`]'s one caller excludes whole
/// declarations by the name at the head of each. Reading it off the CST rather
/// than off a lowered item is what lets a source leave a declaration out
/// *before* it is lowered, which is the point — the phase cannot lower `print`
/// at all.
fn bound_name(node: &SyntaxNode) -> Option<String> {
    node.children_with_tokens()
        .filter_map(musa_syntax::SyntaxElement::into_token)
        .find(|token| token.kind() == SyntaxKind::Identifier)
        .map(|token| token.text().to_owned())
}

/// Whether `private` was written on this declaration.
fn visibility_of(node: &SyntaxNode) -> Visibility {
    if node
        .children_with_tokens()
        .filter_map(musa_syntax::SyntaxElement::into_token)
        .any(|token| token.kind() == SyntaxKind::PrivateKw)
    {
        Visibility::Private
    } else {
        Visibility::Public
    }
}
