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
            // The one lift. `error.rs` sits below this module and carries the
            // rule's sentence as a plain string; a refusal is what it becomes
            // the moment it reaches the vocabulary that has one.
            CoreError::Refused { message, at } => Self::Refused(Refusal::BuiltinRefused {
                message: Name::from(message),
                at,
            }),
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
        /// The names of the raiser's own namespace that *were* in scope.
        ///
        /// Handed over rather than re-collected by the caller, because the
        /// raiser is the one holding the scope ("hand a consumer what we
        /// already computed"): which names are near the written one is the
        /// surface's judgment to make, but what the candidates *are* is a
        /// fact of the context at the refusal point. Empty where the raiser
        /// has no such list — a trait name looked up among classes is a
        /// different namespace than a value among binders.
        candidates: Vec<Name>,
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
    /// A machine port's type holds a function or a type: `02-core-calculus.md`
    /// §1.2's storability is structural, and this type fails it.
    #[error("`{}` is not storable data: a function or a type occurs in it", crate::show::spelled(.ty))]
    NotStorable {
        /// The signature that required storability.
        at: Origin,
        /// The type that failed it.
        ty: Term,
    },
    /// Something that is not a function was applied.
    #[error("this is applied to an argument, but its type is not a function type")]
    NotAFunction {
        /// The application.
        at: Origin,
        /// The type the applied term turned out to have.
        ty: Term,
    },
    /// A written call that leaves a declared parameter unsupplied.
    ///
    /// `02-core-calculus.md` §1.3: "A call must be complete: an application
    /// supplies every declared parameter, and an under-applied call is a type
    /// error rather than a value." The other direction is [`Self::NotAFunction`],
    /// and they are told apart because the repairs are opposite ones — a
    /// function applied to too much was the wrong function, and a call with too
    /// little is the right one missing an argument.
    ///
    /// The parameters are *named*, and that is what makes this report worth
    /// more than a count. A reader who is told `lifted` takes two arguments has
    /// to go and look at `lifted` to find out which one they left out; a reader
    /// told that nothing is given for `by` is already at the edit. The names
    /// are the declaration's own, which is also the only place a parameter is
    /// declared at all — see
    /// [`declared_parameters`](crate::elab::infer::declared_parameters).
    #[error(
        "`{function}` takes {wanted} argument{}, and {written} {} written: nothing is given for {}",
        if *.wanted == 1 { "" } else { "s" },
        if *.written == 1 { "was" } else { "were" },
        crate::show::listed(.missing),
    )]
    Underapplied {
        /// The call.
        at: Origin,
        /// How the function was spelled, from the head of its own spine.
        function: String,
        /// How many arguments a complete call writes.
        wanted: usize,
        /// How many this one wrote.
        written: usize,
        /// The parameters no argument reached, in the order they were declared.
        /// Never empty.
        missing: Vec<Name>,
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
    /// A declaration would need a universe above `Type 1`, which the calculus
    /// does not have (§1: two universes, fixed).
    #[error("this declaration needs a universe above Type 1, and there are two")]
    BeyondUniverses {
        /// The declaration.
        at: Origin,
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
    ///
    /// The sentence states what *is* admitted before naming the fault, because
    /// there are several faults and one rule: an occurrence to the left of an
    /// arrow, at an index, at a parameter another family is not positive in, or
    /// inside a record field are four ways to fail the one sentence the message
    /// says. The arrow is named last because it is much the commonest, and it is
    /// named rather than left implied because "not allowed" alone was true of
    /// two rules until a nested occurrence became legal.
    #[error(
        "`{family}` occurs in `{constructor}` where a recursive occurrence is not allowed: it may be the field itself, or stand at a parameter of a family that is positive in it — never to the left of an arrow"
    )]
    NonPositive {
        /// The offending occurrence.
        at: Origin,
        /// The family that occurs.
        family: Name,
        /// The constructor whose field it occurs in.
        constructor: Name,
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
    /// Top-level definitions that name each other (§2.4).
    ///
    /// §2.4 admits recursion "through the measure rather than through the
    /// graph", and [`crate::rec`]'s measure is minted by a `match` inside one
    /// body: a mutually recursive pair has no induction hypothesis to become.
    /// So the graph rule is not a leftover from the old checker — it is exactly
    /// what the measure does not reach, and this is where a program lands that
    /// needs what neither has.
    #[error("these definitions name each other: {}", names.join(" → "))]
    DefinitionCycle {
        /// The definitions in the cycle, in the order they name each other.
        names: Vec<Name>,
        /// The definition whose reference closes it.
        at: Origin,
    },
    /// A top-level definition that names itself and wrote no type (§2.4).
    ///
    /// The measure is checked against the type the definition presents, and a
    /// definition that presents none has nothing to check it against — there is
    /// no type to infer it from either, because inferring the body is what
    /// needs the type. Writing the signature is the whole of the repair.
    #[error("`{name}` names itself, so it has to say what its type is")]
    UntypedRecursion {
        /// The definition.
        name: Name,
        /// Where it was written.
        at: Origin,
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
    /// A trait declaring a `where` clause: a supertrait, which is synthesized
    /// dictionary search under a quieter spelling.
    ///
    /// `10-traits.md` §9's first row. What the author wanted is stated at the
    /// use instead: the method or function takes the second dictionary as an
    /// argument, or states it in its own `where`, and the trait stays a record
    /// of exactly its methods.
    #[error("`{class}` declares a super-constraint; take the dictionary as an argument instead")]
    SuperClass {
        /// The clause.
        at: Origin,
        /// The trait.
        class: Name,
    },
    /// An `impl` declaring a `where` clause.
    ///
    /// `10-traits.md` §4: resolution is three steps and no recursion, and an
    /// instance whose own constraints had to be synthesized is the recursion.
    /// The explicit spelling is an ordinary function from the dictionaries to
    /// the dictionary — `fn list_eq<A>(eq: Eq<A>) -> Eq<List<A>>` — or a macro
    /// that writes the concrete instances out.
    #[error("`impl {class}` carries a `where` clause; write the dictionary-building function instead")]
    ConstrainedInstance {
        /// The clause.
        at: Origin,
        /// The trait.
        class: Name,
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
    #[error("`{class}` cannot be implemented for `{}`: only a declared type has instances", crate::show::spelled(.ty))]
    UnkeyedConstraint {
        /// The use.
        at: Origin,
        /// The trait.
        class: Name,
        /// The first argument, which is the type no key could hold.
        ///
        /// Said rather than described, for [`Mismatch`]'s reason: a machine port
        /// that turned out to be `Ratio → Ratio` is a sentence a reader can act
        /// on, and "this type" is one they have to go and reconstruct.
        ty: Term,
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
    /// A structural eliminator whose target is not one of its own arguments.
    ///
    /// §5.8's second family fires on a declared argument — ι fires on the
    /// recursor's target, and a traversal fires on the value it traverses. An
    /// index past the end of the signature names nothing, so the rule could
    /// never fire and the builtin would be a name that reduces for no input.
    #[error("structural eliminator `{name}` fires on an argument it does not take")]
    TargetOutsideSignature {
        /// The builtin.
        name: Name,
        /// Where its declared type was written.
        at: Origin,
    },
    /// A structural eliminator whose target is not a base type.
    ///
    /// A traversal exists because a base type has no constructors and therefore
    /// no recursor. A rewrite standing at a *declared* family would be a second
    /// ι-rule for a type that already has one — two rules over one type, which
    /// is the second path §5's determinism argument rules out.
    #[error("structural eliminator `{name}` fires on an argument that is not a base type")]
    TargetNotABase {
        /// The builtin.
        name: Name,
        /// Where the target argument's type was written.
        at: Origin,
    },
    /// A number written at a family that does not count.
    ///
    /// A numeral is a representation of a *counting* family — no parameters, no
    /// indices, two cases, one a floor and one a step over itself — so a reader
    /// that wrote `384` at anything else asked for a value that has no such
    /// spelling. The reason is carried rather than left to the reader to work
    /// out, because the four conditions fail for four different edits: drop a
    /// parameter, drop an index, change the cases, change a field.
    ///
    /// The host's mistake and not the author's: `RawShape::Numeral` names the
    /// family, and the reader is what chose it. So the sentence addresses
    /// whoever wrote the reading, which is why it says what the family is rather
    /// than what to write instead.
    #[error("`{name}` is not a type a number can be written at: {reason}")]
    NotANumeralFamily {
        /// Where the number was written.
        at: Origin,
        /// The family the reader named.
        name: Name,
        /// Which condition of the counting rule it fails, first failure first.
        reason: &'static str,
    },
    /// A δ-builtin whose signature holds something that is not finite data.
    ///
    /// §5.8's D1 states the shape positively: every argument type and the result
    /// type is "a base type or a finite constructor over base types". A record
    /// type, a universe, or a bare type variable is none of those, and the
    /// consequence is concrete rather than aesthetic — a δ-rule reads and answers
    /// [`Datum`](crate::Datum), which can say a literal and a constructor
    /// application and nothing else, so a signature outside that shape declares
    /// an operation whose rule could never be written.
    ///
    /// An arrow gets [`Self::HigherOrderDelta`] instead, because it is the one a
    /// table author actually writes and it deserves the specific sentence.
    #[error("δ-builtin `{name}` has an argument or result type that is not finite data")]
    NotFiniteData {
        /// The builtin.
        name: Name,
        /// Where the offending type was written.
        at: Origin,
    },
    /// A δ-rule said no to the arguments the program gave it.
    ///
    /// The one refusal whose sentence this crate did not write. Every other
    /// variant above states a rule of the calculus and can therefore say what
    /// went wrong in the calculus's own words; this one is an operation of the
    /// *language* — a stretch factor, a chord's length, a division — rejecting a
    /// value, and only the rule knows why. [`Answer::Refused`](crate::Answer)
    /// carries the sentence out of the rule, and δ-reduction attaches the origin
    /// of the application it fired at, which is the node the composer wrote.
    ///
    /// It is a refusal rather than a [`Malformed`] and that is the whole point:
    /// before it existed a rule's only "no" was silence, which the evaluator
    /// reads as its own table being wrong, so operations that needed to reject a
    /// program answered `Result τ Text` and made every caller carry a failure it
    /// could not do anything about.
    #[error("{message}")]
    BuiltinRefused {
        /// What the rule said.
        message: Name,
        /// The application it fired at.
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
    /// The whole pair the comparison started from, when the comparison knows.
    ///
    /// The endpoints say *where* the types disagree; the roots say *what* was
    /// being asked, in the shape the author wrote it — "expected
    /// `Option<Pitch>`, found `Option<Degree>`" is a sentence about the
    /// program, and "expected `Pitch`, found `Degree` at the argument" is a
    /// sentence about the machine's walk. Unification's entry points attach
    /// the pair they were handed; a comparison that began mid-term (a
    /// retried constraint, the rechecker's two normal forms) leaves this
    /// `None`, and a renderer falls back to the endpoints, which there are
    /// the whole types.
    pub whole: Option<Box<(Term, Term)>>,
}

/// The route, and then the two subterms it ends at.
///
/// Both halves, because either alone leaves the reader to reconstruct the
/// other: a path with no types says a mismatch happened somewhere in an
/// argument, and two types with no path leaves a reader holding `Bool` and
/// `Ratio` and no idea which position of which type they came out of. The
/// spelling is [`crate::show`]'s, so it is the *core*'s vocabulary — a surface
/// `Machine<K, A, B>` reads back as an application, which is what the term is.
impl fmt::Display for Mismatch {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        out.write_str("type mismatch")?;
        if !self.path.is_empty() {
            out.write_str(" at ")?;
            for (position, step) in self.path.iter().enumerate() {
                if position > 0 {
                    out.write_str(", ")?;
                }
                write!(out, "{step}")?;
            }
        }
        write!(
            out,
            ": expected `{}`, found `{}`",
            crate::show::spelled(&self.expected),
            crate::show::spelled(&self.found)
        )
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
