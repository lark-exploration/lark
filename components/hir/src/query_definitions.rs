use ast::ast as a;
use crate::ErrorReported;
use crate::HirDatabase;
use crate::Member;
use debug::DebugWith;
use intern::Intern;
use intern::Untern;
use lark_entity::Entity;
use lark_entity::EntityData;
use lark_entity::MemberKind;
use parser::StringId;
use std::sync::Arc;
use ty::declaration::Declaration;
use ty::BaseData;
use ty::BaseKind;
use ty::Erased;
use ty::Generics;
use ty::Ty;
use ty::TypeFamily;

crate fn boolean_entity(_db: &impl HirDatabase, _key: ()) -> Entity {
    unimplemented!()
}

crate fn members(db: &impl HirDatabase, owner: Entity) -> Result<Arc<Vec<Member>>, ErrorReported> {
    match db.ast_of_item(owner) {
        Ok(ast) => match &*ast {
            a::Item::Struct(s) => Ok(Arc::new(
                s.fields
                    .iter()
                    .map(|f| {
                        let field_entity = EntityData::MemberName {
                            base: owner,
                            kind: MemberKind::Field,
                            id: *f.name,
                        }
                        .intern(db);

                        Member {
                            name: *f.name,
                            kind: MemberKind::Field,
                            entity: field_entity,
                        }
                    })
                    .collect(),
            )),

            a::Item::Def(_) => panic!("asked for members of a function"),
        },

        Err(_parse_error) => Err(ErrorReported),
    }
}

crate fn member_entity(
    db: &impl HirDatabase,
    (owner, kind, name): (Entity, MemberKind, StringId),
) -> Result<Option<Entity>, ErrorReported> {
    Ok(db
        .members(owner)?
        .iter()
        .filter_map(|member| {
            if member.kind == kind && member.name == name {
                Some(member.entity)
            } else {
                None
            }
        })
        .next())
}

crate fn ty(db: &impl HirDatabase, entity: Entity) -> ty::Ty<Declaration> {
    match entity.untern(db) {
        EntityData::ItemName { .. } => {
            let ast = match db.ast_of_item(entity) {
                Ok(ast) => ast,
                Err(_) => return Declaration::error_ty(db),
            };
            let kind = match &*ast {
                a::Item::Struct(_) | a::Item::Def(_) => BaseKind::Named(entity),
            };
            let generics = Generics::empty();
            let base = Declaration::intern_base_data(db, BaseData { kind, generics });
            Ty { perm: Erased, base }
        }

        EntityData::MemberName { .. } => unimplemented!(),

        EntityData::InputFile { .. } => panic!(
            "cannot get type of entity with data {:?}",
            entity.untern(db).debug_with(db),
        ),
    }
}

crate fn signature(_db: &impl HirDatabase, _key: Entity) -> ty::Signature<Declaration> {
    unimplemented!()
}

crate fn generic_declarations(
    _db: &impl HirDatabase,
    _key: Entity,
) -> Arc<ty::GenericDeclarations> {
    unimplemented!()
}
