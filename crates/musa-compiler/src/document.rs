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
//! # The order the four doors are opened in
//!
//! `musa-core` has four: [`musa_core::declare`] for a family group,
//! [`musa_core::declare_trait`], [`musa_core::declare_impl`], and
//! [`musa_core::declare_program`] for the definitions. They are not
//! interchangeable and the order between them is forced:
//!
//! 1. **Families first**, ordered among themselves, because a `data` field is a
//!    type and a type is a family or a base type.
//! 2. **Traits**, whose method types may name any family.
//! 3. **Definitions**, in one group, because §2.4's forward reference is a
//!    property of the group rather than of a written order — which is the whole
//!    of what [`musa_core::declare_program`] is for.
//! 4. **Instances last**, because an `impl`'s method bodies are ordinary terms
//!    and may call any definition — the one edge that would otherwise run
//!    backwards.
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

use musa_core::{Cx, Definitions, ElabError, Name, Origin, Raw, RawData, RawProgram, RawTopLevel, Term, Visibility};
use musa_language::{SyntaxKind, SyntaxNode};

use crate::diagnose::{Code, Diagnostic};
use crate::lower::items::{Definition, Item};
use crate::lower::{Lowering, Sites, refusals};
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
    /// Whether `02-core-calculus.md` §5.9's phase vocabulary is readable here.
    pub(crate) in_phase: bool,
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
    _definitions: Arc<Definitions>,
    names: Vec<Name>,
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
pub(crate) fn elaborate(resolver: &mut Resolver, sources: &[Source]) -> Option<Document> {
    let mut sites = Sites::default();
    let mut read = Read::default();
    for source in sources {
        read.gather(resolver, &mut sites, source);
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
    };
    let definitions = match musa_core::declare_program(&cx, &program) {
        Ok(definitions) => definitions,
        Err(error) => {
            resolver.report(refusals::restate(&sites, &error));
            return None;
        }
    };
    cx = cx.defining(&definitions);
    for raw in &read.instances {
        match musa_core::declare_impl(&cx, raw) {
            Ok(declared) => cx = cx.declaring_instance(&declared),
            Err(error) => {
                resolver.report(refusals::restate(&sites, &error));
                refused = true;
            }
        }
    }
    if refused {
        return None;
    }
    Some(Document {
        cx,
        sites,
        _definitions: definitions,
        names,
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
}

impl Read {
    /// Every declaration written directly inside `source`, lowered and filed.
    ///
    /// Directly inside, and not every descendant: a `fn` written inside a
    /// `piece` is the piece's and is gathered when the piece is a source of its
    /// own, which is what keeps a template's body out of the root's scope.
    fn gather(&mut self, resolver: &mut Resolver, sites: &mut Sites, source: &Source) {
        for node in source.root.children() {
            let visibility = visibility_of(&node);
            let item = {
                let mut lowering = if source.in_phase {
                    Lowering::phase(resolver, sites)
                } else {
                    Lowering::new(resolver, sites)
                };
                lowering.item(&node)
            };
            match item {
                Some(Item::Data(data)) => self.families.push((node, data)),
                Some(Item::Class(class)) => self.classes.push(class),
                Some(Item::Instance(instance)) => self.instances.push(instance),
                Some(Item::Definition(definition)) => self.definitions.push(top_level(definition, visibility)),
                None => {}
            }
        }
    }
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
