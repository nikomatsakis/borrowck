use super::*;

pub struct TransposedGraph<G: Graph> {
    base_graph: G,
}

impl<G: Graph> TransposedGraph<G> {
    pub fn new(base_graph: G) -> Self {
        TransposedGraph { base_graph: base_graph }
    }
}

impl<G: Graph> Graph for TransposedGraph<G> {
    type Node = G::Node;

    fn num_nodes(&self) -> usize {
        self.base_graph.num_nodes()
    }

    fn start_node(&self) -> Self::Node {
        self.base_graph.start_node()
    }

    fn predecessors<'graph>(&'graph self, node: Self::Node)
                            -> <Self as GraphPredecessors<'graph>>::Iter {
        self.base_graph.successors(node)
    }

    fn successors<'graph>(&'graph self, node: Self::Node)
                          -> <Self as GraphSuccessors<'graph>>::Iter {
        self.base_graph.predecessors(node)
    }
}

impl<'graph, G: Graph> GraphPredecessors<'graph> for TransposedGraph<G> {
    type Item = G::Node;
    type Iter = <G as GraphSuccessors<'graph>>::Iter;
}

impl<'graph, G: Graph> GraphSuccessors<'graph> for TransposedGraph<G> {
    type Item = G::Node;
    type Iter = <G as GraphPredecessors<'graph>>::Iter;
}
