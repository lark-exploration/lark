use ast::ast as a;
use crate::error::or_sentinel;
use crate::error::WithError;
use crate::HirDatabase;
use debug::DebugWith;
use intern::Intern;
use intern::Untern;
use lark_entity::Entity;
use lark_entity::EntityData;
use lark_entity::LangItem;
use lark_entity::MemberKind;
use ty::declaration::Declaration;
use ty::BaseData;
use ty::BaseKind;
use ty::BoundVar;
use ty::BoundVarOr;
use ty::Erased;
use ty::GenericKind;
use ty::Generics;
use ty::Ty;
use ty::TypeFamily;

crate fn ty(db: &impl HirDatabase, entity: Entity) -> WithError<ty::Ty<Declaration>> {
    match entity.untern(db) {
        EntityData::Error => WithError::error_sentinel(db),

        EntityData::LangItem(LangItem::Boolean) => {
            WithError::ok(declaration_ty_named(db, entity, Generics::empty()))
        }

        EntityData::LangItem(LangItem::Tuple(arity)) => {
            let generics: Generics<Declaration> = (0..arity)
                .map(|i| BoundVarOr::BoundVar(BoundVar::new(i)))
                .map(|bv| Ty {
                    base: bv.intern(db),
                    perm: Declaration::own_perm(db),
                })
                .map(|ty| GenericKind::Ty(ty))
                .collect();
            WithError::ok(declaration_ty_named(db, entity, generics))
        }

        EntityData::ItemName { .. } => {
            let ast = or_sentinel!(db, db.ast_of_item(entity));

            match &*ast {
                a::Item::Struct(_) | a::Item::Def(_) => {
                    WithError::ok(declaration_ty_named(db, entity, Generics::empty()))
                }
            }
        }

        EntityData::MemberName {
            base,
            kind: MemberKind::Field,
            id,
        } => match &*or_sentinel!(db, db.ast_of_item(base)) {
            a::Item::Struct(s) => match s.fields.iter().find(|f| *f.name == id) {
                Some(field) => declaration_ty_from_ast_ty(db, entity, &field.ty),

                None => panic!("no such field"),
            },

            ast => panic!("field of invalid entity {:?}", ast),
        },

        EntityData::MemberName {
            base: _,
            kind: MemberKind::Method,
            id: _,
        } => WithError::ok(declaration_ty_named(db, entity, Generics::empty())),

        EntityData::InputFile { .. } => panic!(
            "cannot get type of entity with data {:?}",
            entity.untern(db).debug_with(db),
        ),
    }
}

fn declaration_ty_named(
    db: &impl HirDatabase,
    entity: Entity,
    generics: Generics<Declaration>,
) -> Ty<Declaration> {
    let kind = BaseKind::Named(entity);
    let base = Declaration::intern_base_data(db, BaseData { kind, generics });
    Ty { perm: Erased, base }
}

fn declaration_ty_from_ast_ty(
    db: &impl HirDatabase,
    scope_entity: Entity,
    ast_ty: &a::Type,
) -> WithError<Ty<Declaration>> {
    match db.resolve_name(scope_entity, *ast_ty.name) {
        Some(entity) => WithError::ok(declaration_ty_named(db, entity, Generics::empty())),
        None => WithError::report_error(db, ast_ty.name.1),
    }
}
