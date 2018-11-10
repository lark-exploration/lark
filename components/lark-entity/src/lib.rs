#![feature(macro_at_most_once_rep)]
#![feature(specialization)]
#![feature(const_fn)]
#![feature(const_let)]

use debug::{DebugWith, FmtWithSpecialized};
use intern::{Intern, Untern};
use lark_debug_derive::DebugWith;
use lark_error::{Diagnostic, ErrorSentinel};
use lark_string::global::GlobalIdentifier;

indices::index_type! {
    pub struct Entity { .. }
}

#[derive(Clone, Debug, DebugWith, PartialEq, Eq, Hash)]
pub enum EntityData {
    /// Indicates that fetching the entity somehow failed with an
    /// error (which has been separately reported).
    Error(Diagnostic),

    LangItem(LangItem),

    InputFile {
        file: GlobalIdentifier,
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
            | EntityData::Error(_) => true,
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

intern::intern_tables! {
    pub struct EntityTables {
        struct EntityTablesData {
            item_ids: map(Entity, EntityData),
        }
    }
}

debug::debug_fallback_impl!(Entity);

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
    pub fn input_file(self, db: &dyn AsRef<EntityTables>) -> Option<GlobalIdentifier> {
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
    fn error_sentinel(db: &DB, spans: &[Diagnostic]) -> Self {
        // Pick the first error arbitrarily
        EntityData::Error(spans[0].clone()).intern(db)
    }
}
