#![feature(in_band_lifetimes)]

use lark_debug_with::DebugWith;
use std::fmt::Debug;
use std::hash::{Hash, Hasher};
use std::iter::{once, FromIterator, IntoIterator};
use std::ops::{Deref, DerefMut, Range};
use std::sync::Arc;

mod test;

pub struct Seq<T> {
    vec: Option<Arc<Vec<T>>>,
    start: usize,
    end: usize,
}

impl<T> Seq<T> {
    /// Modifies this restrict to a subset of its current range.
    pub fn select(&mut self, range: Range<usize>) {
        let len = range.end - range.start;
        let new_start = self.start + range.start;
        let new_end = new_start + len;
        assert!(new_end <= self.end);

        self.start = new_start;
        self.end = new_end;
    }

    /// Extract a new `Text` that is a subset of an old `Text`
    /// -- `text.extract(1..3)` is similar to `&foo[1..3]` except that
    /// it gives back an owned value instead of a borrowed value.
    pub fn extract(&self, range: Range<usize>) -> Self {
        let mut result = self.clone();
        result.select(range);
        result
    }

    /// Extend this `Seq`. Note that seqs from which this was cloned
    /// are not affected.
    pub fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = T>,
        T: Clone,
    {
        let mut iter = iter.into_iter();

        // Peel off the first item; if there is none, then just do nothing.
        if let Some(first) = iter.next() {
            // Create an iterator with that first item plus the rest
            let iter = once(first).chain(iter);

            // Try to extend in place:
            if let Some(vec) = &mut self.vec {
                if self.start == 0 && self.end == vec.len() {
                    Arc::make_mut(vec).extend(iter);
                    self.end = vec.len();
                    return;
                }
            }

            // If not, construct a new vector.
            let v: Vec<T> = self.iter().cloned().chain(iter).collect();
            let len = v.len();
            self.vec = Some(Arc::new(v));
            self.start = 0;
            self.end = len;
        }
    }
}

impl<T> Clone for Seq<T> {
    fn clone(&self) -> Self {
        Self {
            vec: self.vec.clone(),
            start: self.start,
            end: self.end,
        }
    }
}

impl<T> Default for Seq<T> {
    fn default() -> Self {
        Self {
            vec: None,
            start: 0,
            end: 0,
        }
    }
}

impl<T> From<Arc<Vec<T>>> for Seq<T> {
    fn from(vec: Arc<Vec<T>>) -> Self {
        if vec.is_empty() {
            Seq::default()
        } else {
            let len = vec.len();
            Seq {
                vec: Some(vec),
                start: 0,
                end: len,
            }
        }
    }
}

impl<T> From<Vec<T>> for Seq<T> {
    fn from(vec: Vec<T>) -> Self {
        if vec.is_empty() {
            Seq::default()
        } else {
            Self::from(Arc::new(vec))
        }
    }
}

impl<T: Clone> From<&[T]> for Seq<T> {
    fn from(text: &[T]) -> Self {
        let vec: Vec<T> = text.iter().cloned().collect();
        Self::from(vec)
    }
}

impl<T> Deref for Seq<T> {
    type Target = [T];

    fn deref(&self) -> &[T] {
        match &self.vec {
            None => &[],
            Some(vec) => &vec[self.start..self.end],
        }
    }
}

impl<T: Clone> DerefMut for Seq<T> {
    fn deref_mut(&mut self) -> &mut [T] {
        match &mut self.vec {
            None => &mut [],
            Some(vec) => &mut Arc::make_mut(vec)[self.start..self.end],
        }
    }
}

impl<T: Debug> std::fmt::Debug for Seq<T> {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        <[T] as std::fmt::Debug>::fmt(self, fmt)
    }
}

impl<T: DebugWith> DebugWith for Seq<T> {
    fn fmt_with<Cx: ?Sized>(&self, cx: &Cx, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        <[T] as DebugWith>::fmt_with(self, cx, fmt)
    }
}

impl<T: PartialEq> PartialEq<Seq<T>> for Seq<T> {
    fn eq(&self, other: &Seq<T>) -> bool {
        let this: &[T] = self;
        let other: &[T] = other;
        this == other
    }
}

impl<T: Eq> Eq for Seq<T> {}

impl<T: PartialEq> PartialEq<[T]> for Seq<T> {
    fn eq(&self, other: &[T]) -> bool {
        let this: &[T] = self;
        this == other
    }
}

impl<T: PartialEq> PartialEq<Vec<T>> for Seq<T> {
    fn eq(&self, other: &Vec<T>) -> bool {
        let this: &[T] = self;
        let other: &[T] = other;
        this == other
    }
}

impl<A: ?Sized, T: PartialEq> PartialEq<&A> for Seq<T>
where
    Seq<T>: PartialEq<A>,
{
    fn eq(&self, other: &&A) -> bool {
        self == *other
    }
}

impl<T> FromIterator<T> for Seq<T> {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = T>,
    {
        let vec: Vec<T> = iter.into_iter().collect();
        Seq::from(vec)
    }
}

impl<T> IntoIterator for &'seq Seq<T> {
    type IntoIter = <&'seq [T] as IntoIterator>::IntoIter;
    type Item = <&'seq [T] as IntoIterator>::Item;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<T: Clone> IntoIterator for &'seq mut Seq<T> {
    type IntoIter = <&'seq mut [T] as IntoIterator>::IntoIter;
    type Item = <&'seq mut [T] as IntoIterator>::Item;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

impl<T: Hash> Hash for Seq<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        <[T] as Hash>::hash(self, state)
    }
}

#[macro_export]
macro_rules! seq {
    () => {
        $crate::Seq::default()
    };

    ($($v:expr),* $(,)*) => {
        $crate::Seq::from(vec![$($v),*])
    };
}
