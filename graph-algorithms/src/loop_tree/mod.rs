use super::Graph;
use super::dominators::{Dominators, dominators};


#[cfg(test)]
mod test;
mod tree;
mod walk;

pub use self::tree::LoopTree;

pub fn loop_tree<G: Graph>(graph: &G) -> LoopTree<G> {
    let dominators = dominators(graph);
    loop_tree_given(graph, &dominators)
}

pub fn loop_tree_given<G: Graph>(graph: &G,
                                 dominators: &Dominators<G>)
                                 -> LoopTree<G>
{
    walk::LoopTreeWalk::new(graph, dominators).compute_loop_tree()
}


