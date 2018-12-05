use crate::declaration::{Declaration, DeclarationTables, DeclaredPermKind};
use crate::full_inferred::{FullInferred, FullInferredTables};
use crate::{BaseData, BaseKind, BoundVarOr, PermKind, Ty, TypeFamily};
use lark_entity::{EntityData, EntityTables, ItemKind, LangItem};
use lark_intern::Untern;

pub trait PrettyPrinter:
    AsRef<DeclarationTables>
    + AsRef<FullInferredTables>
    + AsRef<EntityTables>
    + AsRef<lark_string::GlobalIdentifierTables>
{
}

impl Ty<Declaration> {
    pub fn pretty_print(&self, db: &impl PrettyPrinter) -> String {
        format!(
            "{}{}",
            match self.perm.untern(db) {
                DeclaredPermKind::Own => "",
            },
            match self.base.untern(db) {
                BoundVarOr::BoundVar(var) => format!("{:?}", var),
                BoundVarOr::Known(base_data) => match base_data {
                    BaseData { kind, .. } => pretty_print_kind(kind, db),
                },
            }
        )
    }
}

impl Ty<FullInferred> {
    pub fn pretty_print(&self, db: &impl PrettyPrinter) -> String {
        format!(
            "{}{}",
            match self.perm {
                PermKind::Own => "",
                PermKind::Share => "shared ",
                PermKind::Borrow => "borrowed ",
            },
            match self.base.untern(db) {
                BaseData { kind, .. } => pretty_print_kind(kind, db),
            }
        )
    }
}

fn pretty_print_kind<T: TypeFamily>(kind: BaseKind<T>, db: &impl PrettyPrinter) -> String {
    match kind {
        BaseKind::Named(entity) => match entity.untern(db) {
            EntityData::LangItem(LangItem::Boolean) => "bool".into(),
            EntityData::LangItem(LangItem::Uint) => "uint".into(),
            EntityData::LangItem(LangItem::Tuple(0)) => "void".into(),
            EntityData::LangItem(LangItem::Debug) => "<internal debug>".into(),
            EntityData::ItemName {
                kind: ItemKind::Struct,
                id,
                ..
            } => id.untern(db).to_string(),
            x => format!("{:?}", x),
        },
        BaseKind::Placeholder(..) => "<placeholder>".into(),
        BaseKind::Error => "<error>".into(),
    }
}
