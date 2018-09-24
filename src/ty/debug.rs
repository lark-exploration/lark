use crate::ir::DefId;
use crate::ty::intern::Interners;
use crate::ty::*;
use std::fmt::{self, Debug, Formatter};

crate trait TyDebugContext: Interners {
    fn write_region(
        &self,
        region: Region,
        _cx: &dyn TyDebugContext,
        fmt: &mut Formatter<'_>,
    ) -> fmt::Result {
        write!(fmt, "{:?}", region)
    }

    fn write_infer_var(
        &self,
        var: InferVar,
        _cx: &dyn TyDebugContext,
        fmt: &mut Formatter<'_>,
    ) -> fmt::Result {
        write!(fmt, "{:?}", var)
    }

    fn write_bound(
        &self,
        index: BoundIndex,
        _cx: &dyn TyDebugContext,
        fmt: &mut Formatter<'_>,
    ) -> fmt::Result {
        write!(fmt, "{:?}", index)
    }

    fn write_base_placeholder(
        &self,
        placeholder: Placeholder,
        _cx: &dyn TyDebugContext,
        fmt: &mut Formatter<'_>,
    ) -> fmt::Result {
        write!(fmt, "{:?}", placeholder)
    }

    fn write_perm_placeholder(
        &self,
        placeholder: Placeholder,
        _cx: &dyn TyDebugContext,
        fmt: &mut Formatter<'_>,
    ) -> fmt::Result {
        write!(fmt, "{:?}", placeholder)
    }

    fn write_type_name(
        &self,
        def_id: DefId,
        _cx: &dyn TyDebugContext,
        fmt: &mut Formatter<'_>,
    ) -> fmt::Result {
        write!(fmt, "DefId({})", def_id)
    }
}

crate struct DebugInWrapper<'me, T> {
    context: &'me dyn TyDebugContext,
    data: &'me T,
}

crate trait DebugIn: Sized {
    fn debug_in(&'a self, cx: &'a dyn TyDebugContext) -> DebugInWrapper<'a, Self> {
        DebugInWrapper {
            context: cx,
            data: self,
        }
    }
}

impl<T> DebugIn for T {}

impl Debug for DebugInWrapper<'me, Ty> {
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

impl Debug for DebugInWrapper<'me, Base> {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> fmt::Result {
        let base_data = self.context.interners().untern(*self.data);
        write!(fmt, "{:?}", base_data.debug_in(self.context))
    }
}

impl<T> Debug for DebugInWrapper<'me, Inferable<T>>
where
    DebugInWrapper<'me, T>: Debug,
{
    fn fmt(&self, fmt: &mut Formatter<'_>) -> fmt::Result {
        match self.data {
            Inferable::Infer { var } => self.context.write_infer_var(*var, self.context, fmt),

            Inferable::Bound { index } => self.context.write_bound(*index, self.context, fmt),

            Inferable::Known(k) => write!(fmt, "{:?}", k.debug_in(self.context)),
        }
    }
}

impl Debug for DebugInWrapper<'me, BaseData> {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> fmt::Result {
        let BaseData { kind, generics } = self.data;

        match kind {
            BaseKind::Named { name } => self.context.write_type_name(*name, self.context, fmt)?,

            BaseKind::Placeholder { placeholder } => {
                self.context
                    .write_base_placeholder(*placeholder, self.context, fmt)?
            }
        }

        let generics_data = self.context.interners().untern(*generics);
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

impl Debug for DebugInWrapper<'me, Perm> {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> fmt::Result {
        let perm_data = self.context.interners().untern(*self.data);
        write!(fmt, "{:?}", perm_data.debug_in(self.context))
    }
}

impl Debug for DebugInWrapper<'me, PermData> {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> fmt::Result {
        match self.data {
            PermData::Own => write!(fmt, "own"),
            PermData::Shared { region } => {
                write!(fmt, "shared({:?})", region.debug_in(self.context))
            }
            PermData::Borrow { region } => {
                write!(fmt, "borrow({:?})", region.debug_in(self.context))
            }
            PermData::Placeholder { placeholder } => {
                self.context
                    .write_perm_placeholder(*placeholder, self.context, fmt)
            }
        }
    }
}

impl Debug for DebugInWrapper<'me, Region> {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> fmt::Result {
        self.context.write_region(*self.data, self.context, fmt)
    }
}

impl Debug for DebugInWrapper<'me, Generic> {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> fmt::Result {
        match self.data {
            Generic::Ty(t) => write!(fmt, "{:?}", t.debug_in(self.context)),
        }
    }
}
