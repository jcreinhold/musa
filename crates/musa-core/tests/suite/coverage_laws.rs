//! §6.2's dependent `match`: what it compiles to, what it covers, and what it
//! refuses.
//!
//! Stated through the facade, like `family_laws.rs`, and stated as
//! *convertibility with the recursor the author would have written by hand*.
//! That is the strongest law available here and the only one worth having: a
//! test that asserted the compiler produced some particular case tree would fix
//! the compiler's internals rather than the language's meaning, and §6.2 fixes
//! the meaning.

use musa_core::{Cx, Raw, RawArm, RawPattern, Refusal, Term};

use crate::family_laws::{apply, core_constant, nat_context, type0, var, vec};
use crate::programs::WRITTEN;

/// `Nat` and `Vec`, both declared.
///
/// # Panics
///
/// If either declaration is refused, which would be a defect in this crate.
pub(crate) fn nat_vec_context() -> Cx {
    let (cx, _) = nat_context();
    let group = musa_core::declare(&cx, &vec()).expect("Vec is a declaration");
    cx.declaring(&group)
}

fn bind(name: &str) -> RawPattern {
    RawPattern::bind(WRITTEN, name)
}

fn con(name: &str, fields: impl IntoIterator<Item = RawPattern>) -> RawPattern {
    RawPattern::constructor(WRITTEN, name, fields)
}

fn arm(patterns: Vec<RawPattern>, body: Raw) -> RawArm {
    RawArm { patterns, body }
}

fn matching(subjects: impl IntoIterator<Item = Raw>, arms: Vec<RawArm>) -> Raw {
    Raw::match_on(WRITTEN, subjects, arms)
}

fn arrow(domain: Raw, codomain: Raw) -> Raw {
    Raw::pi(WRITTEN, "_", domain, codomain)
}

/// A written type as a core term, for a checking question.
///
/// # Panics
///
/// If it is not a type, which is a defect in the test that wrote it.
fn core(cx: &Cx, name: &str, ty: &Raw) -> Term {
    musa_core::infer(cx, ty)
        .unwrap_or_else(|error| panic!("{name}: {error}"))
        .0
}

/// `Nat → Nat`, the type most of these questions are asked at.
fn nat_to_nat(cx: &Cx) -> Term {
    core(cx, "Nat → Nat", &arrow(var("Nat"), var("Nat")))
}

/// A function applied to arguments, with its own type written down.
///
/// The annotation is not decoration: §2 gives a bare λ no inference rule, so a
/// function written inline says what it is before anything applies it.
fn applied(function: &Raw, ty: &Raw, arguments: impl IntoIterator<Item = Raw>) -> Raw {
    apply(Raw::annot(WRITTEN, function.clone(), ty.clone()), arguments)
}

/// The successor of a Nat literal, as many times as asked.
fn number(count: u32) -> Raw {
    (0..count).fold(var("Nat.Zero"), |built, _| apply(var("Nat.Succ"), [built]))
}

/// Assert two terms at `ty` are definitionally equal.
///
/// # Panics
///
/// If they are not, or if either is not a term at `ty`.
fn same(cx: &Cx, name: &str, ty: &Term, left: &Raw, right: &Raw) {
    let left = musa_core::check(cx, ty, left).unwrap_or_else(|error| panic!("{name} (left): {error}"));
    let right = musa_core::check(cx, ty, right).unwrap_or_else(|error| panic!("{name} (right): {error}"));
    assert!(
        musa_core::convertible(cx, ty, &left, &right).unwrap_or_else(|error| panic!("{name}: {error}")),
        "{name}"
    );
}

/// §6.2: a `match` *is* the recursor the author would otherwise have written.
///
/// The predecessor, both ways. Written as a whole function rather than as an
/// application to a literal, so the two are compared as functions and not merely
/// at the arguments this test happened to pick.
#[test]
fn a_match_is_the_recursor_it_compiles_to() {
    let cx = nat_vec_context();
    let ty = nat_to_nat(&cx);
    let by_match = Raw::lam(
        WRITTEN,
        "n",
        matching(
            [var("n")],
            vec![
                arm(vec![con("Nat.Zero", [])], var("Nat.Zero")),
                arm(vec![con("Nat.Succ", [bind("k")])], var("k")),
            ],
        ),
    );
    let by_recursor = Raw::lam(
        WRITTEN,
        "n",
        apply(
            var("Nat.elim"),
            [
                Raw::lam(WRITTEN, "_", var("Nat")),
                var("Nat.Zero"),
                Raw::lam(WRITTEN, "k", Raw::lam(WRITTEN, "ih", var("k"))),
                var("n"),
            ],
        ),
    );
    same(&cx, "the predecessor", &ty, &by_match, &by_recursor);
}

/// A compiled `match` is an ordinary core term: the re-checker, which knows
/// nothing about patterns, coverage, or case trees, accepts it.
///
/// The invariant that makes every other law here evidence about the language
/// rather than about one elaborator's bookkeeping. §6.2 compiles `match` away,
/// and this is the test that says "away" means *gone* — what reaches the core is
/// a recursor application and a few `let`s, and a second implementation of the
/// rules agrees it is well typed.
#[test]
fn a_compiled_match_re_checks_in_the_core() {
    let cx = nat_vec_context();
    let ty = nat_to_nat(&cx);
    let by_match = Raw::lam(
        WRITTEN,
        "n",
        matching(
            [var("n")],
            vec![
                arm(vec![con("Nat.Zero", [])], var("Nat.Zero")),
                arm(
                    vec![con("Nat.Succ", [bind("k")])],
                    matching(
                        [var("k")],
                        vec![
                            arm(vec![con("Nat.Zero", [])], var("Nat.Zero")),
                            arm(vec![bind("j")], apply(var("Nat.Succ"), [var("j")])),
                        ],
                    ),
                ),
            ],
        ),
    );
    let _compiled = musa_core::check(&cx, &ty, &by_match).expect("a nested match elaborates");
}

/// ch. 5's variable rule: a variable pattern in a split column is *expanded*
/// into one row per constructor, not deferred into a fall-through.
///
/// The distinction is visible because the catch-all comes second: with a fat bar
/// the first arm would fail into it, and with expansion the second arm is simply
/// the row that survives the `Zero` branch. §6.2 refuses the fat bar, so this is
/// the law that says which mechanism ran.
#[test]
fn a_variable_pattern_is_expanded_rather_than_deferred() {
    let cx = nat_vec_context();
    let ty = nat_to_nat(&cx);
    let by_match = Raw::lam(
        WRITTEN,
        "n",
        matching(
            [var("n")],
            vec![
                arm(vec![con("Nat.Succ", [bind("k")])], var("k")),
                // Binds the whole subject, and in the `Zero` branch that subject
                // is `Zero` — so the body may name it and get the refined value.
                arm(vec![bind("m")], var("m")),
            ],
        ),
    );
    let by_recursor = Raw::lam(
        WRITTEN,
        "n",
        apply(
            var("Nat.elim"),
            [
                Raw::lam(WRITTEN, "_", var("Nat")),
                var("Nat.Zero"),
                Raw::lam(WRITTEN, "k", Raw::lam(WRITTEN, "ih", var("k"))),
                var("n"),
            ],
        ),
    );
    same(&cx, "a catch-all after a constructor", &ty, &by_match, &by_recursor);
}

/// A nested pattern splits the column it names, and then the column its fields
/// opened.
#[test]
fn a_nested_pattern_splits_the_columns_it_opens() {
    let cx = nat_vec_context();
    let ty = nat_to_nat(&cx);
    // `n - 2`, clamped: 0 ↦ 0, 1 ↦ 0, 2 ↦ 0, 3 ↦ 1.
    let minus_two = Raw::lam(
        WRITTEN,
        "n",
        matching(
            [var("n")],
            vec![
                arm(vec![con("Nat.Zero", [])], var("Nat.Zero")),
                arm(vec![con("Nat.Succ", [con("Nat.Zero", [])])], var("Nat.Zero")),
                arm(vec![con("Nat.Succ", [con("Nat.Succ", [bind("k")])])], var("k")),
            ],
        ),
    );
    let nat = core_constant(&cx, "Nat");
    let written = arrow(var("Nat"), var("Nat"));
    musa_core::check(&cx, &ty, &minus_two).expect("a nested match covers Nat");
    for (argument, expected) in [(0, 0), (1, 0), (2, 0), (3, 1), (5, 3)] {
        same(
            &cx,
            "clamped subtraction",
            &nat,
            &applied(&minus_two, &written, [number(argument)]),
            &number(expected),
        );
    }
}

/// Several subjects are one matrix, split left to right.
#[test]
fn several_subjects_are_matched_as_one_matrix() {
    let cx = nat_vec_context();
    let ty = core(
        &cx,
        "Nat → Nat → Nat",
        &arrow(var("Nat"), arrow(var("Nat"), var("Nat"))),
    );
    // Zero on the left, and otherwise the right — written so that no single
    // column decides the answer, which is what makes it a matrix.
    let program = Raw::lam(
        WRITTEN,
        "a",
        Raw::lam(
            WRITTEN,
            "b",
            matching(
                [var("a"), var("b")],
                vec![
                    arm(vec![con("Nat.Zero", []), bind("_")], var("Nat.Zero")),
                    arm(vec![con("Nat.Succ", [bind("x")]), con("Nat.Zero", [])], var("Nat.Zero")),
                    arm(
                        vec![con("Nat.Succ", [bind("x")]), con("Nat.Succ", [bind("y")])],
                        apply(var("Nat.Succ"), [var("y")]),
                    ),
                ],
            ),
        ),
    );
    let nat = core_constant(&cx, "Nat");
    let written = arrow(var("Nat"), arrow(var("Nat"), var("Nat")));
    musa_core::check(&cx, &ty, &program).expect("a two-column matrix covers Nat × Nat");
    for (left, right, expected) in [(0, 3, 0), (2, 0, 0), (2, 3, 3), (1, 1, 1)] {
        same(
            &cx,
            "two columns",
            &nat,
            &applied(&program, &written, [number(left), number(right)]),
            &number(expected),
        );
    }
}

/// A `match` elaboration must refuse, and the refusal it owes.
///
/// Shared with `elaboration_laws.rs`'s coverage gate for the reason
/// [`crate::family_laws::refused_declarations`] is: four refusals are reached
/// only by a `match`, and a gate satisfied by whichever corpus happens to live
/// beside it is not a gate.
pub(crate) struct RefusedMatch {
    pub(crate) name: &'static str,
    pub(crate) raw: Raw,
    /// The type to check it against, written rather than elaborated: these
    /// programs are checked in [`nat_vec_context`], and a `Term` cannot be built
    /// without it.
    pub(crate) ty: Raw,
    pub(crate) expected: fn(&Refusal) -> bool,
}

/// Every way §6.2 says a `match` is wrong.
pub(crate) fn refused_matches() -> Vec<RefusedMatch> {
    vec![
        RefusedMatch {
            name: "a constructor of some other family",
            raw: Raw::lam(
                WRITTEN,
                "n",
                matching([var("n")], vec![arm(vec![con("Vec.Nil", [])], var("Nat.Zero"))]),
            ),
            ty: arrow(var("Nat"), var("Nat")),
            expected: |refusal| matches!(refusal, Refusal::NoSuchConstructor { .. }),
        },
        RefusedMatch {
            name: "a constructor with no arm",
            raw: Raw::lam(
                WRITTEN,
                "n",
                matching([var("n")], vec![arm(vec![con("Nat.Zero", [])], var("Nat.Zero"))]),
            ),
            ty: arrow(var("Nat"), var("Nat")),
            expected: |refusal| matches!(refusal, Refusal::IncompleteMatch { .. }),
        },
        RefusedMatch {
            name: "an arm an earlier one already covers",
            raw: Raw::lam(
                WRITTEN,
                "n",
                matching(
                    [var("n")],
                    vec![
                        arm(vec![bind("m")], var("m")),
                        arm(vec![con("Nat.Zero", [])], var("Nat.Zero")),
                    ],
                ),
            ),
            ty: arrow(var("Nat"), var("Nat")),
            expected: |refusal| matches!(refusal, Refusal::UnreachableBranch { .. }),
        },
    ]
}

/// Each of those, refused for the reason it is wrong.
///
/// The corpus is exercised here as well as by the coverage gate, so that a
/// failure names the law rather than only the gate.
#[test]
fn a_match_is_refused_for_the_reason_it_is_wrong() {
    let cx = nat_vec_context();
    for RefusedMatch {
        name,
        raw,
        ty,
        expected,
    } in refused_matches()
    {
        let ty = core(&cx, name, &ty);
        let Err(error) = musa_core::check(&cx, &ty, &raw) else {
            panic!("{name}: the match was admitted");
        };
        let refusal = crate::programs::refusal(name, error);
        assert!(expected(&refusal), "{name}: refused, but as `{refusal}`");
    }
}

/// `Nat → … → Nat`, with `columns` arguments.
fn nat_arrows(cx: &Cx, columns: usize) -> Term {
    let written = (0..columns).fold(var("Nat"), |built, _| arrow(var("Nat"), built));
    core(cx, "Nat → … → Nat", &written)
}

/// Note 44's interleaved-column program: one arm per column that constrains
/// *that* column and nothing else, and a catch-all.
///
/// ```text
/// λ x₀ … x_{k-1}. match x₀, …, x_{k-1} {
///   Zero, y, …, y  => 0,
///   y, Zero, …, y  => 0,
///   …
///   y, …, y, Zero  => 0,
///   z₀, …, z_{k-1} => 30,
/// }
/// ```
fn interleaved(columns: usize) -> Raw {
    let mut arms = Vec::with_capacity(columns.saturating_add(1));
    for constrained in 0..columns {
        let patterns = (0..columns)
            .map(|column| {
                if column == constrained {
                    con("Nat.Zero", [])
                } else {
                    bind("y")
                }
            })
            .collect();
        arms.push(arm(patterns, number(0)));
    }
    arms.push(arm((0..columns).map(|_| bind("z")).collect(), number(30)));
    let body = matching((0..columns).map(|column| var(&subject(column))), arms);
    (0..columns)
        .rev()
        .fold(body, |built, column| Raw::lam(WRITTEN, subject(column), built))
}

/// The name of the interleaved program's `column`th subject.
fn subject(column: usize) -> String {
    format!("x{column}")
}

/// The elaborated size of a `match`, as a proxy a law can compare across widths.
fn interleaved_size(cx: &Cx, columns: usize) -> usize {
    let ty = nat_arrows(cx, columns);
    let compiled = musa_core::check(cx, &ty, &interleaved(columns)).expect("the interleaved program elaborates");
    format!("{compiled:?}").len()
}

/// §6.2's column selection is *necessary*, so a program whose arms each
/// constrain a different column grows polynomially rather than exponentially.
///
/// Note 44 measured this exact program at 2.2× per added column in both term
/// size and elaboration time, because `testable` scanned every row for a
/// constructor and split a column the first row did not need — duplicating that
/// row's body into each branch and asking the same question again in both. The
/// first row decides now, so a row that matches everything ends the search.
///
/// Stated as a ratio rather than a byte count: what the law fixes is the
/// *growth*, and a pinned size would be a fact about `Debug` formatting. The
/// bound is set at doubling the columns rather than at any particular exponent,
/// because what makes the program tractable is that adding a column multiplies
/// nothing — the measured series is quadratic, and a quadratic doubling is 4×.
#[test]
fn an_interleaved_match_grows_polynomially_in_its_columns() {
    let cx = nat_vec_context();
    let narrow = interleaved_size(&cx, 3);
    let wide = interleaved_size(&cx, 6);
    let ratio = (wide as f64) / (narrow as f64);
    assert!(
        ratio < 4.0,
        "3 columns: {narrow} chars, 6 columns: {wide} chars — {ratio:.2}× across a doubling is not polynomial growth"
    );
}

/// A `match` whose goal is still a metavariable elaborates, at the level the
/// goal already stands at.
///
/// §1.3 has no universe polymorphism, so a split reads its motive's universe off
/// the goal per use site — and a goal an argument position handed over is a
/// metavariable, which is not a written type and has no syntax to read. It is
/// not an *unknown* level, though: a metavariable records the type it stands at,
/// and a goal's type is `Type ℓ`. Reading it there is what lets a `match` be an
/// argument at all, which every `filter` written against a trait's postponed
/// element type turns out to need.
#[test]
fn a_match_whose_goal_is_a_metavariable_elaborates() {
    let cx = nat_vec_context();
    let ty = core(&cx, "Nat", &var("Nat"));
    let identity = Raw::implicit_pi(WRITTEN, "X", type0(), arrow(var("X"), var("X")));
    let predecessor = matching(
        [number(3)],
        vec![
            arm(vec![con("Nat.Zero", [])], var("Nat.Zero")),
            arm(vec![con("Nat.Succ", [bind("k")])], var("k")),
        ],
    );
    // A `let` rather than an annotated λ applied here: `same` re-checks, and an
    // elimination applied straight to an introduction form is the one shape the
    // re-checker cannot give a type to.
    let applied = Raw::annotated_bind(
        WRITTEN,
        "identity",
        identity,
        Raw::implicit_lam(WRITTEN, "X", Raw::lam(WRITTEN, "x", var("x"))),
        apply(var("identity"), [predecessor]),
    );
    same(
        &cx,
        "the predecessor through an implicit identity",
        &ty,
        &applied,
        &number(2),
    );
}
