use ast::ast as a;
use crate::HirDatabase;
use debug::DebugWith;
use intern::Intern;
use intern::Untern;
use lark_entity::Entity;
use lark_entity::EntityData;
use lark_entity::LangItem;
use lark_entity::MemberKind;
use lark_error::or_return_sentinel;
use lark_error::ErrorReported;
use lark_error::ErrorSentinel;
use lark_error::WithError;
use lark_ty::declaration::Declaration;
use lark_ty::BaseData;
use lark_ty::BaseKind;
use lark_ty::BoundVar;
use lark_ty::Erased;
use lark_ty::GenericDeclarations;
use lark_ty::GenericKind;
use lark_ty::Generics;
use lark_ty::Signature;
use lark_ty::Ty;
use lark_ty::TypeFamily;
use std::sync::Arc;

crate fn generic_declarations(
    db: &impl HirDatabase,
    entity: Entity,
) -> WithError<Result<Arc<GenericDeclarations>, ErrorReported>> {
    let empty_declarations = |parent_item: Option<Entity>| {
        Arc::new(GenericDeclarations {
            parent_item,
            declarations: Default::default(),
        })
    };

    match entity.untern(db) {
        EntityData::Error(report) => WithError::error_sentinel(db, report),

        EntityData::LangItem(LangItem::Boolean)
        | EntityData::LangItem(LangItem::String)
        | EntityData::LangItem(LangItem::Int)
        | EntityData::LangItem(LangItem::Uint) => WithError::ok(Ok(empty_declarations(None))),

        EntityData::LangItem(LangItem::Tuple(arity)) => {
            if arity != 0 {
                unimplemented!("non-zero arity tuples");
            }
            WithError::ok(Ok(empty_declarations(None)))
        }

        EntityData::ItemName { .. } => {
            let ast = or_return_sentinel!(db, db.ast_of_item(entity));

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

crate fn ty(db: &impl HirDatabase, entity: Entity) -> WithError<Ty<Declaration>> {
    match entity.untern(db) {
        EntityData::Error(report) => WithError::error_sentinel(db, report),

        EntityData::LangItem(LangItem::Boolean)
        | EntityData::LangItem(LangItem::String)
        | EntityData::LangItem(LangItem::Int)
        | EntityData::LangItem(LangItem::Uint) => {
            WithError::ok(declaration_ty_named(db, entity, Generics::empty()))
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
            let ast = or_return_sentinel!(db, db.ast_of_item(entity));

            match &*ast {
                a::Item::Struct(_) | a::Item::Def(_) => {
                    WithError::ok(declaration_ty_named(db, entity, Generics::empty()))
                }
            }
        }

        EntityData::MemberName {
            kind: MemberKind::Field,
            ..
        } => {
            let field = or_return_sentinel!(db, db.ast_of_field(entity));
            declaration_ty_from_ast_ty(db, entity, &field.ty)
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
    db: &impl HirDatabase,
    owner: Entity,
) -> WithError<Result<Signature<Declaration>, ErrorReported>> {
    let mut errors = vec![];

    match &*or_return_sentinel!(db, db.ast_of_item(owner)) {
        a::Item::Struct(_) => panic!("asked for signature of a struct"),

        a::Item::Def(d) => {
            let inputs: Vec<_> = d
                .parameters
                .iter()
                .map(|p| {
                    declaration_ty_from_ast_ty(db, owner, &p.ty).accumulate_errors_into(&mut errors)
                })
                .collect();

            let output = match &d.ret {
                None => unit_ty(db),
                Some(ty) => {
                    declaration_ty_from_ast_ty(db, owner, ty).accumulate_errors_into(&mut errors)
                }
            };

            WithError {
                value: Ok(Signature {
                    inputs: Arc::new(inputs),
                    output,
                }),
                errors,
            }
        }
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
        None => {
            let msg = format!("unknown type: {}", db.untern_string(ast_ty.name.0));
            WithError::report_error(db, msg, ast_ty.name.1)
        }
    }
}
