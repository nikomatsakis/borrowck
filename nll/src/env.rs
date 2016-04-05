use graph::{BasicBlockIndex, FuncGraph};
use graph_algorithms::Graph;
use graph_algorithms::dominators::{self, Dominators, DominatorTree};
use graph_algorithms::iterate::reverse_post_order;
use graph_algorithms::loop_tree::{self, LoopTree};
use graph_algorithms::reachable::{self, Reachability};
use graph_algorithms::transpose::TransposedGraph;

pub struct Environment<'func, 'arena: 'func> {
    pub graph: &'func FuncGraph<'arena>,
    pub dominators: Dominators<FuncGraph<'arena>>,
    pub postdominators: Dominators<TransposedGraph<&'func FuncGraph<'arena>>>,
    pub reachable: Reachability<FuncGraph<'arena>>,
    pub loop_tree: LoopTree<FuncGraph<'arena>>,
    pub reverse_post_order: Vec<BasicBlockIndex>,
}

impl<'func, 'arena> Environment<'func, 'arena> {
    pub fn new(graph: &'func FuncGraph<'arena>) -> Self {
        let rpo = reverse_post_order(graph, graph.start_node());
        let dominators = dominators::dominators_given_rpo(graph, &rpo);
        let reachable = reachable::reachable_given_rpo(graph, &rpo);
        let loop_tree = loop_tree::loop_tree_given(graph, &dominators);

        let postdominators = {
            let exit = graph.block_index("EXIT");
            let transpose = &TransposedGraph::with_start(graph, exit);
            dominators::dominators(transpose)
        };

        Environment {
            graph: graph,
            dominators: dominators,
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
                 self.graph.block_name(node));

        for &child in tree.children(node) {
            self.dump_dominator_tree(tree, child, indent + 2)
        }
    }
}
