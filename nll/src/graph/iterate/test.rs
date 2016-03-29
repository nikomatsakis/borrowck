use graph::test::TestGraph;
use super::{post_order, reverse_post_order};

#[test]
fn diamond_post_order() {
    let graph = TestGraph::new(&[
        (0, 1),
        (0, 2),
        (1, 3),
        (2, 3),
    ]);

    let result = post_order(&graph, 0);
    assert_eq!(result, vec![3, 1, 2, 0]);
}

