//! A written type read as a raw core type.
//!
//! Short, and for the reason the module documentation gives: a written type is
//! almost always a name and an argument list, and the core is what knows whether
//! the name resolves and whether the arity is right. What is left here is the
//! four spellings the grammar gives their own node kinds — `Option<τ>`,
//! `List<τ>`, `Result<τ, τ>`, `τ -> τ` — and the three base types that take an
//! *index* rather than a type argument.

use musa_core::{Origin, Raw};
use musa_language::{SyntaxKind, SyntaxNode};

use super::{Lowering, applied, child, children, is_type_node};
use crate::core::Coordinate;
use crate::diagnose::{Code, Diagnostic};

impl Lowering<'_> {
    /// The raw type a written type node denotes, or [`None`] with a diagnostic
    /// reported at the node.
    ///
    /// # What is *not* checked here
    ///
    /// Arity, and whether a name denotes a type at all. `Option<Nat, Nat>`
    /// lowers to `Option Nat Nat` and the core refuses it as
    /// [`Refusal::NotAFunction`](musa_core::Refusal::NotAFunction) against
    /// `Option`'s own kind, which is the same answer one pass later and from the
    /// declaration rather than from a table beside it. The two exceptions below
    /// are the cases where lowering *cannot* write a term at all, so there is
    /// nothing for the core to be wrong about.
    pub(crate) fn ty(&mut self, node: &SyntaxNode) -> Option<Raw> {
        let origin = self.origin(node);
        match node.kind() {
            SyntaxKind::TypeExpr => {
                let inner = child(node, is_type_node)?;
                self.ty(&inner)
            }
            SyntaxKind::TypeName => self.named_type(node, origin),
            SyntaxKind::AppliedType => self.applied_type(node, origin),
            SyntaxKind::OptionType => self.one_argument(node, origin, "Option"),
            SyntaxKind::ListType => self.one_argument(node, origin, "List"),
            SyntaxKind::ResultType => {
                let parts = children(node, is_type_node);
                let value = self.ty(parts.first()?)?;
                let error = self.ty(parts.get(1)?)?;
                Some(applied(origin, Raw::var(origin, "Result"), [value, error]))
            }
            SyntaxKind::FunctionType => {
                let parts = children(node, is_type_node);
                let domain = self.ty(parts.first()?)?;
                let codomain = self.ty(parts.get(1)?)?;
                // The binder is named rather than anonymous because a Π always
                // binds: nothing in a written arrow refers to the argument, so
                // the name is unreachable and its only job is to be printable.
                Some(Raw::pi(origin, "argument", domain, codomain))
            }
            // `01-surface.md`'s anonymous product, as the `Pair` the prelude
            // already declares. The value side reads `(a, b)` as `Pair.Both a b`
            // for the reason [`crate::prelude`]'s own note gives — positions are
            // what a written pair has, and inventing `first`/`second` would make
            // two products that named them differently different types — and a
            // type that disagreed with the value side would be one construct
            // contradicting itself.
            //
            // Only two, and the refusal keeps the help text it always had. The
            // corpus writes 36 products and every one of them is a pair, so the
            // arity that would force a choice between `(A, (B, C))` and
            // `((A, B), C)` is one nobody has written; a reading invented for it
            // would be an encoding no reader could check against the source.
            SyntaxKind::ProductType => {
                let members: Option<Vec<Raw>> = children(node, is_type_node).iter().map(|held| self.ty(held)).collect();
                match members?.as_slice() {
                    [first, second] => Some(applied(
                        origin,
                        Raw::var(origin, "Pair"),
                        [first.clone(), second.clone()],
                    )),
                    written => self.wide_product(node, written.len()),
                }
            }
            _ => None,
        }
    }

    /// "an anonymous product of `n` has no core spelling", for `n` other than
    /// two.
    ///
    /// Shared with [`super::values`] so a written `(a, b, c)` and its type are
    /// refused in the same words: the value side reads a pair and the type side
    /// reads a pair, and one of them refusing a triple more helpfully than the
    /// other would be a reader's problem rather than a distinction.
    ///
    /// The help text is the one the whole construct carried before 142 gave the
    /// pair a spelling, and it is still the repair: a record's fields say what
    /// the positions meant, which is exactly what is missing when three of them
    /// are written and nothing says how they group.
    pub(super) fn wide_product<T>(&mut self, node: &SyntaxNode, written: usize) -> Option<T> {
        self.refuse(
            Diagnostic::error(
                Code::UnsupportedLanguageStage,
                format!("an anonymous product of {written} has no core spelling"),
            )
            .at(crate::resolve::trimmed_span(node), "written here")
            .help("declare a `record` whose fields name what the positions meant, and write that")
            .note("a product of two is `Pair`; wider ones would have to choose a nesting nothing wrote"),
        )
    }

    /// `Option<τ>` and `List<τ>`, which have their own node kinds because the
    /// grammar spells them that way, and one raw term because the core does not.
    fn one_argument(&mut self, node: &SyntaxNode, origin: Origin, family: &'static str) -> Option<Raw> {
        let inner = child(node, is_type_node)?;
        let member = self.ty(&inner)?;
        Some(Raw::app(origin, Raw::var(origin, family), member))
    }

    /// A bare type name.
    ///
    /// The order is the one the old checker used and for the same reason: the
    /// compiler's own vocabulary is asked first, so no library declaration and
    /// no type parameter can quietly become a second reading of `Pitch`. What is
    /// different is the tail — an unknown name is *written through* as a variable
    /// rather than refused here, because the core is what knows which names stand
    /// in the context and answers with
    /// [`Refusal::UnknownName`](musa_core::Refusal::UnknownName) at this very
    /// origin.
    fn named_type(&mut self, node: &SyntaxNode, origin: Origin) -> Option<Raw> {
        let written = node.to_string();
        let written = written.trim();
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
        if let Some(now) = musa_language::respelled_type(written) {
            return compiler_type(now).map(|spelled| Raw::var(origin, spelled));
        }
        Some(Raw::var(origin, written))
    }

    /// A type applied to arguments.
    fn applied_type(&mut self, node: &SyntaxNode, origin: Origin) -> Option<Raw> {
        // The head is the first type child and the arguments are the rest: an
        // argument may itself be a bare `TypeName`, so telling them apart by kind
        // would take `Pair<Nat>` for a `Pair` of nothing.
        let parts = children(node, is_type_node);
        let (head, arguments) = parts.split_first()?;
        let written = head.to_string();
        let written = written.trim().to_owned();
        // `Duration<C>`, `Position<C>`, and `Syntax<Cat>` take an *index* rather
        // than a type: nothing inhabits `WrittenTime` or `Expr`, and the word is
        // read from the argument node's own text. That is what keeps a coordinate
        // and a category out of the type argument position, so there is no way to
        // write `List<WrittenTime>`.
        if let Some((index, help)) = self.indexed_base(&written) {
            return self.written_index(index, &written, node, arguments, help);
        }
        let head = self.ty(head)?;
        let arguments: Option<Vec<Raw>> = arguments.iter().map(|child| self.ty(child)).collect();
        Some(applied(origin, head, arguments?))
    }

    /// The one indexed base type an [`Index`] and a written word name.
    ///
    /// Called only where [`Lowering::indexed_base`] has already said `written`
    /// is one of them, so there is no fourth answer to give: the index is read
    /// from the argument's own text, or the reading is refused at the node.
    fn written_index(
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
            Index::Category => crate::syntax::Cat::named(&word).map(crate::registry::category_literal),
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
    fn indexed_base(&self, written: &str) -> Option<(Index, String)> {
        match written {
            // `EventTrack` is here rather than in [`compiler_type`] because it is
            // registered at `Coordinate → Type 0` exactly as the two tagged
            // rationals are: a track of written beats and a track of seconds are
            // different types, and the word alone names neither. It is what the
            // ledger's row for `Music` replaces, so a fragment's type is now
            // written the way its duration always was.
            "Duration" | "Position" | "EventTrack" => Some((
                Index::Coordinate,
                format!("write `{written}<WrittenTime>`, or `<PhysicalTime>` for clock time"),
            )),
            "Syntax" if self.in_phase => Some((
                Index::Category,
                "write `Syntax<Expr>` for a tree that parses as an expression, or `Syntax<TokenTree>`".to_owned(),
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
enum Index {
    /// `Duration<C>` and `Position<C>`, indexed by a [`Coordinate`].
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
fn indexed_type(origin: Origin, name: &str, index: musa_core::Literal) -> Raw {
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
        "Pc12" => "Pc12",
        "PcSet12" => "PcSet12",
        "Row12" => "Row12",
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
