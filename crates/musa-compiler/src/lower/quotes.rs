//! `quote at here { … }` and its pattern form, as core terms.
//!
//! One body walk, read twice. A quote that builds and a quote that matches share
//! a grammar — the same parser reads both bodies, and only what a `$` *means*
//! differs — so [`Walk`] carries which direction is being read rather than there
//! being two walks to keep in step. That is the arrangement `crate::core`'s own
//! `QuoteWalk` already had; what changes here is only what the walk *answers*.
//!
//! # What a quote becomes
//!
//! A [`crate::syntax::Template`] is a **literal of an inert base type**
//! (`02-core-calculus.md` §5.8's D1), so the body of a quote is not a term at all
//! — it is a constant, and building the tree it describes is one δ-application:
//!
//! ```text
//! instantiate_quote(anchor, ⟦body⟧, [[s₀], s₁, [s₂], …])
//! ```
//!
//! The splices are a list of lists because a hole holds a run or a node, and the
//! core has no sum of the two a lowering could write; the template says which is
//! which, so a single splice contributes the one-element list and a `$..xs`
//! contributes its own list. The rule reads them back the same way.
//!
//! Nothing here computes a derived path or mints a scope.
//! [`crate::syntax::instantiate`] does both, once, and this module's whole claim
//! is that a lowered quote reaches *that* function rather than a second copy of
//! it — which is `02-core-calculus.md` §5's second-path audit applied where two
//! answers would be worst, since a disagreement about identity is not a wrong
//! answer but two nodes that are one node in one pass and two in the next.
//!
//! # What a quote pattern becomes
//!
//! The same literal, read backwards, asked one hole at a time — the chain
//! [`super::values`] already writes for a match against inert values, with
//! `match_quote` where that one writes `==`:
//!
//! ```text
//! let ?subject = s;
//! if match_quote(?subject, T₀) {
//!     let head = quote_hole(?subject, T₀, 0);
//!     let rest = quote_holes(?subject, T₀, 1);
//!     … the first arm's body …
//! } else { … the arms after it … }
//! ```
//!
//! Three registrations rather than one `Option` of a list of lists, and prompt
//! 141ga's Design argues why from two facts of the core: a δ signature admits a
//! base type at a *literal* index and refuses one at a variable, so the holes are
//! read at `⟨tokentree⟩` and not at the scrutinee's own category; and an
//! unannotated `let` infers its value where a `match` has no inference rule, so
//! the continuation of a chain cannot be bound to a name and would have to be
//! copied into every coverage hole of a destructuring.

use musa_calculus::{Origin, Raw, RawArm, RawPattern};
#[cfg(test)]
mod laws;

use musa_language::ast::AstNode;
use musa_language::{SyntaxElement, SyntaxKind, SyntaxNode, SyntaxToken};

use super::{Lowering, applied, child, is_expr_node, listed, significant_tokens, whole};
use crate::syntax::{Hygiene, Template};
use musa_score::diagnose::{Code, Diagnostic};
use musa_score::origin::SourceSpan;

/// One quote body being read, and what it has found so far.
///
/// `matching` decides which of the two hole vectors is filled — exactly one of
/// them ever is. `scope` is always empty in a pattern, because a pattern writes
/// no binder: it writes nothing.
struct Walk {
    /// Whether this body is a pattern (`11-quotation.md` §4) rather than a
    /// construction.
    matching: bool,
    /// A construction's holes, each already the *list* its position contributes:
    /// the one-element list for `$x`, and the spliced expression itself for
    /// `$..xs`.
    splices: Vec<Raw>,
    /// A pattern's holes: the name each binds, and whether it binds a run.
    bound: Vec<(String, bool)>,
    /// The binders the body itself writes, for hygiene, each named by its
    /// position in the template being built.
    scope: Vec<(String, Vec<u32>)>,
}

impl Walk {
    fn building() -> Self {
        Self {
            matching: false,
            splices: Vec::new(),
            bound: Vec::new(),
            scope: Vec::new(),
        }
    }

    fn matching() -> Self {
        Self {
            matching: true,
            ..Self::building()
        }
    }
}

impl Lowering<'_> {
    /// `quote at a { … }` — one application of `instantiate_quote`.
    pub(super) fn quote(&mut self, node: &SyntaxNode, origin: Origin) -> Option<Raw> {
        self.quotable(node)?;
        let quote = musa_language::ast::QuoteExpr::cast(node.clone())?;
        let anchor = self.value(&quote.anchor()?)?;
        let mut walk = Walk::building();
        let (template, quotation) = self.body(&quote.body()?, &mut walk)?;
        Some(applied(
            origin,
            Raw::hosted(origin, "instantiate_quote"),
            [
                anchor,
                Raw::lit(origin, crate::registry::template_literal(template, quotation)),
                listed(origin, walk.splices),
            ],
        ))
    }

    /// A match whose arms are quote patterns, as nested `match_quote` tests.
    ///
    /// Beside the equality chain [`super::values`] writes for a match against
    /// inert values and shaped after it, down to the first catch-all ending the
    /// chain: an arm after it is unreachable, and the core is what says so when
    /// it sees the `Bool` match this becomes.
    pub(super) fn quote_chain(
        &mut self,
        node: &SyntaxNode,
        origin: Origin,
        subject: Raw,
        arms: &[SyntaxNode],
    ) -> Option<Raw> {
        // 11-quotation §4: the pattern is read at the scrutinee's category, so
        // its holes bind there — `quote_hole_expr` beside `quote_hole`, at the
        // literal index the δ signatures can say. The lowering runs before
        // types exist, so the category is the one the author wrote on the
        // parameter the scrutinee stands behind; anything else binds at
        // `⟨token-tree⟩`, and a hole that then reaches an expression position
        // earns §7's wrong-category refusal rather than a silent crossing.
        let at_expression = matches!(
            subject.shape(),
            musa_calculus::RawShape::Var(name) if self.scrutinee_is_expression(name)
        );
        let bound = self.mint("subject");
        let mut fallback = None;
        let mut tests = Vec::new();
        // The shapes seen so far, for the unreachability half of §4's rule:
        // one shape is one coverage, so a second arm written with it can never
        // be selected. The template is the shape — the holes are indices into
        // it, so the names they bind do not enter the comparison, which is the
        // law's "a shape is a shape however its holes are spelled".
        let mut covered: Vec<Template> = Vec::new();
        for arm in arms {
            let written = child(arm, |kind| kind == SyntaxKind::Pattern)?;
            let span = crate::resolve::trimmed_span(&written);
            let at = self.origin(&written);
            let Some(quoted) = child(&written, |kind| kind == SyntaxKind::QuotePattern) else {
                if super::values::matches_a_literal(arm) {
                    // Dropping the arm would be worse than refusing it: the
                    // chain would lower to a term the core accepts, with one
                    // written arm gone and nothing to say so.
                    return self.refuse(
                        Diagnostic::error(
                            Code::PatternCategory,
                            "a quote pattern and a literal pattern match different types",
                        )
                        .at(span, "this arm matches a literal")
                        .help(
                            "match the syntax value with a quote pattern, or match a literal on a different scrutinee",
                        )
                        .note("one scrutinee has one type, and a quote pattern's is a syntax value"),
                    );
                }
                let body = child(arm, is_expr_node)?;
                let body = self.expr(&body)?;
                fallback.get_or_insert(body);
                continue;
            };
            let mut walk = Walk::matching();
            let (template, quotation) = self.body(
                &musa_language::ast::QuotePattern::cast(quoted.clone())?.body()?,
                &mut walk,
            )?;
            self.holes_are_distinct(&walk, span)?;
            // Every word the pattern matches literally, for the restatement's
            // answer to "the arm uses what the pattern only matched" — see
            // [`Sites::quoted_literal`]. A splice's own name is the binding,
            // not a literal.
            let literals: Vec<String> = quoted
                .descendants_with_tokens()
                .filter_map(SyntaxElement::into_token)
                .filter(|token| token.kind() == SyntaxKind::Identifier)
                .filter(|token| {
                    !token
                        .parent()
                        .and_then(|held| held.parent())
                        .is_some_and(|held| matches!(held.kind(), SyntaxKind::Splice | SyntaxKind::SequenceSplice))
                })
                .map(|token| token.text().to_owned())
                .collect();
            if let Some(written_body) = child(arm, is_expr_node) {
                let body_origin = self.origin(&written_body);
                self.sites
                    .quote_arm(body_origin, crate::resolve::trimmed_span(&written_body), literals);
            }
            if covered.contains(&template) {
                return self.refuse(
                    Diagnostic::error(Code::UnreachablePattern, "this match arm can never be selected")
                        .at(span, "already covered above")
                        .note("a quote pattern is one shape, and an earlier arm wrote the same one"),
                );
            }
            covered.push(template.clone());
            let body = child(arm, is_expr_node)?;
            let mut body = self.expr(&body)?;
            let literal = crate::registry::template_literal(template, quotation);
            // Innermost first, so the bindings read in the order they were
            // written. They are independent — each reads the subject the arm
            // already matched — so the order is legibility rather than meaning.
            for (index, (name, sequence)) in walk.bound.iter().enumerate().rev() {
                let read = match (*sequence, at_expression) {
                    (false, false) => "quote_hole",
                    (true, false) => "quote_holes",
                    (false, true) => "quote_hole_expr",
                    (true, true) => "quote_holes_expr",
                };
                let held = applied(
                    at,
                    Raw::var(at, read),
                    [
                        Raw::var(at, bound.as_str()),
                        Raw::lit(at, literal.clone()),
                        whole(at, u64::try_from(index).unwrap_or(u64::MAX)),
                    ],
                );
                body = Raw::bind(at, name.as_str(), held, body);
            }
            tests.push((at, literal, body));
        }
        // §4's other half: a quote pattern constrains and does not enumerate,
        // so the chain cannot end on its last shape the way a constructor
        // match can end on its last case. The catch-all is the arm coverage
        // requires, and its absence is the exhaustiveness refusal — stated
        // here, because the chain this lowers to is `Bool` matches and the
        // core's coverage could never see what was not written.
        let Some(built) = fallback else {
            return self.refuse(
                Diagnostic::error(Code::NonExhaustiveMatch, "this match leaves a possible value uncovered")
                    .at(crate::resolve::trimmed_span(node), "add a fallback arm")
                    .note(
                        "a quote pattern constrains and does not enumerate: no finite set of shapes                          exhausts the token trees, so the last arm is a pattern that matches anything",
                    ),
            );
        };
        let mut built = built;
        for (at, literal, body) in tests.into_iter().rev() {
            let matches = applied(
                at,
                Raw::hosted(at, "match_quote"),
                [Raw::var(at, bound.as_str()), Raw::lit(at, literal)],
            );
            built = Raw::match_on(
                at,
                [matches],
                vec![
                    RawArm {
                        patterns: vec![RawPattern::constructor(at, "Bool.True", [])],
                        body,
                    },
                    RawArm {
                        patterns: vec![RawPattern::constructor(at, "Bool.False", [])],
                        body: built,
                    },
                ],
            );
        }
        Some(Raw::bind(origin, bound.as_str(), subject, built))
    }

    /// Whether a quote may be written here at all (`02-core-calculus.md` §5.9).
    fn quotable(&mut self, node: &SyntaxNode) -> Option<()> {
        if self.in_phase {
            return Some(());
        }
        self.refuse(
            Diagnostic::error(Code::Misplaced, "a quote is an adapter's form")
                .at(
                    crate::resolve::trimmed_span(node),
                    "this is syntax, and a piece is not written in syntax",
                )
                .note("`Syntax<Cat>` exists in the expansion phase; a piece can neither name one nor obtain one"),
        )
    }

    /// One quote body, as the template it builds and the site it builds at.
    ///
    /// The quotation counter is taken here rather than by the caller because
    /// this is the position that consumes one: `11-quotation.md` §3 needs two
    /// quotes with identical bodies at one anchor to build distinguishable
    /// nodes, and the counter is what distinguishes them.
    fn body(&mut self, node: &SyntaxNode, walk: &mut Walk) -> Option<(Template, u32)> {
        let quotation = self.resolver.next_quotation;
        self.resolver.next_quotation = quotation.saturating_add(1);
        let template = self.template(node, &mut crate::syntax::template_root(), walk, false)?;
        Some((template, quotation))
    }

    /// One node of a quote's body, as the shape it will build.
    ///
    /// `path` is where this node sits in the template, which is what its derived
    /// identity is computed from; `spreadable` is whether the position this node
    /// stands in admits a run of siblings rather than one node.
    ///
    /// Trivia is dropped. A read region keeps it because its text has to come
    /// back; a quote's does not — the printer re-spaces every expansion, and a
    /// comment written inside a quote belongs to the adapter's source rather
    /// than to what it builds.
    fn template(
        &mut self,
        node: &SyntaxNode,
        path: &mut Vec<u32>,
        walk: &mut Walk,
        spreadable: bool,
    ) -> Option<Template> {
        if node.kind() == SyntaxKind::Error {
            return Some(Template::Missing);
        }
        if matches!(node.kind(), SyntaxKind::Splice | SyntaxKind::SequenceSplice) {
            return self.splice(node, walk, spreadable);
        }
        // A binder is found before the body that refers to it, because the
        // grammar puts it first: a lambda's parameters precede its body and a
        // pattern precedes its arm. So one left-to-right walk with a scope stack
        // is enough, and no second pass is needed to tie a use to its
        // declaration.
        let framed = matches!(node.kind(), SyntaxKind::LambdaExpr | SyntaxKind::MatchArm);
        let frame = walk.scope.len();
        let separated = matches!(
            node.kind(),
            SyntaxKind::ExprArgList | SyntaxKind::ListExpr | SyntaxKind::ProductExpr
        );
        let pieces: Vec<SyntaxElement> = node
            .children_with_tokens()
            .filter(|piece| !piece.kind().is_trivia())
            // A separated position supplies its own commas
            // ([`crate::syntax::Template::Group::separated`]), so the ones
            // written here are not children of it.
            .filter(|piece| !(separated && piece.kind() == SyntaxKind::Comma))
            .collect();
        let (delimiter, pieces) = crate::syntax::delimited(pieces);
        // Where a spread may stand, which is the one rule the two directions do
        // not share. Building a run needs the separator its position supplies,
        // so only a comma-separated position has room for one. Matching one
        // needs no separator and only a run to bind, so any group that survives
        // [`crate::syntax::matched`]'s peeling will do — and a layout group
        // holding one node does not survive it, which is why the body of
        // `quote { $..xs }` is still a position that holds one node.
        let spreads = if walk.matching {
            delimiter != crate::syntax::Delimiter::Layout || pieces.len() >= 2
        } else {
            separated
        };
        let binders = binder_positions(node, &pieces);
        let mut children = Vec::with_capacity(pieces.len());
        let mut spread_here = false;
        for (index, piece) in pieces.iter().enumerate() {
            path.push(u32::try_from(index).unwrap_or(u32::MAX));
            let child = match piece {
                SyntaxElement::Node(inner) => self.template(&spread_argument(inner), path, walk, spreads),
                // A pattern writes no binder, so it needs no hygiene: it builds
                // nothing for a generated name to stand in, and §4 has it
                // compare names rather than scopes.
                SyntaxElement::Token(token) => {
                    self.quoted_token(token, path, walk, !walk.matching && binders.contains(&index))
                }
            };
            path.pop();
            let child = child?;
            // One spread per group, checked where the group is: two would leave
            // the split between them undetermined, and choosing it would be a
            // search over where the author meant one run to end
            // (`11-quotation.md` §4).
            if matches!(child, Template::Sequence(_)) {
                if spread_here {
                    return self.refuse(
                        Diagnostic::error(Code::AmbiguousSpread, "two spreads stand in one position")
                            .at(element_span(piece), "this is the second one")
                            .help("bind the run with one `$..xs` and take it apart afterwards")
                            .note("with two, where the first run ends is a guess, and this language does not search"),
                    );
                }
                spread_here = true;
            }
            children.push(child);
        }
        if framed {
            walk.scope.truncate(frame);
        }
        Some(Template::Group {
            delimiter,
            separated,
            children,
        })
    }

    /// One token of a quote's body.
    fn quoted_token(&mut self, token: &SyntaxToken, path: &[u32], walk: &mut Walk, binds: bool) -> Option<Template> {
        if token.kind() != SyntaxKind::Identifier {
            return Some(Template::Token {
                kind: token.kind(),
                text: token.text().to_owned(),
            });
        }
        let name = token.text();
        if looks_generated(name) {
            return self.refuse(
                Diagnostic::error(
                    Code::QuotedCapture,
                    format!("`{name}` is spelled like a name this quote could generate"),
                )
                .at(
                    token_span(token),
                    "a `_g` and a number is how a quoted binder is renamed on the way out",
                )
                .help("rename it; the suffix belongs to the compiler")
                .note("hygiene works by renaming, so a name that could be a renaming defeats it"),
            );
        }
        let hygiene = if binds {
            walk.scope.push((name.to_owned(), path.to_vec()));
            Hygiene::Binder
        } else if let Some((_, declared)) = walk
            .scope
            .iter()
            .rev()
            // Only a name *used as an expression* refers to a binding. A field
            // label, a method name, and a record's field are identifiers in
            // other positions, and renaming one because a nearby lambda took a
            // parameter by that name would change what the quote says.
            .find(|(held, _)| held == name && token.parent().is_some_and(|up| up.kind() == SyntaxKind::NameExpr))
        {
            Hygiene::Bound(declared.clone())
        } else {
            Hygiene::Free
        };
        Some(Template::Identifier {
            name: name.to_owned(),
            hygiene,
        })
    }

    /// One `$x`, `${ e }`, or `$..xs`.
    ///
    /// A construction's splice is lowered and no more. The old checker asked
    /// here whether the spliced value's category was `Expr`; the core asks it
    /// now, at this term's own origin, because `instantiate_quote`'s third
    /// argument is a `List (List (Syntax ⟨expr⟩))` and a `Syntax ⟨tokentree⟩`
    /// does not inhabit it. One answer rather than two that agree today.
    fn splice(&mut self, node: &SyntaxNode, walk: &mut Walk, spreadable: bool) -> Option<Template> {
        let span = crate::resolve::trimmed_span(node);
        let sequence = node.kind() == SyntaxKind::SequenceSplice;
        if sequence && !spreadable {
            // Two directions, two faults, and the *message* carries which,
            // because a message says what is wrong and these are two different
            // wrong things.
            let (complaint, advice, why) = if walk.matching {
                (
                    "nothing here holds a run to bind",
                    "write `$x` for the one node, or put the spread among a group's children",
                    "a spread binds a run of siblings, and this position has no siblings to run",
                )
            } else {
                (
                    "nothing here spreads a sequence",
                    "write `$x` for the one node, or move the spread into an argument list, `[…]`, or `(…, …)`",
                    "a spread needs the position's own separator, and only a comma-separated one has it",
                )
            };
            return self.refuse(
                Diagnostic::error(Code::UnspreadSequence, complaint)
                    .at(span, "this position holds one node")
                    .help(advice)
                    .note(why),
            );
        }
        if walk.matching {
            return self.pattern_splice(node, walk, sequence, span);
        }
        // Both forms hold one expression — `$..xs` wraps its name the way `$x`
        // does — so there is one path here and no second way to reach a spliced
        // name.
        let written = child(node, is_expr_node)?;
        let at = self.origin(&written);
        let expression = self.value(&written)?;
        let hole = walk.splices.len();
        // A hole's contribution is a *list* either way, which is what lets one
        // argument carry both kinds: a spread already has one, and a single
        // splice is the list of the one node it stands for.
        walk.splices.push(if sequence {
            expression
        } else {
            listed(at, vec![expression])
        });
        Some(if sequence {
            Template::Sequence(hole)
        } else {
            Template::Splice(hole)
        })
    }

    /// One `$x` or `$..xs` in a pattern, where a splice declares rather than
    /// computes.
    ///
    /// `11-quotation.md` §4: a pattern quote binds only splice variables. So the
    /// two spellings that name something are the two the form admits, and
    /// `${ e }` is not one of them — a braced splice holds an expression to be
    /// evaluated, and a pattern has nothing to evaluate it for. Refusing it here
    /// rather than in the grammar keeps one splice production for both
    /// directions, which is what stops the two from drifting.
    fn pattern_splice(
        &mut self,
        node: &SyntaxNode,
        walk: &mut Walk,
        sequence: bool,
        span: SourceSpan,
    ) -> Option<Template> {
        if node
            .children_with_tokens()
            .any(|piece| piece.kind() == SyntaxKind::LBrace)
        {
            return self.refuse(
                Diagnostic::error(Code::Misplaced, "a pattern has nothing to evaluate")
                    .at(span, "`${ … }` splices a value, and this position binds a name")
                    .help("write `$name` for the node this position holds")
                    .note("a quote that matches reads its holes; only a quote that builds fills them"),
            );
        }
        let name = child(node, |kind| kind == SyntaxKind::NameExpr)
            .and_then(|held| significant_tokens(&held).next())
            .map(|token| token.text().to_owned())?;
        let hole = walk.bound.len();
        walk.bound.push((name, sequence));
        Some(if sequence {
            Template::Sequence(hole)
        } else {
            Template::Splice(hole)
        })
    }

    /// That no two holes of one pattern bind the same name.
    ///
    /// Two holes of one name would be two nodes, and nothing in this language
    /// says they are the same node — §4 gives a pattern no equality test to make
    /// them one.
    fn holes_are_distinct(&mut self, walk: &Walk, span: SourceSpan) -> Option<()> {
        let mut seen = Vec::with_capacity(walk.bound.len());
        for (name, _) in &walk.bound {
            if seen.contains(&name.as_str()) {
                return self.refuse(
                    Diagnostic::error(Code::DuplicateName, format!("pattern binding `{name}` is repeated"))
                        .at(span, "bind each hole once")
                        .note("two holes of one name would be two nodes, and nothing here says they are the same node"),
                );
            }
            seen.push(name.as_str());
        }
        Some(())
    }
}

/// Which of a node's pieces are binders it declares.
///
/// The two positions the grammar marks unambiguously, and no others. A `Param`
/// declares its own name — the identifier before the annotation — and a
/// `Pattern` declares the names *after* its head. What a lone pattern head is
/// cannot be decided here: it is a constructor when the scrutinee's type has one
/// by that name and a binding otherwise, and the scrutinee's type belongs to the
/// expansion site, which the adapter's own reading cannot see. So a lone head is
/// written plain, and the residue is a quote whose `match e { x -> … }` can
/// shadow a composer's `x` — narrow, and stated rather than quietly claimed as
/// covered.
fn binder_positions(node: &SyntaxNode, pieces: &[SyntaxElement]) -> Vec<usize> {
    let identifiers = || {
        pieces
            .iter()
            .enumerate()
            .filter(|(_, piece)| piece.kind() == SyntaxKind::Identifier)
            .map(|(index, _)| index)
    };
    if node.kind() == SyntaxKind::Param {
        return identifiers().take(1).collect();
    }
    if node.kind() == SyntaxKind::Pattern {
        return identifiers().skip(1).collect();
    }
    Vec::new()
}

/// An `ExprArg` that holds nothing but a spread, seen through.
///
/// The wrapper is the parser's node for "one argument", and a spread is not one
/// argument — it is however many the list has. Leaving the wrapper in place would
/// put the run inside a group of its own, and the commas the argument list
/// supplies would go around that group rather than between its elements.
fn spread_argument(node: &SyntaxNode) -> SyntaxNode {
    if node.kind() != SyntaxKind::ExprArg {
        return node.clone();
    }
    let mut inside = node.children_with_tokens().filter(|piece| !piece.kind().is_trivia());
    match (inside.next(), inside.next()) {
        (Some(SyntaxElement::Node(only)), None) if only.kind() == SyntaxKind::SequenceSplice => only,
        _ => node.clone(),
    }
}

/// Whether a name is spelled the way the printer renames a generated binder.
///
/// `crate::syntax::print` appends `_g` and the scope's ordinal, so a name ending
/// that way is one the printer could have written — and a quote that writes it by
/// hand would be a name a generated binder could capture. The test is on the
/// spelling because the collision is on the spelling: the printed text is where
/// hygiene has to survive, and there the scopes are gone.
fn looks_generated(name: &str) -> bool {
    let Some((head, tail)) = name.rsplit_once("_g") else {
        return false;
    };
    !head.is_empty() && !tail.is_empty() && tail.bytes().all(|byte| byte.is_ascii_digit())
}

/// Where one piece of a quote body was written.
fn element_span(piece: &SyntaxElement) -> SourceSpan {
    let range = piece.text_range();
    SourceSpan::new(u32::from(range.start()), u32::from(range.end()))
}

/// Where one token was written.
fn token_span(token: &SyntaxToken) -> SourceSpan {
    SourceSpan::new(
        u32::from(token.text_range().start()),
        u32::from(token.text_range().end()),
    )
}
