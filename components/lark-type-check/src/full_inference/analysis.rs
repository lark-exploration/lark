use crate::full_inference::constraint::ConstraintAt;
use crate::full_inference::perm::PermVar;
use crate::full_inference::FullInference;
use crate::full_inference::FullInferenceTables;
use crate::full_inference::Perm;
use crate::results::TypeCheckResults;
use crate::HirLocation;
use crate::TypeCheckDatabase;
use lark_collections::{FxIndexMap, FxIndexSet, IndexVec, U32Index};
use lark_debug_derive::DebugWith;
use lark_entity::Entity;
use lark_error::Diagnostic;
use lark_hir as hir;
use lark_string::GlobalIdentifier;
use lark_ty::PermKind;
use lark_unify::UnificationTable;

mod builder;
mod dump;
mod initialization;
mod kind_inference;

use dump::DumpCx;
use initialization::Initialization;
use kind_inference::KindInference;

/// The "analysis IR" is a view onto a HIR fn body that adds a
/// control-flow graph as well as a number of tuples that are used
/// during safety analysis.  It's plausible that this struct -- or
/// some subset of it -- should be extracted into its own crate,
/// particularly if we wind up wanting to consume the HIR in
/// "control-flow graph form" (i.e., this IR could play a role similar
/// to MIR in Rust).
#[derive(Default)]
crate struct AnalysisIr {
    /// For each node, information about what it represents (the
    /// analysis itself doesn't care).
    crate node_datas: IndexVec<Node, HirLocation>,

    /// Map from `HirLocation` to nodes -- the index in the map is
    /// equal to the `Node`. Use `lookup_node` to access conveniently.
    reverse_node_datas: FxIndexMap<HirLocation, ()>,

    /// For each path, information about what it represents (the
    /// analysis itself doesn't care).
    crate path_datas: IndexVec<Path, PathData>,

    /// Edges in the control-flow graph.
    crate cfg_edge: Vec<(Node, Node)>,

    /// Contains pairs `(Path1, Path2)` where `Path1` is a "parent
    /// path" of `Path2` -- e.g., there would be a pair for `foo` and
    /// `foo.bar`. This contains *immediate* parents only, so there
    /// would NOT be a pair `(foo, foo.bar.baz)`.
    crate owner_path: Vec<(Path, Path)>,

    /// Paths that represent a "local slot" in the fn, either a
    /// user-declared variable or a temporary. These paths begin
    /// in an uninitialized state.
    crate local_path: Vec<Path>,

    crate imprecise_path: Vec<Path>,

    /// An "access" of the given path with the given permission takes place
    /// at the given node.
    crate access: Vec<(Perm, Path, Node)>,

    /// Indicates that the value of `Path` is overwritten at the given `Node`
    /// (e.g., `x = 5` overwrites `x`).
    crate overwritten: Vec<(Path, Node)>,

    /// Indicates that the value of `Path` is "traversed" -- i.e.,
    /// instantaneously accessed -- at the given node. This is used
    /// when a subpath is overwritten; so, for example, `x.y = 10`
    /// "traverses" `x`, and hence `x` cannot be moved.
    crate traverse: Vec<(Path, Node)>,

    /// Indicates that data with the given permission is "used" at the
    /// given node. For example, if you have `foo(x)`, then at the
    /// node for the call to `foo`, all permissions appearing in the
    /// type of `x` are "used".
    crate used: Vec<(Perm, Node)>,

    /// `Pa <= Pb` -- the permission `Pb` must at least permit `Pa`
    /// (written `Pb: Pa` in source)
    crate perm_less_base: Vec<(Perm, Perm, Node)>,

    /// `Pc[Pa <= Pb]` -- if `Pc` is borrow/own, then `Pb` must permit
    /// `Pa` (occurs during subtyping)
    crate perm_less_if_base: Vec<(Perm, Perm, Perm, Node)>,
}

crate struct AnalysisResults {
    crate perm_kinds: FxIndexMap<PermVar, PermKind>,
    crate errors: Vec<Diagnostic>,
}

impl AnalysisIr {
    crate fn new(
        fn_body: &hir::FnBody,
        results: &TypeCheckResults<FullInference>,
        constraints: &FxIndexSet<ConstraintAt>,
        unify: &mut UnificationTable<FullInferenceTables, hir::MetaIndex>,
    ) -> AnalysisIr {
        builder::AnalysisBuilder::analyze(fn_body, results, constraints, unify)
    }

    fn dump(&self, cx: &DumpCx<'_, impl TypeCheckDatabase>) {
        cx.dump_facts("node_datas", self.node_datas.iter_enumerated())
            .unwrap();
        cx.dump_facts("path_datas", self.path_datas.iter_enumerated())
            .unwrap();
        cx.dump_facts("cfg_edge", self.cfg_edge.iter()).unwrap();
        cx.dump_facts("owner_path", self.owner_path.iter()).unwrap();
        cx.dump_facts("local_path", self.local_path.iter()).unwrap();
        cx.dump_facts("imprecise_path", self.imprecise_path.iter())
            .unwrap();
        cx.dump_facts("access", self.access.iter()).unwrap();
        cx.dump_facts("overwritten", self.overwritten.iter())
            .unwrap();
        cx.dump_facts("traverse", self.traverse.iter()).unwrap();
        cx.dump_facts("used", self.used.iter()).unwrap();
        cx.dump_facts("perm_less_base", self.perm_less_base.iter())
            .unwrap();
        cx.dump_facts("perm_less_if_base", self.perm_less_if_base.iter())
            .unwrap();
    }

    crate fn infer(
        self,
        entity: Entity,
        db: &impl TypeCheckDatabase,
        fn_body: &hir::FnBody,
        tables: &impl AsRef<FullInferenceTables>,
    ) -> AnalysisResults {
        let cx = &DumpCx::new(db, fn_body, tables.as_ref(), entity);

        self.dump(cx);

        let kind_inference =
            KindInference::new(tables, &self.perm_less_base, &self.perm_less_if_base);

        let initialization = Initialization::new(cx, &self, &kind_inference);

        let perm_kinds = kind_inference.to_kind_map(tables);

        let mut errors = vec![];

        for &(node, ()) in initialization.error_move_of_imprecise_path.iter() {
            let span = match self.node_datas[node] {
                HirLocation::Expression(e) => fn_body.span(e),
                l => panic!("move of imprecise path at `{:?}`", l),
            };

            errors.push(Diagnostic::new(format!("move of imprecise path"), span));
        }

        for &(_path, node) in initialization.error_access_to_uninitialized_path.iter() {
            let span = match self.node_datas[node] {
                HirLocation::Expression(e) => fn_body.span(e),
                HirLocation::Place(p) => fn_body.span(p),
                l => panic!("move of imprecise path at `{:?}`", l),
            };

            errors.push(Diagnostic::new(
                format!("access to uninitialized path"),
                span,
            ));
        }

        AnalysisResults { perm_kinds, errors }
    }

    crate fn lookup_node(&self, data: impl Into<HirLocation>) -> Node {
        let data: HirLocation = data.into();
        Node::from_usize(match self.reverse_node_datas.get_full(&data) {
            Some((index, ..)) => index,
            None => panic!("no node created for `{:?}`", data),
        })
    }
}

lark_collections::index_type! {
    /// A node in the control-flow graph. Typically represents a HIR
    /// expression, but may represent other sorts of events.
    crate struct Node { .. }
}

lark_debug_with::debug_fallback_impl!(Node);

lark_collections::index_type! {
    /// A "path" is an expression that leads to a memory location. This is
    /// the unit that can be accessed. For example, `x` is a path as is
    /// `x.a.b`, but `foo()` is not (because that expression produces a
    /// value and doesn't reference a storage location). Paths are often
    /// called "lvalues" in C.
    ///
    /// Paths are not necessarily fully executable; they may be
    /// approximated to remove information that the analysis does not need
    /// -- for example, the expression `foo[bar]` may be a path, but we
    /// retain only `foo[]`.
    crate struct Path { .. }
}

lark_debug_with::debug_fallback_impl!(Path);

#[derive(Copy, Clone, Debug, DebugWith, Hash, PartialEq, Eq)]
crate enum PathData {
    /// A variable in the HIR like `x`
    Variable(hir::Variable),

    /// A static variable
    Entity(Entity),

    /// A temporary on the stack containing the result of some
    /// value-producing expression (e.g., `foo()`); such temporaries
    /// may be produced when you have something like:
    ///
    /// ```ignore
    /// let p = borrow foo().bar
    /// ```
    Temporary(hir::Expression),

    /// A path like `owner.name`
    Field { owner: Path, name: GlobalIdentifier },

    /// A path like `owner[_]`.
    Index { owner: Path },
}

impl PathData {
    fn owner(self) -> Option<Path> {
        match self {
            PathData::Entity(_) | PathData::Temporary(_) | PathData::Variable(_) => None,
            PathData::Field { owner, name: _ } | PathData::Index { owner } => Some(owner),
        }
    }

    fn precise(self, path_datas: &IndexVec<Path, PathData>) -> bool {
        match self {
            PathData::Entity(_) | PathData::Temporary(_) | PathData::Variable(_) => true,
            PathData::Field { owner, name: _ } => path_datas[owner].precise(path_datas),
            PathData::Index { owner: _ } => false,
        }
    }
}
