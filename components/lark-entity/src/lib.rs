#![feature(macro_at_most_once_rep)]
#![feature(specialization)]
#![feature(const_fn)]
#![feature(const_let)]

use debug::DebugWith;
use intern::Has;
use intern::Untern;
use parser::StringId;

indices::index_type! {
    pub struct Entity { .. }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum EntityData {
    InputFile { file: StringId },
    ItemName { base: Entity, id: StringId },
    MemberName { base: Entity, id: StringId },
}

intern::intern_tables! {
    pub struct EntityTables {
        struct EntityTablesData {
            item_ids: map(Entity, EntityData),
        }
    }
}

impl<Cx> DebugWith<Cx> for Entity
where
    Cx: Has<EntityTables>,
{
    fn fmt_with(&self, cx: &Cx, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let data = self.untern(cx);
        data.fmt_with(cx, fmt)
    }
}

impl<Cx> DebugWith<Cx> for EntityData
where
    Cx: Has<EntityTables>,
{
    fn fmt_with(&self, cx: &Cx, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EntityData::InputFile { file } => fmt
                .debug_struct("InputFile")
                .field("file", &file.debug_with(cx))
                .finish(),
            EntityData::ItemName { base, id } => fmt
                .debug_struct("ItemName")
                .field("base", &base.debug_with(cx))
                .field("id", &id.debug_with(cx))
                .finish(),
            EntityData::MemberName { base, id } => fmt
                .debug_struct("MemberName")
                .field("base", &base.debug_with(cx))
                .field("id", &id.debug_with(cx))
                .finish(),
        }
    }
}

impl Entity {
    pub fn input_file(self, db: &dyn Has<EntityTables>) -> StringId {
        match self.untern(db) {
            EntityData::InputFile { file } => file,
            EntityData::ItemName { base, id: _ } => base.input_file(db),
            EntityData::MemberName { base, id: _ } => base.input_file(db),
        }
    }
}
