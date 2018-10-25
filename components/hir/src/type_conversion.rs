use ast::ast as a;
use crate::error::or_sentinel;
use crate::error::ErrorReported;
use crate::error::WithError;
use crate::HirDatabase;
use debug::DebugWith;
use intern::Intern;
use intern::Untern;
use lark_entity::Entity;
use lark_entity::EntityData;
use lark_entity::LangItem;
use lark_entity::MemberKind;
use std::sync::Arc;
use ty::declaration::Declaration;
use ty::BaseData;
use ty::BaseKind;
use ty::BoundVar;
use ty::BoundVarOr;
use ty::Erased;
use ty::GenericDeclarations;
use ty::GenericKind;
use ty::Generics;
use ty::Signature;
use ty::Ty;
use ty::TypeFamily;

crate fn generic_declarations(
    db: &impl HirDatabase,
    entity: Entity,
) -> WithError<Result<Arc<ty::GenericDeclarations>, ErrorReported>> {
    let empty_declarations = |parent_item: Option<Entity>| {
        Arc::new(GenericDeclarations {
            parent_item,
            declarations: Default::default(),
        })
    };

    match entity.untern(db) {
        EntityData::Error => WithError::error_sentinel(db),

        EntityData::LangItem(LangItem::Boolean) => WithError::ok(Ok(empty_declarations(None))),

        EntityData::LangItem(LangItem::Tuple(arity)) => {
            if arity != 0 {
                unimplemented!("don't feel like dealing with tuples yet");
            }
            WithError::ok(Ok(empty_declarations(None)))
        }

        EntityData::ItemName { .. } => {
            let ast = or_sentinel!(db, db.ast_of_item(entity));

            // Eventually, items ought to be permitted to have generic types attached to them.
            match &*ast {
                a::Item::Struct(_) | a::Item::Def(_) => WithError::ok(Ok(empty_declarations(None))),
            }
        }

        EntityData::MemberName {
            base,
            kind: MemberKind::Field,
            id: _,
        } => WithError::ok(Ok(empty_declarations(Some(base)))),

        // Eventually, methods ought to be permitted to have generic types attached to them.
        EntityData::MemberName {
            base,
            kind: MemberKind::Method,
            id: _,
        } => WithError::ok(Ok(empty_declarations(Some(base)))),

        EntityData::InputFile { .. } => panic!(
            "cannot get generics of entity with data {:?}",
            entity.untern(db).debug_with(db),
        ),
    }
}

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

crate fn signature(
    db: &impl HirDatabase,
    owner: Entity,
) -> WithError<Result<ty::Signature<Declaration>, ErrorReported>> {
    let mut errors = vec![];

    match db.ast_of_item(owner) {
        Ok(ast) => match &*ast {
            a::Item::Struct(_) => panic!("asked for signature of a struct"),

            a::Item::Def(d) => {
                let inputs: Vec<_> = d
                    .parameters
                    .iter()
                    .map(|p| {
                        declaration_ty_from_ast_ty(db, owner, &p.ty)
                            .accumulate_errors_into(&mut errors)
                    })
                    .collect();

                let output = match &d.ret {
                    None => unit_ty(db),
                    Some(ty) => declaration_ty_from_ast_ty(db, owner, ty)
                        .accumulate_errors_into(&mut errors),
                };

                WithError {
                    value: Ok(Signature {
                        inputs: Arc::new(inputs),
                        output,
                    }),
                    errors,
                }
            }
        },

        Err(_parse_error) => WithError::error_sentinel(db),
    }
}

fn unit_ty(db: &impl HirDatabase) -> Ty<Declaration> {
    declaration_ty_named(
        db,
        EntityData::LangItem(LangItem::Tuple(0)).intern(db),
        Generics::empty(),
    )
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
