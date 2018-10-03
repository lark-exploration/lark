use crate::debug::DebugWith;
use crate::ir::DefId;
use crate::ty::intern::Interners;
use crate::ty::*;
use crate::unify::InferVar;

/// The `TyDebugContext` lets you customize how types
/// are represented during debugging. There are various
/// implementations that have different behaviors. The most
/// basic is to supply a `TyInterners`, which at least
/// lets us dereference the various indices. Better perhaps
/// is to give a unification table, which can canonicalize
/// inference variables. During testing, we use debug
/// context that can also handle def-ids.
crate trait TyDebugContext: Interners + 'static {
    fn write_region(
        &self,
        region: Region,
        _context: &dyn TyDebugContext,
        fmt: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        write!(fmt, "{:?}", region)
    }

    fn write_infer_var(
        &self,
        var: InferVar,
        _context: &dyn TyDebugContext,
        fmt: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        write!(fmt, "{:?}", var)
    }

    fn write_bound(
        &self,
        index: BoundIndex,
        _context: &dyn TyDebugContext,
        fmt: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        write!(fmt, "{:?}", index)
    }

    fn write_placeholder(
        &self,
        placeholder: Placeholder,
        _context: &dyn TyDebugContext,
        fmt: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        write!(fmt, "{:?}", placeholder)
    }

    fn write_type_name(
        &self,
        def_id: DefId,
        _context: &dyn TyDebugContext,
        fmt: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        write!(fmt, "DefId({})", def_id)
    }
}

impl DebugWith<dyn TyDebugContext> for Ty {
    fn fmt_with(
        &self,
        cx: &dyn TyDebugContext,
        fmt: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        let Ty { perm, base } = *self;
        write!(fmt, "{:?} {:?}", perm.debug_with(cx), base.debug_with(cx))
    }
}

impl DebugWith<dyn TyDebugContext> for Base {
    fn fmt_with(
        &self,
        cx: &dyn TyDebugContext,
        fmt: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        let base_data = cx.interners().untern(*self);
        write!(fmt, "{:?}", base_data.debug_with(cx))
    }
}

impl<T> DebugWith<dyn TyDebugContext> for Inferable<T>
where
    T: DebugWith<dyn TyDebugContext>,
{
    fn fmt_with(
        &self,
        cx: &dyn TyDebugContext,
        fmt: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            Inferable::Infer(var) => cx.write_infer_var(*var, cx, fmt),

            Inferable::Bound(index) => cx.write_bound(*index, cx, fmt),

            Inferable::Known(k) => write!(fmt, "{:?}", k.debug_with(cx)),
        }
    }
}

impl DebugWith<dyn TyDebugContext> for BaseData {
    fn fmt_with(
        &self,
        cx: &dyn TyDebugContext,
        fmt: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        let BaseData { kind, generics } = self;

        match kind {
            BaseKind::Named(name) => cx.write_type_name(*name, cx, fmt)?,

            BaseKind::Placeholder(placeholder) => cx.write_placeholder(*placeholder, cx, fmt)?,
        }

        if generics.is_not_empty() {
            write!(fmt, "{}", generics.debug_with(cx))?;
        }

        Ok(())
    }
}

impl DebugWith<dyn TyDebugContext> for Generics {
    fn fmt_with(
        &self,
        cx: &dyn TyDebugContext,
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

impl DebugWith<dyn TyDebugContext> for Perm {
    fn fmt_with(
        &self,
        cx: &dyn TyDebugContext,
        fmt: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        let perm_data = cx.interners().untern(*self);
        write!(fmt, "{:?}", perm_data.debug_with(cx))
    }
}

impl DebugWith<dyn TyDebugContext> for PermData {
    fn fmt_with(
        &self,
        cx: &dyn TyDebugContext,
        fmt: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            PermData::Own => write!(fmt, "own"),
            PermData::Shared(region) => write!(fmt, "shared({:?})", region.debug_with(cx)),
            PermData::Borrow(region) => write!(fmt, "borrow({:?})", region.debug_with(cx)),
            PermData::Placeholder(placeholder) => cx.write_placeholder(*placeholder, cx, fmt),
        }
    }
}

impl DebugWith<dyn TyDebugContext> for Region {
    fn fmt_with(
        &self,
        cx: &dyn TyDebugContext,
        fmt: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        cx.write_region(*self, cx, fmt)
    }
}

impl DebugWith<dyn TyDebugContext> for Generic {
    fn fmt_with(
        &self,
        cx: &dyn TyDebugContext,
        fmt: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            Generic::Ty(t) => write!(fmt, "{:?}", t.debug_with(cx)),
        }
    }
}

impl DebugWith<dyn TyDebugContext> for Predicate {
    fn fmt_with(
        &self,
        cx: &dyn TyDebugContext,
        fmt: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            Predicate::BaseBaseEq(base1, base2) => write!(
                fmt,
                "BaseBaseEq({:?}, {:?})",
                base1.debug_with(cx),
                base2.debug_with(cx),
            ),

            Predicate::BaseReprEq(base1, base2) => write!(
                fmt,
                "BaseReprEq({:?}, {:?})",
                base1.debug_with(cx),
                base2.debug_with(cx),
            ),

            Predicate::PermReprEq(perm1, perm2) => write!(
                fmt,
                "PermReprEq({:?}, {:?})",
                perm1.debug_with(cx),
                perm2.debug_with(cx),
            ),

            Predicate::RelateTypes {
                direction,
                ty1,
                ty2,
            } => write!(
                fmt,
                "RelateTypes({:?}, {:?}, {:?})",
                direction,
                ty1.debug_with(cx),
                ty2.debug_with(cx),
            ),

            Predicate::RelatePerms {
                direction,
                perm1,
                perm2,
            } => write!(
                fmt,
                "RelatePerms({:?}, {:?}, {:?})",
                direction,
                perm1.debug_with(cx),
                perm2.debug_with(cx),
            ),

            Predicate::RelateRegions {
                direction,
                region1,
                region2,
            } => write!(
                fmt,
                "RelateRegions({:?}, {:?}, {:?})",
                direction,
                region1.debug_with(cx),
                region2.debug_with(cx),
            ),

            Predicate::IntersectPerms {
                perm1,
                perm2,
                perm3,
            } => write!(
                fmt,
                "IntersectPerms({:?}, {:?}, {:?})",
                perm1.debug_with(cx),
                perm2.debug_with(cx),
                perm3.debug_with(cx),
            ),

            Predicate::UnionRegions {
                region1,
                region2,
                region3,
            } => write!(
                fmt,
                "UnionRegions({:?}, {:?}, {:?})",
                region1.debug_with(cx),
                region2.debug_with(cx),
                region3.debug_with(cx),
            ),
        }
    }
}
