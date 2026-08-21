//! How a core term is spelled when a refusal has to name it.
//!
//! A refusal that says "type mismatch" and stops is a refusal the reader has to
//! reconstruct: [`Mismatch`](crate::Mismatch) has held the two disagreeing
//! subterms since it was written, and until this module there was no way to say
//! them. Handing a consumer what the stage already computed is the whole of the
//! argument for it.
//!
//! **Core spelling, not surface spelling.** `Machine K A B` prints as an
//! application because that is what the term is; the surface writes
//! `Machine<K, A, B>` and this crate has never heard of angle brackets
//! (`02-core-calculus.md` §7). A reader who wrote the surface form still
//! recognizes the head and the arguments, which is what a mismatch is about.
//!
//! **Binders travel, so a variable has a name.** A term is de Bruijn-indexed and
//! a name is carried on the binder for exactly this reason (§7's α-equality
//! excludes it from conversion and keeps it for diagnostics). What is printed is
//! walked with a stack of the names it passed, so `(x : A) → x` prints `x` and
//! not an index. An index reaching past the stack is *free* in what is being
//! printed — a mismatch is a subterm, so this is ordinary — and prints as the
//! index it is rather than as a name that would be a guess.

use std::fmt::Write as _;
use std::sync::Arc;

use crate::term::{Filling, Name, Shape, Term};

/// How `term` is spelled, closed or not.
pub(crate) fn spelled(term: &Term) -> String {
    let mut out = String::new();
    write(&mut out, term, Level::Outer, &mut Vec::new());
    out
}

/// How the *head* of `term`'s application spine is spelled.
///
/// What a refusal about a call names: `feedback {?0} {?1} {?2} {?3} ⟨dict⟩ m`
/// is the elaborated term, and `feedback` is the word the author wrote. The
/// inserted arguments are peeled with the written ones because a message that
/// showed them would be reporting the elaborator's work rather than the call's.
pub(crate) fn head_spelled(term: &Term) -> String {
    spelled(spine(term).0)
}

/// Names in backticks, comma-separated, with `and` before the last.
///
/// A sentence rather than a debug list, because the reader of a refusal is
/// reading English: "nothing is given for `by`" and "nothing is given for
/// `by` and `then`" both end at the edit, and `["by", "then"]` ends at a
/// translation.
pub(crate) fn listed(names: &[Name]) -> String {
    let quoted: Vec<String> = names.iter().map(|name| format!("`{name}`")).collect();
    match quoted.split_last() {
        None => String::new(),
        Some((last, [])) => last.clone(),
        Some((last, rest)) => format!("{} and {last}", rest.join(", ")),
    }
}

/// How much of an expression a position may hold without parentheses.
///
/// Three, because there are three ways a term binds looser than its neighbour:
/// a binder extends to the right as far as it can, an application binds tighter
/// than an arrow, and an argument binds tighter than an application.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Level {
    /// Anything.
    Outer,
    /// An application or tighter: an arrow or a binder needs parentheses.
    Applied,
    /// An atom: an application needs them too.
    Argument,
}

fn write(out: &mut String, term: &Term, level: Level, names: &mut Vec<Name>) {
    match term.shape() {
        Shape::Hole(hole) => {
            let _ = write!(out, "{hole}");
        }
        Shape::Var(index) => {
            let depth = names.len();
            match depth
                .checked_sub(usize::try_from(index.0).unwrap_or(usize::MAX))
                .and_then(|above| above.checked_sub(1))
                .and_then(|level| names.get(level))
            {
                Some(name) => out.push_str(name),
                None => {
                    let _ = write!(out, "#{}", index.0);
                }
            }
        }
        Shape::Const(constant) => {
            let _ = write!(out, "{constant}");
        }
        Shape::Def(def) => {
            let _ = write!(out, "{def}");
        }
        Shape::Base(base) => {
            let _ = write!(out, "{base}");
        }
        Shape::Lit(literal) => {
            let _ = write!(out, "{literal}");
        }
        // `384`, not 384 `Nat.Succ`s. What the author wrote is what a
        // diagnostic, a hover, and a semantic hash should all say back.
        Shape::Numeral(numeral) => {
            let _ = write!(out, "{numeral}");
        }
        Shape::Builtin(builtin) => {
            let _ = write!(out, "{builtin}");
        }
        Shape::Universe(universe) => {
            let _ = write!(out, "Type {universe}");
        }
        Shape::App { .. } => parenthesized(out, level, Level::Applied, |out| {
            let (head, arguments) = spine(term);
            write(out, head, Level::Applied, names);
            for argument in arguments {
                out.push(' ');
                write(out, argument, Level::Argument, names);
            }
        }),
        Shape::Pi {
            filling,
            name,
            domain,
            codomain,
        } => parenthesized(out, level, Level::Outer, |out| {
            match filling {
                // An arrow, because nothing after it names the binder. Told by
                // the *name* rather than by an occurs check: every reading that
                // writes an unnamed Π says so here, and a binder a reader can
                // see mentioned is one they would rather see written out.
                Filling::Written if !mentioned(codomain) => {
                    write(out, domain, Level::Applied, names);
                    out.push_str(" → ");
                }
                Filling::Written => {
                    let _ = write!(out, "({name} : ");
                    write(out, domain, Level::Outer, names);
                    out.push_str(") → ");
                }
                Filling::Parameter => {
                    let _ = write!(out, "{{{name} : ");
                    write(out, domain, Level::Outer, names);
                    out.push_str("} → ");
                }
                // The constraint and not the dictionary type, because `{}` is
                // what every one of them reduces to and the trait is the whole
                // of what the binder means.
                Filling::Constraint(constraint) => {
                    let _ = write!(out, "[{}", constraint.class);
                    for argument in constraint.args.iter() {
                        out.push(' ');
                        write(out, argument, Level::Argument, names);
                    }
                    out.push_str("] → ");
                }
            }
            names.push(Arc::clone(name));
            write(out, codomain, Level::Outer, names);
            names.pop();
        }),
        Shape::Lam { name, body } => parenthesized(out, level, Level::Outer, |out| {
            let _ = write!(out, "λ{name}. ");
            names.push(Arc::clone(name));
            write(out, body, Level::Outer, names);
            names.pop();
        }),
        Shape::RecordType(fields) => {
            out.push_str("{ ");
            for (position, field) in fields.iter().enumerate() {
                if position > 0 {
                    out.push_str(", ");
                }
                let _ = write!(out, "{} : ", field.name);
                write(out, &field.term, Level::Outer, names);
                names.push(Arc::clone(&field.name));
            }
            for _ in fields.iter() {
                names.pop();
            }
            out.push_str(if fields.is_empty() { "}" } else { " }" });
        }
        Shape::Record(fields) => {
            out.push_str("{ ");
            for (position, field) in fields.iter().enumerate() {
                if position > 0 {
                    out.push_str(", ");
                }
                let _ = write!(out, "{} = ", field.name);
                write(out, &field.term, Level::Outer, names);
            }
            out.push_str(if fields.is_empty() { "}" } else { " }" });
        }
        Shape::Project { record, field } => {
            write(out, record, Level::Argument, names);
            let _ = write!(out, ".{field}");
        }
        Shape::Let { name, body, .. } => parenthesized(out, level, Level::Outer, |out| {
            let _ = write!(out, "let {name} in ");
            names.push(Arc::clone(name));
            write(out, body, Level::Outer, names);
            names.pop();
        }),
    }
}

/// `body`, in parentheses when `at` would bind looser than `needed`.
fn parenthesized(out: &mut String, needed: Level, at: Level, body: impl FnOnce(&mut String)) {
    let wrap = needed > at;
    if wrap {
        out.push('(');
    }
    body(out);
    if wrap {
        out.push(')');
    }
}

/// A spine's head and its arguments, outermost function first.
fn spine(term: &Term) -> (&Term, Vec<&Term>) {
    let mut arguments = Vec::new();
    let mut head = term;
    while let Shape::App { function, argument } = head.shape() {
        arguments.push(argument);
        head = function;
    }
    arguments.reverse();
    (head, arguments)
}

/// Whether the nearest enclosing binder is named anywhere in `term`.
fn mentioned(term: &Term) -> bool {
    occurs(term, 0)
}

fn occurs(term: &Term, depth: u32) -> bool {
    match term.shape() {
        Shape::Var(index) => index.0 == depth,
        Shape::App { function, argument } => occurs(function, depth) || occurs(argument, depth),
        Shape::Pi {
            filling,
            domain,
            codomain,
            ..
        } => {
            let constrains = match filling {
                Filling::Constraint(constraint) => constraint.args.iter().any(|argument| occurs(argument, depth)),
                Filling::Written | Filling::Parameter => false,
            };
            constrains || occurs(domain, depth) || occurs(codomain, depth.saturating_add(1))
        }
        Shape::Lam { body, .. } => occurs(body, depth.saturating_add(1)),
        Shape::Let { ty, value, body, .. } => {
            occurs(ty, depth) || occurs(value, depth) || occurs(body, depth.saturating_add(1))
        }
        Shape::RecordType(fields) => fields
            .iter()
            .enumerate()
            .any(|(position, field)| occurs(&field.term, depth.saturating_add(u32::try_from(position).unwrap_or(0)))),
        Shape::Record(fields) => fields.iter().any(|field| occurs(&field.term, depth)),
        Shape::Project { record, .. } => occurs(record, depth),
        // Closed, or a leaf. A metavariable stands for a closed term applied to
        // the binders in scope (`term.rs`), so it holds no index of its own.
        Shape::Const(_)
        | Shape::Def(_)
        | Shape::Base(_)
        | Shape::Lit(_)
        | Shape::Numeral(_)
        | Shape::Hole(_)
        | Shape::Builtin(_)
        | Shape::Universe(_) => false,
    }
}
