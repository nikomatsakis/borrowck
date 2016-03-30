use super::Graph;
use super::node_vec::NodeVec;

#[cfg(test)]
mod test;

pub fn post_order<G: Graph>(graph: &G, start_node: G::Node) -> Vec<G::Node> {
    let mut visited: NodeVec<G, bool> = NodeVec::from_default(graph);
    let mut result: Vec<G::Node> = Vec::with_capacity(graph.num_nodes());
    post_order_walk(graph, start_node, &mut result, &mut visited);
    result
}

fn post_order_walk<G: Graph>(graph: &G,
                             node: G::Node,
                             result: &mut Vec<G::Node>,
                             visited: &mut NodeVec<G, bool>) {
    println!("post_order_walk(node: {:?})", node);
    if visited[node] {
        return;
    }
    visited[node] = true;

    for successor in graph.successors(node) {
        post_order_walk(graph, successor, result, visited);
    }

    result.push(node);
}

pub fn reverse_post_order<G: Graph>(graph: &G, start_node: G::Node) -> Vec<G::Node> {
    let mut vec = post_order(graph, start_node);
    vec.reverse();
    vec
}
