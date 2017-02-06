#![allow(dead_code)]

use graph::{BasicBlockIndex, FuncGraph};
use graph_algorithms::node_vec::NodeVec;
use nll_repr::repr;
use region_map::RegionVariable;
use std::collections::HashMap;

pub struct TypeMap {
    per_blocks: NodeVec<FuncGraph, PerBlockAssignments>,
}

pub struct PerBlockAssignments {
    pub entry: Assignments,
    pub exit: Assignments,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Assignments {
    map: HashMap<repr::Variable, repr::Ty<RegionVariable>>,
}

impl TypeMap {
    pub fn new(graph: &FuncGraph) -> Self {
        TypeMap {
            per_blocks: NodeVec::from_default(graph),
        }
    }

    pub fn assignments_mut(&mut self, blk: BasicBlockIndex) -> &mut PerBlockAssignments {
        &mut self.per_blocks[blk]
    }

    pub fn assignments(&self, blk: BasicBlockIndex) -> &PerBlockAssignments {
        &self.per_blocks[blk]
    }
}

impl Default for PerBlockAssignments {
    fn default() -> Self {
        PerBlockAssignments {
            entry: Assignments::new(),
            exit: Assignments::new(),
        }
    }
}

impl Assignments {
    fn new() -> Self {
        Assignments { map: HashMap::new() }
    }

    pub fn set_var(&mut self,
                   name: repr::Variable,
                   ty: repr::Ty<RegionVariable>)
    {
        self.map.insert(name, ty);
    }

    pub fn get(&self, v: repr::Variable) -> &repr::Ty<RegionVariable> {
        self.map.get(&v).unwrap_or_else(|| panic!("no variable `{:?}`", v))
    }
}
