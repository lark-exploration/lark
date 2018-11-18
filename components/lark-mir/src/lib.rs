#![feature(crate_visibility_modifier)]
#![feature(const_fn)]
#![feature(const_let)]
#![feature(decl_macro)]
#![feature(in_band_lifetimes)]
#![feature(macro_at_most_once_rep)]
#![feature(specialization)]

use indices::{IndexVec, U32Index};
use lark_debug_derive::DebugWith;
use lark_entity::Entity;
use lark_error::WithError;
use lark_string::global::GlobalIdentifier;
use lark_ty::declaration::DeclarationTables;
use lark_type_check as typecheck;
use parser::pos::{HasSpan, Span, Spanned};
use std::sync::Arc;

mod fn_bytecode;

salsa::query_group! {
    pub trait MirDatabase: typecheck::TypeCheckDatabase + AsRef<DeclarationTables> {
        fn fn_bytecode(key: Entity) -> WithError<Arc<FnBytecode>> {
            type FnBytecodeQuery;
            use fn fn_bytecode::fn_bytecode;
        }
    }
}

/*
indices::index_type! {
    pub struct Identifier { .. }
}

#[derive(Copy, Clone, Debug, DebugWith, PartialEq, Eq, Hash)]
pub struct IdentifierData {
    pub text: GlobalIdentifier,
}

indices::index_type! {
    pub struct Variable { .. }
}

#[derive(Copy, Clone, Debug, DebugWith, PartialEq, Eq, Hash)]
pub struct VariableData {
    pub name: Identifier,
}

*/

indices::index_type! {
    pub struct Error { .. }
}

#[derive(Clone, Debug, DebugWith, PartialEq, Eq, Hash)]
pub enum ErrorData {
    Misc,
    Unimplemented,
    UnknownIdentifier { text: GlobalIdentifier },
}

/// All the data for a fn-body is stored in these tables.a
#[derive(Clone, Debug, DebugWith, Default, PartialEq, Eq, Hash)]
pub struct FnBytecodeTables {
    /// Map each statement to its associated data.
    pub statements: IndexVec<Statement, Spanned<StatementData>>,

    /// The blocks that make up the code of the function
    pub basic_blocks: IndexVec<BasicBlock, Spanned<BasicBlockData>>,

    /// Map each place index to its associated data.
    pub places: IndexVec<Place, Spanned<PlaceData>>,

    /// Map each variable index to its associated data.
    pub variables: IndexVec<Variable, Spanned<VariableData>>,

    /// Map each identifier index to its associated data.
    pub identifiers: IndexVec<Identifier, Spanned<IdentifierData>>,

    /// Map each rvalue index to its associated data.
    pub rvalues: IndexVec<Rvalue, Spanned<RvalueData>>,

    /// Map each operand index to its associated data.
    pub operands: IndexVec<Operand, Spanned<OperandData>>,

    /// The data values for any `List<I>` values that appear elsewhere
    /// in the HIR; the way this works is that all of the list value
    /// are concatenated into one big vector, and each list just pulls
    /// out a slice of that. Note that this just contains `u32` values
    /// -- the actual `List<I>` remembers the index type `I` for its
    /// own values and does the casting back and forth.
    pub list_entries: Vec<u32>,
}

impl AsMut<FnBytecodeTables> for FnBytecodeTables {
    fn as_mut(&mut self) -> &mut Self {
        self
    }
}

//Lark MIR representation of a single function
#[derive(Clone, Debug, DebugWith, PartialEq, Eq, Hash)]
pub struct FnBytecode {
    /// List of arguments to the function. The type of each argument
    /// is given by the function signature (which can be separately queried).
    pub arguments: List<Variable>,

    /// The code of the function body, split into basic blocks
    pub basic_blocks: List<BasicBlock>,

    pub tables: FnBytecodeTables,
}

indices::index_type! {
    pub struct BasicBlock { .. }
}

#[derive(Clone, Debug, DebugWith, PartialEq, Eq, Hash)]
pub struct BasicBlockData {
    pub statements: List<Statement>,
    pub terminator: Terminator,
}

indices::index_type! {
    pub struct Statement { .. }
}

#[derive(Clone, Debug, DebugWith, PartialEq, Eq, Hash)]
pub struct StatementData {
    pub kind: StatementKind,
}

#[derive(Clone, Debug, DebugWith, PartialEq, Eq, Hash)]
pub enum StatementKind {
    Assign(Place, Rvalue),

    /// Start a live range for the storage of the variable.
    StorageLive(Variable),

    /// End the current live range for the storage of the variable.
    StorageDead(Variable),

    Expression(Rvalue),
}

#[derive(Clone, Debug, DebugWith, PartialEq, Eq, Hash)]
pub enum Terminator {
    Return,
    PassThrough,
}

indices::index_type! {
    pub struct Rvalue { .. }
}
#[derive(Clone, Debug, DebugWith, PartialEq, Eq, Hash)]
pub enum RvalueData {
    Use(Operand),
    BinaryOp(BinOp, Variable, Variable),
    //FIXME: MIR has this as a TerminatorData, presumably because stack can unwind
    Call(Entity, List<Operand>),
}

indices::index_type! {
    pub struct Operand { .. }
}
#[derive(Clone, Debug, DebugWith, PartialEq, Eq, Hash)]
pub enum OperandData {
    Copy(Place),
    Move(Place),
    //FIXME: Move to Box<Constant>
    ConstantInt(i32),
    ConstantString(String),
}

#[derive(Clone, Debug, DebugWith, PartialEq, Eq, Hash)]
pub enum BinOp {
    Add,
    Sub,
}

/// Trait implemented by the various kinds of indices that reach into
/// the HIR; allows us to grab the vector that they correspond to.
pub trait MirIndex: U32Index + Into<MetaIndex> {
    type Data: Clone;

    fn index_vec(mir: &FnBytecodeTables) -> &IndexVec<Self, Spanned<Self::Data>>;
    fn index_vec_mut(mir: &mut FnBytecodeTables) -> &mut IndexVec<Self, Spanned<Self::Data>>;
}

pub trait MirIndexData: Sized + Clone {
    type Index: MirIndex<Data = Self>;

    fn index_vec(mir: &FnBytecodeTables) -> &IndexVec<Self::Index, Spanned<Self>> {
        <<Self as MirIndexData>::Index as MirIndex>::index_vec(mir)
    }

    fn index_vec_mut(mir: &mut FnBytecodeTables) -> &mut IndexVec<Self::Index, Spanned<Self>> {
        <<Self as MirIndexData>::Index as MirIndex>::index_vec_mut(mir)
    }
}

impl AsRef<FnBytecodeTables> for FnBytecode {
    fn as_ref(&self) -> &FnBytecodeTables {
        &self.tables
    }
}

impl AsRef<FnBytecodeTables> for Arc<FnBytecode> {
    fn as_ref(&self) -> &FnBytecodeTables {
        &self.tables
    }
}

/// Permit indexing the HIR by any of the various index types.
/// Returns the underlying data from the index, skipping over the
/// span.
impl<I> std::ops::Index<I> for FnBytecode
where
    I: MirIndex,
{
    type Output = I::Data;

    fn index(&self, index: I) -> &I::Data {
        &self.tables[index]
    }
}

impl<I> std::ops::Index<I> for FnBytecodeTables
where
    I: MirIndex,
{
    type Output = I::Data;

    fn index(&self, index: I) -> &I::Data {
        &I::index_vec(self)[index]
    }
}

indices::index_type! {
    pub struct Place { .. }
}

#[derive(Copy, Clone, Debug, DebugWith, PartialEq, Eq, Hash)]
pub enum PlaceData {
    Variable(Variable),
    Entity(Entity),
    Field { owner: Place, name: Identifier },
}

#[derive(Copy, Clone, Debug, DebugWith, PartialEq, Eq, Hash)]
pub enum LiteralData {
    String(GlobalIdentifier),
}

indices::index_type! {
    pub struct Variable { .. }
}

#[derive(Copy, Clone, Debug, DebugWith, PartialEq, Eq, Hash)]
pub struct VariableData {
    pub name: Identifier,
}

indices::index_type! {
    pub struct Identifier { .. }
}

#[derive(Copy, Clone, Debug, DebugWith, PartialEq, Eq, Hash)]
pub struct IdentifierData {
    pub text: GlobalIdentifier,
}

/// Trait for the various types for which a span can be had --
/// corresponds to all the index types plus `MetaIndex`.
pub trait SpanIndex {
    fn span_from(self, tables: &FnBytecodeTables) -> Span;
}

impl FnBytecode {
    /// Get the span for the given part of the HIR.
    pub fn span(&self, index: impl SpanIndex) -> Span {
        index.span_from(&self.tables)
    }
}

impl FnBytecodeTables {
    /// Get the span for the given part of the HIR.
    pub fn span(&self, index: impl SpanIndex) -> Span {
        index.span_from(self)
    }
}

impl<I: MirIndex> SpanIndex for I {
    fn span_from(self, tables: &FnBytecodeTables) -> Span {
        I::index_vec(tables)[self].span()
    }
}

/// Declares impls for each kind of MIR index as well as the
/// `mir::MetaIndex` enum.
macro_rules! define_meta_index {
    ($(($index_ty:ident, $data_ty:ty, $field:ident),)*) => {
        $(
            impl MirIndex for $index_ty {
                type Data = $data_ty;

                fn index_vec(mir: &FnBytecodeTables) -> &IndexVec<Self, Spanned<Self::Data>> {
                    &mir.$field
                }

                fn index_vec_mut(
                    mir: &mut FnBytecodeTables,
                ) -> &mut IndexVec<Self, Spanned<Self::Data>> {
                    &mut mir.$field
                }
            }

            impl MirIndexData for $data_ty {
                type Index = $index_ty;
            }

            debug::debug_fallback_impl!($index_ty);

            impl From<$index_ty> for MetaIndex {
                fn from(value: $index_ty) -> MetaIndex {
                    MetaIndex::$index_ty(value)
                }
            }
        )*

        /// The HIR has a number of *kinds* of indices that
        /// reach into it. This enum brings them together into
        /// a sort of "meta index". It's useful sometimes.
        #[derive(Copy, Clone, Debug, DebugWith, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum MetaIndex {
            $(
                $index_ty($index_ty),
            )*
        }

        impl SpanIndex for MetaIndex {
            fn span_from(self, tables: &FnBytecodeTables) -> Span {
                match self {
                    $(
                        MetaIndex::$index_ty(index) => index.span_from(tables),
                    )*
                }
            }
        }
    };
}

define_meta_index! {
    (Statement, StatementData, statements),
    (BasicBlock, BasicBlockData, basic_blocks),
    (Place, PlaceData, places),
    (Variable, VariableData, variables),
    (Identifier, IdentifierData, identifiers),
    (Operand, OperandData, operands),
    (Rvalue, RvalueData, rvalues),
}

/// A list of "MIR indices" of type `I`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct List<I: MirIndex> {
    start_index: u32,
    len: u32,
    marker: std::marker::PhantomData<I>,
}

impl<I: MirIndex> Default for List<I> {
    fn default() -> Self {
        List {
            start_index: 0,
            len: 0,
            marker: std::marker::PhantomData,
        }
    }
}

impl<I: MirIndex> List<I> {
    /// Creates a list containing the values from in the
    /// `start_index..end_index` from the enclosing `FnBodyTables`.
    /// Ordinarily, you would not use this constructor, but rather
    /// `from_iterator`.
    fn from_start_and_end(start_index: usize, end_index: usize) -> Self {
        assert_eq!((start_index as u32) as usize, start_index);
        assert!(end_index >= start_index);

        if start_index == end_index {
            List::default()
        } else {
            List {
                start_index: start_index as u32,
                len: (end_index - start_index) as u32,
                marker: std::marker::PhantomData,
            }
        }
    }

    /// Creates a `List` containing the results of `from_iterator`.
    pub fn from_iterator(
        mut fn_bytecode: impl AsMut<FnBytecodeTables>,
        iterator: impl IntoIterator<Item = I>,
    ) -> Self {
        let tables = fn_bytecode.as_mut();
        let start_index = tables.list_entries.len();
        tables
            .list_entries
            .extend(iterator.into_iter().map(|i| i.as_u32()));
        let end_index = tables.list_entries.len();
        List::from_start_and_end(start_index, end_index)
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn len(&self) -> usize {
        self.len as usize
    }

    /// Iterate over the elements in the list.
    pub fn iter(
        &self,
        fn_bytecode: &'f impl AsRef<FnBytecodeTables>,
    ) -> impl Iterator<Item = I> + 'f {
        let tables: &FnBytecodeTables = fn_bytecode.as_ref();
        let start_index = self.start_index as usize;
        let end_index = start_index + self.len as usize;
        tables.list_entries[start_index..end_index]
            .iter()
            .cloned()
            .map(I::from_u32)
    }

    /// Iterate over the data for each the element in the list.
    pub fn iter_data(
        &self,
        fn_bytecode: &'f impl AsRef<FnBytecodeTables>,
    ) -> impl Iterator<Item = I::Data> + 'f {
        self.iter_enumerated_data(fn_bytecode).map(|(_, d)| d)
    }

    /// Iterate over the elements in the list *and* their associated
    /// data.
    pub fn iter_enumerated_data(
        &self,
        fn_bytecode: &'f impl AsRef<FnBytecodeTables>,
    ) -> impl Iterator<Item = (I, I::Data)> + 'f {
        let tables: &FnBytecodeTables = fn_bytecode.as_ref();
        let data_vec = I::index_vec(tables);
        self.iter(fn_bytecode).map(move |i| {
            let data: &I::Data = &data_vec[i];
            (i, data.clone())
        })
    }
}

debug::debug_fallback_impl!(for[I: MirIndex] List<I>);
