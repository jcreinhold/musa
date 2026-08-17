//! Declaration templates: families of pieces and voices, expanded before any
//! score context exists (`docs/rules/language/04-templates-and-modules.md` §1–§2).
//!
//! A template is not a value. Nothing can pass one, return one, store one, or
//! ask what is inside it — the only thing that may be done with a template is
//! write `make N(…) as I;` where the declaration it makes belongs. That is
//! why expansion lives here rather than in the value core: the core's job
//! ends at values, and a piece is not one.
//!
//! Expansion is a *binding*, never a rewrite. The arguments at a site are
//! evaluated in the scope the site stands in, and the template's body is then
//! checked with its parameters bound to those values. No syntax is copied, no
//! span is moved, and no name can be captured, so what an editor points at
//! inside a template body is the text the author wrote, once, for every
//! instance of it.
//!
//! Identity is generative and comes from the *site*, not the arguments
//! (§2). Editing what `study_in_g` is given changes what it contains; it does
//! not make it a different piece. Two sites with equal arguments stay two
//! declarations.

use indexmap::IndexMap;
use musa_language::SyntaxNode;
use musa_language::ast::{AstNode as _, MakeStmt, PieceDecl, TemplateDecl, VoiceDecl};

use crate::core::{Binding, Program};
use crate::diagnose::{Code, Diagnostic};
use crate::origin::{ExpansionStep, SourceSpan};
use crate::resolve::{Resolver, trimmed_span};

/// The digest version. Bumping it changes every generated identity on
/// purpose, which is why it is written down rather than implied by the code
/// that happens to compute one.
pub(crate) const DIGEST_VERSION: &str = "musa-template-1";

/// The separator inside a generated key. A unit separator, because no part of
/// a key — a document name, a template name, a structural path — can contain
/// one, so no two different keys can encode to the same bytes.
pub(crate) const UNIT: char = '\u{1f}';

/// What a template parameterizes.
///
/// Two kinds, and no more, because these are the two this prompt implements:
/// a part, a performance block, or a studio graph would each need its own
/// placement rule, and inventing them ahead of a caller would be inventing
/// the rule too.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    /// `template piece …` — stands at a document's root and is its piece.
    Piece,
    /// `template voice …` — stands among a part's voices.
    Voice,
}

impl Kind {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Piece => "piece",
            Self::Voice => "voice",
        }
    }
}

/// One template declaration, with everything an instance site needs from it.
struct Entry {
    kind: Kind,
    /// The declaration the template parameterizes: a `PieceDecl` or a
    /// `VoiceDecl` node, indistinguishable from a written-out one apart from
    /// the parameter list.
    declaration: SyntaxNode,
    parameters: Vec<Parameter>,
    /// The template's own name, for the "declared here" half of a
    /// diagnostic.
    span: SourceSpan,
}

#[derive(Clone)]
struct Parameter {
    name: String,
    name_span: SourceSpan,
    span: SourceSpan,
    ty: SyntaxNode,
}

/// The templates a document declares, by the name sites call them.
pub(crate) struct Templates {
    by_name: IndexMap<String, Entry>,
    /// Generated identities already handed out, and the key each came from.
    /// A digest that repeats for a different key is refused rather than
    /// silently identifying two declarations (§2).
    assigned: IndexMap<u128, String>,
}

impl Templates {
    /// Collect the templates written at a document's lexical root.
    ///
    /// Declaring one twice is an error here rather than at the site that
    /// calls it, because the second declaration is the mistake and the site
    /// is innocent.
    pub(crate) fn collect(resolver: &mut Resolver, root: &SyntaxNode) -> Self {
        let mut by_name: IndexMap<String, Entry> = IndexMap::new();
        for template in TemplateDecl::all_at_root(root) {
            let (kind, declaration) = if let Some(piece) = template.piece() {
                (Kind::Piece, piece.syntax().clone())
            } else if let Some(voice) = template.voice() {
                (Kind::Voice, voice.syntax().clone())
            } else {
                continue;
            };
            let span = trimmed_span(template.syntax());
            let Some(name) = template.name() else { continue };
            let parameters = collect_parameters(resolver, &template, &name);
            if let Some(first) = by_name.get(&name) {
                resolver.report(
                    Diagnostic::error(Code::DuplicateName, format!("`{name}` is declared twice as a template"))
                        .at(span, "declared again here")
                        .also(first.span, "first declared here")
                        .help("give one of them a different name"),
                );
                continue;
            }
            by_name.insert(
                name,
                Entry {
                    kind,
                    declaration,
                    parameters,
                    span,
                },
            );
        }
        Self {
            by_name,
            assigned: IndexMap::new(),
        }
    }

    /// Resolve a `make` site to the template it names, checked against the
    /// kind of declaration the place it stands in requires.
    ///
    /// `enclosing` is the template whose body the site is written in, when it
    /// is written in one: a site that names it would be a template making
    /// itself, and that is the cycle the static dependency graph forbids.
    pub(crate) fn instance(
        &mut self,
        resolver: &mut Resolver,
        stmt: &MakeStmt,
        path: String,
        want: Kind,
        enclosing: Option<&str>,
        namespace: &str,
    ) -> Option<Instance> {
        let span = trimmed_span(stmt.syntax());
        let name = stmt.template()?;
        let alias = stmt.alias().unwrap_or_default();
        if enclosing == Some(name.as_str()) {
            resolver.report(
                Diagnostic::error(Code::DependencyCycle, format!("template `{name}` makes itself"))
                    .at(span, format!("this instance is inside `{name}`'s own body"))
                    .help("a template's dependencies must be finite: nothing it makes may be itself")
                    .note("expansion happens once, before any music exists, so a self-instance has no base case"),
            );
            return None;
        }
        let Some(entry) = self.by_name.get(&name) else {
            resolver.report(
                Diagnostic::error(Code::UnknownName, format!("no template called `{name}`"))
                    .at(span, "not a template this file declares")
                    .help("templates are declared at the top of the file, before its piece"),
            );
            return None;
        };
        if entry.kind != want {
            resolver.report(
                Diagnostic::error(
                    Code::Misplaced,
                    format!(
                        "`{name}` is a {} template, and a {} belongs here",
                        entry.kind.as_str(),
                        want.as_str()
                    ),
                )
                .at(span, format!("this place makes a {}", want.as_str()))
                .also(entry.span, format!("declared as a {} template", entry.kind.as_str()))
                .help("a piece instance stands at the file's root; a voice instance stands among a part's voices"),
            );
            return None;
        }
        let arguments = arguments_of(resolver, stmt, entry, &name)?;
        let declaration = entry.declaration.clone();
        let parameters = entry.parameters.clone();
        // Identity is settled here, once, where the site is resolved: an
        // instance that exists has an identity, and one that failed to
        // resolve never gets to claim a digest.
        let key = format!(
            "{DIGEST_VERSION}{UNIT}{namespace}{UNIT}{} {name}{UNIT}{path}{UNIT}",
            entry.kind.as_str(),
        );
        let identity = self.claim(resolver, key, span)?;
        Some(Instance {
            name: name.clone(),
            declaration,
            parameters,
            arguments,
            step: ExpansionStep::TemplateInstance {
                template: name,
                alias: alias.clone(),
                site: span,
                identity: format!("{identity:032x}"),
            },
            alias,
            path,
            span,
        })
    }

    /// Record a generated identity, refusing a digest that has already stood
    /// for a different key.
    fn claim(&mut self, resolver: &mut Resolver, key: String, span: SourceSpan) -> Option<u128> {
        let digest = musa_kernel::stable_digest(key.as_bytes());
        if let Some(first) = self.assigned.get(&digest)
            && *first != key
        {
            resolver.report(
                Diagnostic::error(Code::DuplicateName, "two generated declarations hashed to one identity")
                    .at(span, "this instance")
                    .note("the digest is 128 bits, so this is a compiler defect rather than something to work around"),
            );
            return None;
        }
        self.assigned.insert(digest, key);
        Some(digest)
    }
}

/// One resolved instance site.
pub(crate) struct Instance {
    /// The template's name.
    name: String,
    /// The declaration the template parameterizes.
    declaration: SyntaxNode,
    parameters: Vec<Parameter>,
    /// The argument expressions, already matched to parameters.
    arguments: Vec<SyntaxNode>,
    /// The provenance step every event this instance produces carries, and
    /// the generated identity inside it.
    step: ExpansionStep,
    /// The `as` name: this instance's address in the source.
    alias: String,
    /// The site's structural address, which is what identity is derived
    /// from. A span would move whenever anything above it was edited; a path
    /// moves only when the site does.
    path: String,
    span: SourceSpan,
}

impl Instance {
    /// The piece this instance makes, when it makes one.
    pub(crate) fn piece(&self) -> Option<PieceDecl> {
        PieceDecl::cast(self.declaration.clone())
    }

    /// The voice this instance makes, when it makes one.
    pub(crate) fn voice(&self) -> Option<VoiceDecl> {
        VoiceDecl::cast(self.declaration.clone())
    }

    /// The provenance step every event this instance produces carries.
    pub(crate) fn step(&self) -> ExpansionStep {
        self.step.clone()
    }

    /// The `as` name.
    pub(crate) fn alias(&self) -> &str {
        &self.alias
    }

    /// The template this instance names.
    pub(crate) fn template(&self) -> &str {
        &self.name
    }

    /// The span of the `make` statement.
    pub(crate) fn span(&self) -> SourceSpan {
        self.span
    }

    /// Each parameter's name, the type the template declared it at, and the
    /// expression this site gives it, in declaration order.
    ///
    /// Three written nodes rather than a binding, because a reading that
    /// elaborates into the core writes the binding itself: the parameters
    /// become λs over the template's body and the arguments become what that
    /// λ is applied to, so every argument is elaborated in the scope the site
    /// stands in without a holder name to keep it out of the body's way.
    /// [`Self::holders`] and [`Self::bindings`] are the same question answered
    /// for the checker being replaced, which holds values and needs the two
    /// scopes kept apart by hand.
    pub(crate) fn bound(&self) -> impl Iterator<Item = Bound<'_>> {
        self.parameters
            .iter()
            .zip(&self.arguments)
            .map(|(parameter, argument)| Bound {
                name: &parameter.name,
                ty: &parameter.ty,
                argument,
            })
    }

    /// Bindings that carry this site's arguments into the pass that checks
    /// the scope the site stands in.
    ///
    /// The names are unspellable on purpose: they hold values on the way to
    /// somewhere else, and a name an author could write would be a name an
    /// author could collide with.
    pub(crate) fn holders(&self) -> Vec<Binding> {
        self.parameters
            .iter()
            .zip(&self.arguments)
            .map(|(parameter, argument)| {
                Binding::argument(
                    self.holder(&parameter.name),
                    parameter.name_span,
                    trimmed_span(argument),
                    parameter.ty.clone(),
                    argument.clone(),
                    true,
                )
            })
            .collect()
    }

    /// Those same arguments, under the names the template's body reads them
    /// by. This is the substitution, done as a binding.
    pub(crate) fn bindings(&self, site_scope: &Program) -> Vec<Binding> {
        self.parameters
            .iter()
            .filter_map(|parameter| {
                site_scope.rebind(
                    &self.holder(&parameter.name),
                    parameter.name.clone(),
                    parameter.name_span,
                    parameter.span,
                    parameter.ty.clone(),
                )
            })
            .collect()
    }

    fn holder(&self, parameter: &str) -> String {
        // `#` and the path separators cannot occur in an identifier, so a
        // holder can never be the name of anything the source declares.
        format!("make{UNIT}{}#{parameter}", self.path)
    }
}

/// One parameter of a template, and the argument an instance site gives it.
///
/// Borrowed from the [`Instance`], because the caller writes terms out of these
/// and never keeps them: the nodes belong to the parsed document, and a copy
/// would only be a second name for the same tree.
pub(crate) struct Bound<'a> {
    /// The name the template's body reads the argument by.
    pub(crate) name: &'a str,
    /// The type the template declared it at, as written.
    pub(crate) ty: &'a SyntaxNode,
    /// The expression the site wrote for it, in the scope the site stands in.
    pub(crate) argument: &'a SyntaxNode,
}

/// The parameters a template declares, with duplicates reported once.
fn collect_parameters(resolver: &mut Resolver, template: &TemplateDecl, name: &str) -> Vec<Parameter> {
    let mut parameters: Vec<Parameter> = Vec::new();
    for parameter in template.params() {
        let span = trimmed_span(parameter.syntax());
        let Some(parameter_name) = parameter.name() else {
            continue;
        };
        let Some(ty) = crate::core::type_node_of(parameter.syntax()) else {
            resolver.report(
                Diagnostic::error(Code::NotAValue, format!("parameter `{parameter_name}` has no type"))
                    .at(span, "a template parameter states the type it takes")
                    .help("write `name: type`, as a function's parameters do"),
            );
            continue;
        };
        if let Some(first) = parameters.iter().find(|held| held.name == parameter_name) {
            resolver.report(
                Diagnostic::error(
                    Code::DuplicateName,
                    format!("template `{name}` binds `{parameter_name}` twice"),
                )
                .at(span, "bound again here")
                .also(first.span, "first bound here"),
            );
            continue;
        }
        let name_span =
            crate::resolve::token_span(parameter.syntax(), musa_language::SyntaxKind::Identifier).unwrap_or(span);
        parameters.push(Parameter {
            name: parameter_name,
            name_span,
            span,
            ty,
        });
    }
    parameters
}

/// Match a site's arguments to its template's parameters, positionally.
fn arguments_of(resolver: &mut Resolver, stmt: &MakeStmt, entry: &Entry, name: &str) -> Option<Vec<SyntaxNode>> {
    let span = trimmed_span(stmt.syntax());
    let written = stmt.args();
    if written.len() != entry.parameters.len() {
        resolver.report(
            Diagnostic::error(
                Code::WrongArity,
                format!(
                    "`{name}` takes {} argument{}, and this instance gives {}",
                    entry.parameters.len(),
                    if entry.parameters.len() == 1 { "" } else { "s" },
                    written.len()
                ),
            )
            .at(span, "this instance")
            .also(entry.span, "declared here"),
        );
        return None;
    }
    let mut arguments = Vec::with_capacity(written.len());
    for argument in &written {
        let node = argument.syntax();
        if crate::resolve::token_span(node, musa_language::SyntaxKind::Colon).is_some() {
            resolver.report(
                Diagnostic::error(Code::WrongArity, "template arguments are positional")
                    .at(trimmed_span(node), "this argument is named")
                    .help("write the arguments in the order the template declares its parameters"),
            );
            return None;
        }
        arguments.push(crate::core::expr_node_of(node)?);
    }
    Some(arguments)
}
