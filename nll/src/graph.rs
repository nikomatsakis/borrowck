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
    blocks: Vec<BasicBlockKind>,
    successors: Vec<Vec<BasicBlockIndex>>,
    predecessors: Vec<Vec<BasicBlockIndex>>,
    block_indices: BTreeMap<repr::BasicBlock, BasicBlockIndex>,
    skolemized_end_indices: BTreeMap<repr::RegionName, BasicBlockIndex>,
    skolemized_end_actions: BTreeMap<repr::RegionName, [repr::Action; 1]>,
}

#[derive(Copy, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct BasicBlockIndex {
    index: usize,
}

#[derive(Copy, Clone, Debug)]
pub enum BasicBlockKind {
    Code(repr::BasicBlock),
    SkolemizedEnd(repr::RegionName),
}

#[derive(Copy, Clone, Debug)]
pub enum BasicBlockData<'a> {
    Code(&'a repr::BasicBlockData),
    SkolemizedEnd(&'a [repr::Action]),
}

impl FuncGraph {
    pub fn new(func: repr::Func) -> Self {
        let blocks: Vec<_> = func.data
            .keys()
            .map(|&bb| BasicBlockKind::Code(bb))
            .chain(
                func.regions
                    .iter()
                    .map(|rd| BasicBlockKind::SkolemizedEnd(rd.name)),
            )
            .collect();
        let block_indices: BTreeMap<_, _> = func.data
            .keys()
            .cloned()
            .enumerate()
            .map(|(index, block)| (block, BasicBlockIndex { index: index }))
            .collect();
        let skolemized_end_indices: BTreeMap<_, _> = func.regions
            .iter()
            .enumerate()
            .map(|(index, rd)| {
                (
                    rd.name,
                    BasicBlockIndex {
                        index: index + block_indices.len(),
                    },
                )
            })
            .collect();
        let skolemized_end_actions: BTreeMap<_, _> = func.regions
            .iter()
            .map(|rd| {
                (
                    rd.name,
                    [
                        repr::Action {
                            kind: repr::ActionKind::SkolemizedEnd(rd.name),
                            should_have_error: false,
                        },
                    ],
                )
            })
            .collect();
        let mut predecessors: Vec<_> = (0..blocks.len()).map(|_| Vec::new()).collect();
        let mut successors: Vec<_> = (0..blocks.len()).map(|_| Vec::new()).collect();

        for (block, &index) in &block_indices {
            let data = &func.data[block];
            for successor in &data.successors {
                let successor_index = block_indices
                    .get(successor)
                    .cloned()
                    .unwrap_or_else(|| panic!("no index for {:?}", successor));
                successors[index.index].push(successor_index);
                predecessors[successor_index.index].push(index);
            }
        }

        let start_block = block_indices[&repr::BasicBlock::start()];

        FuncGraph {
            func,
            blocks,
            start_block,
            predecessors,
            successors,
            block_indices,
            skolemized_end_indices,
            skolemized_end_actions,
        }
    }

    pub fn block(&self, name: repr::BasicBlock) -> BasicBlockIndex {
        self.block_indices[&name]
    }

    pub fn skolemized_end(&self, name: repr::RegionName) -> BasicBlockIndex {
        self.skolemized_end_indices[&name]
    }

    pub fn block_data(&self, index: BasicBlockIndex) -> BasicBlockData {
        match self.blocks[index.index] {
            BasicBlockKind::Code(block) => BasicBlockData::Code(&self.func.data[&block]),
            BasicBlockKind::SkolemizedEnd(r) => BasicBlockData::SkolemizedEnd(
                &self.skolemized_end_actions[&r],
            ),
        }
    }

    pub fn free_regions(&self) -> &[repr::RegionDecl] {
        &self.func.regions
    }

    pub fn decls(&self) -> &[repr::VariableDecl] {
        &self.func.decls
    }

    pub fn assertions(&self) -> &[repr::Assertion] {
        &self.func.assertions
    }

    pub fn struct_decls(&self) -> &[repr::StructDecl] {
        &self.func.structs
    }
}

impl ga::Graph for FuncGraph {
    type Node = BasicBlockIndex;

    fn num_nodes(&self) -> usize {
        self.blocks.len()
    }

    fn start_node(&self) -> BasicBlockIndex {
        self.start_block
    }

    fn predecessors<'graph>(
        &'graph self,
        node: BasicBlockIndex,
    ) -> <Self as ga::GraphPredecessors<'graph>>::Iter {
        self.predecessors[node.index].iter().cloned()
    }

    fn successors<'graph>(
        &'graph self,
        node: BasicBlockIndex,
    ) -> <Self as ga::GraphSuccessors<'graph>>::Iter {
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

impl ga::NodeIndex for BasicBlockIndex {}

impl From<usize> for BasicBlockIndex {
    fn from(v: usize) -> BasicBlockIndex {
        BasicBlockIndex { index: v }
    }
}

impl Into<usize> for BasicBlockIndex {
    fn into(self) -> usize {
        self.index
    }
}

thread_local! {
    static NAMES: RefCell<Vec<BasicBlockKind>> = RefCell::new(vec![])
}

pub fn with_graph<OP, R>(g: &FuncGraph, op: OP) -> R
where
    OP: FnOnce() -> R,
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
                match names[self.index] {
                    BasicBlockKind::Code(bb) => write!(fmt, "{}", bb),
                    BasicBlockKind::SkolemizedEnd(rn) => write!(fmt, "{}", rn),
                }
            } else {
                write!(fmt, "BB{}", self.index)
            }
        })
    }
}

impl<'a> BasicBlockData<'a> {
    pub fn actions(self) -> &'a [repr::Action] {
        match self {
            BasicBlockData::Code(d) => &d.actions,
            BasicBlockData::SkolemizedEnd(actions) => actions,
        }
    }
}

