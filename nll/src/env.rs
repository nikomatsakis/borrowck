use graph::{BasicBlockIndex, FuncGraph};
use graph_algorithms::Graph;
use graph_algorithms::dominators::{self, Dominators, DominatorTree};
use graph_algorithms::iterate::reverse_post_order;
use graph_algorithms::loop_tree::{self, LoopTree};
use graph_algorithms::reachable::{self, Reachability};
use graph_algorithms::transpose::TransposedGraph;
use std::fmt;

pub struct Environment<'func, 'arena: 'func> {
    pub graph: &'func FuncGraph<'arena>,
    pub dominators: Dominators<FuncGraph<'arena>>,
    pub dominator_tree: DominatorTree<FuncGraph<'arena>>,
    pub postdominators: Dominators<TransposedGraph<&'func FuncGraph<'arena>>>,
    pub reachable: Reachability<FuncGraph<'arena>>,
    pub loop_tree: LoopTree<FuncGraph<'arena>>,
    pub reverse_post_order: Vec<BasicBlockIndex>,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Point {
    pub block: BasicBlockIndex,
    pub action: usize,
}

impl<'func, 'arena> Environment<'func, 'arena> {
    pub fn new(graph: &'func FuncGraph<'arena>) -> Self {
        let rpo = reverse_post_order(graph, graph.start_node());
        let dominators = dominators::dominators_given_rpo(graph, &rpo);
        let dominator_tree = dominators.dominator_tree();
        let reachable = reachable::reachable_given_rpo(graph, &rpo);
        let loop_tree = loop_tree::loop_tree_given(graph, &dominators);

        let postdominators = {
            let exit = graph.block_index_str("EXIT");
            let transpose = &TransposedGraph::with_start(graph, exit);
            dominators::dominators(transpose)
        };

        Environment {
            graph: graph,
            dominators: dominators,
            dominator_tree: dominator_tree,
            postdominators: postdominators,
            reachable: reachable,
            loop_tree: loop_tree,
            reverse_post_order: rpo,
        }
    }

    pub fn dump_dominators(&self) {
        let tree = self.dominators.dominator_tree();
        self.dump_dominator_tree(&tree, tree.root(), 0)
    }

    pub fn dump_postdominators(&self) {
        let tree = self.postdominators.dominator_tree();
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

    pub fn last_action(&self, block: BasicBlockIndex) -> usize {
        self.graph.block_data(block).actions.len()
    }

    pub fn end_action(&self, block: BasicBlockIndex) -> usize {
        self.last_action(block) + 1
    }

    pub fn end_point(&self, block: BasicBlockIndex) -> Point {
        Point {
            block: block,
            action: self.end_action(block)
        }
    }

    pub fn can_reach(&self, pred: Point, succ: Point) -> bool {
        if pred.block == succ.block {
            pred.action <= succ.action
        } else {
            self.reachable.can_reach(pred.block, succ.block)
        }
    }
}

impl fmt::Debug for Point {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(fmt, "({:?} @ {})", self.block, self.action)
    }
}
