use ast::ast as a;
use crate::error::ErrorReported;
use crate::HirDatabase;
use crate::Member;
use intern::Intern;
use lark_entity::Entity;
use lark_entity::EntityData;
use lark_entity::LangItem;
use lark_entity::MemberKind;
use parser::StringId;
use std::sync::Arc;
use ty::declaration::Declaration;

crate fn boolean_entity(db: &impl HirDatabase) -> Entity {
    EntityData::LangItem(LangItem::Boolean).intern(db)
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
    owner: Entity,
    kind: MemberKind,
    name: StringId,
) -> Option<Entity> {
    match db.members(owner) {
        Err(_) => Some(EntityData::Error.intern(db)),

        Ok(members) => members
            .iter()
            .filter_map(|member| {
                if member.kind == kind && member.name == name {
                    Some(member.entity)
                } else {
                    None
                }
            })
            .next(),
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
