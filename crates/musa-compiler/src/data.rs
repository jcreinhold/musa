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




impl Declaration {
}




impl TypeScope<'_> {

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
            phase: self.phase,
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
        Self::read_with(resolver, owners, false)
    }

    fn read_with(resolver: &mut Resolver, owners: &[SyntaxNode], phase: bool) -> Self {
        let mut world = Self {
            phase,
            ..Self::default()
        };
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
                phase: world.phase,
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
    /// arrow and a sealed step is storable data, both are refused right here,
    /// and so a group that survives this is storable data by construction.
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
                    // A sealed step is refused for a stronger reason than an
                    // arrow's, and so says so separately: it *holds* the
                    // algebra of the recursor that minted it, which is four
                    // source closures and a child nobody else may name. Stored
                    // in a declaration it would outlive the traversal that
                    // sealed it, which is exactly what sealing is for.
                    if holds_sealed_step(&field.ty) {
                        resolver.report(
                            Diagnostic::error(Code::TypeMismatch, "a stored field may not be a sealed step")
                                .at(field.span, format!("`{}` stores a suspended descent", field.name))
                                .note(
                                    "a `SyntaxStep<C, A>` holds the algebra it was minted under, so it is never                                      storable data and is excluded from `d`                                      (`docs/rules/language/02-core-calculus.md` §5.9)",
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
    /// value, which is exactly `nat_fold`'s and `list_fold_from_end`'s shape —
    /// the catamorphisms, which is what a generated fold is.
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
                // a list fold's `start`, and a nullary arrow here would make
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

    /// Where a declaration's name is written — what a diagnostic about one of
    /// its fields points at second, so a reader can see the field list it was
    /// judged against without going to look for it.
    pub(crate) fn declared_at(&self, id: &NominalId) -> Option<SourceSpan> {
        self.declarations.get(id).map(|declaration| declaration.name_span)
    }

    /// Which declaration a constructor belongs to, and which variant it is —
    /// what a pattern needs in order to say which case it covers.
    pub(crate) fn constructor_of(&self, id: &NominalId, name: &str) -> Option<usize> {
        let declaration = self.declarations.get(id)?;
        declaration.variants.iter().position(|variant| variant.name == name)
    }
}



/// The name of the fold a declaration generates: `motive_fold` for `Motive`,
/// `chord_shape_fold` for `ChordShape`.
///
/// The shape is `nat_fold`, `option_fold`, `list_fold_from_end` — the
/// catamorphisms the language already had — because a generated fold *is* one
/// of those, and a reader who knows one should not have to learn a second
/// convention. A generated fold has no direction to name: a case sees its
/// group-member fields already folded, one constructor layer at a time, which
/// is what "replaces one constructor layer" means. Only `list` has two names,
/// because only `list` nests its constructors against its element order.
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









