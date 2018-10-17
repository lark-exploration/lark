use crate::HasParserState;
use debug::DebugWith;
use intern::Has;
use intern::Untern;
use lark_debug_derive::DebugWith;
use parser::StringId;

indices::index_type! {
    pub struct ItemId { .. }
}

#[derive(Copy, Clone, Debug, DebugWith, PartialEq, Eq, Hash)]
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
    Cx: Has<ItemIdTables> + HasParserState,
{
    fn fmt_with(&self, cx: &Cx, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let data = self.untern(cx);
        data.fmt_with(cx, fmt)
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
