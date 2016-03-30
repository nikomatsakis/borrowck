use graph::test::TestGraph;
use super::{loop_tree, LoopTree};

#[test]
fn test1() {
    // 0 -> 1 -> 2 -> 3
    //      ^    v
    //      6 <- 4 -> 5
    let graph = TestGraph::new(0, &[
        (0, 1),
        (1, 2),
        (2, 3),
        (2, 4),
        (4, 5),
        (4, 6),
        (6, 1),
    ]);
    let loop_tree = loop_tree(&graph);
    assert_eq!(loop_tree.loop_head(0), None);
    assert_eq!(loop_tree.loop_head(1), Some(1));
    assert_eq!(loop_tree.loop_head(2), Some(1));
    assert_eq!(loop_tree.loop_head(3), None);
    assert_eq!(loop_tree.loop_head(4), Some(1));
    assert_eq!(loop_tree.loop_head(5), None);
    assert_eq!(loop_tree.loop_head(6), Some(1));
}

#[test] #[ignore] // BROKEN
fn nested_loop() {
    // 0 -> 1 ->     2     -> 3 -> 5
    //      ^     ^    v      |
    //      |     6 <- 4      |
    //      +-----------------+
    let graph = TestGraph::new(0, &[
        (0, 1),
        (1, 2),
        (2, 3),
        (3, 5),
        (3, 1),
        (2, 4),
        (4, 6),
        (6, 2),
    ]);
    let loop_tree = loop_tree(&graph);
    assert_eq!(loop_tree.loop_head(0), None);
    assert_eq!(loop_tree.loop_head(1), Some(1));
    assert_eq!(loop_tree.loop_head(2), Some(2));
    assert_eq!(loop_tree.loop_head(3), Some(1));
    assert_eq!(loop_tree.loop_head(4), Some(2));
    assert_eq!(loop_tree.loop_head(5), None);
    assert_eq!(loop_tree.loop_head(6), Some(2));
}


