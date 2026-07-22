//! Bump-arena allocator for the Nori AST (oxc-style).
//!
//! Nodes allocated here share a single lifetime tied to the [`Allocator`].
//! Dropping the allocator frees everything at once — no per-node `Drop`.

use std::{fmt, hash::Hash, ops::Deref};

use bumpalo::Bump;

/// Owns the bump arena that backs an AST for a single compile.
#[derive(Default)]
pub struct Allocator {
    bump: Bump,
}

impl Allocator {
    #[inline]
    pub fn new() -> Self {
        Self { bump: Bump::new() }
    }

    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            bump: Bump::with_capacity(capacity),
        }
    }

    #[inline]
    pub fn alloc<T>(&self, value: T) -> &mut T {
        self.bump.alloc(value)
    }

    #[inline]
    pub fn alloc_str<'a>(&'a self, s: &str) -> &'a str {
        self.bump.alloc_str(s)
    }

    #[inline]
    pub fn alloc_slice_copy<'a, T: Copy>(&'a self, slice: &[T]) -> &'a [T] {
        self.bump.alloc_slice_copy(slice)
    }

    #[inline]
    pub fn box_in<'a, T>(&'a self, value: T) -> Box<'a, T> {
        Box::new_in(value, self)
    }

    #[inline]
    pub fn vec_in<'a, T>(&'a self) -> Vec<'a, T> {
        Vec::new_in(self.as_bump())
    }

    #[inline]
    pub fn vec_from_iter_in<'a, T, I>(&'a self, iter: I) -> Vec<'a, T>
    where
        I: IntoIterator<Item = T>,
    {
        let mut vec = Vec::new_in(self.as_bump());
        vec.extend(iter);
        vec
    }

    #[inline]
    pub fn atom<'a>(&'a self, s: &str) -> Atom<'a> {
        Atom(self.alloc_str(s))
    }

    #[inline]
    pub fn as_bump(&self) -> &Bump {
        &self.bump
    }
}

/// Arena-allocated owned pointer.
#[derive(Debug, PartialEq, Eq)]
pub struct Box<'a, T: ?Sized>(bumpalo::boxed::Box<'a, T>);

impl<'a, T> Box<'a, T> {
    #[inline]
    pub fn new_in(value: T, allocator: &'a Allocator) -> Self {
        Self(bumpalo::boxed::Box::new_in(value, allocator.as_bump()))
    }

    #[inline]
    pub fn into_inner(self) -> T {
        bumpalo::boxed::Box::into_inner(self.0)
    }
}

impl<T: ?Sized> Deref for Box<'_, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: ?Sized> std::ops::DerefMut for Box<'_, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T: ?Sized + fmt::Display> fmt::Display for Box<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Arena-allocated growable vector.
pub type Vec<'a, T> = bumpalo::collections::Vec<'a, T>;

/// Interned / borrowed string slice used for identifiers and atoms.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Atom<'a>(pub &'a str);

impl<'a> Atom<'a> {
    #[inline]
    pub const fn new(s: &'a str) -> Self {
        Self(s)
    }

    #[inline]
    pub const fn as_str(self) -> &'a str {
        self.0
    }
}

impl Deref for Atom<'_> {
    type Target = str;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl AsRef<str> for Atom<'_> {
    #[inline]
    fn as_ref(&self) -> &str {
        self.0
    }
}

impl fmt::Debug for Atom<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.0, f)
    }
}

impl fmt::Display for Atom<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self.0, f)
    }
}

impl PartialEq<str> for Atom<'_> {
    #[inline]
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}

impl PartialEq<&str> for Atom<'_> {
    #[inline]
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}

impl PartialEq<String> for Atom<'_> {
    #[inline]
    fn eq(&self, other: &String) -> bool {
        self.0 == other.as_str()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allocates_and_frees_with_arena() {
        let allocator = Allocator::new();
        let a = allocator.atom("hello");
        let b = allocator.box_in(42u32);
        assert_eq!(a.as_str(), "hello");
        assert_eq!(*b, 42);
        let mut v = allocator.vec_in::<u32>();
        v.push(1);
        v.push(2);
        assert_eq!(v.as_slice(), &[1, 2]);
    }
}
