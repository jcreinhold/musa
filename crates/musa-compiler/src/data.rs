//! Nominal data a library declares for itself.
//!
//! `docs/rules/language/02-core-calculus.md` §1 admits `N[τ, …]` — *finite,
//! strictly positive* nominal data with one generated fold — and §1.1 says how
//! a declaration group is checked: mutually recursive declarations are
//! grouped, the group is checked once, a stored function field is rejected,
//! and the group is storable data when every field leaving it is. The check
//! terminates because the declaration graph is finite.
//!
//! This module owns that check and the two things a declaration adds to the
//! language: its constructors, and its fold. Everything else a reader does
//! with a value — taking it apart — is `match`, which the checker already had.
//!
//! Nothing here is public. A declaration's identity is *build-local*: it is
//! derived from where the declaration resolved, never from a digest of the
//! text, so editing a comment above a declaration does not make a new type and
//! two packages that both declare `Motive` do not share one.

use indexmap::{IndexMap, IndexSet};
use musa_language::ast::{AstNode as _, DataDecl, DataVariant, StructureDecl};
use musa_language::{SyntaxKind, SyntaxNode};

use crate::core::Type;
use crate::diagnose::{Code, Diagnostic};
use crate::infer::{Kind, Unifier};
use crate::origin::SourceSpan;
use crate::resolve::Resolver;

/// Which declaration a nominal type is.
///
/// The key is `<where it resolved>::<name>`, so two declarations of one name
/// in two structures are two types and the same declaration read twice is one.
/// It is not a hash of the source: a build that reformats a declaration keeps
/// its identity, which is what "build-local" buys and what a digest would
/// throw away.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) struct NominalId(String);

impl NominalId {
    /// The declaration's own name, which is what a diagnostic shows.
    pub(crate) fn name(&self) -> &str {
        self.0.rsplit("::").next().unwrap_or(&self.0)
    }
}

impl std::fmt::Display for NominalId {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        out.write_str(self.name())
    }
}

/// One stored field: a name, and the type it stores.
struct Field {
    name: String,
    /// The written type, with each of the declaration's parameters standing as
    /// `Type::Var(parameter index)`. It is a *template*: nothing reads it
    /// without substituting arguments first, which is what
    /// [`substitute_parameters`] does.
    ty: Type,
    span: SourceSpan,
}

/// One constructor.
struct Variant {
    name: String,
    fields: Vec<Field>,
    span: SourceSpan,
}

/// One `data` declaration, as everything downstream reads it.
struct Declaration {
    id: NominalId,
    /// What its type parameters are called, in order. A parameter's *index*
    /// here is the `Type::Var` a field template writes.
    parameters: Vec<String>,
    variants: Vec<Variant>,
    /// Which mutually recursive group it was checked in.
    group: usize,
    /// The structure whose members may name its constructors, when it was
    /// declared inside one. A declaration at a document's root has none, and
    /// its constructors are nameable wherever its type is.
    owner: Option<String>,
    name_span: SourceSpan,
}

impl Declaration {
    /// The type this declaration makes, applied to `arguments`.
    fn applied(&self, arguments: Vec<Type>) -> Type {
        Type::Nominal(self.id.clone(), arguments)
    }
}

/// `template` with each `Type::Var(i)` replaced by `arguments[i]`.
///
/// A parameter index is not a unifier variable and never meets one: a template
/// is substituted the moment it is read, so the two numbering schemes never
/// share a type.
fn substitute_parameters(template: &Type, arguments: &[Type]) -> Type {
    if let Type::Var(index) = template {
        return arguments
            .get(usize::try_from(*index).unwrap_or(usize::MAX))
            .cloned()
            .unwrap_or_else(|| template.clone());
    }
    crate::infer::rebuilt(template, |member| substitute_parameters(member, arguments))
}

/// Every `data` declaration a checking pass can see.
///
/// Built before anything else is checked, because a declaration is what a
/// written type *means*: `lower_type` cannot read `Motive` until this says
/// what `Motive` is.
#[derive(Default)]
pub(crate) struct World {
    declarations: IndexMap<NominalId, Declaration>,
    /// The name a type is written by, and which declaration it reaches.
    named: IndexMap<String, NominalId>,
    /// A constructor's name, the declaration it belongs to, and which variant
    /// it is.
    constructors: IndexMap<String, (NominalId, usize)>,
    /// A generated fold's name, and the declaration it folds.
    folds: IndexMap<String, NominalId>,
}

/// What a written type name may mean where it is written: the parameters of
/// the declaration being read stand for themselves, and every declaration in
/// the world stands for itself.
pub(crate) struct TypeScope<'a> {
    world: Option<&'a World>,
    parameters: &'a [String],
}

impl TypeScope<'_> {
    /// What `name` denotes here, if anything: a parameter in scope, or a
    /// declaration, applied to `arguments`.
    pub(crate) fn named(&self, name: &str, arguments: Vec<Type>) -> Option<Type> {
        if arguments.is_empty()
            && let Some(index) = self.parameters.iter().position(|parameter| parameter == name)
        {
            return Some(Type::Var(u32::try_from(index).unwrap_or(u32::MAX)));
        }
        let world = self.world?;
        let id = world.named.get(name)?;
        // A declaration is a type only at its own arity: `Pair` alone is not
        // a type the way `Pair<Nat, Bool>` is. Answering `None` here is what
        // sends the caller to the diagnostic that says how many it takes.
        if world.declarations.get(id)?.parameters.len() != arguments.len() {
            return None;
        }
        Some(Type::Nominal(id.clone(), arguments))
    }

    /// How many parameters the declaration `name` reaches takes, for the
    /// diagnostic that says an application is the wrong size.
    pub(crate) fn arity(&self, name: &str) -> Option<usize> {
        let world = self.world?;
        let id = world.named.get(name)?;
        Some(world.declarations.get(id)?.parameters.len())
    }
}

impl World {
    /// The scope in which this world's declarations are types and nothing is a
    /// parameter — how every type outside a `data` declaration is read.
    pub(crate) fn scope(&self) -> TypeScope<'_> {
        TypeScope {
            world: Some(self),
            parameters: &[],
        }
    }

    /// Read every `data` declaration under `owners`, check each group once,
    /// and answer with what the rest of the pass may name.
    ///
    /// `owners` yields the nodes whose children the declarations are: an
    /// imported library, a document's lexical root, the piece being checked.
    /// A declaration written inside a `structure` is found through that
    /// structure, and remembers it — that is what seals its constructors.
    pub(crate) fn read(resolver: &mut Resolver, owners: &[SyntaxNode]) -> Self {
        let mut world = Self::default();
        // Two passes over the same declarations, because a field may name a
        // type declared after it — or the one it is declared in. The first
        // pass learns which names are types; only then can the second read a
        // field's written type at all.
        let mut found = Vec::new();
        for node in owners {
            for (owner, declaration) in data_declarations(node) {
                let Some(name) = declaration.name() else { continue };
                if crate::core::is_builtin_type_name(&name) {
                    resolver.report(
                        Diagnostic::error(Code::DuplicateName, format!("`{name}` is already a type"))
                            .at(name_span(declaration.syntax()), "declared again here")
                            .help("the compiler owns this name, so a declaration of it would leave two readings"),
                    );
                    continue;
                }
                if let Some(first) = world.named.get(&name) {
                    let first = world.declarations.get(first).map(|first| first.name_span);
                    let mut report =
                        Diagnostic::error(Code::DuplicateName, format!("`{name}` is declared twice as a type"))
                            .at(name_span(declaration.syntax()), "declared again here")
                            .help("give one of the declarations a different name");
                    if let Some(first) = first {
                        report = report.also(first, "first declared here");
                    }
                    resolver.report(report);
                    continue;
                }
                let id = NominalId(match &owner {
                    Some(owner) => format!("{owner}::{name}"),
                    None => format!("::{name}"),
                });
                world.named.insert(name, id.clone());
                // A skeleton, so that reading a field's type can already ask
                // how many arguments a declaration takes — including the one
                // being read, which is the ordinary case for a recursive
                // field. The second pass replaces it in place.
                world.declarations.insert(
                    id.clone(),
                    Declaration {
                        id: id.clone(),
                        parameters: declaration.parameters(),
                        variants: Vec::new(),
                        group: 0,
                        owner: owner.clone(),
                        name_span: name_span(declaration.syntax()),
                    },
                );
                found.push((id, owner, declaration));
            }
        }
        let mut built = Vec::new();
        for (id, owner, declaration) in &found {
            let parameters = declaration.parameters();
            let scope = TypeScope {
                world: Some(&world),
                parameters: &parameters,
            };
            let variants = declaration
                .variants()
                .iter()
                .map(|variant| read_variant(resolver, &scope, variant))
                .collect::<Vec<_>>();
            built.push(Declaration {
                id: id.clone(),
                parameters,
                variants,
                group: 0,
                owner: owner.clone(),
                name_span: name_span(declaration.syntax()),
            });
        }
        for declaration in built {
            world.declarations.insert(declaration.id.clone(), declaration);
        }
        world.group();
        world.index_operations(resolver);
        world.check(resolver);
        world
    }

    /// Number the mutually recursive groups: two declarations are in one group
    /// when each reaches the other through its fields.
    ///
    /// The relation is "reaches", closed over a finite set — the same
    /// finiteness §1.1 says the check terminates by.
    fn group(&mut self) {
        let ids: Vec<NominalId> = self.declarations.keys().cloned().collect();
        let mut reaches: IndexMap<NominalId, IndexSet<NominalId>> = IndexMap::new();
        for id in &ids {
            let mut direct = IndexSet::new();
            if let Some(declaration) = self.declarations.get(id) {
                for variant in &declaration.variants {
                    for field in &variant.fields {
                        mentioned(&field.ty, &mut direct);
                    }
                }
            }
            reaches.insert(id.clone(), direct);
        }
        // Transitive closure by repetition. Each round that changes anything
        // adds at least one edge to a graph with a finite number of them, so
        // this stops — which is the termination §1.1 asks for.
        loop {
            let mut grew = false;
            for id in &ids {
                let mut widened = reaches.get(id).cloned().unwrap_or_default();
                for reached in reaches.get(id).cloned().unwrap_or_default() {
                    for further in reaches.get(&reached).cloned().unwrap_or_default() {
                        grew |= widened.insert(further);
                    }
                }
                reaches.insert(id.clone(), widened);
            }
            if !grew {
                break;
            }
        }
        let mut assigned: IndexMap<NominalId, usize> = IndexMap::new();
        let mut next = 0usize;
        for id in &ids {
            if assigned.contains_key(id) {
                continue;
            }
            let group = next;
            next = next.saturating_add(1);
            assigned.insert(id.clone(), group);
            for other in &ids {
                let mutual = reaches.get(id).is_some_and(|set| set.contains(other))
                    && reaches.get(other).is_some_and(|set| set.contains(id));
                if mutual {
                    assigned.insert(other.clone(), group);
                }
            }
        }
        for (id, group) in assigned {
            if let Some(declaration) = self.declarations.get_mut(&id) {
                declaration.group = group;
            }
        }
    }

    /// File every constructor and every generated fold under the name it is
    /// written by, reporting the collisions that makes visible.
    fn index_operations(&mut self, resolver: &mut Resolver) {
        let mut constructors: IndexMap<String, (NominalId, usize)> = IndexMap::new();
        let mut folds = IndexMap::new();
        for declaration in self.declarations.values() {
            for (index, variant) in declaration.variants.iter().enumerate() {
                if constructors
                    .insert(variant.name.clone(), (declaration.id.clone(), index))
                    .is_some()
                {
                    resolver.report(
                        Diagnostic::error(
                            Code::DuplicateName,
                            format!("`{}` is a constructor of two declarations", variant.name),
                        )
                        .at(variant.span, "declared again here")
                        .help("a constructor is named once, because a use names it and nothing else"),
                    );
                }
            }
            folds.insert(fold_name(declaration.id.name()), declaration.id.clone());
        }
        self.constructors = constructors;
        self.folds = folds;
    }

    /// §1.1's group check: no declaration may stand to the left of an arrow in
    /// its own group, and no field may store a function.
    ///
    /// Positivity is asked first because it is the more precise of the two
    /// answers — a non-positive field is also a field storing a function, and
    /// being told only the second would send the writer looking for the wrong
    /// repair. Both are reported *at the field*, because the field is what has
    /// to change; a diagnostic at the group would name a set the source never
    /// wrote down.
    ///
    /// What is not here is a separate storable-data check. Every type but an
    /// arrow is storable data, an arrow is refused right here, and so a group
    /// that survives this is storable data by construction.
    fn check(&self, resolver: &mut Resolver) {
        for declaration in self.declarations.values() {
            let group: IndexSet<NominalId> = self
                .declarations
                .values()
                .filter(|other| other.group == declaration.group)
                .map(|other| other.id.clone())
                .collect();
            for variant in &declaration.variants {
                for field in &variant.fields {
                    if let Some(offender) = negative_occurrence(&field.ty, &group) {
                        resolver.report(
                            Diagnostic::error(
                                Code::TypeMismatch,
                                format!("`{offender}` stands to the left of an arrow in its own declaration"),
                            )
                            .at(field.span, format!("this field takes a `{offender}`"))
                            .note(
                                "a declaration is finite and strictly positive, so a value of it cannot be an \
                                 argument to a function stored inside it",
                            ),
                        );
                        continue;
                    }
                    if holds_function(&field.ty) {
                        resolver.report(
                            Diagnostic::error(Code::TypeMismatch, "a stored field may not be a function")
                                .at(field.span, format!("`{}` stores a function", field.name))
                                .note(
                                    "an arrow is never storable data, and neither is anything holding one \
                                     (`docs/rules/language/02-core-calculus.md` §1.1)",
                                ),
                        );
                    }
                }
            }
        }
    }

    /// What a constructor's name means: which variant it is, what its fields
    /// take, and the type it makes.
    ///
    /// `owner` is the structure the use is written in, so that a constructor
    /// declared inside a structure is nameable by that structure's own members
    /// and nowhere else. That is what makes a constructor private: outside,
    /// the signature may still export the *type*, and the way in is gone.
    pub(crate) fn constructor(&self, name: &str, owner: Option<&str>, unifier: &mut Unifier) -> Option<Constructing> {
        let (id, variant) = self.constructors.get(name)?;
        let declaration = self.declarations.get(id)?;
        if let Some(declared_in) = &declaration.owner
            && Some(declared_in.as_str()) != owner
        {
            return Some(Constructing::Sealed {
                declared_in: declared_in.clone(),
                ty: declaration.id.name().to_owned(),
                declared_at: declaration.name_span,
            });
        }
        let arguments = fresh_arguments(declaration, unifier);
        let found = declaration.variants.get(*variant)?;
        Some(Constructing::Found {
            id: id.clone(),
            variant: *variant,
            fields: found
                .fields
                .iter()
                .map(|field| (field.name.clone(), substitute_parameters(&field.ty, &arguments)))
                .collect(),
            arguments: arguments.clone(),
            result: declaration.applied(arguments),
        })
    }

    /// Whether `name` is a constructor of any declaration, whatever it is
    /// visible from. A use of a sealed constructor has to be told what it
    /// reached, not that the name means nothing.
    pub(crate) fn is_constructor(&self, name: &str) -> bool {
        self.constructors.contains_key(name)
    }

    /// The type of the fold `name` names, if it names one.
    ///
    /// One fold per declaration, and its cases cover the whole group: for a
    /// declaration that reaches another and is reached back, a traversal that
    /// stopped at the group boundary would not be a traversal. With one
    /// declaration in the group — every declaration that is not mutually
    /// recursive — the shape degenerates to one case per constructor plus the
    /// value, which is exactly `nat_fold`'s and `list_fold`'s shape.
    pub(crate) fn fold(&self, name: &str, unifier: &mut Unifier) -> Option<Folding> {
        let id = self.folds.get(name)?;
        let declaration = self.declarations.get(id)?;
        let group: Vec<&Declaration> = self
            .declarations
            .values()
            .filter(|other| other.group == declaration.group)
            .collect();
        let arguments = fresh_arguments(declaration, unifier);
        // One result type per group member, so that a case answering "what the
        // fold made of a `Stmt`" and a case answering "what it made of an
        // `Expr`" are answering different questions.
        let results: IndexMap<NominalId, Type> = group
            .iter()
            .map(|member| (member.id.clone(), unifier.fresh(Kind::Ordinary)))
            .collect();
        let mut cases = Vec::new();
        let mut shape = Vec::new();
        for member in &group {
            for (index, variant) in member.variants.iter().enumerate() {
                let parameters = variant
                    .fields
                    .iter()
                    .map(|field| folded(&substitute_parameters(&field.ty, &arguments), &results))
                    .collect::<Vec<_>>();
                let result = results.get(&member.id)?.clone();
                // A constructor holding nothing takes the *answer*, not a
                // function returning it: that is `nat_fold`'s `zero` and
                // `list_fold`'s `start`, and a nullary arrow here would make
                // a reader who knows those write something else.
                cases.push(if parameters.is_empty() {
                    result
                } else {
                    Type::Function(parameters, Box::new(result))
                });
                shape.push((member.id.clone(), index));
            }
        }
        let result = results.get(&declaration.id)?.clone();
        let mut parameters = cases;
        parameters.push(declaration.applied(arguments));
        Some(Folding {
            cases: shape,
            ty: Type::Function(parameters, Box::new(result)),
        })
    }

    /// Whether `name` names a generated fold.
    pub(crate) fn is_fold(&self, name: &str) -> bool {
        self.folds.contains_key(name)
    }

    /// A declaration's constructors, in declaration order, with how many
    /// fields each takes — what an exhaustiveness check reads.
    pub(crate) fn variants(&self, id: &NominalId) -> Vec<(String, usize)> {
        self.declarations
            .get(id)
            .map(|declaration| {
                declaration
                    .variants
                    .iter()
                    .map(|variant| (variant.name.clone(), variant.fields.len()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// One variant's fields, instantiated at `arguments` — what a pattern
    /// binds and what a fold hands its case.
    pub(crate) fn fields(&self, id: &NominalId, variant: usize, arguments: &[Type]) -> Vec<(String, Type)> {
        let Some(declaration) = self.declarations.get(id) else {
            return Vec::new();
        };
        let Some(found) = declaration.variants.get(variant) else {
            return Vec::new();
        };
        found
            .fields
            .iter()
            .map(|field| (field.name.clone(), substitute_parameters(&field.ty, arguments)))
            .collect()
    }

    /// Whether the structure `owner` declares a type called `name`.
    ///
    /// This is what a signature's `data Name;` asks: the signature says the
    /// type exists and says nothing about its constructors, and the
    /// constructors are the structure's own by [`Self::constructor`]. Matching
    /// is by name, as everything else in `04-templates-and-modules.md` §4 is.
    pub(crate) fn declared_by(&self, owner: &str, name: &str) -> bool {
        self.declarations
            .values()
            .any(|declaration| declaration.owner.as_deref() == Some(owner) && declaration.id.name() == name)
    }

    /// Which declaration a constructor belongs to, and which variant it is —
    /// what a pattern needs in order to say which case it covers.
    pub(crate) fn constructor_of(&self, id: &NominalId, name: &str) -> Option<usize> {
        let declaration = self.declarations.get(id)?;
        declaration.variants.iter().position(|variant| variant.name == name)
    }
}

/// What a constructor's name was found to mean.
pub(crate) enum Constructing {
    /// It names a constructor the use may write.
    Found {
        id: NominalId,
        variant: usize,
        fields: Vec<(String, Type)>,
        /// The type arguments this use makes it at.
        arguments: Vec<Type>,
        result: Type,
    },
    /// It names a constructor of a type declared inside a structure, read from
    /// outside it. The type may well be nameable there; the way in is not.
    Sealed {
        declared_in: String,
        ty: String,
        declared_at: SourceSpan,
    },
}

/// A generated fold: which constructor each of its cases answers for, in
/// order, and the fold's own type.
pub(crate) struct Folding {
    pub(crate) cases: Vec<(NominalId, usize)>,
    pub(crate) ty: Type,
}

/// The name of the fold a declaration generates: `motive_fold` for `Motive`,
/// `chord_shape_fold` for `ChordShape`.
///
/// The shape is `nat_fold`, `list_fold`, `option_fold` — the eliminators the
/// language already had — because a generated fold *is* one of those, and a
/// reader who knows one should not have to learn a second convention.
pub(crate) fn fold_name(ty: &str) -> String {
    let mut out = String::with_capacity(ty.len().saturating_add(6));
    for (index, character) in ty.chars().enumerate() {
        if character.is_uppercase() && index > 0 {
            out.push('_');
        }
        out.extend(character.to_lowercase());
    }
    out.push_str("_fold");
    out
}

/// One fresh type argument per parameter: one instantiation per use.
fn fresh_arguments(declaration: &Declaration, unifier: &mut Unifier) -> Vec<Type> {
    declaration
        .parameters
        .iter()
        .map(|_| unifier.fresh(Kind::Ordinary))
        .collect()
}

/// A field's type as the fold's case sees it: a direct occurrence of a group
/// member is what the fold already made of it, and everything else is itself.
///
/// Direct, and not under a container: a fold replaces *one constructor layer*,
/// so a `List<Tree>` field arrives as the list it is and the writer maps over
/// it. Reaching inside containers would be a deriving mechanism, which this
/// prompt's **Stop** forbids and which no reader could then override.
fn folded(ty: &Type, results: &IndexMap<NominalId, Type>) -> Type {
    if let Type::Nominal(id, _) = ty {
        return results.get(id).cloned().unwrap_or_else(|| ty.clone());
    }
    ty.clone()
}

/// Every declaration a type mentions, at any depth.
fn mentioned(ty: &Type, out: &mut IndexSet<NominalId>) {
    if let Type::Nominal(id, _) = ty {
        out.insert(id.clone());
    }
    for member in crate::infer::member_types(ty) {
        mentioned(member, out);
    }
}

/// The first group member standing to the left of an arrow, if any.
fn negative_occurrence(ty: &Type, group: &IndexSet<NominalId>) -> Option<String> {
    if let Type::Function(parameters, result) = ty {
        let mut mentions = IndexSet::new();
        for parameter in parameters {
            mentioned(parameter, &mut mentions);
        }
        return mentions
            .iter()
            .find(|id| group.contains(*id))
            .map(|id| id.name().to_owned())
            .or_else(|| negative_occurrence(result, group));
    }
    crate::infer::member_types(ty)
        .into_iter()
        .find_map(|member| negative_occurrence(member, group))
}

/// Whether an arrow appears anywhere inside a type.
fn holds_function(ty: &Type) -> bool {
    matches!(ty, Type::Function(_, _)) || crate::infer::member_types(ty).into_iter().any(holds_function)
}

/// Every `data` declaration `owner` holds, each with the structure that owns
/// it when it is written inside one.
///
/// One level down, and not a walk of descendants: the grammar admits a `data`
/// declaration at a document's root, in a library, in a piece, and in a
/// structure, and nowhere else.
fn data_declarations(owner: &SyntaxNode) -> Vec<(Option<String>, DataDecl)> {
    let mut found: Vec<_> = DataDecl::all_at_root(owner)
        .into_iter()
        .map(|declaration| (None, declaration))
        .collect();
    for child in owner.children() {
        if child.kind() != SyntaxKind::StructureDecl {
            continue;
        }
        let structure = StructureDecl::cast(child.clone()).and_then(|structure| structure.name());
        found.extend(
            DataDecl::all_at_root(&child)
                .into_iter()
                .map(|declaration| (structure.clone(), declaration)),
        );
    }
    found
}

fn read_variant(resolver: &mut Resolver, scope: &TypeScope<'_>, variant: &DataVariant) -> Variant {
    let fields = variant
        .fields()
        .iter()
        .filter_map(|field| {
            let name = field.name()?;
            let node = field.ty()?;
            let ty = crate::core::scoped_type(resolver, scope, &node)?;
            Some(Field {
                name,
                ty,
                span: crate::resolve::trimmed_span(field.syntax()),
            })
        })
        .collect();
    Variant {
        name: variant.name().unwrap_or_default(),
        fields,
        span: crate::resolve::trimmed_span(variant.syntax()),
    }
}

/// Where a declaration's name is written.
fn name_span(node: &SyntaxNode) -> SourceSpan {
    crate::resolve::token_span(node, SyntaxKind::Identifier).unwrap_or_else(|| crate::resolve::trimmed_span(node))
}
