use std::default::Default;
use std::marker::PhantomData;
use std::ops::{Index, IndexMut};
pub use std::slice::Iter;

use super::Graph;

pub struct NodeVec<G: Graph, T> {
    pub vec: Vec<T>,
    graph: PhantomData<G>,
}

impl<G: Graph, T: Clone> NodeVec<G, T> {
    pub fn from_elem(graph: &G, default: &T) -> Self {
        NodeVec::from_fn(graph, |_| default.clone())
    }
}

impl<G: Graph, T: Default> NodeVec<G, T> {
    pub fn from_default(graph: &G) -> Self {
        NodeVec::from_fn(graph, |_| T::default())
    }
}

impl<G: Graph, T> NodeVec<G, T> {
    pub fn from_fn<F>(graph: &G, f: F) -> Self
        where F: FnMut(G::Node) -> T
    {
        let num_nodes = graph.num_nodes();
        NodeVec {
            vec: (0..num_nodes).map(G::Node::from).map(f).collect(),
            graph: PhantomData,
        }
    }

    pub fn iter(&self) -> Iter<T> {
        self.vec.iter()
    }
}

impl<G: Graph, T> Index<G::Node> for NodeVec<G, T> {
    type Output = T;

    fn index(&self, index: G::Node) -> &T {
        let index: usize = index.into();
        &self.vec[index]
    }
}

impl<G: Graph, T> IndexMut<G::Node> for NodeVec<G, T> {
    fn index_mut(&mut self, index: G::Node) -> &mut T {
        let index: usize = index.into();
        &mut self.vec[index]
    }
}

