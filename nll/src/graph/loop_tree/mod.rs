use super::Graph;
use super::dominators::{Dominators, dominators_given_rpo};
use super::iterate::reverse_post_order;
use super::node_vec::NodeVec;
use super::reachable::{Reachability, reachable_given_rpo};

use std::u32;

#[cfg(test)]
mod test;

pub fn loop_tree<G: Graph>(graph: &G) -> LoopTree<G> {
    let reverse_post_order = reverse_post_order(graph, graph.start_node());
    let reachable = reachable_given_rpo(graph, &reverse_post_order);
    let dominators = dominators_given_rpo(graph, &reverse_post_order);
    loop_tree_given(graph, &reverse_post_order, &reachable, &dominators)
}

pub fn loop_tree_given<G: Graph>(graph: &G,
                                 rpo: &[G::Node],
                                 reachable: &Reachability<G>,
                                 dominators: &Dominators<G>)
                                 -> LoopTree<G>
{
    let mut loop_ids: NodeVec<G, Option<LoopId>> = NodeVec::from_default(graph);
    let mut loop_nodes: Vec<LoopNode<G>> = vec![];

    for &node in rpo {
        // Find innermost loop that `node` is a member of (excluding
        // any loop where `node` is the head), if any.
        let innermost_loop_id =
            dominators.dominators(node)
                      .skip(1)
                      .filter(|&dom| reachable.can_reach(node, dom))
                      .next()
                      .map(|n| loop_ids[n].unwrap());

        // `node` itself is a loop head if it dominates any of its
        // predecessors.
        let node_is_loop_head =
            graph.predecessors(node)
                 .any(|pred| dominators.is_dominated_by(pred, node));

        if node_is_loop_head {
            loop_nodes.push(LoopNode {
                parent: innermost_loop_id,
                head: node
            });
            loop_ids[node] = Some(LoopId::from(loop_nodes.len() - 1));
        } else {
            loop_ids[node] = innermost_loop_id;
        }
    }

    LoopTree {
        loop_ids: loop_ids,
        loop_nodes: loop_nodes,
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct LoopId {
    index: u32
}

impl From<usize> for LoopId {
    fn from(value: usize) -> LoopId {
        assert!(value < (u32::MAX as usize));
        LoopId { index: value as u32 }
    }
}

impl From<LoopId> for usize {
    fn from(id: LoopId) -> usize {
        id.index as usize
    }
}

#[derive(Debug)]
pub struct LoopNode<G: Graph> {
    parent: Option<LoopId>,
    head: G::Node,
}

impl<G: Graph> Copy for LoopNode<G> { }

impl<G: Graph> Clone for LoopNode<G> { fn clone(&self) -> Self { *self } }

pub struct LoopTree<G: Graph> {
    loop_ids: NodeVec<G, Option<LoopId>>,
    loop_nodes: Vec<LoopNode<G>>,
}

impl<G: Graph> LoopTree<G> {
    pub fn loop_id(&self, node: G::Node) -> Option<LoopId> {
        self.loop_ids[node]
    }

    pub fn loop_head(&self, node: G::Node) -> Option<G::Node> {
        self.loop_ids[node].map(|l| self.loop_node(l).head)
    }

    pub fn loop_node(&self, id: LoopId) -> LoopNode<G> {
        self.loop_nodes[usize::from(id)]
    }
}
