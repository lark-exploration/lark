use crate::ParserDatabase;

use debug::DebugWith;
use intern::{Intern, Untern};
use lark_entity::{Entity, EntityData, LangItem, MemberKind};
use lark_error::{ErrorReported, ErrorSentinel, WithError};
use lark_ty::{
    BaseData, BaseKind, BoundVar, Declaration, Erased, GenericDeclarations, GenericKind, Generics,
    Signature, Ty, TypeFamily,
};
use std::sync::Arc;

crate fn generic_declarations(
    db: &impl ParserDatabase,
    entity: Entity,
) -> WithError<Result<Arc<GenericDeclarations>, ErrorReported>> {
    match entity.untern(db) {
        EntityData::Error(report) => WithError::error_sentinel(db, report),

        EntityData::LangItem(LangItem::Boolean)
        | EntityData::LangItem(LangItem::String)
        | EntityData::LangItem(LangItem::Int)
        | EntityData::LangItem(LangItem::Uint)
        | EntityData::LangItem(LangItem::False)
        | EntityData::LangItem(LangItem::True)
        | EntityData::LangItem(LangItem::Debug) => {
            WithError::ok(Ok(GenericDeclarations::empty(None)))
        }

        EntityData::LangItem(LangItem::Tuple(arity)) => {
            if arity != 0 {
                unimplemented!("non-zero arity tuples");
            }
            WithError::ok(Ok(GenericDeclarations::empty(None)))
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

crate fn ty(db: &impl ParserDatabase, entity: Entity) -> WithError<Ty<Declaration>> {
    match entity.untern(db) {
        EntityData::Error(report) => WithError::error_sentinel(db, report),

        EntityData::LangItem(LangItem::Boolean)
        | EntityData::LangItem(LangItem::String)
        | EntityData::LangItem(LangItem::Int)
        | EntityData::LangItem(LangItem::Uint)
        | EntityData::LangItem(LangItem::Debug) => {
            WithError::ok(declaration_ty_named(db, entity, Generics::empty()))
        }

        EntityData::LangItem(LangItem::False) | EntityData::LangItem(LangItem::True) => {
            let boolean_entity = EntityData::LangItem(LangItem::Boolean).intern(db);
            ty(db, boolean_entity)
        }

        EntityData::LangItem(LangItem::Tuple(arity)) => {
            let generics: Generics<Declaration> = (0..arity)
                .map(|i| BoundVar::new(i))
                .map(|bv| Ty {
                    base: Declaration::intern_bound_var(db, bv),
                    perm: Declaration::own_perm(db),
                })
                .map(|ty| GenericKind::Ty(ty))
                .collect();
            WithError::ok(declaration_ty_named(db, entity, generics))
        }

        EntityData::ItemName { .. } => {
            unimplemented!()
            //let ast = db.uhir_of_entity(entity);
            //
            //match &ast.value {
            //    uhir::Entity::Struct(_) | uhir::Entity::Def(_) => {
            //        WithError::ok(declaration_ty_named(db, entity, Generics::empty()))
            //    }
            //}
        }

        EntityData::MemberName {
            kind: MemberKind::Field,
            ..
        } => {
            unimplemented!()
            //let field = db.uhir_of_field(entity);
            //declaration_ty_from_ast_ty(db, entity, &field.value.ty)
        }

        EntityData::MemberName {
            kind: MemberKind::Method,
            ..
        } => WithError::ok(declaration_ty_named(db, entity, Generics::empty())),

        EntityData::InputFile { .. } => panic!(
            "cannot get type of entity with data {:?}",
            entity.untern(db).debug_with(db),
        ),
    }
}

crate fn signature(
    _db: &impl ParserDatabase,
    _owner: Entity,
) -> WithError<Result<Signature<Declaration>, ErrorReported>> {
    unimplemented!()
    //let mut errors = vec![];
    //
    //match db.uhir_of_entity(owner).value {
    //    uhir::Entity::Struct(_) => panic!("asked for signature of a struct"),
    //
    //    uhir::Entity::Def(d) => {
    //        let inputs: Seq<_> = d
    //            .parameters
    //            .iter()
    //            .map(|p| {
    //                declaration_ty_from_ast_ty(db, owner, &p.ty).accumulate_errors_into(&mut errors)
    //            })
    //            .collect();
    //
    //        let output = match &d.ret {
    //            None => unit_ty(db),
    //            Some(ty) => {
    //                declaration_ty_from_ast_ty(db, owner, ty).accumulate_errors_into(&mut errors)
    //            }
    //        };
    //
    //        WithError {
    //            value: Ok(Signature { inputs, output }),
    //            errors,
    //        }
    //    }
    //}
}

fn unit_ty(db: &impl ParserDatabase) -> Ty<Declaration> {
    declaration_ty_named(
        db,
        EntityData::LangItem(LangItem::Tuple(0)).intern(db),
        Generics::empty(),
    )
}

fn declaration_ty_named(
    db: &impl ParserDatabase,
    entity: Entity,
    generics: Generics<Declaration>,
) -> Ty<Declaration> {
    let kind = BaseKind::Named(entity);
    let base = Declaration::intern_base_data(db, BaseData { kind, generics });
    Ty { perm: Erased, base }
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
