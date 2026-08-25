//! The bridge to canonical payloads: a term read as a datum, and a datum
//! realized as a value of a family.
//!
//! One concern of the `family` module; see its docs for the calculus.

use super::constant::{Constant, Numeral};
use super::group::{Group, element};
use super::iota::saturated;
use crate::kernel::base::Datum;
use crate::kernel::budget::Meter;
use crate::kernel::context::Globals;
use crate::kernel::error::{CoreError, Malformed};
use crate::kernel::eval::{apply, eval};
use crate::kernel::origin::Origin;
use crate::kernel::term::{Constant as Written, Definition, Name, Shape, Term};
use crate::kernel::value::{Form, Value};
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

/// [`canonical`] with the table already in hand, which is what this crate's own
/// callers pass.
///
/// A loop over a work stack rather than a recursion, for the reason
/// [`realize`] is one: the depth here is the depth of the *data*, a
/// six-hundred-element list is six hundred levels, and a walk that spends a
/// host frame a level aborts on the data a δ-rule is handed rather than
/// answering about it. Nothing is charged — this reads a normal form the caller
/// already paid to compute, and §4's "a value is charged once, where it is
/// constructed" is the sentence that says a projection of it is free.
fn read(globals: &Globals, term: &Term) -> Option<Datum> {
    /// One constructor whose earlier fields are read and whose later ones are not.
    struct Reading<'a> {
        constructor: Name,
        /// The fields still to read, pushed reversed so `pop` takes the next.
        rest: Vec<&'a Term>,
        read: Vec<Datum>,
    }
    let mut stack: Vec<Reading<'_>> = Vec::new();
    let mut here: &Term = term;
    loop {
        let mut answer = match one(globals, here)? {
            Read::Leaf(datum) => datum,
            Read::Case {
                constructor,
                mut fields,
            } => {
                fields.reverse();
                match fields.pop() {
                    Some(first) => {
                        stack.push(Reading {
                            constructor,
                            rest: fields,
                            read: Vec::new(),
                        });
                        here = first;
                        continue;
                    }
                    None => Datum::Case {
                        constructor,
                        fields: Vec::new(),
                    },
                }
            }
        };
        // Hand the field up, and keep handing finished constructors up until
        // one still has a field waiting.
        loop {
            let Some(mut reading) = stack.pop() else {
                return Some(answer);
            };
            reading.read.push(answer);
            match reading.rest.pop() {
                Some(next) => {
                    stack.push(reading);
                    here = next;
                    break;
                }
                None => {
                    answer = Datum::Case {
                        constructor: reading.constructor,
                        fields: reading.read,
                    };
                }
            }
        }
    }
}

/// What one node of [`read`] is, before its fields have been read.
enum Read<'a> {
    Leaf(Datum),
    Case { constructor: Name, fields: Vec<&'a Term> },
}

/// One node classified: the whole of the old `read`'s match, minus its
/// recursion into the fields.
fn one<'a>(globals: &Globals, term: &'a Term) -> Option<Read<'a>> {
    let (head, arguments) = applied_spine(term);
    match *head.shape() {
        Shape::Meta(_) | Shape::MetaAt { .. } => None,
        Shape::Lit(Written::Payload(ref literal)) if arguments.is_empty() => {
            Some(Read::Leaf(Datum::Lit(literal.clone())))
        }
        Shape::Lit(Written::Numeral(ref numeral)) if arguments.is_empty() => counted(numeral).map(Read::Leaf),
        Shape::Named {
            ref name,
            ref role,
            ref levels,
        } => {
            let Definition::Declared(constant) = globals.definition(name, role, levels) else {
                return None;
            };
            let (constructor, params) = saturated(&constant, arguments.len())?;
            Some(Read::Case {
                constructor,
                fields: arguments.into_iter().skip(params).collect(),
            })
        }
        // Written out rather than left to a wildcard so that the list in the
        // doc comment above is checked by the compiler: a shape added to the
        // core has to be classified here before this crate builds again.
        //
        // `App` cannot appear — the peel above ended because the head was not
        // one — and it is named anyway, because an arm that says "unreachable"
        // is a claim a later reader has to re-derive.
        Shape::Lit(_) | Shape::Var(_) | Shape::Universe(_) | Shape::Bind { .. } | Shape::App { .. } => None,
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
/// # Why this is a loop and charges steps
///
/// The inverse walk of [`crate::kernel::eval`]'s `canonical` has the same two
/// mistakes to unmake, and unmakes them the same way: the descent is an
/// explicit stack in this function's own frame, and the charge is one step a
/// node rather than one **nesting** level. §4.1 derives nesting from how deeply
/// a term is written and how deeply `quote` walks a value back; building a
/// δ-rule's answer is neither, and the depth it reached was the length of the
/// list the rule answered with. See `canonical`'s own note, note 59, and
/// prompt 165b.
///
/// The frame holds the group by `Arc` and the constructor by number rather than
/// by reference, so that looking a field's declared type up is an index rather
/// than a borrow the frame would have to outlive.
///
/// # Errors
///
/// [`Malformed::MisfitAnswer`] when the data and the type disagree, otherwise as
/// [`eval`], plus the step budget at data larger than the meter allows.
pub(crate) fn realize(
    meter: &mut Meter,
    here: Origin,
    globals: &Globals,
    datum: &Datum,
    ty: &Value,
) -> Result<Value, CoreError> {
    let mut stack: Vec<Building<'_>> = Vec::new();
    let mut datum: &Datum = datum;
    let mut ty: Value = ty.clone();
    loop {
        meter.step("data realization")?;
        let mut answer = match *datum {
            Datum::Lit(ref literal) => {
                let (nodes, bytes) = literal.logical_shape();
                meter.construct("data realization", nodes, bytes)?;
                Value::new(here, Form::Lit(literal.clone()))
            }
            Datum::Count { ref family, count } => realize_count(meter, here, family, count, &ty)?,
            Datum::Case {
                ref constructor,
                ref fields,
            } => {
                let building = entered(meter, here, globals, constructor, fields, &ty)?;
                match waiting(meter, &building)? {
                    Some((field, field_type)) => {
                        stack.push(building);
                        datum = field;
                        ty = field_type;
                        continue;
                    }
                    None => building.value,
                }
            }
        };
        // Hand the field up, and keep handing saturated constructors up until
        // one still has a field waiting.
        loop {
            let Some(mut building) = stack.pop() else {
                return Ok(answer);
            };
            building.reading = building.reading.push(answer.clone());
            building.value = apply(meter, here, building.value, answer)?;
            building.index = building.index.saturating_add(1);
            match waiting(meter, &building)? {
                Some((field, field_type)) => {
                    stack.push(building);
                    datum = field;
                    ty = field_type;
                    break;
                }
                None => answer = building.value,
            }
        }
    }
}

/// One constructor whose earlier fields are built and whose later ones are not.
struct Building<'a> {
    group: Arc<Group>,
    family: u32,
    which: u32,
    /// The data still to build, in the constructor's own field order.
    fields: &'a [Datum],
    index: usize,
    /// The declaration context, the parameters, and the fields built so far —
    /// the environment the next field's declared type is read in.
    reading: crate::kernel::value::Env,
    /// The constructor applied to the parameters and to the fields built so far.
    value: Value,
}

/// The field this constructor is waiting on, at the type it is declared with,
/// or `None` where every field is built.
fn waiting<'a>(meter: &mut Meter, building: &Building<'a>) -> Result<Option<(&'a Datum, Value)>, CoreError> {
    let Some(field) = building.fields.get(building.index) else {
        return Ok(None);
    };
    let Some(binder) = building
        .group
        .family_at(building.family)
        .and_then(|declared| declared.constructor_at(building.which))
        .and_then(|rule| rule.fields.get(building.index))
    else {
        return Ok(None);
    };
    let field_type = eval(meter, &building.reading, &binder.ty)?;
    Ok(Some((field, field_type)))
}

/// One constructor application resolved against the family type it stands at:
/// which constructor it is, the environment its fields are read in, and the
/// constant applied to the family's parameters.
///
/// Everything [`realize_case`] used to do before its field loop, which is now
/// [`realize`]'s own loop.
fn entered<'a>(
    meter: &mut Meter,
    here: Origin,
    globals: &Globals,
    constructor: &Name,
    fields: &'a [Datum],
    ty: &Value,
) -> Result<Building<'a>, CoreError> {
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
    let which = u32::try_from(which).unwrap_or(u32::MAX);
    let Some(rule) = declared.constructor_at(which) else {
        return Err(misfit());
    };
    if rule.fields.len() != fields.len() {
        return Err(misfit());
    }

    let mut reading = Group::declarations(&element.group, globals);
    for param in &element.params {
        reading = reading.push(param.clone());
    }
    meter.construct("data realization", 1, 1)?;
    let mut value = Constant::constructor(&element.group, element.family, which).value(here, globals);
    for param in &element.params {
        value = apply(meter, here, value, param.clone())?;
    }
    Ok(Building {
        group: Arc::clone(&element.group),
        family: element.family,
        which,
        fields,
        index: 0,
        reading,
        value,
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
    meter.construct("data realization", 1, 8)?;
    Ok(Value::new(
        here,
        Form::Numeral(Numeral {
            family: constant,
            count,
        }),
    ))
}
