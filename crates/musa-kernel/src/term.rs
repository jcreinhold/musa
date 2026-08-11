//! The term calculus (docs/kernel/10-term-calculus.md): a syntax whose
//! meanings are the timelines this crate already builds.
//!
//! Six forms and a reference. Nothing here adds a meaning: every term denotes
//! a timeline `03-denotational-semantics.md` defines, and [`evaluate`] hands
//! off to [`sequence`], [`overlay`], [`Timeline::scale`] and
//! [`Timeline::restrict`] rather than reimplementing them. What the calculus
//! buys is what values cannot express — **sharing** (`let`, so a canon's
//! subject is stated once), **deferred observation**, and **interchange**.
//!
//! The type is opaque and built through constructors, which is what makes the
//! static rules of `02-static-semantics.md` K7 mostly unrepresentable rather
//! than diagnosed: a non-positive `scale` factor, a negative `shift`, a
//! disordered window, and an empty `seq` cannot be constructed. What remains
//! is the two rules that are not local to one node — a free name and a
//! shadowed one — and [`Term::check`] answers those.

use num_rational::Ratio;

use crate::error::KernelError;
use crate::occurrence::Occurrence;
use crate::time::{Beat, Span};
use crate::timeline::{Timeline, overlay, sequence, timeline, zero};

/// A term of the kernel calculus.
///
/// Opaque on purpose: a public enum is a public layout, and callers need to
/// *build* terms, not to match on them. Accessors arrive with the consumer
/// that needs them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Term<A> {
    form: Form<A>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Form<A> {
    /// A literal timeline (E-Timeline).
    Literal(Timeline<A>),
    /// Temporal succession (E-Seq).
    Seq(Vec<Term<A>>),
    /// Simultaneous presence (E-Over).
    Over(Vec<Term<A>>),
    /// Sugar for `seq (timeline by {}) body`, expanded here rather than
    /// denoted separately (10-term-calculus, "`shift` is sugar").
    Shift { by: Beat, body: Box<Term<A>> },
    /// Time scaling (E-Scale).
    Scale { by: Ratio<i64>, body: Box<Term<A>> },
    /// Observation (E-Restrict).
    Restrict { window: Span, body: Box<Term<A>> },
    /// Sharing (E-Let); `name` scopes over `body`.
    Let {
        name: String,
        value: Box<Term<A>>,
        body: Box<Term<A>>,
    },
    /// A reference (E-Var), optionally marked (E-Mark).
    Var { name: String, mark: Option<String> },
}

impl<A> Term<A> {
    /// A literal timeline. Its occurrences were bounds-checked when the
    /// timeline was built (K1), so there is nothing left to reject.
    pub fn literal(value: Timeline<A>) -> Self {
        Self {
            form: Form::Literal(value),
        }
    }

    /// The timeline this term is, taken by value, or the term back when it is
    /// not a literal.
    ///
    /// For a producer that builds bottom-up: a run of adjacent literals in a
    /// `seq` says nothing a single literal does not, and coalescing it keeps
    /// the printed term to the structure that was actually written. Taking by
    /// value rather than by reference is the whole point — folding a run by
    /// cloning each literal out costs a full copy of every occurrence in it,
    /// which on material with no sharing to gain is pure loss.
    ///
    /// Deliberately this narrow: `Form` stays private, and a consumer that
    /// wants to walk a term wants an interpreter, not a getter.
    ///
    /// # Errors
    ///
    /// The term itself, unchanged, when it is not a literal.
    pub fn into_literal(self) -> Result<Timeline<A>, Self> {
        match self.form {
            Form::Literal(value) => Ok(value),
            form @ (Form::Seq(_)
            | Form::Over(_)
            | Form::Shift { .. }
            | Form::Scale { .. }
            | Form::Restrict { .. }
            | Form::Let { .. }
            | Form::Var { .. }) => Err(Self { form }),
        }
    }

    /// The ambient extent this term denotes, read off its syntax.
    ///
    /// The extent is compositional (`03-denotational-semantics.md`): a
    /// literal carries its own, `seq` adds, `over` takes the maximum, `shift`
    /// translates, `scale` scales, `restrict` keeps the body's — observation
    /// never shortens the work it looks at — and `let` is the extent of its
    /// body under the binding.
    ///
    /// Reading it rather than evaluating is what an *assembler* needs: a
    /// caller building a piece one statement at a time asks where the next
    /// one starts, and evaluating to find out would evaluate the whole term
    /// once per statement. Marks cannot make it wrong: instantiation may
    /// rewrite payloads but never times (`11-realization.md`).
    ///
    /// # Errors
    ///
    /// [`KernelError::FreeName`] for a reference nothing binds — an extent
    /// needs the binding, and the same rule [`Term::check`] states.
    pub fn extent(&self) -> Result<Beat, KernelError> {
        let mut environment: Vec<(&str, Beat)> = Vec::new();
        self.extent_in(&mut environment)
    }

    fn extent_in<'a>(&'a self, environment: &mut Vec<(&'a str, Beat)>) -> Result<Beat, KernelError> {
        let extent = match &self.form {
            Form::Literal(value) => value.extent(),
            Form::Seq(parts) => {
                let mut total = Beat::ZERO;
                for part in parts {
                    total = total.plus(part.extent_in(environment)?);
                }
                total
            }
            Form::Over(parts) => {
                let mut longest = Beat::ZERO;
                for part in parts {
                    longest = longest.max(part.extent_in(environment)?);
                }
                longest
            }
            Form::Shift { by, body } => by.plus(body.extent_in(environment)?),
            Form::Scale { by, body } => body.extent_in(environment)?.times(*by),
            Form::Restrict { body, .. } => body.extent_in(environment)?,
            Form::Let { name, value, body } => {
                let bound = value.extent_in(environment)?;
                environment.push((name, bound));
                let extent = body.extent_in(environment);
                environment.pop();
                extent?
            }
            Form::Var { name, .. } => environment
                .iter()
                .rev()
                .find_map(|(bound, extent)| (bound == name).then_some(*extent))
                .ok_or_else(|| KernelError::FreeName { name: name.clone() })?,
        };
        Ok(extent)
    }

    /// How many occurrences this term evaluates to, at most.
    ///
    /// A host that shares material has to decide whether evaluating a term is
    /// affordable *before* evaluating it, and the answer is structural: `seq`
    /// and `over` add, the moving forms pass through, and a name contributes
    /// the count of what binds it — so material named once and referenced
    /// four times is counted four times, which is what evaluating it costs.
    ///
    /// An upper bound rather than a count because [`Term::restrict`] can only
    /// remove: a window is not read here, since reading one means comparing
    /// spans, and a bound that has to evaluate the term is not a bound.
    ///
    /// Saturating: a term whose count would not fit reports [`u64::MAX`],
    /// which every caller of a bound already treats as "too much".
    pub fn occurrence_bound(&self) -> u64 {
        let mut environment: Vec<(&str, u64)> = Vec::new();
        self.bound_in(&mut environment)
    }

    fn bound_in<'a>(&'a self, environment: &mut Vec<(&'a str, u64)>) -> u64 {
        match &self.form {
            Form::Literal(value) => u64::try_from(value.occurrences().len()).unwrap_or(u64::MAX),
            Form::Seq(parts) | Form::Over(parts) => parts
                .iter()
                .fold(0, |total, part| total.saturating_add(part.bound_in(environment))),
            Form::Shift { body, .. } | Form::Scale { body, .. } | Form::Restrict { body, .. } => {
                body.bound_in(environment)
            }
            Form::Let { name, value, body } => {
                let bound = value.bound_in(environment);
                environment.push((name, bound));
                let count = body.bound_in(environment);
                environment.pop();
                count
            }
            // A free name binds nothing, so it contributes nothing. Rejecting
            // one is `check`'s job; a bound is total on purpose.
            Form::Var { name, .. } => environment
                .iter()
                .rev()
                .find_map(|(bound, count)| (bound == name).then_some(*count))
                .unwrap_or(0),
        }
    }

    /// Where `name` is referenced, in this term's own time.
    ///
    /// The *quotation locus* of `docs/language/01-surface.md` §7: the position
    /// a hole sits at, which is what a host must know to instantiate the
    /// material it splices in at the right place. It follows the same
    /// compositional reading as [`Term::extent`] — `seq` adds the exact
    /// extents of everything before it, `over` leaves it alone, `shift`
    /// translates it, a positive `scale` scales the relative offset,
    /// `restrict` relocates nothing, and a `let` value begins where its `let`
    /// does.
    ///
    /// `None` when nothing references `name`. The *first* reference wins:
    /// a caller that needs one answer per site gives each site its own name,
    /// which is what hole hygiene does anyway.
    pub fn locus(&self, name: &str) -> Option<Beat> {
        self.locus_in(name, Beat::ZERO)
    }

    fn locus_in(&self, name: &str, base: Beat) -> Option<Beat> {
        match &self.form {
            Form::Literal(_) => None,
            Form::Seq(parts) => {
                let mut at = base;
                for part in parts {
                    if let Some(found) = part.locus_in(name, at) {
                        return Some(found);
                    }
                    at = at.plus(part.extent().ok()?);
                }
                None
            }
            Form::Over(parts) => parts.iter().find_map(|part| part.locus_in(name, base)),
            Form::Shift { by, body } => body.locus_in(name, base.plus(*by)),
            // The body's own time is scaled, so a locus inside it is a
            // *relative* offset scaled and then placed.
            Form::Scale { by, body } => body.locus_in(name, Beat::ZERO).map(|inner| base.plus(inner.times(*by))),
            Form::Restrict { body, .. } => body.locus_in(name, base),
            Form::Let {
                name: bound,
                value,
                body,
            } => {
                if let Some(found) = value.locus_in(name, base) {
                    return Some(found);
                }
                // A shadowed name is not this one any more.
                (bound != name).then(|| body.locus_in(name, base)).flatten()
            }
            Form::Var { name: found, .. } => (found == name).then_some(base),
        }
    }

    /// Visit every payload written literally in this term, in source order.
    ///
    /// A term's payloads are opaque to the kernel (§12) but not to whoever
    /// wrote them: a host that *quotes* a term has to read them back to say
    /// whether they mean anything in the position the quote sits in. Nothing
    /// here interprets one — the visitor does, which is the whole point of
    /// handing them over rather than judging them.
    pub fn for_each_payload(&self, visit: &mut impl FnMut(&A)) {
        match &self.form {
            Form::Literal(value) => {
                for occurrence in value.occurrences() {
                    visit(occurrence.payload());
                }
            }
            Form::Seq(parts) | Form::Over(parts) => {
                for part in parts {
                    part.for_each_payload(visit);
                }
            }
            Form::Shift { body, .. } | Form::Scale { body, .. } | Form::Restrict { body, .. } => {
                body.for_each_payload(visit);
            }
            Form::Let { value, body, .. } => {
                value.for_each_payload(visit);
                body.for_each_payload(visit);
            }
            Form::Var { .. } => {}
        }
    }

    /// Rewrite every payload written literally in this term.
    ///
    /// The provenance half of [`Term::for_each_payload`]: a host that splices
    /// a term into a larger one records that fact *in the payloads*, because
    /// provenance rides inside payloads and nowhere else (§20). Times,
    /// counts, and order are untouched — this is a map over payloads, not
    /// over occurrences.
    pub fn map_payloads(&mut self, rewrite: &mut impl FnMut(&mut A)) {
        match &mut self.form {
            Form::Literal(value) => {
                for payload in value.payloads_mut() {
                    rewrite(payload);
                }
            }
            Form::Seq(parts) | Form::Over(parts) => {
                for part in parts {
                    part.map_payloads(rewrite);
                }
            }
            Form::Shift { body, .. } | Form::Scale { body, .. } | Form::Restrict { body, .. } => {
                body.map_payloads(rewrite);
            }
            Form::Let { value, body, .. } => {
                value.map_payloads(rewrite);
                body.map_payloads(rewrite);
            }
            Form::Var { .. } => {}
        }
    }

    /// Whether any reference in this term names `name`.
    ///
    /// For a producer that binds speculatively — elaboration binds a motif
    /// body before it knows whether the level above will need it as a value —
    /// this is how an unreferenced binding is dropped instead of printed. It
    /// answers about the whole term, shadowing included, which is the safe
    /// direction: a `let` that is kept is never wrong, only verbose.
    #[must_use]
    pub fn references_name(&self, name: &str) -> bool {
        match &self.form {
            Form::Literal(_) => false,
            Form::Var { name: bound, .. } => bound == name,
            Form::Seq(parts) | Form::Over(parts) => parts.iter().any(|part| part.references_name(name)),
            Form::Shift { body, .. } | Form::Scale { body, .. } | Form::Restrict { body, .. } => {
                body.references_name(name)
            }
            Form::Let { value, body, .. } => value.references_name(name) || body.references_name(name),
        }
    }

    /// Temporal succession of one or more terms (D2).
    ///
    /// Zero arguments are rejected rather than given a unit: the units of
    /// `seq` and `over` differ (L2/L6), so the empty case is written as the
    /// literal it is instead of inferred from context (K7, "Arity").
    ///
    /// # Errors
    ///
    /// [`KernelError::EmptyComposition`] when `parts` is empty.
    pub fn seq(parts: Vec<Self>) -> Result<Self, KernelError> {
        if parts.is_empty() {
            return Err(KernelError::EmptyComposition { form: "seq" });
        }
        Ok(Self { form: Form::Seq(parts) })
    }

    /// Simultaneous presence of one or more terms (D3).
    ///
    /// # Errors
    ///
    /// [`KernelError::EmptyComposition`] when `parts` is empty.
    pub fn over(parts: Vec<Self>) -> Result<Self, KernelError> {
        if parts.is_empty() {
            return Err(KernelError::EmptyComposition { form: "over" });
        }
        Ok(Self {
            form: Form::Over(parts),
        })
    }

    /// `body` delayed by `by ≥ 0` beats — sugar for a sequence after an empty
    /// timeline (D8).
    ///
    /// # Errors
    ///
    /// [`KernelError::InvalidSpan`] when `by` is negative: a delay runs
    /// forwards.
    pub fn shift(by: Beat, body: Self) -> Result<Self, KernelError> {
        if by < Beat::ZERO {
            return Err(KernelError::InvalidSpan {
                start: Beat::ZERO,
                end: by,
            });
        }
        Ok(Self {
            form: Form::Shift {
                by,
                body: Box::new(body),
            },
        })
    }

    /// `body` scaled by a positive rational factor (D5).
    ///
    /// # Errors
    ///
    /// [`KernelError::NonPositiveScale`] when `by ≤ 0` (K2).
    pub fn scale(by: Ratio<i64>, body: Self) -> Result<Self, KernelError> {
        if by <= Ratio::ZERO {
            return Err(KernelError::NonPositiveScale { factor: by.to_string() });
        }
        Ok(Self {
            form: Form::Scale {
                by,
                body: Box::new(body),
            },
        })
    }

    /// `body` observed through `window` (D6).
    ///
    /// No error case: `Span` is ordered by construction, and a window past the
    /// extent observes nothing rather than failing — observation is total (L17).
    pub fn restrict(window: Span, body: Self) -> Self {
        Self {
            form: Form::Restrict {
                window,
                body: Box::new(body),
            },
        }
    }

    /// `let name = value in body`: `name` denotes `value` throughout `body`.
    ///
    /// Sharing changes cost, never meaning (T2), so this is always safe to
    /// introduce and always safe to expand.
    pub fn bind(name: impl Into<String>, value: Self, body: Self) -> Self {
        Self {
            form: Form::Let {
                name: name.into(),
                value: Box::new(value),
                body: Box::new(body),
            },
        }
    }

    /// The term's form, for the in-crate printer (`text.rs`). Not public:
    /// a public enum is a public layout.
    pub(crate) fn form(&self) -> &Form<A> {
        &self.form
    }

    /// A reference to a `let`-bound name.
    ///
    /// Whether the name is actually bound is not knowable here — the binder
    /// is built around the reference, not before it — which is why
    /// [`Term::check`] exists at all.
    pub fn var(name: impl Into<String>) -> Self {
        Self {
            form: Form::Var {
                name: name.into(),
                mark: None,
            },
        }
    }

    /// A reference that records how *this* use of the shared body differs
    /// (`10-term-calculus.md` T6, E-Mark).
    ///
    /// The mark is an opaque string. The kernel never reads it: it hands it to
    /// the `instantiate` function given to [`evaluate_marked`], which chooses a
    /// payload map from it. That map may change payloads and nothing else —
    /// spans, extent, occurrence count and order are D7's to preserve (L9–L12),
    /// which is what makes T6 hold and what stops this from being `map f`
    /// smuggled in as a term.
    ///
    /// It exists because sharing and provenance pull opposite ways: a `repeat`
    /// elaborated once cannot have its occurrences each carry a different
    /// iteration index, so the *reference* carries it instead.
    pub fn var_marked(name: impl Into<String>, mark: impl Into<String>) -> Self {
        Self {
            form: Form::Var {
                name: name.into(),
                mark: Some(mark.into()),
            },
        }
    }

    /// The first well-formedness violation, or `Ok(())`
    /// (`02-static-semantics.md` K7).
    ///
    /// Only two rules reach here; the rest are unrepresentable (see the module
    /// docs). Both are about names, and neither is local to one node:
    ///
    /// - **no free names** — a term means one thing on its own or it does not
    ///   mean anything, so a free name is rejected rather than resolved
    ///   against an ambient environment;
    /// - **no shadowing** — nothing needs it, and forbidding it makes
    ///   substitution textual, which is what lets T2 be stated without a
    ///   capture-avoidance apparatus.
    ///
    /// A term that passes evaluates (T4). Callers that build terms from text
    /// call this before evaluating; callers that build them programmatically
    /// through the constructors above still can, and should.
    ///
    /// # Errors
    ///
    /// [`KernelError::FreeName`] or [`KernelError::ShadowedName`], whichever
    /// the traversal meets first.
    pub fn check(&self) -> Result<(), KernelError> {
        let mut bound: Vec<&str> = Vec::new();
        self.check_in(&mut bound)
    }

    fn check_in<'a>(&'a self, bound: &mut Vec<&'a str>) -> Result<(), KernelError> {
        match &self.form {
            Form::Literal(_) => Ok(()),
            Form::Seq(parts) | Form::Over(parts) => parts.iter().try_for_each(|part| part.check_in(bound)),
            Form::Shift { body, .. } | Form::Scale { body, .. } | Form::Restrict { body, .. } => body.check_in(bound),
            Form::Let { name, value, body } => {
                value.check_in(bound)?;
                if bound.contains(&name.as_str()) {
                    return Err(KernelError::ShadowedName { name: name.clone() });
                }
                bound.push(name.as_str());
                let result = body.check_in(bound);
                bound.pop();
                result
            }
            Form::Var { name, .. } => {
                if bound.contains(&name.as_str()) {
                    Ok(())
                } else {
                    Err(KernelError::FreeName { name: name.clone() })
                }
            }
        }
    }
}

/// Evaluate a term to its denotation (10-term-calculus, E-rules).
///
/// Total and deterministic (T4): the rules are syntax-directed, there is no
/// recursion to diverge through, and every constructor takes finitely many
/// finite arguments. It therefore returns a `Timeline` rather than a `Result`
/// — every way a term can be ill-formed is either unrepresentable or caught
/// by [`Term::check`].
///
/// The one case that could not be pushed into construction is a **free name**,
/// because a reference is built before the binder that encloses it. An
/// unchecked term containing one evaluates it to the empty timeline `(0, ∅)`
/// — the denotation of "nothing here" — rather than panicking in a library.
/// Call [`Term::check`] first and the case cannot arise; a debug build asserts
/// that you did.
///
/// `let` binds an **evaluated value**, evaluated once (E-Let is call-by-value).
/// That is the whole point of sharing: a subject used four times is evaluated
/// once. References currently *clone* the bound value rather than sharing it
/// behind an `Rc`; only a caller producing terms with heavy reuse would
/// justify changing that.
pub fn evaluate<A: Clone>(term: Term<A>) -> Timeline<A> {
    evaluate_marked(term, |_, _| {})
}

/// Evaluate a term, honouring the marks on its references (E-Mark, T6).
///
/// The term is taken **by value**: a literal's occurrences move into the
/// result rather than being copied. For a producer whose term holds the whole
/// piece exactly once — which is what elaboration builds when there is nothing
/// to share — borrowing would double the allocations of every compilation. A
/// caller that needs the term afterwards clones it and says so.
///
/// `instantiate` is the `φ` of E-Mark: it is handed a reference's mark and the
/// freshly instantiated copy of the bound value, and may rewrite that copy's
/// **payloads**. Once per reference site, not once per occurrence, so a caller
/// that has to decode the mark decodes it once.
///
/// The contract it owes, and the whole of what T6 rests on: it must not change
/// how many occurrences there are, where they sit, or what order they are in.
/// The kernel cannot enforce that — a `&mut Timeline` is the only signature
/// that lets a caller rewrite payloads in place instead of rebuilding — so it
/// is stated here and checked by `a_mark_changes_payloads_and_nothing_else`.
/// [`Timeline::map_payload`] is the safe way to honour it.
///
/// [`evaluate`] is this with the identity, which is why a consumer that has no
/// marks never sees this function.
pub fn evaluate_marked<A: Clone>(term: Term<A>, mut instantiate: impl FnMut(&str, &mut Timeline<A>)) -> Timeline<A> {
    debug_assert!(term.check().is_ok(), "evaluate expects a checked term (K7)");
    let mut environment: Vec<(String, Timeline<A>)> = Vec::new();
    eval(term, &mut environment, &mut instantiate)
}

fn eval<A: Clone>(
    term: Term<A>,
    environment: &mut Vec<(String, Timeline<A>)>,
    instantiate: &mut impl FnMut(&str, &mut Timeline<A>),
) -> Timeline<A> {
    match term.form {
        Form::Literal(value) => value,
        Form::Seq(parts) => sequence(
            parts
                .into_iter()
                .map(|part| eval(part, environment, instantiate))
                .collect(),
        ),
        Form::Over(parts) => overlay(
            parts
                .into_iter()
                .map(|part| eval(part, environment, instantiate))
                .collect(),
        ),
        Form::Shift { by, body } => {
            // The stated expansion, applied rather than denoted separately:
            // `shift d t = seq (timeline d {}) t` (10-term-calculus).
            let silence = timeline(by, Vec::new()).unwrap_or_else(|_| zero());
            sequence(vec![silence, eval(*body, environment, instantiate)])
        }
        Form::Scale { by, body } => {
            let value = eval(*body, environment, instantiate);
            // The factor was checked positive at construction, so `scale`
            // cannot reject it; the fallback keeps this total rather than
            // asserting an invariant twice.
            value.scale(by).unwrap_or(value)
        }
        Form::Restrict { window, body } => materialize(&eval(*body, environment, instantiate), window),
        Form::Let { name, value, body } => {
            let bound = eval(*value, environment, instantiate);
            environment.push((name, bound));
            let result = eval(*body, environment, instantiate);
            environment.pop();
            result
        }
        Form::Var { name, mark } => {
            let mut instance = environment
                .iter()
                .rev()
                .find(|(bound, _)| *bound == name)
                .map_or_else(zero, |(_, value)| value.clone());
            if let Some(mark) = mark {
                instantiate(&mark, &mut instance);
            }
            instance
        }
    }
}

/// D6's observation as a value: the occurrences `window` shows, each keeping
/// its **whole** span, in a timeline of the observed extent.
///
/// Restriction never rewrites where an occurrence began (§17), in a term any
/// more than in a value, so this materializes whole spans rather than visible
/// ones. The extent is the observed timeline's, which is what makes
/// `restrict` at the full extent the identity (L16).
fn materialize<A: Clone>(value: &Timeline<A>, window: Span) -> Timeline<A> {
    let observation = value.restrict(window);
    let occurrences: Vec<Occurrence<A>> = observation
        .observed()
        .map(|(_, occurrence)| occurrence.clone())
        .collect();
    timeline(value.extent(), occurrences).unwrap_or_else(|_| zero())
}
