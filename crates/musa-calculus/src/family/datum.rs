//! The bridge to canonical payloads: a term read as a datum, and a datum
//! realized as a value of a family.
//!
//! One concern of the `family` module; see its docs for the calculus.

use super::constant::{Constant, Numeral};
use super::group::{Group, element};
use super::iota::saturated;
use crate::base::Datum;
use crate::budget::Meter;
use crate::context::Globals;
use crate::error::{CoreError, Malformed};
use crate::eval::{apply, eval};
use crate::origin::Origin;
use crate::term::{Constant as Written, Definition, Name, Shape, Term};
use crate::value::{Form, Value};
use std::sync::Arc;

/// A numeral as canonical data: the family it stands at, and the count.
///
/// One node, like the numeral itself. The obvious alternative — read it back as
/// the `count` nested constructor applications it denotes — is what the old
/// representation forced and what this one exists to remove: the tree costs a
/// node per unit, and *dropping* it recurses on the host stack, so a host asking
/// for a large `Nat` as data would abort where the term never could. A rule that
/// wants the number reads [`Datum::Count`] and has it.
pub(crate) fn counted(numeral: &Numeral) -> Option<Datum> {
    numeral.family.counting()?;
    Some(Datum::Count {
        family: numeral.family.name(),
        count: numeral.count,
    })
}

/// The canonical data a normal form denotes, or [`None`] when it denotes none.
///
/// The same name as `eval`'s private reading of a [`Value`], because it is the
/// same question: §5.8's D1 asked of a *term* rather than of a value: a literal, or a
/// constructor of a declared family applied to more of the same. It is what a
/// consumer of [`crate::check`] uses to look inside an answer that is not a bare
/// literal — a count written as a `Nat`, a list, a pair — and the shape it hands
/// back is the one a δ-rule is already written against, so a host reads one
/// vocabulary rather than two.
///
/// Cannot fail, and reads the context for one thing: where a constructor's
/// fields begin, which is its family's parameter count. The term names the
/// constructor and the context holds the declaration (§6), so the question is a
/// lookup rather than a computation; and a normal form has nothing left to
/// compute, which is what makes this a projection where [`realize`] — its
/// inverse — must be type-directed.
///
/// # What answers `None`
///
/// Everything that is not saturated canonical data, which is a longer list than
/// it sounds: a constructor one argument short or one too many, a variable, a
/// definition, a family or a recursor applied or bare, a builtin, a λ, a Π, a
/// universe, a record, a record type, and a literal that has somehow been
/// applied to something. A record is on that list
/// deliberately — [`Datum`] has no record arm, and `01-surface.md`'s written
/// product reaches here as `Pair.Both` rather than as one.
///
/// Nothing here is an error, because "not data" is an ordinary answer: it is
/// exactly what a blocked δ-spine reports, and a caller that wanted a `Nat` says
/// so itself.
pub fn canonical(cx: &crate::Cx, term: &Term) -> Option<Datum> {
    read(cx.globals(), term)
}

/// [`canonical`] with the table already in hand, which is what the recursion
/// and this crate's own callers pass.
fn read(globals: &Globals, term: &Term) -> Option<Datum> {
    let (head, arguments) = applied_spine(term);
    match *head.shape() {
        Shape::Meta(_) => None,
        // An indexed type is a *type*, and a type is not data a δ-rule reads. It
        // reaches here only in a signature, never in an argument position, and
        // "not data" is this function's ordinary answer rather than an error.
        Shape::Indexed { .. } => None,
        Shape::Lit(Written::Payload(ref literal)) if arguments.is_empty() => Some(Datum::Lit(literal.clone())),
        Shape::Lit(Written::Numeral(ref numeral)) if arguments.is_empty() => counted(numeral),
        Shape::Named { ref name, role } => {
            let Definition::Declared(constant) = globals.definition(name, role) else {
                return None;
            };
            let (constructor, params) = saturated(&constant, arguments.len())?;
            let fields = arguments
                .into_iter()
                .skip(params)
                .map(|field| read(globals, field))
                .collect::<Option<Vec<_>>>()?;
            Some(Datum::Case { constructor, fields })
        }
        // Written out rather than left to a wildcard so that the list in the
        // doc comment above is checked by the compiler: a shape added to the
        // core has to be classified here before this crate builds again.
        //
        // `App` cannot appear — the peel above ended because the head was not
        // one — and it is named anyway, because an arm that says "unreachable"
        // is a claim a later reader has to re-derive.
        Shape::Lit(_)
        | Shape::Var(_)
        | Shape::Universe(_)
        | Shape::Bind { .. }
        | Shape::App { .. }
        | Shape::RecordType(_)
        | Shape::Record(_)
        | Shape::Project { .. } => None,
    }
}

/// A term as its head and the arguments applied to it, outermost last.
///
/// `Shape::App` nests to the left, so peeling collects the arguments backwards
/// and this puts them in written order — which is the order a constructor's
/// parameters precede its fields in, and therefore the only order `skip` above
/// means anything in.
fn applied_spine(term: &Term) -> (&Term, Vec<&Term>) {
    let mut head = term;
    let mut arguments = Vec::new();
    while let Shape::App {
        ref function,
        ref argument,
    } = *head.shape()
    {
        arguments.push(argument);
        head = function;
    }
    arguments.reverse();
    (head, arguments)
}

/// Canonical data as a value of `ty`.
///
/// The inverse of [`constructed`], and the reason it needs a type where reading
/// needed none. A [`Datum::Case`] says `Option.Some 3` and a constructor *value*
/// is `Option.Some Int 3`: the parameters come before the fields, ι reads them
/// by position, and nothing in the name or the fields says what they are. `ty`
/// is where they come from — for a δ-rule's answer it is the builtin's own
/// signature applied to the argument values, so nothing ambient is consulted.
///
/// The walk is `case.rs`'s split in reverse. At `F p⃗` the answer names one of
/// `F`'s constructors, the parameters are `p⃗`, and each field is realized at its
/// declared type read in the declaration context, then the parameters, then the
/// fields before it — the same environment a split assumes its fields in. So a
/// nested family resolves against the type in hand at that node rather than
/// against a table, which is why two occurrences of one family in a signature
/// cannot be ambiguous about anything.
///
/// # Errors
///
/// [`Malformed::MisfitAnswer`] when the data and the type disagree, otherwise as
/// [`eval`].
pub(crate) fn realize(
    meter: &mut Meter,
    here: Origin,
    globals: &Globals,
    datum: &Datum,
    ty: &Value,
) -> Result<Value, CoreError> {
    meter.nested("data realization", |meter| match *datum {
        Datum::Lit(ref literal) => Ok(Value::new(here, Form::Lit(literal.clone()))),
        Datum::Count { ref family, count } => realize_count(meter, here, family, count, ty),
        Datum::Case {
            ref constructor,
            ref fields,
        } => realize_case(meter, here, globals, constructor, fields, ty),
    })
}

/// A count, at the counting family the answer's type says it stands at.
///
/// The type decides, and the name the rule wrote is checked against it rather
/// than trusted: a rule that answered `Nat` where the signature promised some
/// other counting family would otherwise build a value at the wrong type, and
/// the two are indistinguishable once the count is all that is left.
fn realize_count(meter: &mut Meter, here: Origin, family: &Name, count: u64, ty: &Value) -> Result<Value, CoreError> {
    let misfit = || CoreError::from(Malformed::MisfitAnswer(Arc::clone(family)));
    let Some(element) = element(meter, ty)? else {
        return Err(misfit());
    };
    let constant = Constant::family(&element.group, element.family);
    if constant.counting().is_none() || *constant.name() != **family {
        return Err(misfit());
    }
    Ok(Value::new(
        here,
        Form::Numeral(Numeral {
            family: constant,
            count,
        }),
    ))
}

/// One constructor application, at the family type it stands at.
fn realize_case(
    meter: &mut Meter,
    here: Origin,
    globals: &Globals,
    constructor: &Name,
    fields: &[Datum],
    ty: &Value,
) -> Result<Value, CoreError> {
    let misfit = || CoreError::from(Malformed::MisfitAnswer(Arc::clone(constructor)));
    let Some(element) = element(meter, ty)? else {
        return Err(misfit());
    };
    let Some(declared) = element.group.family_at(element.family) else {
        return Err(misfit());
    };
    // Qualified, the spelling `Constant::name` prints and `Found::named` reads,
    // so the name a rule writes is the name a diagnostic would have shown it.
    let Some(case) = constructor
        .strip_prefix(&*declared.name)
        .and_then(|rest| rest.strip_prefix('.'))
    else {
        return Err(misfit());
    };
    let Some(which) = declared
        .constructors
        .iter()
        .position(|declared| *declared.name == *case)
    else {
        return Err(misfit());
    };
    let Some(rule) = declared.constructor_at(u32::try_from(which).unwrap_or(u32::MAX)) else {
        return Err(misfit());
    };
    if rule.fields.len() != fields.len() {
        return Err(misfit());
    }

    let mut reading = Group::declarations(&element.group, globals);
    for param in &element.params {
        reading = reading.push(param.clone());
    }
    let mut value = Constant::constructor(&element.group, element.family, u32::try_from(which).unwrap_or(u32::MAX))
        .value(here, globals);
    for param in &element.params {
        value = apply(meter, here, value, param.clone())?;
    }
    for (binder, field) in rule.fields.iter().zip(fields) {
        let field_type = eval(meter, &reading, &binder.ty)?;
        let built = realize(meter, here, globals, field, &field_type)?;
        reading = reading.push(built.clone());
        value = apply(meter, here, value, built)?;
    }
    Ok(value)
}
