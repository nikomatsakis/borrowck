use super::tree::*;
use super::super::Graph;
use super::super::dominators::{Dominators, dominators};
use super::super::node_vec::NodeVec;

use std::collections::HashSet;
use std::default::Default;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum NodeState {
    NotYetStarted,
    InProgress(Option<LoopId>),
    FinishedHeadWalk,
    EnqueuedExitWalk,
}
use self::NodeState::*;

impl Default for NodeState {
    fn default() -> Self {
        NotYetStarted
    }
}

pub struct LoopTreeWalk<'walk, G: Graph + 'walk> {
    graph: &'walk G,
    dominators: &'walk Dominators<G>,
    state: NodeVec<G, NodeState>,
    loop_tree: LoopTree<G>,
}

impl<'walk, G: Graph> LoopTreeWalk<'walk, G> {
    pub fn new(graph: &'walk G,
               dominators: &'walk Dominators<G>)
               -> Self {
        LoopTreeWalk {
            graph: graph,
            dominators: dominators,
            state: NodeVec::from_default(graph),
            loop_tree: LoopTree::new(graph),
        }
    }

    pub fn compute_loop_tree(mut self) -> LoopTree<G> {
        let set = HashSet::new(); // temporary storage used during the walk
        self.head_walk(self.graph.start_node(), set);
        self.exit_walk(self.graph.start_node());
        self.loop_tree
    }

    /// First walk: identify loop heads and loop parents. This uses a
    /// variant of Tarjan's SCC algorithm. Basically, we do a
    /// depth-first search. Each time we encounter a backedge, the
    /// target of that backedge is a loop-head, so we make a
    /// corresponding loop, if we haven't done so already. We then track
    /// the set of loops that `node` was able to reach via backedges.
    /// The innermost such loop is the loop-id of `node`, and we then
    /// return the set for use by the predecessor of `node`.
    ///
    /// (As an optimization, we have the predecessor pass in a set
    /// that `head_walk` will adjust and return, just so we're not
    /// allocating a ton of sets.)
    fn head_walk(&mut self, node: G::Node, mut set: HashSet<LoopId>) -> HashSet<LoopId> {
        match self.state[node] {
            NotYetStarted => {
                self.state[node] = InProgress(None);
            }
            InProgress(opt_loop_id) => {
                // Backedge.
                if let Some(loop_id) = opt_loop_id {
                    set.insert(loop_id);
                } else {
                    set.insert(self.promote_to_loop_head(node));
                }
                return set;
            }
            FinishedHeadWalk => {
                // Cross edge.
                return set;
            }
            EnqueuedExitWalk => {
                unreachable!()
            }
        }

        for successor in self.graph.successors(node) {
            set = self.head_walk(successor, set);
        }

        self.state[node] = FinishedHeadWalk;

        // First, we need to determine the innermost loop.
        match self.innermost(&set) {
            Some(loop_id) => {
                self.loop_tree.set_loop_id(node, Some(loop_id));

                // Check if we are the loop head. In that case, we
                // should remove ourselves from the returned set,
                // since our parent in the spanning tree is not a
                // member of this loop.
                let loop_head = self.loop_tree.loop_head(loop_id);
                if node == loop_head {
                    set.remove(&loop_id);

                    // Now the next-innermost loop is the parent of this loop.
                    let parent_loop_id = self.innermost(&set);
                    self.loop_tree.set_parent(loop_id, parent_loop_id);
                }
            }
            None => {
                assert!(set.is_empty());
                assert!(self.loop_tree.loop_id(node).is_none()); // all none by default
            }
        }

        set
    }

    fn exit_walk(&mut self, node: G::Node) {
        let mut stack = vec![node];

        assert_eq!(self.state[node], FinishedHeadWalk);
        self.state[node] = EnqueuedExitWalk;

        while let Some(node) = stack.pop() {
            // For each successor, check what loop they are in. If any of
            // them are in a loop outer to ours -- or not in a loop at all
            // -- those are exits from this inner loop.
            if let Some(loop_id) = self.loop_tree.loop_id(node) {
                for successor in self.graph.successors(node) {
                    self.update_loop_exit(loop_id, successor);
                }
            }

            // Visit our successors.
            for successor in self.graph.successors(node) {
                match self.state[successor] {
                    NotYetStarted | InProgress(_) => {
                        unreachable!();
                    }
                    FinishedHeadWalk => {
                        stack.push(successor);
                        self.state[successor] = EnqueuedExitWalk;
                    }
                    EnqueuedExitWalk => {
                    }
                }
            }
        }
    }

    fn promote_to_loop_head(&mut self,
                            node: G::Node)
                            -> LoopId {
        assert_eq!(self.state[node], InProgress(None));
        let loop_id = self.loop_tree.new_loop(node);
        self.state[node] = InProgress(Some(loop_id));
        loop_id
    }

    fn innermost(&self, set: &HashSet<LoopId>) -> Option<LoopId> {
        let mut innermost = None;
        for &loop_id1 in set {
            if let Some(loop_id2) = innermost {
                if self.is_inner_loop_of(loop_id1, loop_id2) {
                    innermost = Some(loop_id1);
                }
            } else {
                innermost = Some(loop_id1);
            }
        }
        innermost
    }

    fn is_inner_loop_of(&self, l1: LoopId, l2: LoopId) -> bool {
        let h1 = self.loop_tree.loop_head(l1);
        let h2 = self.loop_tree.loop_head(l2);
        assert!(h1 != h2);
        if self.dominators.is_dominated_by(h1, h2) {
            true
        } else {
            // These two must have a dominance relationship or else
            // the graph is not reducible.
            assert!(self.dominators.is_dominated_by(h2, h1));
            false
        }
    }

    /// Some node that is in loop `loop_id` has the successor
    /// `successor`. Check if `successor` is not in the loop
    /// `loop_id` and update loop exits appropriately.
    fn update_loop_exit(&mut self, loop_id: LoopId, successor: G::Node) {
        match self.loop_tree.loop_id(successor) {
            Some(successor_loop_id) => {
                // Successor is in a loop, so check if it is an inner loop
                // to ours.
                if
                    loop_id != successor_loop_id &&
                    self.is_inner_loop_of(loop_id, successor_loop_id)
                {
                    self.loop_tree.push_loop_exit(loop_id, successor);
                }
            }
            None => {
                // Successor is not in a loop, so they are
                // definitely an exit from our loop.
                self.loop_tree.push_loop_exit(loop_id, successor);
            }
        }
    }
}
