use parser::StringId;
use std::sync::Arc;

indices::index_type! {
    pub struct ItemId { .. }
}

/// Eventually this would be a richer notion of path.
#[derive(Clone, PartialEq, Eq, Hash)]
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
