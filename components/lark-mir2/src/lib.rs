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
use lark_hir as hir;
use lark_string::global::GlobalIdentifier;
use lark_ty::declaration::Declaration;
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

#[derive(Copy, Clone, Debug, DebugWith, PartialEq, Eq, Hash)]
pub enum LiteralData {
    String(GlobalIdentifier),
}

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
    /// Map each expression index to its associated data.
    pub statements: IndexVec<Statement, Spanned<StatementData>>,

    pub basic_blocks: IndexVec<BasicBlock, Spanned<BasicBlockData>>,

    /// Map each place index to its associated data.
    pub places: IndexVec<Place, Spanned<PlaceData>>,

    /// Map each variable index to its associated data.
    pub variables: IndexVec<Variable, Spanned<VariableData>>,

    /// Map each identifier index to its associated data.
    pub identifiers: IndexVec<Identifier, Spanned<IdentifierData>>,

    /// Map each struct index to its associated data.
    pub structs: IndexVec<Struct, Spanned<StructData>>,

    /// Map each field index to its associated data.
    pub fields: IndexVec<Field, Spanned<FieldData>>,

    /// Map each terminator index to its associated data.
    pub terminators: IndexVec<Terminator, Spanned<TerminatorData>>,

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
    pub basic_blocks: List<BasicBlock>,

    //First local = return value pointer
    //Followed by arg_count parameters to the function
    //Followed by user defined variables and temporaries
    pub local_decls: List<Variable>,

    pub arg_count: usize,

    pub name: String,

    pub tables: FnBytecodeTables,
}

/*
impl FnBytecode {
    pub fn new(return_ty: Ty, mut args: Vec<LocalDecl>, name: String) -> FnBytecode {
        let arg_count = args.len();
        let mut local_decls = vec![LocalDecl::new_return_place(return_ty)];
        local_decls.append(&mut args);

        FnBytecode {
            basic_blocks: vec![],
            local_decls,
            arg_count,
            name,
        }
    }

    pub fn new_temp(&mut self, ty: Ty) -> VarId {
        self.local_decls.push(LocalDecl::new_temp(ty));
        self.local_decls.len() - 1
    }

    pub fn push_block(&mut self, block: BasicBlockData) {
        self.basic_blocks.push(block);
    }
}
*/

indices::index_type! {
    pub struct Struct { .. }
}

#[derive(Clone, Debug, DebugWith, PartialEq, Eq, Hash)]
pub struct StructData {
    pub fields: List<Field>,
    pub name: GlobalIdentifier,
}

indices::index_type! {
    pub struct Field { .. }
}

#[derive(Clone, Debug, DebugWith, PartialEq, Eq, Hash)]
pub struct FieldData {
    pub name: String,
}

indices::index_type! {
    pub struct BasicBlock { .. }
}

#[derive(Clone, Debug, DebugWith, PartialEq, Eq, Hash)]
pub struct BasicBlockData {
    pub statements: Vec<StatementData>,
    pub terminator: Option<TerminatorData>,
}

impl BasicBlockData {
    pub fn new() -> BasicBlockData {
        BasicBlockData {
            statements: vec![],
            terminator: None,
        }
    }

    pub fn push_stmt(&mut self, kind: StatementKind) {
        self.statements.push(StatementData { kind });
    }

    pub fn terminate(&mut self, terminator_kind: TerminatorKind) {
        self.terminator = Some(TerminatorData {
            kind: terminator_kind,
        });
    }
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
    Assign(PlaceData, Rvalue),
    DebugPrint(PlaceData),
}

indices::index_type! {
    pub struct Terminator { .. }
}

#[derive(Clone, Debug, DebugWith, PartialEq, Eq, Hash)]
pub struct TerminatorData {
    pub kind: TerminatorKind,
}

#[derive(Clone, Debug, DebugWith, PartialEq, Eq, Hash)]
pub enum TerminatorKind {
    Return,
}

indices::index_type! {
    pub struct Place { .. }
}

#[derive(Clone, Debug, DebugWith, PartialEq, Eq, Hash)]
pub enum PlaceData {
    Variable(Variable),
    Static(Entity),
    Field { owner: Place, name: Identifier },
}

#[derive(Clone, Debug, DebugWith, PartialEq, Eq, Hash)]
pub enum Rvalue {
    Use(Operand),
    BinaryOp(BinOp, Variable, Variable),
    //FIXME: MIR has this as a TerminatorData, presumably because stack can unwind
    Call(Entity, Vec<Operand>),
}

#[derive(Clone, Debug, DebugWith, PartialEq, Eq, Hash)]
pub enum Operand {
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
    (Field, FieldData, fields),
    (Struct, StructData, structs),
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
        mut fn_body: impl AsMut<FnBytecodeTables>,
        iterator: impl IntoIterator<Item = I>,
    ) -> Self {
        let tables = fn_body.as_mut();
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

/*
#[derive(Clone, Debug, DebugWith, PartialEq, Eq, Hash)]
pub struct LocalDecl {
    pub ty: Ty,
    pub name: Option<String>,
}

impl LocalDecl {
    pub fn new_return_place(return_ty: Ty) -> LocalDecl {
        LocalDecl {
            ty: return_ty,
            name: None,
        }
    }

    pub fn new_temp(ty: Ty) -> LocalDecl {
        LocalDecl { ty, name: None }
    }

    pub fn new(ty: Ty, name: Option<String>) -> LocalDecl {
        LocalDecl { ty, name }
    }
}


pub mod builtin_type {
    #[allow(unused)]
    pub const UNKNOWN: usize = 0;
    pub const VOID: usize = 1;
    pub const I32: usize = 2;
    pub const STRING: usize = 3;
    pub const ERROR: usize = 100;
}

#[derive(Debug)]
pub enum Definition {
    Builtin,
    Fn(FnBytecode),
    Struct(Struct),
}

pub struct Context {
    pub definitions: Vec<Definition>,
}

impl Context {
    pub fn new() -> Context {
        let mut definitions = vec![];

        for _ in 0..(builtin_type::ERROR + 1) {
            definitions.push(Definition::Builtin); // UNKNOWN
        }

        Context { definitions }
    }

    pub fn add_definition(&mut self, def: Definition) -> usize {
        self.definitions.push(def);
        self.definitions.len() - 1
    }

    pub fn simple_type_for_entity(&self, entity: Entity) -> Ty {
        Ty { entity: entity }
    }

    pub fn get_entity_for_ty(&self, ty: Ty) -> Option<Entity> {
        Some(ty.entity)
    }
}
*/
