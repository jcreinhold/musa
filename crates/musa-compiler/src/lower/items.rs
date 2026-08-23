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
//! Five of the six words the surface declares with produce two core shapes,
//! and the interesting one is that three of the five are the same shape:
//!
//! - **`data`, `enum`, and `record` are one declaration**, spelled three ways,
//!   and each becomes a [`RawData`] of one family. `01-surface.md` §1.3: "each
//!   declaration generates its own family". [`Lowering::declaration`] is that
//!   one path: the word decides only where the cases are read from — a `data`'s
//!   variants, an `enum`'s cases, or the single case a `record` is — and
//!   nothing about what the declaration denotes. Prompt 161 made it one by
//!   giving `data` the two things only `enum` could say, a `private` marker on
//!   a case and a positional field list, rather than by taking either away:
//!   root `AGENTS.md`'s rule about sublanguages, one level up.
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
//! A bare `<A>` stands at `Type 0`, which is the level a written signature
//! lands in — §1 says the surface never writes a level, and every type the
//! surface *can* write is small. `<{n : Nat}>` is §1's other spelling and
//! stands at whatever it says: it is the only way to abstract over something
//! that is not a type, and `A` is exactly `{A : Type 0}` with the type left
//! out.

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
    /// A `data`, an `enum`, or a `record` — the one declaration, however spelled.
    Data(RawData),
    /// A `let` or a `fn`.
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
            SyntaxKind::DataDecl | SyntaxKind::EnumDecl | SyntaxKind::RecordDecl => {
                read(self.declaration(node).map(Item::Data))
            }
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

    // ---- the one declaration ----

    /// `data Motive<A> { Silence, private Sounded(pitch: Pitch, Duration) }`,
    /// `enum Reading<A> { Done(A) }`, `record Pending { read: Reading; }` —
    /// one declaration, whichever of the three words wrote it.
    ///
    /// One family, one visibility rule, one telescope, one constructor reader.
    /// The word decides exactly two things and neither is semantic: *where* the
    /// cases are written, which [`Lowering::cases`] answers, and whether an
    /// index telescope may stand after the name, which [`Lowering::index_telescope`]
    /// refuses by that word's own spelling. A bug in `enum` handling is not
    /// possible here because there is no `enum` handling.
    fn declaration(&mut self, node: &SyntaxNode) -> Option<RawData> {
        let origin = self.origin(node);
        let visibility = visibility_of(node);
        let name = declared_name(node)?;
        let params = self.type_parameters(node);
        let indices = self.index_telescope(node)?;
        let constructors = self.cases(node, &name, visibility)?;
        // No `where` clause is read here: `01-surface.md` §1's `where_clause`
        // is called from `trait`, `impl`, and a `fn` signature, and none of the
        // three type declarations is among them.
        Some(RawData {
            origin,
            params,
            families: vec![RawFamily {
                name,
                visibility,
                indices,
                constructors,
            }],
        })
    }

    /// Every constructor of a declaration, in source order.
    ///
    /// The three words put their cases in three places and that is the whole of
    /// the difference: a `data` writes [`SyntaxKind::DataVariant`]s, an `enum`
    /// writes [`SyntaxKind::EnumCase`]s, and a `record` writes its fields
    /// directly, which makes the declaration itself the one case — spelled the
    /// same as the family, since `01-surface.md` §1.2 is a declaration of one
    /// shape and there is no second name for it to take.
    fn cases(&mut self, node: &SyntaxNode, name: &Name, declared: Visibility) -> Option<Vec<RawConstructor>> {
        let written = match node.kind() {
            SyntaxKind::DataDecl => children(node, |kind| kind == SyntaxKind::DataVariant),
            SyntaxKind::EnumDecl => children(node, |kind| kind == SyntaxKind::EnumCase),
            // A record is its own case, so the node the fields hang off is the
            // declaration and the constructor is read straight from it.
            _ => return Some(vec![self.constructor(node, Arc::clone(name), declared)?]),
        };
        let mut built = Vec::new();
        for case in &written {
            let case_name = declared_name(case)?;
            built.push(self.constructor(case, case_name, declared)?);
        }
        Some(built)
    }

    /// One constructor, from whichever node carries its arguments.
    ///
    /// The argument list is read in source order and each argument is named or
    /// it is not: a [`SyntaxKind::DataField`] and a [`SyntaxKind::FieldDecl`]
    /// both write a name, and a bare type does not. An unnamed one is called
    /// `_0`, `_1`, … by its position among *all* the arguments, because
    /// `01-surface.md` §1.3 says the positional form "names types and not
    /// fields": the names exist only to be distinct, and a leading underscore
    /// cannot collide with a written one because §1's identifiers do not start
    /// with one.
    ///
    /// Visibility is the case's own marker where it wrote one and the
    /// declaration's where it did not. [`RawConstructor::visibility`] is
    /// all-or-none — a family whose cases disagree is refused at its
    /// declaration — so an unmarked case has to answer the same as its
    /// neighbours, and the declaration's marker is the one answer that is the
    /// same for all of them.
    fn constructor(&mut self, node: &SyntaxNode, name: Name, declared: Visibility) -> Option<RawConstructor> {
        let origin = self.origin(node);
        let mut fields = Vec::new();
        for written in node.children() {
            let binder = match written.kind() {
                SyntaxKind::DataField | SyntaxKind::FieldDecl => self.declared_field(&written)?,
                kind if is_type_node(kind) => RawBinder {
                    name: Arc::from(format!("_{}", fields.len()).as_str()),
                    ty: self.ty(&written)?,
                },
                _ => continue,
            };
            fields.push(binder);
        }
        Some(RawConstructor {
            origin,
            name,
            visibility: if writes(node, SyntaxKind::PrivateKw) {
                Visibility::Private
            } else {
                declared
            },
            fields,
            chosen: self.chosen_indices(node)?,
        })
    }

    /// `(n: Nat)` — the index telescope a declaration writes, or nothing.
    ///
    /// Empty for a declaration that writes no parentheses, which is the
    /// ordinary case: a family with no indices is a family whose constructors
    /// all stand at the same type.
    ///
    /// Only `data` may write one, and the other two words are refused *here*
    /// rather than in the grammar. §1.2 declares fields and §1.3 declares
    /// cases; neither writes a signature, so neither has anywhere to say what
    /// a case's result is indexed by. A parser that stopped at the `(` would
    /// report what it wanted — ``expected `{}` `` — and this says where an
    /// indexed family is written instead.
    fn index_telescope(&mut self, node: &SyntaxNode) -> Option<Vec<RawBinder>> {
        let Some(written) = child(node, |kind| kind == SyntaxKind::DataIndices) else {
            return Some(Vec::new());
        };
        if let Some(word) = other_than_data(node) {
            return self.refuse(
                Diagnostic::error(
                    Code::Misplaced,
                    format!("a `{word}` declaration takes no index telescope"),
                )
                .at(trimmed_span(&written), "written here")
                .note("an indexed family is written with `data`, where a case says which indices it chooses")
                .help(format!("write `data` in place of `{word}`, or drop the parentheses")),
            );
        }
        let mut built = Vec::new();
        for binder in children(&written, |kind| kind == SyntaxKind::DataField) {
            built.push(self.declared_field(&binder)?);
        }
        Some(built)
    }

    /// `: (n + 1)` — the indices a constructor chooses, or nothing.
    ///
    /// Read as ordinary expressions. Whether there are the right *number* of
    /// them is the core's question, not this one's: it holds the declaration's
    /// telescope and answers with
    /// [`Refusal::IndexCount`](musa_calculus::Refusal::IndexCount) at this very
    /// origin.
    fn chosen_indices(&mut self, node: &SyntaxNode) -> Option<Vec<Raw>> {
        let Some(written) = child(node, |kind| kind == SyntaxKind::DataChosen) else {
            return Some(Vec::new());
        };
        let mut built = Vec::new();
        for chosen in children(&written, is_expr_node) {
            built.push(self.expr(&chosen)?);
        }
        Some(built)
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
            definition.name = Name::from(format!("{head}{}{}", crate::lower::DOT, definition.name));
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

    /// `<A, B>`, `<{n : Nat}>` — the parameters a declaration abstracts over.
    ///
    /// `01-surface.md` §1's two spellings, and the braced one is here for the
    /// half the bare one cannot say: a parameter's own type. `A` means
    /// `{A : Type 0}` and that is what it lowers to, so the bare form is not a
    /// second rule — it is this one with the type left out.
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
                let ty = match child(written, is_type_node) {
                    Some(stated) => self.ty(&stated)?,
                    None => Raw::universe(at, Sort::ZERO),
                };
                Some(RawBinder {
                    name: declared_name(written)?,
                    ty,
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

/// The word a declaration was written with, when it is not `data`.
///
/// For diagnostics only, and never for typing: the three words denote the same
/// thing, so the one thing left for a spelling to decide is which spelling a
/// refusal about it uses.
fn other_than_data(node: &SyntaxNode) -> Option<&'static str> {
    match node.kind() {
        SyntaxKind::EnumDecl => Some("enum"),
        SyntaxKind::RecordDecl => Some("record"),
        _ => None,
    }
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
