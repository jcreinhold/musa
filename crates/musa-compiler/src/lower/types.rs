//! A written type read as a raw core type.
//!
//! Short, and for the reason the module documentation gives: a written type is
//! almost always a name and an argument list, and the core is what knows whether
//! the name resolves and whether the arity is right. Application therefore uses
//! the same value-lowering path in type and term positions. What is left here is
//! the structural type syntax (`τ -> τ` and products) and the base names whose
//! host representation needs a coordinate or registry lookup.

use musa_calculus::{Origin, Raw, Sort};
use musa_syntax::{SyntaxKind, SyntaxNode};

use super::{Lowering, applied, child, children, is_type_node, paired};
use crate::phase::Coordinate;
use musa_score::diagnose::{Code, Diagnostic};

impl Lowering<'_> {
    /// The raw type a written type node denotes, or [`None`] with a diagnostic
    /// reported at the node.
    ///
    /// # What is *not* checked here
    ///
    /// Arity, and whether a name denotes a type at all. `Option<Nat, Nat>`
    /// lowers to `Option Nat Nat` and the core refuses it as
    /// [`Refusal::NotAFunction`](musa_calculus::Refusal::NotAFunction) against
    /// `Option`'s own kind, which is the same answer one pass later and from the
    /// declaration rather than from a table beside it. The two exceptions below
    /// are the cases where lowering *cannot* write a term at all, so there is
    /// nothing for the core to be wrong about.
    pub(crate) fn ty(&mut self, node: &SyntaxNode) -> Option<Raw> {
        let origin = self.origin(node);
        match node.kind() {
            SyntaxKind::TypeExpr => {
                let inner = child(node, |kind| is_type_node(kind) || kind == SyntaxKind::ApplyExpr)?;
                if inner.kind() == SyntaxKind::ApplyExpr {
                    self.value(&inner)
                } else {
                    self.ty(&inner)
                }
            }
            SyntaxKind::TypeName => self.named_type(node, origin),
            SyntaxKind::FunctionType => {
                let parts = children(node, is_type_node);
                let domain = self.ty(parts.first()?)?;
                let codomain = self.ty(parts.get(1)?)?;
                // `arrow` and not `pi`: a written arrow declares no parameter,
                // and §1.3's completeness rule counts the ones that were
                // declared. `musa_calculus::ARROW_BINDER` says the rest.
                Some(Raw::arrow(origin, domain, codomain))
            }
            // `01-surface.md`'s anonymous product, as the `Pair` the prelude
            // already declares. The value side reads `(a, b)` as `Pair.Both a b`
            // for the reason [`crate::prelude`]'s own note gives — positions are
            // what a written pair has, and inventing `first`/`second` would make
            // two products that named them differently different types — and a
            // type that disagreed with the value side would be one construct
            // contradicting itself.
            //
            // Any width, right-nested, which is the same fold the value side and
            // a pattern run: [`super::paired`] holds the direction and the
            // argument for it.
            SyntaxKind::ProductType => {
                let members: Option<Vec<Raw>> = children(node, is_type_node).iter().map(|held| self.ty(held)).collect();
                paired(members?, |first, second| {
                    applied(origin, Raw::hosted(origin, "Pair"), [first, second])
                })
            }
            _ => None,
        }
    }

    /// A bare type name.
    ///
    /// The order is the one the old checker used and for the same reason: the
    /// compiler's own vocabulary is asked first, so no library declaration and
    /// no type parameter can quietly become a second reading of `Pitch`. What is
    /// different is the tail — an unknown name is *written through* as a variable
    /// rather than refused here, because the core is what knows which names stand
    /// in the context and answers with
    /// [`Refusal::UnknownName`](musa_calculus::Refusal::UnknownName) at this very
    /// origin.
    fn named_type(&mut self, node: &SyntaxNode, origin: Origin) -> Option<Raw> {
        let written = node.to_string();
        let written = written.trim();
        if written == "Type" {
            return Some(Raw::universe(origin, Sort::ZERO));
        }
        if let Some(spelled) = compiler_type(written) {
            return Some(Raw::var(origin, spelled));
        }
        if self.in_phase
            && let Some(spelled) = phase_type(written)
        {
            return Some(Raw::var(origin, spelled));
        }
        // The three types that name nothing without an argument. Each is refused
        // here rather than written through, because `Duration` alone is not a
        // term the core could resolve to anything — the base type is registered
        // at `Coordinate → Type 0`, so the honest complaint is about the missing
        // argument and the core would report a kind mismatch instead.
        if let Some((_, help)) = self.indexed_base(written) {
            return self.refuse(
                Diagnostic::error(Code::WrongArity, format!("`{written}` takes an argument"))
                    .at(crate::resolve::trimmed_span(node), "written with none")
                    .help(help),
            );
        }
        // A removed spelling has already been reported at the word by the parser,
        // with the capital that replaces it. Reading it as the type it named
        // leaves the rest of the declaration checked and keeps one complaint one
        // complaint.
        if let Some(now) = musa_syntax::respelled_type(written) {
            return compiler_type(now).map(|spelled| Raw::var(origin, spelled));
        }
        Some(Raw::var(origin, written))
    }

    /// `Machine<K, A, B>` and `Primitive<K, A, B>`: a step tag, then two ports.
    ///
    /// The step is position-restricted the way an index word is, and for the
    /// same reason one step over: `03-machine-calculus.md` §2 gives a step tag
    /// no values, so nothing computes one and no definition ranges over one —
    /// the word is looked up in the build's registry and refused if it is not
    /// there.
    ///
    /// It has to be checked *here* rather than in the signature, because the
    /// signature cannot say it. A step tag is a host notion:
    /// [`crate::registry`] owns the table and `musa-calculus` owns the mechanism, so
    /// `Machine Nat A B` is a well-typed core term and nothing in the core could
    /// learn otherwise. The one place the surface writes a step is the one place
    /// left to refuse it.
    ///
    /// The other half of §2's premises — a port holds storable data — is *not*
    /// here, and that is the difference the restatement made: it is `Storable`
    /// on the registered signatures, so `Machine<K, Ratio -> Ratio, …>` is
    /// refused by the elaborator rather than by a second reading of the same
    /// written type.
    pub(super) fn machine_type(
        &mut self,
        node: &SyntaxNode,
        origin: Origin,
        written: &str,
        head: &SyntaxNode,
        arguments: &[SyntaxNode],
    ) -> Option<Raw> {
        let [step, input, output] = arguments else {
            return self.refuse(
                Diagnostic::error(
                    Code::WrongArity,
                    format!("`{written}` takes 3 type arguments: a step, an input port, and an output port"),
                )
                .at(crate::resolve::trimmed_span(node), "written here")
                .help(format!("write `{written}(AudioFrameStep, τ, τ)`")),
            );
        };
        let word = step.to_string();
        if !crate::registry::is_step_tag(word.trim()) {
            return self.refuse(
                Diagnostic::error(Code::WrongArity, format!("`{}` is not a step", word.trim()))
                    .at(crate::resolve::trimmed_span(step), "written where a step belongs")
                    .help("a step says what one step of the machine counts; `AudioFrameStep` is one".to_owned()),
            );
        }
        let head = self.value(head)?;
        let read = [step, input, output]
            .into_iter()
            .map(|child| {
                if is_type_node(child.kind()) {
                    self.ty(child)
                } else {
                    self.value(child)
                }
            })
            .collect::<Option<Vec<Raw>>>()?;
        Some(applied(origin, head, read))
    }

    /// The one indexed base type an [`Index`] and a written word name.
    ///
    /// Called only where [`Lowering::indexed_base`] has already said `written`
    /// is one of them, so there is no fourth answer to give: the index is read
    /// from the argument's own text, or the reading is refused at the node.
    pub(super) fn written_index(
        &mut self,
        index: Index,
        written: &str,
        node: &SyntaxNode,
        arguments: &[SyntaxNode],
        help: String,
    ) -> Option<Raw> {
        let origin = self.origin(node);
        let word = self.single_index(written, node, arguments, &help)?;
        let literal = match index {
            Index::Coordinate => coordinate_named(&word).map(crate::registry::coordinate_literal),
            Index::Category => crate::quote::Cat::named(&word).map(crate::registry::category_literal),
        };
        let Some(literal) = literal else {
            return self.refuse(
                Diagnostic::error(Code::UnknownName, format!("unknown {} `{word}`", index.what()))
                    .at(crate::resolve::trimmed_span(node), "written here")
                    .help(help),
            );
        };
        Some(indexed_type(origin, written, literal))
    }

    /// The one word an indexed base type's argument list must hold.
    fn single_index(
        &mut self,
        written: &str,
        node: &SyntaxNode,
        arguments: &[SyntaxNode],
        help: &str,
    ) -> Option<String> {
        let Some(argument) = arguments.first().filter(|_| arguments.len() == 1) else {
            return self.refuse(
                Diagnostic::error(
                    Code::WrongArity,
                    format!("`{written}` takes 1 argument, not {}", arguments.len()),
                )
                .at(crate::resolve::trimmed_span(node), "written here")
                .help(help.to_owned()),
            );
        };
        Some(argument.to_string().trim().to_owned())
    }

    /// Which index a written base-type word takes, and the help that says so.
    ///
    /// One list, asked by both readers: [`Lowering::named_type`] to refuse the
    /// bare word and [`Lowering::applied_type`] to read the argument as an
    /// index. Two lists would be two answers to "is `Syntax` a type here", and
    /// the phase scope is exactly where they would disagree.
    pub(super) fn indexed_base(&self, written: &str) -> Option<(Index, String)> {
        match written {
            // `EventTrack` is here rather than in [`compiler_type`] because it is
            // registered at `Coordinate → Type 0` exactly as the two tagged
            // rationals are: a track of written beats and a track of seconds are
            // different types, and the word alone names neither. It is what the
            // ledger's row for `Music` replaces, so a fragment's type is now
            // written the way its duration always was.
            "Duration" | "Position" | "EventTrack" => Some((
                Index::Coordinate,
                format!("write `{written}(WrittenTime)`, or `(PhysicalTime)` for clock time"),
            )),
            "Syntax" if self.in_phase => Some((
                Index::Category,
                "write `Syntax(Expr)` for a tree that parses as an expression, or `Syntax(TokenTree)`".to_owned(),
            )),
            _ => None,
        }
    }
}

/// Which kind of index one of the three indexed base types takes.
///
/// Not a type argument: nothing inhabits `WrittenTime` or `Expr`, so the word is
/// read from the argument node's own text and becomes a literal.
#[derive(Clone, Copy)]
pub(super) enum Index {
    /// `Duration(C)` and `Position(C)`, indexed by a [`Coordinate`].
    Coordinate,
    /// `Syntax<Cat>`, indexed by a syntax category.
    Category,
}

impl Index {
    /// What an unknown index word failed to name, for the complaint.
    fn what(self) -> &'static str {
        match self {
            Self::Coordinate => "coordinate",
            Self::Category => "syntax category",
        }
    }
}

/// One indexed base type applied to the literal that indexes it.
fn indexed_type(origin: Origin, name: &str, index: musa_calculus::Literal) -> Raw {
    Raw::app(origin, Raw::var(origin, name), Raw::lit(origin, index))
}

/// The coordinate a written word names.
fn coordinate_named(text: &str) -> Option<Coordinate> {
    match text {
        "WrittenTime" => Some(Coordinate::WrittenTime),
        "PhysicalTime" => Some(Coordinate::PhysicalTime),
        _ => None,
    }
}

/// Which name in the core context a written base-type word denotes.
///
/// The one spelling that differs is the one the surface renamed: `NoteName` is
/// what a composer writes for a pitch class. The rest are the same word, and are
/// listed rather than defaulted so that adding a base type to
/// [`crate::registry`] without a surface spelling is a compile error here rather
/// than a name that silently resolves to nothing.
pub(super) fn compiler_type(written: &str) -> Option<&'static str> {
    Some(match written {
        "Bool" => "Bool",
        "Nat" => "Nat",
        "Ratio" => "Ratio",
        "Text" => "Text",
        "Pitch" => "Pitch",
        "NoteName" => "PitchClass",
        "Interval" => "Interval",
        "Scale" => "Scale",
        "Key" => "Key",
        "Degree" => "Degree",
        "Frame" => "Frame",
        "ChordClass" => "ChordClass",
        "Triad" => "Triad",
        "Roman" => "Roman",
        "Voicing" => "Voicing",
        _ => return None,
    })
}

/// Which name in the core context a written phase-type word denotes.
fn phase_type(written: &str) -> Option<&'static str> {
    Some(match written {
        "TokenKind" => "TokenKind",
        "Delimiter" => "Delimiter",
        "NodePath" => "NodePath",
        "BindingPath" => "BindingPath",
        "SyntaxStep" => "SyntaxStep",
        _ => return None,
    })
}
