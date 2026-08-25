//! `../../rules/across-stages/03-machine-calculus.md` §2's machine forms, as
//! core constructors.
//!
//! §5.8's *fourth* family, and the one that reduces least: §2 gives its forms
//! typing rules and **no reductions**, so a machine's application is its value
//! and two machines are the same machine exactly when they were built the same
//! way. [`musa_calculus::Builtin::constructor`] is how a table says that, and this
//! module is its first caller.
//!
//! # Why none of these could have been a rule
//!
//! Every form is polymorphic in its step tag and its ports, so a *type* stands
//! on every one of their spines. [`musa_calculus::eval`]'s `canonical` answers
//! `None` at a universe, so a δ-rule registered here would block forever and
//! [`musa_calculus::Malformed::BuiltinStuck`] would never even get the chance to
//! report it. That is not a mechanism to route around: a machine has nothing to
//! compute until §3 gives it a step, and §3 is prompts 171–174.
//!
//! # What the ports are, and what is not checked here
//!
//! §2's premises read `data A`: a port holds storable data, never a function.
//! The old checker enforced it by giving every port a [`crate::infer::Kind::Data`]
//! variable, whose [`crate::infer::Unifier::bind`] refuses an arrow at any
//! depth. Both halves survive the restatement, each as something the signature
//! *says* rather than something a pass goes looking for:
//!
//! - **The step tag** is one implicit binder shared by every argument of a form,
//!   so `connect`'s two machines must agree on it and §2's "the step tag `K`
//!   prevents machines whose steps mean different things from being connected"
//!   is a property of the signature.
//! - **Storability** is `02-core-calculus.md` §1.2's `Storable` constraint,
//!   written into these signatures by [`musa_calculus::requiring_storable`] and
//!   discharged during elaboration. There is no instance for an arrow — §1.2
//!   admits instances only on declared types and generates them, so a function
//!   type is refused before any table is consulted — and the base types this
//!   compiler owns say for themselves which ones are storable data
//!   ([`crate::registry`]'s `storable`).
//!
//! So `Machine ⟨step⟩ (Nat → Nat) Nat` is refused where it is written, and
//! refused for §1.2's reason rather than by a rule about arrows. What a
//! signature still does not say is that the *step* position holds a step tag:
//! `Machine Nat A B` type-checks here and has to be refused where the surface
//! writes it, because a step tag is a host notion and this crate's registry is
//! the only thing that knows the list. Preparation refuses what is left of both
//! (§5: "a well-typed machine may still fail preparation").
//!
//! # The ninth form is not here
//!
//! `primitive` is §1's, not §2's: §2's grammar starts at `machine(p)` and gives
//! `primitive` no typing rule at all, because its type is read out of the
//! build-local registry rather than written down. See [`UNREGISTERED`].

use musa_calculus::{Builtin, Cx, ElabError, Index, Term};

use super::{HERE, ported, type0};

/// The eight forms registered here, in `BUILTIN_OWNERSHIP`'s order.
pub(super) const SPELLINGS: [&str; 8] = [
    "machine", "identity", "connect", "beside", "feedback", "copy", "drop", "swap",
];

/// The name the registration for one unit is spelled by.
///
/// Written the way a composer writes the call, because it is the only thing a
/// diagnostic about it can say: no source file can name this — the spelling is
/// not an identifier — and [`crate::lower::values`] is the one place that turns
/// `primitive("scale", 1, c)` into an application of it.
pub(crate) fn unit_spelling(id: &str, version: u32) -> String {
    format!("primitive({id:?}, {version})")
}

/// A base type per step tag this build's units count in.
///
/// A step tag is a *type* — `Machine`'s first index — and §2 gives it no values,
/// which is the whole of what it is for: two machines connect when their tags
/// are the same type, and a tag with an inhabitant would be a tag a program
/// could compute. Read off the registry rather than listed, so a build that
/// registers a unit counting something new registers the type that says so.
/// Whether `written` names one of them.
///
/// Read off the same registry [`step_tags`] is, so a build that registers a
/// unit counting something new accepts the word that names it without a second
/// list to keep in step.
///
/// One caller, and it is why this exists: the surface's `Machine<K, A, B>` is
/// the only position a step tag can be written, and the *core* signature cannot
/// refuse `Machine Nat A B` for it — a step tag is a host notion, and `base.rs`
/// keeps the mechanism in the core and the table in the host. So the position
/// restriction is checked where the position is, in [`crate::lower::types`].
pub(crate) fn is_step_tag(written: &str) -> bool {
    units()
        .into_iter()
        .any(|descriptor| descriptor.step().spelling() == written)
}

/// The ninth machine row, which is registered **per registered unit**.
///
/// `primitive(name, version, configuration)` is typed by a registry rather than
/// by a signature: the written name and version select a descriptor from
/// [`musa_score::machine`], and *that* supplies the step, the two ports, **and the
/// type of the configuration argument** — which differs per unit, so it is not
/// one Π short of writable, it is a different type per registered pair.
///
/// The registrable alternative was to take all of it explicitly —
/// `(step input output configuration : Type 0) → Text → Nat → configuration →
/// Primitive step input output` — and it was refused. Four type arguments the
/// name and version already decide are four chances for a program to say
/// something the registry contradicts, and a signature that admits them is a
/// second way to type a `primitive`: the audits' own smell. What is registered
/// would not be §1's operation, only an operation that shares its spelling.
///
/// So the answer is neither one signature nor none: it is [`primitives`], one
/// closed signature per `(name, version)` the build registers, and a reading
/// that turns the written call into an application of the one its arguments
/// name. A pair the build does not register has no signature to be applied to,
/// which is the refusal stated as a registration rather than as a check.
///
/// The spelling itself stays here and stays unregistered, and that is the point
/// rather than a leftover: `primitive` is a *source word* with no type of its
/// own, so a program that writes it anywhere but at a call with a name and a
/// version in hand is naming something that does not exist.
#[cfg(test)]
pub(super) const UNREGISTERED: [&str; 1] = ["primitive"];

pub(super) fn step_tags() -> Vec<musa_calculus::Base> {
    let mut tags: Vec<&'static str> = Vec::new();
    for descriptor in units() {
        let spelling = descriptor.step().spelling();
        if !tags.contains(&spelling) {
            tags.push(spelling);
        }
    }
    tags.into_iter().map(super::plain).collect()
}

/// One constructor per `(name, version)` this build registers, at the closed
/// type its descriptor decides.
///
/// `configuration → Primitive step input output`, with every one of the four
/// read from [`musa_score::machine`]. No binder, implicit or otherwise: a unit's
/// ports are decided by which unit it is, so there is nothing for a use site to
/// supply and nothing for the elaborator to solve.
///
/// # Errors
///
/// [`ElabError`] when a port shape names a prelude family that is not declared
/// in `cx`, which is a compiler defect.
pub(super) fn primitives(cx: &Cx) -> Result<Vec<Builtin>, ElabError> {
    let mut registered = Vec::new();
    for descriptor in units() {
        let ty = Term::pi(
            HERE,
            "configuration",
            port(cx, descriptor.configuration())?,
            applied(
                ported("Primitive").term(HERE),
                [
                    super::plain(descriptor.step().spelling()).term(HERE),
                    port(cx, descriptor.input())?,
                    port(cx, descriptor.output())?,
                ],
            ),
        );
        registered.push(Builtin::constructor(
            unit_spelling(descriptor.id(), descriptor.version()),
            ty,
            musa_calculus::Family::Machine,
        ));
    }
    Ok(registered)
}

/// Every descriptor this build registers, in registration order.
fn units() -> Vec<&'static musa_score::machine::PrimitiveDescriptor> {
    musa_score::machine::registered_ids()
        .into_iter()
        .flat_map(|id| {
            musa_score::machine::versions_of(id)
                .into_iter()
                .filter_map(move |version| musa_score::machine::descriptor(id, version))
        })
        .collect()
}

/// The core type one port shape denotes.
///
/// A product is `Pair` folded from the right, which is what `beside`'s own
/// signature already writes: §2 pairs two ports and says nothing about three,
/// so three is two of them.
fn port(cx: &Cx, shape: musa_score::machine::PortShape) -> Result<Term, ElabError> {
    match shape {
        // The storable spelling: §1.2's constraint on a port is discharged by
        // computing over this base, and a `Ratio` port is storable data —
        // which the owner says at the registration, per the one shape this
        // crate cannot look inside.
        musa_score::machine::PortShape::Ratio => Ok(super::storable("Ratio").term(HERE)),
        musa_score::machine::PortShape::Unit => crate::prelude::constant(cx, "Unit"),
        musa_score::machine::PortShape::Bool => crate::prelude::constant(cx, "Bool"),
        musa_score::machine::PortShape::Nat => crate::prelude::constant(cx, "Nat"),
        musa_score::machine::PortShape::Product(members) => {
            let pair = crate::prelude::constant(cx, "Pair")?;
            let mut built = Vec::with_capacity(members.len());
            for member in members {
                built.push(port(cx, *member)?);
            }
            let Some((last, rest)) = built.split_last() else {
                return crate::prelude::constant(cx, "Unit");
            };
            Ok(rest.iter().rev().fold(last.clone(), |second, first| {
                applied(pair.clone(), [first.clone(), second])
            }))
        }
    }
}

// ---- reading one back -------------------------------------------------------

/// The step and the two ports `ty` decides, when it decides all three.
///
/// Asked of the *type* and before the value is normalized, because the type is
/// what says whether there is a machine here at all — and asking it of every
/// definition in a document has to be cheap.
///
/// [`None`] when `ty` is not `Machine step input output`, and when any of the
/// three is something this build has no spelling for. The second is not a
/// program a source file can write: §2's forms are polymorphic in their ports,
/// and a declaration that leaves them undetermined is refused for that before it
/// is ever read back.
pub(crate) fn ports(ty: &Term) -> Option<(musa_score::machine::StepTag, String, String)> {
    let (head, arguments) = spine(ty);
    let musa_calculus::Shape::Named {
        ref name,
        role: musa_calculus::Role::Base,
        ..
    } = *head.shape()
    else {
        return None;
    };
    if &**name != "Machine" {
        return None;
    }
    let [step, input, output] = arguments[..] else {
        return None;
    };
    Some((
        musa_score::machine::StepTag::named(&spelled(step)?)?,
        spelled(input)?,
        spelled(output)?,
    ))
}

/// The nodes `normal` describes, children before parents.
///
/// The inverse of [`builtins`] and [`primitives`], and here for that reason:
/// this module wrote the eight spellings and the per-unit ones, so it is the
/// module that may read a spine built from them without any other having to
/// learn what a machine's vocabulary is.
///
/// [`None`] when the spine is not one of those forms saturated — which, after
/// [`ports`] has answered, means a compiler defect rather than a program's,
/// since the term was checked at the machine type it is being read at.
pub(crate) fn nodes(cx: &musa_calculus::Cx, normal: &Term) -> Option<Vec<musa_score::machine::SpecNode>> {
    let mut nodes = Vec::new();
    node(cx, normal, &mut nodes)?;
    Some(nodes)
}

/// Append one form's nodes to `nodes`, children first, and answer where its own
/// node landed — [`musa_score::MachineSpec`]'s promised order.
fn node(cx: &musa_calculus::Cx, term: &Term, nodes: &mut Vec<musa_score::machine::SpecNode>) -> Option<usize> {
    use musa_score::machine::{SpecForm, SpecNode};

    let (head, arguments) = spine(term);
    let musa_calculus::Shape::Named {
        ref name,
        role: musa_calculus::Role::Builtin,
        ..
    } = *head.shape()
    else {
        return None;
    };
    let built = match &**name {
        // `machine(p)` is not a node of its own. §2 gives it a typing rule
        // because a primitive is not yet a machine, and gives it nothing to do:
        // what the projection describes is the unit inside it.
        "machine" => return node(cx, written(&arguments, 1)?.first().copied()?, nodes),
        "identity" => SpecNode::wiring(SpecForm::Identity, Vec::new()),
        "copy" => SpecNode::wiring(SpecForm::Copy, Vec::new()),
        "drop" => SpecNode::wiring(SpecForm::Drop, Vec::new()),
        "swap" => SpecNode::wiring(SpecForm::Swap, Vec::new()),
        "connect" => joined(cx, SpecForm::Connect, &arguments, nodes)?,
        "beside" => joined(cx, SpecForm::Beside, &arguments, nodes)?,
        "feedback" => {
            let written = written(&arguments, 2)?;
            // The stored value before the loop, because the loop's own node
            // reads it: `initialized` takes bytes and children, and building
            // the children first would leave nothing to fail on if the value
            // turned out not to be storable.
            let initial = stored(cx, written.first().copied()?)?;
            let children = vec![node(cx, written.get(1).copied()?, nodes)?];
            SpecNode::initialized(SpecForm::Feedback, children, initial)
        }
        // The ninth form, whose spelling is a registration rather than a word:
        // see [`UNREGISTERED`].
        spelling => SpecNode::primitive(
            unit_named(spelling)?,
            stored(cx, written(&arguments, 1)?.first().copied()?)?,
        ),
    };
    nodes.push(built);
    Some(nodes.len().saturating_sub(1))
}

/// `connect` and `beside`, which differ only in which form they are.
fn joined(
    cx: &musa_calculus::Cx,
    form: musa_score::machine::SpecForm,
    arguments: &[&Term],
    nodes: &mut Vec<musa_score::machine::SpecNode>,
) -> Option<musa_score::machine::SpecNode> {
    let written = written(arguments, 2)?;
    let children = vec![
        node(cx, written.first().copied()?, nodes)?,
        node(cx, written.get(1).copied()?, nodes)?,
    ];
    Some(musa_score::machine::SpecNode::wiring(form, children))
}

/// The last `count` arguments of a spine: the ones a source program wrote.
///
/// Counted from the end because every signature above binds its step tag and
/// its ports implicitly and an elaborated term carries the solutions —
/// `machine(p)` is `machine K A B p`, so the argument a reader wants is the
/// last rather than the first, and its position depends on how many binders the
/// form has.
fn written<'a, 'b>(arguments: &'b [&'a Term], count: usize) -> Option<&'b [&'a Term]> {
    arguments.get(arguments.len().checked_sub(count)?..)
}

/// The descriptor whose registration is spelled `spelling`.
///
/// Matched against [`unit_spelling`] rather than parsed out of it: the spelling
/// is this module's own construction, and a reader that took it apart would be
/// a second place that decides what a unit's registration is called.
fn unit_named(spelling: &str) -> Option<&'static musa_score::machine::PrimitiveDescriptor> {
    units()
        .into_iter()
        .find(|descriptor| unit_spelling(descriptor.id(), descriptor.version()) == spelling)
}

/// The exact bytes a configuration or a feedback value stores.
///
/// §1.1's storable data, restated over the core's canonical data: a port shape
/// is a `Ratio`, a `Unit`, a `Bool`, a `Nat`, or a product of those, so those
/// are the cases, and a value that is none of them is one no port could have
/// held. Each case writes a distinguishing tag and every part is either fixed
/// width or a known arity, so two different values cannot write one string —
/// which is what makes a machine's digest an identity rather than a hint.
fn stored(cx: &musa_calculus::Cx, term: &Term) -> Option<Vec<u8>> {
    let mut bytes = Vec::new();
    write_stored(&musa_calculus::canonical(cx, term)?, &mut bytes)?;
    Some(bytes)
}

fn write_stored(datum: &musa_calculus::Datum, bytes: &mut Vec<u8>) -> Option<()> {
    match *datum {
        musa_calculus::Datum::Lit(ref literal) => {
            let exact = super::held::<num_rational::Ratio<i64>>(literal)?;
            bytes.push(0);
            bytes.extend_from_slice(&exact.numer().to_be_bytes());
            bytes.extend_from_slice(&exact.denom().to_be_bytes());
        }
        musa_calculus::Datum::Count { .. } => {
            bytes.push(3);
            bytes.extend_from_slice(&super::rules::nat(datum)?.to_be_bytes());
        }
        musa_calculus::Datum::Case {
            ref constructor,
            ref fields,
        } => match &**constructor {
            "Unit.Only" => bytes.push(1),
            "Bool.False" => bytes.extend_from_slice(&[2, 0]),
            "Bool.True" => bytes.extend_from_slice(&[2, 1]),
            "Pair.Both" => {
                bytes.push(4);
                write_stored(fields.first()?, bytes)?;
                write_stored(fields.get(1)?, bytes)?;
            }
            _ => return None,
        },
    }
    Some(())
}

/// How this build spells the type `ty`, in the vocabulary
/// [`musa_score::MachineSpec`] reports ports in.
///
/// Base types and the prelude's own nullary families print their names, and a
/// product prints as source writes it. [`None`] for anything else, which is
/// what makes an undecided port answer no projection: a metavariable and a
/// variable have no spelling a consumer could prepare.
fn spelled(ty: &Term) -> Option<String> {
    let (head, arguments) = spine(ty);
    match *head.shape() {
        musa_calculus::Shape::Named {
            ref name,
            role: musa_calculus::Role::Base,
            ..
        } if arguments.is_empty() => Some(name.to_string()),
        musa_calculus::Shape::Named {
            ref name,
            role:
                musa_calculus::Role::TypeConstructor
                | musa_calculus::Role::Constructor
                | musa_calculus::Role::Recursor,
            ..
        } => {
            let name = name.to_string();
            if name == "Pair" {
                let [first, second] = arguments[..] else {
                    return None;
                };
                return Some(format!("({}, {})", spelled(first)?, spelled(second)?));
            }
            arguments.is_empty().then_some(name)
        }
        // Written out rather than left to a wildcard, so that a shape added to
        // the core has to be classified here before this crate builds again —
        // `musa_calculus::canonical`'s own discipline, and for its reason.
        musa_calculus::Shape::Named { .. }
        | musa_calculus::Shape::Var(_)
        | musa_calculus::Shape::Lit(_)
        | musa_calculus::Shape::Universe(_)
        | musa_calculus::Shape::Bind { .. }
        // `App` cannot appear — the peel above ended because the head was not
        // one — and it is named anyway, because an arm that says "unreachable"
        // is a claim a later reader has to re-derive.
        | musa_calculus::Shape::App { .. }
        | musa_calculus::Shape::Meta(_)
        | musa_calculus::Shape::MetaAt { .. } => None,
    }
}

/// A term as its head and the arguments applied to it, in written order.
fn spine(term: &Term) -> (&Term, Vec<&Term>) {
    let mut head = term;
    let mut arguments = Vec::new();
    while let musa_calculus::Shape::App {
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

/// The eight forms, read off [`crate::phase::MachineOp::instantiate`] rather than
/// retyped.
///
/// ```text
/// machine  : {K A B   : Type 0} → Primitive K A B → Machine K A B
/// identity : {K A     : Type 0} → Machine K A A
/// connect  : {K A B C : Type 0} → Machine K A B → Machine K B C → Machine K A C
/// beside   : {K A B C D : Type 0} → Machine K A B → Machine K C D
///                                 → Machine K (Pair A C) (Pair B D)
/// feedback : {K A B F : Type 0} → F → Machine K (Pair A F) (Pair B F)
///                               → Machine K A B
/// copy     : {K A     : Type 0} → Machine K A (Pair A A)
/// drop     : {K A     : Type 0} → Machine K A Unit
/// swap     : {K A B   : Type 0} → Machine K (Pair A B) (Pair B A)
/// ```
///
/// Implicit binders throughout, and that is what makes `identity` a machine
/// rather than a function to one: §2 names four of the eight rather than
/// applying them, and an implicit argument is one a use site does not write.
/// The ports of a nullary form are then solved from the position it stands in,
/// which is the same thing the old checker's fresh unification variables did.
///
/// # Errors
///
/// [`ElabError`] when `Pair` or `Unit` is not declared in `cx`, which is a
/// compiler defect.
pub(super) fn builtins(cx: &Cx) -> Result<Vec<Builtin>, ElabError> {
    let words = Words {
        pair: crate::prelude::constant(cx, "Pair")?,
        unit: crate::prelude::constant(cx, "Unit")?,
    };
    Ok([
        machine_type(),
        identity_type(),
        connect_type(),
        beside_type(&words),
        feedback_type(&words),
        copy_type(&words),
        drop_type(&words),
        swap_type(&words),
    ]
    .into_iter()
    .zip(SPELLINGS)
    .map(|(ty, spelling)| Builtin::constructor(spelling, ty, musa_calculus::Family::Machine))
    .collect())
}

/// The declared constants a signature here names, resolved once.
///
/// Two, and both from [`crate::prelude`]: §2 writes its ports over `(A, D)` and
/// `Unit`, and those are `Pair` and `Unit`. Resolved through the context that
/// declared them for [`super::traversal`]'s reason — the term a signature names
/// and the term a source program's `Pair` denotes are one term, not two that
/// agree today.
struct Words {
    pair: Term,
    unit: Term,
}

// ---- the eight signatures ---------------------------------------------------
//
// Each is written in terms of the *positions* its binders stand at, counted from
// the outermost, and `at` turns a position into the index it has at a given
// depth. That is [`super::traversal`]'s idiom and it is here for the same
// reason: `step` is `Index(3)` in `connect`'s first argument and `Index(5)` in
// its result, and a signature that wrote those numbers would be a signature
// nobody could check by reading.

/// The step tag, which every form binds first.
const STEP: usize = 0;

fn machine_type() -> Term {
    let (step, input, output) = (STEP, 1, 2);
    let bound = 5;
    scheme(
        &["step", "input", "output"],
        &[input, output],
        vec![ported_type("Primitive", bound, step, input, output)],
        ported_type("Machine", bound + 1, step, input, output),
    )
}

fn identity_type() -> Term {
    let (step, port) = (STEP, 1);
    scheme(
        &["step", "port"],
        &[port],
        Vec::new(),
        ported_type("Machine", 3, step, port, port),
    )
}

fn connect_type() -> Term {
    let (step, input, middle, output) = (STEP, 1, 2, 3);
    let bound = 4;
    scheme(
        &["step", "input", "middle", "output"],
        &[],
        vec![
            ported_type("Machine", bound, step, input, middle),
            ported_type("Machine", bound + 1, step, middle, output),
        ],
        ported_type("Machine", bound + 2, step, input, output),
    )
}

fn beside_type(words: &Words) -> Term {
    let (step, input, output, other_input, other_output) = (STEP, 1, 2, 3, 4);
    let bound = 5;
    let at_depth = bound + 2;
    scheme(
        &["step", "input", "output", "other_input", "other_output"],
        &[],
        vec![
            ported_type("Machine", bound, step, input, output),
            ported_type("Machine", bound + 1, step, other_input, other_output),
        ],
        applied(
            ported("Machine").term(HERE),
            [
                at(at_depth, step),
                pair(words, at(at_depth, input), at(at_depth, other_input)),
                pair(words, at(at_depth, output), at(at_depth, other_output)),
            ],
        ),
    )
}

fn feedback_type(words: &Words) -> Term {
    let (step, input, output, stored) = (STEP, 1, 2, 3);
    let bound = 5;
    let inner = bound + 1;
    scheme(
        &["step", "input", "output", "stored"],
        &[stored],
        vec![
            at(bound, stored),
            applied(
                ported("Machine").term(HERE),
                [
                    at(inner, step),
                    pair(words, at(inner, input), at(inner, stored)),
                    pair(words, at(inner, output), at(inner, stored)),
                ],
            ),
        ],
        ported_type("Machine", bound + 2, step, input, output),
    )
}

fn copy_type(words: &Words) -> Term {
    let (step, port) = (STEP, 1);
    let bound = 2;
    scheme(
        &["step", "port"],
        &[],
        Vec::new(),
        applied(
            ported("Machine").term(HERE),
            [
                at(bound, step),
                at(bound, port),
                pair(words, at(bound, port), at(bound, port)),
            ],
        ),
    )
}

fn drop_type(words: &Words) -> Term {
    let (step, port) = (STEP, 1);
    let bound = 2;
    scheme(
        &["step", "port"],
        &[],
        Vec::new(),
        applied(
            ported("Machine").term(HERE),
            [at(bound, step), at(bound, port), words.unit.clone()],
        ),
    )
}

fn swap_type(words: &Words) -> Term {
    let (step, first, second) = (STEP, 1, 2);
    let bound = 3;
    scheme(
        &["step", "first", "second"],
        &[],
        Vec::new(),
        applied(
            ported("Machine").term(HERE),
            [
                at(bound, step),
                pair(words, at(bound, first), at(bound, second)),
                pair(words, at(bound, second), at(bound, first)),
            ],
        ),
    )
}

// ---- writing a signature ----------------------------------------------------

/// `{v₁ … vₙ : Type 0} → [Storable v_{s₁}] … → α₁ → … → αₘ → ρ`.
///
/// Every argument must already be written at the depth its own position gives it
/// — `n + c` for the first and `n + c + k` for the k-th, where `c` is
/// `storable.len()` — and the result at `n + c + m`. Folding from the right is
/// what makes that true, exactly as in [`super::traversal`]'s telescope.
///
/// `storable` names the binders §2's typing rules write `data A` above, and
/// nothing else: `machine(p)` for both ports, `identity` for its one, and
/// `feedback` for the value it stores. The forms whose rules carry no `data`
/// premise carry no constraint here — a `connect` inherits its ports from the
/// two machines it chains, and inventing a premise the document does not write
/// would refuse a program §2 admits.
fn scheme(binders: &[&'static str], storable: &[usize], arguments: Vec<Term>, result: Term) -> Term {
    let bound = binders.len();
    let applied = arguments
        .into_iter()
        .rev()
        .fold(result, |built, argument| Term::pi(HERE, "argument", argument, built));
    // Innermost constraint first, so the k-th from the *outside* stands under
    // `bound + k` binders and reads its port there.
    let constrained = storable
        .iter()
        .enumerate()
        .rev()
        .fold(applied, |built, (which, position)| {
            musa_calculus::requiring_storable(HERE, at(bound.saturating_add(which), *position), built)
        });
    binders.iter().rev().fold(constrained, |built, name| {
        Term::parameter_pi(HERE, *name, type0(), built)
    })
}

/// `Machine step input output` or `Primitive step input output`, where all three
/// are bound variables read at `depth`.
///
/// Six of the eight signatures need nothing else; the two that pair a port reach
/// for [`applied`] and [`pair`] directly.
fn ported_type(name: &'static str, depth: usize, step: usize, input: usize, output: usize) -> Term {
    applied(
        ported(name).term(HERE),
        [at(depth, step), at(depth, input), at(depth, output)],
    )
}

/// `Pair first second`.
fn pair(words: &Words, first: Term, second: Term) -> Term {
    applied(words.pair.clone(), [first, second])
}

/// `head a b …`.
fn applied(head: Term, arguments: impl IntoIterator<Item = Term>) -> Term {
    arguments
        .into_iter()
        .fold(head, |function, argument| Term::app(HERE, function, argument))
}

/// The variable bound at `position`, counted from the outermost, read at
/// `depth`.
fn at(depth: usize, position: usize) -> Term {
    let index = depth.saturating_sub(position).saturating_sub(1);
    Term::var(HERE, Index(u32::try_from(index).unwrap_or_default()))
}

#[cfg(test)]
mod laws;
