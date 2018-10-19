use ast::ast as a;
use crate::ErrorReported;
use crate::HirDatabase;
use debug::DebugWith;
use intern::Untern;
use lark_entity::Entity;
use lark_entity::EntityData;
use lark_entity::MemberKind;
use ty::declaration::Declaration;
use ty::BaseData;
use ty::BaseKind;
use ty::Erased;
use ty::Generics;
use ty::Ty;
use ty::TypeFamily;

crate fn ty(db: &impl HirDatabase, entity: Entity) -> Result<ty::Ty<Declaration>, ErrorReported> {
    match entity.untern(db) {
        EntityData::ItemName { .. } => {
            let ast = db.ast_of_item(entity)?;
            match &*ast {
                a::Item::Struct(_) | a::Item::Def(_) => {
                    Ok(declaration_ty_named(db, entity, Generics::empty()))
                }
            }
        }

        EntityData::MemberName {
            base,
            kind: MemberKind::Field,
            id,
        } => match &*db.ast_of_item(base)? {
            a::Item::Struct(s) => match s.fields.iter().find(|f| *f.name == id) {
                Some(field) => Ok(declaration_ty_from_ast_ty(db, entity, &field.ty)?),

                None => panic!("no such field"),
            },

            ast => panic!("field of invalid entity {:?}", ast),
        },

        EntityData::MemberName {
            base: _,
            kind: MemberKind::Method,
            id: _,
        } => Ok(declaration_ty_named(db, entity, Generics::empty())),

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
    _db: &impl HirDatabase,
    _scope_entity: Entity,
    _ast_ty: &a::Type,
) -> Result<Ty<Declaration>, ErrorReported> {
    unimplemented!()
}
