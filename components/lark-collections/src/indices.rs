use lark_debug_with::DebugWith;
use std::fmt::Debug;
use std::hash::Hash;

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

impl<I, T> DebugWith for IndexVec<I, T>
where
    I: U32Index,
    T: DebugWith,
{
    fn fmt_with<Cx: ?Sized>(&self, cx: &Cx, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt.debug_list()
            .entries(self.iter().map(|elem| elem.debug_with(cx)))
            .finish()
    }
}
