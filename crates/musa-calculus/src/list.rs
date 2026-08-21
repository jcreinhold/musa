//! An immutable list, shared by everything in this crate that is indexed from
//! the innermost binder outward.
//!
//! Three structures have that shape and would otherwise be the same thirty
//! lines written three times: the evaluation environment, the types a context
//! assumed its binders at, and the names elaboration reads a variable by.
//!
//! Persistent rather than a `Vec`, and the reason is the same in all three
//! places: a checker descends into two branches from one context and neither may
//! see the other's binders, so the alternative is a clone per binder and an
//! elaboration that costs O(n²) in the depth of the term it is reading. Here
//! extension is one allocation and no copy, and the old list stays usable.
//!
//! Lookup walks, which is the trade: it is O(index) rather than O(1). That is
//! the right side of the trade because a term reaches for the binder it just
//! introduced far more often than for one at the bottom of a deep context, and
//! because the walk is bounded by how many binders a term actually mentions
//! rather than by how many are in scope.

use std::sync::Arc;

/// A shared immutable list, innermost first.
pub(crate) struct List<T>(Option<Arc<Cell<T>>>);

struct Cell<T> {
    head: T,
    rest: List<T>,
}

/// Cloning shares rather than copies, and does not require `T: Clone`.
///
/// Derived `Clone` would demand it, because the derive puts the bound on the
/// type parameter rather than asking what the fields need — and what the fields
/// need here is an [`Arc`] clone.
impl<T> Clone for List<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> Default for List<T> {
    fn default() -> Self {
        Self::EMPTY
    }
}

impl<T> List<T> {
    /// The empty list.
    pub(crate) const EMPTY: Self = Self(None);

    /// This list with `head` innermost.
    pub(crate) fn push(&self, head: T) -> Self {
        Self(Some(Arc::new(Cell {
            head,
            rest: self.clone(),
        })))
    }

    /// The entry `index` steps in from the innermost, or `None` past the end.
    pub(crate) fn get(&self, index: u32) -> Option<&T> {
        let mut here = self;
        let mut remaining = index;
        loop {
            let cell = here.0.as_ref()?;
            if remaining == 0 {
                return Some(&cell.head);
            }
            remaining = remaining.checked_sub(1)?;
            here = &cell.rest;
        }
    }

    /// The entries, innermost first.
    pub(crate) fn iter(&self) -> Iter<'_, T> {
        Iter(self.0.as_deref())
    }
}

/// [`List::iter`]'s iterator.
pub(crate) struct Iter<'a, T>(Option<&'a Cell<T>>);

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<&'a T> {
        let cell = self.0?;
        self.0 = cell.rest.0.as_deref();
        Some(&cell.head)
    }
}
