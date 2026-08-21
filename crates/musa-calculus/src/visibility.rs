//! Which module a declaration was written in, and whether it may be named from
//! outside it.
//!
//! `docs/rules/language/01-surface.md` §1.3 and
//! `docs/rules/language/04-templates-and-modules.md` §4: a declaration is
//! visible everywhere unless it is marked `private`, and a `private` case of an
//! enum keeps its type public while making its constructors module-local, so a
//! package can maintain an invariant its clients cannot break.
//!
//! # A module is a number here, and nothing else
//!
//! [`ModuleId`] is opaque in this crate for the same reason [`crate::Origin`]
//! is: `musa-calculus` is a leaf, it does not know what a file, a package, or an
//! `import` is, and it must not learn. What it carries is a number the caller
//! assigned, and the only thing it ever does with that number is compare it with
//! another one. There is no path, no nesting, no parent relation, and no module
//! *table* — the caller keeps whatever structure it has, and hands down the one
//! bit of it this rule needs.
//!
//! A context that names **no** module is inside every module. That is what keeps
//! the rule invisible to callers that have no packages: a test, or any of the
//! elaborations this crate ran before the marker existed, sees everything.
//!
//! # Why hiding is a refusal and not an absence
//!
//! The cheaper implementation is for the caller to build each module's context
//! without the private declarations in it at all, and it is the wrong one. A
//! name that is not there is [`crate::Refusal::UnknownName`], which sends a
//! reader looking for a typo. What they need to be told is that the name exists
//! and is maintained elsewhere — so resolution answers with [`Hidden`] rather
//! than failing to find anything, and the diagnostic can say which module to go
//! to.

/// The module a declaration was written in.
///
/// `Copy` and one word wide, like [`crate::Origin`], because a declaration
/// group holds one and a group that had to allocate to say where it came from
/// would make the marker a thing worth switching off.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ModuleId(u32);

impl ModuleId {
    /// The module the caller numbered `id`.
    ///
    /// Minted by the caller and never by this crate, which has no way to tell
    /// one module from another except by being told.
    #[must_use]
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    /// The number the caller assigned, so a diagnostic can be rendered against
    /// whatever the caller calls this module.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// Whether a declaration may be named outside the module that wrote it.
///
/// Public by default, and `private` is the explicit subtraction — the argument
/// is in `01-surface.md` §1.3, and it is about Musa's packages being
/// vocabularies rather than about which default a fresh language should pick.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Visibility {
    /// Nameable from anywhere. The default, and what an unmarked declaration is.
    #[default]
    Public,
    /// Nameable only from inside the module that declared it.
    Private,
}

impl Visibility {
    /// Whether a declaration this visible, written in `home`, is nameable by a
    /// context elaborating in `viewer`.
    ///
    /// Two ways to be visible and both are deliberate: the declaration is
    /// public, or the viewer is standing in the module that wrote it. A viewer
    /// with no module is inside every module, and a declaration with no module
    /// was written nowhere in particular and hides from nobody.
    pub(crate) fn visible_from(self, home: Option<ModuleId>, viewer: Option<ModuleId>) -> bool {
        match (self, home, viewer) {
            (Self::Public, _, _) | (Self::Private, None, _) | (Self::Private, Some(_), None) => true,
            (Self::Private, Some(home), Some(viewer)) => home == viewer,
        }
    }
}
