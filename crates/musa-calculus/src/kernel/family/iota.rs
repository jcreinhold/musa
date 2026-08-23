//! ι-reduction: a recursor applied to a constructor steps to the method
//! declared for it.
//!
//! One concern of the `family` module; see its docs for the calculus.

use super::constant::{Constant, Numeral};
use super::group::{Group, Role};
use crate::kernel::budget::Meter;
use crate::kernel::context::Globals;
use crate::kernel::error::CoreError;
use crate::kernel::eval::{apply, eval};
use crate::kernel::sort::Sort;
use crate::kernel::term::Name;
use crate::kernel::value::{Elim, Form, Head, Neutral, Value};
use std::sync::Arc;

/// The constant at the head of a blocked spine, and what has been applied to it.
pub(super) fn spine(neutral: &Neutral) -> Option<(Constant, Vec<Value>)> {
    let Head::Const(constant, _) = &neutral.head else {
        return None;
    };
    let mut arguments = Vec::with_capacity(neutral.spine.len());
    for elimination in &neutral.spine {
        let Elim::App { argument, .. } = elimination;
        arguments.push(Value::clone(argument));
    }
    Some((constant.clone(), arguments))
}

/// ι at an inductive family, or `None` when the elimination stays blocked.
///
/// `elim_i p⃗ P⃗ m⃗ i⃗ (c p⃗ a⃗) ⟶ m_c a⃗ ih⃗`. The induction hypothesis for a
/// recursive field is `elim_{j'} p⃗ P⃗ m⃗ i⃗' a_ℓ` — another elimination, which
/// [`apply`] fires in turn, and which is why recursion here is the family's own
/// structure rather than a fixed point. §1.1's strict positivity is what makes
/// that descent terminate; the meter is what makes a violated invariant a refusal
/// rather than a hang.
///
/// A hypothesis whose method never names it is **not computed**. `01-surface.md`
/// §1.3's `match` compiles to nested recursors (§6.2), and a case tree does case
/// analysis rather than recursion — every method it writes binds its hypothesis
/// and ignores it. Computing one anyway makes each level of a tree cost the
/// descent twice, once for the hypothesis and once for the branch that reads the
/// same field, so a tree `d` levels deep costs `2^d`: `match n { 12 -> … }` at
/// `n = 12` spent the whole 200,000-step budget on values no method read. That
/// is Peyton Jones ch. 22's analysis with its sign reversed — not "this argument
/// is certainly needed" but "this binder is certainly absent" — and
/// [`crate::kernel::term::occurrences`] decides it exactly, so nothing is skipped that
/// a body could have named.
///
/// The saving is in *what is built*, not in what is charged: an unread
/// hypothesis costs no reduction steps because it is never reduced. §4's law is
/// untouched in the direction that matters — a program that was accepted still
/// evaluates to the same value, since the argument that changed is one no body
/// mentions — and it moves in the only safe direction for the budget, which is
/// that a program refused for exhaustion may now be accepted.
///
/// **This decides; it does not reduce.** `subject` is the last argument already
/// opened by the caller when [`opens_last`] asked for it, and what comes back is
/// a decision the evaluator's control stack then carries out. Nothing here
/// evaluates, so nothing here can stand inside itself.
pub(crate) fn iota(neutral: &Neutral, subject: Option<&Value>) -> Option<Fired> {
    if let Some(field) = projected(neutral, subject) {
        return Some(Fired::Field(field));
    }
    ready(neutral, subject).map(|reduction| Fired::Method(Box::new(reduction)))
}

/// What ι answered: a field read out, or a method still to be applied.
///
/// Two answers rather than one value because the second one is *work* —
/// applying the method to the fields and then to one induction hypothesis per
/// recursive field — and that work is the evaluator's own recursion. It is
/// handed back rather than performed here so that
/// [`crate::kernel::eval`]'s control stack carries it, which is what keeps a
/// fold's depth off the host stack (§4.1).
pub(crate) enum Fired {
    /// A generated accessor's answer: the field the constructor holds.
    Field(Value),
    /// A recursor's: everything [`Reduction`] names, none of it applied yet.
    Method(Box<Reduction>),
}

/// Whether the *last* argument of this blocked spine has to be opened before
/// [`iota`] or [`stepped`] can decide.
///
/// The three rules below each look through exactly one value — a projection's
/// subject, a recursor's target, a counting constructor's predecessor — and it
/// is the last argument in all three. Asking the question separately is what
/// lets the opening be a transition of the evaluator's machine rather than a
/// host call from inside the rule, and the three guards here are the same ones
/// the rules apply, in the same order, so that a spine which never reached an
/// `opened` before does not reach one now.
pub(crate) fn opens_last(neutral: &Neutral) -> bool {
    let Head::Const(ref constant, _) = neutral.head else {
        return false;
    };
    let saturated = u32::try_from(neutral.spine.len()).unwrap_or(u32::MAX) == constant.arity();
    match constant.role {
        Role::Projection(_) | Role::Recursor(_) => saturated && !neutral.spine.is_empty(),
        Role::Constructor(which) => {
            let family = Constant {
                group: Arc::clone(&constant.group),
                family: constant.family,
                role: Role::Family,
            };
            family.counting().is_some_and(|counting| which == counting.step) && neutral.spine.len() == 1
        }
        Role::Family => false,
    }
}

/// ι at a generated field accessor: `f_i p⃗ (C p⃗ a⃗) ⟶ a_i`.
///
/// The one-constructor case of the rule above, and it is a rule of its own
/// because an accessor is not a recursor: it takes the parameters and the value
/// and nothing else, so there is no motive to instantiate and no method to
/// find. What it steps to is the argument the constructor was applied to, which
/// is what makes `01-surface.md` §1.2's projection compute.
///
/// Blocked on anything but a constructor spine, which is the ordinary case for
/// a variable and is what η at [`crate::kernel::quote`] then reads.
///
/// `subject` is the last argument opened, which [`opens_last`] asked the
/// evaluator for; `None` where it did not, and then this answers nothing.
fn projected(neutral: &Neutral, subject: Option<&Value>) -> Option<Value> {
    let (accessor, arguments) = spine(neutral)?;
    let Role::Projection(field) = accessor.role else {
        return None;
    };
    if u32::try_from(arguments.len()).unwrap_or(u32::MAX) != accessor.arity() {
        return None;
    }
    let subject = subject?;
    let Form::Neutral(ref built) = subject.form else {
        return None;
    };
    let (constructor, applied) = spine(built)?;
    // The family is not asked for: a group declares one accessor per field of
    // one constructor, so a spine that reached here at all was built by the
    // constructor this accessor reads. What *is* asked for is that the head is
    // a constructor and that the spine is saturated, since a partially applied
    // one holds no field to answer with.
    if !matches!(constructor.role, Role::Constructor(_))
        || u32::try_from(applied.len()).unwrap_or(u32::MAX) != constructor.arity()
    {
        return None;
    }
    let params = usize::try_from(accessor.group.params()).unwrap_or(usize::MAX);
    let position = params.saturating_add(usize::try_from(field).unwrap_or(usize::MAX));
    applied.get(position).cloned()
}

/// The value to hand a binder that is provably absent from the body it binds.
///
/// `Some` exactly when `method` is a λ whose body never names its own binder, in
/// which case β discards whatever is pushed and the cheapest thing to push is
/// what this returns. `Type 0` and not the field or a fresh variable: those are
/// values a body could plausibly have wanted, so a mistake in the analysis would
/// read as a wrong answer, while a universe standing where a proof belongs is
/// wrong in a way the next conversion says out loud.
pub(crate) fn unread(method: &Value) -> Option<Value> {
    let Form::Lam(closure) = &method.form else {
        return None;
    };
    // Level one and level zero: at the top of a closure body the binder just
    // pushed is the innermost, and `occurrences` counts from the outside.
    (crate::kernel::term::occurrences(&closure.body, 1, 0) == 0)
        .then(|| Value::new(method.origin, Form::Universe(crate::kernel::sort::Sort::ZERO)))
}

/// The numeral a counting family's step constructor collapses to, when it has
/// been applied to a numeral of that same family.
///
/// The other half of [`Numeral`]'s canonicity: [`crate::kernel::eval::eval`] turns the
/// floor into a zero and this turns `step k` into `k + 1`, so a value at a
/// counting family is *always* a numeral and never a constructor spine. Called
/// from [`crate::kernel::eval::apply`] beside [`iota`], because both answer the same
/// question about a freshly blocked spine — whether it is blocked at all.
///
/// A count that would pass [`u64::MAX`] simply does not collapse: the spine is a
/// representation this crate already has and already reads correctly, so nothing
/// is lost by leaving one there and no error path has to exist for a case that
/// costs 2⁶⁴ steps to reach.
///
/// `subject` is the argument opened, as [`iota`]'s is and for the same reason.
pub(crate) fn stepped(neutral: &Neutral, subject: Option<&Value>) -> Option<Value> {
    let Head::Const(ref constructor, _) = neutral.head else {
        return None;
    };
    let Role::Constructor(which) = constructor.role else {
        return None;
    };
    let family = Constant {
        group: Arc::clone(&constructor.group),
        family: constructor.family,
        role: Role::Family,
    };
    if family.counting().is_none_or(|counting| which != counting.step) {
        return None;
    }
    if neutral.spine.len() != 1 {
        return None;
    }
    let Form::Numeral(ref below) = subject?.form else {
        return None;
    };
    if below.family != family {
        return None;
    }
    let count = below.count.checked_add(1)?;
    Some(Value::new(
        neutral.outer_origin(),
        Form::Numeral(Numeral { family, count }),
    ))
}

/// Everything ι needs once it has decided the elimination fires.
pub(crate) struct Reduction {
    /// The method for the constructor the target was built by.
    pub(crate) method: Value,
    /// The constructor's field arguments.
    pub(crate) fields: Vec<Value>,
    /// The recursor's parameters, motives, and methods — what an induction
    /// hypothesis is the same elimination at.
    prefix: Vec<Value>,
    /// The group, and the constructor within it that the target was built by.
    group: Arc<Group>,
    family: u32,
    which: u32,
    /// The universe the motives land in, which an induction hypothesis inherits.
    level: Sort,
    /// The table the recursor's name was resolved under, which the hypothesis
    /// it builds is read in — see [`Head::Base`](crate::kernel::value::Head::Base).
    globals: Globals,
}

/// Decide whether `neutral` is a saturated recursor applied to a constructor,
/// and take apart what it is applied to.
fn ready(neutral: &Neutral, subject: Option<&Value>) -> Option<Reduction> {
    let (recursor, arguments) = spine(neutral)?;
    let Head::Const(_, ref globals) = neutral.head else {
        return None;
    };
    let Role::Recursor(level) = recursor.role.clone() else {
        return None;
    };
    if u32::try_from(arguments.len()).unwrap_or(u32::MAX) != recursor.arity() {
        return None;
    }
    let group = Arc::clone(&recursor.group);
    let prefix_len = usize::try_from(
        group
            .params()
            .saturating_add(group.arity())
            .saturating_add(group.methods()),
    )
    .unwrap_or(usize::MAX);
    let target = subject?;
    let params = usize::try_from(group.params()).unwrap_or(usize::MAX);
    let (family, which, fields) = match target.form {
        // This is where the tower reappears, one level and no more: a numeral
        // knows which constructor it was built by from its count, and what that
        // constructor was applied to is the numeral one below. The level under
        // *that* is not built, which is the whole saving — a fold over `n` costs
        // what a fold over a tower of height `n` costs, and building the number
        // costs nothing.
        Form::Numeral(ref numeral) => {
            let counting = numeral.family.counting()?;
            let which = counting.case_of(numeral.count);
            let below = numeral
                .family
                .below(numeral.count)
                .map(|below| Value::new(target.origin, Form::Numeral(below)));
            (numeral.family.family, which, below.into_iter().collect())
        }
        Form::Neutral(ref target) => {
            let (constructor, built) = spine(target)?;
            let Role::Constructor(which) = constructor.role else {
                return None;
            };
            (
                constructor.family,
                which,
                built.get(params..).unwrap_or_default().to_vec(),
            )
        }
        Form::Universe(_) | Form::Pi { .. } | Form::Lam(_) | Form::Lit(_) => return None,
    };
    let motives = usize::try_from(group.arity()).unwrap_or(usize::MAX);
    let position = usize::try_from(group.method_position(family, which)).unwrap_or(usize::MAX);
    let method = arguments.get(params.saturating_add(motives).saturating_add(position))?;
    Some(Reduction {
        globals: globals.clone(),
        method: method.clone(),
        fields,
        prefix: arguments.get(..prefix_len).unwrap_or_default().to_vec(),
        group,
        family,
        which,
        level,
    })
}

/// An induction hypothesis assembled up to its last argument.
///
/// The recursor at the parameters, motives, methods, and the field's own
/// indices — everything that builds a blocked spine and reduces nothing. The
/// *field* is held back, because applying it is the moment the descent fires,
/// and [`iota`] only wants that moment for a hypothesis some method reads.
pub(crate) struct Pending {
    recursor: Value,
    field: Value,
}

impl Pending {
    /// The hypothesis waiting to be forced: the recursor, and the field that
    /// fires it.
    ///
    /// Handed apart rather than applied here. Applying is where the descent
    /// happens, and the descent belongs on [`crate::kernel::eval`]'s control
    /// stack — a hypothesis forced from inside this module would put one host
    /// frame per level of the data back under the evaluator (§4.1).
    pub(crate) fn parts(self) -> (Value, Value) {
        (self.recursor, self.field)
    }
}

/// One induction hypothesis per recursive field, in field order, each stopped
/// one argument short of firing.
///
/// Which fields are recursive is read off [`Constructor::recursive`] rather than
/// re-derived from the field types here, so that the method the hypothesis is
/// passed to and the hypothesis itself cannot disagree about how many arguments
/// there are: they are the same list.
pub(crate) fn hypotheses(meter: &mut Meter, reduction: &Reduction) -> Result<Vec<Pending>, CoreError> {
    let group = &reduction.group;
    let here = group.origin;
    let Some(rule) = group
        .family_at(reduction.family)
        .and_then(|declared| declared.constructor_at(reduction.which))
    else {
        return Ok(Vec::new());
    };
    let params = usize::try_from(group.params()).unwrap_or(usize::MAX);
    // A stored field type is read in the declaration context, then the
    // parameters, then the fields before it — which is exactly the environment
    // this walk builds up.
    let mut reading = Group::declarations(group, &reduction.globals);
    for param in reduction.prefix.iter().take(params) {
        reading = reading.push(param.clone());
    }
    let mut recursive = rule.recursive.iter().peekable();
    let mut built = Vec::new();
    for (position, field) in reduction.fields.iter().enumerate() {
        let position = u32::try_from(position).unwrap_or(u32::MAX);
        let ty = match rule.fields.get(usize::try_from(position).unwrap_or(usize::MAX)) {
            Some(binder) => eval(meter, &reading, &binder.ty)?,
            None => break,
        };
        if recursive.peek().is_some_and(|(at, _)| *at == position) {
            let of_family = recursive.next().map_or(0, |(_, family)| *family);
            let mut hypothesis = Constant {
                group: Arc::clone(group),
                family: of_family,
                role: Role::Recursor(reduction.level.clone()),
            }
            .value(here, &reduction.globals);
            for argument in &reduction.prefix {
                hypothesis = apply(meter, here, hypothesis, argument.clone())?;
            }
            // The field's own indices, which stand between the methods and the
            // field in the recursor's telescope. Read off the field's *type*,
            // because that is where the declaration put them: a recursive field
            // of `Vect A n` is eliminated at `n`, and a hypothesis assembled
            // without it would be the recursor one argument short of firing at
            // the wrong place rather than at the right one.
            if let Some(found) = super::element(meter, &ty)? {
                for index in &found.indices {
                    hypothesis = apply(meter, here, hypothesis, index.clone())?;
                }
            }
            built.push(Pending {
                recursor: hypothesis,
                field: field.clone(),
            });
        }
        reading = reading.push(field.clone());
    }
    Ok(built)
}

/// The constructor a saturated spine is built by, and how many of its arguments
/// are parameters rather than fields.
///
/// What §5.8's δ needs in order to see a *constructor application* as canonical
/// data: the qualified name a rule writes, and where its fields begin. The
/// parameters are excluded because they are types rather than data — `Some 3` is
/// `Option.Some Int 3`, and only the `3` is something a rule can be handed.
///
/// `None` for anything that is not a saturated constructor: a family, a
/// recursor, a variable, or a constructor one field short. Each of those is a
/// blocked spine, which is exactly what a δ-builtin over an open term should
/// stay.
pub(crate) fn constructed(neutral: &Neutral) -> Option<(Name, usize)> {
    let Head::Const(constant, _) = &neutral.head else {
        return None;
    };
    // A constructor's type is a Π chain, so anything but an application means
    // the spine was assembled by something other than the elaborator.
    if !neutral
        .spine
        .iter()
        .all(|elimination| matches!(*elimination, Elim::App { .. }))
    {
        return None;
    }
    saturated(constant, neutral.spine.len())
}

/// The same question of a constant and how many arguments it was applied to.
///
/// The parameter-count rule itself, with the two readings of it above and in
/// [`canonical`] left holding only the walk that finds the spine. One rule
/// because there is one fact — where a constructor's fields begin is fixed by
/// the declaration — and two copies of it could disagree about a family whose
/// parameters changed.
pub(super) fn saturated(constant: &Constant, applied: usize) -> Option<(Name, usize)> {
    if !matches!(constant.role, Role::Constructor(_)) {
        return None;
    }
    if u32::try_from(applied).unwrap_or(u32::MAX) != constant.arity() {
        return None;
    }
    Some((
        constant.name(),
        usize::try_from(constant.group.params()).unwrap_or(usize::MAX),
    ))
}
