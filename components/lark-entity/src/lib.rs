#![feature(macro_at_most_once_rep)]
#![feature(specialization)]
#![feature(const_fn)]
#![feature(const_let)]

use debug::DebugWith;
use intern::Has;
use intern::Untern;
use parser::StringId;

indices::index_type! {
    pub struct ItemId { .. }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ItemIdData {
    InputFile { file: StringId },
    ItemName { base: ItemId, id: StringId },
    MemberName { base: ItemId, id: StringId },
}

intern::intern_tables! {
    pub struct ItemIdTables {
        struct ItemIdTablesData {
            item_ids: map(ItemId, ItemIdData),
        }
    }
}

impl<Cx> DebugWith<Cx> for ItemId
where
    Cx: Has<ItemIdTables>,
{
    fn fmt_with(&self, cx: &Cx, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let data = self.untern(cx);
        data.fmt_with(cx, fmt)
    }
}

impl<Cx> DebugWith<Cx> for ItemIdData
where
    Cx: Has<ItemIdTables>,
{
    fn fmt_with(&self, cx: &Cx, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ItemIdData::InputFile { file } => fmt
                .debug_struct("InputFile")
                .field("file", &file.debug_with(cx))
                .finish(),
            ItemIdData::ItemName { base, id } => fmt
                .debug_struct("ItemName")
                .field("base", &base.debug_with(cx))
                .field("id", &id.debug_with(cx))
                .finish(),
            ItemIdData::MemberName { base, id } => fmt
                .debug_struct("MemberName")
                .field("base", &base.debug_with(cx))
                .field("id", &id.debug_with(cx))
                .finish(),
        }
    }
}

impl ItemId {
    pub fn input_file(self, db: &dyn Has<ItemIdTables>) -> StringId {
        match self.untern(db) {
            ItemIdData::InputFile { file } => file,
            ItemIdData::ItemName { base, id: _ } => base.input_file(db),
            ItemIdData::MemberName { base, id: _ } => base.input_file(db),
        }
    }
}
