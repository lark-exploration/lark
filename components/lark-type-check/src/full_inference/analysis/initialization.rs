use crate::full_inference::analysis::dump::DumpCx;
use crate::full_inference::analysis::kind_inference::KindInference;
use crate::full_inference::analysis::AnalysisIr;
use crate::full_inference::analysis::Node;
use crate::full_inference::analysis::Path;
use crate::full_inference::Perm;
use crate::HirLocation;
use crate::TypeCheckDatabase;
use datafrog::Iteration;
use datafrog::Relation;
use datafrog::RelationLeaper;

crate struct Initialization {
    crate error_move_of_imprecise_path: Relation<(Node, ())>,
    crate error_access_to_uninitialized_path: Relation<(Path, Node)>,
}

impl Initialization {
    /// Executes the **initialization** analysis.
    ///
    /// TL;DR:
    ///
    /// - We track the **uninitialized paths** at every point
    ///   - Each local variable and temporary is **uninitialized** on entry
    ///   - But we generate `overwritten` facts for parameters at the entry point
    /// - We propagate `uninitialized(Path)` facts across edges, unless there exists an
    ///   **overwritten(PathParent)** fact for some path `PathParent` where `PathParent` owns `Path`.
    ///   - e.g., if `a.b` is uninitialized on entry to point P1, that is propagated to P2
    ///   - but if there is an assignment of `a` at P2, then this fact is not propagated to P3
    /// - When we see a **move** of some path `Path`:
    ///   - If `Path` is **imprecise**, that is an error.
    ///   - Otherwise, we add the path `Path` to the **uninitialized** set in successors.
    /// - When we see an access to some path `Path`:
    ///   - if any parent or child path of `Path` is uninitialized, that is an error.
    ///   - so e.g. accessing `a.b` when `a`, `a.b`, or `a.b.c` is uninitialized is an error.
    ///
    /// Why track **uninitialized** and not **initialized**? Easier,
    /// because we can do **union** on joins.
    crate fn new(
        cx: &DumpCx<'_, impl TypeCheckDatabase>,
        analysis_ir: &AnalysisIr,
        kind_inference: &KindInference,
    ) -> Self {
        ///////////////////////////////////////////////////////////////////////////
        // Round 0: Compute `transitive_owner_path`

        let owner_path: Relation<_> = analysis_ir.owner_path.iter().collect();

        // .decl transitive_owner_path(Path1:path, Path2:node)
        //
        // Transitive version of `owner_path`.
        let transitive_owner_path: Relation<(Path, Path)> = {
            let mut iteration = Iteration::new();

            // .decl transitive_owner_path(Path, Path)
            let transitive_owner_path = iteration.variable::<(Path, Path)>("transitive_owner_path");

            // transitive_owner_path(Path1, Path2) :-
            //   owner_path(Path1, Path2).
            transitive_owner_path.insert(owner_path.clone());

            while iteration.changed() {
                // transitive_owner_path(Path1, Path3) :-
                //   transitive_owner_path(Path1, Path2),
                //   owner_path(Path2, Path3).
                transitive_owner_path.from_leapjoin(
                    &transitive_owner_path,
                    owner_path.extend_with(|&(path1, _)| path1),
                    |&(path1, _), &path3| (path1, path3),
                );
            }

            transitive_owner_path.complete()
        };

        ///////////////////////////////////////////////////////////////////////////
        // Round 1: Compute `transitive_overwritten`

        let transitive_overwritten = {
            let mut iteration = Iteration::new();

            // .decl transitive_overwritten(Path, Node)
            let transitive_overwritten =
                iteration.variable::<(Path, Node)>("transitive_overwritten");

            // transitive_overwritten(Path, Node) :- overwritten(Path, Node).
            transitive_overwritten.extend(&analysis_ir.overwritten);

            while iteration.changed() {
                // transitive_overwritten(PathChild, Node) :-
                //   transitive_overwritten(PathParent, Node),
                //   owner_path(PathParent, PathChild).
                transitive_overwritten.from_leapjoin(
                    &transitive_overwritten,
                    owner_path.extend_with(|&(path_parent, _)| path_parent),
                    |&(_, node), &path_child| (path_child, node),
                );
            }

            transitive_overwritten.complete()
        };

        cx.dump_facts("transitive_overwritten", transitive_overwritten.iter())
            .unwrap();

        ///////////////////////////////////////////////////////////////////////////
        // Round 2

        // .decl access(Perm:perm, Path:path, Node:node)
        // .input access
        //
        // Keyed based on the `perm`
        let access_by_perm: Relation<(Perm, (Path, Node))> = analysis_ir
            .access
            .iter()
            .map(|&(perm, path, node)| (perm, (path, node)))
            .collect();

        let mut iteration = Iteration::new();

        // Variant of `transitive_owner_path` keyed by the child
        let transitive_owner_path_by_child: Relation<_> = transitive_owner_path
            .iter()
            .map(|&(path_parent, path_child)| (path_child, path_parent))
            .collect();

        // .decl access_path(Path:path, Node:node)
        //
        // The path `Path` is accessed sometime during the node `Node`
        // -- in particular, its value on entry to `Node` is accessed.
        let access_path = Relation::from_iter(
            // access_path(Path, Node) :- access(_, Path, Node).
            analysis_ir
                .access
                .iter()
                .map(|&(_, path, node)| ((path, node), ())),
        )
        .merge(
            // access_path(PathChild, Node) :-
            //   access(_, PathParent, Node),
            //   transitive_owner_path(PathParent, PathChild).
            Relation::from_leapjoin(
                &access_by_perm,
                transitive_owner_path.extend_with(|&(_, (path_parent, _))| path_parent),
                |&(_, (_, node)), &path_child| ((path_child, node), ()),
            ),
        )
        .merge(
            // access_path(PathParent, Node) :-
            //   access(_, PathChild, Node),
            //   transitive_owner_path(PathParent, PathChild).
            Relation::from_leapjoin(
                &access_by_perm,
                transitive_owner_path_by_child.extend_with(|&(_, (path_child, _))| path_child),
                |&(_, (_, node)), &path_parent| ((path_parent, node), ()),
            ),
        );
        cx.dump_facts("access_path", access_path.iter()).unwrap();

        // .decl owned(Perm:perm)
        // .input owned
        let owned = &kind_inference.owned;

        // .decl moved(Path:Path, Node:node)
        //
        // Indicates that the path `Path` is **moved** at the given node.
        //
        // moved(Path, Node) :-
        //   access(Perm, Path, Node),
        //   owned(Perm),
        let moved =
            Relation::from_join(&access_by_perm, owned, |&_, &(path, node), &_| (path, node));
        cx.dump_facts("moved", moved.iter()).unwrap();

        // .decl cfg_edge(Node1:node, Node2:node)
        // .input cfg_edge
        let cfg_edge: Relation<_> = analysis_ir.cfg_edge.iter().collect();

        // .decl imprecise_path(Path:path)
        // .input cfg_edge
        let imprecise_path: Relation<(Path, ())> = analysis_ir
            .imprecise_path
            .iter()
            .map(|&path| (path, ()))
            .collect();

        // .decl uninitialized_path(Path:path, Node:node)
        //
        // Indicates that a path `P` is initialized **on entry** to
        // the node `N`.
        let uninitialized_path = iteration.variable::<((Path, Node), ())>("uninitialized_path");

        // uninitialized_path(Path, Node) :-
        //   entry_node(Node),
        //   local_path(Path).
        let entry_node = analysis_ir.lookup_node(HirLocation::Start);
        uninitialized_path.extend(
            analysis_ir
                .local_path
                .iter()
                .map(|&path| ((path, entry_node), ())),
        );

        // .decl error_move_of_imprecise_path(Node:node)
        //
        // Indicates that an imprecise path was moved at `Node`
        let error_move_of_imprecise_path =
            iteration.variable::<(Node, ())>("error_move_of_imprecise_path");

        // .decl error_access_to_uninitialized_path(Path:path, Node:node)
        let error_access_to_uninitialized_path =
            iteration.variable::<(Path, Node)>("error_access_to_uninitialized_path");

        // uninitialized_path(Path, Node2) :-
        //   moved(Path, Node1),
        //   !transitive_overwritten(Path, Node1),
        //   cfg_edge(Node1, Node2).
        uninitialized_path.insert(Relation::from_leapjoin(
            &moved,
            (
                transitive_overwritten.filter_anti(|&(path, node1)| (path, node1)),
                cfg_edge.extend_with(|&(_, node1)| node1),
            ),
            |&(path, _), &node2| ((path, node2), ()),
        ));

        // error_move_of_imprecise_path(Node) :-
        //   moved(Path, Node),
        //   imprecise_path(Path).
        error_move_of_imprecise_path.insert(Relation::from_leapjoin(
            &moved,
            imprecise_path.filter_with(|&(path, _)| (path, ())),
            |&(_, node), &()| (node, ()),
        ));

        while iteration.changed() {
            // uninitialized_path(Path, Node2) :-
            //   uninitialized_path(Path, Node1),
            //   !transitive_overwritten(Path, Node1),
            //   cfg_edge(Node1, Node2).
            uninitialized_path.from_leapjoin(
                &uninitialized_path,
                (
                    transitive_overwritten.filter_anti(|&((path, node1), ())| (path, node1)),
                    cfg_edge.extend_with(|&((_, node1), ())| node1),
                ),
                |&((path, _), ()), &node2| ((path, node2), ()),
            );

            // error_access_to_uninitialized_path(Path, Node) :-
            //   uninitialized_path(Path, Node),
            //   access_path(Path, Node),
            error_access_to_uninitialized_path.from_join(
                &uninitialized_path,
                &access_path,
                |&(path, node), &(), &()| (path, node),
            );
        }

        if cx.dump_enabled() {
            cx.dump_facts("uninitialized_path", uninitialized_path.complete().iter())
                .unwrap();
        }

        let error_move_of_imprecise_path = error_move_of_imprecise_path.complete();
        let error_access_to_uninitialized_path = error_access_to_uninitialized_path.complete();

        cx.dump_facts(
            "error_move_of_imprecise_path",
            error_move_of_imprecise_path.iter(),
        )
        .unwrap();

        cx.dump_facts(
            "error_access_to_uninitialized_path",
            error_access_to_uninitialized_path.iter(),
        )
        .unwrap();

        Initialization {
            error_move_of_imprecise_path,
            error_access_to_uninitialized_path,
        }
    }
}
