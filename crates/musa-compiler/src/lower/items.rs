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
//! - **`impl` is a namespace**, so `impl Pitch { fn act(…) { … } }` becomes the
//!   ordinary definition `Pitch.act`. §1.5: a member is reached by taking the
//!   head of the receiver's type and looking up one dotted name, so an `impl`
//!   block declares nothing a `fn` at the top level does not — it only spells
//!   the prefix once for every definition inside it.
//! - **`let` and `fn`** become a [`Definition`]: a name, the type the
//!   declaration wrote, and a value.
//!
//! # Type parameters, and why their filling differs
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

use musa_calculus::{Name, Origin, Raw, RawBinder, RawConstructor, RawData, RawFamily, Sort, Visibility};
use musa_syntax::{SyntaxKind, SyntaxNode};

use super::types::compiler_type;
use super::{Lowering, child, children, is_expr_node, is_type_node, own_tokens, writes};
use crate::resolve::trimmed_span;
use musa_score::diagnose::{Code, Diagnostic};

/// One declaration, read.
///
/// Three variants because `musa-calculus` has two doors — [`musa_calculus::declare`]
/// and [`musa_calculus::declare_program`] — and one of them is reached with a
/// list. A caller matches once and calls; it never has to ask what word the
/// source used.
#[derive(Debug)]
pub(crate) enum Item {
    /// A `data` or an `enum`.
    Data(RawData),
    /// A `let`, a `fn`, or a `record`.
    Definition(Definition),
    /// An `impl` — every `fn` inside it, under the type's name, beside the node
    /// each was written at.
    ///
    /// The node travels with the definition because a caller files documentation
    /// and duplicate-name reports against the declaration a reader can see, and
    /// that is the `fn` rather than the block around it.
    Namespace(Vec<(SyntaxNode, Definition)>),
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
/// lost: what changes is which of [`musa_calculus::check`] and [`musa_calculus::infer`]
/// a caller uses.
///
/// # What is deliberately not here
///
/// **Visibility.** `musa-calculus` gives a family, a constructor, and a trait one
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

/// What a node turned out to be, when a walk over a file's children asked.
///
/// Three answers rather than two because a walk has two unrelated reasons to
/// pass a node by, and only one of them is a failure. [`Self::Elsewhere`] is a
/// node that is not a declaration at all — a `piece`, a `library`, a
/// `structure`, an `import` — which some other pass reads for itself, so
/// answering with a complaint would make walking a document an error.
/// [`Self::Refused`] is a declaration that *is* this reading's and could not be
/// read, and the difference is worth a variant because a document that quietly
/// drops one goes on to report every *use* of the missing name as an unknown
/// name — burying the one thing that actually went wrong under its own
/// consequences.
#[derive(Debug)]
pub(crate) enum Declared {
    /// The declaration, through whichever of `musa-calculus`'s doors it goes.
    Item(Item),
    /// A declaration that could not be read. The refusal is already reported at
    /// whatever could not be read, so a caller states no complaint of its own.
    Refused,
    /// Not a declaration — somebody else's node.
    Elsewhere,
}

impl Lowering<'_> {
    /// What `node` declares, when it declares anything — see [`Declared`].
    pub(crate) fn item(&mut self, node: &SyntaxNode) -> Declared {
        let read = |item: Option<Item>| item.map_or(Declared::Refused, Declared::Item);
        match node.kind() {
            SyntaxKind::DataDecl => read(self.nominal(node).map(Item::Data)),
            SyntaxKind::EnumDecl => read(self.enumeration(node).map(Item::Data)),
            SyntaxKind::RecordDecl => read(self.structural(node).map(Item::Definition)),
            SyntaxKind::ImplDecl => read(self.namespace(node).map(Item::Namespace)),
            SyntaxKind::FnDecl => read(self.function(node).map(Item::Definition)),
            SyntaxKind::LetDecl => read(self.binding(node).map(Item::Definition)),
            // `01-surface.md` §2's two notation declarations, which desugar to
            // the two above with a role retained. `super::notation` owns them
            // because what they declare is a block.
            SyntaxKind::MotifDecl => read(self.motif(node).map(Item::Definition)),
            SyntaxKind::FragmentDecl => read(self.fragment(node).map(Item::Definition)),
            _ => Declared::Elsewhere,
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
        // `data` has no `where` clause to read: `01-surface.md` §1's
        // `where_clause` is called from `record`, `enum`, `trait`, `impl`,
        // and a `fn` signature, and `data_decl` is not among them.
        Some(RawData {
            origin,
            params,
            families: vec![RawFamily {
                name,
                visibility,
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
                constructors,
            }],
        })
    }

    /// One enum case: empty, positional, or named.
    ///
    /// A positional case's fields are named `_0`, `_1`, …, because §1.3 says the
    /// positional form "names types and not fields": the names exist only to be
    /// distinct, and a leading underscore cannot collide with one a `record`
    /// wrote because `01-surface.md`'s identifiers do not start with one.
    fn case(&mut self, node: &SyntaxNode) -> Option<RawConstructor> {
        let origin = self.origin(node);
        let name = declared_name(node)?;
        let mut fields = Vec::new();
        for (index, written) in children(node, is_type_node).iter().enumerate() {
            fields.push(RawBinder {
                name: Arc::from(format!("_{index}").as_str()),
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
        let params = self.type_parameters(node);
        let mut names = Vec::new();
        let mut types = Vec::new();
        for written in children(node, |kind| kind == SyntaxKind::FieldDecl) {
            let field = self.declared_field(&written)?;
            names.push(field.name);
            types.push(field.ty);
        }
        let mut value = Raw::record_type(origin, names.iter().map(|name| &**name).zip(types));
        let mut ty = Raw::universe(origin, Sort::ZERO);
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

    // ---- namespaces ----

    /// `impl Pitch { fn act(subject: Pitch, operation: Interval) -> Pitch { … } }`.
    ///
    /// One block is *n* ordinary definitions, named `Pitch.act` and so on. There
    /// is no second declaration form here and nothing the core has to be told:
    /// `01-surface.md` §1.5 resolves `p.act(i)` by taking the head of `p`'s type
    /// and looking up that one name, so what an `impl` contributes is the prefix,
    /// written once instead of on every definition inside it.
    ///
    /// The head is a bare type name and the block takes no parameters of its
    /// own, both refused rather than read. A namespace is keyed on the head
    /// constant — `Duration<WrittenTime>` and `Duration<PhysicalTime>` share
    /// `Duration`'s — so arguments written here would be read by nobody, and a
    /// type parameter of the block would be a binder with no signature to sit
    /// on. Each `fn` writes its own, which is where a reader looks anyway.
    fn namespace(&mut self, node: &SyntaxNode) -> Option<Vec<(SyntaxNode, Definition)>> {
        let head = self.namespace_head(node)?;
        if let Some(list) = child(node, |kind| kind == SyntaxKind::TypeParams) {
            return self.refuse(
                Diagnostic::error(Code::Misplaced, "an `impl` block takes no type parameters")
                    .at(trimmed_span(&list), "written here")
                    .note("the block names a namespace, and a namespace is one name")
                    .help("write the parameters on each `fn` inside it"),
            );
        }
        let mut declared = Vec::new();
        for written in children(node, |kind| kind == SyntaxKind::FnDecl) {
            let mut definition = self.function(&written)?;
            definition.name = Name::from(format!("{head}{}{}", crate::module::DOT, definition.name));
            declared.push((written, definition));
        }
        Some(declared)
    }

    /// The type name an `impl` block opens, refusing anything that is not one.
    fn namespace_head(&mut self, node: &SyntaxNode) -> Option<Name> {
        let written = child(node, is_type_node)?;
        let inner = match written.kind() {
            SyntaxKind::TypeExpr => child(&written, is_type_node)?,
            _ => written,
        };
        match inner.kind() {
            SyntaxKind::TypeName => Some(written_head(&inner)),
            _ => self.refuse(
                Diagnostic::error(Code::Misplaced, "an `impl` block is opened by a type's name")
                    .at(trimmed_span(&inner), "written here")
                    .note("a namespace is keyed on the head of a type and not on its arguments")
                    .help("write the name alone, and let each `fn` inside write the arguments it needs"),
            ),
        }
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
        let parameters = self.parameters(node)?;
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
        // Bracketed as [`values::lambda`]'s is: the body's quote patterns read
        // the scrutinee's written category off this list.
        let remembered = self.push_syntax_categories(&parameters);
        let value = self.expr(&child(node, is_expr_node)?);
        self.pop_syntax_categories(remembered);
        let mut value = value?;
        for parameter in parameters.iter().rev() {
            let at = self.origin(parameter);
            let bound = declared_name(parameter)?;
            let written = child(parameter, is_type_node)?;
            ty = Raw::pi(at, Arc::clone(&bound), self.ty(&written)?, ty);
            value = Raw::lam(at, bound, value);
        }
        for parameter in self.type_parameters(node).iter().rev() {
            ty = Raw::parameter_pi(origin, Arc::clone(&parameter.name), parameter.ty.clone(), ty);
        }
        Some(Definition {
            origin,
            name,
            ty: Some(ty),
            value,
        })
    }

    /// A declaration's parameters, refusing a name the list writes twice.
    ///
    /// The rule is the surface's and cannot be the core's. A telescope binds by
    /// position, so `(value : Nat) → (value : Nat) → Nat` is a perfectly good Π
    /// in which the second binder shadows the first — the core has a
    /// `DuplicateField` and a `DuplicateCase` and deliberately no duplicate
    /// *binder*, because shadowing is what a de Bruijn index is for. What the
    /// core cannot know is that both names were written by an author who can
    /// only ever reach one of them: every mention of `value` in the body means
    /// the second parameter, so the first is a value the function takes and no
    /// program can read. That is a mistake at the place it was made, and it is
    /// refused here for the same reason `quotes.rs` refuses two holes of one
    /// name.
    ///
    /// One reader for three callers — a signature's Π, a method's dictionary
    /// field, and a bare λ — because the rule is a property of the list rather
    /// than of what any one of them builds out of it.
    pub(super) fn parameters(&mut self, node: &SyntaxNode) -> Option<Vec<SyntaxNode>> {
        let written = written_parameters(node);
        let mut seen: Vec<(Name, musa_score::SourceSpan)> = Vec::with_capacity(written.len());
        for parameter in &written {
            let name = declared_name(parameter)?;
            if let Some((_, previous)) = seen.iter().find(|(taken, _)| *taken == name) {
                return self.refuse(
                    Diagnostic::error(Code::DuplicateName, format!("parameter `{name}` is written twice"))
                        .at(trimmed_span(parameter), "written again here")
                        .also(*previous, "first written here")
                        .help("give the two parameters different names")
                        .note("the second binding shadows the first, so nothing in the body could read it"),
                );
            }
            seen.push((name, trimmed_span(parameter)));
        }
        Some(written)
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
    /// The filling is not decided here, because a [`RawBinder`] does not carry
    /// one: the caller writes them into a Π at the filling its own declaration
    /// wants, and [`RawTrait`], [`RawImpl`], and [`RawMethod`] leave the choice
    /// to `musa-calculus`.
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
                    ty: Raw::universe(at, Sort::ZERO),
                })
            })
            .collect()
    }
}

/// The name a declaration binds: the first identifier it writes itself.
///
/// Its *own* token, so a type parameter, a field, and a parameter — each of
/// which is a child node — cannot answer for the declaration. [`None`] without a
/// diagnostic when the parser produced no name, which it has already complained
/// about.
pub(super) fn declared_name(node: &SyntaxNode) -> Option<Name> {
    own_tokens(node)
        .find(|token| token.kind() == SyntaxKind::Identifier)
        .map(|token| Name::from(token.text()))
}

/// Every `Param` of a declaration's parameter list, in order.
///
/// Reading only. [`Lowering::parameters`] is what a caller that can refuse
/// should use; this is here for the one caller that only wants to know whether
/// the list is empty.
pub(super) fn written_parameters(node: &SyntaxNode) -> Vec<SyntaxNode> {
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
