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
        // Round 1: Compute `transitive_overwritten`

        let owner_path = Relation::from(analysis_ir.owner_path.iter().cloned());
        let owner_path_child = Relation::from(
            analysis_ir
                .owner_path
                .iter()
                .map(|&(path_owner, path_child)| (path_child, path_owner)),
        );

        let transitive_overwritten = {
            let mut iteration = Iteration::new();

            // .decl transitive_overwritten(Path, Node)
            let transitive_overwritten =
                iteration.variable::<(Path, Node)>("transitive_overwritten");

            // transitive_overwritten(Path, Node) :- overwritten(Path, Node).
            transitive_overwritten.insert(Relation::from(analysis_ir.overwritten.iter().cloned()));

            while iteration.changed() {
                // transitive_overwritten(PathChild, Node) :-
                //   transitive_overwritten(PathParent, Node),
                //   owner_path(PathParent, PathChild).
                transitive_overwritten.from_leapjoin(
                    &transitive_overwritten,
                    &mut [&mut owner_path.extend_with(|&(path_parent, _)| path_parent)],
                    |&(_, node), &path_child| (path_child, node),
                );
            }

            transitive_overwritten.complete()
        };

        cx.dump_facts("transitive_overwritten", transitive_overwritten.iter())
            .unwrap();

        ///////////////////////////////////////////////////////////////////////////
        // Round 2

        let mut iteration = Iteration::new();

        // .decl access(Perm:perm, Path:path, Node:node)
        // .input access
        //
        // Keyed based on the `perm`
        let access_perm = iteration.variable::<(Perm, (Path, Node))>("access_perm");
        access_perm.insert(Relation::from(
            analysis_ir
                .access
                .iter()
                .map(|&(perm, path, node)| (perm, (path, node))),
        ));

        // .decl access_path(Path:path, Node:node)
        //
        // The path `Path` is accessed sometime during the node `Node`
        // -- in particular, its value on entry to `Node` is accessed.
        let access_path = iteration.variable::<(Path, Node)>("access_path");

        // Maintain an index of `access_path` with both `(path, node)` indexed.
        let access_path_full = iteration.variable::<((Path, Node), ())>("access_path_full");

        // .decl owned(Perm:perm)
        // .input owned
        let owned = iteration.variable::<(Perm, ())>("owned");
        owned.insert(kind_inference.owned.clone());

        // .decl cfg_edge(Node1:node, Node2:node)
        // .input cfg_edge
        let cfg_edge = Relation::from(analysis_ir.cfg_edge.iter().cloned());

        // .decl imprecise_path(Path:path)
        // .input cfg_edge
        let imprecise_path: Relation<(Path, ())> =
            Relation::from(analysis_ir.imprecise_path.iter().map(|&path| (path, ())));

        // .decl moved(Path:Path, Node:node)
        //
        // Indicates that the path `Path` is **moved** at the given node.
        let moved = iteration.variable::<(Path, Node)>("entry_node");

        // .decl uninitialized_path(Path:path, Node:node)
        //
        // Indicates that a path `P` is initialized **on entry** to
        // the node `N`.
        let uninitialized_path = iteration.variable::<((Path, Node), ())>("uninitialized_path");

        // uninitialized_path(Path, Node) :-
        //   entry_node(Node),
        //   local_path(Path).
        let entry_node = analysis_ir.lookup_node(HirLocation::Start);
        uninitialized_path.insert(Relation::from(
            analysis_ir
                .local_path
                .iter()
                .map(|&path| ((path, entry_node), ())),
        ));

        // .decl error_move_of_imprecise_path(Node:node)
        //
        // Indicates that an imprecise path was moved at `Node`
        let error_move_of_imprecise_path =
            iteration.variable::<(Node, ())>("error_move_of_imprecise_path");

        // .decl error_access_to_uninitialized_path(Path:path, Node:node)
        let error_access_to_uninitialized_path =
            iteration.variable::<(Path, Node)>("error_access_to_uninitialized_path");

        // access_path(Path, Node) :- access(_, Path, Node).
        access_path.insert(Relation::from(
            analysis_ir
                .access
                .iter()
                .map(|&(_, path, node)| (path, node)),
        ));

        while iteration.changed() {
            // access_path(PathChild, Node) :-
            //   access_path(PathParent, Node),
            //   owner_path(PathParent, PathChild).
            access_path.from_leapjoin(
                &access_path,
                &mut [&mut owner_path.extend_with(|&(path_parent, _)| path_parent)],
                |&(_, node), &path_child| (path_child, node),
            );

            // access_path(PathParent, Node) :-
            //   access_path(PathChild, Node),
            //   owner_path(PathParent, PathChild).
            access_path.from_leapjoin(
                &access_path,
                &mut [&mut owner_path_child.extend_with(|&(path_child, _)| path_child)],
                |&(_, node), &path_parent| (path_parent, node),
            );

            // `access_path_full` is just an index from `access_path`
            access_path_full.from_map(&access_path, |&(path, node)| ((path, node), ()));

            // moved(Path, Node) :-
            //   access(Perm, Path, Node),
            //   owned(Perm),
            moved.from_join(&access_perm, &owned, |&_, &(path, node), &_| (path, node));

            // uninitialized_path(Path, Node2) :-
            //   uninitialized_path(Path, Node1),
            //   !transitive_overwritten(Path, Node1),
            //   cfg_edge(Node1, Node2).
            uninitialized_path.from_leapjoin(
                &uninitialized_path,
                &mut [
                    &mut transitive_overwritten.filter_anti(|&((path, node1), ())| (path, node1)),
                    &mut cfg_edge.extend_with(|&((_, node1), ())| node1),
                ],
                |&((path, _), ()), &node2| ((path, node2), ()),
            );

            // uninitialized_path(Path, Node2) :-
            //   moved(Path, Node1),
            //   !transitive_overwritten(Path, Node1),
            //   cfg_edge(Node1, Node2).
            uninitialized_path.from_leapjoin(
                &moved,
                &mut [
                    &mut transitive_overwritten.filter_anti(|&(path, node1)| (path, node1)),
                    &mut cfg_edge.extend_with(|&(_, node1)| node1),
                ],
                |&(path, _), &node2| ((path, node2), ()),
            );

            // error_move_of_imprecise_path(Node) :-
            //   moved(Path, Node),
            //   imprecise_path(Path).
            error_move_of_imprecise_path.from_leapjoin(
                &moved,
                &mut [&mut imprecise_path.filter_with(|&(path, _)| (path, ()))],
                |&(_, node), &()| (node, ()),
            );

            // error_access_to_uninitialized_path(Path, Node) :-
            //   uninitialized_path(Path, Node),
            //   access_path(Path, Node),
            error_access_to_uninitialized_path.from_join(
                &uninitialized_path,
                &access_path_full,
                |&(path, node), &(), &()| (path, node),
            );
        }

        if cx.dump_enabled() {
            cx.dump_facts("moved", moved.complete().iter()).unwrap();

            cx.dump_facts("uninitialized_path", uninitialized_path.complete().iter())
                .unwrap();

            cx.dump_facts("access_path", access_path.complete().iter())
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
