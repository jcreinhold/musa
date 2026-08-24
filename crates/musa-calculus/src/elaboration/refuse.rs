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

use crate::kernel::budget::ResourceError;
use crate::kernel::error::{CoreError, Malformed};
use crate::kernel::meta::MetaSource;
use crate::kernel::origin::Origin;
use crate::kernel::term::{Name, Term};
use crate::kernel::visibility::ModuleId;

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

/// The one crossing.
///
/// Nothing under `kernel/` names this module, and `tests/suite/boundary_laws.rs`
/// enforces that, so a kernel outcome reaches an author through here or not at
/// all. Two of the three arms are carried rather than translated —
/// [`ElabError`] has an [`ElabError::Exhausted`] and an [`ElabError::Malformed`]
/// of its own because neither becomes a refusal on the way up: exhaustion is
/// §4's third outcome, and a malformed term is this compiler's defect, and
/// spelling either as a refusal would blame an author for it. The third arm is
/// the lift proper, where a δ-rule's sentence meets the vocabulary that has a
/// refusal for it.
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
///
/// # Which enum a case belongs to
///
/// [`CoreError`]'s own doc states the rule; this is its other half. A refusal
/// is addressed to someone who **wrote** something and is raised while the
/// thing they wrote is still in hand — an author writing `.musa`, and equally a
/// host author writing a registration, which is why
/// [`Registry::new`](crate::Registry::new)'s checks answer here and live in
/// [`admit`](crate::elaboration::admit) rather than in the kernel. A mistake
/// found after that, with nothing left to point at, is addressed to whoever
/// maintains this compiler and is a [`Malformed`].
///
/// Five of these are the elaborator's half of a claim the kernel also makes —
/// `NotAFunction`, `NotARecord`, `NoSuchField`, `NotAType` and `Uninferable`
/// — and [`Self::Mismatch`] stands beside
/// [`Malformed::Mistyped`] the same way. Each pair is two sentences for two
/// readers rather than one sentence written twice: this side names what the
/// author wrote and where, and the kernel's side names a term nobody should
/// have built.
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
        /// has no such list.
        candidates: Vec<Name>,
    },
    /// A `let` whose value names the binder the `let` is introducing, where
    /// nothing else in scope carries that name.
    ///
    /// Told apart from [`Self::UnknownName`] by the name alone: a `let` is
    /// non-recursive, so its value is read in the scope *outside* the binder,
    /// and a value that reaches for the name being bound is asking for a
    /// recursion the term does not have. Shadowing is untouched — where an
    /// outer binder of the same name is in scope the value means that one,
    /// which is the reading `let sofar = f(sofar);` wants and gets.
    #[error("`{name}` is being bound here, and a `let` cannot name itself")]
    RecursiveBinding {
        /// The name, as the binder spells it.
        name: Name,
        /// Where the value reached for it.
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
    /// A machine port's type holds a function or a type: `02-core-calculus.md`
    /// §1.2's storability is structural, and this type fails it.
    #[error("`{}` is not storable data: a function or a type occurs in it", crate::elaboration::show::spelled(.ty))]
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
    /// declared at all — see `elab::infer`'s `declared_parameters`.
    #[error(
        "`{function}` takes {wanted} argument{}, and {written} {} written: nothing is given for {}",
        if *.wanted == 1 { "" } else { "s" },
        if *.written == 1 { "was" } else { "were" },
        crate::elaboration::show::listed(.missing),
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
    FillingMismatch {
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
    /// A record literal names one family and stands where another is wanted.
    ///
    /// Two names for one value, and they disagree. Kept as its own refusal
    /// rather than left to conversion because the two families are what the
    /// author needs to see, and a conversion mismatch would show them applied
    /// to whatever parameters each stood at.
    #[error("this record literal names `{found}`, but `{expected}` is what stands here")]
    RecordHead {
        /// The written head.
        at: Origin,
        /// The family the expected type names.
        expected: Name,
        /// The family the literal names.
        found: Name,
    },
    /// A one-constructor family declares one field name twice.
    ///
    /// A telescope with two `f`s is not merely confusing where a projection is
    /// generated per field (§1.2): the two would generate two accessors of one
    /// name, and one of the fields would be one no projection can reach and no
    /// literal can decline to write. A family with several constructors
    /// generates nothing and is left to shadow, which is what a telescope does.
    #[error("this record declares `{field}` twice")]
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
    /// Two universes met whose levels no assignment can make equal.
    ///
    /// §1's hierarchy is non-cumulative, so `Type ℓ` and `Type ℓ'` agree only
    /// when the levels do. Distinct *closed* levels are an ordinary
    /// [`Self::Mismatch`] — `Type 0` and `Type 1` are two types, and the report
    /// that names them reads better than one about universes. This one is for
    /// the case a level *variable* is involved and there is still no solution:
    /// `u` against `u+1`, which fails the occurs check, or a variable already
    /// fixed elsewhere in the declaration. The author's edit is to write the
    /// definition polymorphically or to split it, and nothing about the term
    /// standing there says so.
    #[error("this needs universe level `{expected}`, and the level here is `{found}`")]
    LevelMismatch {
        /// Where the two met.
        at: Origin,
        /// The level the surrounding term required.
        expected: crate::kernel::sort::Sort,
        /// The level the term standing there is at.
        found: crate::kernel::sort::Sort,
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
    /// A constructor's result stands at a different number of indices than its
    /// family takes.
    ///
    /// Counted rather than inferred, and refused rather than padded. §1.1 makes
    /// the index list the one thing a constructor writes about its own result,
    /// so a missing index is a value the author has not chosen and there is
    /// nothing to choose it for them: `Nil : Vec<A>` under `data Vec<A> : (n :
    /// Nat) -> Type` is not `Nil : Vec<A>(0)` with the zero left implicit, it
    /// is a length nobody said.
    ///
    /// One refusal for both directions because the repair is the same sentence
    /// read either way — the result and the signature disagree, and the author
    /// looks at both.
    #[error("`{constructor}` stands at {written} of `{family}`'s {declared} indices")]
    IndexCount {
        /// Where the constructor was written.
        at: Origin,
        /// The family it belongs to.
        family: Name,
        /// The constructor itself.
        constructor: Name,
        /// How many indices its result wrote.
        written: usize,
        /// How many the family's signature declares.
        declared: usize,
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
    /// An alternation whose alternatives do not bind the same names (§6.2).
    ///
    /// Peyton Jones ch. 5's condition. The arm's body is one expression checked
    /// once per branch the alternation reaches, so the scope it is checked in
    /// cannot depend on which alternative matched: a name one alternative binds
    /// and another does not is a name the body may read and may not have.
    #[error("this alternative does not bind `{name}`, and another beside it does")]
    AlternativeBindings {
        /// The alternative that lacks the name.
        at: Origin,
        /// The name it lacks. One rather than all of them, for
        /// [`Self::IncompleteMatch`]'s reason.
        name: Name,
    },
    /// An arm no case the tree reaches can ever select.
    #[error("no value reaches this arm; an earlier one already covers it")]
    UnreachableBranch {
        /// The arm.
        at: Origin,
    },
    /// Top-level declarations that name each other (§2.4).
    ///
    /// §2.4 admits recursion through the structural rule rather than through
    /// the graph, and [`crate::elaboration::rec`]'s hypothesis is minted by a `match` inside
    /// one body: a mutually recursive pair has no induction hypothesis to
    /// become. So the graph rule is not a leftover from the old checker — it is
    /// exactly what the structural rule does not reach, and this is where a
    /// program lands that needs what neither has.
    ///
    /// **Declarations** rather than definitions, because a `data` group is on
    /// the graph too since prompt 162ba. A family has a mutual-recursion door of
    /// its own — §1.1 checks strict positivity on the whole group — but it opens
    /// for one `data` declaration with shared parameters, which two written
    /// declarations naming each other are not.
    #[error("these declarations name each other: {}", names.join(" → "))]
    DefinitionCycle {
        /// The declarations in the cycle, in the order they name each other.
        names: Vec<Name>,
        /// The declaration whose reference closes it.
        at: Origin,
    },
    /// A top-level definition that names itself and wrote no type (§2.4).
    ///
    /// The descent is checked against the type the definition presents, and a
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
    /// A recursive call the structural rule could not see descend (§2.4).
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
    /// `f({A = …}, …)` naming a type parameter `f` does not have.
    ///
    /// `01-surface.md` §1's `arg := … | "{" IDENT "=" expr "}"` supplies an
    /// inferred parameter *by the callee's binder name*, so a name that no
    /// inferred binder bears is a name the author believed the signature had.
    /// Reporting it needs both halves — the function, and the names it does
    /// bear — because the mistake is almost always a spelling or a signature
    /// the author is remembering from somewhere else.
    #[error(
        "`{function}` has no type parameter named `{name}`; it {}",
        if borne.is_empty() {
            "has none".to_owned()
        } else {
            format!("has {}", crate::elaboration::show::listed(.borne))
        },
    )]
    NoSuchParameter {
        /// Where the call was written.
        at: Origin,
        /// How the function was spelled, from the head of its own spine.
        function: String,
        /// The name the author wrote.
        name: Name,
        /// The inferred binders the walk met, in the order it met them.
        borne: Vec<Name>,
    },
    /// `x.m(…)` where `x`'s type is not a declared type constructor.
    ///
    /// §1.5: `x.m(…)` is `Head.m(x, …)` where `Head` is the rigid head of `x`'s
    /// type, so a receiver whose type has no rigid head names no definition. A
    /// generic parameter `A`, a function type, a universe, a record type written
    /// out, and a type still unknown here are all of them: none is a name a
    /// lookup could be keyed on.
    ///
    /// Qualification is the repair and always available: `Head::m(x, …)` writes
    /// the namespace out, so it needs no receiver type to say it.
    #[error("`.{method}` needs a receiver whose type is a declared type; write `Head::{method}(…)` instead")]
    MethodOnVariable {
        /// The use.
        at: Origin,
        /// The method, as written.
        method: Name,
    },
    /// `x.m(…)` where neither table has `m` for `x`'s head — no `Head.m` in
    /// scope, and no field of that name.
    ///
    /// §1.5's "none is an error naming the type and the method". This is an
    /// ordinary unresolved name wearing the spelling the author used: the term
    /// it would have elaborated to is a top-level definition, and the reason it
    /// is a refusal of its own is that reporting `Pitch.act` unresolved would
    /// name a spelling the author never wrote.
    ///
    /// The fields are carried because the rule reads two tables and a reader
    /// has to be able to see both misses. `Group` has fields `unit`, `compose`
    /// and `inverse`, and a report that named none of them would send someone
    /// who mistyped one looking in the wrong table.
    #[error("no method `{method}` for `{head}`")]
    NoMethodForType {
        /// The use.
        at: Origin,
        /// The head of the receiver's type.
        head: Name,
        /// The method, as written.
        method: Name,
        /// The receiver type's own field names, where it is a product, in
        /// declaration order; empty where it is not one.
        fields: Vec<Name>,
    },
    /// `x.m(…)` where `Head.m` is in scope *and* `m` is a field of `x`'s type.
    ///
    /// §1.5's two-table rule refusing its collision. The two candidates are not
    /// two definitions of one name — they are two forms, at different arities
    /// and with different first arguments — so nothing filters between them:
    /// choosing by the expected type would mean elaborating both and keeping
    /// whichever converted, which is the trial elaboration the method rule
    /// exists to not have.
    ///
    /// Both readings stay reachable, which is what makes refusing affordable:
    /// `Head::m(x, …)` names the definition and `(x.m)(…)` names the field, and
    /// each says which it means without asking anything of the other.
    #[error("`{method}` is both a definition in `{head}` and a field of `{head}`")]
    MemberAndField {
        /// The use.
        at: Origin,
        /// The head of the receiver's type.
        head: Name,
        /// The member, as written.
        method: Name,
    },
    /// A bare member spelling that the expected type did not narrow to one
    /// namespace.
    ///
    /// §1.5's "two is an error naming both", and the failure mode that replaced
    /// "no instance found". The candidates are listed because the repair is to
    /// write one of them: `Head::m` is always available and always unambiguous.
    /// An *empty* narrowing reports here too — the member exists and the
    /// expected type ruled every namespace out, which the reader needs to see
    /// with the same list.
    #[error("`{method}` is declared in more than one namespace")]
    AmbiguousMethod {
        /// The use.
        at: Origin,
        /// The head the site fixed, or the member itself where it fixed none.
        head: Name,
        /// The member, as written.
        method: Name,
        /// The namespaces that declare it, sorted.
        candidates: Vec<Name>,
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
    /// retried constraint, the re-checker's two normal forms) leaves this
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
/// spelling is [`crate::elaboration::show`]'s, so it is the *core*'s vocabulary — a surface
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
            crate::elaboration::show::spelled(&self.expected),
            crate::elaboration::show::spelled(&self.found)
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
        }
    }
}
