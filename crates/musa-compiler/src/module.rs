//! Signatures, modules, and static module functors
//! (`docs/rules/language/04-templates-and-modules.md` §4).
//!
//! A module is a *name for a group of declarations*, not a thing. It holds no
//! state, is never a value, never crosses a crate boundary, and has stopped
//! existing by the time anything is evaluated: this stage turns
//! `structure M { let a … }` into an ordinary definition called `M.a` in the
//! one flat namespace the core already has, and turns `make F(A) as G;` into the
//! same definitions again, checked a second time in a scope where `F`'s
//! parameters name `A`'s members.
//!
//! That is why there is no Rust `Functor` trait and no public module object.
//! A trait would model a functor as a value with methods, and a module object
//! would model a module as a thing that exists at run time; neither is true
//! here, and either would invite the applicative sharing this design refuses.
//! What a functor *is* is a second checking pass over one body — so what it
//! is written as is a scope, and [`NameScope`] is the whole of it.
//!
//! Expansion is *binding*, never rewriting. The template's body is checked
//! once per instance with its parameter names resolving elsewhere; no syntax
//! is copied, no span moves, and no name can be captured. Identity is
//! generative: it is derived from the functor, its arguments' identities, and
//! the *site*, so two instances with equal arguments are two modules, always.

use indexmap::{IndexMap, IndexSet};
use musa_language::ast::{
    AstNode as _, DataMember, FnDecl, LetDecl, MakeStmt, SignatureDecl, StructureDecl, TemplateDecl,
};
use musa_language::{SyntaxKind, SyntaxNode};

use crate::diagnose::{Code, Diagnostic};
use crate::origin::SourceSpan;
use crate::resolve::{NameKind, Resolver, trimmed_span};
use crate::template::{DIGEST_VERSION, UNIT};

/// The separator between a module and one of its members.
///
/// A dot, because that is what the source writes. Names in the core's flat
/// namespace are plain identifiers, so no member name can ever collide with a
/// qualified one.
pub(crate) const DOT: char = '.';

/// One member of a module, flattened into the definition the core will check.
pub(crate) struct Member {
    /// The qualified name the flat namespace holds it by: `CMajor.tonic`.
    pub(crate) name: String,
    /// The declaration to check — the module's own, or, for an instance, the
    /// functor's. Nothing is copied: two instances of one functor point at
    /// the same syntax and differ only in [`Member::scope`].
    pub(crate) item: MemberItem,
    /// How names read inside it.
    pub(crate) scope: NameScope,
    /// The document it was written in, when that is not this one.
    pub(crate) source: Option<String>,
}

/// The two declaration forms a module member may take.
///
/// A function is a value of arrow type, so a signature spells both as `let`;
/// a module still writes a function as `fn`, because that is how a function
/// is written everywhere else in the language.
#[derive(Clone)]
pub(crate) enum MemberItem {
    Let(LetDecl),
    Function(FnDecl),
}

/// One value argument of a functor instance, on its way into the pass that
/// checks the scope the instance is written in.
pub(crate) struct Argument {
    /// The unspellable name the value is held under.
    pub(crate) holder: String,
    /// The parameter's declared type, as written.
    pub(crate) ty: SyntaxNode,
    /// The expression at the site, checked in the site's own scope.
    pub(crate) expr: SyntaxNode,
}

/// How names read inside one definition.
///
/// Empty for everything that is not a module member, which is why it is the
/// default: an ordinary `let` at a document's root reads names the way it
/// always has.
#[derive(Clone, Default)]
pub(crate) struct NameScope {
    /// The module the definition belongs to. A bare sibling member is reached
    /// through it, so a module's own definitions read each other by their
    /// short names.
    owner: Option<String>,
    /// What the first word of a name stands for here: a functor's module
    /// parameter for the module the site passed it, and a value parameter for
    /// the holder its argument was evaluated into.
    heads: IndexMap<String, Head>,
}

impl NameScope {
    /// The scope everything outside a module reads in.
    ///
    /// One shared value rather than a fresh default per checker: an empty
    /// scope has nothing to own, and a borrow of it outlives any pass.
    pub(crate) fn empty() -> &'static Self {
        static EMPTY: std::sync::OnceLock<NameScope> = std::sync::OnceLock::new();
        EMPTY.get_or_init(Self::default)
    }
}

#[derive(Clone)]
struct Head {
    /// The name in the flat namespace the head stands for.
    target: String,
    /// The signature the head is seen through, when it is a module. A functor
    /// body may name exactly what its parameter's signature lists, however
    /// much more the module the site passed happens to define.
    through: Option<String>,
}

/// How one written name reads, once the scope it is written in has had its
/// say.
pub(crate) struct Reading {
    /// The name the flat namespace holds it by.
    pub(crate) name: String,
    /// The signature that hides it, when the name reaches into a module for
    /// something that module does not export, and where that signature is
    /// written.
    pub(crate) sealed_by: Option<(String, SourceSpan)>,
}

/// A signature, and what it requires.
struct Signature {
    span: SourceSpan,
    members: IndexMap<String, Required>,
    /// The types the signature requires without saying what they are: a
    /// `data Name;` member. What matches one is a `data Name { … }` in the
    /// structure, and the constructors stay the structure's own.
    types: IndexMap<String, SourceSpan>,
}

struct Required {
    ty: Written,
    span: SourceSpan,
}

/// A type as the two sides of a match wrote it.
///
/// §4 matches "by name and exact type", and the language has no type aliases,
/// so two members' types agree exactly when their spellings do. Comparing
/// spellings rather than checked types is also the only reading that can carry
/// *every* type the language has: a signature that writes
/// `EventTrack<WrittenTime>` is an ordinary signature, and a module system that
/// could only match the types one pass happened to have a constructor for would
/// be a module system for part of the language.
///
/// A [`Vec`] and not a string because a `fn` writes its arrow in pieces and a
/// signature writes it whole, and because a part a declaration left unannotated
/// matches whatever the signature puts there — the one liberty §4 allows, and
/// the reason each part is an [`Option`].
#[derive(Clone, PartialEq, Eq)]
struct Written(Vec<Option<String>>);

impl Written {
    /// The spelling of one written type node, layout collapsed.
    fn one(node: &SyntaxNode) -> Self {
        Self(vec![Some(spelling(node))])
    }

    /// The spelling of an arrow, split at its arrows: `Nat -> Option<Pitch>`
    /// reads as the same two parts a `fn` of that type writes separately.
    fn arrow(node: &SyntaxNode) -> Self {
        let inner = if node.kind() == SyntaxKind::TypeExpr {
            node.children().find(|child| is_type(child.kind()))
        } else {
            Some(node.clone())
        };
        let Some(inner) = inner else { return Self::one(node) };
        if inner.kind() != SyntaxKind::FunctionType {
            return Self::one(&inner);
        }
        Self(
            inner
                .children()
                .filter(|child| is_type(child.kind()))
                .map(|child| Some(spelling(&child)))
                .collect(),
        )
    }

    /// Whether a module providing this satisfies a signature requiring
    /// `required`.
    fn matches(&self, required: &Self) -> bool {
        self.0.len() == required.0.len()
            && self
                .0
                .iter()
                .zip(&required.0)
                .all(|(provided, required)| provided.is_none() || provided == required)
    }
}

impl std::fmt::Display for Written {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut parts = self.0.iter();
        if let Some(first) = parts.next() {
            out.write_str(first.as_deref().unwrap_or("_"))?;
        }
        for part in parts {
            write!(out, " -> {}", part.as_deref().unwrap_or("_"))?;
        }
        Ok(())
    }
}

/// Whether a structure body declares a type called `name`.
///
/// What a signature's `data Name;` asks. Matching is by name, as everything
/// else in §4 is, and the question is answered off the structure's own
/// declarations because that is where a `data` written inside one is: an
/// instance's abstract types are the functor's, so asking the body asks the
/// right node for a written module and a made one alike.
fn declares_type(body: &StructureDecl, name: &str) -> bool {
    musa_language::ast::DataDecl::all_at_root(body.syntax())
        .iter()
        .any(|declaration| declaration.name().as_deref() == Some(name))
}

/// One written type node's text, with every run of layout collapsed to one
/// space, so `List< Pitch >` and `List<Pitch>` are the one type they are.
fn spelling(node: &SyntaxNode) -> String {
    node.to_string().split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The arrow a `fn` declares, in the parts a signature writes it in.
///
/// A parameter's type is written inside the parameter list and the result is
/// the one type node the `fn` has as a direct child, so the two are read
/// separately and joined here. An omitted annotation stays [`None`]: the
/// declaration did not say, and a signature that does say is entitled to.
fn written_arrow(declaration: &FnDecl) -> Written {
    let mut parts: Vec<Option<String>> = declaration
        .params()
        .iter()
        .map(|parameter| {
            parameter
                .syntax()
                .children()
                .find(|node| is_type(node.kind()))
                .as_ref()
                .map(spelling)
        })
        .collect();
    parts.push(
        declaration
            .syntax()
            .children()
            .find(|node| is_type(node.kind()))
            .as_ref()
            .map(spelling),
    );
    Written(parts)
}

/// A module in scope, and the signature that says what may be named in it.
struct Sealed {
    signature: String,
    /// Where the signature's name is written at the module, which is the
    /// label a sealing diagnostic points at.
    ascription: SourceSpan,
}

/// Everything a document's `signature`, `structure`, `template structure`, and
/// module-`make` declarations mean, once matching has been checked.
///
/// Built once per checking pass and read from the checker. It answers exactly
/// two questions — what definitions exist, and how a name reads — because
/// those are the only two the rest of the compiler has for it.
#[derive(Default)]
pub(crate) struct Modules {
    signatures: IndexMap<String, Signature>,
    sealed: IndexMap<String, Sealed>,
    members: Vec<Member>,
    arguments: Vec<Argument>,
    /// Each module's generative identity: its own name for a written module,
    /// and the digest of the instance for a made one.
    identities: IndexMap<String, String>,
    /// Digests already handed out, and the key each came from.
    assigned: IndexMap<u128, String>,
}

impl Modules {
    /// Read every signature, module, and functor instance a checking pass can
    /// see: what its imports export, in import order, then its own.
    ///
    /// `owners` yields the node whose direct children the declarations are —
    /// an imported `library` block, or a document's lexical root — together
    /// with the document it belongs to when that is not this one.
    pub(crate) fn read<'a>(
        resolver: &mut Resolver,
        owners: impl Iterator<Item = (Option<&'a str>, SyntaxNode)>,
    ) -> Self {
        let mut modules = Self::default();
        let owners: Vec<_> = owners.collect();
        for (source, owner) in &owners {
            modules.read_signatures(resolver, *source, owner);
        }
        for (source, owner) in &owners {
            Self::read_templates(resolver, *source, owner);
        }
        for (source, owner) in &owners {
            modules.read_modules(resolver, *source, owner);
        }
        let mut sites = Vec::new();
        for (source, owner) in &owners {
            sites.extend(instance_sites(*source, owner));
        }
        for site in order(resolver, sites) {
            modules.read_instance(resolver, &site);
        }
        modules
    }

    /// Whether any of `owners` writes a module declaration at all.
    ///
    /// Asked before [`Self::read`] because reading needs a [`crate::data::World`]
    /// and building one is a pass over every `data` declaration in the import
    /// closure. Almost no document writes a module, and the answer for one that
    /// does not is [`Self::default`] — so the cost of the module system is paid
    /// by the documents that use it rather than by all of them.
    ///
    /// A `make` is not on the list. It names a template, and the template it
    /// names is a `template structure` or it is a template *piece* and no
    /// module at all; either way a `template structure` had to be written
    /// somewhere in the same closure, so looking for one is both cheaper and
    /// exact where looking for a `make` would be neither.
    pub(crate) fn written_in<'a>(owners: impl Iterator<Item = &'a SyntaxNode>) -> bool {
        owners.flat_map(SyntaxNode::children).any(|node| {
            matches!(
                node.kind(),
                SyntaxKind::SignatureDecl | SyntaxKind::StructureDecl | SyntaxKind::TemplateDecl
            )
        })
    }

    /// The flattened members, in the order they were read.
    pub(crate) fn members(&self) -> &[Member] {
        &self.members
    }

    /// The value arguments every instance site wrote.
    pub(crate) fn arguments(&self) -> &[Argument] {
        &self.arguments
    }

    /// How `name`, written inside `scope`, reads — when a module decides it at
    /// all.
    ///
    /// [`None`] when none does, which is the answer for almost every name in
    /// almost every document, and is what lets a reading fall through to what
    /// `.` means everywhere else. `10-traits.md` §6 gives `.` to projection, so
    /// a walk that read every dotted name as a module member would take
    /// `pending.anchor` away from records; a walk that read none as one would
    /// take `Away.tonic` away from modules. The question the two readings turn
    /// on is not the spelling but whether the head *is* a module here, and that
    /// is a question only this type can answer.
    ///
    /// Three ways it can be one: the head is a functor's parameter bound by the
    /// scope, the head is a module this pass read, or the name is bare and the
    /// module it is written in has a member by that name. Everything else
    /// belongs to somebody else.
    pub(crate) fn resolve(&self, scope: &NameScope, name: &str) -> Option<Reading> {
        let head = name.split_once(DOT).map_or(name, |(head, _)| head);
        let decides = scope.heads.contains_key(head)
            || self.identities.contains_key(head)
            || (!name.contains(DOT)
                && scope
                    .owner
                    .as_ref()
                    .is_some_and(|owner| self.defines(&format!("{owner}{DOT}{name}"))));
        decides.then(|| self.read_name(scope, name, &|candidate| self.defines(candidate)))
    }

    /// Whether a member of that qualified name was flattened out of some
    /// module.
    fn defines(&self, name: &str) -> bool {
        self.members.iter().any(|member| member.name == name)
    }

    /// How `name`, written inside `scope`, reads.
    ///
    /// `known` decides whether a candidate qualified name exists, so a bare
    /// name inside a module reaches a sibling member when there is one and
    /// falls through to the document's own names when there is not.
    pub(crate) fn read_name(&self, scope: &NameScope, name: &str, known: &dyn Fn(&str) -> bool) -> Reading {
        let (head, member) = match name.split_once(DOT) {
            Some((head, member)) => (head, Some(member)),
            None => (name, None),
        };
        if let Some(bound) = scope.heads.get(head) {
            let Some(member) = member else {
                return Reading {
                    name: bound.target.clone(),
                    sealed_by: None,
                };
            };
            return Reading {
                name: format!("{}{DOT}{member}", bound.target),
                sealed_by: bound
                    .through
                    .as_ref()
                    .and_then(|signature| self.hidden(signature, member, None)),
            };
        }
        if member.is_none()
            && let Some(owner) = &scope.owner
        {
            let sibling = format!("{owner}{DOT}{name}");
            if known(&sibling) {
                return Reading {
                    name: sibling,
                    sealed_by: None,
                };
            }
        }
        let sealed_by = member.and_then(|member| {
            if scope.owner.as_deref() == Some(head) {
                return None;
            }
            let sealed = self.sealed.get(head)?;
            self.hidden(&sealed.signature, member, Some(sealed.ascription))
        });
        Reading {
            name: name.to_owned(),
            sealed_by,
        }
    }

    /// The signature that hides `member`, and where to point at it, or `None`
    /// when the signature exports it.
    fn hidden(&self, signature: &str, member: &str, ascription: Option<SourceSpan>) -> Option<(String, SourceSpan)> {
        let declared = self.signatures.get(signature)?;
        if declared.members.contains_key(member) {
            return None;
        }
        Some((signature.to_owned(), ascription.unwrap_or(declared.span)))
    }

    fn read_signatures(&mut self, resolver: &mut Resolver, source: Option<&str>, owner: &SyntaxNode) {
        for declaration in SignatureDecl::all_at_root(owner) {
            let Some(name) = declaration.name() else { continue };
            let span = name_span(declaration.syntax());
            let mut members: IndexMap<String, Required> = IndexMap::new();
            let mut types: IndexMap<String, SourceSpan> = IndexMap::new();
            for member in declaration.syntax().children().filter_map(DataMember::cast) {
                if let Some(name) = member.name() {
                    types.insert(name, name_span(member.syntax()));
                }
            }
            for member in declaration.members() {
                let Some(member_name) = member.name() else { continue };
                let member_span = name_span(member.syntax());
                let Some(ty) = member.ty().map(|ty| Written::arrow(&ty)) else {
                    continue;
                };
                if let Some(first) = members.get(&member_name) {
                    resolver.report(
                        Diagnostic::error(Code::DuplicateName, format!("`{name}` requires `{member_name}` twice"))
                            .at(member_span, "required again here")
                            .also(first.span, "first required here"),
                    );
                    continue;
                }
                // The member is a name a reader may write — `M.tonic`, for a
                // template parameter `M: TonalContext` — so it owes the same
                // record every other reachable name has. It is a *required*
                // type and not a definition, which is why the signature reads
                // `let TonalContext.spell: Nat -> Option<Pitch>`: no parameter
                // is named here because a signature names none.
                resolver.references.document(member_document(
                    &format!("{name}.{member_name}"),
                    &ty,
                    member_span,
                    source,
                    member.syntax(),
                ));
                members.insert(member_name, Required { ty, span: member_span });
            }
            if let Some(first) = self.signatures.get(&name) {
                resolver.report(
                    Diagnostic::error(
                        Code::DuplicateName,
                        format!("`{name}` is declared twice as a signature"),
                    )
                    .at(span, "declared again here")
                    .also(first.span, "first declared here")
                    .help("give one of them a different name"),
                );
                continue;
            }
            if source.is_none() {
                resolver.references.declare(NameKind::Module, &name, span);
            }
            resolver.references.document(document(
                &name,
                NameKind::Module,
                format!("signature {name}"),
                span,
                source,
                declaration.syntax(),
            ));
            self.signatures.insert(name, Signature { span, members, types });
        }
    }

    /// Record the templates written here: what a `make` site names.
    ///
    /// Reading, not checking — a template is checked where it is applied,
    /// because until then there is nothing to check it against. What this
    /// pass owes an editor is the declaration a `make` site jumps to and the
    /// signature it is applied against, and both are written here.
    fn read_templates(resolver: &mut Resolver, source: Option<&str>, owner: &SyntaxNode) {
        for declaration in TemplateDecl::all_at_root(owner) {
            let Some(name) = declaration.name() else { continue };
            // The name is written on what the template parameterizes, not on
            // the `template` word: a span taken from the outer node would be
            // the whole declaration, and a jump to it would select the body.
            let span = declaration
                .piece()
                .map(|piece| name_span(piece.syntax()))
                .or_else(|| declaration.voice().map(|voice| name_span(voice.syntax())))
                .or_else(|| declaration.structure().map(|structure| name_span(structure.syntax())))
                .unwrap_or_else(|| name_span(declaration.syntax()));
            if source.is_none() {
                resolver.references.declare(NameKind::Template, &name, span);
            }
            let ascribed = declaration
                .structure()
                .and_then(|structure| structure.signature())
                .map_or_else(String::new, |signature| format!(": {signature}"));
            resolver.references.document(document(
                &name,
                NameKind::Template,
                format!("template {name}{ascribed}"),
                span,
                source,
                declaration.syntax(),
            ));
        }
    }

    fn read_modules(&mut self, resolver: &mut Resolver, source: Option<&str>, owner: &SyntaxNode) {
        for declaration in StructureDecl::all_at_root(owner) {
            let Some(name) = declaration.name() else { continue };
            let span = name_span(declaration.syntax());
            let scope = NameScope {
                owner: Some(name.clone()),
                heads: IndexMap::new(),
            };
            let provided = self.flatten(resolver, &name, &declaration, &scope, source);
            let Some(signature) = declaration.signature() else {
                continue;
            };
            let ascription = ascription_span(declaration.syntax());
            self.match_signature(resolver, &signature, ascription, &name, span, &provided);
            self.match_abstract_types(resolver, &declaration, &signature, &name, span);
            if source.is_none() {
                resolver.references.declare(NameKind::Module, &name, span);
                resolver.references.record_use(NameKind::Module, &signature, ascription);
            }
            resolver.references.document(document(
                &name,
                NameKind::Module,
                format!("structure {name}: {signature}"),
                span,
                source,
                declaration.syntax(),
            ));
            self.declare(resolver, name.clone(), signature, ascription, span);
            self.identities.insert(name.clone(), format!("module {name}"));
        }
    }

    /// Every `data Name;` a signature requires must be a `data Name { … }` in
    /// the structure ascribed to it.
    ///
    /// A structure that declared no such type would export a name with
    /// nothing behind it, which is exactly the hole a signature exists to
    /// close.
    fn match_abstract_types(
        &self,
        resolver: &mut Resolver,
        body: &StructureDecl,
        signature: &str,
        module: &str,
        span: SourceSpan,
    ) {
        let Some(declared) = self.signatures.get(signature) else {
            return;
        };
        for (name, required) in &declared.types {
            if declares_type(body, name) {
                continue;
            }
            resolver.report(
                Diagnostic::error(Code::UnknownName, format!("`{module}` does not declare `{name}`"))
                    .at(*required, format!("required as a type by `{signature}`"))
                    .also(span, format!("`{signature}` is not satisfied here"))
                    .help(format!("write `data {name} {{ … }}` in `{module}`")),
            );
        }
    }

    fn read_instance(&mut self, resolver: &mut Resolver, site: &Site) {
        let span = trimmed_span(site.stmt.syntax());
        let Some(scope) = self.apply(resolver, site, span) else {
            return;
        };
        let provided = self.flatten(resolver, &site.alias, &site.functor, &scope, site.source.as_deref());
        let Some(signature) = site.functor.signature() else {
            return;
        };
        let ascription = ascription_span(site.functor.syntax());
        self.match_signature(resolver, &signature, ascription, &site.alias, span, &provided);
        if site.source.is_none() {
            resolver
                .references
                .declare(NameKind::Module, &site.alias, site.alias_span);
            resolver
                .references
                .record_use(NameKind::Template, &site.functor_name, name_span(site.stmt.syntax()));
        }
        resolver.references.document(document(
            &site.alias,
            NameKind::Module,
            format!("structure {} = {}: {signature}", site.alias, site.functor_name),
            site.alias_span,
            site.source.as_deref(),
            site.stmt.syntax(),
        ));
        self.declare(resolver, site.alias.clone(), signature, ascription, site.alias_span);
    }

    /// Resolve one instance site's arguments into the scope its members are
    /// checked in, and settle its generative identity.
    fn apply(&mut self, resolver: &mut Resolver, site: &Site, span: SourceSpan) -> Option<NameScope> {
        let Site {
            functor,
            functor_name,
            alias,
            ..
        } = site;
        let parameters = functor.params();
        let arguments = site.stmt.args();
        if parameters.len() != arguments.len() {
            resolver.report(
                Diagnostic::error(
                    Code::WrongArity,
                    format!(
                        "`{functor_name}` takes {} argument{}, and {} {} given",
                        parameters.len(),
                        if parameters.len() == 1 { "" } else { "s" },
                        arguments.len(),
                        if arguments.len() == 1 { "was" } else { "were" },
                    ),
                )
                .at(span, "this instance")
                .also(name_span(functor.syntax()), "declared here"),
            );
            return None;
        }
        let mut heads: IndexMap<String, Head> = IndexMap::new();
        let mut argument_identities = Vec::with_capacity(parameters.len());
        for (parameter, argument) in parameters.iter().zip(&arguments) {
            let Some(parameter_name) = parameter.name() else {
                continue;
            };
            let Some(ty) = parameter.syntax().children().find(|node| is_type(node.kind())) else {
                continue;
            };
            let Some(expression) = argument.syntax().children().next() else {
                continue;
            };
            let parameter_span = name_span(parameter.syntax());
            match self.signature_named(&ty) {
                Some(signature) => {
                    let Some(module) = module_named(&expression) else {
                        resolver.report(
                            Diagnostic::error(
                                Code::TypeMismatch,
                                format!("`{parameter_name}` takes a structure matching `{signature}`"),
                            )
                            .at(trimmed_span(&expression), "this is not a structure's name")
                            .also(parameter_span, "declared here")
                            .help("pass the name of a structure written with `structure`, or made with `make`"),
                        );
                        return None;
                    };
                    argument_identities.push(self.check_argument(resolver, &module, &signature, &expression)?);
                    heads.insert(
                        parameter_name,
                        Head {
                            target: module,
                            through: Some(signature),
                        },
                    );
                }
                None => {
                    let holder = holder(alias, &parameter_name);
                    self.arguments.push(Argument {
                        holder: holder.clone(),
                        ty,
                        expr: expression,
                    });
                    heads.insert(
                        parameter_name,
                        Head {
                            target: holder,
                            through: None,
                        },
                    );
                }
            }
        }
        // Identity is settled here, once, where the site resolved: an
        // instance that exists has an identity, and one that failed to
        // resolve never gets to claim a digest. The key names the functor,
        // what it was applied to, and the *site* — never the argument
        // expressions — so equal arguments at two sites stay two modules.
        let digest = self.claim(resolver, identity_key(functor_name, alias, &argument_identities), span)?;
        self.identities.insert(alias.to_owned(), format!("{digest:032x}"));
        Some(NameScope {
            owner: Some(alias.to_owned()),
            heads,
        })
    }

    /// Check that the module an argument names satisfies the parameter's
    /// signature, and answer with that module's identity.
    fn check_argument(
        &self,
        resolver: &mut Resolver,
        module: &str,
        signature: &str,
        expression: &SyntaxNode,
    ) -> Option<String> {
        let span = trimmed_span(expression);
        let Some(sealed) = self.sealed.get(module) else {
            resolver.report(
                Diagnostic::error(Code::UnknownName, format!("no structure called `{module}`"))
                    .at(span, "nothing declares this structure")
                    .help("structures are written with `structure`, or made with `make`, before they are passed"),
            );
            return None;
        };
        // Matching is by name and exact type, and both sides have already
        // been checked against their own signature, so satisfying the
        // parameter is a question about the two signatures.
        let provided = sealed.signature.clone();
        if provided != signature
            && let (Some(wanted), Some(have)) = (self.signatures.get(signature), self.signatures.get(&provided))
        {
            for (name, required) in &wanted.members {
                let mismatch = match have.members.get(name) {
                    // The same relation the ascription check reads, rather
                    // than plain equality: a member that left its annotation
                    // out is still the member the signature named, and one
                    // question decided two ways is two languages.
                    Some(found) if found.ty.matches(&required.ty) => continue,
                    Some(found) => format!("`{module}` gives it type {}", found.ty),
                    None => format!("`{module}` does not provide it"),
                };
                resolver.report(
                    Diagnostic::error(Code::TypeMismatch, format!("`{module}` does not match `{signature}`"))
                        .at(span, format!("`{signature}` requires `{name}`: {}", required.ty))
                        .also(required.span, mismatch)
                        .help("a structure matches a signature when it defines every member with exactly that type"),
                );
                return None;
            }
        }
        self.identities.get(module).cloned()
    }

    /// Flatten a module body's declarations into members under `qualifier`,
    /// answering with each member's declared type for signature matching.
    fn flatten(
        &mut self,
        resolver: &mut Resolver,
        qualifier: &str,
        declaration: &StructureDecl,
        scope: &NameScope,
        source: Option<&str>,
    ) -> IndexMap<String, Required> {
        let mut provided: IndexMap<String, Required> = IndexMap::new();
        let mut items: Vec<(String, SourceSpan, MemberItem, Option<Written>)> = Vec::new();
        for binding in declaration.lets() {
            let Some(name) = binding.name() else { continue };
            let ty = binding
                .syntax()
                .children()
                .find(|node| is_type(node.kind()))
                .map(|node| Written::one(&node));
            items.push((name, name_span(binding.syntax()), MemberItem::Let(binding), ty));
        }
        for function in declaration.fns() {
            let Some(name) = function.name() else { continue };
            let ty = Some(written_arrow(&function));
            items.push((name, name_span(function.syntax()), MemberItem::Function(function), ty));
        }
        for (name, span, item, ty) in items {
            if let Some(first) = provided.get(&name) {
                resolver.report(
                    Diagnostic::error(Code::DuplicateName, format!("`{qualifier}` defines `{name}` twice"))
                        .at(span, "defined again here")
                        .also(first.span, "first defined here"),
                );
                continue;
            }
            if let Some(ty) = ty {
                provided.insert(name.clone(), Required { ty, span });
            }
            self.members.push(Member {
                name: format!("{qualifier}{DOT}{name}"),
                item,
                scope: scope.clone(),
                source: source.map(str::to_owned),
            });
        }
        provided
    }

    /// Report every way `provided` fails to satisfy `signature`.
    ///
    /// Both labels, always: the member the signature asked for and the module
    /// that answered, because reading only one of them never says what to
    /// change. Members the signature does not list are *private*, not errors
    /// — that is what sealing means, and [`Modules::read_name`] is where it
    /// is enforced.
    fn match_signature(
        &self,
        resolver: &mut Resolver,
        signature: &str,
        ascription: SourceSpan,
        module: &str,
        span: SourceSpan,
        provided: &IndexMap<String, Required>,
    ) {
        let Some(declared) = self.signatures.get(signature) else {
            resolver.report(
                Diagnostic::error(Code::UnknownName, format!("no signature called `{signature}`"))
                    .at(ascription, "not a signature this file declares")
                    .help("a structure names the signature it provides, and that signature must be declared"),
            );
            return;
        };
        for (name, required) in &declared.members {
            match provided.get(name) {
                // Exact type, part for part — except where the member left an
                // annotation out, which the signature is entitled to supply.
                Some(found) if found.ty.matches(&required.ty) => {}
                Some(found) => resolver.report(
                    Diagnostic::error(Code::TypeMismatch, format!("`{module}` gives `{name}` the wrong type"))
                        .at(found.span, format!("this is a {}", found.ty))
                        .also(required.span, format!("`{signature}` requires a {}", required.ty))
                        .help("matching is by name and exact type"),
                ),
                None => resolver.report(
                    Diagnostic::error(Code::UnknownName, format!("`{module}` does not define `{name}`"))
                        .at(span, format!("`{signature}` is not satisfied here"))
                        .also(required.span, format!("required as a {}", required.ty))
                        .help(format!(
                            "define `{name}` in `{module}`, or take it out of `{signature}`"
                        )),
                ),
            }
        }
    }

    fn declare(
        &mut self,
        resolver: &mut Resolver,
        name: String,
        signature: String,
        ascription: SourceSpan,
        span: SourceSpan,
    ) {
        if self.sealed.contains_key(&name) {
            resolver.report(
                Diagnostic::error(
                    Code::DuplicateName,
                    format!("`{name}` is declared twice as a structure"),
                )
                .at(span, "declared again here")
                .help("give one of them a different name"),
            );
            return;
        }
        self.sealed.insert(name, Sealed { signature, ascription });
    }

    /// The signature a parameter's declared type names, when it names one.
    ///
    /// A signature is not a value type, so it can never be confused with one:
    /// `parse_type` knows every value type there is, and a bare name that is
    /// not among them and *is* a declared signature is a module parameter.
    fn signature_named(&self, ty: &SyntaxNode) -> Option<String> {
        let name = ty
            .descendants()
            .find(|node| node.kind() == SyntaxKind::TypeName)
            .map(|node| node.to_string().trim().to_owned())?;
        self.signatures.contains_key(&name).then_some(name)
    }

    /// Record a generated identity, refusing a digest that has already stood
    /// for a different key.
    fn claim(&mut self, resolver: &mut Resolver, key: String, span: SourceSpan) -> Option<u128> {
        let digest = musa_kernel::stable_digest(key.as_bytes());
        if let Some(first) = self.assigned.get(&digest)
            && *first != key
        {
            resolver.report(
                Diagnostic::error(Code::DuplicateName, "two generated modules hashed to one identity")
                    .at(span, "this instance")
                    .note("the digest is 128 bits, so this is a compiler defect rather than something to work around"),
            );
            return None;
        }
        self.assigned.insert(digest, key);
        Some(digest)
    }
}

/// What an editor is told about one module declaration (`crate::docs`).
///
/// Static structure: it names a group of declarations and never a value, so
/// it has no result type and no parameters. Its members are documented
/// separately, under the qualified names they are reached by, which is what a
/// reader completing `Harmony.` is actually asking for.
fn document(
    name: &str,
    kind: NameKind,
    signature: String,
    span: SourceSpan,
    source: Option<&str>,
    declaration: &SyntaxNode,
) -> crate::docs::ItemDoc {
    let summary = crate::docs::summary_above(declaration);
    crate::docs::ItemDoc {
        name: name.to_owned(),
        kind,
        source: crate::docs::ItemSource {
            uri: source.map(str::to_owned),
            span,
            read_only: source.is_some_and(|uri| crate::imports::standard_library_source(uri).is_some()),
        },
        deprecation: summary.as_deref().and_then(crate::docs::deprecation_in),
        summary,
        signature,
        result: None,
        parameters: Vec::new(),
    }
}

/// What an editor is told about one signature member (`crate::docs`).
///
/// A required value: it names a type and no definition, so there is a result
/// and there are no parameters even when the type is a function. Reached as
/// `Signature.member`, which is how a template body writes it, and how the
/// generated reference lists the contract a structure must meet.
fn member_document(
    name: &str,
    ty: &Written,
    span: SourceSpan,
    source: Option<&str>,
    declaration: &SyntaxNode,
) -> crate::docs::ItemDoc {
    let summary = crate::docs::summary_above(declaration);
    let result = crate::docs::TypeNote::new(ty.to_string());
    crate::docs::ItemDoc {
        name: name.to_owned(),
        kind: NameKind::Value,
        source: crate::docs::ItemSource {
            uri: source.map(str::to_owned),
            span,
            read_only: source.is_some_and(|uri| crate::imports::standard_library_source(uri).is_some()),
        },
        deprecation: summary.as_deref().and_then(crate::docs::deprecation_in),
        summary,
        signature: format!("let {name}: {}", result.name),
        result: Some(result),
        parameters: Vec::new(),
    }
}

/// The key an instance's generative identity is digested from.
///
/// The functor, what it was applied to, and the *site* — and nothing else.
/// Not the argument expressions, so two sites given equal arguments stay two
/// modules, which is what makes instantiation generative rather than
/// applicative. Not a span either, so editing the text above a site changes
/// where it is written and not what it made.
fn identity_key(functor: &str, alias: &str, arguments: &[String]) -> String {
    format!(
        "{DIGEST_VERSION}{UNIT}module {functor}{UNIT}module/{alias}{UNIT}{}",
        arguments.join(&UNIT.to_string()),
    )
}

/// One functor instance site, with everything reading it needs.
struct Site {
    stmt: MakeStmt,
    functor: StructureDecl,
    functor_name: String,
    alias: String,
    alias_span: SourceSpan,
    /// The modules its arguments name, which is what it must be read after.
    depends_on: Vec<String>,
    source: Option<String>,
}

/// Every functor instance an owner writes.
///
/// A `make` naming a piece or a voice template, or naming nothing at all,
/// belongs to the declaration-template stage, which reports what it cannot
/// resolve; this reads only the ones that make modules.
fn instance_sites(source: Option<&str>, owner: &SyntaxNode) -> Vec<Site> {
    MakeStmt::all_at_root(owner)
        .into_iter()
        .filter_map(|stmt| {
            let functor_name = stmt.template()?;
            let functor = functor_at(owner, &functor_name)?;
            let alias = stmt.alias()?;
            let depends_on = stmt
                .args()
                .iter()
                .filter_map(|argument| argument.syntax().children().next())
                .filter_map(|expression| module_named(&expression))
                .collect();
            Some(Site {
                alias_span: alias_span(stmt.syntax()),
                stmt,
                functor,
                functor_name,
                alias,
                depends_on,
                source: source.map(str::to_owned),
            })
        })
        .collect()
}

/// The order instance sites must be read in: an instance passed to another is
/// read first, wherever the two are written.
///
/// A cycle is refused rather than ordered. Instantiation is finite and
/// acyclic by construction — it happens once, before anything is evaluated,
/// so a functor that consumes what it produces has no base case to start
/// from.
fn order(resolver: &mut Resolver, sites: Vec<Site>) -> Vec<Site> {
    let mut remaining: Vec<Option<Site>> = sites.into_iter().map(Some).collect();
    let mut ordered = Vec::with_capacity(remaining.len());
    let mut placed: IndexSet<String> = IndexSet::new();
    loop {
        let ready: Vec<usize> = remaining
            .iter()
            .enumerate()
            .filter_map(|(index, site)| {
                let site = site.as_ref()?;
                let waiting = site.depends_on.iter().any(|name| {
                    !placed.contains(name)
                        && remaining
                            .iter()
                            .flatten()
                            .any(|other| &other.alias == name && other.alias != site.alias)
                });
                (!waiting).then_some(index)
            })
            .collect();
        if ready.is_empty() {
            break;
        }
        for index in ready {
            if let Some(site) = remaining.get_mut(index).and_then(Option::take) {
                placed.insert(site.alias.clone());
                ordered.push(site);
            }
        }
    }
    for site in remaining.into_iter().flatten() {
        resolver.report(
            Diagnostic::error(
                Code::DependencyCycle,
                format!("`{}` is made from something it makes", site.alias),
            )
            .at(trimmed_span(site.stmt.syntax()), "this instance is part of a cycle")
            .help("a functor's arguments must already exist: instantiation happens once, before anything is evaluated"),
        );
    }
    ordered
}

/// The template module a `make` site names, among the declarations beside it.
fn functor_at(owner: &SyntaxNode, name: &str) -> Option<StructureDecl> {
    TemplateDecl::all_at_root(owner)
        .into_iter()
        .find(|template| template.name().as_deref() == Some(name))
        .and_then(|template| template.structure())
}

/// The module an argument expression names, when it is a bare name.
fn module_named(expression: &SyntaxNode) -> Option<String> {
    if expression.kind() != SyntaxKind::NameExpr {
        return None;
    }
    let text = expression.to_string().trim().to_owned();
    (!text.contains(DOT)).then_some(text)
}

/// The unspellable name a value argument is held under.
///
/// `#` and `/` cannot occur in an identifier, so a holder can never be the
/// name of anything the source declares.
fn holder(alias: &str, parameter: &str) -> String {
    format!("make{UNIT}module/{alias}#{parameter}")
}

fn name_span(node: &SyntaxNode) -> SourceSpan {
    crate::resolve::token_span(node, SyntaxKind::Identifier).unwrap_or_else(|| trimmed_span(node))
}

/// The span of the signature name a module ascribes: the second identifier,
/// because the module writes its own name first.
fn ascription_span(node: &SyntaxNode) -> SourceSpan {
    identifier_spans(node).nth(1).unwrap_or_else(|| trimmed_span(node))
}

/// The span of a `make` site's `as` name, which is likewise the second
/// identifier it writes.
fn alias_span(node: &SyntaxNode) -> SourceSpan {
    identifier_spans(node).nth(1).unwrap_or_else(|| trimmed_span(node))
}

fn identifier_spans(node: &SyntaxNode) -> impl Iterator<Item = SourceSpan> + '_ {
    node.children_with_tokens()
        .filter_map(musa_language::SyntaxElement::into_token)
        .filter(|token| token.kind() == SyntaxKind::Identifier)
        .map(|token| {
            let range = token.text_range();
            SourceSpan::new(u32::from(range.start()), u32::from(range.end()))
        })
}

/// Whether a node is a written type.
///
/// Delegated rather than matched here: a second hand-written list drifts, and
/// this one had — it predated `AppliedType`, so a template parameter written
/// `Duration<WrittenTime>` or `Pair<Nat, Bool>` was not seen as a value
/// parameter at all, and the body's use of it could not find its name.
fn is_type(kind: SyntaxKind) -> bool {
    musa_language::ast::is_type(kind)
}

#[cfg(test)]
mod tests {
    use super::identity_key;

    #[test]
    fn identity_comes_from_the_functor_its_arguments_and_the_site() {
        let base = identity_key("Canon", "MajorCanon", &["module CMajor".to_owned()]);
        assert_eq!(
            base,
            identity_key("Canon", "MajorCanon", &["module CMajor".to_owned()]),
            "the same instance, read twice, is the same instance"
        );
        assert_ne!(
            base,
            identity_key("Canon", "OtherCanon", &["module CMajor".to_owned()]),
            "two sites given equal arguments are two modules"
        );
        assert_ne!(
            base,
            identity_key("Sequence", "MajorCanon", &["module CMajor".to_owned()]),
            "a different functor makes a different thing"
        );
        assert_ne!(
            base,
            identity_key("Canon", "MajorCanon", &["module AMinor".to_owned()]),
            "what a functor was applied to is part of what it made"
        );
    }

    #[test]
    fn identity_reads_no_span_and_no_argument_text() {
        // Everything in the key is a name. Nothing in it moves when the text
        // above the site is edited, which is why an editor may remember a
        // collapsed instance across an edit.
        let key = identity_key("Canon", "MajorCanon", &["module CMajor".to_owned(), "1/2".to_owned()]);
        assert!(key.split('\u{1f}').all(|part| !part.is_empty()));
        assert!(key.contains("module/MajorCanon"));
    }

    #[test]
    fn arguments_are_separated_by_a_character_no_name_can_contain() {
        let together = identity_key("F", "G", &["ab".to_owned()]);
        let apart = identity_key("F", "G", &["a".to_owned(), "b".to_owned()]);
        assert_ne!(together, apart);
    }
}
