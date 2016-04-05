use super::Graph;
use super::iterate::reverse_post_order;
use super::node_vec::NodeVec;

use std::fmt;

#[cfg(test)]
mod test;

pub fn dominators<G: Graph>(graph: &G)
                            -> Dominators<G>
{
    let start_node = graph.start_node();
    let rpo = reverse_post_order(graph, start_node);
    dominators_given_rpo(graph, &rpo)
}

pub fn dominators_given_rpo<G: Graph>(graph: &G,
                                      rpo: &[G::Node])
                                      -> Dominators<G>
{
    let start_node = graph.start_node();
    assert_eq!(rpo[0], start_node);

    // compute the post order index (rank) for each node
    let mut post_order_rank: NodeVec<G, usize> = NodeVec::from_default(graph);
    for (index, node) in rpo.iter().rev().cloned().enumerate() {
        post_order_rank[node] = index;
    }

    let mut immediate_dominators: NodeVec<G, Option<G::Node>> =
        NodeVec::from_default(graph);
    immediate_dominators[start_node] = Some(start_node);

    let mut changed = true;
    while changed {
        changed = false;

        for &node in &rpo[1..] {
            let mut new_idom = None;
            for pred in graph.predecessors(node) {
                if immediate_dominators[pred].is_some() { // (*)
                    // (*) dominators for `pred` have been calculated
                    new_idom = intersect_opt(&post_order_rank,
                                             &immediate_dominators,
                                             new_idom,
                                             Some(pred));
                }
            }

            if new_idom != immediate_dominators[node] {
                immediate_dominators[node] = new_idom;
                changed = true;
            }
        }
    }

    Dominators {
        post_order_rank: post_order_rank,
        immediate_dominators: immediate_dominators,
    }
}

fn intersect_opt<G: Graph>(post_order_rank: &NodeVec<G, usize>,
                           immediate_dominators: &NodeVec<G, Option<G::Node>>,
                           node1: Option<G::Node>,
                           node2: Option<G::Node>)
                           -> Option<G::Node>
{
    match (node1, node2) {
        (None, None) => None,
        (Some(n), None) | (None, Some(n)) => Some(n),
        (Some(n1), Some(n2)) => Some(intersect(post_order_rank,
                                               immediate_dominators,
                                               n1,
                                               n2)),
    }
}

fn intersect<G: Graph>(post_order_rank: &NodeVec<G, usize>,
                       immediate_dominators: &NodeVec<G, Option<G::Node>>,
                       mut node1: G::Node,
                       mut node2: G::Node)
                       -> G::Node
{
    while node1 != node2 {
        while post_order_rank[node1] < post_order_rank[node2] {
            node1 = immediate_dominators[node1].unwrap();
        }

        while post_order_rank[node2] < post_order_rank[node1] {
            node2 = immediate_dominators[node2].unwrap();
        }
    }
    return node1;
}

pub struct Dominators<G: Graph> {
    post_order_rank: NodeVec<G, usize>,
    immediate_dominators: NodeVec<G, Option<G::Node>>,
}

impl<G: Graph> Dominators<G> {
    pub fn is_reachable(&self, node: G::Node) -> bool {
        self.immediate_dominators[node].is_some()
    }

    pub fn immediate_dominator(&self, node: G::Node) -> G::Node {
        assert!(self.is_reachable(node), "node {:?} is not reachable", node);
        self.immediate_dominators[node].unwrap()
    }

    pub fn dominators(&self, node: G::Node) -> Iter<G> {
        assert!(self.is_reachable(node), "node {:?} is not reachable", node);
        Iter { dominators: self, node: Some(node) }
    }

    pub fn is_dominated_by(&self, node: G::Node, dom: G::Node) -> bool {
        // FIXME -- could be optimized by using post-order-rank
        self.dominators(node).any(|n| n == dom)
    }

    pub fn mutual_dominator(&self, node1: G::Node, node2: G::Node) -> G::Node {
        assert!(self.is_reachable(node1), "node {:?} is not reachable", node1);
        assert!(self.is_reachable(node2), "node {:?} is not reachable", node2);
        intersect(&self.post_order_rank, &self.immediate_dominators, node1, node2)
    }

    pub fn all_immediate_dominators(&self) -> &NodeVec<G, Option<G::Node>> {
        &self.immediate_dominators
    }

    pub fn dominator_tree(&self) -> DominatorTree<G> {
        let mut children: NodeVec<G, Vec<G::Node>> =
            NodeVec::from_default_with_len(self.immediate_dominators.len());
        let mut root = None;
        for (index, immed_dom) in self.immediate_dominators.iter().enumerate() {
            let node = G::Node::from(index);
            match *immed_dom {
                None => { /* node not reachable */ }
                Some(immed_dom) => {
                    if node == immed_dom {
                        root = Some(node);
                    } else {
                        children[immed_dom].push(node);
                    }
                }
            }
        }
        DominatorTree { root: root.unwrap(), children: children }
    }
}

pub struct Iter<'dom, G: Graph + 'dom> {
    dominators: &'dom Dominators<G>,
    node: Option<G::Node>
}

impl<'dom, G: Graph> Iterator for Iter<'dom, G> {
    type Item = G::Node;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(node) = self.node {
            let dom = self.dominators.immediate_dominator(node);
            if dom == node {
                self.node = None; // reached the root
            } else {
                self.node = Some(dom);
            }
            return Some(node);
        } else {
            return None;
        }
    }
}

pub struct DominatorTree<G: Graph> {
    root: G::Node,
    children: NodeVec<G, Vec<G::Node>>,
}

impl<G: Graph> DominatorTree<G> {
    pub fn root(&self) -> G::Node {
        self.root
    }

    pub fn children(&self, node: G::Node) -> &[G::Node] {
        &self.children[node]
    }
}

impl<G: Graph> fmt::Debug for DominatorTree<G> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        fmt::Debug::fmt(&DominatorTreeNode { tree: self, node: self.root }, fmt)
    }
}

struct DominatorTreeNode<'tree, G: Graph + 'tree> {
    tree: &'tree DominatorTree<G>,
    node: G::Node,
}

impl<'tree, G: Graph> fmt::Debug for DominatorTreeNode<'tree, G> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        let subtrees: Vec<_> =
            self.tree.children(self.node)
                     .iter()
                     .map(|&child| DominatorTreeNode { tree: self.tree, node: child })
                     .collect();
        fmt.debug_tuple("")
           .field(&self.node)
           .field(&subtrees)
           .finish()
    }
}

