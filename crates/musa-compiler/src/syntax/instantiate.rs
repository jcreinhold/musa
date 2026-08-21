//! Instantiating a template: the captured splices written back into a tree
//! of freshly derived paths.
//!
//! One concern of the `syntax` module; see its docs for the calculus.

use super::build::{binder, group, identifier, reference, token};
use super::matching::{Hygiene, Spliced, Template};
use super::path::{Derived, NodePath};
use super::tree::{SourceInfo, Syntax};

/// Build one quote's answer: `template`, with `spliced` in its holes, derived
/// from `origin` by construction site `quotation`.
///
/// **This is where provenance is minted** (`11-quotation.md` §3). Every node the
/// template writes is [`SourceInfo::Generated`] at a [`Derived`] path, and every
/// node that arrived through a splice is copied in with the identity it already
/// had. So the author supplies no number: the origin comes from the anchor, the
/// quotation from the elaborator's counter, and the path from the position in
/// the template — which is fixed when the quote is read, so a sequence splice
/// expanding to three nodes does not shift what its siblings are called.
///
/// `None` where a hole's value has the wrong arity for the position it stands
/// in, which the checker has already refused; it is here as well because a
/// total evaluator cannot assume its own checker ran.
pub(crate) fn instantiate(
    template: &Template,
    origin: &NodePath,
    quotation: u32,
    spliced: &[Spliced],
) -> Option<Syntax> {
    build(template, origin, quotation, spliced, &mut template_root())
}

/// Where a walk of a quote's template starts.
///
/// One step in, never at the origin itself: what a quote builds outermost is a
/// node of its own, and two quotes reading one input node have to disagree
/// about where that node sits — see [`Derived`]'s `path` field.
///
/// Both walks start here. The checker records a quoted binder's position
/// against its walk of the body and [`build`] rebuilds that position when it
/// instantiates, so a binder and the uses that name it are one binding only
/// while the two walks agree about the first step.
pub(crate) fn template_root() -> Vec<u32> {
    vec![0]
}

fn build(
    template: &Template,
    origin: &NodePath,
    quotation: u32,
    spliced: &[Spliced],
    path: &mut Vec<u32>,
) -> Option<Syntax> {
    let at = |path: &Vec<u32>| {
        Derived {
            origin: origin.clone(),
            quotation,
            path: path.clone(),
        }
        .path()
    };
    match template {
        Template::Missing => Some(Syntax::Missing(SourceInfo::Generated(at(path)))),
        Template::Token { kind, text } => Some(token(at(path), *kind, text.clone())),
        Template::Identifier { name, hygiene } => Some(match hygiene {
            Hygiene::Free => identifier(at(path), name.clone()),
            // The binder's own path *is* its binding's, so a quote that writes
            // two binders writes two names and one written twice is one name,
            // with nothing allocated either way.
            Hygiene::Binder => binder(&at(path).binding(quotation), name.clone()),
            Hygiene::Bound(declared) => reference(at(path), &at(&declared.clone()).binding(quotation), name.clone()),
        }),
        Template::Group {
            delimiter,
            separated,
            children,
        } => {
            let mut elements: Vec<Syntax> = Vec::with_capacity(children.len());
            for (index, child) in children.iter().enumerate() {
                if let Template::Sequence(hole) = child {
                    let Spliced::Many(values) = spliced.get(*hole)? else {
                        return None;
                    };
                    elements.extend(values.iter().cloned());
                    continue;
                }
                path.push(u32::try_from(index).unwrap_or(u32::MAX));
                let node = build(child, origin, quotation, spliced, path);
                path.pop();
                elements.push(node?);
            }
            if *separated {
                // Every child of a separated position is one element, and so is
                // every node a spread put there, so one comma goes between each
                // consecutive pair and none goes at either end. The commas are
                // numbered past the template's own children, so they cannot
                // collide with a literal node's path however long the spread
                // turns out to be.
                let mut minted = u32::try_from(children.len()).unwrap_or(u32::MAX);
                let mut separated_elements = Vec::with_capacity(elements.len().saturating_mul(2));
                for element in elements {
                    if !separated_elements.is_empty() {
                        path.push(minted);
                        separated_elements.push(token(at(path), musa_language::SyntaxKind::Comma, ",".to_owned()));
                        path.pop();
                        minted = minted.saturating_add(1);
                    }
                    separated_elements.push(element);
                }
                elements = separated_elements;
            }
            Some(group(at(path), *delimiter, elements))
        }
        Template::Splice(hole) => match spliced.get(*hole)? {
            Spliced::One(node) => Some(node.clone()),
            Spliced::Many(_) => None,
        },
        // Reached only if a sequence splice stood where no group could spread
        // it, which the checker refuses with a diagnostic that can say where.
        Template::Sequence(_) => None,
    }
}
