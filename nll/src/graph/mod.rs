use std::borrow::Cow;
use std::fmt::Debug;
use std::hash::Hash;

mod dominators;
mod iterate;
mod node_vec;

#[cfg(test)]
mod test;

pub trait Graph {
    type Node: NodeIndex;

    fn num_nodes(&self) -> usize;
    fn predecessors<'graph>(&'graph self, node: Self::Node) -> Cow<'graph, [Self::Node]>;
    fn successors<'graph>(&'graph self, node: Self::Node) -> Cow<'graph, [Self::Node]>;
}

pub trait NodeIndex: Copy + Debug + Eq + Ord + Hash + Into<usize> + From<usize> {
}


