use std::borrow::Cow;
use std::fmt::Debug;
use std::hash::Hash;

mod bit_set;
mod dominators;
mod iterate;
mod loop_tree;
mod reachable;
mod node_vec;

#[cfg(test)]
mod test;

pub trait Graph {
    type Node: NodeIndex;

    fn num_nodes(&self) -> usize;
    fn start_node(&self) -> Self::Node;
    fn predecessors<'graph>(&'graph self, node: Self::Node) -> Cow<'graph, [Self::Node]>;
    fn successors<'graph>(&'graph self, node: Self::Node) -> Cow<'graph, [Self::Node]>;
}

pub trait NodeIndex: Copy + Debug + Eq + Ord + Hash + Into<usize> + From<usize> {
    fn as_usize(self) -> usize {
        self.into()
    }
}


