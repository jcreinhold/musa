//! The two ways into a syntax value, as structural eliminators.
//!
//! `recurse_syntax` and `syntax_fold_from_leaves` are §5.8's *second* family and
//! not its first: each takes four branch functions, so neither is a δ-builtin,
//! and each fires on a `Syntax` literal and rewrites to a term the core then
//! evaluates. That is the shape prompt 141c gave `musa-calculus`, and this is the
//! first thing to use it.
//!
//! # What a rewrite may name, and how
//!
//! Three sources, and between them they cover everything these two need.
//!
//! - **The literal it fired on.** A group's children, a token's kind and text, a
//!   node's own path — read off the payload and written back as literals at the
//!   same base types the registration used.
//! - **The spine's arguments, by index.** The answer is evaluated in an
//!   environment built from the arguments innermost-last, so the last argument is
//!   `Index(0)` and a traversal's type parameters are as nameable as its branches
//!   are. `Answer` is an argument, so `List Answer` is writable.
//! - **The registered vocabulary.** `List.Empty`, `List.Cons`, `SyntaxStep`, and
//!   `SyntaxStep.Step` are constructors of *declared* families, which no amount
//!   of indexing reaches. [`Builtin::structural_with`] takes them as closed terms
//!   at registration and [`Builtin::vocabulary`] hands them back.
//!
//! # Termination
//!
//! §5.8's structural obligation is that every application of the builtin inside
//! the answer stands at a literal strictly smaller than the target. Both
//! traversals apply themselves only at an immediate *child* of the group they
//! fired on, and a child of a finite tree is strictly smaller in the node count
//! that measures it. `syntax_fold_from_leaves` applies itself directly;
//! `recurse_syntax` applies itself under a λ, which delays the call and does not
//! change what it stands at. §4's meter is the backstop, not the argument.
//!
//! # Why a sealed step is a λ and not a payload
//!
//! A step captures which child and which algebra. A base type cannot hold a
//! closure, and a payload holding a de Bruijn index would be meaningless the
//! moment it left the spine it was minted in. A λ in a rewrite's answer becomes a
//! value that holds its environment, which is exactly the capture wanted — so the
//! seal moved to [`crate::prelude`]'s `SyntaxStep`, whose one constructor is
//! private to the phase module.

#[cfg(test)]
mod laws;

use musa_calculus::{Builtin, Cx, ElabError, Index, Literal, Term};

use super::rules::Kind;
use super::{HERE, held, literal, plain_type, syntax_type, type0};
use crate::syntax::{Cat, Delimiter, Syntax};

/// The two traversals, registered against the context that declared the prelude.
///
/// # Errors
///
/// [`ElabError`] when a name a vocabulary or a signature reaches for is not
/// declared, which is a defect in this compiler rather than in any program.
pub(super) fn eliminators(cx: &Cx) -> Result<Vec<Builtin>, ElabError> {
    Ok(vec![
        Builtin::structural_with(
            SPELLINGS[0],
            recurse_type(cx)?,
            musa_calculus::Family::Eliminator,
            Recurse::SUBJECT,
            vocabulary(cx, &["List.Empty", "List.Cons", "SyntaxStep", "SyntaxStep.Step"])?,
            rewrite_recurse,
        ),
        Builtin::structural_with(
            SPELLINGS[1],
            fold_type(cx)?,
            musa_calculus::Family::Eliminator,
            Fold::SUBJECT,
            vocabulary(cx, &["List.Empty", "List.Cons"])?,
            rewrite_fold,
        ),
    ])
}

/// The phase rows registered here rather than in [`super::rules`].
///
/// Named rather than counted, because "which two" is the claim: the third row of
/// `SYNTAX_OWNERSHIP`'s descent family is `run_syntax_step`, which is defined
/// over `SyntaxStep` and not registered at all.
pub(super) const SPELLINGS: [&str; 2] = ["recurse_syntax", "syntax_fold_from_leaves"];

/// The constants a rewrite may write, resolved by name.
///
/// Resolved through [`crate::prelude::constant`] rather than assembled here, so
/// that the term a rewrite splices and the term a source program's `List.Cons`
/// denotes are one term and not two that agree today. Each is closed because a
/// declared constant is closed by construction, which is the invariant
/// [`Builtin::structural_with`] leaves to its caller.
fn vocabulary(cx: &Cx, names: &[&str]) -> Result<Vec<Term>, ElabError> {
    names.iter().map(|name| crate::prelude::constant(cx, name)).collect()
}

/// Where each vocabulary entry stands, for the rewrite that reads it back.
mod word {
    pub(super) const EMPTY: usize = 0;
    pub(super) const CONS: usize = 1;
    pub(super) const SEALED: usize = 2;
    pub(super) const STEP: usize = 3;
}

/// `recurse_syntax`'s arguments, by position.
///
/// Named positions rather than numbers written three times each — in the
/// signature, in the rewrite, and in the recursive call under the λ. A signature
/// and a rewrite that disagreed about which argument is the group branch would
/// both type-check and together answer nonsense.
struct Recurse;

impl Recurse {
    const CONTEXT_TYPE: usize = 0;
    const ANSWER: usize = 1;
    const MISSING: usize = 2;
    const TOKEN: usize = 3;
    const IDENTIFIER: usize = 4;
    const GROUP: usize = 5;
    const CONTEXT: usize = 6;
    const SUBJECT: usize = 7;
    /// How many arguments it is written with, which is also how many binders its
    /// answer is read under.
    const ARITY: usize = 8;
}

/// `syntax_fold_from_leaves`'s arguments, by position.
struct Fold;

impl Fold {
    const ANSWER: usize = 0;
    const MISSING: usize = 1;
    const TOKEN: usize = 2;
    const IDENTIFIER: usize = 3;
    const GROUP: usize = 4;
    const SUBJECT: usize = 5;
    const ARITY: usize = 6;
}

// ---- counting binders -------------------------------------------------------

/// The variable at argument position `position`, seen from `depth` binders in.
///
/// One conversion from a position to a de Bruijn index, serving both the
/// signatures and the rewrites, because both count the same binders: a signature
/// counts its own Π binders left to right, and the core pushes a spine's
/// arguments innermost-last so that the last one is `Index(0)`. Those are the
/// same numbering, which is why one function does.
fn at(depth: usize, position: usize) -> Term {
    let index = depth.saturating_sub(position).saturating_sub(1);
    Term::var(HERE, Index(u32::try_from(index).unwrap_or_default()))
}

/// `domain → codomain`.
///
/// Still a Π and still binds, which is why a codomain is always written one
/// depth further in than its domain.
fn arrow(domain: Term, codomain: Term) -> Term {
    Term::pi(HERE, "argument", domain, codomain)
}

/// A Π-telescope over `binders`, ending in `body`.
///
/// Each domain must already be written at the depth its own position gives it,
/// and `body` at the full arity. Folding from the right is what makes that true:
/// the last binder is applied first and every earlier one wraps it.
fn telescope(binders: Vec<(&'static str, Term)>, body: Term) -> Term {
    binders
        .into_iter()
        .rev()
        .fold(body, |built, (name, domain)| Term::pi(HERE, name, domain, built))
}

/// The same, over type parameters a use site does not write.
///
/// Implicit because that is what a type parameter is everywhere else in this
/// language: `fn list_fold_from_start<A, B>(…)` is called `list_fold_from_start(
/// seed, step, values)`, and a builtin whose answer type had to be spelled out
/// would be the one generic function in the language with a different calling
/// convention — for no reason a reader could name, since the branches determine
/// it exactly as a library function's arguments do.
///
/// It changes the *type* and not the term. An implicit argument is still an
/// argument, so both rewrites still read their branches at the positions
/// [`Recurse`] and [`Fold`] give them, and the arity is what it was.
fn parameterized(names: &[&'static str], body: Term) -> Term {
    names
        .iter()
        .rev()
        .fold(body, |built, name| Term::implicit_pi(HERE, *name, type0(), built))
}

/// One branch of a traversal: `arguments`, then the traversal's answer.
///
/// `written` is where the branch itself is bound. Each argument must already be
/// written at `written + k` for the k-th, and the answer lands at
/// `written + arguments.len()` — which is what the caller has to know to write a
/// depth-dependent argument like the group branch's list.
fn branch(answer: &dyn Fn(usize) -> Term, written: usize, arguments: Vec<Term>) -> Term {
    let taken = arguments.len();
    arguments
        .into_iter()
        .rev()
        .fold(answer(written.saturating_add(taken)), |built, argument| {
            arrow(argument, built)
        })
}

/// `head a b …`.
fn applied(head: Term, arguments: impl IntoIterator<Item = Term>) -> Term {
    arguments
        .into_iter()
        .fold(head, |function, argument| Term::app(HERE, function, argument))
}

/// `Syntax ⟨token-tree⟩`, the one category a traversal descends into.
///
/// `Expr` is reached only by the checked parse and by the gate, which are the two
/// operations that run the real parser. A traversal is handed a tree nobody has
/// promised anything about.
fn tree_type() -> Term {
    syntax_type(Cat::TokenTree)
}

// ---- the two signatures -----------------------------------------------------

/// ```text
/// recurse_syntax
///   : {Context : Type 0} → {Answer : Type 0}
///   → (Context → NodePath → Answer)
///   → (Context → NodePath → TokenKind → Text → Answer)
///   → (Context → NodePath → Text → Answer)
///   → (Context → NodePath → Delimiter → List (SyntaxStep Context Answer) → Answer)
///   → Context → Syntax ⟨token-tree⟩ → Answer
/// ```
///
/// Two parameters, both ordinary: an adapter may inherit a function and answer
/// with one. `SyntaxStep Context Answer` is a declared family applied to them
/// rather than a quantifier of its own, which is what keeps this one telescope.
///
/// # Errors
///
/// [`ElabError`] when `List` or `SyntaxStep` is not declared in `cx`.
fn recurse_type(cx: &Cx) -> Result<Term, ElabError> {
    let list = crate::prelude::constant(cx, "List")?;
    let sealed = crate::prelude::constant(cx, "SyntaxStep")?;
    let context = |depth| at(depth, Recurse::CONTEXT_TYPE);
    let answer = |depth| at(depth, Recurse::ANSWER);
    let path = || plain_type("NodePath");
    // The list of steps is the third argument of the branch bound at `GROUP`, so
    // it is written three binders past it, and its member mentions both
    // parameters. This is the one depth in either signature that is not a closed
    // type, and the reason `branch` states where its arguments are read.
    let inside = Recurse::GROUP.saturating_add(3);
    let steps = Term::app(HERE, list, applied(sealed, [context(inside), answer(inside)]));
    Ok(parameterized(
        &["Context", "Answer"],
        telescope(
            vec![
                (
                    "missing",
                    branch(&answer, Recurse::MISSING, vec![context(Recurse::MISSING), path()]),
                ),
                (
                    "token",
                    branch(
                        &answer,
                        Recurse::TOKEN,
                        vec![
                            context(Recurse::TOKEN),
                            path(),
                            plain_type("TokenKind"),
                            plain_type("Text"),
                        ],
                    ),
                ),
                (
                    "identifier",
                    branch(
                        &answer,
                        Recurse::IDENTIFIER,
                        vec![context(Recurse::IDENTIFIER), path(), plain_type("Text")],
                    ),
                ),
                (
                    "group",
                    branch(
                        &answer,
                        Recurse::GROUP,
                        vec![context(Recurse::GROUP), path(), plain_type("Delimiter"), steps],
                    ),
                ),
                ("context", context(Recurse::CONTEXT)),
                ("subject", tree_type()),
            ],
            answer(Recurse::ARITY),
        ),
    ))
}

/// ```text
/// syntax_fold_from_leaves
///   : {Answer : Type 0}
///   → (NodePath → Answer)
///   → (NodePath → TokenKind → Text → Answer)
///   → (NodePath → Text → Answer)
///   → (NodePath → Delimiter → List Answer → Answer)
///   → Syntax ⟨token-tree⟩ → Answer
/// ```
///
/// [`recurse_type`] at a context nothing reads, which is what "derived" means
/// here: the same traversal in its other mode, with every child already read
/// before its group's branch runs.
///
/// # Errors
///
/// [`ElabError`] when `List` is not declared in `cx`.
fn fold_type(cx: &Cx) -> Result<Term, ElabError> {
    let list = crate::prelude::constant(cx, "List")?;
    let answer = |depth| at(depth, Fold::ANSWER);
    let path = || plain_type("NodePath");
    let inside = Fold::GROUP.saturating_add(2);
    let read = Term::app(HERE, list, answer(inside));
    Ok(parameterized(
        &["Answer"],
        telescope(
            vec![
                ("missing", branch(&answer, Fold::MISSING, vec![path()])),
                (
                    "token",
                    branch(
                        &answer,
                        Fold::TOKEN,
                        vec![path(), plain_type("TokenKind"), plain_type("Text")],
                    ),
                ),
                (
                    "identifier",
                    branch(&answer, Fold::IDENTIFIER, vec![path(), plain_type("Text")]),
                ),
                (
                    "group",
                    branch(&answer, Fold::GROUP, vec![path(), plain_type("Delimiter"), read]),
                ),
                ("subject", tree_type()),
            ],
            answer(Fold::ARITY),
        ),
    ))
}

// ---- what a node is written back as -----------------------------------------

/// The node's own path, which is what makes a derived path derivable.
///
/// Read off the node rather than reconstructed, so a transformer cannot reach a
/// node without also holding the path it would build output from.
fn node_path(node: &Syntax) -> Term {
    literal(plain_type("NodePath"), node.info().path().clone()).term(HERE)
}

fn text(spelling: &str) -> Term {
    literal(plain_type("Text"), spelling.to_owned()).term(HERE)
}

fn kind(which: musa_language::SyntaxKind) -> Term {
    literal(plain_type("TokenKind"), Kind(which)).term(HERE)
}

fn delimiter(which: Delimiter) -> Term {
    literal(plain_type("Delimiter"), which).term(HERE)
}

/// A child, at the same type its parent literal carried.
///
/// The type comes off the target rather than being rebuilt, because a child of a
/// `Syntax ⟨token-tree⟩` is one too and rebuilding it would be a second place for
/// the category to be decided.
fn child(ty: &Term, node: &Syntax) -> Term {
    literal(ty.clone(), node.clone()).term(HERE)
}

/// `List ⟨member⟩`, from its elements in order.
///
/// Built from the tail, which is what a cons list is; the reversal that needs is
/// this function's business rather than its callers'.
fn listing(builtin: &Builtin, member: &Term, elements: Vec<Term>) -> Option<Term> {
    let empty = builtin.vocabulary().get(word::EMPTY)?;
    let cons = builtin.vocabulary().get(word::CONS)?;
    let mut built = Term::app(HERE, empty.clone(), member.clone());
    for element in elements.into_iter().rev() {
        built = applied(cons.clone(), [member.clone(), element, built]);
    }
    Some(built)
}

// ---- the two rewrites -------------------------------------------------------

/// One step down into a group, sealed.
///
/// `SyntaxStep.Step Context Answer (fn (context) { recurse_syntax … context
/// ⟨child⟩ })`. The λ is where the capture happens: `Context`, `Answer`, and the
/// four branches are read from the enclosing spine, and the one thing left open
/// is the context the runner supplies. Everything inside it is one binder further
/// away, which is the whole of why [`at`] takes a depth.
fn step(builtin: &Builtin, ty: &Term, node: &Syntax) -> Option<Term> {
    let outside = |position| at(Recurse::ARITY, position);
    let inside = |position| at(Recurse::ARITY.saturating_add(1), position);
    let resumed = applied(
        builtin.term(HERE),
        [
            inside(Recurse::CONTEXT_TYPE),
            inside(Recurse::ANSWER),
            inside(Recurse::MISSING),
            inside(Recurse::TOKEN),
            inside(Recurse::IDENTIFIER),
            inside(Recurse::GROUP),
            Term::var(HERE, Index(0)),
            child(ty, node),
        ],
    );
    Some(applied(
        builtin.vocabulary().get(word::STEP)?.clone(),
        [
            outside(Recurse::CONTEXT_TYPE),
            outside(Recurse::ANSWER),
            Term::lam(HERE, "context", resumed),
        ],
    ))
}

/// `recurse_syntax` at a node: the matching branch, applied to what the node
/// holds.
///
/// Under a group, each child becomes a sealed step rather than an answer, so the
/// branch decides whether, in what order, and under what context each is read.
fn rewrite_recurse(builtin: &Builtin, subject: &Literal) -> Option<Term> {
    let node = held::<Syntax>(subject)?;
    let arg = |position| at(Recurse::ARITY, position);
    let here = node_path(node);
    Some(match *node {
        Syntax::Missing(_) => applied(arg(Recurse::MISSING), [arg(Recurse::CONTEXT), here]),
        Syntax::Token {
            kind: which,
            text: ref spelling,
            ..
        } => applied(
            arg(Recurse::TOKEN),
            [arg(Recurse::CONTEXT), here, kind(which), text(spelling)],
        ),
        Syntax::Identifier { ref name, .. } => {
            applied(arg(Recurse::IDENTIFIER), [arg(Recurse::CONTEXT), here, text(name)])
        }
        Syntax::Group {
            delimiter: which,
            ref children,
            ..
        } => {
            let member = applied(
                builtin.vocabulary().get(word::SEALED)?.clone(),
                [arg(Recurse::CONTEXT_TYPE), arg(Recurse::ANSWER)],
            );
            let minted = children
                .iter()
                .map(|kid| step(builtin, subject.ty(), kid))
                .collect::<Option<Vec<_>>>()?;
            applied(
                arg(Recurse::GROUP),
                [
                    arg(Recurse::CONTEXT),
                    here,
                    delimiter(which),
                    listing(builtin, &member, minted)?,
                ],
            )
        }
    })
}

/// `syntax_fold_from_leaves` at a node.
///
/// The same four branches with the context dropped, and a group's children read
/// before its branch runs — the recursive call stands in the list rather than
/// under a λ, so the core evaluates it on the way to the branch.
fn rewrite_fold(builtin: &Builtin, subject: &Literal) -> Option<Term> {
    let node = held::<Syntax>(subject)?;
    let arg = |position| at(Fold::ARITY, position);
    let here = node_path(node);
    Some(match *node {
        Syntax::Missing(_) => applied(arg(Fold::MISSING), [here]),
        Syntax::Token {
            kind: which,
            text: ref spelling,
            ..
        } => applied(arg(Fold::TOKEN), [here, kind(which), text(spelling)]),
        Syntax::Identifier { ref name, .. } => applied(arg(Fold::IDENTIFIER), [here, text(name)]),
        Syntax::Group {
            delimiter: which,
            ref children,
            ..
        } => {
            let read = children
                .iter()
                .map(|kid| {
                    applied(
                        builtin.term(HERE),
                        [
                            arg(Fold::ANSWER),
                            arg(Fold::MISSING),
                            arg(Fold::TOKEN),
                            arg(Fold::IDENTIFIER),
                            arg(Fold::GROUP),
                            child(subject.ty(), kid),
                        ],
                    )
                })
                .collect::<Vec<_>>();
            applied(
                arg(Fold::GROUP),
                [here, delimiter(which), listing(builtin, &arg(Fold::ANSWER), read)?],
            )
        }
    })
}
