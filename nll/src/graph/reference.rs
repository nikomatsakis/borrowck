use super::*;

impl<'graph, G: Graph> Graph for &'graph G {
    type Node = G::Node;

    fn num_nodes(&self) -> usize {
        (**self).num_nodes()
    }

    fn start_node(&self) -> Self::Node {
        (**self).start_node()
    }

    fn predecessors<'iter>(&'iter self, node: Self::Node)
                            -> <Self as GraphPredecessors<'iter>>::Iter {
        (**self).predecessors(node)
    }

    fn successors<'iter>(&'iter self, node: Self::Node)
                          -> <Self as GraphSuccessors<'iter>>::Iter {
        (**self).successors(node)
    }
}

impl<'iter, 'graph, G: Graph> GraphPredecessors<'iter> for &'graph G {
    type Item = G::Node;
    type Iter = <G as GraphPredecessors<'iter>>::Iter;
}

impl<'iter, 'graph, G: Graph> GraphSuccessors<'iter> for &'graph G {
    type Item = G::Node;
    type Iter = <G as GraphSuccessors<'iter>>::Iter;
}
