#![cfg(disabled)]

use crate::debug::DebugWith;
use crate::ir::DefId;
use crate::ty::*;
use unify::InferVar;

impl<Cx: ?Sized, F: TypeFamily> DebugWith<Cx> for Ty<F> {
    fn fmt_with(
        &self,
        cx: &dyn TyDebugContext<F>,
        fmt: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        let Ty { perm, base } = *self;
        write!(fmt, "{:?} {:?}", perm.debug_with(cx), base.debug_with(cx))
    }
}

impl<F: TypeFamily> DebugWith<dyn TyDebugContext<F>> for BaseData<F> {
    fn fmt_with(
        &self,
        cx: &dyn TyDebugContext<F>,
        fmt: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        let BaseData { kind, generics } = self;

        match kind {
            BaseKind::Named(name) => cx.write_type_name(*name, cx, fmt)?,
            BaseKind::InferVar(infer_var) => cx.write_infer_var(*infer_var, cx, fmt)?,
            BaseKind::Placeholder(placeholder) => cx.write_placeholder(*placeholder, cx, fmt)?,
        }

        if generics.is_not_empty() {
            write!(fmt, "{}", generics.debug_with(cx))?;
        }

        Ok(())
    }
}

impl<F: TypeFamily> DebugWith<dyn TyDebugContext<F>> for Generics<F> {
    fn fmt_with(
        &self,
        cx: &dyn TyDebugContext<F>,
        fmt: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        write!(fmt, "<")?;
        for (index, generic) in self.iter().enumerate() {
            if index > 0 {
                write!(fmt, ", ")?;
            }
            write!(fmt, "{:?}", generic.debug_with(cx))?;
        }
        write!(fmt, ">")?;
        Ok(())
    }
}

impl<F: TypeFamily> DebugWith<dyn TyDebugContext<F>> for Generic<F> {
    fn fmt_with(
        &self,
        cx: &dyn TyDebugContext<F>,
        fmt: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            Generic::Ty(t) => write!(fmt, "{:?}", t.debug_with(cx)),
        }
    }
}

impl<F: TypeFamily> DebugWith<dyn TyDebugContext<F>> for Erased {
    fn fmt_with(
        &self,
        cx: &dyn TyDebugContext<F>,
        fmt: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        Ok(())
    }
}
