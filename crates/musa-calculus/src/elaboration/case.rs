//! Compiling `match` to a case tree, and the case tree to generated recursors.
//!
//! §6.2 fixes the algorithm's shape and its sources: Peyton Jones ch. 5's
//! variable, constructor, empty, and mixture rules, with the `FAIL`/fat-bar
//! mechanism **not** adopted. The fat-bar exists so an equation can fail into the
//! next one, and Musa's arms do not fall through — so the mixture rule here
//! *expands* a variable pattern into one row per constructor instead of deferring
//! it to a fall-through, and the rows a split reaches are exactly the rows that
//! could still match.
//!
//! # Which column is split, and whether one is split at all
//!
//! The **first** row decides. If it has a variable in every remaining column it
//! matches whatever the subjects are, no later row at that node can be reached,
//! and the leaf is the answer; otherwise the column is the leftmost the first
//! row tests. That is Maranget's necessity condition specialized to a `match`
//! whose arms are ordered and do not fall through — testing a column the first
//! row does not need duplicates that row's body into every branch and asks the
//! same question again in each, which is exponential in the columns and answers
//! nothing. Scanning *every* row for a constructor, which is what this did
//! before, is the version of the rule for an unordered set of equations.
//!
//! An arm's body is still elaborated once per leaf it reaches, which is
//! Peyton Jones §5.4.1's remaining cost. The obvious repair — hoist the body
//! into a `let`-bound function the leaves apply — is not obviously correct here,
//! because a leaf binds its pattern variables by *definition* and a λ binder is
//! an assumption. `coverage_laws.rs` holds the program that shows the
//! difference, and prompt 165 owns the change.
//!
//! # The goal is refined as the tree descends
//!
//! A split emits `N.elim`, and a recursor's methods are typed at a **motive**.
//! §1.1's motive is a *family*: `λ(t : N p⃗). G`, the goal with the subject
//! abstracted out of it. A method is then checked at that motive **applied to
//! the constructor form it is the method for**, so an arm that matched `Cons`
//! is checked at the goal with `Cons h t` where the subject stood — which is
//! what refinement by matching means, and what the previous non-dependent
//! eliminator could not do.
//!
//! Abstracting the subject is a substitution, and this crate has none on terms
//! (§1). It is done in the semantic domain instead: the goal is quoted, then
//! re-evaluated in the environment the subject's binder was *rebound* in
//! ([`Env::rebinding`](crate::kernel::value::Env)). A goal that does not
//! mention the subject comes back unchanged, so a non-dependent `match` emits
//! the constant motive `λ_. G` and behaves exactly as it did.
//!
//! # Coverage is decided while the tree is built
//!
//! A split asks every constructor of the family for an arm. One with no arm is
//! [`Refusal::IncompleteMatch`], named. An arm that no leaf ever selects is
//! [`Refusal::UnreachableBranch`], reported once at the end rather than at each
//! leaf — a row is duplicated across branches by the mixture rule, so "this row
//! lost here" is not evidence about the arm.
//!
//! # Mutual families
//!
//! `N.elim` takes a motive and methods for **every** family of its group, not
//! just the one being matched. The siblings get the motive `G → G`, whose
//! methods are the identity — trivially inhabited at the goal's universe, which
//! `{}` would not be, since §1 makes universes non-cumulative. The cost is that a
//! `match` on one family of a mutual group cannot recurse into another: the
//! induction hypothesis for a sibling's field lands at `G → G` rather than at
//! `G`, and [`crate::elaboration::rec`] refuses such a call rather than mis-typing it.

use std::sync::{Arc, OnceLock};

use crate::elaboration::elab::Elaborator;
use crate::elaboration::raw::{Raw, RawArm, RawPattern};
use crate::elaboration::refuse::{ElabError, Refusal};
use crate::kernel::budget::Meter;
use crate::kernel::case_tree::{Alternative, CaseTree, Split as TreeSplit};
use crate::kernel::error::CoreError;
use crate::kernel::eval::{apply, eval, field_type, opened, project};
use crate::kernel::family::{Constant, Element, element};
use crate::kernel::origin::Origin;
use crate::kernel::quote::{quote, quote_type};
use crate::kernel::scope::Scope;
use crate::kernel::sort::Sort;
use crate::kernel::term::{Index, Level, Name, Shape, Term};
use crate::kernel::value::{Form, Head, Value};

/// Elaborate `match subjects… { arms… }` against `goal`.
///
/// # Errors
///
/// [`Refusal::NoSuchConstructor`], [`Refusal::IncompleteMatch`], and
/// [`Refusal::UnreachableBranch`] for the ways a match is wrong, and otherwise
/// as [`crate::check`] — an arm's body is ordinary elaboration.
pub(crate) fn compile(
    elaborator: &mut Elaborator,
    scope: &Scope,
    here: Origin,
    subjects: &[Raw],
    arms: &[RawArm],
    goal: &Value,
) -> Result<Term, ElabError> {
    // Each subject is elaborated under the binders the ones before it needed,
    // so a term read at one depth is never used at another.
    let mut inner = scope.clone();
    let mut bound = Vec::new();
    let mut columns = Vec::with_capacity(subjects.len());
    for raw in subjects {
        let (column, binder) = subject(elaborator, &inner, raw)?;
        if let Some((name, ty_term, value_term)) = binder {
            inner = inner.assume(Some(Arc::clone(&name)), raw.origin(), Arc::clone(&column.ty));
            bound.push((name, ty_term, value_term));
        }
        columns.push(column);
    }
    let mut rows = Vec::with_capacity(arms.len());
    for (which, arm) in arms.iter().enumerate() {
        if arm.patterns.len() != columns.len() {
            return Err(Refusal::IncompleteMatch {
                at: arm.body.origin(),
                constructor: Arc::from("a pattern for every subject"),
            }
            .into());
        }
        rows.push(Row {
            patterns: arm.patterns.iter().collect(),
            bindings: Vec::new(),
            arm: which,
        });
    }

    let mut tree = Tree {
        elaborator,
        here,
        arms,
        selected: vec![false; arms.len()],
    };
    let compiled = tree.solve(
        &inner,
        &Problem {
            columns,
            rows,
            goal: Arc::new(goal.clone()),
        },
    )?;
    // Coverage, re-derived from the declaration group rather than read off the
    // builder's bookkeeping — see [`CaseTree::uncovered`]. Asked before
    // reachability because a `match` missing a constructor is wrong about the
    // type it is matching, and an arm nothing selected is wrong about the arms.
    if let Some(constructor) = compiled.uncovered() {
        return Err(Refusal::IncompleteMatch { at: here, constructor }.into());
    }
    if let Some(arm) = tree.unselected() {
        return Err(Refusal::UnreachableBranch { at: arm }.into());
    }
    let mut term = compiled.emitted()?;
    for (name, ty, value) in bound.into_iter().rev() {
        term = Term::bind(here, name, ty, value, term);
    }
    Ok(term)
}

/// One subject, as the tree carries it.
///
/// A *value*, not a term: a subject outlives several splits and every split is at
/// a deeper context than the last, so a term would need shifting at each one.
/// Values are de Bruijn-levelled, so this one is written down once.
///
/// **The value is always a variable.** [`subject`] names anything else first,
/// and the invariant is what makes [`Split::read`]'s readback cost the size of
/// the subject's *type* rather than the size of its normal form.
struct Subject {
    value: Value,
    ty: Arc<Value>,
    /// Where the author wrote it, for a refusal about its type.
    at: Origin,
}

/// One row of the pattern matrix.
struct Row<'a> {
    /// One pattern per remaining column.
    patterns: Vec<&'a RawPattern>,
    /// The names a variable pattern bound, and what they stand for.
    ///
    /// Carried rather than applied immediately, because a variable pattern is
    /// consumed at a split that may still be several levels above the leaf where
    /// its body is elaborated.
    bindings: Vec<(Name, Value, Arc<Value>)>,
    /// Which arm this row came from.
    arm: usize,
}

/// What the leftmost tested column asks for.
enum Test {
    /// A case analysis on a family: emit its recursor.
    Split(usize),
    /// A record, whose one shape means the column becomes its fields.
    Open(usize),
}

/// The matrix, the subjects it is against, and the type its bodies answer.
struct Problem<'a> {
    columns: Vec<Subject>,
    rows: Vec<Row<'a>>,
    goal: Arc<Value>,
}

/// The state one `match` is compiled in.
struct Tree<'a, 'b> {
    elaborator: &'a mut Elaborator,
    /// Where the `match` was written, which every generated node carries (§7).
    here: Origin,
    arms: &'b [RawArm],
    /// Whether each arm was selected at some leaf.
    selected: Vec<bool>,
}

impl Tree<'_, '_> {
    /// The first arm no leaf selected.
    fn unselected(&self) -> Option<Origin> {
        self.selected
            .iter()
            .position(|selected| !selected)
            .and_then(|which| self.arms.get(which))
            .map(|arm| arm.body.origin())
    }

    /// Compile a matrix.
    fn solve(&mut self, scope: &Scope, problem: &Problem<'_>) -> Result<CaseTree, ElabError> {
        let Some(first) = problem.rows.first() else {
            // Reached only where a split found no arm for a constructor, which
            // names it; a matrix that starts empty is refused at `compile`.
            return Err(Refusal::IncompleteMatch {
                at: self.here,
                constructor: Arc::from("every constructor"),
            }
            .into());
        };
        match Self::testable(problem) {
            Some(Test::Split(column)) => self.split(scope, problem, column),
            // A record has one shape, so opening a column is not a case
            // analysis and emits no term: the column becomes one column per
            // field it names, and the same matrix is solved again. That is ch.
            // 5's constructor rule at a type with a single constructor, which is
            // what a record is.
            Some(Test::Open(column)) => {
                let opened = self.opened(scope, problem, column)?;
                self.solve(scope, &opened)
            }
            // Peyton Jones ch. 5's variable rule, and its empty rule: with no
            // constructor left to test, the first row wins outright — there is
            // nothing for a later row to be tried *after*.
            None => self.leaf(scope, first, &problem.columns, &problem.goal),
        }
    }

    /// Refuse a column whose subject has a base type and whose patterns take it
    /// apart.
    ///
    /// §5.8's D1 gives a base type no eliminator, so a column of one has
    /// nothing to split on and nothing to open: the only pattern that may stand
    /// there is the catch-all, which [`RawPattern::Bind`] already is — and a
    /// column of catch-alls is never tested, so coverage needs no rule here. A
    /// literal *pattern* is decidable equality rather than a case analysis, and
    /// arrives already desugared into a match on the host's `Bool`, which is
    /// prompt 141b's Design and [`RawPattern`]'s own doc.
    ///
    /// Asked before [`element`] and before the record unfolding, so the author
    /// hears that the type has no structure rather than that some constructor
    /// they did not write is missing.
    fn not_matchable(&mut self, problem: &Problem<'_>, column: usize) -> Result<Option<ElabError>, CoreError> {
        let Some(subject) = problem.columns.get(column) else {
            return Ok(None);
        };
        let meter = self.elaborator.meter();
        let unfolded = opened(meter, &subject.ty)?;
        let Form::Neutral(neutral) = &unfolded.as_ref().unwrap_or(&subject.ty).form else {
            return Ok(None);
        };
        let Head::Base(base, _) = &neutral.head else {
            return Ok(None);
        };
        let at = problem
            .rows
            .iter()
            .find_map(|row| match row.patterns.get(column)? {
                pattern @ (RawPattern::Constructor { .. } | RawPattern::Record { .. }) => Some(pattern.origin()),
                RawPattern::Bind { .. } => None,
            })
            .unwrap_or(subject.at);
        Ok(Some(
            Refusal::BaseNotMatchable {
                base: Arc::clone(base.name()),
                at,
            }
            .into(),
        ))
    }

    /// The leftmost column the **first** row tests, and what it tests it with.
    ///
    /// The first row, not any row, and that is Maranget's necessity condition
    /// specialized to an ordered `match`. A row whose every remaining pattern is
    /// a variable matches whatever the subjects are, so if the first row is that
    /// row it is the answer and no later row at this node can be reached — there
    /// is nothing a test could decide. Scanning every row instead finds a later
    /// row's constructor and splits on it, which duplicates the first row's body
    /// into each branch and asks the same question again in every one; several
    /// arms each constraining a different column made that exponential.
    ///
    /// Coverage is unaffected: a row that matches everything covers everything.
    /// Reachability is not — the rows this steps over are exactly the rows an
    /// earlier arm already covers, and `unselected` reports them, which is the
    /// verdict they had coming.
    ///
    /// One search rather than two, because "leftmost" has to be decided across
    /// both kinds: a matrix whose first column is opened and whose second is
    /// split must open first, or the split would be against subjects the open
    /// has not produced yet.
    fn testable(problem: &Problem<'_>) -> Option<Test> {
        problem
            .rows
            .first()?
            .patterns
            .iter()
            .enumerate()
            .find_map(|(column, pattern)| match pattern {
                RawPattern::Constructor { .. } => Some(Test::Split(column)),
                RawPattern::Record { .. } => Some(Test::Open(column)),
                RawPattern::Bind { .. } => None,
            })
    }

    /// The problem with a record column replaced by the fields its patterns
    /// name.
    ///
    /// Only the named fields, not every declared one: a record pattern says what
    /// it binds, and opening the rest would put a column of wildcards in the
    /// matrix for every field the author declined to mention.
    fn opened<'a>(&mut self, scope: &Scope, problem: &Problem<'a>, column: usize) -> Result<Problem<'a>, ElabError> {
        let Some(subject) = problem.columns.get(column) else {
            return Err(Refusal::IncompleteMatch {
                at: self.here,
                constructor: Arc::from("a pattern for every subject"),
            }
            .into());
        };
        let at = problem
            .rows
            .iter()
            .find_map(|row| match row.patterns.get(column)? {
                pattern @ RawPattern::Record { .. } => Some(pattern.origin()),
                RawPattern::Bind { .. } | RawPattern::Constructor { .. } => None,
            })
            .unwrap_or(subject.at);
        if let Some(refusal) = self.not_matchable(problem, column)? {
            return Err(refusal);
        }
        let meter = self.elaborator.meter();
        let unfolded = opened(meter, &subject.ty)?;
        let record_ty = Value::clone(unfolded.as_ref().unwrap_or(&subject.ty));
        let Form::RecordType(telescope) = &record_ty.form else {
            return Err(Refusal::NotARecord {
                at,
                ty: scope.quote_type(meter, &subject.ty)?,
            }
            .into());
        };
        let telescope = telescope.clone();
        let names = Self::opening(problem, column, &telescope)?;

        let mut columns = Vec::with_capacity(problem.columns.len().saturating_add(names.len()));
        let mut opened = Vec::with_capacity(names.len());
        for name in &names {
            let meter = self.elaborator.meter();
            opened.push(Subject {
                value: project(meter, subject.at, subject.value.clone(), name)?,
                ty: Arc::new(field_type(meter, &telescope, &subject.value, name)?),
                at: subject.at,
            });
        }
        for (position, held) in problem.columns.iter().enumerate() {
            if position == column {
                columns.append(&mut opened);
            } else {
                columns.push(Subject {
                    value: held.value.clone(),
                    ty: Arc::clone(&held.ty),
                    at: held.at,
                });
            }
        }

        let mut rows = Vec::with_capacity(problem.rows.len());
        for row in &problem.rows {
            let Some(pattern) = row.patterns.get(column) else {
                continue;
            };
            let mut bindings = row.bindings.clone();
            let inner: Vec<&RawPattern> = match pattern {
                RawPattern::Record { fields, .. } => names
                    .iter()
                    .map(|name| match fields.iter().find(|(field, _)| field == name) {
                        Some((_, sub)) => sub,
                        // A field this column opened that this row did not
                        // name: matched by a wildcard, exactly as at a split.
                        None => wildcard(),
                    })
                    .collect(),
                // A variable names the whole record and none of its fields, as
                // it does at a constructor split.
                RawPattern::Bind { name, .. } => {
                    bindings.push((Arc::clone(name), subject.value.clone(), Arc::clone(&subject.ty)));
                    vec![wildcard(); names.len()]
                }
                RawPattern::Constructor { origin, name, .. } => {
                    return Err(Refusal::NoSuchConstructor {
                        at: *origin,
                        name: Arc::clone(name),
                        ty: scope.quote_type(self.elaborator.meter(), &record_ty)?,
                        cases: Vec::new(),
                    }
                    .into());
                }
            };
            let mut patterns = Vec::with_capacity(row.patterns.len().saturating_add(names.len()));
            for (position, held) in row.patterns.iter().enumerate() {
                if position == column {
                    patterns.extend(inner.iter().copied());
                } else {
                    patterns.push(held);
                }
            }
            rows.push(Row {
                patterns,
                bindings,
                arm: row.arm,
            });
        }
        Ok(Problem {
            columns,
            rows,
            goal: Arc::clone(&problem.goal),
        })
    }

    /// The fields a record column opens, in telescope order.
    fn opening(
        problem: &Problem<'_>,
        column: usize,
        telescope: &crate::kernel::value::Telescope,
    ) -> Result<Vec<Name>, ElabError> {
        for row in &problem.rows {
            let Some(RawPattern::Record { origin, fields }) = row.patterns.get(column).copied() else {
                continue;
            };
            for (field, _) in fields {
                if !telescope.fields.iter().any(|declared| declared.name == *field) {
                    return Err(Refusal::NoSuchField {
                        at: *origin,
                        field: Arc::clone(field),
                    }
                    .into());
                }
            }
        }
        // Telescope order, not the order the first pattern happened to write:
        // a later field's type may mention an earlier field's value, so the
        // columns have to stand in the order the type does.
        Ok(telescope
            .fields
            .iter()
            .map(|declared| Arc::clone(&declared.name))
            .filter(|name| {
                problem.rows.iter().any(|row| {
                    matches!(row.patterns.get(column), Some(RawPattern::Record { fields, .. })
                        if fields.iter().any(|(field, _)| field == name))
                })
            })
            .collect())
    }

    /// Elaborate a row's body, with the names its patterns bound in scope.
    fn leaf(&mut self, scope: &Scope, row: &Row<'_>, columns: &[Subject], goal: &Value) -> Result<CaseTree, ElabError> {
        let Some(arm) = self.arms.get(row.arm) else {
            return Err(Refusal::IncompleteMatch {
                at: self.here,
                constructor: Arc::from("every constructor"),
            }
            .into());
        };
        if let Some(selected) = self.selected.get_mut(row.arm) {
            *selected = true;
        }
        // A pattern's binder is a *definition* rather than an assumption: it
        // stands for a value the context already holds, and defining it is what
        // makes `xs` in the body and the field it names convertible without a
        // rule that says so.
        //
        // Every binding is also a binder, so the body is elaborated deeper than
        // the method's own λs and has to come back out under one `let` each. A
        // scope extended without the matching `let` would read back at the wrong
        // depth — the classic way to answer an induction hypothesis where the
        // author wrote the field.
        let mut inner = scope.clone();
        let mut bound = Vec::with_capacity(row.bindings.len().saturating_add(columns.len()));
        // ch. 5's variable rule at the leaf: a column no split consumed still
        // has a pattern in it, and with nothing left to test that pattern is a
        // variable. It names the subject the column stands for — which, after a
        // split, is a field of the constructor that branch matched.
        let left: Vec<(Name, Value, Arc<Value>)> = row
            .patterns
            .iter()
            .zip(columns)
            .filter_map(|(pattern, subject)| match pattern {
                RawPattern::Bind { name, .. } => {
                    Some((Arc::clone(name), subject.value.clone(), Arc::clone(&subject.ty)))
                }
                RawPattern::Constructor { .. } | RawPattern::Record { .. } => None,
            })
            .collect();
        for (name, value, ty) in row.bindings.iter().chain(left.iter()) {
            let meter = self.elaborator.meter();
            let ty_term = inner.quote_type(meter, ty)?;
            let value_term = quote(meter, inner.depth(), crate::kernel::quote::Mode::Keep, ty, value)?;
            bound.push((Arc::clone(name), ty_term, value_term));
            inner = inner.define(self.elaborator.meter(), Arc::clone(name), Arc::clone(ty), value.clone())?;
        }
        let mut term = self.elaborator.check_open(&inner, &arm.body, goal)?;
        for (name, ty, value) in bound.into_iter().rev() {
            term = Term::bind(self.here, name, ty, value, term);
        }
        Ok(CaseTree::Answer(term))
    }

    /// Split on one column: a [`CaseTree::Split`] over the family, with one
    /// alternative per constructor of every family in its group.
    ///
    /// Nothing is emitted here. A missing constructor leaves a hole in the
    /// alternatives, and [`CaseTree::uncovered`] is what finds it — which is
    /// what "coverage is decided on the tree" (§1.1) buys over deciding it in
    /// the loop that happens to build the branches.
    fn split(&mut self, scope: &Scope, problem: &Problem<'_>, column: usize) -> Result<CaseTree, ElabError> {
        let Some(subject) = problem.columns.get(column) else {
            return Err(Refusal::IncompleteMatch {
                at: self.here,
                constructor: Arc::from("every constructor"),
            }
            .into());
        };
        let at = subject.at;
        if let Some(refusal) = self.not_matchable(problem, column)? {
            return Err(refusal);
        }
        let Some(found) = element(self.elaborator.meter(), &subject.ty)? else {
            return Err(self.not_a_constructor(scope, problem, column, at)?);
        };
        // A split *is* the case analysis `private` cases exist to prevent, so it
        // is the one place that has to ask. Reaching it means the first row
        // names a constructor: a `match` that only binds never splits, and
        // therefore never takes an abstract type apart, which is why this is
        // the split rather than the `match`.
        if let Some(module) = found.abstract_from(scope.cx().module()) {
            return Err(Refusal::AbstractMatch {
                family: found.name(),
                module,
                at,
            }
            .into());
        }
        self.belong(scope, problem, column, &found)?;
        let split = Analysed::read(self, scope, subject, &found, &problem.goal, at)?;
        let motives = self.motives(scope, problem, &split, subject)?;

        let mut alternatives = Vec::new();
        for family in 0..found.group.arity() {
            let count = found
                .group
                .family_at(family)
                .map_or(0, |declared| declared.constructors.len());
            for which in 0..count {
                let which = u32::try_from(which).unwrap_or(u32::MAX);
                if let Some(alternative) = self.method(scope, problem, &split, &motives, column, family, which)? {
                    alternatives.push(alternative);
                }
            }
        }
        Ok(CaseTree::Split(Box::new(TreeSplit {
            origin: self.here,
            group: Arc::clone(&found.group),
            family: found.family,
            params: Arc::from(split.params.clone()),
            motives: motives.iter().map(|motive| motive.term.clone()).collect(),
            level: split.level.clone(),
            on: split.target.clone(),
            alternatives: Arc::from(alternatives),
        })))
    }

    /// The refusal a subject whose type is not a family owes, named after the
    /// constructor the author actually wrote.
    fn not_a_constructor(
        &mut self,
        scope: &Scope,
        problem: &Problem<'_>,
        column: usize,
        at: Origin,
    ) -> Result<ElabError, CoreError> {
        let (name, origin) = problem
            .rows
            .iter()
            .find_map(|row| match row.patterns.get(column) {
                Some(RawPattern::Constructor { origin, name, .. }) => Some((Arc::clone(name), *origin)),
                Some(RawPattern::Bind { .. } | RawPattern::Record { .. }) | None => None,
            })
            .unwrap_or_else(|| (Arc::from("?"), at));
        let ty = problem
            .columns
            .get(column)
            .map(|subject| scope.quote_type(self.elaborator.meter(), &subject.ty))
            .transpose()?
            .unwrap_or_else(|| Term::universe(at, Sort::ZERO));
        // No cases to list: the subject's type is not a family, so there is no
        // declaration to read them off.
        Ok(Refusal::NoSuchConstructor {
            at: origin,
            name,
            ty,
            cases: Vec::new(),
        }
        .into())
    }

    /// Refuse a pattern naming a constructor of some other family.
    ///
    /// Asked once of the column rather than branch by branch: a pattern that
    /// matches nothing would otherwise surface as whichever real constructor
    /// found no arm, which names the wrong thing entirely — the author wrote a
    /// name, and the name is what is wrong.
    fn belong(
        &mut self,
        scope: &Scope,
        problem: &Problem<'_>,
        column: usize,
        found: &Element,
    ) -> Result<(), ElabError> {
        let count = found
            .group
            .family_at(found.family)
            .map_or(0, |declared| declared.constructors.len());
        let known: Vec<Name> = (0..count)
            .map(|which| {
                let which = u32::try_from(which).unwrap_or(u32::MAX);
                Constant::constructor(&found.group, found.family, which).name()
            })
            .collect();
        let ty = match problem.columns.get(column) {
            Some(subject) => scope.quote_type(self.elaborator.meter(), &subject.ty)?,
            None => Term::universe(self.here, Sort::ZERO),
        };
        for row in &problem.rows {
            match row.patterns.get(column).copied() {
                Some(RawPattern::Constructor { origin, name, .. }) => {
                    if known.iter().any(|constructor| selects(constructor, name)) {
                        continue;
                    }
                    return Err(Refusal::NoSuchConstructor {
                        at: *origin,
                        name: Arc::clone(name),
                        ty,
                        cases: known,
                    }
                    .into());
                }
                // A record pattern against a family. Reported as what it is —
                // the subject is not a record — rather than as a constructor
                // nobody wrote.
                Some(pattern @ RawPattern::Record { .. }) => {
                    return Err(Refusal::NotARecord {
                        at: pattern.origin(),
                        ty,
                    }
                    .into());
                }
                // A variable, or a row that ends before this column: neither
                // names a constructor, so neither can name a wrong one.
                Some(RawPattern::Bind { .. }) | None => {}
            }
        }
        Ok(())
    }

    /// One motive per family of the group, each a **family** of types.
    ///
    /// The family being split gets `λ(t : N p⃗). G` — the goal with the subject
    /// abstracted out, which is §1.1's dependent motive. Every other family
    /// gets `λ(_ : M p⃗). G → G`, which is inhabited at the goal's universe by
    /// the identity and says nothing about a value nobody matched.
    fn motives(
        &mut self,
        scope: &Scope,
        problem: &Problem<'_>,
        split: &Analysed,
        subject: &Subject,
    ) -> Result<Vec<Motive>, ElabError> {
        let depth = scope.depth();
        let mut built = Vec::new();
        for family in 0..split.element.group.arity() {
            let term = if family == split.element.family {
                self.abstracted(scope, subject, &problem.goal)?
            } else {
                // `Π (_ : G). G`, not `{}`: universes are not cumulative (§1),
                // so the empty record inhabits `Type 0` and nothing above it.
                // One binder deeper than it used to be, because the motive is
                // now a family and the Π sits under its subject binder.
                let domain = quote_type(
                    self.elaborator.meter(),
                    depth.deeper(),
                    crate::kernel::quote::Mode::Keep,
                    &problem.goal,
                )?;
                let codomain = quote_type(
                    self.elaborator.meter(),
                    depth.deeper().deeper(),
                    crate::kernel::quote::Mode::Keep,
                    &problem.goal,
                )?;
                Term::lam(
                    self.here,
                    "unmatched",
                    Term::pi(self.here, "impossible", domain, codomain),
                )
            };
            let value = scope.eval(self.elaborator.meter(), &term)?;
            built.push(Motive { term, value });
        }
        Ok(built)
    }

    /// The goal as a family: `λ(t : N p⃗). G`, with the subject abstracted out.
    ///
    /// Abstraction by re-evaluation, for the reason the module doc gives: the
    /// goal is quoted at the splitting depth, the subject's binder is rebound to
    /// a variable one level deeper, and re-evaluating puts that variable
    /// wherever the subject stood. Quoting the answer one binder deeper reads it
    /// back as index 0, which is the λ this returns.
    ///
    /// **A goal that does not mention the subject is not a special case.** It
    /// comes back unchanged, the λ ignores its argument, and the motive applied
    /// to anything is `G` — exactly the non-dependent behaviour, reached by the
    /// same path rather than by a branch that could disagree with it.
    fn abstracted(&mut self, scope: &Scope, subject: &Subject, goal: &Value) -> Result<Term, ElabError> {
        let depth = scope.depth();
        let meter = self.elaborator.meter();
        let Some(level) = variable(&subject.value) else {
            // [`subject`] names every subject that is not already a variable, so
            // there is nothing here to abstract. Quoting one binder deeper is
            // the weakening that puts the goal under the λ.
            let body = quote_type(meter, depth.deeper(), crate::kernel::quote::Mode::Keep, goal)?;
            return Ok(Term::lam(self.here, "t", body));
        };
        let quoted = quote_type(meter, depth, crate::kernel::quote::Mode::Keep, goal)?;
        let standing = Value::var(self.here, depth, Arc::clone(&subject.ty));
        let env = scope.env().rebinding(Level(level), standing);
        let opened = eval(meter, &env, &quoted)?;
        let body = quote_type(meter, depth.deeper(), crate::kernel::quote::Mode::Keep, &opened)?;
        Ok(Term::lam(self.here, "t", body))
    }

    /// One alternative: the sub-matrix for a constructor, under its fields and
    /// induction hypotheses.
    ///
    /// `None` where no row survives the split, which is a constructor the
    /// `match` did not cover. Reported by [`CaseTree::uncovered`] rather than
    /// here, so that the verdict comes from the declaration group and not from
    /// this loop's own place in it.
    fn method(
        &mut self,
        scope: &Scope,
        problem: &Problem<'_>,
        split: &Analysed,
        motives: &[Motive],
        column: usize,
        family: u32,
        which: u32,
    ) -> Result<Option<Alternative>, ElabError> {
        let group = Arc::clone(&split.element.group);
        let Some(rule) = group
            .family_at(family)
            .and_then(|declared| declared.constructor_at(which))
        else {
            return Err(Refusal::IncompleteMatch {
                at: self.here,
                constructor: Arc::from("?"),
            }
            .into());
        };

        // The declaration context, the parameters, and then each field as it is
        // assumed: the environment a stored field type is read in.
        let mut reading = crate::kernel::family::Group::declarations(&group, scope.cx().globals());
        for param in &split.element.params {
            reading = reading.push(param.clone());
        }
        let mut inner = scope.clone();
        let mut fields = Vec::with_capacity(rule.fields.len());
        for binder in rule.fields.iter() {
            let ty = Arc::new(eval(self.elaborator.meter(), &reading, &binder.ty)?);
            let value = inner.fresh_var(self.here, Arc::clone(&ty));
            // Assumed without a name it can be *written*. The λ still carries
            // the declaration's field name, because that is what a printed
            // method should read as, but a name in scope is a name the author's
            // body resolves against, and this one is not the author's: two
            // matches on the same family would put the same declaration names
            // in scope twice, and the inner one would silently shadow the outer
            // row's binding of the same spelling. What a row may write is bound
            // by [`Self::leaf`] from that row's own pattern.
            inner = inner.assume(None, self.here, Arc::clone(&ty));
            reading = reading.push(value.clone());
            fields.push(Subject {
                value,
                ty,
                at: self.here,
            });
        }
        // The hypotheses come after every field, which is the order
        // [`crate::kernel::family`] assembles the method type in. They are assumed
        // without names for the reason the fields are: what a row's body may
        // write is named after that row's own pattern, and [`Self::narrowed`]
        // binds the two together — so `Succ k` gives `k#ih` and `Succ j` gives
        // `j#ih` from the same method, and a `match` nested inside an arm
        // cannot shadow an outer hypothesis except the way an author's own
        // shadowing does.
        let mut hypotheses = Vec::with_capacity(rule.recursive.len());
        for (position, _) in rule.recursive.iter() {
            let position = usize::try_from(*position).unwrap_or(usize::MAX);
            let Some(field) = fields.get(position) else {
                continue;
            };
            let ty = self.hypothesis(motives, field)?;
            let value = inner.fresh_var(self.here, Arc::clone(&ty));
            inner = inner.assume(None, self.here, Arc::clone(&ty));
            hypotheses.push((
                position,
                Subject {
                    value,
                    ty,
                    at: self.here,
                },
            ));
        }

        // What this method knows its subject to be. §6.2's variable rule expands
        // rather than defers, so a variable pattern in the split column has to
        // name something, and this is what it names.
        let built = self.built(scope, &group, family, which, &split.element.params, &fields)?;

        let body = if family == split.element.family {
            let goal = self.method_goal(motives, family, &built)?;
            let rows = Self::narrowed(problem, column, family, which, &group, &fields, &hypotheses, &built)?;
            if rows.is_empty() {
                return Ok(None);
            }
            let mut columns = Vec::with_capacity(problem.columns.len().saturating_add(fields.len()));
            for (position, subject) in problem.columns.iter().enumerate() {
                if position == column {
                    for field in &fields {
                        columns.push(Subject {
                            value: field.value.clone(),
                            ty: Arc::clone(&field.ty),
                            at: field.at,
                        });
                    }
                } else {
                    columns.push(Subject {
                        value: subject.value.clone(),
                        ty: Arc::clone(&subject.ty),
                        at: subject.at,
                    });
                }
            }
            self.solve(
                &inner,
                &Problem {
                    columns,
                    rows,
                    goal: Arc::new(goal),
                },
            )?
        } else {
            // A sibling family's motive is `λ_. G → G`, so its method is the
            // identity and says nothing about a value nobody matched.
            CaseTree::Answer(Term::lam(self.here, "impossible", Term::var(self.here, Index(0))))
        };

        // Fields first, then one hypothesis per recursive field: the order
        // [`crate::kernel::family`] assembles the method type in, and the order
        // [`Alternative`] wraps them back into λs in. Kept as two lists because
        // reduction binds them differently — see [`Alternative`].
        let bound: Vec<Name> = rule.fields.iter().map(|binder| Arc::clone(&binder.name)).collect();
        let mut hypothesised: Vec<Name> = Vec::with_capacity(rule.recursive.len());
        for (position, _) in rule.recursive.iter() {
            hypothesised.push(
                rule.fields
                    .get(usize::try_from(*position).unwrap_or(usize::MAX))
                    .map_or_else(|| Arc::from("hypothesis"), |binder| Arc::from(format!("{}#ih", binder.name))),
            );
        }
        Ok(Some(Alternative {
            constructor: Constant::constructor(&group, family, which).name(),
            fields: Arc::from(bound),
            hypotheses: Arc::from(hypothesised),
            body,
        }))
    }

    /// The type of the induction hypothesis for a recursive field: the field's
    /// own motive **applied to the field**.
    ///
    /// Dependent, and that is what makes a recursive arm able to say something
    /// about the value it recursed on rather than merely produce an inhabitant
    /// of a fixed answer type. [`crate::kernel::family`] assembles the method
    /// type the same way, so the two agree by construction.
    fn hypothesis(&mut self, motives: &[Motive], field: &Subject) -> Result<Arc<Value>, ElabError> {
        let Some(found) = element(self.elaborator.meter(), &field.ty)? else {
            return Err(Refusal::IncompleteMatch {
                at: self.here,
                constructor: Arc::from("?"),
            }
            .into());
        };
        let Some(motive) = motives.get(usize::try_from(found.family).unwrap_or(usize::MAX)) else {
            return Err(Refusal::IncompleteMatch {
                at: self.here,
                constructor: Arc::from("?"),
            }
            .into());
        };
        let applied = apply(
            self.elaborator.meter(),
            self.here,
            motive.value.clone(),
            field.value.clone(),
        )?;
        Ok(Arc::new(applied))
    }

    /// The subject a method is the method *for*: `c p⃗ a⃗`, at the parameters
    /// the split read off the subject's type.
    ///
    /// What a variable pattern in the split column binds. §6.2's variable rule
    /// expands rather than defers, so such a pattern has to name something, and
    /// this is what it names.
    fn built(
        &mut self,
        scope: &Scope,
        group: &Arc<crate::kernel::family::Group>,
        family: u32,
        which: u32,
        params: &[Value],
        fields: &[Subject],
    ) -> Result<Built, ElabError> {
        let Some(_rule) = group
            .family_at(family)
            .and_then(|declared| declared.constructor_at(which))
        else {
            return Err(Refusal::IncompleteMatch {
                at: self.here,
                constructor: Arc::from("?"),
            }
            .into());
        };
        let globals = scope.cx().globals();
        let mut value = Constant::constructor(group, family, which).value(self.here, globals);
        let mut ty = Constant::family(group, family).value(self.here, globals);
        for param in params {
            value = apply(self.elaborator.meter(), self.here, value, param.clone())?;
            ty = apply(self.elaborator.meter(), self.here, ty, param.clone())?;
        }
        for field in fields {
            value = apply(self.elaborator.meter(), self.here, value, field.value.clone())?;
        }
        Ok(Built {
            value,
            ty: Arc::new(ty),
        })
    }

    /// The goal a constructor's method answers: the motive **applied to the
    /// constructor form this method is the method for**.
    ///
    /// This is the sentence §1.1 states and the whole of what this prompt
    /// changed: an arm that matched `Cons h t` is checked at the goal with
    /// `Cons h t` where the subject stood, so a `Vec`, an `Equal`, or any other
    /// refinement by matching has somewhere to land.
    ///
    /// Read off the motive rather than off `problem.goal`: this is exactly the
    /// type [`crate::kernel::family`] assembled the method at, and two
    /// computations of it could disagree where one cannot.
    fn method_goal(&mut self, motives: &[Motive], family: u32, built: &Built) -> Result<Value, ElabError> {
        let Some(motive) = motives.get(usize::try_from(family).unwrap_or(usize::MAX)) else {
            return Err(Refusal::IncompleteMatch {
                at: self.here,
                constructor: Arc::from("?"),
            }
            .into());
        };
        let motive = motive.value.clone();
        Ok(apply(self.elaborator.meter(), self.here, motive, built.value.clone())?)
    }

    /// The rows that survive a split, with the split column replaced by the
    /// constructor's fields.
    ///
    /// This is ch. 5's mixture rule without the fat bar: a variable pattern in
    /// the split column matches *every* constructor, so it is expanded into one
    /// row per constructor with a binder per field, rather than left to fall
    /// through into a default.
    fn narrowed<'a>(
        problem: &Problem<'a>,
        column: usize,
        family: u32,
        which: u32,
        group: &Arc<crate::kernel::family::Group>,
        fields: &[Subject],
        hypotheses: &[(usize, Subject)],
        built: &Built,
    ) -> Result<Vec<Row<'a>>, ElabError> {
        let wanted = Constant::constructor(group, family, which).name();
        let mut rows = Vec::new();
        for row in &problem.rows {
            let Some(pattern) = row.patterns.get(column) else {
                continue;
            };
            let mut bindings = row.bindings.clone();
            let inner: Vec<&RawPattern> = match pattern {
                RawPattern::Constructor { name, fields: sub, .. } => {
                    if !selects(&wanted, name) {
                        continue;
                    }
                    if sub.len() != fields.len() {
                        return Err(Refusal::NoSuchConstructor {
                            at: pattern.origin(),
                            name: Arc::clone(name),
                            ty: Term::universe(pattern.origin(), Sort::ZERO),
                            cases: vec![Arc::clone(&wanted)],
                        }
                        .into());
                    }
                    sub.iter().collect()
                }
                RawPattern::Bind { name, .. } => {
                    // What the variable stood for, as this branch knows it: the
                    // constructor rather than the subject one level up, so a
                    // body that uses the name is reading the refined value.
                    bindings.push((Arc::clone(name), built.value.clone(), Arc::clone(&built.ty)));
                    // ch. 5's variable rule: it matches this constructor as it
                    // matches every other, and the fields it did not name are
                    // matched by wildcards rather than by nothing.
                    vec![wildcard(); fields.len()]
                }
                // Refused by `belong` before any split runs, so a column that
                // reached here has none.
                RawPattern::Record { .. } => continue,
            };
            let mut patterns = Vec::with_capacity(row.patterns.len().saturating_add(fields.len()));
            for (position, held) in row.patterns.iter().enumerate() {
                if position == column {
                    patterns.extend(inner.iter().copied());
                } else {
                    patterns.push(held);
                }
            }
            rows.push(Row {
                patterns,
                bindings,
                arm: row.arm,
            });
        }
        Ok(rows)
    }
}

/// Whether a pattern's name selects the constructor spelled `qualified`.
///
/// The qualified spelling always does. So does the bare one, and that is
/// `01-surface.md` §1.3's rule rather than a convenience: a pattern is read
/// against the subject's type, and the subject's type is what qualifies the
/// name. Two families may each declare an `Untied` for exactly this reason —
/// the one meant is the one the column is being split on.
fn selects(qualified: &str, written: &str) -> bool {
    qualified == written || qualified.rsplit_once('.').is_some_and(|(_, case)| case == written)
}

/// What the induction hypothesis for the field named `field` is called.
///
/// The pattern an expanded variable leaves in each field position.
///
/// One shared value rather than one per position: a variable pattern names the
/// whole subject and none of the fields, so nothing the body writes can refer to
/// these and they need no distinct identity. `'static` because a [`Row`] borrows
/// its patterns from the arms, and this one belongs to no arm.
fn wildcard() -> &'static RawPattern {
    static WILDCARD: OnceLock<RawPattern> = OnceLock::new();
    WILDCARD.get_or_init(|| RawPattern::bind(Origin::UNKNOWN, "_"))
}

/// What a method knows its subject to be: `c p⃗ a⃗`, and its type.
struct Built {
    value: Value,
    ty: Arc<Value>,
}

/// A motive, as both the term the recursor is applied to and the value the
/// method goals are computed from.
struct Motive {
    term: Term,
    value: Value,
}

/// The universe the motives land in, which this use site does not get to choose.
///
/// §1.3 has no universe polymorphism, so a recursor takes a level per use, and
/// the motive here is the goal with binders in front of it — the level is the
/// goal's, and asking is the only way to learn it.
///
/// **Where the asking happens depends on whether the goal has syntax yet.** A
/// goal with a head is read by
/// [`recheck::universe_of`](crate::kernel::recheck::universe_of), which is the
/// core rules answering about a finished term. A goal that is still
/// a **metavariable** is not a finished term, and handing one to the re-checker
/// would be asking it a question its own contract says it never receives — the
/// answer is [`Malformed`](crate::kernel::error::Malformed), which is a defect report
/// and not a verdict about the program.
///
/// The level is not unknown there, only written somewhere else: a metavariable
/// records the type it stands at, and a goal's type is `Type ℓ`. So the spine is
/// walked through that telescope and the level comes out of the end. This is
/// what lets a `match` be written where its result type is determined later —
/// `xs.fold_from_start(empty, λacc. λx. match keep(x) { … })`, where the
/// accumulator's type is fixed by what the fold is checked against and not by
/// anything the arms can see.
fn motive_level(meter: &mut Meter, scope: &Scope, goal: &Value) -> Result<Sort, ElabError> {
    let quoted = scope.quote_type(meter, goal)?;
    Ok(crate::kernel::recheck::universe_of(&quoted)?)
}

/// What reading a subject's type told the splitter.
struct Analysed {
    element: Element,
    /// The parameters, as terms at the splitting depth.
    params: Vec<Term>,
    /// The subject itself, as a term at the splitting depth.
    target: Term,
    /// The universe the motive lands in.
    level: Sort,
}

impl Analysed {
    fn read(
        tree: &mut Tree<'_, '_>,
        scope: &Scope,
        subject: &Subject,
        found: &Element,
        goal: &Value,
        at: Origin,
    ) -> Result<Self, ElabError> {
        let _ = at;
        let depth = scope.depth();
        let meter = tree.elaborator.meter();
        // Forced first: a subject whose type is still headed by a metavariable
        // the solver has since filled reads back as that metavariable, and the
        // family's parameters would come off it as nothing at all — emitting a
        // recursor spine short by exactly the parameters, which ι then never
        // fires on. Forcing follows the solution without unfolding anything
        // else, which is all this read needs.
        let forced = crate::kernel::eval::force(meter, &subject.ty)?;
        let ty = quote_type(
            meter,
            depth,
            crate::kernel::quote::Mode::Keep,
            forced.as_ref().unwrap_or(&subject.ty),
        )?;
        let (_, arguments) = spine(&ty);
        let params = usize::try_from(found.group.params()).unwrap_or(usize::MAX);
        // Cheap because [`Subject`]'s value is a variable: this reads back a
        // variable, η-expanded at its type, and never a call's normal form.
        let target = quote(
            meter,
            depth,
            crate::kernel::quote::Mode::Keep,
            &subject.ty,
            &subject.value,
        )?;
        let level = motive_level(meter, scope, goal)?;
        Ok(Self {
            element: Element {
                group: Arc::clone(&found.group),
                family: found.family,
                params: found.params.clone(),
            },
            params: arguments.get(..params).unwrap_or_default().to_vec(),
            target,
            level,
        })
    }
}

/// Elaborate one subject, and keep it as a value — a *variable*, if it was not
/// one already, together with the binder that names it.
///
/// Peyton Jones ch. 5 states the match algorithm over variables ("the `u_i` are
/// variables") and §5.2.4 introduces exactly this `let` for the general case.
/// Here the reason is sharper than presentation. [`Analysed::read`] hands the
/// split its target as a *term*, and the only way to turn a value back into
/// a term is [`quote`], which writes a **normal form**: a subject that is a call
/// would put that call's whole unfolding into the emitted tree, and the
/// unfolding of a call into `stdlib/` is the transitive closure of everything it
/// reaches. Naming it first is what keeps the tree the size of the program.
///
/// **And it is what makes the motive dependent at all.** [`Tree::abstracted`]
/// abstracts the subject out of the goal by rebinding *its binder*, so a subject
/// that is not a variable has no binder to rebind and no refinement to gain. The
/// `let` [`compile`] writes around the tree is δ, so the value is back before
/// anything evaluates — the naming costs the program nothing and buys it §1.1.
fn subject(
    elaborator: &mut Elaborator,
    scope: &Scope,
    raw: &Raw,
) -> Result<(Subject, Option<(Name, Term, Term)>), ElabError> {
    let (term, ty) = elaborator.infer_open(scope, raw)?;
    let value = scope.eval(elaborator.meter(), &term)?;
    let ty = Arc::new(ty);
    let at = raw.origin();
    if variable(&value).is_some() {
        return Ok((Subject { value, ty, at }, None));
    }
    let ty_term = scope.quote_type(elaborator.meter(), &ty)?;
    let value = scope.fresh_var(at, Arc::clone(&ty));
    // A space, so that no author's identifier is shadowed by it and every
    // refusal that prints a binder still prints something a reader recognizes.
    let name: Name = Arc::from("match subject");
    Ok((Subject { value, ty, at }, Some((name, ty_term, term))))
}

/// The de Bruijn level a value is, when it is a variable.
fn variable(value: &Value) -> Option<u32> {
    match &value.form {
        // A *bare* variable: a spine means something was applied to it, and
        // `f x` is not the variable `f`.
        crate::kernel::value::Form::Neutral(neutral) if neutral.spine.is_empty() => match &neutral.head {
            crate::kernel::value::Head::Var(level, _) => Some(level.0),
            crate::kernel::value::Head::Const(..)
            | crate::kernel::value::Head::Base(..)
            | crate::kernel::value::Head::Builtin(..)
            | crate::kernel::value::Head::Meta(_)
            | crate::kernel::value::Head::Def(_, _, _) => None,
        },
        crate::kernel::value::Form::Neutral(_) => None,
        crate::kernel::value::Form::Universe(_)
        | crate::kernel::value::Form::Pi { .. }
        | crate::kernel::value::Form::Lam { .. }
        | crate::kernel::value::Form::RecordType(_)
        | crate::kernel::value::Form::Record(_)
        | crate::kernel::value::Form::Lit(_)
        | crate::kernel::value::Form::Numeral(_) => None,
    }
}

/// The head of an application spine, and what is applied to it.
fn spine(term: &Term) -> (&Term, Vec<Term>) {
    let mut arguments = Vec::new();
    let mut head = term;
    while let Shape::App { function, argument } = head.shape() {
        arguments.push(argument.clone());
        head = function;
    }
    arguments.reverse();
    (head, arguments)
}
