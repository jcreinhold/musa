//! The controls for the audit the kernel makes of what this side produces.
//!
//! [`compiled`](crate::kernel::recheck::compiled) answers about a `Compiled`,
//! and a `Compiled` is `pub(crate)` — so a test that hands it a *bad* one has
//! to live inside this crate. It lives on *this* side of the boundary because
//! `boundary_laws.rs` forbids `kernel/` to name anything here, and building the
//! tree a broken builder would have produced means writing raw syntax and
//! elaborating it. Elaboration may name the kernel; the kernel may not name
//! elaboration. That is the direction, and this file is what it implies.
//!
//! Nothing here is reachable through the front door, which is the point.
//! Elaboration refuses an uncovered `match` and a recursion it cannot see
//! descend, so the defects these controls stage are ones that would have to get
//! *past* it. The audit exists for that case, and a re-checker nobody has
//! watched reject anything is a function that returns `Ok`.
//!
//! The other constructs' controls are in `tests/suite/recheck_laws.rs`, which
//! is where the audit table naming all of them lives.

/// The two controls that cannot be written from outside this crate.
///
/// [`compiled`] answers about a [`Compiled`], and a `Compiled` is
/// `pub(crate)` — so a test that hands it a *bad* one has to live here. That is
/// not an accident of visibility: elaboration refuses an uncovered `match` and
/// a recursion it cannot see descend, so neither defect can be built through
/// the front door at all. This pass exists for the case where something got
/// past that, and the only way to watch it reject is to assemble the tree a
/// broken builder would have produced.
///
/// The other constructs' controls are in `tests/suite/recheck_laws.rs`, which
/// is where the audit table naming all of them lives.
#[cfg(test)]
#[allow(clippy::panic, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use std::sync::Arc;

    use crate::elaboration::raw::{RawBinder, RawConstructor, RawData, RawFamily};
    use crate::kernel::case_tree::{Alternative, CaseTree, Compiled, Split};
    use crate::kernel::context::Cx;
    use crate::kernel::error::{CoreError, Malformed};
    use crate::kernel::family::Constant;
    use crate::kernel::origin::Origin;
    use crate::kernel::recheck::compiled;
    use crate::kernel::sort::Sort;
    use crate::kernel::term::{Index, Name, Role, Term};
    use crate::kernel::visibility::Visibility;

    const HERE: Origin = Origin::node(700);

    fn name(text: &str) -> Name {
        Arc::from(text)
    }

    /// `data Bool { False, True }` — two constructors, so one alternative can
    /// be missing and the other still present.
    fn two_cases() -> RawData {
        let case = |spelling: &str| RawConstructor {
            origin: HERE,
            name: name(spelling),
            visibility: Visibility::Public,
            fields: Vec::<RawBinder>::new(),
            chosen: Vec::new(),
        };
        RawData {
            origin: HERE,
            params: Vec::new(),
            families: vec![RawFamily {
                name: name("Bool"),
                visibility: Visibility::Public,
                indices: Vec::new(),
                constructors: vec![case("False"), case("True")],
            }],
        }
    }

    #[test]
    fn the_kernel_rejects_a_tree_that_leaves_a_constructor_unanalysed() {
        let cx = Cx::new();
        let (group, _) = crate::elaboration::declare::declare(&cx, &two_cases()).expect("`Bool` is declarable");
        // A split naming `False` and nothing else. Every other field is as
        // small as it can be, because `uncovered` reads the group and the
        // alternatives' names and nothing more — asking the *declaration*
        // which constructors exist is the whole point of the rule.
        let split = Split {
            origin: HERE,
            group: Arc::clone(&group),
            family: 0,
            params: Arc::from([]),
            indices: Arc::from([]),
            motives: Arc::from([]),
            level: Sort::ZERO,
            on: Term::var(HERE, Index(0)),
            alternatives: Arc::from([Alternative {
                constructor: Constant::constructor(&group, 0, 0).name(),
                fields: Arc::from([]),
                hypotheses: Arc::from([]),
                body: CaseTree::Answer(Term::universe(HERE, Sort::ZERO)),
            }]),
        };
        let body = Compiled {
            binders: Arc::from([name("subject")]),
            tree: CaseTree::Split(Box::new(split)),
        };

        let fault = compiled(&name("decided"), &body).expect_err("a missing case is not a covered tree");
        match fault {
            CoreError::Malformed(Malformed::Uncovered(missing)) => {
                assert_eq!(&*missing, "Bool.True", "the control should name the case it dropped");
            }
            CoreError::Malformed(_) | CoreError::Exhausted(_) | CoreError::Refused { .. } => {
                panic!("the control should be rejected as uncovered, and was rejected as {fault}")
            }
        }
    }

    /// `data Nat { Zero, Succ(pred: Nat) }` — recursive, because only a
    /// recursive definition gets a *tree* body: an inline `match` is 155's
    /// emission standing in a term, which is a `Body::Value` like any other.
    fn counting() -> RawData {
        RawData {
            origin: HERE,
            params: Vec::new(),
            families: vec![RawFamily {
                name: name("Nat"),
                visibility: Visibility::Public,
                indices: Vec::new(),
                constructors: vec![
                    RawConstructor {
                        origin: HERE,
                        name: name("Zero"),
                        visibility: Visibility::Public,
                        fields: Vec::new(),
                        chosen: Vec::new(),
                    },
                    RawConstructor {
                        origin: HERE,
                        name: name("Succ"),
                        visibility: Visibility::Public,
                        fields: vec![RawBinder {
                            name: name("pred"),
                            ty: crate::elaboration::raw::Raw::var(HERE, "Nat"),
                        }],
                        chosen: Vec::new(),
                    },
                ],
            }],
        }
    }

    /// `copy : Nat -> Nat`, structurally recursive — a real tree, built by the
    /// real builder, so that swapping one branch is the *only* difference
    /// between the control and something elaboration accepted.
    fn copying() -> (Cx, Arc<crate::kernel::program::Program>) {
        use crate::elaboration::raw::{Raw, RawArm, RawPattern, RawProgram, RawTopLevel};

        let cx = Cx::new();
        let (group, _) = crate::elaboration::declare::declare(&cx, &counting()).expect("`Nat` is declarable");
        let cx = cx.declaring(&group);
        let var = |spelling: &str| Raw::var(HERE, spelling);
        let program = RawProgram {
            families: Vec::new(),
            definitions: vec![RawTopLevel {
                origin: HERE,
                name: name("copy"),
                visibility: Visibility::Public,
                module: None,
                ty: Some(Raw::pi(HERE, "_", var("Nat"), var("Nat"))),
                value: Raw::lam(
                    HERE,
                    "n",
                    Raw::match_on(
                        HERE,
                        [var("n")],
                        vec![
                            RawArm {
                                patterns: vec![RawPattern::constructor(HERE, "Nat.Zero", [])],
                                body: var("Nat.Zero"),
                            },
                            RawArm {
                                patterns: vec![RawPattern::constructor(
                                    HERE,
                                    "Nat.Succ",
                                    [RawPattern::bind(HERE, "k")],
                                )],
                                body: Raw::app(HERE, var("Nat.Succ"), Raw::app(HERE, var("copy"), var("k"))),
                            },
                        ],
                    ),
                ),
            }],
        };
        let (declared, _) = crate::elaboration::declare_program::declare_program(&cx, &program)
            .expect("`copy` is an ordinary structural recursion");
        (cx, declared)
    }

    /// `tree` with the first answer it reaches replaced by `answer`.
    fn rewritten(tree: &CaseTree, answer: &Term) -> CaseTree {
        match *tree {
            CaseTree::Answer(_) => CaseTree::Answer(answer.clone()),
            CaseTree::Impossible => CaseTree::Impossible,
            CaseTree::Split(ref split) => CaseTree::Split(Box::new(Split {
                origin: split.origin,
                group: Arc::clone(&split.group),
                family: split.family,
                params: Arc::clone(&split.params),
                indices: Arc::clone(&split.indices),
                motives: Arc::clone(&split.motives),
                level: split.level.clone(),
                on: split.on.clone(),
                alternatives: split
                    .alternatives
                    .iter()
                    .enumerate()
                    .map(|(position, alternative)| Alternative {
                        constructor: Arc::clone(&alternative.constructor),
                        fields: Arc::clone(&alternative.fields),
                        hypotheses: Arc::clone(&alternative.hypotheses),
                        body: if position == 0 {
                            rewritten(&alternative.body, answer)
                        } else {
                            CaseTree::Answer(alternative.body.emitted(HERE).expect("the arm emits"))
                        },
                    })
                    .collect(),
            })),
        }
    }

    /// The control for the invariant the *emission* carries, and it is the one
    /// nothing else in the crate re-states.
    ///
    /// A method's type is the motive instantiated at that method's own pattern,
    /// so a branch answering something else is rejected without this pass
    /// knowing what a tree is. That single mechanism is what discharges three
    /// obligations at once — 155's motive, 155a's tree-as-body, and 156's
    /// refuted branch, whose motive is a *large elimination over the subject's
    /// index* rather than a flag copied from the decision that refuted it. The
    /// control swaps one arm of a real tree for a term of the wrong type and
    /// watches the emitted eliminator spine stop type-checking.
    #[test]
    fn the_kernel_rejects_a_branch_that_does_not_answer_its_motive() {
        use crate::kernel::program::Body;

        let (cx, declared) = copying();
        let member = Arc::clone(declared.members().first().expect("one definition"));
        let Body::Compiled { ref tree, .. } = member.body else {
            panic!("a structural recursion on its whole argument compiles to a tree body");
        };
        // `Type 0` where a `Nat` belongs: a type standing in for a value, which
        // is wrong in a way the next conversion says out loud.
        let broken = rewritten(&tree.tree, &Term::universe(HERE, Sort::ZERO));
        let emitted = tree.binders.iter().rev().fold(
            broken.emitted(HERE).expect("the broken tree still emits"),
            |built, binder| Term::lam(HERE, Arc::clone(binder), built),
        );

        let inner = cx.defining(&declared);
        let mut meter = inner.meter();
        let fault = crate::kernel::recheck::checking(&inner, &mut meter, &emitted, &member.ty)
            .expect_err("a branch answering `Type 0` does not answer the motive");
        assert!(
            matches!(fault, CoreError::Malformed(Malformed::Mistyped { .. })),
            "the control should be rejected as mistyped, and was rejected as {fault}"
        );
    }

    #[test]
    fn the_kernel_rejects_a_recursion_that_does_not_descend() {
        // `loops x = loops x` — a call at the definition's own binder, which is
        // not smaller than itself. No family and no split: the descent rule
        // reads the call spine, and a tree that never analyses anything is the
        // smallest place a non-descending call can stand.
        let itself = name("loops");
        let call = Term::app(
            HERE,
            Term::named(HERE, Arc::clone(&itself), Role::Defined),
            Term::var(HERE, Index(0)),
        );
        let body = Compiled {
            binders: Arc::from([name("x")]),
            tree: CaseTree::Answer(call),
        };

        let fault = compiled(&itself, &body).expect_err("a call at its own argument does not descend");
        assert!(
            matches!(fault, CoreError::Malformed(Malformed::Undescending(_))),
            "the control should be rejected for its recursion, and was rejected as {fault}"
        );
    }
}
