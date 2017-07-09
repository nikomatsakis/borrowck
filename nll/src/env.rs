use graph::{BasicBlockIndex, FuncGraph};
use graph_algorithms::Graph;
use graph_algorithms::dominators::{self, Dominators, DominatorTree};
use graph_algorithms::iterate::reverse_post_order;
use graph_algorithms::loop_tree::{self, LoopTree};
use graph_algorithms::reachable::{self, Reachability};
use nll_repr::repr;
use std::collections::HashMap;
use std::fmt;

pub struct Environment<'func> {
    pub graph: &'func FuncGraph,
    pub dominators: Dominators<FuncGraph>,
    pub dominator_tree: DominatorTree<FuncGraph>,
    pub reachable: Reachability<FuncGraph>,
    pub loop_tree: LoopTree<FuncGraph>,
    pub reverse_post_order: Vec<BasicBlockIndex>,
    pub var_map: HashMap<repr::Variable, &'func repr::VariableDecl>,
    pub struct_map: HashMap<repr::StructName, &'func repr::StructDecl>,
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
        let var_map = graph.decls().iter().map(|vd| (vd.var, vd)).collect();
        let struct_map = graph
            .struct_decls()
            .iter()
            .map(|sd| (sd.name, sd))
            .collect();

        Environment {
            graph: graph,
            dominators: dominators,
            dominator_tree: dominator_tree,
            reachable: reachable,
            loop_tree: loop_tree,
            reverse_post_order: rpo,
            var_map: var_map,
            struct_map: struct_map,
        }
    }

    pub fn dump_dominators(&self) {
        let tree = self.dominators.dominator_tree();
        self.dump_dominator_tree(&tree, tree.root(), 0)
    }

    fn dump_dominator_tree<G1>(
        &self,
        tree: &DominatorTree<G1>,
        node: BasicBlockIndex,
        indent: usize,
    ) where
        G1: Graph<Node = BasicBlockIndex>,
    {
        println!("{0:1$}- {2:?}", "", indent, node);

        for &child in tree.children(node) {
            self.dump_dominator_tree(tree, child, indent + 2)
        }
    }

    pub fn start_point(&self, block: BasicBlockIndex) -> Point {
        Point {
            block: block,
            action: 0,
        }
    }

    pub fn end_point(&self, block: BasicBlockIndex) -> Point {
        let actions = self.graph.block_data(block).actions.len();
        Point {
            block: block,
            action: actions,
        }
    }

    pub fn successor_points(&self, p: Point) -> Vec<Point> {
        let end_point = self.end_point(p.block);
        if p != end_point {
            vec![
                Point {
                    block: p.block,
                    action: p.action + 1,
                },
            ]
        } else {
            self.graph
                .successors(p.block)
                .map(|b| self.start_point(b))
                .collect()
        }
    }

    pub fn var_ty(&self, v: repr::Variable) -> Box<repr::Ty> {
        match self.var_map.get(&v) {
            Some(decl) => decl.ty.clone(),
            None => panic!("no variable named {:?}", v),
        }
    }

    pub fn path_ty(&self, path: &repr::Path) -> Box<repr::Ty> {
        match *path {
            repr::Path::Base(v) => self.var_ty(v),
            repr::Path::Extension(ref base, field_name) => {
                let base_ty = self.path_ty(base);
                self.field_ty(&base_ty, field_name)
            }
        }
    }

    pub fn field_ty(&self, base_ty: &repr::Ty, field_name: repr::FieldName) -> Box<repr::Ty> {
        match *base_ty {
            repr::Ty::Ref(_, _kind, ref t) => {
                if field_name == repr::FieldName::star() {
                    t.clone()
                } else {
                    panic!("cannot index & with field `{:?}`, use `star`", field_name)
                }
            }

            repr::Ty::Unit => panic!("cannot index `()` type"),

            repr::Ty::Struct(n, ref parameters) => {
                let struct_decl = self.struct_map[&n];
                let field_decl = struct_decl
                    .fields
                    .iter()
                    .find(|fd| fd.name == field_name)
                    .unwrap_or_else(|| panic!("no field named `{:?}` in `{:?}`", field_name, n));
                let field_ty = &field_decl.ty;
                Box::new(field_ty.subst(parameters))
            }

            repr::Ty::Bound(_) => panic!("field_ty: unexpected bound type"),
        }
    }
}

impl fmt::Debug for Point {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(fmt, "{:?}/{}", self.block, self.action)
    }
}
