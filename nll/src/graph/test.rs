use std::borrow::Cow;
use std::collections::HashMap;
use std::cmp::max;

use super::{Graph, NodeIndex};

pub struct TestGraph {
    num_nodes: usize,
    successors: HashMap<usize, Vec<usize>>,
    predecessors: HashMap<usize, Vec<usize>>,
}

impl TestGraph {
    pub fn new(edges: &[(usize, usize)]) -> Self {
        let mut graph = TestGraph {
            num_nodes: 0,
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

    fn num_nodes(&self) -> usize {
        self.num_nodes
    }

    fn predecessors<'graph>(&'graph self, node: usize) -> Cow<'graph, [Self::Node]> {
        Cow::Borrowed(&self.predecessors[&node])
    }

    fn successors<'graph>(&'graph self, node: usize) -> Cow<'graph, [Self::Node]> {
        Cow::Borrowed(&self.successors[&node])
    }
}

impl NodeIndex for usize {
}
