use crate::substitute::Substitution;
use crate::TypeCheckDatabase;
use crate::TypeCheckFamily;
use crate::TypeChecker;
use crate::TypeCheckerFields;
use intern::Intern;
use lark_entity::EntityData;
use lark_entity::LangItem;
use lark_hir as hir;
use lark_ty::base_inference::{Base, BaseInference, BaseInferenceTables, BaseTy};
use lark_ty::declaration::Declaration;
use lark_ty::identity::Identity;
use lark_ty::map_family::Map;
use lark_ty::Erased;
use lark_ty::Ty;
use lark_ty::TypeFamily;
use lark_ty::{BaseData, BaseKind};
use lark_ty::{GenericKind, Generics};

impl TypeCheckFamily for BaseInference {
    type TcBase = Base;

    fn new_infer_ty(this: &mut impl TypeCheckerFields<Self>) -> Ty<Self> {
        Ty {
            repr: Erased,
            perm: Erased,
            base: this.unify().new_inferable(),
        }
    }

    fn equate_types(
        this: &mut impl TypeCheckerFields<Self>,
        cause: hir::MetaIndex,
        ty1: Ty<BaseInference>,
        ty2: Ty<BaseInference>,
    ) {
        let Ty {
            repr: Erased,
            perm: Erased,
            base: base1,
        } = ty1;
        let Ty {
            repr: Erased,
            perm: Erased,
            base: base2,
        } = ty2;

        match this.unify().unify(cause, base1, base2) {
            Ok(()) => {}

            Err((data1, data2)) => {
                match (data1.kind, data2.kind) {
                    (BaseKind::Error, _) => {
                        propagate_error(this, cause, data2);
                        return;
                    }
                    (_, BaseKind::Error) => {
                        propagate_error(this, cause, data1);
                        return;
                    }
                    _ => {}
                }

                if data1.kind != data2.kind {
                    this.record_error("Mismatched types".into(), cause);
                    return;
                }

                for (generic1, generic2) in data1.generics.iter().zip(&data2.generics) {
                    match (generic1, generic2) {
                        (GenericKind::Ty(g1), GenericKind::Ty(g2)) => {
                            Self::equate_types(this, cause, g1, g2);
                        }
                    }
                }
            }
        }
    }

    fn boolean_type(this: &impl TypeCheckerFields<Self>) -> BaseTy {
        primitive_type(this, LangItem::Boolean)
    }

    fn int_type(this: &impl TypeCheckerFields<Self>) -> BaseTy {
        primitive_type(this, LangItem::Int)
    }

    fn uint_type(this: &impl TypeCheckerFields<Self>) -> BaseTy {
        primitive_type(this, LangItem::Uint)
    }

    fn unit_type(this: &impl TypeCheckerFields<Self>) -> BaseTy {
        primitive_type(this, LangItem::Tuple(0))
    }

    fn string_type(this: &impl TypeCheckerFields<Self>) -> BaseTy {
        primitive_type(this, LangItem::String)
    }

    fn apply_user_perm(
        _this: &mut impl TypeCheckerFields<Self>,
        _perm: hir::Perm,
        place_ty: Ty<BaseInference>,
    ) -> Ty<BaseInference> {
        // In the "erased type check", we don't care about permissions.
        place_ty
    }

    fn require_assignable(
        this: &mut impl TypeCheckerFields<Self>,
        expression: hir::Expression,
        value_ty: Ty<BaseInference>,
        place_ty: Ty<BaseInference>,
    ) {
        Self::equate_types(this, expression.into(), value_ty, place_ty)
    }

    fn least_upper_bound(
        this: &mut impl TypeCheckerFields<Self>,
        if_expression: hir::Expression,
        true_ty: Ty<BaseInference>,
        false_ty: Ty<BaseInference>,
    ) -> Ty<BaseInference> {
        Self::equate_types(this, if_expression.into(), true_ty, false_ty);
        true_ty
    }

    fn substitute<M>(
        this: &mut impl TypeCheckerFields<Self>,
        _location: hir::MetaIndex,
        generics: &Generics<Self>,
        value: M,
    ) -> M::Output
    where
        M: Map<Declaration, Self>,
    {
        value.map(&mut Substitution::new(this, this, generics))
    }

    fn apply_owner_perm<M>(
        this: &mut impl TypeCheckerFields<Self>,
        _location: impl Into<hir::MetaIndex>,
        _owner_perm: Erased,
        value: M,
    ) -> M::Output
    where
        M: Map<Self, Self>,
    {
        value.map(&mut Identity::new(this))
    }
}

fn propagate_error<F: TypeCheckFamily>(
    this: &mut impl TypeCheckerFields<F>,
    cause: hir::MetaIndex,
    data: BaseData<F>,
) {
    let BaseData { kind: _, generics } = data;

    let error_type = F::error_type(this);

    for generic in generics.iter() {
        match generic {
            GenericKind::Ty(ty) => F::equate_types(this, cause, error_type, ty),
        }
    }
}

impl<DB> AsRef<BaseInferenceTables> for TypeChecker<'_, DB, BaseInference>
where
    DB: TypeCheckDatabase,
{
    fn as_ref(&self) -> &BaseInferenceTables {
        &self.f_tables
    }
}

fn primitive_type(this: &impl TypeCheckerFields<BaseInference>, item: LangItem) -> BaseTy {
    let entity = EntityData::LangItem(item).intern(this);
    Ty {
        repr: Erased,
        perm: Erased,
        base: BaseInference::intern_base_data(
            this,
            BaseData {
                kind: BaseKind::Named(entity),
                generics: Generics::empty(),
            },
        ),
    }
}
