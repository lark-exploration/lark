use crate::ty::intern::TyArenas;

pub struct Arenas<'arena> {
    crate ty_arenas: TyArenas<'arena>,
}
