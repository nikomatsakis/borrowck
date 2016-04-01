use std::fmt::Debug;
use std::hash::Hash;

mod bit_set;
mod dominators;
mod iterate;
mod loop_tree;
mod reachable;
mod reference;
mod node_vec;
mod transpose;

#[cfg(test)]
mod test;

pub trait Graph
    where Self: for<'graph> GraphPredecessors<'graph, Item=<Self as Graph>::Node>,
          Self: for<'graph> GraphSuccessors<'graph, Item=<Self as Graph>::Node>
{
    type Node: NodeIndex;

    fn num_nodes(&self) -> usize;
    fn start_node(&self) -> Self::Node;
    fn predecessors<'graph>(&'graph self, node: Self::Node)
                            -> <Self as GraphPredecessors<'graph>>::Iter;
    fn successors<'graph>(&'graph self, node: Self::Node)
                            -> <Self as GraphSuccessors<'graph>>::Iter;
}

pub trait GraphPredecessors<'graph> {
    type Item;
    type Iter: Iterator<Item=Self::Item>;
}

pub trait GraphSuccessors<'graph> {
    type Item;
    type Iter: Iterator<Item=Self::Item>;
}

pub trait NodeIndex: Copy + Debug + Eq + Ord + Hash + Into<usize> + From<usize> {
    fn as_usize(self) -> usize {
        self.into()
    }
}

