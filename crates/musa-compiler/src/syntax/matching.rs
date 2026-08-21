//! Matching a value against a quoted template, and what a splice captures.
//!
//! One concern of the `syntax` module; see its docs for the calculus.

use super::category::Delimiter;
use super::tree::Syntax;

/// A quote's body, with a hole where each splice stands.
///
/// The shape [`read_region`] would have read out of the same text, minus the
/// positions the author left open. Keeping it is what lets one `quote at …` be
/// read once, at its definition, and instantiated at every call: the tree does
/// not depend on the environment, and what does — the anchor and each splice's
/// value — is a hole here and an expression in the checked quote.
///
/// It carries no source information at all. Every node a quote writes is
/// generated, and its path is a function of the anchor, the quotation, and the
/// position in *this* tree, which is exactly what [`instantiate`] computes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Template {
    /// A node the parser expected inside the body and did not find.
    Missing,
    Token {
        kind: musa_language::SyntaxKind,
        text: String,
    },
    Identifier {
        name: String,
        hygiene: Hygiene,
    },
    Group {
        delimiter: Delimiter,
        /// Whether this position separates its elements with commas.
        ///
        /// §2: "the separator a sequence splice needs — the commas of an
        /// argument list — is supplied by the grammar of the position". So the
        /// commas the body writes are *not* children of a separated group: one
        /// is minted between every pair of elements at instantiation, which is
        /// what makes `f($a, $..rest)` right for a `rest` of any length, the
        /// empty one included. A group that kept the written commas and spread
        /// beside them would leave a trailing one there, and musa rejects a
        /// trailing comma.
        separated: bool,
        children: Vec<Self>,
    },
    /// `$x` or `${ e }` — one node, from the splice at this index.
    Splice(usize),
    /// `$..xs` — a run of nodes, from the splice at this index. Legal only as a
    /// separated group's child, which is the only place a run of siblings has
    /// both room and a separator.
    Sequence(usize),
}

/// What a quoted name refers to.
///
/// `11-quotation.md` §4's two halves, decided while the body is read rather
/// than while it is instantiated: a name the quote itself binds gets a scope
/// derived from where the binder stands, and every other name is written plain
/// and means what it meant where the quote was written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Hygiene {
    /// A name the quote does not bind.
    Free,
    /// A binder the quote writes.
    Binder,
    /// A use of a binder the quote writes, named by that binder's position in
    /// this template.
    Bound(Vec<u32>),
}

/// One splice's value, at instantiation — and what matching hands back.
///
/// One type for both directions, because they are one relation read two ways:
/// [`instantiate`] takes these and a template and answers a value, and
/// [`matched`] takes that value and the same template and answers these. A
/// separate "what a pattern bound" type would be the same three words with a
/// second name, and the law that the two are inverse would have to be stated
/// across a conversion instead of as an equation.
#[derive(Clone, Debug)]
pub(crate) enum Spliced {
    One(Syntax),
    Many(Vec<Syntax>),
}

/// Match `value` against `template`, binding each of its `holes`.
///
/// `11-quotation.md` §4's pattern form, and the inverse of [`instantiate`]:
/// `matched(t, &instantiate(t, …, s)?, s.len())` is `s` again, node for node.
/// `None` where the value has a different shape, which is a pattern doing its
/// job rather than a failure.
///
/// **What "the same shape" means**, since matching is where it has to be said
/// exactly:
///
/// - **Trivia is not shape.** A read region keeps every comment and space it
///   was written with, and a template kept none, so the significant children
///   are what is compared. A comment between two spliced elements therefore
///   does not defeat a match, which is what keeps an adapter from breaking
///   when a composer runs the formatter.
/// - **The parser's bracketing is not shape.** A quote's body goes through the
///   expression grammar, so `a` inside an argument list arrives wrapped in the
///   layout groups the CST puts around a name, an argument, and an operand; a
///   region's own reader wraps nothing. Peeling a layout group that holds one
///   node is what makes those two the same tree — and it is the whole of the
///   difference, because a layout group holding *two* nodes is a shape the
///   grammar meant.
/// - **A separated position's commas are not shape either.** They are supplied
///   by the position ([`Template::Group::separated`]) and a template has none,
///   so a value's are read as what separates its elements: exactly one between
///   each pair, none at either end.
/// - **Provenance is not shape at all.** Nothing here reads a [`SourceInfo`],
///   a [`Scope`], or a [`Derived`], so a node a quote built and a node a
///   composer wrote match alike — §4's second rule, and the reason an adapter
///   cannot ask where a node came from.
pub(crate) fn matched(template: &Template, value: &Syntax, holes: usize) -> Option<Vec<Spliced>> {
    let mut bound: Vec<Option<Spliced>> = vec![None; holes];
    if !match_one(peel_template(template), peel(value), &mut bound) {
        return None;
    }
    bound.into_iter().collect()
}

/// One peeled template against one peeled value.
fn match_one(template: &Template, value: &Syntax, bound: &mut [Option<Spliced>]) -> bool {
    match template {
        Template::Splice(hole) => bind(bound, *hole, Spliced::One(value.clone())),
        // A run of nodes is not one node. The checker refuses a spread that
        // stands where one node stands, and this is the evaluator refusing to
        // guess what it would have meant.
        Template::Sequence(_) => false,
        Template::Missing => matches!(value, Syntax::Missing(_)),
        Template::Token { kind, text } => {
            matches!(value, Syntax::Token { kind: found, text: written, .. } if found == kind && written == text)
        }
        // Names only: a scope is a coordinate the compiler owns, and §4's
        // second rule is that a pattern reads nothing of the sort.
        Template::Identifier { name, .. } => matches!(value, Syntax::Identifier { name: found, .. } if found == name),
        Template::Group {
            delimiter,
            separated,
            children,
        } => {
            let Syntax::Group {
                delimiter: found,
                children: held,
                ..
            } = value
            else {
                return false;
            };
            if found != delimiter {
                return false;
            }
            let items: Vec<&Syntax> = held.iter().filter(|child| !is_trivia(child)).collect();
            if !*separated {
                return match_run(children, &items, bound);
            }
            let Some(elements) = elements(&items) else {
                return false;
            };
            match_run(children, &elements, bound)
        }
    }
}

/// A run of templates against a run of values, with at most one spread.
///
/// The spread's length is decided by the two ends rather than searched for:
/// what stands before it matches from the front, what stands after it matches
/// from the back, and what is left over is the run it binds. Two spreads would
/// leave the split between them undetermined, so the checker refuses them and
/// this refuses to guess.
fn match_run(templates: &[Template], items: &[&Syntax], bound: &mut [Option<Spliced>]) -> bool {
    let peeled: Vec<&Template> = templates.iter().map(peel_template).collect();
    let mut spreads = peeled
        .iter()
        .enumerate()
        .filter_map(|(index, template)| match template {
            Template::Sequence(hole) => Some((index, *hole)),
            Template::Missing
            | Template::Token { .. }
            | Template::Identifier { .. }
            | Template::Group { .. }
            | Template::Splice(_) => None,
        });
    let Some((at, hole)) = spreads.next() else {
        return peeled.len() == items.len()
            && peeled
                .iter()
                .zip(items)
                .all(|(template, value)| match_one(template, peel(value), bound));
    };
    if spreads.next().is_some() {
        return false;
    }
    let after = peeled.len().saturating_sub(at).saturating_sub(1);
    let Some(tail) = items.len().checked_sub(after) else {
        return false;
    };
    if tail < at {
        return false;
    }
    let before = peeled
        .get(..at)
        .into_iter()
        .flatten()
        .zip(items.get(..at).into_iter().flatten());
    let behind = peeled
        .get(at.saturating_add(1)..)
        .into_iter()
        .flatten()
        .zip(items.get(tail..).into_iter().flatten());
    for (template, value) in before.chain(behind) {
        if !match_one(template, peel(value), bound) {
            return false;
        }
    }
    let run = items.get(at..tail).unwrap_or_default();
    bind(
        bound,
        hole,
        Spliced::Many(run.iter().map(|node| (*node).clone()).collect()),
    )
}

/// Fill one hole. Each hole is written by exactly one splice in the template
/// it came from, so there is nothing to reconcile here; a hole out of range is
/// a template the checker never made.
fn bind(bound: &mut [Option<Spliced>], hole: usize, value: Spliced) -> bool {
    let Some(slot) = bound.get_mut(hole) else {
        return false;
    };
    *slot = Some(value);
    true
}

/// A value with the parser's bracketing taken off.
///
/// See [`matched`] for why a layout group holding one node is not a shape.
fn peel(value: &Syntax) -> &Syntax {
    let mut node = value;
    loop {
        let Syntax::Group {
            delimiter: Delimiter::Layout,
            children,
            ..
        } = node
        else {
            return node;
        };
        let mut significant = children.iter().filter(|child| !is_trivia(child));
        let (Some(only), None) = (significant.next(), significant.next()) else {
            return node;
        };
        node = only;
    }
}

/// The same, on the pattern's side.
///
/// A separated group is never peeled, whatever its delimiter turned out to be:
/// its commas are part of how its children are read, and a group that lost
/// them would be read as a run of siblings instead of a list of elements.
fn peel_template(template: &Template) -> &Template {
    let mut node = template;
    loop {
        let Template::Group {
            delimiter: Delimiter::Layout,
            separated: false,
            children,
        } = node
        else {
            return node;
        };
        let [only] = children.as_slice() else {
            return node;
        };
        node = only;
    }
}

fn is_trivia(node: &Syntax) -> bool {
    matches!(node, Syntax::Token { kind, .. } if kind.is_trivia())
}

/// The elements a separated group holds, or `None` if its children are not
/// one node between each pair of commas.
///
/// `(a, b)` is two elements; `()` is none; `(a,)`, `(, a)` and `(a b, c)` are
/// not a separated position's shape at all, so nothing matches them.
fn elements<'a>(items: &[&'a Syntax]) -> Option<Vec<&'a Syntax>> {
    let mut out: Vec<&Syntax> = Vec::with_capacity(items.len());
    let mut element: Option<&Syntax> = None;
    for item in items {
        if matches!(item, Syntax::Token { kind, .. } if *kind == musa_language::SyntaxKind::Comma) {
            out.push(element.take()?);
        } else if element.replace(item).is_some() {
            return None;
        }
    }
    match element {
        Some(last) => out.push(last),
        None if out.is_empty() => {}
        None => return None,
    }
    Some(out)
}
