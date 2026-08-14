//! Rank-1 Hindley–Milner inference, with two classes of type variable.
//!
//! `docs/rules/language/02-core-calculus.md` §1.1 fixes the discipline this
//! module implements: an **ordinary** variable ranges over any value type, a
//! **data** variable over storable data only, unification never replaces a
//! data variable with a function or a container holding one at any depth,
//! generalization is at a declaration and instantiation at a use, and a
//! principal type exists and is computed.
//!
//! The data kind is a side condition on ordinary unification — not subtyping,
//! not overloading, and not a type class the source can name. It costs one
//! structural walk when a data variable is bound, and nothing otherwise.
//!
//! Nothing here is public. A type variable exists between the moment the
//! checker meets an unannotated declaration and the moment it finishes one;
//! every type that leaves the checker has been through [`Unifier::resolve`]
//! and, by [`Unifier::residue`], been proved to hold none.

use crate::core::Type;

/// What a type variable may stand for.
///
/// Two classes, not a lattice: there is no third kind and no ordering
/// between these two beyond the one rule in [`Unifier::bind`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    /// Any value type, a function type included.
    Ordinary,
    /// Storable data only — `docs/rules/language/02-core-calculus.md` §1.1.
    Data,
}

/// A type variable, named by the [`Unifier`] that made it.
pub(crate) type TypeVar = u32;

/// A type with its variables quantified: what a name means at a use.
///
/// Rank 1 is the whole shape of this type. The quantifiers are here, at the
/// outside, and nowhere else — there is no way to write a `Scheme` inside a
/// [`Type`], which is what makes [`Unifier::instantiate`] a substitution
/// rather than a search.
#[derive(Clone, Debug)]
pub(crate) struct Scheme {
    quantified: Vec<(TypeVar, Kind)>,
    ty: Type,
}

impl Scheme {
    /// A scheme quantifying nothing: an annotated declaration, a parameter,
    /// or a declaration still being inferred.
    pub(crate) fn monomorphic(ty: Type) -> Self {
        Self {
            quantified: Vec::new(),
            ty,
        }
    }

    /// The body with its variables renumbered in the order they are first
    /// written, so that one scheme reads the same way wherever it is shown.
    ///
    /// Renaming the whole body at once is the point: a signature whose
    /// parameter and result share a variable must still share it after
    /// renaming, which taking the parts separately would lose.
    pub(crate) fn renamed(&self) -> Type {
        let mut order = Vec::new();
        appearances(&self.ty, &mut order);
        rename(&self.ty, &order)
    }
}

/// A scheme prints with its variables renamed `a`, `b`, … in the order they
/// are first written, so that the same scheme reads the same way whichever
/// definition it was inferred in. That stability is what makes an inferred
/// type quotable in a diagnostic and comparable in a test.
impl std::fmt::Display for Scheme {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(out, "{}", self.renamed())
    }
}

/// The immediate members of a type: what a traversal has to look inside.
///
/// Every walk in this module goes through this or through [`rebuilt`], and
/// these two are the only places the shape of a type is written down. Both
/// matches are exhaustive on purpose: a type added later — `Result`, an event
/// track — cannot quietly be taken for a leaf whose members no one visits.
///
/// [`crate::data`] walks types too, for the group check, and walks them
/// through here for exactly that reason.
pub(crate) fn member_types(ty: &Type) -> Vec<&Type> {
    match ty {
        Type::Product(members) => members.iter().collect(),
        Type::Nominal(_, arguments) => arguments.iter().collect(),
        Type::Sum(value, error) => vec![value, error],
        Type::Option(member) | Type::List(member) => vec![member],
        Type::Function(parameters, result) => parameters.iter().chain(std::iter::once(result.as_ref())).collect(),
        // A machine's step, input, and output are ordinary members: the ports
        // are types that unify, and the step tag unifies like any other index.
        Type::Primitive { step, input, output } | Type::Machine { step, input, output } => {
            vec![step, input, output]
        }
        // A step's context and answer are ordinary members: the recursor's
        // group branch is what unifies them with the rest of the algebra, and
        // it can only do that if a walk looks inside.
        Type::SyntaxStep { context, answer } => vec![context, answer],
        Type::Var(_)
        | Type::Unit
        | Type::Bool
        | Type::Nat
        | Type::Ratio
        | Type::Text
        | Type::Duration(_)
        | Type::Position(_)
        | Type::Pitch
        | Type::PitchClass
        | Type::Interval
        | Type::Scale
        | Type::Key
        | Type::Degree
        | Type::Frame
        | Type::ChordClass
        | Type::Triad
        | Type::Roman
        | Type::Voicing
        | Type::Pc12
        | Type::PcSet12
        | Type::Row12
        | Type::Step(_)
        // A syntax value, a node path, and a binding path are leaves: each is
        // an opaque finite value with nothing inside it that unifies.
        | Type::Syntax
        | Type::NodePath
        | Type::BindingPath
        | Type::Music => Vec::new(),
    }
}

/// A type rebuilt with `member` applied to each of its immediate members. A
/// leaf has none and rebuilds as itself.
pub(crate) fn rebuilt(ty: &Type, mut member: impl FnMut(&Type) -> Type) -> Type {
    match ty {
        Type::Product(members) => Type::Product(members.iter().map(member).collect()),
        Type::Nominal(id, arguments) => Type::Nominal(id.clone(), arguments.iter().map(member).collect()),
        Type::Sum(value, error) => Type::Sum(Box::new(member(value)), Box::new(member(error))),
        Type::Option(inner) => Type::Option(Box::new(member(inner))),
        Type::List(inner) => Type::List(Box::new(member(inner))),
        Type::Function(parameters, result) => {
            Type::Function(parameters.iter().map(&mut member).collect(), Box::new(member(result)))
        }
        Type::Primitive { step, input, output } => Type::Primitive {
            step: Box::new(member(step)),
            input: Box::new(member(input)),
            output: Box::new(member(output)),
        },
        Type::Machine { step, input, output } => Type::Machine {
            step: Box::new(member(step)),
            input: Box::new(member(input)),
            output: Box::new(member(output)),
        },
        Type::SyntaxStep { context, answer } => Type::SyntaxStep {
            context: Box::new(member(context)),
            answer: Box::new(member(answer)),
        },
        Type::Var(_)
        | Type::Unit
        | Type::Bool
        | Type::Nat
        | Type::Ratio
        | Type::Text
        | Type::Duration(_)
        | Type::Position(_)
        | Type::Pitch
        | Type::PitchClass
        | Type::Interval
        | Type::Scale
        | Type::Key
        | Type::Degree
        | Type::Frame
        | Type::ChordClass
        | Type::Triad
        | Type::Roman
        | Type::Voicing
        | Type::Pc12
        | Type::PcSet12
        | Type::Row12
        | Type::Step(_)
        | Type::Syntax
        | Type::NodePath
        | Type::BindingPath
        | Type::Music => ty.clone(),
    }
}

/// The variables of `ty`, in the order they are first written.
fn appearances(ty: &Type, order: &mut Vec<TypeVar>) {
    if let Type::Var(variable) = ty
        && !order.contains(variable)
    {
        order.push(*variable);
    }
    for member in member_types(ty) {
        appearances(member, order);
    }
}

/// Types in plain form: their variables renamed `a`, `b`, … in the order they
/// first appear, renamed *together* so that one letter means one type across
/// every type in the list.
///
/// A diagnostic quotes what the checker settled on, and what it settled on is
/// a term over the substitution's own counter — the fortieth variable a piece
/// minted prints `n` for no reason a reader can see, and the two sides of a
/// mismatch print unrelated letters that look related. Renaming by first
/// appearance says the one thing a reader needs: which positions share a type.
/// Hover already reads this through [`Scheme::renamed`]; this is the same
/// spelling for the places that hold a bare type.
pub(crate) fn plain(types: [&Type; 2]) -> [Type; 2] {
    let mut order = Vec::new();
    for ty in types {
        appearances(ty, &mut order);
    }
    types.map(|ty| rename(ty, &order))
}

/// One type in plain form.
pub(crate) fn plain_one(ty: &Type) -> Type {
    let mut order = Vec::new();
    appearances(ty, &mut order);
    rename(ty, &order)
}

/// `ty` with each variable replaced by its position in `order`, so that
/// printing reads `a`, `b`, … left to right.
fn rename(ty: &Type, order: &[TypeVar]) -> Type {
    if let Type::Var(variable) = ty {
        return Type::Var(
            order
                .iter()
                .position(|found| found == variable)
                .and_then(|index| u32::try_from(index).ok())
                .unwrap_or(*variable),
        );
    }
    rebuilt(ty, |member| rename(member, order))
}

/// Why two types could not be made equal.
///
/// Three answers rather than one, because the three want different sentences:
/// the shapes differ, a solution would have to be infinite, or a value that
/// must be storable would have to hold a function.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Mismatch {
    /// The two types are different types.
    Shape,
    /// A variable would have to stand for a type containing itself.
    Recursive,
    /// A data variable would have to stand for a function, or a container
    /// holding one — `docs/rules/language/02-core-calculus.md` §1.1.
    NotStorable,
}

/// One variable's state: what it may stand for, and what it does.
struct Variable {
    kind: Kind,
    bound: Option<Type>,
}

/// The substitution being built while one piece is checked.
///
/// One unifier serves the whole piece, so a variable minted for a declaration
/// that did not write its type means the same thing wherever it is read. What
/// keeps declarations from constraining each other is not a second unifier
/// but generalization: a declaration is generalized when its own checking is
/// finished, and every later use instantiates the resulting [`Scheme`].
#[derive(Default)]
pub(crate) struct Unifier {
    variables: Vec<Variable>,
    constraints: u64,
}

impl Unifier {
    /// How many equations this unifier has been asked to solve.
    ///
    /// The `type_constraints` charge of `26-language-design-decision.md` §3.5,
    /// counted where the constraints are actually made rather than estimated
    /// from the shape of the term.
    pub(crate) const fn constraints(&self) -> u64 {
        self.constraints
    }

    /// A variable nothing has said anything about yet.
    pub(crate) fn fresh(&mut self, kind: Kind) -> Type {
        let variable = u32::try_from(self.variables.len()).unwrap_or(u32::MAX);
        self.variables.push(Variable { kind, bound: None });
        Type::Var(variable)
    }

    /// `ty` with every bound variable replaced, all the way down.
    ///
    /// The result holds only variables this unifier has not decided. That is
    /// the invariant every consumer outside this module depends on, and
    /// [`Self::residue`] is how a caller checks it got what it needed.
    pub(crate) fn resolve(&self, ty: &Type) -> Type {
        if let Type::Var(variable) = ty {
            return match self.state(*variable).and_then(|state| state.bound.as_ref()) {
                Some(bound) => self.resolve(bound),
                None => ty.clone(),
            };
        }
        rebuilt(ty, |member| self.resolve(member))
    }

    /// The first variable `ty` still holds after resolution, if any.
    ///
    /// A caller that needs a type the program determined asks this and
    /// reports what it finds; nothing here invents a default.
    pub(crate) fn residue(&self, ty: &Type) -> Option<TypeVar> {
        let mut order = Vec::new();
        appearances(&self.resolve(ty), &mut order);
        order.first().copied()
    }

    /// Make `left` and `right` the same type, or say why they cannot be.
    ///
    /// Bindings made before a failure are kept. The checker reports one
    /// diagnostic and stops checking that expression, so a half-solved
    /// substitution is never read as an answer.
    pub(crate) fn unify(&mut self, left: &Type, right: &Type) -> Result<(), Mismatch> {
        self.constraints = self.constraints.saturating_add(1);
        let left = self.shallow(left);
        let right = self.shallow(right);
        match (&left, &right) {
            (Type::Var(one), Type::Var(other)) if one == other => Ok(()),
            (Type::Var(variable), other) => self.bind(*variable, other),
            (other, Type::Var(variable)) => self.bind(*variable, other),
            (Type::Product(ours), Type::Product(theirs)) if ours.len() == theirs.len() => {
                for (ours, theirs) in ours.iter().zip(theirs) {
                    self.unify(ours, theirs)?;
                }
                Ok(())
            }
            (Type::Option(ours), Type::Option(theirs)) | (Type::List(ours), Type::List(theirs)) => {
                self.unify(ours, theirs)
            }
            // Nominal identity first, arguments after: two declarations that
            // happen to have the same shape are still two types, which is
            // what nominal means.
            (Type::Nominal(ours, our_arguments), Type::Nominal(theirs, their_arguments))
                if ours == theirs && our_arguments.len() == their_arguments.len() =>
            {
                for (ours, theirs) in our_arguments.iter().zip(their_arguments) {
                    self.unify(ours, theirs)?;
                }
                Ok(())
            }
            (Type::Sum(our_value, our_error), Type::Sum(their_value, their_error)) => {
                self.unify(our_value, their_value)?;
                self.unify(our_error, their_error)
            }
            (Type::Function(ours, our_result), Type::Function(theirs, their_result)) if ours.len() == theirs.len() => {
                for (ours, theirs) in ours.iter().zip(theirs) {
                    self.unify(ours, theirs)?;
                }
                self.unify(our_result, their_result)
            }
            // A machine and a registered unit are two type formers, not one:
            // `connect` takes machines, and a primitive reaches it through
            // `machine(p)`. Unifying the step first is deliberate, so that
            // wiring two unlike steps together is reported as the step
            // disagreeing rather than as the port that happened to be read
            // first (`../rules/across-stages/03-machine-calculus.md` §2).
            (
                Type::Primitive {
                    step: our_step,
                    input: our_input,
                    output: our_output,
                },
                Type::Primitive {
                    step: their_step,
                    input: their_input,
                    output: their_output,
                },
            )
            | (
                Type::Machine {
                    step: our_step,
                    input: our_input,
                    output: our_output,
                },
                Type::Machine {
                    step: their_step,
                    input: their_input,
                    output: their_output,
                },
            ) => {
                self.unify(our_step, their_step)?;
                self.unify(our_input, their_input)?;
                self.unify(our_output, their_output)
            }
            // Two sealed steps are one type when they are run under the same
            // context and answer with the same thing. Which child and which
            // algebra a step was minted for is not in its type and could not
            // be: the recursor's group branch is handed a *list* of steps, and
            // they descend to different children.
            (
                Type::SyntaxStep {
                    context: our_context,
                    answer: our_answer,
                },
                Type::SyntaxStep {
                    context: their_context,
                    answer: their_answer,
                },
            ) => {
                self.unify(our_context, their_context)?;
                self.unify(our_answer, their_answer)
            }
            _ if left == right => Ok(()),
            _ => Err(Mismatch::Shape),
        }
    }

    /// A fresh copy of `scheme`: one instantiation per use, which is what
    /// lets one declaration serve two types without either use reaching the
    /// other.
    pub(crate) fn instantiate(&mut self, scheme: &Scheme) -> Type {
        if scheme.quantified.is_empty() {
            return scheme.ty.clone();
        }
        let replacements: Vec<(TypeVar, Type)> = scheme
            .quantified
            .iter()
            .map(|(variable, kind)| (*variable, self.fresh(*kind)))
            .collect();
        substitute(&scheme.ty, &replacements)
    }

    /// Quantify everything `ty` has not pinned down.
    ///
    /// Quantifying *all* the free variables is right here and only here: a
    /// declaration is generalized when its own checking is finished, and the
    /// environment it was checked against — other declarations — holds no
    /// free variable of this unifier to capture.
    pub(crate) fn generalize(&self, ty: &Type) -> Scheme {
        let resolved = self.resolve(ty);
        let mut order = Vec::new();
        appearances(&resolved, &mut order);
        Scheme {
            quantified: order
                .into_iter()
                .map(|variable| (variable, self.kind_of(variable)))
                .collect(),
            ty: resolved,
        }
    }

    /// What a variable may stand for. An unknown variable — one from another
    /// unifier, which the checker's structure prevents — reads as ordinary.
    pub(crate) fn kind_of(&self, variable: TypeVar) -> Kind {
        self.state(variable).map_or(Kind::Ordinary, |state| state.kind)
    }

    /// Follow variable-to-variable bindings, and no further.
    fn shallow(&self, ty: &Type) -> Type {
        if let Type::Var(variable) = ty
            && let Some(bound) = self.state(*variable).and_then(|state| state.bound.as_ref())
        {
            return self.shallow(&bound.clone());
        }
        ty.clone()
    }

    fn state(&self, variable: TypeVar) -> Option<&Variable> {
        usize::try_from(variable)
            .ok()
            .and_then(|index| self.variables.get(index))
    }

    /// Decide `variable` to be `ty`.
    ///
    /// Two conditions, in the order they can fail. The occurs check refuses a
    /// solution that would have to be infinite. The storability check refuses
    /// a *data* variable a function at any depth, and where it meets an
    /// undecided ordinary variable it makes that variable a data variable
    /// too — because whatever it turns out to be will sit inside something
    /// that must be storable.
    fn bind(&mut self, variable: TypeVar, ty: &Type) -> Result<(), Mismatch> {
        if self.occurs(variable, ty) {
            return Err(Mismatch::Recursive);
        }
        if self.kind_of(variable) == Kind::Data {
            self.demand_data(ty)?;
        }
        if let Some(index) = usize::try_from(variable).ok()
            && let Some(state) = self.variables.get_mut(index)
        {
            state.bound = Some(ty.clone());
            return Ok(());
        }
        Err(Mismatch::Shape)
    }

    fn occurs(&self, variable: TypeVar, ty: &Type) -> bool {
        let mut order = Vec::new();
        appearances(&self.resolve(ty), &mut order);
        order.contains(&variable)
    }

    /// `ty` must be storable data — structurally, at every depth.
    ///
    /// This is `docs/rules/language/02-core-calculus.md` §1.1's rule written
    /// out: an arrow is never storable data, and neither is any container
    /// holding one, "including at a depth the surface never writes out, which
    /// is why the check is structural rather than a surface-syntax rule".
    fn demand_data(&mut self, ty: &Type) -> Result<(), Mismatch> {
        let ty = self.shallow(ty);
        // A sealed step is refused here beside the arrow, and for a stronger
        // reason: what it hides is an algebra of source closures, so a `data`
        // variable that admitted one would have admitted four functions at
        // once (`docs/rules/language/02-core-calculus.md` §5.9).
        if matches!(ty, Type::Function(_, _) | Type::SyntaxStep { .. }) {
            return Err(Mismatch::NotStorable);
        }
        if let Type::Var(variable) = ty {
            if let Ok(index) = usize::try_from(variable)
                && let Some(state) = self.variables.get_mut(index)
            {
                state.kind = Kind::Data;
            }
            return Ok(());
        }
        for member in member_types(&ty) {
            self.demand_data(&member.clone())?;
        }
        Ok(())
    }
}

/// Whether `found` is a type `declared` admits.
///
/// A variable admits anything. That is not laxity: where a declared type
/// still holds a variable, the declaration is *polymorphic*, and which type
/// it stands for was chosen by the caller and already proved by the checker.
/// This relation is how the parts of the compiler that meet a type without a
/// unifier — evaluation checking a closure's argument, a module matching a
/// signature — restate that proof instead of re-deciding it.
///
/// Everywhere else it is equality, which is the whole point: nothing else is
/// weakened by allowing this one thing.
pub(crate) fn admits(declared: &Type, found: &Type) -> bool {
    match (declared, found) {
        (Type::Var(_), _) | (_, Type::Var(_)) => true,
        (Type::Product(ours), Type::Product(theirs)) => {
            ours.len() == theirs.len() && ours.iter().zip(theirs).all(|(ours, theirs)| admits(ours, theirs))
        }
        (Type::Option(ours), Type::Option(theirs)) | (Type::List(ours), Type::List(theirs)) => admits(ours, theirs),
        (Type::Sum(our_value, our_error), Type::Sum(their_value, their_error)) => {
            admits(our_value, their_value) && admits(our_error, their_error)
        }
        (Type::Nominal(ours, our_arguments), Type::Nominal(theirs, their_arguments)) => {
            ours == theirs
                && our_arguments.len() == their_arguments.len()
                && our_arguments
                    .iter()
                    .zip(their_arguments)
                    .all(|(ours, theirs)| admits(ours, theirs))
        }
        (Type::Function(ours, our_result), Type::Function(theirs, their_result)) => {
            ours.len() == theirs.len()
                && ours.iter().zip(theirs).all(|(ours, theirs)| admits(ours, theirs))
                && admits(our_result, their_result)
        }
        _ => declared == found,
    }
}

/// `ty` with each listed variable replaced by its partner.
fn substitute(ty: &Type, replacements: &[(TypeVar, Type)]) -> Type {
    if let Type::Var(variable) = ty {
        return replacements
            .iter()
            .find(|(named, _)| named == variable)
            .map_or_else(|| ty.clone(), |(_, replacement)| replacement.clone());
    }
    rebuilt(ty, |member| substitute(member, replacements))
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::{Kind, Mismatch, Scheme, Unifier};
    use crate::core::Type;

    /// The rule §1.1 states, at the depth it states it: not just "a data
    /// variable is not a function" but "and neither is any container holding
    /// one, including at a depth the surface never writes out".
    #[test]
    fn a_data_variable_never_stands_for_a_function_at_any_depth() {
        let arrow = Type::Function(vec![Type::Nat], Box::new(Type::Nat));
        let buried = Type::List(Box::new(Type::Option(Box::new(Type::Product(vec![
            Type::Nat,
            arrow.clone(),
        ])))));
        // A sum holding a function on either side, which is where a `Result`
        // would smuggle one in: the failure side is as storable as the
        // success side, and neither may be an arrow.
        let returned = Type::Sum(Box::new(Type::Row12), Box::new(arrow.clone()));
        let carried = Type::Sum(Box::new(arrow.clone()), Box::new(Type::Nat));
        // And the sealed step, which holds no *written* arrow and is refused
        // anyway: what it hides is an algebra of source closures, so admitting
        // one to a data variable would admit four functions at once.
        let sealed = Type::SyntaxStep {
            context: Box::new(Type::Nat),
            answer: Box::new(Type::Nat),
        };
        let stored = Type::List(Box::new(sealed.clone()));
        for refused in [arrow, buried, returned, carried, sealed, stored] {
            let mut unifier = Unifier::default();
            let data = unifier.fresh(Kind::Data);
            assert_eq!(
                unifier.unify(&data, &refused),
                Err(Mismatch::NotStorable),
                "a data variable took `{refused}`"
            );
        }

        // The same shapes without the arrow are ordinary storable data, so
        // the refusal is about functions rather than about depth.
        let mut unifier = Unifier::default();
        let data = unifier.fresh(Kind::Data);
        let stored = Type::List(Box::new(Type::Option(Box::new(Type::Product(vec![
            Type::Nat,
            Type::Pitch,
        ])))));
        assert_eq!(unifier.unify(&data, &stored), Ok(()));
        assert_eq!(unifier.resolve(&data), stored);
    }

    /// An ordinary variable that ends up *inside* a data variable becomes a
    /// data variable: whatever it turns out to be will be stored, so the
    /// condition has to travel with it rather than be checked once.
    #[test]
    fn a_variable_inside_a_data_variable_becomes_one() {
        let mut unifier = Unifier::default();
        let data = unifier.fresh(Kind::Data);
        let ordinary = unifier.fresh(Kind::Ordinary);
        assert_eq!(ordinary, Type::Var(1), "the second variable minted is named 1");
        assert_eq!(unifier.unify(&data, &Type::List(Box::new(ordinary.clone()))), Ok(()));
        assert_eq!(unifier.kind_of(1), Kind::Data);
        assert_eq!(
            unifier.unify(&ordinary, &Type::Function(vec![Type::Nat], Box::new(Type::Nat))),
            Err(Mismatch::NotStorable)
        );
    }

    /// A solution that would have to be infinite is refused rather than
    /// built. Without this the unifier loops on `a = List<a>`.
    #[test]
    fn a_variable_never_stands_for_a_type_containing_itself() {
        let mut unifier = Unifier::default();
        let variable = unifier.fresh(Kind::Ordinary);
        assert_eq!(
            unifier.unify(&variable, &Type::List(Box::new(variable.clone()))),
            Err(Mismatch::Recursive)
        );
    }

    /// Instantiation is what makes one declaration serve two uses. Two
    /// instances of a scheme share no variable, so deciding one decides
    /// nothing about the other.
    #[test]
    fn two_uses_of_one_scheme_reach_each_other_not_at_all() {
        let mut unifier = Unifier::default();
        let variable = unifier.fresh(Kind::Ordinary);
        let identity = Type::Function(vec![variable.clone()], Box::new(variable));
        let scheme = unifier.generalize(&identity);
        assert_eq!(scheme.to_string(), "a -> a", "a scheme reads in written spelling");

        let first = unifier.instantiate(&scheme);
        let second = unifier.instantiate(&scheme);
        assert_eq!(
            unifier.unify(&first, &Type::Function(vec![Type::Nat], Box::new(Type::Nat))),
            Ok(())
        );
        assert_eq!(
            unifier.unify(&second, &Type::Function(vec![Type::Pitch], Box::new(Type::Pitch))),
            Ok(())
        );
        assert_eq!(
            unifier.resolve(&second),
            Type::Function(vec![Type::Pitch], Box::new(Type::Pitch))
        );
    }

    /// A scheme with nothing to quantify is the annotated case, and must not
    /// acquire polymorphism by passing through generalization: every instance
    /// of it is the one type it names.
    #[test]
    fn a_ground_type_generalizes_to_itself() {
        let mut unifier = Unifier::default();
        let pitches = Type::List(Box::new(Type::Pitch));
        let scheme = unifier.generalize(&pitches);
        assert_eq!(unifier.instantiate(&scheme), pitches);
        assert_eq!(unifier.instantiate(&Scheme::monomorphic(Type::Nat)), Type::Nat);
    }

    /// Types built from the pieces the unifier actually has to take apart:
    /// two base types to tell apart, every container, and two variables so a
    /// generated type can share one with itself.
    fn any_type() -> impl Strategy<Value = Type> {
        let leaf = prop_oneof![
            Just(Type::Nat),
            Just(Type::Pitch),
            Just(Type::Text),
            Just(Type::Var(0)),
            Just(Type::Var(1)),
        ];
        leaf.prop_recursive(4, 24, 3, |inner| {
            prop_oneof![
                inner.clone().prop_map(|member| Type::List(Box::new(member))),
                inner.clone().prop_map(|member| Type::Option(Box::new(member))),
                (inner.clone(), inner.clone()).prop_map(|(value, error)| Type::Sum(Box::new(value), Box::new(error))),
                prop::collection::vec(inner.clone(), 1..3).prop_map(Type::Product),
                (prop::collection::vec(inner.clone(), 1..3), inner.clone())
                    .prop_map(|(parameters, result)| Type::Function(parameters, Box::new(result))),
                // A sealed step is generated so that the two properties below
                // reach it: it unifies structurally like any other former, and
                // it is refused a data variable the way an arrow is
                // (`../rules/language/02-core-calculus.md` §5.9).
                (inner.clone(), inner).prop_map(|(context, answer)| Type::SyntaxStep {
                    context: Box::new(context),
                    answer: Box::new(answer),
                }),
            ]
        })
    }

    proptest! {
        /// Soundness: a substitution the unifier accepts really does make the
        /// two types one type. This is the property everything above the
        /// unifier assumes and nothing above it checks.
        #[test]
        fn a_successful_unification_makes_the_two_types_equal(left in any_type(), right in any_type()) {
            let mut unifier = Unifier::default();
            let ordinary = unifier.fresh(Kind::Ordinary);
            let other = unifier.fresh(Kind::Ordinary);
            prop_assert_eq!(&ordinary, &Type::Var(0));
            prop_assert_eq!(&other, &Type::Var(1));
            if unifier.unify(&left, &right).is_ok() {
                prop_assert_eq!(unifier.resolve(&left), unifier.resolve(&right));
            }
        }

        /// Idempotence: unifying an already-solved pair adds nothing. A
        /// substitution that kept growing would mean `resolve` was not
        /// reaching a fixed point, and every consumer reads a resolved type.
        #[test]
        fn unifying_a_solved_pair_again_changes_nothing(left in any_type(), right in any_type()) {
            let mut unifier = Unifier::default();
            let (first, second) = (unifier.fresh(Kind::Ordinary), unifier.fresh(Kind::Ordinary));
            prop_assert_eq!((first, second), (Type::Var(0), Type::Var(1)));
            if unifier.unify(&left, &right).is_ok() {
                let settled = unifier.resolve(&left);
                prop_assert_eq!(unifier.unify(&left, &right), Ok(()));
                prop_assert_eq!(unifier.resolve(&left), settled);
            }
        }

        /// §1.1's storable-data rule, over every type the generator can build:
        /// a data variable takes a type exactly when no arrow appears anywhere
        /// inside it. Stating it as a property rather than a list of shapes is
        /// what makes it a rule about *structure* — a sum, and so a `Result`,
        /// is storable data exactly when both its members are, and it earns
        /// that from the same clause every other container earns it from.
        #[test]
        fn a_data_variable_takes_exactly_the_types_holding_no_arrow(ty in any_type()) {
            fn holds_an_arrow(ty: &Type) -> bool {
                // A sealed step counts, and its members are not looked at: it
                // is refused for what it *hides*, so a `SyntaxStep<Nat, Nat>`
                // with no arrow written anywhere in it is still not storable.
                matches!(ty, Type::Function(_, _) | Type::SyntaxStep { .. })
                    || super::member_types(ty).into_iter().any(holds_an_arrow)
            }
            let mut unifier = Unifier::default();
            // The two variables the generator can name, minted first so that
            // `Var(0)` and `Var(1)` mean what the generated type says they do.
            let (first, second) = (unifier.fresh(Kind::Ordinary), unifier.fresh(Kind::Ordinary));
            prop_assert_eq!((first, second), (Type::Var(0), Type::Var(1)));
            let data = unifier.fresh(Kind::Data);
            prop_assert_eq!(
                unifier.unify(&data, &ty).is_err(),
                holds_an_arrow(&ty),
                "`{}` was classified against §1.1's structural rule",
                ty
            );
        }

        /// Principality, as the unifier can state it: the solution binds only
        /// variables, never invents a constraint the two types did not
        /// contain. A resolved type therefore holds no variable the pair did
        /// not hold.
        #[test]
        fn a_solution_introduces_no_variable_of_its_own(left in any_type(), right in any_type()) {
            let mut unifier = Unifier::default();
            let (first, second) = (unifier.fresh(Kind::Ordinary), unifier.fresh(Kind::Ordinary));
            prop_assert_eq!((first, second), (Type::Var(0), Type::Var(1)));
            if unifier.unify(&left, &right).is_ok() {
                let solved = unifier.resolve(&left);
                let mut order = Vec::new();
                super::appearances(&solved, &mut order);
                for variable in order {
                    prop_assert!(variable < 2, "the solution invented `{variable}`");
                }
            }
        }
    }
}
