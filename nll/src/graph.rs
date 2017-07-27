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

        let roots: Vec<_> = RegionRoots::extract(&func.regions)
            .iter()
            .map(|rn| skolemized_end_indices[rn])
            .collect();

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

            // Every block with no successors (and hence representing
            // a RETURN or a RESUME point) has, as a successor, the
            // root skolemized end points.
            if data.successors.is_empty() {
                for &root in &roots {
                    successors[index.index].push(root);
                    predecessors[root.index].push(index);
                }
            }
        }

        // Each `'a: 'b` relationship induces an edge `'b -> 'a`
        for a_decl in &func.regions {
            let a_index = skolemized_end_indices[&a_decl.name];

            for b_name in &a_decl.outlives {
                let b_index = skolemized_end_indices[b_name];

                successors[b_index.index].push(a_index);
                predecessors[a_index.index].push(b_index);
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

struct RegionRoots<'a> {
    stack: Vec<repr::RegionName>,
    successors: &'a BTreeMap<repr::RegionName, Vec<repr::RegionName>>,
    state: BTreeMap<repr::RegionName, RegionState>,
}

struct RegionState {
    /// Does this region have a (non-cyclic) predecessor?
    has_pred: bool,
    visited: bool,
}

impl<'a> RegionRoots<'a> {
    fn extract(regions: &[repr::RegionDecl]) -> Vec<repr::RegionName> {
        // This is a bit tricky. Given something like `'a: 'b`, we want to produce
        //
        //   END -> 'b
        //   'b -> 'a
        //
        // But not END -> 'a, I don't think, because that would imply
        // that one can reach the end of 'a *without* passing through
        // the end of 'b!
        //
        // But be wary of `'a: 'b` and `'b: 'a`, in which case we need
        // an edge to one *or* the other, at least!
        //
        // So what are the ROOTS of the graph? We can create a GRAPH
        // with nodes equal to skolemized points and edges `'b -> 'a`
        // if `'a: 'b`. We then do a DFS.

        // Create a state for every region.
        let state: BTreeMap<_, _> = regions
            .iter()
            .map(|rd| {
                (
                    rd.name,
                    RegionState {
                        has_pred: false,
                        visited: false,
                    },
                )
            })
            .collect();

        // Create the edges. If `'a: 'b`, then `'b -> 'a`.
        let mut successors = BTreeMap::new();
        for rd in regions {
            for &rn_outlives in &rd.outlives {
                successors
                    .entry(rn_outlives)
                    .or_insert(vec![])
                    .push(rd.name);
            }
        }

        let stack = vec![];

        let mut roots = RegionRoots {
            state,
            successors: &successors,
            stack,
        };

        for rd in regions {
            roots.dfs(rd.name);
        }

        roots
            .state
            .iter()
            .filter(|&(_, state)| !state.has_pred)
            .map(|(&key, _)| key)
            .collect()
    }

    fn dfs(&mut self, region_name: repr::RegionName) {
        // Did we uncover a cycle? Then just stop.
        //
        // Hence if we have A -> B, B -> A and we visit A first and
        // then B:
        // - We will mark B as "has pred" but not A.
        // - Both will be marked as "visited", and hence when we next
        //   visit B, we'll ignore it.
        if self.stack.contains(&region_name) {
            return;
        }

        // Otherwise, if this node has a predecessor, mark it as a non-root.
        {
            let mut state = self.state.get_mut(&region_name).unwrap();

            if !self.stack.is_empty() {
                state.has_pred = true;
            }

            if state.visited {
                return;
            }

            state.visited = true;
        }

        // Visit successors now.
        self.stack.push(region_name);
        for &succ in self.successors
            .get(&region_name)
            .into_iter()
            .flat_map(|v| v)
        {
            self.dfs(succ);
        }
        self.stack.pop();
    }
}

#[test]
#[allow(bad_style)]
fn root_cycle() {
    let A = repr::RegionName::from("'a");
    let B = repr::RegionName::from("'b");
    let regions = vec![
        repr::RegionDecl {
            name: A,
            outlives: vec![B],
        },
        repr::RegionDecl {
            name: B,
            outlives: vec![A],
        },
    ];

    let roots = RegionRoots::extract(&regions);

    assert_eq!(roots, vec![A]);
}

#[test]
#[allow(bad_style)]
fn root_cycle_reachable_from_outside() {
    let A = repr::RegionName::from("'a");
    let B = repr::RegionName::from("'b");
    let C = repr::RegionName::from("'c");
    let regions = vec![
        repr::RegionDecl {
            name: A,
            outlives: vec![B, C],
        },
        repr::RegionDecl {
            name: B,
            outlives: vec![A],
        },
        repr::RegionDecl {
            name: C,
            outlives: vec![],
        },
    ];

    let roots = RegionRoots::extract(&regions);

    assert_eq!(roots, vec![C]);
}
