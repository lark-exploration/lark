use crate::syntax::entity::LazyParsedEntityDatabase;
use crate::ParserDatabase;
use lark_debug_with::DebugWith;
use lark_entity::{Entity, EntityData, LangItem};
use lark_error::{ErrorReported, ErrorSentinel, WithError};
use lark_intern::{Intern, Untern};
use lark_ty as ty;
use lark_ty::declaration::Declaration;
use lark_ty::declaration::DeclarationTables;
use lark_ty::TypeFamily;
use std::sync::Arc;

crate fn generic_declarations(
    db: &impl ParserDatabase,
    entity: Entity,
) -> WithError<Result<Arc<ty::GenericDeclarations>, ErrorReported>> {
    match entity.untern(db) {
        EntityData::Error(report) => WithError::error_sentinel(db, report),

        EntityData::LangItem(LangItem::Boolean)
        | EntityData::LangItem(LangItem::String)
        | EntityData::LangItem(LangItem::Int)
        | EntityData::LangItem(LangItem::Uint)
        | EntityData::LangItem(LangItem::False)
        | EntityData::LangItem(LangItem::True)
        | EntityData::LangItem(LangItem::Debug) => {
            WithError::ok(Ok(ty::GenericDeclarations::empty(None)))
        }

        EntityData::LangItem(LangItem::Tuple(arity)) => {
            if arity != 0 {
                unimplemented!("non-zero arity tuples");
            }
            WithError::ok(Ok(ty::GenericDeclarations::empty(None)))
        }

        EntityData::ItemName { .. } | EntityData::MemberName { .. } => db
            .parsed_entity(entity)
            .thunk
            .parse_generic_declarations(entity, db),

        EntityData::InputFile { .. } => panic!(
            "cannot get generics of entity with data {:?}",
            entity.untern(db).debug_with(db),
        ),
    }
}

crate fn ty(db: &impl ParserDatabase, entity: Entity) -> WithError<ty::Ty<Declaration>> {
    match entity.untern(db) {
        EntityData::Error(report) => WithError::error_sentinel(db, report),

        EntityData::LangItem(LangItem::Boolean)
        | EntityData::LangItem(LangItem::String)
        | EntityData::LangItem(LangItem::Int)
        | EntityData::LangItem(LangItem::Uint)
        | EntityData::LangItem(LangItem::Debug) => WithError::ok(declaration_ty_named(
            db,
            entity,
            ty::declaration::DeclaredPermKind::Own,
            ty::ReprKind::Direct,
            ty::Generics::empty(),
        )),

        EntityData::LangItem(LangItem::False) | EntityData::LangItem(LangItem::True) => {
            let boolean_entity = EntityData::LangItem(LangItem::Boolean).intern(db);
            ty(db, boolean_entity)
        }

        EntityData::LangItem(LangItem::Tuple(arity)) => {
            let generics: ty::Generics<Declaration> = (0..arity)
                .map(|i| ty::BoundVar::new(i))
                .map(|bv| ty::Ty {
                    base: Declaration::intern_bound_var(db, bv),
                    repr: ty::ReprKind::Direct,
                    perm: Declaration::own_perm(db),
                })
                .map(|ty| ty::GenericKind::Ty(ty))
                .collect();
            WithError::ok(declaration_ty_named(
                db,
                entity,
                ty::declaration::DeclaredPermKind::Own,
                ty::ReprKind::Direct,
                generics,
            ))
        }

        EntityData::ItemName { .. } | EntityData::MemberName { .. } => {
            db.parsed_entity(entity).thunk.parse_type(entity, db)
        }

        EntityData::InputFile { .. } => panic!(
            "cannot get type of entity with data {:?}",
            entity.untern(db).debug_with(db),
        ),
    }
}

crate fn signature(
    db: &impl ParserDatabase,
    entity: Entity,
) -> WithError<Result<ty::Signature<Declaration>, ErrorReported>> {
    match entity.untern(db) {
        EntityData::Error(report) => WithError::error_sentinel(db, report),

        EntityData::LangItem(LangItem::Boolean)
        | EntityData::LangItem(LangItem::String)
        | EntityData::LangItem(LangItem::Int)
        | EntityData::LangItem(LangItem::Uint)
        | EntityData::LangItem(LangItem::False)
        | EntityData::LangItem(LangItem::Tuple(_))
        | EntityData::LangItem(LangItem::Debug)
        | EntityData::LangItem(LangItem::True) => {
            panic!("cannot invoke `signature` of `{:?}`", entity.untern(db))
        }

        EntityData::ItemName { .. } | EntityData::MemberName { .. } => {
            db.parsed_entity(entity).thunk.parse_signature(entity, db)
        }

        EntityData::InputFile { .. } => panic!(),
    }
}

crate fn unit_ty(db: &dyn LazyParsedEntityDatabase) -> ty::Ty<Declaration> {
    declaration_ty_named(
        &db,
        EntityData::LangItem(LangItem::Tuple(0)).intern(&db),
        ty::declaration::DeclaredPermKind::Own,
        ty::ReprKind::Direct,
        ty::Generics::empty(),
    )
}

crate fn declaration_ty_named(
    db: &dyn AsRef<DeclarationTables>,
    entity: Entity,
    perm: impl Intern<DeclarationTables, Key = ty::declaration::Perm>,
    repr: ty::ReprKind,
    generics: ty::Generics<Declaration>,
) -> ty::Ty<Declaration> {
    let kind = ty::BaseKind::Named(entity);
    let base = Declaration::intern_base_data(db, ty::BaseData { kind, generics });
    ty::Ty {
        perm: perm.intern(db),
        repr,
        base,
    }
}

//fn declaration_ty_from_ast_ty(
//    db: &impl ParserDatabase,
//    scope_entity: Entity,
//    ast_ty: &uhir::Type,
//) -> WithError<Ty<Declaration>> {
//    match db.resolve_name(scope_entity, *ast_ty.name) {
//        Some(entity) => WithError::ok(declaration_ty_named(db, entity, Generics::empty())),
//        None => {
//            let msg = format!("unknown type: {}", ast_ty.name.untern(db));
//            WithError::report_error(db, msg, ast_ty.name.span)
//        }
//    }
//}
