#![feature(specialization)]
#![feature(const_mut_refs)]

use lark_debug_derive::DebugWith;
use lark_debug_with::{DebugWith, FmtWithSpecialized};
use lark_error::{ErrorReported, ErrorSentinel};
use lark_intern::{Intern, Untern};
use lark_span::FileName;
use lark_string::{GlobalIdentifier, GlobalIdentifierTables};
use std::path::PathBuf;

lark_collections::index_type! {
    pub struct Entity { .. }
}

impl Entity {
    /// When we are dumping debug information about an entity, this
    /// method gives the directory (a relative path) where such files
    /// should be stored.
    pub fn dump_dir(
        &self,
        db: &(impl AsRef<EntityTables> + AsRef<GlobalIdentifierTables>),
    ) -> PathBuf {
        match self.untern(db) {
            EntityData::Error(err) => {
                let mut dir = PathBuf::new();
                dir.push("error");
                dir.push(format!("{}", err.span().file().id.untern(db)));
                dir
            }

            EntityData::LangItem(lang_item) => {
                let mut dir = PathBuf::new();
                dir.push(format!("{:?}", lang_item));
                dir
            }

            EntityData::InputFile { file } => {
                let mut dir = PathBuf::new();
                dir.push(format!("{}", file.untern(db)));
                dir
            }

            EntityData::ItemName { base, kind, id } => {
                let mut dir = base.dump_dir(db);
                dir.push(format!("{:?}", kind));
                dir.push(format!("{}", id.untern(db)));
                dir
            }

            EntityData::MemberName { base, kind, id } => {
                let mut dir = base.dump_dir(db);
                dir.push(format!("{:?}", kind));
                dir.push(format!("{}", id.untern(db)));
                dir
            }
        }
    }
}

#[derive(Clone, Debug, DebugWith, PartialEq, Eq, Hash)]
pub enum EntityData {
    /// Indicates that fetching the entity somehow failed with an
    /// error (which has been separately reported).
    Error(ErrorReported),

    LangItem(LangItem),

    InputFile {
        file: FileName,
    },
    ItemName {
        base: Entity,
        kind: ItemKind,
        id: GlobalIdentifier,
    },
    MemberName {
        base: Entity,
        kind: MemberKind,
        id: GlobalIdentifier,
    },
}

impl EntityData {
    /// Returns the parent entity, if any. This will be `Some` for
    /// items, members, etc.
    pub fn parent(&self) -> Option<Entity> {
        match self {
            EntityData::Error(_) | EntityData::LangItem(_) | EntityData::InputFile { .. } => None,
            EntityData::ItemName { base, .. } | EntityData::MemberName { base, .. } => Some(*base),
        }
    }

    /// Returns the file in which this entity is located (if
    /// any). This is none for lang items, errors.
    pub fn file_name(&self, db: &dyn AsRef<EntityTables>) -> Option<FileName> {
        match self {
            EntityData::InputFile { file } => Some(*file),
            _ => match self.parent() {
                None => None,
                Some(base) => base.untern(db).file_name(db),
            },
        }
    }

    /// Gives a little information about the name/kind of this entity,
    /// without dumping the whole tree. Meant for debugging.
    pub fn relative_name(self, db: &impl AsRef<GlobalIdentifierTables>) -> String {
        match self {
            EntityData::Error(_) => String::from("<error>"),
            EntityData::LangItem(li) => format!("{:?}", li),
            EntityData::InputFile { file } => format!("InputFile({})", file.untern(db)),
            EntityData::ItemName { id, .. } => format!("ItemName({})", id.untern(db)),
            EntityData::MemberName { id, .. } => format!("MemberName({})", id.untern(db)),
        }
    }

    /// True if this entity represents a value that the user could
    /// store into a variable (or might, in the case of error
    /// entities).
    pub fn is_value(&self) -> bool {
        match self {
            EntityData::InputFile { .. }
            | EntityData::ItemName {
                kind: ItemKind::Struct,
                ..
            }
            | EntityData::LangItem(LangItem::Int)
            | EntityData::LangItem(LangItem::Tuple(_))
            | EntityData::LangItem(LangItem::String)
            | EntityData::LangItem(LangItem::Uint)
            | EntityData::LangItem(LangItem::Boolean) => false,

            EntityData::ItemName {
                kind: ItemKind::Function,
                ..
            }
            | EntityData::MemberName {
                kind: MemberKind::Method,
                ..
            }
            | EntityData::MemberName {
                kind: MemberKind::Field,
                ..
            }
            | EntityData::LangItem(LangItem::True)
            | EntityData::LangItem(LangItem::False)
            | EntityData::LangItem(LangItem::Debug)
            | EntityData::Error(_) => true,
        }
    }

    /// True if this entity has a fn body associated with it.
    pub fn has_fn_body(&self) -> bool {
        match self {
            EntityData::InputFile { .. }
            | EntityData::ItemName {
                kind: ItemKind::Struct,
                ..
            }
            | EntityData::MemberName {
                kind: MemberKind::Field,
                ..
            }
            | EntityData::LangItem(_)
            | EntityData::Error(_) => false,

            EntityData::ItemName {
                kind: ItemKind::Function,
                ..
            }
            | EntityData::MemberName {
                kind: MemberKind::Method,
                ..
            } => true,
        }
    }
}

/// Struct definitions that are built-in to Lark.
///
/// Eventually, I would like these to be structs declared in some kind
/// of libcore -- though I'm not sure how tuple would work there.
#[derive(Copy, Clone, Debug, DebugWith, PartialEq, Eq, Hash)]
pub enum LangItem {
    Boolean,
    Int,
    Uint,
    Tuple(usize),
    String,
    True,
    False,
    Debug,
}

#[derive(Copy, Clone, Debug, DebugWith, PartialEq, Eq, Hash)]
pub enum ItemKind {
    Struct,
    Function,
}

#[derive(Copy, Clone, Debug, DebugWith, PartialEq, Eq, Hash)]
pub enum MemberKind {
    Field,
    Method,
}

lark_intern::intern_tables! {
    pub struct EntityTables {
        struct EntityTablesData {
            item_ids: map(Entity, EntityData),
        }
    }
}

lark_debug_with::debug_fallback_impl!(Entity);

impl<Cx> FmtWithSpecialized<Cx> for Entity
where
    Cx: AsRef<EntityTables>,
{
    fn fmt_with_specialized(&self, cx: &Cx, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let data = self.untern(cx);
        data.fmt_with(cx, fmt)
    }
}

impl Entity {
    /// The input file in which an entity appears (if any).
    pub fn input_file(self, db: &dyn AsRef<EntityTables>) -> Option<FileName> {
        match self.untern(db) {
            EntityData::LangItem(_) => None,
            EntityData::InputFile { file } => Some(file),
            EntityData::ItemName { base, .. } => base.input_file(db),
            EntityData::MemberName { base, .. } => base.input_file(db),
            EntityData::Error(_span) => {
                // FIXME we could recover a file here
                None
            }
        }
    }
}

impl<DB> ErrorSentinel<&DB> for Entity
where
    DB: AsRef<EntityTables>,
{
    fn error_sentinel(db: &DB, report: ErrorReported) -> Self {
        EntityData::Error(report).intern(db)
    }
}
