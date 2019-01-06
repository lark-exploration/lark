use crate::full_inference::analysis::Node;
use crate::full_inference::perm::Perm;
use crate::full_inference::perm::PermData;
use crate::full_inference::perm::PermVar;
use crate::full_inference::FullInferenceTables;
use datafrog::Iteration;
use datafrog::Relation;
use lark_collections::FxIndexMap;
use lark_intern::Intern;
use lark_intern::Untern;
use lark_ty::PermKind;

/// **Kind inference:** The role of *kind inference* is to decide, for
/// each permission variable `P`, whether it is "share" or "borrow" or
/// "owned". This is a flow-insensitive analysis -- it does not need
/// to consider the control-flow graph. Instead, it just looks at the
/// relations between permissions: if we have that `Pa: Pb` and `Pb`
/// is borrow (resp. own), then `Pa` must be borrow (resp. own).
crate struct KindInference {
    crate borrow: Relation<(Perm, ())>,
    crate owned: Relation<(Perm, ())>,
}

impl KindInference {
    crate fn new(
        tables: &impl AsRef<FullInferenceTables>,
        perm_less_base: &[(Perm, Perm, Node)],
        perm_less_if_base: &[(Perm, Perm, Perm, Node)],
    ) -> Self {
        let mut iteration = Iteration::new();

        // .decl perm_less(Pa, Pb)
        // perm_less(Pa, Pb) :- perm_less_base(Pa, Pb, _).
        let perm_less = iteration.variable::<(Perm, Perm)>("perm_less");
        perm_less.extend(perm_less_base.iter().map(|&(a, b, _n)| (a, b)));

        // .decl perm_condition(Pc, Pa, Pb)
        // perm_condition(Pc, Pa, Pb) :- perm_less_if_base(Pc, Pa, Pb, _).
        let perm_condition = iteration.variable::<(Perm, (Perm, Perm))>("perm_condition");
        perm_condition.extend(perm_less_if_base.iter().map(|&(c, a, b, _n)| (c, (a, b))));

        let perm_borrow: Perm = PermKind::Borrow.intern(tables);
        let perm_own: Perm = PermKind::Own.intern(tables);

        // .decl borrow(Pa)
        //
        // True if `Pa` is at least borrow.
        let borrow = iteration.variable::<(Perm, ())>("borrow");
        borrow.extend(std::iter::once((perm_borrow, ())).chain(std::iter::once((perm_own, ()))));

        // .decl owned(Pa)
        //
        // True if `Pa` is at least own.
        let owned = iteration.variable::<(Perm, ())>("borrow");
        owned.extend(std::iter::once((perm_own, ())));

        while iteration.changed() {
            // perm_less(Pa, Pb) :- perm_condition(Pc, Pa, Pb), borrow(Pc).
            perm_less.from_join(&perm_condition, &borrow, |&_p_c, &(p_a, p_b), &()| {
                (p_a, p_b)
            });

            // borrow(Pb) :- perm_less(Pa, Pb), borrow(Pa).
            borrow.from_join(&perm_less, &borrow, |&_p_a, &p_b, &()| (p_b, ()));

            // owned(Pb) :- perm_less(Pa, Pb), owned(Pa).
            owned.from_join(&perm_less, &owned, |&_p_a, &p_b, &()| (p_b, ()));
        }

        let borrow = borrow.complete();
        let owned = owned.complete();

        Self { borrow, owned }
    }

    /// Returns a map that specifies whether each `PermVar` is either
    /// borrow or owned. If there is no entry for a given `PermVar`,
    /// then it is shared.
    crate fn to_kind_map(
        &self,
        tables: &impl AsRef<FullInferenceTables>,
    ) -> FxIndexMap<PermVar, PermKind> {
        let mut set: FxIndexMap<PermVar, PermKind> = FxIndexMap::default();

        // Insert all things in `borrow` set into the map with `PermKind = Borrow`
        set.extend(
            self.borrow
                .elements
                .iter()
                .filter_map(|&(v, ())| match v.untern(tables) {
                    PermData::Inferred(v) => Some((v, PermKind::Borrow)),
                    PermData::Known(_) | PermData::Placeholder(_) => None,
                }),
        );

        // Insert all things in `own` set into the map with `PermKind = Own`,
        // in some cases overwriting things inserted previously with `PermKind = Borrow`
        set.extend(
            self.owned
                .elements
                .iter()
                .filter_map(|&(v, ())| match v.untern(tables) {
                    PermData::Inferred(v) => Some((v, PermKind::Own)),
                    PermData::Known(_) | PermData::Placeholder(_) => None,
                }),
        );

        set
    }
}
