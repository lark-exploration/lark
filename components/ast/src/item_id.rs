use crate::HasParserState;
use debug::DebugWith;
use intern::Has;
use intern::Untern;
use parser::StringId;
use std::sync::Arc;

indices::index_type! {
    pub struct ItemId { .. }
}

/// Eventually this would be a richer notion of path.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ItemIdData {
    pub input_file: StringId,
    pub path: Arc<Vec<StringId>>,
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
        fmt.debug_struct("ItemIdData")
            .field("input_file", &data.input_file.debug_with(cx))
            .field("path", &data.path.debug_with(cx))
            .finish()
    }
}
