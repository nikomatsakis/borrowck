use lalrpop_intern::intern;
use graph_algorithms as ga;
use nll_repr::repr;
use std::collections::BTreeMap;
use std::cell::RefCell;
use std::fmt;
use std::mem;
use std::iter;
use std::slice;

pub struct FuncGraph {
    func: repr::Func,
    start_block: BasicBlockIndex,
    blocks: Vec<repr::BasicBlock>,
    successors: Vec<Vec<BasicBlockIndex>>,
    predecessors: Vec<Vec<BasicBlockIndex>>,
}

#[derive(Copy, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct BasicBlockIndex {
    index: usize
}

impl FuncGraph {
    pub fn new(func: repr::Func) -> Self {
        let blocks: Vec<_> =
            func.data.keys().cloned().collect();
        let block_indices: BTreeMap<_, _> =
            func.data.keys()
                     .cloned()
                     .enumerate()
                     .map(|(index, block)| (block, BasicBlockIndex {
                         index: index
                     }))
                     .collect();
        let mut predecessors: Vec<_> =
            (0..blocks.len()).map(|_| Vec::new())
                             .collect();
        let mut successors: Vec<_> =
            (0..blocks.len()).map(|_| Vec::new())
                             .collect();

        for (block, &index) in &block_indices {
            let data = &func.data[block];
            for successor in &data.successors {
                let successor_index = block_indices.get(successor)
                                                   .cloned()
                                                   .unwrap_or_else(|| {
                                                       panic!("no index for {:?}", successor)
                                                   });
                successors[index.index].push(successor_index);
                predecessors[successor_index.index].push(index);
            }
        }

        let start_name = intern("START");
        let start_block = block_indices[&repr::BasicBlock(start_name)];

        FuncGraph {
            func: func,
            blocks: blocks,
            start_block: start_block,
            predecessors: predecessors,
            successors: successors,
        }
    }

    pub fn block_data(&self, index: BasicBlockIndex) -> &repr::BasicBlockData {
        let block = self.blocks[index.index];
        &self.func.data[&block]
    }

    pub fn decls(&self) -> &[repr::VarDecl] {
        &self.func.decls
    }
}

impl ga::Graph for FuncGraph {
    type Node = BasicBlockIndex;

    fn num_nodes(&self) -> usize {
        self.func.data.len()
    }

    fn start_node(&self) -> BasicBlockIndex {
        self.start_block
    }

    fn predecessors<'graph>(&'graph self, node: BasicBlockIndex)
                            -> <Self as ga::GraphPredecessors<'graph>>::Iter {
        self.predecessors[node.index].iter().cloned()
    }

    fn successors<'graph>(&'graph self, node: BasicBlockIndex)
                          -> <Self as ga::GraphSuccessors<'graph>>::Iter {
        self.successors[node.index].iter().cloned()
    }
}

impl<'graph> ga::GraphPredecessors<'graph> for FuncGraph {
    type Item = BasicBlockIndex;
    type Iter = iter::Cloned<slice::Iter<'graph, BasicBlockIndex>>;
}

impl<'graph> ga::GraphSuccessors<'graph> for FuncGraph {
    type Item = BasicBlockIndex;
    type Iter = iter::Cloned<slice::Iter<'graph, BasicBlockIndex>>;
}

impl ga::NodeIndex for BasicBlockIndex {
}

impl From<usize> for BasicBlockIndex {
    fn from(v: usize) -> BasicBlockIndex {
        BasicBlockIndex {
            index: v
        }
    }
}

impl Into<usize> for BasicBlockIndex {
    fn into(self) -> usize {
        self.index
    }
}

thread_local! {
    static NAMES: RefCell<Vec<repr::BasicBlock>> = RefCell::new(vec![])
}

pub fn with_graph<OP, R>(g: &FuncGraph, op: OP)  -> R
    where OP: FnOnce() -> R
{
    NAMES.with(|names| {
        let old_names = mem::replace(&mut *names.borrow_mut(), g.blocks.clone());
        let result = op();
        *names.borrow_mut() = old_names;
        result
    })
}

impl fmt::Debug for BasicBlockIndex {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        NAMES.with(|names| {
            let names = names.borrow();
            if !names.is_empty() {
                write!(fmt, "{}", names[self.index].0)
            } else {
                write!(fmt, "BB{}", self.index)
            }
        })
    }
}



