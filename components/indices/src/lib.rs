#![feature(macro_at_most_once_rep)]

use std::fmt::Debug;
use std::hash::Hash;

/// Use to declare a "newtype'd" index that can be used with `IndexVec`.
/// The simplest usage is just:
///
/// ```ignore
/// index_type! {
///     $v struct Foo { .. }
/// }
/// ```
///
/// where `$v` is whatever visibility you want (`pub`, `crate`, etc).
///
/// Instances offer several methods and also implement a number of convenient
/// traits:
///
/// - `Foo::new(22)` -- make a new `Foo` from a `usize` (works in constants, too)
/// - `Foo::from_u32(22)` -- make a `Foo` from a `u32` (works in constants, too)
/// - `foo.as_u32()` -- extract the inner value as a `u32`
/// - `foo.as_usize()` -- extract the inner value as a `usize`
/// - `Foo: From<usize>` -- you can also use the `From` trait to construct from a usize
/// - `Foo: From<u32>` -- ...or a u32
///
/// Index types also implement the usual suspects (Copy, Clone, Debug, PartialOrd,
/// Ord, PartialEq, Eq, Hash) so they can be used with maps and things.
///
/// ### Storage
///
/// Internally, index types use a `NonZeroU32` for storage. This means that they
/// cannot exceed 2^32 - 1 in value (the code will assert this at runtime). It also means
/// that `Option<Foo>` is just one `u32` in size.
///
/// ### Configuring the newtype
///
/// Before the `..` you can also add some configuration options. Example:
///
/// ```ignore
/// index_type! {
///     struct Foo {
///         debug_name[Bar],
///         .. // <-- NB always end with `..`
///     }
/// }
/// ```
///
/// The options are:
///
/// - `debug_name[$expr]` -- change how the `Debug` impl prints out. This should
///   be a string expression, like `"XXX"`. We will print `"XXX"(N)` where `N`
///   is the index. If you put just `debug_name[]`, then we will not emit any
///   `Debug` impl at all, and you can provide your own.
#[macro_export]
macro_rules! index_type {
    ($(#[$attr:meta])* $visibility:vis struct $name:ident { $($tokens:tt)* }) => {
        $crate::index_type! {
            @with {
                attrs[$($attr)*],
                visibility[$visibility],
                name[$name],
                max[std::u32::MAX - 1],
                debug_name[stringify!($name)],
            }
            @tokens {
                $($tokens)*
            }
        }
    };

    // Base case: no more options
    (
        @with {
            attrs[$($attrs:meta)*],
            visibility[$visibility:vis],
            name[$name:ident],
            max[$max:expr],
            debug_name[$($debug_name:expr)?],
        }
        @tokens {..}
    ) => {
        $crate::index_type! {
            @with {
                attrs[$($attrs)*],
                visibility[$visibility],
                name[$name],
                max[$max],
                debug_name[$($debug_name)?],
            }
        }
    };

    // Consume a `debug_name[]` option
    (
        @with {
            attrs[$($attrs:meta)*],
            visibility[$visibility:vis],
            name[$name:ident],
            max[$max:expr],
            debug_name[$debug_name1:expr],
        }
        @tokens {
            debug_name[$($debug_name2:expr)?],
            $($tokens:tt)*
        }
    ) => {
        $crate::index_type! {
            @with {
                attrs[$($attrs)*],
                visibility[$visibility],
                name[$name],
                max[$max],
                debug_name[$($debug_name2)?],
            }
            @tokens {
                $($tokens)*
            }
        }
    };

    // Generate the type definition and various impls
    (@with {
        attrs[$($attrs:meta)*],
        visibility[$visibility:vis],
        name[$name:ident],
        max[$max:expr],
        debug_name[$($debug_name:expr)?],
    }) => {
        #[derive(Copy, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
        $(#[$attrs])*
        $visibility struct $name {
            private: std::num::NonZeroU32
        }

        impl $name {
            #[inline]
            $visibility const fn new(index: usize) -> Self {
                // This is a wacky assert that is compatible with a
                // const fn.  It will evaluate to an out-of-bounds
                // access if `index >= $max`.
                let v: u32 = [index as u32][(index >= ($max as usize)) as usize];
                unsafe { Self { private: std::num::NonZeroU32::new_unchecked(v + 1) } }
            }

            #[inline]
            $visibility const fn from_u32(index: u32) -> Self {
                // This is a wacky assert that is compatible with a
                // const fn.  It will evaluate to an out-of-bounds
                // access if `index >= $max`.
                let v: u32 = [index][(index >= $max) as usize];
                unsafe { Self { private: std::num::NonZeroU32::new_unchecked(v + 1) } }
            }

            #[inline]
            $visibility fn as_u32(self) -> u32 {
                self.private.get() - 1
            }

            #[inline]
            $visibility fn as_usize(self) -> usize {
                self.as_u32() as usize
            }
        }

        impl From<usize> for $name {
            #[inline]
            fn from(v: usize) -> $name {
                $name::new(v)
            }
        }

        impl From<u32> for $name {
            #[inline]
            fn from(v: u32) -> $name {
                $name::from_u32(v)
            }
        }

        impl std::ops::Add<usize> for $name {
            type Output = $name;

            #[inline]
            fn add(self, v: usize) -> $name {
                $name::new(self.as_usize() + v)
            }
        }

        impl $crate::U32Index for $name {
            #[inline]
            fn as_u32(self) -> u32 {
                self.as_u32()
            }

            #[inline]
            fn from_u32(v: u32) -> Self {
                Self::from_u32(v)
            }

            #[inline]
            fn as_usize(self) -> usize {
                self.as_usize()
            }

            #[inline]
            fn from_usize(v: usize) -> Self {
                Self::new(v)
            }
        }

        $crate::index_type! {
            @debug_impl {
                name[$name],
                debug_name[$($debug_name)?]
            }
        }
    };

    // User requested no debug impl
    (
        @debug_impl {
            name[$name:ident],
            debug_name[]
        }
    ) => {
    };

    // Generate `Debug` impl as user requested
    (
        @debug_impl {
            name[$name:ident],
            debug_name[$debug_name:expr]
        }
    ) => {
        impl std::fmt::Debug for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.debug_tuple($debug_name)
                    .field(&self.as_usize())
                    .finish()
            }
        }
    };
}

pub trait U32Index: Copy + Debug + Eq + Hash + 'static {
    fn as_usize(self) -> usize;

    fn from_usize(v: usize) -> Self;

    fn as_u32(self) -> u32;

    fn from_u32(v: u32) -> Self;
}

/// A vector indexable via values of type `I`.
#[derive(Clone, Eq, PartialEq, Hash)]
pub struct IndexVec<I, T>
where
    I: U32Index,
{
    vec: Vec<T>,
    _marker: std::marker::PhantomData<I>,
}

impl<I, T> IndexVec<I, T>
where
    I: U32Index,
{
    pub fn new() -> Self {
        Default::default()
    }

    pub fn with_capacity(cap: usize) -> Self {
        IndexVec {
            vec: Vec::with_capacity(cap),
            _marker: std::marker::PhantomData,
        }
    }

    pub fn from_elem<S>(elem: T, universe: &IndexVec<I, S>) -> Self
    where
        T: Clone,
    {
        IndexVec {
            vec: vec![elem; universe.len()],
            _marker: std::marker::PhantomData,
        }
    }

    pub fn from_elem_n(elem: T, n: usize) -> Self
    where
        T: Clone,
    {
        IndexVec {
            vec: vec![elem; n],
            _marker: std::marker::PhantomData,
        }
    }

    pub fn len(&self) -> usize {
        self.vec.len()
    }

    pub fn is_empty(&self) -> bool {
        self.vec.is_empty()
    }

    pub fn into_iter(self) -> std::vec::IntoIter<T> {
        self.vec.into_iter()
    }

    pub fn into_iter_enumerated(self) -> impl Iterator<Item = (I, T)> {
        self.vec
            .into_iter()
            .enumerate()
            .map(|(i, v)| (I::from_usize(i), v))
    }

    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.vec.iter()
    }

    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, T> {
        self.vec.iter_mut()
    }

    pub fn iter_enumerated(&self) -> impl Iterator<Item = (I, &T)> {
        self.vec
            .iter()
            .enumerate()
            .map(|(i, v)| (I::from_usize(i), v))
    }

    pub fn iter_enumerated_mut(&mut self) -> impl Iterator<Item = (I, &mut T)> {
        self.vec
            .iter_mut()
            .enumerate()
            .map(|(i, v)| (I::from_usize(i), v))
    }
    pub fn indices(&self) -> impl Iterator<Item = I> {
        (0..self.len()).map(|v| I::from_usize(v))
    }
    pub fn last_idx(&self) -> Option<I> {
        if self.is_empty() {
            None
        } else {
            Some(I::from_usize(self.len() - 1))
        }
    }
    pub fn next_idx(&self) -> I {
        I::from_usize(self.len())
    }
    pub fn shrink_to_fit(&mut self) {
        self.vec.shrink_to_fit()
    }
    pub fn swap(&mut self, l: I, r: I) {
        self.vec.swap(l.as_usize(), r.as_usize())
    }
    pub fn truncate(&mut self, s: usize) {
        self.vec.truncate(s)
    }
    pub fn get(&self, i: I) -> Option<&T> {
        self.vec.get(i.as_usize())
    }
    pub fn get_mut(&mut self, i: I) -> Option<&mut T> {
        self.vec.get_mut(i.as_usize())
    }

    pub fn last(&self) -> Option<&T> {
        self.vec.last()
    }
    pub fn last_mut(&mut self) -> Option<&mut T> {
        self.vec.last_mut()
    }

    pub fn reserve(&mut self, s: usize) {
        self.vec.reserve(s);
    }

    pub fn resize(&mut self, s: usize, v: T)
    where
        T: Clone,
    {
        self.vec.resize(s, v);
    }
    pub fn binary_search(&self, v: &T) -> Result<I, I>
    where
        T: Ord,
    {
        self.vec
            .binary_search(v)
            .map(I::from_usize)
            .map_err(I::from_usize)
    }
    pub fn push(&mut self, d: T) -> I {
        let idx = self.next_idx();
        self.vec.push(d);
        idx
    }

    pub fn push_with_idx<F>(&mut self, f: F) -> I
    where
        F: FnOnce(I) -> T,
    {
        let idx = self.next_idx();
        let d = f(idx);
        self.vec.push(d);
        idx
    }
}

impl<I, T> Default for IndexVec<I, T>
where
    I: U32Index,
{
    fn default() -> Self {
        IndexVec::from(vec![])
    }
}

impl<I, T> From<Vec<T>> for IndexVec<I, T>
where
    I: U32Index,
{
    fn from(vec: Vec<T>) -> Self {
        IndexVec {
            vec,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<I, T> std::ops::Index<I> for IndexVec<I, T>
where
    I: U32Index,
{
    type Output = T;
    fn index(&self, i: I) -> &T {
        &self.vec[i.as_usize()]
    }
}

impl<I, T> std::ops::IndexMut<I> for IndexVec<I, T>
where
    I: U32Index,
{
    fn index_mut(&mut self, i: I) -> &mut T {
        &mut self.vec[i.as_usize()]
    }
}

impl<I, T> std::iter::Extend<T> for IndexVec<I, T>
where
    I: U32Index,
{
    fn extend<IT>(&mut self, iter: IT)
    where
        IT: IntoIterator<Item = T>,
    {
        self.vec.extend(iter)
    }
}

impl<I, T> std::iter::FromIterator<T> for IndexVec<I, T>
where
    I: U32Index,
{
    fn from_iter<IT>(iter: IT) -> Self
    where
        IT: IntoIterator<Item = T>,
    {
        IndexVec {
            vec: iter.into_iter().collect(),
            _marker: std::marker::PhantomData,
        }
    }
}

impl<I, T> IntoIterator for IndexVec<I, T>
where
    I: U32Index,
{
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;

    fn into_iter(self) -> std::vec::IntoIter<T> {
        self.into_iter()
    }
}

impl<'a, I, T> IntoIterator for &'a IndexVec<I, T>
where
    I: U32Index,
{
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
impl<'a, I, T> IntoIterator for &'a mut IndexVec<I, T>
where
    I: U32Index,
{
    type Item = &'a mut T;
    type IntoIter = std::slice::IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

impl<I, T> std::fmt::Debug for IndexVec<I, T>
where
    I: U32Index,
    T: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self.vec, f)
    }
}
