//! A written declaration read as a raw core declaration.
//!
//! The other half of this module, and the boundary the module documentation
//! names: reading a *declaration* and reading an *expression* share the CST walk
//! and the site table and nothing else, so they are two files under one
//! [`Lowering`] rather than two modules. Everything below reaches
//! [`Lowering::ty`] and [`Lowering::expr`] and adds only the shapes
//! `02-core-calculus.md` gives a declaration.
//!
//! # Which surface declaration becomes which raw one
//!
//! Four of the five words the surface declares with produce three core shapes,
//! and the odd one out is the interesting one:
//!
//! - **`data` and `enum` are nominal**, so each becomes a [`RawData`] of one
//!   family. `01-surface.md` §1.3: "each declaration generates its own family".
//!   The difference between the two words is entirely in how a case writes its
//!   fields — `data` names every one, an enum's positional case names none —
//!   and not in what either denotes.
//! - **`record` is structural**, so it becomes a *definition* whose value is a
//!   core record type. §1.2: "a record is its fields", and two declarations with
//!   the same fields at the same types denote the same type. A `RawData` here
//!   would have made `record` nominal by construction and quietly contradicted
//!   the sentence the whole feature rests on.
//! - **`trait` and `impl`** become [`RawTrait`] and [`RawImpl`]. An `impl` is
//!   *always* the latter, including the inherent `impl Duration { … }` form,
//!   because §6 decides which one a block is "by whether its head name resolves
//!   to a trait or to a type" — a resolution question, and resolution is the
//!   core's.
//! - **`let` and `fn`** become a [`Definition`]: a name, the type the
//!   declaration wrote, and a value.
//!
//! # Type parameters, and why their plicity differs
//!
//! A declaration's `<A>` is an explicit binder when the surface writes the
//! argument and an implicit one when it does not, which is one rule and two
//! answers:
//!
//! - A **type** declaration's parameter is explicit. `List<Nat>` lowers to
//!   `List Nat` in [`Lowering::ty`], so `List` has to be a function of one
//!   ordinary argument, and `prelude.rs` already declares the compiler's own
//!   families that way.
//! - A **function**'s parameter is implicit. `same(x, y)` writes no type
//!   argument, so `same` quantifies implicitly and the core fills the binder
//!   with a metavariable per use.
//!
//! Both stand at `Type 0`, which is the level a written signature lands in —
//! §1 says the surface never writes a level, and every type the surface *can*
//! write is small.

use std::sync::Arc;

use musa_core::{
    Level, Name, Origin, Raw, RawBinder, RawConstraint, RawConstructor, RawData, RawDefinition, RawFamily, RawImpl,
    RawMethod, RawTrait, Visibility,
};
use musa_language::{SyntaxKind, SyntaxNode};

use super::types::compiler_type;
use super::values::position_field;
use super::{Lowering, child, children, is_expr_node, is_type_node, own_tokens, writes};
use crate::diagnose::{Code, Diagnostic};
use crate::resolve::trimmed_span;

/// One declaration, read.
///
/// Four variants because `musa-core` has four doors — [`musa_core::declare`],
/// [`musa_core::declare_trait`], [`musa_core::declare_impl`], and
/// [`musa_core::check`] — and this is the type that says which one a written
/// declaration goes through. A caller matches once and calls; it never has to
/// ask what word the source used.
#[derive(Debug)]
pub(crate) enum Item {
    /// A `data` or an `enum`.
    Data(RawData),
    /// A `trait`.
    Class(RawTrait),
    /// An `impl`, whether it implements a trait or opens a type's namespace.
    Instance(RawImpl),
    /// A `let`, a `fn`, or a `record`.
    Definition(Definition),
}

/// A name, the type its declaration wrote, and its value.
///
/// # Why the type is optional
///
/// Because the surface makes it optional. `01-surface.md` §1's `binding` writes
/// `(":" type)?` and its `param` does too, so a `let` with no annotation and a
/// `fn` with an unannotated parameter are both ordinary programs — and there is
/// no raw term for a type nobody wrote, since a hole is a thing the *core*
/// mints and not a thing a reading may hand it. When the type is [`None`] the
/// value still carries every annotation the author did write, so nothing is
/// lost: what changes is which of [`musa_core::check`] and [`musa_core::infer`]
/// a caller uses.
///
/// # What is deliberately not here
///
/// **Visibility.** `musa-core` gives a family, a constructor, and a trait one
/// because it filters names by module (`136a`), and gives a definition none.
/// Carrying a marker this type's consumers cannot pass on would be a field
/// answering a question nobody asked; [`crate::resolve`] reads `private` off the
/// declaration node, which is the node a caller already has in hand when it
/// asks for the item.
#[derive(Debug)]
pub(crate) struct Definition {
    /// Where the declaration was written.
    pub(crate) origin: Origin,
    /// The name it binds.
    pub(crate) name: Name,
    /// The type it wrote, when it wrote one.
    pub(crate) ty: Option<Raw>,
    /// Its value.
    pub(crate) value: Raw,
}

impl Lowering<'_> {
    /// The item a declaration node denotes, or [`None`] with a diagnostic
    /// reported at whatever could not be read.
    ///
    /// [`None`] *without* a diagnostic for a node that is not a declaration at
    /// all, which is how a caller walking a file's children asks "is this one of
    /// mine": a `piece`, a `library`, a `structure`, and an `import` are
    /// containers and statements that a pass reads for itself, and answering
    /// with a complaint would make walking a document an error.
    pub(crate) fn item(&mut self, node: &SyntaxNode) -> Option<Item> {
        match node.kind() {
            SyntaxKind::DataDecl => self.nominal(node).map(Item::Data),
            SyntaxKind::EnumDecl => self.enumeration(node).map(Item::Data),
            SyntaxKind::RecordDecl => self.structural(node).map(Item::Definition),
            SyntaxKind::TraitDecl => self.class(node).map(Item::Class),
            SyntaxKind::ImplDecl => self.instance(node).map(Item::Instance),
            SyntaxKind::FnDecl => self.function(node).map(Item::Definition),
            SyntaxKind::LetDecl => self.binding(node).map(Item::Definition),
            _ => None,
        }
    }

    // ---- the nominal declarations ----

    /// `data Motive<A> { Silence, Sounded(pitch: Pitch, held: Duration<C>) }`.
    ///
    /// One family, no indices, and every constructor at the declaration's own
    /// visibility. The last of those is the grammar's doing rather than a
    /// choice: `data` admits no marker on a variant, and
    /// [`RawConstructor::visibility`] says a family whose cases disagree is
    /// refused at its declaration — so taking the declaration's is the one
    /// reading that cannot disagree with it.
    fn nominal(&mut self, node: &SyntaxNode) -> Option<RawData> {
        let origin = self.origin(node);
        let visibility = visibility_of(node);
        let name = declared_name(node)?;
        let params = self.type_parameters(node);
        let mut constructors = Vec::new();
        for written in children(node, |kind| kind == SyntaxKind::DataVariant) {
            constructors.push(self.variant(&written, visibility)?);
        }
        Some(RawData {
            origin,
            params,
            families: vec![RawFamily {
                name,
                visibility,
                indices: Vec::new(),
                constructors,
            }],
        })
    }

    /// One `data` constructor, whose fields are all named.
    fn variant(&mut self, node: &SyntaxNode, visibility: Visibility) -> Option<RawConstructor> {
        let origin = self.origin(node);
        let name = declared_name(node)?;
        let mut fields = Vec::new();
        for written in children(node, |kind| kind == SyntaxKind::DataField) {
            fields.push(self.declared_field(&written)?);
        }
        Some(RawConstructor {
            origin,
            name,
            visibility,
            fields,
            indices: Vec::new(),
        })
    }

    /// `enum Reading<A> { Done(A), private Refused { at: NodePath, why: Text } }`.
    ///
    /// The same shape as [`Lowering::nominal`] and a different visibility rule:
    /// a case writes its own marker, so `01-surface.md` §1.3's public type with
    /// private cases is a thing an enum can say and a `data` cannot.
    fn enumeration(&mut self, node: &SyntaxNode) -> Option<RawData> {
        let origin = self.origin(node);
        let name = declared_name(node)?;
        self.unconstrained(node)?;
        let params = self.type_parameters(node);
        let mut constructors = Vec::new();
        for written in children(node, |kind| kind == SyntaxKind::EnumCase) {
            constructors.push(self.case(&written)?);
        }
        Some(RawData {
            origin,
            params,
            families: vec![RawFamily {
                name,
                visibility: visibility_of(node),
                indices: Vec::new(),
                constructors,
            }],
        })
    }

    /// One enum case: empty, positional, or named.
    ///
    /// A positional case's fields are named `_0`, `_1`, … — the same spelling a
    /// written product gets, and for the same reason. §1.3 says the positional
    /// form "names types and not fields", so the names exist only to be
    /// distinct, and a leading underscore is not an identifier start.
    fn case(&mut self, node: &SyntaxNode) -> Option<RawConstructor> {
        let origin = self.origin(node);
        let name = declared_name(node)?;
        let mut fields = Vec::new();
        for (index, written) in children(node, is_type_node).iter().enumerate() {
            fields.push(RawBinder {
                name: Arc::from(position_field(index).as_str()),
                ty: self.ty(written)?,
            });
        }
        for written in children(node, |kind| kind == SyntaxKind::FieldDecl) {
            fields.push(self.declared_field(&written)?);
        }
        Some(RawConstructor {
            origin,
            name,
            visibility: visibility_of(node),
            fields,
            indices: Vec::new(),
        })
    }

    // ---- the structural declaration ----

    /// `record Pending<A> { read: Reading; taken: A; }`.
    ///
    /// A definition rather than a declaration, because §1.2 makes a record its
    /// fields: `Pending` *is* `{ read : Reading, taken : A }`, so what the
    /// surface declared is a name for a type — and for a parameterized record, a
    /// function to one. The declared name still stands in every diagnostic,
    /// which is what §1.2 asks of the arrangement, because the core resolves
    /// `Pending` by name before it unfolds it.
    fn structural(&mut self, node: &SyntaxNode) -> Option<Definition> {
        let origin = self.origin(node);
        let name = declared_name(node)?;
        self.unconstrained(node)?;
        let params = self.type_parameters(node);
        let mut names = Vec::new();
        let mut types = Vec::new();
        for written in children(node, |kind| kind == SyntaxKind::FieldDecl) {
            let field = self.declared_field(&written)?;
            names.push(field.name);
            types.push(field.ty);
        }
        let mut value = Raw::record_type(origin, names.iter().map(|name| &**name).zip(types));
        let mut ty = Raw::universe(origin, Level::ZERO);
        for parameter in params.iter().rev() {
            value = Raw::lam(origin, Arc::clone(&parameter.name), value);
            ty = Raw::pi(origin, Arc::clone(&parameter.name), parameter.ty.clone(), ty);
        }
        Some(Definition {
            origin,
            name,
            ty: Some(ty),
            value,
        })
    }

    // ---- traits and instances ----

    /// `trait Ord<A> where Eq<A> { fn less(x: A, y: A) -> Bool; }`.
    fn class(&mut self, node: &SyntaxNode) -> Option<RawTrait> {
        let origin = self.origin(node);
        let name = declared_name(node)?;
        let params = self.type_parameters(node);
        let context = self.written_constraints(node)?;
        let mut methods = Vec::new();
        for written in children(node, |kind| kind == SyntaxKind::FnDecl) {
            methods.push(self.method(&written)?);
        }
        Some(RawTrait {
            origin,
            name,
            visibility: visibility_of(node),
            params,
            context,
            methods,
        })
    }

    /// One method of a trait, required or derived.
    ///
    /// Its type is assembled here and an `impl`'s is not, and the asymmetry is
    /// [`RawMethod`]'s own: a trait *declares* a method and so writes its type,
    /// while an impl *defines* one and takes the type from the dictionary field
    /// it fills. So this is the one place a written signature has to become a Π,
    /// and the one place an unwritten parameter type is fatal rather than
    /// optional — a dictionary field whose type had a hole in it would be a hole
    /// every instance inherited.
    fn method(&mut self, node: &SyntaxNode) -> Option<RawMethod> {
        let origin = self.origin(node);
        let name = declared_name(node)?;
        let params = self.type_parameters(node);
        let context = self.written_constraints(node)?;
        let Some(result) = child(node, is_type_node) else {
            return self.refuse(
                Diagnostic::error(Code::UnsolvedMetavariable, "this method writes no result type")
                    .at(trimmed_span(node), "declared here")
                    .help("a trait's method is a field of its dictionary, so its type is written in full"),
            );
        };
        let mut ty = self.ty(&result)?;
        // Innermost binder first, which is the order both folds below want: the
        // Π is built from its codomain outwards, and the λ from its body.
        let mut bound: Vec<(Origin, Name)> = Vec::new();
        for parameter in written_parameters(node).iter().rev() {
            let at = self.origin(parameter);
            let name = declared_name(parameter)?;
            let Some(written) = child(parameter, is_type_node) else {
                return self.refuse(
                    Diagnostic::error(Code::UnsolvedMetavariable, "this parameter writes no type")
                        .at(trimmed_span(parameter), "declared here")
                        .help("a trait's method is a field of its dictionary, so its type is written in full"),
                );
            };
            ty = Raw::pi(at, Arc::clone(&name), self.ty(&written)?, ty);
            bound.push((at, name));
        }
        let body = match child(node, is_expr_node) {
            Some(block) => {
                let mut built = self.expr(&block)?;
                for (at, name) in &bound {
                    built = Raw::lam(*at, Arc::clone(name), built);
                }
                Some(built)
            }
            None => None,
        };
        Some(RawMethod {
            origin,
            name,
            params,
            context,
            ty,
            body,
        })
    }

    /// `impl<A> Eq<List<A>> where Eq<A> { fn equal(x, y) { … } }`, and
    /// `impl Duration { … }`.
    ///
    /// One reading for both, because the surface is one declaration and §6 says
    /// which it is by resolving the head name. A reading that guessed would be
    /// guessing at exactly the fact the core holds.
    fn instance(&mut self, node: &SyntaxNode) -> Option<RawImpl> {
        let origin = self.origin(node);
        let params = self.type_parameters(node);
        let head = child(node, is_type_node)?;
        let (name, args) = self.head_and_arguments(&head)?;
        let context = self.written_constraints(node)?;
        let mut methods = Vec::new();
        for written in children(node, |kind| kind == SyntaxKind::FnDecl) {
            let at = self.origin(&written);
            methods.push(RawDefinition {
                origin: at,
                name: declared_name(&written)?,
                // The annotations stay on the λ here, because a
                // [`RawDefinition`] has nowhere else to put them: the type is
                // the trait's, and what the author wrote about a parameter is
                // checked against it rather than dropped.
                value: self.lambda(&written, at)?,
            });
        }
        Some(RawImpl {
            origin,
            name,
            params,
            args,
            context,
            methods,
        })
    }

    // ---- the definitions ----

    /// `fn same<A>(x: A, y: A) -> Bool { x == y }`.
    ///
    /// # Why the two branches
    ///
    /// A written type is lowered **once**, whichever branch it lands in. When
    /// the declaration wrote a type for every parameter and for its result, the
    /// signature becomes the Π and the λ binds without repeating it; when it did
    /// not, there is no Π to write and the annotations stay where the author put
    /// them. Reading the declaration twice — once for a type and once for a
    /// value — would report a bad parameter type twice and leave the core
    /// converting each written domain against itself.
    ///
    /// The implicit binders are on the type only. `02-core-calculus.md` §2 wraps
    /// a term checked against an implicit Π in the λ it needs, so writing one
    /// here would be writing what the core is about to write.
    fn function(&mut self, node: &SyntaxNode) -> Option<Definition> {
        let origin = self.origin(node);
        let name = declared_name(node)?;
        self.unconstrained(node)?;
        let parameters = written_parameters(node);
        let signed = child(node, is_type_node).is_some()
            && parameters
                .iter()
                .all(|parameter| child(parameter, is_type_node).is_some());
        if !signed {
            return Some(Definition {
                origin,
                name,
                ty: None,
                value: self.lambda(node, origin)?,
            });
        }
        let result = child(node, is_type_node)?;
        let mut ty = self.ty(&result)?;
        let mut value = self.expr(&child(node, is_expr_node)?)?;
        for parameter in parameters.iter().rev() {
            let at = self.origin(parameter);
            let bound = declared_name(parameter)?;
            let written = child(parameter, is_type_node)?;
            ty = Raw::pi(at, Arc::clone(&bound), self.ty(&written)?, ty);
            value = Raw::lam(at, bound, value);
        }
        for parameter in self.type_parameters(node).iter().rev() {
            ty = Raw::implicit_pi(origin, Arc::clone(&parameter.name), parameter.ty.clone(), ty);
        }
        Some(Definition {
            origin,
            name,
            ty: Some(ty),
            value,
        })
    }

    /// `let tonic: Key = key c major;`.
    fn binding(&mut self, node: &SyntaxNode) -> Option<Definition> {
        let origin = self.origin(node);
        let name = declared_name(node)?;
        let ty = match child(node, is_type_node) {
            Some(written) => Some(self.ty(&written)?),
            None => None,
        };
        Some(Definition {
            origin,
            name,
            ty,
            value: self.expr(&child(node, is_expr_node)?)?,
        })
    }

    // ---- the parts a declaration is made of ----

    /// `name: τ;` — one declared field of a record, an enum case, or a `data`
    /// constructor.
    fn declared_field(&mut self, node: &SyntaxNode) -> Option<RawBinder> {
        let name = declared_name(node)?;
        let written = child(node, is_type_node)?;
        Some(RawBinder {
            name,
            ty: self.ty(&written)?,
        })
    }

    /// `<A, B>` — the parameters a declaration abstracts over, each at `Type 0`.
    ///
    /// The plicity is not decided here, because a [`RawBinder`] does not carry
    /// one: the caller writes them into a Π at the plicity its own declaration
    /// wants, and [`RawTrait`], [`RawImpl`], and [`RawMethod`] leave the choice
    /// to `musa-core`.
    fn type_parameters(&mut self, node: &SyntaxNode) -> Vec<RawBinder> {
        let Some(list) = child(node, |kind| kind == SyntaxKind::TypeParams) else {
            return Vec::new();
        };
        children(&list, |kind| kind == SyntaxKind::TypeParam)
            .iter()
            .filter_map(|written| {
                let at = self.origin(written);
                Some(RawBinder {
                    name: declared_name(written)?,
                    ty: Raw::universe(at, Level::ZERO),
                })
            })
            .collect()
    }

    /// `where Eq<A>, Ord<B>` — the constraints a trait, a method, or an `impl`
    /// wrote.
    fn written_constraints(&mut self, node: &SyntaxNode) -> Option<Vec<RawConstraint>> {
        let Some(clause) = child(node, |kind| kind == SyntaxKind::WhereClause) else {
            return Some(Vec::new());
        };
        children(&clause, |kind| kind == SyntaxKind::Constraint)
            .iter()
            .map(|written| {
                let origin = self.origin(written);
                let (name, args) = self.head_and_arguments(written)?;
                Some(RawConstraint { origin, name, args })
            })
            .collect()
    }

    /// A `where` clause the core has no binder for, refused where it was
    /// written.
    ///
    /// `01-surface.md` §1.4 elaborates a constraint on a free definition to "an
    /// extra parameter holding the dictionary", and `10-traits.md` §4 step 1
    /// finds that parameter by the constraint it discharges. `musa-core`
    /// discharges one at a trait, at a derived method, and at an `impl`, and a
    /// definition is none of the three — so a reading that invented the
    /// parameter would produce a term whose every method use refused, which is
    /// worse than saying so here.
    fn unconstrained(&mut self, node: &SyntaxNode) -> Option<()> {
        let Some(clause) = child(node, |kind| kind == SyntaxKind::WhereClause) else {
            return Some(());
        };
        self.refuse(
            Diagnostic::error(
                Code::UnsupportedLanguageStage,
                "a constraint here has no core spelling yet",
            )
            .at(trimmed_span(&clause), "written here")
            .note("the core binds a dictionary at a trait, at a derived method, and at an `impl`")
            .help("declare the operation as a trait method, or write the constraint on an `impl`"),
        )
    }

    /// A written type read as the name at its head and the arguments it was
    /// applied to.
    ///
    /// The one thing [`Lowering::ty`] cannot answer, and the reason is that a
    /// constraint is not a type: `Eq<List<A>>` names a trait and a head, and a
    /// [`RawConstraint`] holds the two apart because `10-traits.md` §4's lookup
    /// is keyed on them. The arguments *are* types and go through the ordinary
    /// reading.
    fn head_and_arguments(&mut self, node: &SyntaxNode) -> Option<(Name, Vec<Raw>)> {
        match node.kind() {
            SyntaxKind::TypeExpr => {
                let inner = child(node, is_type_node)?;
                self.head_and_arguments(&inner)
            }
            SyntaxKind::TypeName => Some((written_head(node), Vec::new())),
            SyntaxKind::AppliedType => {
                let parts = children(node, is_type_node);
                let (head, written) = parts.split_first()?;
                let arguments: Option<Vec<Raw>> = written.iter().map(|argument| self.ty(argument)).collect();
                Some((written_head(head), arguments?))
            }
            // The three the grammar gives their own node kinds. Reachable as an
            // inherent `impl List<A> { … }`, and as nothing else: a trait is
            // named by an identifier.
            SyntaxKind::OptionType | SyntaxKind::ListType | SyntaxKind::ResultType => {
                let name = match node.kind() {
                    SyntaxKind::OptionType => "Option",
                    SyntaxKind::ListType => "List",
                    _ => "Result",
                };
                let arguments: Option<Vec<Raw>> = children(node, is_type_node)
                    .iter()
                    .map(|argument| self.ty(argument))
                    .collect();
                Some((Name::from(name), arguments?))
            }
            _ => self.refuse(
                Diagnostic::error(Code::UnknownName, "this names no trait and no type")
                    .at(trimmed_span(node), "written here")
                    .help("an `impl` head and a constraint are a name, and the type arguments it is applied to"),
            ),
        }
    }
}

/// The name a declaration binds: the first identifier it writes itself.
///
/// Its *own* token, so a type parameter, a field, and a parameter — each of
/// which is a child node — cannot answer for the declaration. [`None`] without a
/// diagnostic when the parser produced no name, which it has already complained
/// about.
fn declared_name(node: &SyntaxNode) -> Option<Name> {
    own_tokens(node)
        .find(|token| token.kind() == SyntaxKind::Identifier)
        .map(|token| Name::from(token.text()))
}

/// Every `Param` of a declaration's parameter list, in order.
fn written_parameters(node: &SyntaxNode) -> Vec<SyntaxNode> {
    child(node, |kind| kind == SyntaxKind::ParamList)
        .map(|list| children(&list, |kind| kind == SyntaxKind::Param))
        .unwrap_or_default()
}

/// Whether `private` was written on this declaration.
fn visibility_of(node: &SyntaxNode) -> Visibility {
    if writes(node, SyntaxKind::PrivateKw) {
        Visibility::Private
    } else {
        Visibility::Public
    }
}

/// The name at the head of a written type, in the spelling the core knows it by.
///
/// [`compiler_type`] is asked for the same reason [`Lowering::ty`] asks it
/// first: an inherent `impl NoteName { … }` opens the namespace of the type the
/// composer calls `NoteName` and the core calls `PitchClass`, and a head read
/// two different ways in two places is two namespaces.
fn written_head(node: &SyntaxNode) -> Name {
    let written = node.to_string();
    let written = written.trim();
    Name::from(compiler_type(written).unwrap_or(written))
}
