use crate::ty;

pub struct Arenas<'arena> {
    crate type_data_arena: typed_arena::Arena<ty::TyData<'arena>>,
    crate kind_arena: typed_arena::Arena<ty::Kind<'arena>>,
}

