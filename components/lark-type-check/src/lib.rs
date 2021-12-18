#![feature(const_mut_refs)]
#![feature(crate_visibility_modifier)]
#![feature(in_band_lifetimes)]
#![feature(never_type)]
#![feature(specialization)]
#![feature(trait_alias)]

use generational_arena::Arena;
use lark_collections::{FxIndexMap, IndexVec};
use lark_debug_derive::DebugWith;
use lark_entity::{Entity, EntityTables};
use lark_error::{Diagnostic, WithError};
use lark_hir as hir;
use lark_parser::ParserDatabase;
use lark_pretty_print::PrettyPrintDatabase;
use lark_ty::base_inferred::BaseInferred;
use lark_ty::base_inferred::BaseInferredTables;
use lark_ty::declaration::Declaration;
use lark_ty::declaration::DeclarationTables;
use lark_ty::full_inferred::FullInferred;
use lark_ty::full_inferred::FullInferredTables;
use lark_ty::map_family::Map;
use lark_ty::BaseData;
use lark_ty::Generics;
use lark_ty::Placeholder;
use lark_ty::Ty;
use lark_ty::TypeFamily;
use lark_ty::Universe;
use lark_unify::InferVar;
use lark_unify::Inferable;
use lark_unify::UnificationTable;
use std::sync::Arc;

mod base_inference;
mod full_inference;
mod hir_typeck;
mod ops;
mod results;
mod substitute;

#[salsa::query_group(TypeCheckStorage)]
pub trait TypeCheckDatabase:
    ParserDatabase + AsRef<BaseInferredTables> + AsRef<FullInferredTables> + PrettyPrintDatabase
{
    /// Compute the "base type information" for a given fn body.
    /// This is the type information excluding permissions.
    #[salsa::invoke(base_inference::query_definition::base_type_check)]
    fn base_type_check(&self, key: Entity) -> WithError<Arc<TypeCheckResults<BaseInferred>>>;

    /// Compute the "base type information" for a given fn body.
    /// This is the type information excluding permissions.
    #[salsa::invoke(full_inference::query_definition::full_type_check)]
    fn full_type_check(&self, key: Entity) -> WithError<Arc<TypeCheckResults<FullInferred>>>;
}

pub use results::TypeCheckResults;

struct TypeChecker<'me, F: TypeCheckerFamily, S> {
    /// Salsa database.
    db: &'me dyn TypeCheckDatabase,

    /// Intern tables for the family `F`. These are typically local to
    /// the type-check itself.
    f_tables: F::InternTables,

    /// Entity being type-checked.
    fn_entity: Entity,

    /// Storage that depends on the type-checker family.
    storage: S,

    /// HIR for the `fn_entity` being type-checked.
    hir: Arc<hir::FnBody>,

    /// Arena where we allocate suspended type-check operations;
    /// operations are suspended until type-inference variables
    /// get unified.
    ops_arena: Arena<Box<dyn ops::BoxedTypeCheckerOp<Self>>>,

    /// Map storing blocked operations: once the given infer variable
    /// is unified, we should execute the operation.
    ops_blocked: FxIndexMap<InferVar, Vec<ops::OpIndex>>,

    /// Unification table for the type-check family.
    unify: UnificationTable<F::InternTables, hir::MetaIndex>,

    /// Information about each universe that we have created.
    universe_binders: IndexVec<Universe, UniverseBinder>,

    /// Errors that we encountered during the type-check.
    errors: Vec<Diagnostic>,
}

enum UniverseBinder {
    Root,
    FromItem(Entity),
}

/// A trait alias for a type family that has `Placeholder` mapped to
/// `Placeholder`.  These are the kinds of type families the type
/// checker can use.
trait TypeCheckerFamily: TypeFamily<Placeholder = Placeholder> {}
impl<T: TypeFamily<Placeholder = Placeholder>> TypeCheckerFamily for T {}

/// An "extension trait" for the `TypeChecker` that defines the
/// operations which differ depending on the active type-family.  You
/// will find e.g. one implementation of this for the `BaseInference`
/// type family and one for more complete inference (not yet
/// implemented).
trait TypeCheckerFamilyDependentExt<F: TypeCheckerFamily>
where
    Self: AsRef<F::InternTables>,
    Self: TypeCheckerVariableExt<F, Ty<F>>,
    F::Base: Inferable<F::InternTables, KnownData = BaseData<F>>,
{
    /// Substitute the given generics into the value `M`, which must
    /// be something in the `Declaration` type family (e.g., the type
    /// of a field).
    fn substitute<M>(
        &mut self,
        location: impl Into<HirLocation>,
        generics: &Generics<F>,
        value: M,
    ) -> M::Output
    where
        M: Map<Declaration, F>;

    /// Adjust the type of `value` to account for having been
    /// projected from an owned with the given permissions
    /// `access_perm` (e.g., when accessing a field).
    fn apply_owner_perm(
        &mut self,
        cause: impl Into<hir::MetaIndex>,
        location: impl Into<HirLocation>,
        access_perm: F::Perm,
        field_ty: Ty<F>,
    ) -> Ty<F>;

    /// Requests the type for a given HIR variable. Upon the first
    /// request, the result may be a fresh inference variable.
    fn request_variable_ty(&mut self, var: hir::Variable) -> Ty<F>;

    /// Records that the type of the variable `var` is `ty`.
    fn record_variable_ty(&mut self, var: hir::Variable, ty: Ty<F>);

    /// Records that the "max type" of the expression `expr` is
    /// `ty`. Returns the "accessed type" for the expression. The "max
    /// type" indicates the "full permissions" available from the
    /// given expression; the accessed type includes variables to
    /// figure out how many permissions are needed from this
    /// particular access.
    ///
    /// Example:
    ///
    /// ```ignore
    /// foo(bar.baz)
    /// ```
    ///
    /// the expression `bar.baz` may have a type like `own String`,
    /// but `foo` may only require `share String`. In full inference
    /// mode, invoking `record_max_expression_ty(bar.bar, own String)`
    /// would return a variable `P0 String`, where `P0` is a
    /// permission variable. This variable `P0` would later be
    /// inferred to only `share`.
    fn record_max_expression_ty(&mut self, expr: hir::Expression, ty: Ty<F>) -> Ty<F>;

    /// Requests the type for a given HIR variable. Upon the first
    /// request, the result may be a fresh inference variable.
    fn record_place_ty(&mut self, place: hir::Place, ty: Ty<F>) -> Ty<F>;

    /// Record the entity to which a particular identifier in the HIR resolved.
    /// Used for:
    ///
    /// - field names, in places and aggregate expressions
    /// - method names, in calls
    fn record_entity(&mut self, index: hir::Identifier, entity: Entity);

    /// Records that `index` refers to `entity` and returns the
    /// generic parameters it uses to do so; may instantiate fresh
    /// type variables.
    fn record_entity_and_get_generics(
        &mut self,
        index: impl Into<hir::MetaIndex>,
        entity: Entity,
    ) -> Generics<F>;
}

/// Trait for "inferable values" of type `V` (e.g., types).
trait TypeCheckerVariableExt<F: TypeCheckerFamily, V> {
    /// Creates a new type (or other value) with fresh inference variable(s).
    fn new_variable(&mut self) -> V;

    /// Equates two types or other inferable values (producing an error if they are not
    /// equatable).
    fn equate(
        &mut self,
        cause: impl Into<hir::MetaIndex>,
        location: impl Into<HirLocation>,
        val1: V,
        val2: V,
    );
}

impl<F, S> AsRef<DeclarationTables> for TypeChecker<'_, F, S>
where
    F: TypeCheckerFamily,
{
    fn as_ref(&self) -> &DeclarationTables {
        self.db.as_ref()
    }
}

impl<F, S> AsRef<BaseInferredTables> for TypeChecker<'_, F, S>
where
    F: TypeCheckerFamily,
{
    fn as_ref(&self) -> &BaseInferredTables {
        self.db.as_ref()
    }
}

impl<F, S> AsRef<EntityTables> for TypeChecker<'_, F, S>
where
    F: TypeCheckerFamily,
{
    fn as_ref(&self) -> &EntityTables {
        self.db.as_ref()
    }
}

#[derive(Copy, Clone, Hash, Debug, DebugWith, PartialEq, Eq)]
enum HirLocation {
    Start,
    Return,
    Expression(hir::Expression),
    AfterExpression(hir::Expression),
    Place(hir::Place),
    Error,
}

impl Into<HirLocation> for hir::Expression {
    fn into(self) -> HirLocation {
        HirLocation::Expression(self)
    }
}

impl Into<HirLocation> for hir::Place {
    fn into(self) -> HirLocation {
        HirLocation::Place(self)
    }
}
