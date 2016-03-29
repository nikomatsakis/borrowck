use std::mem;
use super::{Graph, NodeIndex};
use super::iterate::reverse_post_order;
use super::node_vec::NodeVec;

pub type ImmediateDominators<G: Graph> = NodeVec<G, Option<G::Node>>;

#[cfg(test)]
mod test;

pub fn immediate_dominators<G: Graph>(graph: &G,
                                      start_node: G::Node)
                                      -> ImmediateDominators<G>
{
    let rpo = reverse_post_order(graph, start_node);
    assert_eq!(rpo[0], start_node);

    // compute the post order index (rank) for each node
    let mut po_rank: NodeVec<G, usize> = NodeVec::from_default(graph);
    for (index, node) in rpo.iter().rev().cloned().enumerate() {
        po_rank[node] = index;
    }

    let mut immediate_dominators: NodeVec<G, Option<G::Node>> =
        NodeVec::from_default(graph);
    immediate_dominators[start_node] = Some(start_node);

    let mut changed = true;
    while mem::replace(&mut changed, false) {
        println!("iterate");
        for &node in &rpo[1..] {
            println!("node={:?}", node);
            let mut new_idom = None;
            for &pred in graph.predecessors(node).iter() {
                println!("pred={:?} new_idom={:?}", pred, new_idom);
                if immediate_dominators[pred].is_some() { // (*)
                    // (*) dominators for `pred` have been calculated
                    new_idom = intersect_opt(&po_rank,
                                             &immediate_dominators,
                                             new_idom,
                                             Some(pred));
                }
            }
            println!("node={:?} new_idom={:?}", node, new_idom);
            if new_idom != immediate_dominators[node] {
                immediate_dominators[node] = new_idom;
                changed = true;
            }
        }
    }

    immediate_dominators
}

fn intersect_opt<G: Graph>(po_rank: &NodeVec<G, usize>,
                           immediate_dominators: &ImmediateDominators<G>,
                           node1: Option<G::Node>,
                           node2: Option<G::Node>)
                           -> Option<G::Node>
{
    match (node1, node2) {
        (None, None) => None,
        (Some(n), None) | (None, Some(n)) => Some(n),
        (Some(n1), Some(n2)) => Some(intersect(po_rank,
                                               immediate_dominators,
                                               n1,
                                               n2)),
    }
}

fn intersect<G: Graph>(po_rank: &NodeVec<G, usize>,
                       immediate_dominators: &ImmediateDominators<G>,
                       mut node1: G::Node,
                       mut node2: G::Node)
                       -> G::Node
{
    while node1 != node2 {
        println!("intersect(node1={:?}, node2={:?})", node1, node2);

        while po_rank[node1] < po_rank[node2] {
            println!("po_rank[{:?}]={}, po_rank[{:?}]={} / 1",
                     node1, po_rank[node1], node2, po_rank[node2]);
            node1 = immediate_dominators[node1].unwrap();
        }

        while po_rank[node2] < po_rank[node1] {
            println!("po_rank[{:?}]={}, po_rank[{:?}]={} / 2",
                     node1, po_rank[node1], node2, po_rank[node2]);
            node2 = immediate_dominators[node2].unwrap();
        }
    }
    return node1;
}

