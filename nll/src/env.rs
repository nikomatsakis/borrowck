use graph::{BasicBlockIndex, FuncGraph};
use graph_algorithms::Graph;
use graph_algorithms::dominators::{self, Dominators, DominatorTree};
use graph_algorithms::iterate::reverse_post_order;
use graph_algorithms::loop_tree::{self, LoopTree};
use graph_algorithms::reachable::{self, Reachability};
use region::Region;
use std::collections::HashSet;
use std::fmt;

pub struct Environment<'func> {
    pub graph: &'func FuncGraph,
    pub dominators: Dominators<FuncGraph>,
    pub dominator_tree: DominatorTree<FuncGraph>,
    pub reachable: Reachability<FuncGraph>,
    pub loop_tree: LoopTree<FuncGraph>,
    pub reverse_post_order: Vec<BasicBlockIndex>,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Point {
    pub block: BasicBlockIndex,
    pub action: usize,
}

impl<'func> Environment<'func> {
    pub fn new(graph: &'func FuncGraph) -> Self {
        let rpo = reverse_post_order(graph, graph.start_node());
        let dominators = dominators::dominators_given_rpo(graph, &rpo);
        let dominator_tree = dominators.dominator_tree();
        let reachable = reachable::reachable_given_rpo(graph, &rpo);
        let loop_tree = loop_tree::loop_tree_given(graph, &dominators);

        Environment {
            graph: graph,
            dominators: dominators,
            dominator_tree: dominator_tree,
            reachable: reachable,
            loop_tree: loop_tree,
            reverse_post_order: rpo,
        }
    }

    pub fn dump_dominators(&self) {
        let tree = self.dominators.dominator_tree();
        self.dump_dominator_tree(&tree, tree.root(), 0)
    }

    fn dump_dominator_tree<G1>(&self,
                               tree: &DominatorTree<G1>,
                               node: BasicBlockIndex,
                               indent: usize)
        where G1: Graph<Node=BasicBlockIndex>
    {
        println!("{0:1$}- {2:?}",
                 "",
                 indent,
                 node);

        for &child in tree.children(node) {
            self.dump_dominator_tree(tree, child, indent + 2)
        }
    }

    pub fn start_point(&self, block: BasicBlockIndex) -> Point {
        Point { block: block, action: 0 }
    }

    pub fn end_point(&self, block: BasicBlockIndex) -> Point {
        let actions = self.graph.block_data(block).actions.len();
        Point { block: block, action: actions }
    }

    pub fn successor_points(&self, p: Point) -> Vec<Point> {
        let end_point = self.end_point(p.block);
        if p != end_point {
            vec![Point { block: p.block, action: p.action + 1 }]
        } else {
            self.graph.successors(p.block)
                      .map(|b| self.start_point(b))
                      .collect()
        }
    }
}

impl fmt::Debug for Point {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(fmt, "{:?}/{}", self.block, self.action)
    }
}
