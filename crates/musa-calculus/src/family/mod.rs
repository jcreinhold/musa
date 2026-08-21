//! Inductive families: what a declaration group is, and the recursor it
//! generates.
//!
//! `docs/rules/language/02-core-calculus.md` §1.1 declares families with
//! **parameters**, fixed across the whole declaration, and **indices**, which a
//! constructor chooses:
//!
//! ```text
//! data Vec (A : Type l) : (n : Nat) → Type l {
//!     Nil  : Vec A zero,
//!     Cons : (n : Nat) → A → Vec A n → Vec A (succ n),
//! }
//! ```
//!
//! The difference is load-bearing rather than cosmetic: the generated recursor's
//! motive quantifies over the *indices* and not over the parameters, so blurring
//! them produces an eliminator that type-checks and proves nothing useful. The
//! rule is enforced by the representation instead of by a check — a constructor
//! does not write its result type at all, only the index arguments it chooses, so
//! "parameters appear uniformly in every constructor's result" cannot be violated
//! by anything a caller writes.
//!
//! # The declaration context, and why terms are read under it
//!
//! A constructor's field type mentions the family it is declaring, and a mutual
//! group's constructors mention each other. Storing that as a [`Constant`]
//! pointing back at the [`Group`] would make the group an [`Arc`] cycle — a leak,
//! and a construction order with no first step.
//!
//! So every [`Term`] a group stores is read under **the declaration context**:
//! `k` binders, one per family in the group, outermost first. A recursive
//! occurrence is then an ordinary variable, which is also what makes strict
//! positivity a question about *variables* rather than about a name table. The
//! constants appear only in terms handed to a caller, which the group does not
//! own, so the cycle cannot form.
//!
//! [`Group::declarations`] is that context as an environment of values, which is
//! how a stored term is read: evaluating in it replaces each declaration variable
//! with the constant it names.
//!
//! # What the recursor is, and why it is assembled rather than stored
//!
//! For family `N_i` of a group `N_1 … N_k` over shared parameters `p⃗`:
//!
//! ```text
//! elim_i : (p⃗ : Params)
//!        → (P_1 : (i⃗ : Indices_1) → N_1 p⃗ i⃗ → Type ℓ) → … → (P_k : …)
//!        → (methods, one per constructor of every family in the group)
//!        → (i⃗ : Indices_i) → (t : N_i p⃗ i⃗) → P_i i⃗ t
//! ```
//!
//! and the method for a constructor `c` of family `N_j` is
//!
//! ```text
//! m_c : (a⃗ : Fields_c) → (ih⃗) → P_j idx_c (c p⃗ a⃗)
//! ```
//!
//! with one induction hypothesis `P_{j'} i⃗' a_ℓ` per recursive field. Motives and
//! methods sit *between* the parameters and the fields, so a field type stored at
//! one depth appears in the method at another — and this crate has no
//! substitution function to shift it with (§3: "reduction is never performed on
//! syntax"). The types are therefore **assembled semantically**: each stored term
//! is evaluated in the environment it was written in and quoted at the depth it
//! now stands at, which is what weakening *is* in a levelled semantic domain.
//!
//! That is also why assembly is handed the motive's universe: `ℓ` is chosen per
//! use site rather than fixed at declaration time, so one family supports both
//! small and large elimination without the universe polymorphism §1.3 refuses.
//!
//! # Recursive fields are direct
//!
//! A field may be an arrow, and it may mention the family — but not both: `sup :
//! (Nat → W) → W` is refused, while `Cons : A → List A → List A` is not. §1.1
//! permits the infinitary constructor, so this is a real narrowing, and §1.2 is
//! what makes it free: "an arrow type is never storable, and neither is any
//! container holding one", so a family with an infinitary constructor could never
//! carry a payload, be a machine port, or cross the event track boundary. What it buys
//! is that an induction hypothesis is an application rather than a synthesized
//! closure, so ι never builds syntax.
//!
//! # By file
//!
//! - [`group`] — what a declaration group is: its families, constructors, and
//!   the binders they are declared with.
//! - [`constant`] — a name a group introduces (a family, a constructor, a
//!   recursor, a numeral), and the type it stands at.
//! - [`assemble`] — the telescope that builds those types semantically, which
//!   is where the recursor is generated.
//! - [`iota`] — ι-reduction: a recursor applied to a constructor steps to the
//!   method for it.
//! - [`datum`] — the bridge to canonical payloads: a term read as a datum, a
//!   datum realized as a value.

mod assemble;
mod constant;
mod datum;
mod group;
mod iota;

pub use constant::Constant;
pub(crate) use constant::{Found, Numeral};
pub use datum::canonical;
pub(crate) use datum::{counted, realize};
pub use group::{Binder, Constructor, Declared, Group};
pub(crate) use group::{Counting, Element, Role, element};
pub(crate) use iota::{constructed, iota, stepped};
