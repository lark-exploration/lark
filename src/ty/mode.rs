use crate::ir::DefId;
use crate::ty::context::TyContextData;
use crate::ty::{Generics, Mode, ModeKind, Region, Ty, TyName, TyKind};

impl TyContextData<'global> {
    /// Creates a new mode that corresponds to data with mode `mode`
    /// being shared for the region `region`.
    ///
    /// If `mode` is already shared, this just returns `mode`.
    crate fn share_mode(&self, region: Region, mode: Mode<'global>) -> (Region, Mode<'global>) {
        match mode.kind() {
            // Given `share(r1) M`, just return `share(r1) M`, as `r1: r`
            // must hold.
            ModeKind::Shared { region: region1, mode: _ } => (*region1, mode),

            // Given `borrow(r1)`, construct the composite mode
            // `share(r) borrow(r)`. Note that `r1: r` must hold.
            ModeKind::Borrow { .. } => {
                let borrow = self.intern.mode(ModeKind::Borrow { region });
                let shared = self.intern.mode(ModeKind::Shared { region, mode: borrow });
                (region, shared)
            }

            // Given `own`, return `share(r) own`.
            ModeKind::Owned => {
                let shared = self.intern.mode(ModeKind::Shared { region, mode });
                (region, shared)
            }
        }
    }

    crate fn share_generics(&self, region: Region, generics: Generics<'global>) -> Generics<'global> {
        unimplemented!()
    }

    crate fn share_ty(&self, region: Region, ty: Ty<'global>) -> Ty<'global> {
        match ty.kind() {
            TyKind::Mode { mode } => {
                let (region_sh, mode_sh) = self.share_mode(region, mode);
                if mode_sh == mode {
                    ty
                } else {
                    let generics_sh = self.share_generics(region_sh, ty.generics());
                    self.intern.ty(TyKind::Mode { mode: mode_sh }, generics_sh)
                }
            }

            TyKind::Named { name } => {
                // FIXME: We ought to lookup the variance or whatever
                // for this name. For now we just assume everything is
                // an `owned` type parameter.
                let generics_sh = self.share_generics(region, ty.generics());
                let name_ty = self.intern.ty(TyKind::Named { name }, generics_sh);

                if self.is_value_type(name) {
                    name_ty
                } else {
                    let (_, share_mode) = self.share_mode(region, self.common.own_mode);
                    self.intern.ty_in_mode(name_ty, share_mode)
                }
            }

            TyKind::Bound { .. } | TyKind::Infer { .. } | TyKind::Placeholder { .. } => {
                assert!(ty.generics().is_empty()); // no HKT yet
                let (_, share_mode) = self.share_mode(region, self.common.own_mode);
                self.intern.ty_in_mode(ty, share_mode)
            }
        }
    }

    fn is_value_type(&self, name: TyName) -> bool {
        unimplemented!()
    }
}
