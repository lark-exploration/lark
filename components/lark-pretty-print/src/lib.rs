use lark_entity::{Entity, EntityData, ItemKind, LangItem, MemberKind};
use lark_intern::Untern;
use lark_parser::ParserDatabase;
use lark_ty::declaration::{Declaration, DeclaredPermKind};
use lark_ty::full_inferred::{FullInferred, FullInferredTables};
use lark_ty::{BaseData, BaseKind, BoundVarOr, PermKind, Ty, TypeFamily};

pub trait PrettyPrintDatabase: ParserDatabase + AsRef<FullInferredTables> {}

pub trait PrettyPrint {
    fn pretty_print(&self, db: &impl PrettyPrintDatabase) -> String;
}

impl PrettyPrint for Ty<Declaration> {
    fn pretty_print(&self, db: &impl PrettyPrintDatabase) -> String {
        format!(
            "{}{}",
            match self.perm.untern(db) {
                DeclaredPermKind::Own => "",
            },
            match self.base.untern(db) {
                BoundVarOr::BoundVar(var) => format!("{:?}", var),
                BoundVarOr::Known(base_data) => base_data.pretty_print(db),
            }
        )
    }
}

impl PrettyPrint for Ty<FullInferred> {
    fn pretty_print(&self, db: &impl PrettyPrintDatabase) -> String {
        format!(
            "{}{}",
            match self.perm {
                PermKind::Own => "",
                PermKind::Share => "shared ",
                PermKind::Borrow => "borrowed ",
            },
            self.base.untern(db).pretty_print(db),
        )
    }
}

impl<T: TypeFamily> PrettyPrint for BaseData<T> {
    fn pretty_print(&self, db: &impl PrettyPrintDatabase) -> String {
        self.kind.pretty_print(db)
    }
}

impl PrettyPrint for Entity {
    fn pretty_print(&self, db: &impl PrettyPrintDatabase) -> String {
        match self.untern(db) {
            EntityData::LangItem(LangItem::Boolean) => "bool".into(),
            EntityData::LangItem(LangItem::Uint) => "uint".into(),
            EntityData::LangItem(LangItem::Int) => "int".into(),
            EntityData::LangItem(LangItem::Tuple(0)) => "void".into(),
            EntityData::LangItem(LangItem::Debug) => "<debug>".into(),
            EntityData::MemberName {
                kind: MemberKind::Field,
                ..
            } => {
                let field_ty = db.ty(*self).into_value();
                format!("{}", field_ty.pretty_print(db))
            }
            EntityData::MemberName {
                kind: MemberKind::Method,
                id,
                ..
            } => {
                let mut output_sig = "(".to_string();
                let mut first = true;

                let sig = db.signature(*self).value.unwrap();

                for input in sig.inputs.iter() {
                    if !first {
                        output_sig.push_str(", ");
                    } else {
                        first = false;
                    }

                    output_sig.push_str(&input.pretty_print(db));
                }

                output_sig.push_str(") -> ");
                output_sig.push_str(&sig.output.pretty_print(db));

                format!("{}{}", id.untern(db).to_string(), output_sig)
            }
            EntityData::ItemName {
                kind: ItemKind::Struct,
                id,
                ..
            } => format!("{}", id.untern(db)),
            EntityData::ItemName {
                kind: ItemKind::Function,
                id,
                ..
            } => {
                let mut output_sig = "(".to_string();
                let mut first = true;

                let sig = db.signature(*self).value.unwrap();

                for input in sig.inputs.iter() {
                    if !first {
                        output_sig.push_str(", ");
                    } else {
                        first = false;
                    }

                    output_sig.push_str(&input.pretty_print(db));
                }

                output_sig.push_str(") -> ");
                output_sig.push_str(&sig.output.pretty_print(db));

                format!("{}{}", id.untern(db).to_string(), output_sig)
            }
            x => format!("{:?}", x),
        }
    }
}

impl<T: TypeFamily> PrettyPrint for BaseKind<T> {
    fn pretty_print(&self, db: &impl PrettyPrintDatabase) -> String {
        match self {
            BaseKind::Named(entity) => entity.pretty_print(db),
            BaseKind::Placeholder(..) => "<placeholder>".into(),
            BaseKind::Error => "<error>".into(),
        }
    }
}
