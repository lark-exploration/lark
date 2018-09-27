use crate::ir::DefId;
use crate::ty::intern::Interners;
use crate::ty::*;
use std::fmt::Debug;

/// The `TyDebugContext` lets you customize how types
/// are represented during debugging. There are various
/// implementations that have different behaviors. The most
/// basic is to supply a `TyInterners`, which at least
/// lets us dereference the various indices. Better perhaps
/// is to give a unification table, which can canonicalize
/// inference variables. During testing, we use debug
/// context that can also handle def-ids.
crate trait TyDebugContext: Interners {
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

impl<T> Debug for DebugInWrapper<'me, Vec<T>>
where
    DebugInWrapper<'me, T>: Debug,
{
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt.debug_list()
            .entries(self.data.iter().map(|elem| elem.debug_in(self.context)))
            .finish()
    }
}

impl Debug for DebugInWrapper<'me, Ty> {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let base_data = self.context.interners().untern(*self.data);
        write!(fmt, "{:?}", base_data.debug_in(self.context))
    }
}

impl<T> Debug for DebugInWrapper<'me, Inferable<T>>
where
    DebugInWrapper<'me, T>: Debug,
{
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.data {
            Inferable::Infer(var) => self.context.write_infer_var(*var, self.context, fmt),

            Inferable::Bound(index) => self.context.write_bound(*index, self.context, fmt),

            Inferable::Known(k) => write!(fmt, "{:?}", k.debug_in(self.context)),
        }
    }
}

impl Debug for DebugInWrapper<'me, BaseData> {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let BaseData { kind, generics } = self.data;

        match kind {
            BaseKind::Named(name) => self.context.write_type_name(*name, self.context, fmt)?,

            BaseKind::Placeholder(placeholder) => {
                self.context
                    .write_placeholder(*placeholder, self.context, fmt)?
            }

            BaseKind::Error => write!(fmt, "(Error)")?,
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
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let perm_data = self.context.interners().untern(*self.data);
        write!(fmt, "{:?}", perm_data.debug_in(self.context))
    }
}

impl Debug for DebugInWrapper<'me, PermData> {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.data {
            PermData::Own => write!(fmt, "own"),
            PermData::Shared(region) => write!(fmt, "shared({:?})", region.debug_in(self.context)),
            PermData::Borrow(region) => write!(fmt, "borrow({:?})", region.debug_in(self.context)),
            PermData::Placeholder(placeholder) => {
                self.context
                    .write_placeholder(*placeholder, self.context, fmt)
            }
        }
    }
}

impl Debug for DebugInWrapper<'me, Region> {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.context.write_region(*self.data, self.context, fmt)
    }
}

impl Debug for DebugInWrapper<'me, Generic> {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.data {
            Generic::Ty(t) => write!(fmt, "{:?}", t.debug_in(self.context)),
        }
    }
}

impl Debug for DebugInWrapper<'me, Predicate> {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.data {
            Predicate::BaseBaseEq(base1, base2) => write!(
                fmt,
                "BaseBaseEq({:?}, {:?})",
                base1.debug_in(self.context),
                base2.debug_in(self.context),
            ),

            Predicate::BaseReprEq(base1, base2) => write!(
                fmt,
                "BaseReprEq({:?}, {:?})",
                base1.debug_in(self.context),
                base2.debug_in(self.context),
            ),

            Predicate::PermReprEq(perm1, perm2) => write!(
                fmt,
                "PermReprEq({:?}, {:?})",
                perm1.debug_in(self.context),
                perm2.debug_in(self.context),
            ),

            Predicate::RelateTypes {
                direction,
                ty1,
                ty2,
            } => write!(
                fmt,
                "RelateTypes({:?}, {:?}, {:?})",
                direction,
                ty1.debug_in(self.context),
                ty2.debug_in(self.context),
            ),

            Predicate::RelatePerms {
                direction,
                perm1,
                perm2,
            } => write!(
                fmt,
                "RelatePerms({:?}, {:?}, {:?})",
                direction,
                perm1.debug_in(self.context),
                perm2.debug_in(self.context),
            ),

            Predicate::RelateRegions {
                direction,
                region1,
                region2,
            } => write!(
                fmt,
                "RelateRegions({:?}, {:?}, {:?})",
                direction,
                region1.debug_in(self.context),
                region2.debug_in(self.context),
            ),

            Predicate::IntersectPerms {
                perm1,
                perm2,
                perm3,
            } => write!(
                fmt,
                "IntersectPerms({:?}, {:?}, {:?})",
                perm1.debug_in(self.context),
                perm2.debug_in(self.context),
                perm3.debug_in(self.context),
            ),

            Predicate::UnionRegions {
                region1,
                region2,
                region3,
            } => write!(
                fmt,
                "UnionRegions({:?}, {:?}, {:?})",
                region1.debug_in(self.context),
                region2.debug_in(self.context),
                region3.debug_in(self.context),
            ),
        }
    }
}
