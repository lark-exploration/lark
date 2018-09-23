use crate::ir::DefId;
use crate::ty::intern::Interners;
use crate::ty::*;
use std::fmt::{self, Debug, Formatter};

crate trait TyDebugContext: Interners {
    fn write_region(&self, region: Region, fmt: &mut Formatter<'_>) -> fmt::Result {
        write!(fmt, "{:?}", region)
    }

    fn write_base_infer_var(&self, var: InferVar, fmt: &mut Formatter<'_>) -> fmt::Result {
        write!(fmt, "{:?}", var)
    }

    fn write_base_bound(&self, index: BoundIndex, fmt: &mut Formatter<'_>) -> fmt::Result {
        write!(fmt, "{:?}", index)
    }

    fn write_base_placeholder(
        &self,
        placeholder: Placeholder,
        fmt: &mut Formatter<'_>,
    ) -> fmt::Result {
        write!(fmt, "{:?}", placeholder)
    }

    fn write_perm_infer_var(&self, var: InferVar, fmt: &mut Formatter<'_>) -> fmt::Result {
        write!(fmt, "{:?}", var)
    }

    fn write_perm_bound(&self, index: BoundIndex, fmt: &mut Formatter<'_>) -> fmt::Result {
        write!(fmt, "{:?}", index)
    }

    fn write_perm_placeholder(
        &self,
        placeholder: Placeholder,
        fmt: &mut Formatter<'_>,
    ) -> fmt::Result {
        write!(fmt, "{:?}", placeholder)
    }

    fn write_type_name(&self, def_id: DefId, fmt: &mut Formatter<'_>) -> fmt::Result {
        write!(fmt, "DefId({})", def_id)
    }
}

crate struct DebugInWrapper<'me, C, T>
where
    C: TyDebugContext,
{
    context: &'me C,
    data: &'me T,
}

crate trait DebugIn: Sized {
    fn debug_in<C: TyDebugContext>(&'a self, cx: &'a C) -> DebugInWrapper<'a, C, Self> {
        DebugInWrapper {
            context: cx,
            data: self,
        }
    }
}

impl<T> DebugIn for T {}

impl<C> Debug for DebugInWrapper<'me, C, Ty>
where
    C: TyDebugContext,
{
    fn fmt(&self, fmt: &mut Formatter<'_>) -> fmt::Result {
        let Ty { perm, base } = self.data;
        write!(
            fmt,
            "{:?} {:?}",
            perm.debug_in(self.context),
            base.debug_in(self.context)
        )
    }
}

impl<C> Debug for DebugInWrapper<'me, C, Base>
where
    C: TyDebugContext,
{
    fn fmt(&self, fmt: &mut Formatter<'_>) -> fmt::Result {
        let base_data = self.context.untern(*self.data);
        write!(fmt, "{:?}", base_data.debug_in(self.context))
    }
}

impl<C> Debug for DebugInWrapper<'me, C, BaseData>
where
    C: TyDebugContext,
{
    fn fmt(&self, fmt: &mut Formatter<'_>) -> fmt::Result {
        let BaseData { kind, generics } = self.data;

        match kind {
            BaseKind::Named { name } => self.context.write_type_name(*name, fmt)?,

            BaseKind::Infer { var } => self.context.write_base_infer_var(*var, fmt)?,

            BaseKind::Bound { index } => self.context.write_base_bound(*index, fmt)?,

            BaseKind::Placeholder { placeholder } => {
                self.context.write_base_placeholder(*placeholder, fmt)?
            }
        }

        let generics_data = self.context.untern(*generics);
        if generics_data.is_not_empty() {
            write!(fmt, "<")?;
            for (index, generic) in generics_data.iter().enumerate() {
                if index > 0 {
                    write!(fmt, ", ")?;
                }
                write!(fmt, "{:?}", generic.debug_in(self.context))?;
            }
            write!(fmt, ">")?;
        }

        Ok(())
    }
}

impl<C> Debug for DebugInWrapper<'me, C, Perm>
where
    C: TyDebugContext,
{
    fn fmt(&self, fmt: &mut Formatter<'_>) -> fmt::Result {
        let perm_data = self.context.untern(*self.data);
        write!(fmt, "{:?}", perm_data.debug_in(self.context))
    }
}

impl<C> Debug for DebugInWrapper<'me, C, PermData>
where
    C: TyDebugContext,
{
    fn fmt(&self, fmt: &mut Formatter<'_>) -> fmt::Result {
        match self.data {
            PermData::Own => write!(fmt, "own"),
            PermData::Shared { region } => {
                write!(fmt, "shared({:?})", region.debug_in(self.context))
            }
            PermData::Borrow { region } => {
                write!(fmt, "borrow({:?})", region.debug_in(self.context))
            }
            PermData::Infer { var } => self.context.write_perm_infer_var(*var, fmt),
            PermData::Bound { index } => self.context.write_perm_bound(*index, fmt),
            PermData::Placeholder { index } => self.context.write_perm_placeholder(*index, fmt),
        }
    }
}

impl<C> Debug for DebugInWrapper<'me, C, Region>
where
    C: TyDebugContext,
{
    fn fmt(&self, fmt: &mut Formatter<'_>) -> fmt::Result {
        self.context.write_region(*self.data, fmt)
    }
}

impl<C> Debug for DebugInWrapper<'me, C, Generic>
where
    C: TyDebugContext,
{
    fn fmt(&self, fmt: &mut Formatter<'_>) -> fmt::Result {
        match self.data {
            Generic::Ty(t) => write!(fmt, "{:?}", t.debug_in(self.context)),
        }
    }
}
