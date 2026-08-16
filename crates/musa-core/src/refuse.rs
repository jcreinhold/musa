//! What elaboration says when it rejects a program.
//!
//! `docs/rules/language/02-core-calculus.md` §4 gives every judgment three
//! outcomes — accepted, refused, exhausted — and [`ElabError`] is that list
//! written down. Refusal and exhaustion are separate variants rather than two
//! shades of one error because §4 is explicit that they are different answers:
//! "this program was not rejected; the checker ran out of room" is a different
//! sentence from "this program is wrong", and a caller that could not tell them
//! apart would let a resource limit silently decide a program's meaning.
//!
//! # How much of a type a mismatch prints
//!
//! This is the policy the prompt owes, and it is short: **report the smallest
//! pair of subterms unification actually disagreed on, together with the path
//! from the root of the two types to it.** Not the whole normal forms.
//!
//! It falls out of the algorithm rather than being a separate printer.
//! Unification descends two types structurally, forcing each head only as far as
//! it needs to compare it, so the pair it fails on is by construction the
//! shallowest place they differ and is already unfolded exactly as far as
//! deciding the question required. Anything more would be a second traversal
//! whose output no rule constrains — and a conversion error that prints two
//! forty-line normal forms is a failed diagnostic even when it is a correct one.
//!
//! The path is what keeps that honest. `Nat` versus `Bool` is useless on its
//! own; "the domain of the second argument's function type: expected `Nat`,
//! found `Bool`" is not. And because every term carries an [`Origin`] (§7), each
//! side of the pair can point at the source node it came from even though
//! neither is a term the author wrote.

use std::fmt;

use crate::budget::ResourceError;
use crate::error::{CoreError, Malformed};
use crate::meta::MetaSource;
use crate::origin::Origin;
use crate::term::{Name, Term};
use crate::visibility::ModuleId;

/// Why elaboration did not produce a core term.
///
/// §4's three outcomes, minus acceptance: the two ways of not answering, plus
/// the caller defect [`CoreError`] already distinguishes.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ElabError {
    /// The program was rejected, and this says why.
    #[error(transparent)]
    Refused(#[from] Refusal),
    /// The deterministic budget ended elaboration. **Not** a type error: the
    /// program was not judged.
    #[error(transparent)]
    Exhausted(ResourceError),
    /// A term handed in — a checking type, or a term given to the re-checker —
    /// was not one this crate could have produced.
    #[error("malformed core term: {0}")]
    Malformed(Malformed),
}

impl From<Malformed> for ElabError {
    fn from(malformed: Malformed) -> Self {
        Self::Malformed(malformed)
    }
}

impl From<CoreError> for ElabError {
    fn from(error: CoreError) -> Self {
        match error {
            CoreError::Exhausted(exhausted) => Self::Exhausted(exhausted),
            CoreError::Malformed(malformed) => Self::Malformed(malformed),
        }
    }
}

/// A program elaboration rejected, and the reason.
///
/// Every variant carries the [`Origin`] of the term it is about, because a
/// refusal a reader cannot locate is barely a refusal.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum Refusal {
    /// A name with no binder in scope.
    #[error("no binder named `{name}` is in scope")]
    UnknownName {
        /// The name as written.
        name: Name,
        /// Where it was written.
        at: Origin,
    },
    /// §2's `Switch` called conversion and conversion said no.
    #[error("{0}")]
    Mismatch(Box<Mismatch>),
    /// A name that exists, and is private to the module that declared it.
    ///
    /// Deliberately not [`Self::UnknownName`]: `01-surface.md` §1.3's value is
    /// in telling a reader that the thing they wrote is real and maintained
    /// somewhere else, which is a different sentence from "no such name".
    #[error("`{name}` is private to the module that declares it")]
    Private {
        /// The name as written.
        name: Name,
        /// The module it is private to. Opaque here — this crate does not know
        /// what a module is called, so a caller that does renders it.
        module: ModuleId,
        /// Where it was written.
        at: Origin,
    },
    /// An enum with a `private` case beside a public one.
    ///
    /// Refused at the declaration rather than at each use, because a partly
    /// private type has no coverage rule anyone would want to explain: outside
    /// the module, the arms an author is allowed to write never exhaust it.
    #[error("`{family}` has both public and private cases: `{public}` and `{private}`")]
    MixedVisibility {
        /// The family whose cases disagree.
        family: Name,
        /// The first public case.
        public: Name,
        /// The first private case.
        private: Name,
        /// Where the public case was written.
        at: Origin,
    },
    /// A `match` that would take apart a family whose cases are private here.
    ///
    /// Refused where it is written rather than silently made inexhaustive. A
    /// client eliminates through whatever its package exports, which is the
    /// point of hiding the cases.
    #[error("`{family}`'s cases are private to the module that declares it, so this cannot take one apart")]
    AbstractMatch {
        /// The family the subject belongs to.
        family: Name,
        /// The module its cases are private to.
        module: ModuleId,
        /// The subject the split was going to be on.
        at: Origin,
    },
    /// A metavariable was still unsolved when the declaration that created it
    /// ended. Never defaulted and never generalized (§2.1).
    #[error("could not determine {}", site.describe())]
    Unsolved {
        /// Which of §2.1's sites created it. Named `site` and not `source`
        /// because `thiserror` reads a field called `source` as the error this
        /// one wraps, and a [`MetaSource`] is not an error.
        site: MetaSource,
        /// The term whose elaboration created it.
        created: Origin,
        /// The term whose constraint was still postponed, when one was.
        blocked: Option<Origin>,
    },
    /// Something that is not a function was applied.
    #[error("this is applied to an argument, but its type is not a function type")]
    NotAFunction {
        /// The application.
        at: Origin,
        /// The type the applied term turned out to have.
        ty: Term,
    },
    /// An implicit argument was written in braces at an explicit binder, or an
    /// explicit binder was abstracted where the type wanted an implicit one.
    #[error("this argument is written implicitly, but the binder it fills is not")]
    PlicityMismatch {
        /// The argument or binder.
        at: Origin,
    },
    /// Something that is not a record was projected.
    #[error("this is projected at a field, but its type is not a record type")]
    NotARecord {
        /// The projection.
        at: Origin,
        /// The type the projected term turned out to have.
        ty: Term,
    },
    /// A record type has no such field.
    #[error("this record type has no field named `{field}`")]
    NoSuchField {
        /// The projection.
        at: Origin,
        /// The field asked for.
        field: Name,
    },
    /// A record literal's fields are not the record type's, in order.
    ///
    /// Order and not merely membership: a later field's type may mention an
    /// earlier field's value, so the telescope order is part of the type rather
    /// than a formatting preference.
    #[error("this record literal does not give the type's fields in order")]
    RecordShape {
        /// The literal.
        at: Origin,
        /// The fields the type declares, in telescope order.
        expected: Vec<Name>,
        /// The fields the literal writes, in the order written.
        found: Vec<Name>,
    },
    /// A record type declares one field name twice.
    ///
    /// A telescope with two `f`s is not merely confusing: the second shadows the
    /// first, so one of the two is a field no projection can ever reach and no
    /// literal can decline to write.
    #[error("this record type declares `{field}` twice")]
    DuplicateField {
        /// The second declaration.
        at: Origin,
        /// The first, so the report can point at both.
        previous: Origin,
        /// The name declared twice.
        field: Name,
    },
    /// A family declares one constructor name twice.
    #[error("`{family}` declares `{case}` twice")]
    DuplicateCase {
        /// The second declaration.
        at: Origin,
        /// The first, so the report can point at both.
        previous: Origin,
        /// The family declaring it.
        family: Name,
        /// The name declared twice.
        case: Name,
    },
    /// Two replacements in one update where one path is a prefix of the other.
    ///
    /// Refused rather than ordered, because both orders are defensible and
    /// neither is what the author meant: `p with { read = x, read.refusal = e }`
    /// either replaces `read` and then edits the replacement, or edits the old
    /// `read` and then discards it. An update whose meaning depends on which is
    /// a rule nobody should have to remember.
    #[error("this replacement's path is covered by another in the same update")]
    OverlappingUpdate {
        /// The later replacement.
        at: Origin,
        /// The one whose path is a prefix of it, or which it is a prefix of.
        previous: Origin,
    },
    /// A constructor written bare where nothing said which type it builds.
    ///
    /// `01-surface.md` §1.3 admits the bare form in a *checking* position, where
    /// the expected type names the family whose namespace the name is read in.
    /// In an inferring position there is no such type, and choosing among the
    /// families that happen to declare the name would make a program mean
    /// whatever was declared last.
    #[error("`{name}` is a constructor, and nothing here says of which type")]
    BareConstructor {
        /// The name.
        at: Origin,
        /// What was written.
        name: Name,
        /// The families that declare a case of this name, so the report can
        /// offer the qualified form the author meant.
        families: Vec<Name>,
    },
    /// A term stood in type position whose own type is not a universe.
    #[error("this stands where a type is needed, but it is not one")]
    NotAType {
        /// The term.
        at: Origin,
        /// The type it turned out to have.
        ty: Term,
    },
    /// A constructor field mentions a family the declaration is declaring,
    /// somewhere §1.1's strict positivity does not allow.
    ///
    /// The occurrence is named rather than the constructor alone, because the
    /// edit an author makes is at the occurrence: the field is usually right and
    /// one argument of it is wrong.
    #[error("`{family}` occurs in `{constructor}` where a recursive occurrence is not allowed")]
    NonPositive {
        /// The offending occurrence.
        at: Origin,
        /// The family that occurs.
        family: Name,
        /// The constructor whose field it occurs in.
        constructor: Name,
    },
    /// A constructor chose a different number of index arguments than the family
    /// it belongs to declares.
    #[error("this constructor chooses {found} index arguments, but the family declares {expected}")]
    IndexCount {
        /// The constructor.
        at: Origin,
        /// How many indices the family declares.
        expected: usize,
        /// How many the constructor wrote.
        found: usize,
    },
    /// A pattern named something that is not a constructor of the type the
    /// subject it stands against has.
    ///
    /// One refusal rather than two, because "this type has no constructors at
    /// all" and "this type has constructors and not that one" are the same
    /// sentence to the author: the name they wrote does not build this.
    #[error("`{name}` is not one of this type's constructors")]
    NoSuchConstructor {
        /// The pattern, or the qualified name.
        at: Origin,
        /// The name it wrote.
        name: Name,
        /// The type the name was looked up in: the subject's type at a pattern,
        /// the family itself at a qualified name.
        ty: Term,
        /// The constructors that type does declare, so the report can offer them
        /// rather than only deny the one asked for.
        cases: Vec<Name>,
    },
    /// A `match` left a constructor with no arm, at a subject the index
    /// constraints leave reachable (§6.2).
    #[error("this match has no arm for `{constructor}`")]
    IncompleteMatch {
        /// The match.
        at: Origin,
        /// A constructor no arm covers. One rather than all of them, because
        /// coverage is decided while the tree is built and the first gap is
        /// where the author's model went wrong.
        constructor: Name,
    },
    /// An arm no case the tree reaches can ever select.
    #[error("no value reaches this arm; an earlier one already covers it")]
    UnreachableBranch {
        /// The arm.
        at: Origin,
    },
    /// A subject whose type fixes an index to something other than a variable.
    ///
    /// The rule that would refine it is `02-core-calculus.md` §1.4's deletion,
    /// which requires K and is therefore not in this checker until a program
    /// needs it. The index is named because the edit is at the *subject's type*
    /// — generalize the index and match on it too — and not at the match.
    #[error("this match's subject fixes an index, which needs a unification rule this checker does not have")]
    ForcedIndex {
        /// The subject.
        at: Origin,
        /// The index argument that is not a variable.
        index: Term,
    },
    /// A recursive call the structural measure could not see decrease (§2.4).
    #[error("`{name}` calls itself on something this checker cannot see decrease")]
    UncheckedRecursion {
        /// The call.
        at: Origin,
        /// The definition being defined.
        name: Name,
    },
    /// An introduction form stood where a type had to be synthesized.
    ///
    /// §2's discipline in one variant: introduction forms *check*, so a record
    /// literal — or, in the re-checker, a λ, whose core form carries no domain
    /// (§1) — has no type of its own to report. The fix an author makes is to
    /// write one.
    #[error("this cannot be given a type on its own; write the type it should have")]
    Uninferable {
        /// The term.
        at: Origin,
    },
    /// A `trait` declaration using a name this crate generates instances for.
    ///
    /// `Storable` is the one trait `02-core-calculus.md` §1.2 reserves, and
    /// reserving the *word* rather than only the declaration is what closes the
    /// spelling: an author who declares their own `Storable` would otherwise
    /// shadow the generated instances with hand-written ones and the kernel
    /// payload boundary would have a hole in it.
    #[error("`{class}` is generated for every storable type and cannot be declared")]
    ReservedClass {
        /// The declaration.
        at: Origin,
        /// The name it used.
        class: Name,
    },
    /// A `trait` with no parameters.
    ///
    /// `10-traits.md` §1: the first parameter is the head, and lookup is keyed
    /// on it. A trait with none has nothing to key on, so it could only ever be
    /// a global value wearing a trait's syntax.
    #[error("`{class}` has no parameters, so nothing can be an instance of it")]
    HeadlessClass {
        /// The declaration.
        at: Origin,
        /// Its name.
        class: Name,
    },
    /// A **required** method carrying a `where` clause of its own.
    ///
    /// `10-traits.md` §1 makes a required method a *field* of the dictionary,
    /// and a field is filled by the impl that writes the instance. A constraint
    /// in its type would have to be discharged by somebody, and neither
    /// candidate works: the impl never wrote the field's type, and a use site
    /// resolving it would be a second lookup hidden inside a projection. A
    /// derived method has the `where` clause because it is a *function*, which
    /// is the half of §1's split that can take a dictionary.
    #[error("`{class}.{method}` is required, and a required method cannot carry a `where` clause")]
    ConstrainedField {
        /// The method.
        at: Origin,
        /// The trait.
        class: Name,
        /// Its name.
        method: Name,
    },
    /// A trait declaring one method name twice.
    #[error("`{class}` declares `{method}` twice")]
    DuplicateMethod {
        /// The second declaration.
        at: Origin,
        /// The first.
        previous: Origin,
        /// The trait.
        class: Name,
        /// The method.
        method: Name,
    },
    /// A trait or instance applied to the wrong number of arguments.
    #[error("`{class}` takes {wanted} argument(s), and {written} were written")]
    ClassArity {
        /// Where it was written.
        at: Origin,
        /// The trait.
        class: Name,
        /// How many parameters it has.
        wanted: usize,
        /// How many arguments were written.
        written: usize,
    },
    /// A source `impl Storable`, behind any spelling.
    ///
    /// Its own variant rather than [`Self::ReservedClass`] because it is the
    /// refusal §3 calls the single exception in the system, and the sentence an
    /// author needs is about *why* they cannot write it rather than about the
    /// name being taken.
    #[error("`Storable` instances are generated from the declaration and are never written")]
    HandWrittenStorable {
        /// The declaration.
        at: Origin,
    },
    /// An `impl` whose head argument is a bare type variable.
    ///
    /// `10-traits.md` §9's blanket-instance refusal. A head that is a variable
    /// matches everything, so it is not a key — admitting one would make every
    /// lookup that missed fall back to it, which is the search §4 forbids.
    #[error("`{class}` is implemented here for every type, which would make lookup a search")]
    BlanketInstance {
        /// The declaration.
        at: Origin,
        /// The trait.
        class: Name,
    },
    /// A second `impl` answering a key another already answers.
    ///
    /// §2's coherence, reported at the *second* declaration and naming the
    /// first, because that is the pair an author has to choose between.
    #[error("`{class}` is already implemented for `{head}`")]
    DuplicateInstance {
        /// The second declaration.
        at: Origin,
        /// The first.
        previous: Origin,
        /// The trait.
        class: Name,
        /// The head type both answer for.
        head: Name,
    },
    /// An `impl` in a package that declares neither its trait nor its head type.
    #[error("`{class}` for `{head}` belongs in the package that declares one of them")]
    OrphanInstance {
        /// The declaration.
        at: Origin,
        /// The trait.
        class: Name,
        /// The head type.
        head: Name,
    },
    /// An instance whose context §4's measure cannot see decrease.
    ///
    /// Checked at the declaration and never at a use, because a use site is the
    /// wrong place to learn that a library cannot answer: the author reading the
    /// message is not the author who can fix it.
    #[error("resolving `{class}` here need not terminate, because {reason}")]
    UnboundedInstance {
        /// The constraint.
        at: Origin,
        /// The trait it constrains.
        class: Name,
        /// Which half of the measure failed.
        reason: &'static str,
    },
    /// An `impl` supplying a method the trait derives.
    ///
    /// §1: a derived method is written once, in the trait. An impl that could
    /// replace one would make two dictionaries for the same instance
    /// distinguishable, which is what coherence exists to prevent.
    #[error("`{class}` derives `{method}`, so an instance does not define it")]
    DerivedMethod {
        /// The definition.
        at: Origin,
        /// The trait.
        class: Name,
        /// The method.
        method: Name,
    },
    /// An `impl` supplying a method the trait does not declare.
    #[error("`{class}` has no method `{method}`")]
    NoSuchMethod {
        /// The definition.
        at: Origin,
        /// The trait.
        class: Name,
        /// The name written.
        method: Name,
        /// The methods it does have, in declaration order.
        methods: Vec<Name>,
    },
    /// An `impl` leaving a required method undefined.
    #[error("this instance of `{class}` does not define `{method}`")]
    MissingMethod {
        /// The declaration.
        at: Origin,
        /// The trait.
        class: Name,
        /// The method not defined.
        method: Name,
    },
    /// A constraint no instance and no enclosing `where` answers.
    ///
    /// The one refusal here that an ordinary author sees often, so it says both
    /// halves of the key: an author fixes it either by writing the instance or
    /// by adding the constraint to the signature they are inside.
    #[error("nothing implements `{class}` for `{head}`")]
    UnresolvedInstance {
        /// The use.
        at: Origin,
        /// The trait.
        class: Name,
        /// The head its first argument has, as written.
        head: Name,
    },
    /// A constraint on a type variable that no enclosing `where` supplies.
    ///
    /// Told apart from [`Self::UnresolvedInstance`] because the fix is
    /// different and the checker knows which one it is: a variable can never
    /// acquire a global instance, so the only repair is a constraint on the
    /// signature.
    #[error("`{class}` is not available for this type variable; constrain it with `where {class}`")]
    UnconstrainedVariable {
        /// The use.
        at: Origin,
        /// The trait.
        class: Name,
    },
    /// `x.m(…)` where `x`'s type is not a declared type constructor.
    ///
    /// §6: a value of a generic parameter `A` never acquires a method, because
    /// finding one would mean scanning every trait in scope and adding a trait
    /// to a package would change what existing code means. The same is true of
    /// a receiver whose type is a function type, a universe, or a record type
    /// written out, and of one still unknown here — none of them is a name a
    /// lookup could be keyed on.
    ///
    /// Qualification is the repair and always available (§6): `Add.add(x, y)`
    /// says which trait, so it needs no receiver type to say it, and under a
    /// `where` it resolves exactly as an operator does.
    #[error("`.{method}` needs a receiver whose type is a declared type; write `Trait.{method}(…)` instead")]
    MethodOnVariable {
        /// The use.
        at: Origin,
        /// The method, as written.
        method: Name,
    },
    /// `x.m(…)` where no trait with a dictionary at `x`'s head declares `m`.
    ///
    /// §6's "none is an error naming the type and the method". Told apart from
    /// [`Self::UnresolvedInstance`] because no trait was named: the question was
    /// which one, and the answer was that none of the candidates is in scope for
    /// this head.
    #[error("no method `{method}` for `{head}`")]
    NoMethodForType {
        /// The use.
        at: Origin,
        /// The head of the receiver's type.
        head: Name,
        /// The method, as written.
        method: Name,
    },
    /// `x.m(…)` where two traits with a dictionary at `x`'s head declare `m`.
    ///
    /// §6's "two is an error naming both". Coherence cannot rule this out: it
    /// keeps one instance per (trait, head) pair, and two *different* traits at
    /// one head are exactly what a package that imports two libraries has.
    #[error("`{method}` for `{head}` is declared by more than one trait")]
    AmbiguousMethod {
        /// The use.
        at: Origin,
        /// The head of the receiver's type.
        head: Name,
        /// The method, as written.
        method: Name,
        /// The traits that declare it, sorted.
        classes: Vec<Name>,
    },
    /// A constraint on a type no instance could ever be keyed on.
    ///
    /// §4 postpones a constraint whose head is not known *yet*; a function
    /// type, a universe, or a record type is as known as it will ever be, and
    /// none of them is a declared name a key can hold. Told apart from
    /// [`Self::UnresolvedInstance`] because there is no instance anyone could
    /// write to repair it — which is exactly what `02-core-calculus.md` §1.2
    /// says about `Storable` and an arrow.
    #[error("`{class}` cannot be implemented for this type: only a declared type has instances")]
    UnkeyedConstraint {
        /// The use.
        at: Origin,
        /// The trait.
        class: Name,
    },
    /// One name registered twice in a [`Registry`](crate::Registry).
    ///
    /// §5.8's law suite checks that "every builtin is classified exactly once",
    /// and a name registered twice is the way that fails at the table rather
    /// than at a family: two entries under one spelling means every use site
    /// silently gets whichever one was inserted last.
    #[error("`{name}` is registered more than once")]
    DuplicateExtern {
        /// The name registered twice.
        name: Name,
        /// Where the second registration's type was written.
        at: Origin,
    },
    /// A δ-builtin whose signature holds an arrow.
    ///
    /// D1: "every argument type and the result type is a base type or a finite
    /// constructor over base types, with no arrow anywhere in the signature." A
    /// higher-order δ-builtin would be an operation the compiler owns and cannot
    /// reason about, which is the one thing the family is defined to exclude —
    /// and §5.8's theorem is stated over first-order operations only.
    #[error("δ-builtin `{name}` takes or returns a function")]
    HigherOrderDelta {
        /// The builtin.
        name: Name,
        /// Where its declared type was written.
        at: Origin,
    },
    /// A δ signature naming a base type the registry does not have.
    ///
    /// D1's inertness is true of a base type *because it is registered as one*
    /// — nothing eliminates what the registry holds. A signature over an
    /// unregistered base type is a claim about a type nobody declared inert.
    #[error("base type `{name}` is not registered")]
    UnknownBase {
        /// The base type as named in the signature.
        name: Name,
        /// Where it was named.
        at: Origin,
    },
    /// A pattern that takes a base type apart.
    ///
    /// D1's other half: "the only pattern that may match [a closed value of a
    /// base type] is a literal or a catch-all". A constructor or record pattern
    /// at a base type asks for structure the type does not have — and admitting
    /// one would be admitting an eliminator, which is what inertness is the
    /// absence of.
    #[error("a value of base type `{base}` has no structure to match on")]
    BaseNotMatchable {
        /// The base type.
        base: Name,
        /// Where the pattern was written.
        at: Origin,
    },
}

/// Two types that could not be made equal, and where the disagreement is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mismatch {
    /// The term whose type was being checked.
    pub at: Origin,
    /// The smallest subterm of the expected type that disagreed.
    pub expected: Term,
    /// The smallest subterm of the type actually found that disagreed.
    pub found: Term,
    /// The route from the two whole types to that pair, outermost step first.
    pub path: Vec<PathStep>,
}

impl fmt::Display for Mismatch {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.path.is_empty() {
            out.write_str("type mismatch")
        } else {
            out.write_str("type mismatch at ")?;
            for (position, step) in self.path.iter().enumerate() {
                if position > 0 {
                    out.write_str(", ")?;
                }
                write!(out, "{step}")?;
            }
            Ok(())
        }
    }
}

/// One step from the root of a type toward the subterm a mismatch is at.
///
/// Named after the *rôle* the subterm plays rather than after the enum variant
/// it sits in, because the reader has a type in front of them and not an AST.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PathStep {
    /// The argument type of a function type.
    Domain,
    /// The result type of a function type.
    Codomain,
    /// Under a λ.
    Body,
    /// The function of an application.
    Function,
    /// The argument of an application.
    Argument,
    /// A named field of a record type or literal.
    Field(Name),
    /// The record a projection is taken from.
    Projected,
    /// The type an identity is at.
    IdType,
    /// An identity's left endpoint.
    IdLeft,
    /// An identity's right endpoint.
    IdRight,
    /// The value `refl` witnesses.
    Witness,
}

impl fmt::Display for PathStep {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Domain => out.write_str("the argument type"),
            Self::Codomain => out.write_str("the result type"),
            Self::Body => out.write_str("the body"),
            Self::Function => out.write_str("the function"),
            Self::Argument => out.write_str("the argument"),
            Self::Field(field) => write!(out, "field `{field}`"),
            Self::Projected => out.write_str("the projected record"),
            Self::IdType => out.write_str("the type the identity is at"),
            Self::IdLeft => out.write_str("the identity's left side"),
            Self::IdRight => out.write_str("the identity's right side"),
            Self::Witness => out.write_str("the witnessed value"),
        }
    }
}
