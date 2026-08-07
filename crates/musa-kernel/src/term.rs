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
/// Opaque on purpose: a public enum is a public layout, and the two callers
/// this exists for — prompt 48's text form and prompt 49's elaboration —
/// need to *build* terms, not to match on them. Accessors arrive with the
/// consumer that needs them.
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
    /// A reference (E-Var).
    Var(String),
}

impl<A> Term<A> {
    /// A literal timeline. Its occurrences were bounds-checked when the
    /// timeline was built (K1), so there is nothing left to reject.
    pub fn literal(value: Timeline<A>) -> Self {
        Self {
            form: Form::Literal(value),
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
    /// extent observes nothing rather than failing — observation is total
    /// (L17), and prompt 37 made it so deliberately.
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
            form: Form::Var(name.into()),
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
            Form::Var(name) => {
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
/// behind an `Rc`; prompt 49 is the measurement that would justify changing
/// that, since it is the first caller to produce terms with heavy reuse.
pub fn evaluate<A: Clone>(term: &Term<A>) -> Timeline<A> {
    debug_assert!(term.check().is_ok(), "evaluate expects a checked term (K7)");
    let mut environment: Vec<(&str, Timeline<A>)> = Vec::new();
    eval(term, &mut environment)
}

fn eval<'a, A: Clone>(term: &'a Term<A>, environment: &mut Vec<(&'a str, Timeline<A>)>) -> Timeline<A> {
    match &term.form {
        Form::Literal(value) => value.clone(),
        Form::Seq(parts) => sequence(parts.iter().map(|part| eval(part, environment)).collect()),
        Form::Over(parts) => overlay(parts.iter().map(|part| eval(part, environment)).collect()),
        Form::Shift { by, body } => {
            // The stated expansion, applied rather than denoted separately:
            // `shift d t = seq (timeline d {}) t` (10-term-calculus).
            let silence = timeline(*by, Vec::new()).unwrap_or_else(|_| zero());
            sequence(vec![silence, eval(body, environment)])
        }
        Form::Scale { by, body } => {
            let value = eval(body, environment);
            // The factor was checked positive at construction, so `scale`
            // cannot reject it; the fallback keeps this total rather than
            // asserting an invariant twice.
            value.scale(*by).unwrap_or(value)
        }
        Form::Restrict { window, body } => materialize(&eval(body, environment), *window),
        Form::Let { name, value, body } => {
            let bound = eval(value, environment);
            environment.push((name.as_str(), bound));
            let result = eval(body, environment);
            environment.pop();
            result
        }
        Form::Var(name) => environment
            .iter()
            .rev()
            .find(|(bound, _)| *bound == name.as_str())
            .map_or_else(zero, |(_, value)| value.clone()),
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
