use graph::test::TestGraph;
use super::dominators;

#[test]
fn diamond() {
    let graph = TestGraph::new(&[
        (0, 1),
        (0, 2),
        (1, 3),
        (2, 3),
    ]);

    let dominators = dominators(&graph, 0);
    assert_eq!(&dominators.all_immediate_dominators().vec[..],
               &[Some(0),
                 Some(0),
                 Some(0),
                 Some(0)]);
}

#[test]
fn paper() {
    // example from the paper:
    let graph = TestGraph::new(&[
        (6, 5),
        (6, 4),
        (5, 1),
        (4, 2),
        (4, 3),
        (1, 2),
        (2, 3),
        (3, 2),
        (2, 1),
    ]);

    let dominators = dominators(&graph, 6);
    assert_eq!(&dominators.all_immediate_dominators().vec[..],
               &[None, // <-- note that 0 is not in graph
                 Some(6), Some(6), Some(6),
                 Some(6), Some(6), Some(6)]);
}

