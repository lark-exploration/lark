#![feature(in_band_lifetimes)]

use indexmap::IndexMap;
use indexmap::IndexSet;
use rustc_hash::FxHasher;
use std::hash::BuildHasherDefault;

pub type FxIndexMap<K, V> = IndexMap<K, V, BuildHasherDefault<FxHasher>>;
pub type FxIndexSet<K> = IndexSet<K, BuildHasherDefault<FxHasher>>;
pub use indexmap::Equivalent;
