use std::collections::HashMap;
use std::cmp::max;
use std::slice;
use std::iter;

use super::{Graph, GraphPredecessors, GraphSuccessors, NodeIndex};

pub struct TestGraph {
    num_nodes: usize,
    start_node: usize,
    successors: HashMap<usize, Vec<usize>>,
    predecessors: HashMap<usize, Vec<usize>>,
}

impl TestGraph {
    pub fn new(start_node: usize, edges: &[(usize, usize)]) -> Self {
        let mut graph = TestGraph {
            num_nodes: start_node + 1,
            start_node: start_node,
            successors: HashMap::new(),
            predecessors: HashMap::new()
        };
        for &(source, target) in edges {
            graph.num_nodes = max(graph.num_nodes, source + 1);
            graph.num_nodes = max(graph.num_nodes, target + 1);
            graph.successors.entry(source).or_insert(vec![]).push(target);
            graph.predecessors.entry(target).or_insert(vec![]).push(source);
        }
        for node in 0..graph.num_nodes {
            graph.successors.entry(node).or_insert(vec![]);
            graph.predecessors.entry(node).or_insert(vec![]);
        }
        graph
    }
}

impl Graph for TestGraph {
    type Node = usize;

    fn start_node(&self) -> usize {
        self.start_node
    }

    fn num_nodes(&self) -> usize {
        self.num_nodes
    }

    fn predecessors<'graph>(&'graph self, node: usize)
                            -> <Self as GraphPredecessors<'graph>>::Iter {
       self.predecessors[&node].iter().cloned()
    }

    fn successors<'graph>(&'graph self, node: usize)
                            -> <Self as GraphSuccessors<'graph>>::Iter {
        self.successors[&node].iter().cloned()
    }
}

impl<'graph> GraphPredecessors<'graph> for TestGraph {
    type Item = usize;
    type Iter = iter::Cloned<slice::Iter<'graph, usize>>;
}

impl<'graph> GraphSuccessors<'graph> for TestGraph {
    type Item = usize;
    type Iter = iter::Cloned<slice::Iter<'graph, usize>>;
}

impl NodeIndex for usize {
}
