use graph_algorithms::Graph;
use graph_algorithms::node_vec::NodeVec;
use graph::{BasicBlockIndex, FuncGraph};
use env::{Environment, Point};
use std::collections::BTreeMap;
use std::cmp;
use std::fmt;

/// A region is characterized by an entry point and a set of leaves.
/// The region contains all blocks B where:
/// - `entry dom B`
/// - and `exists leaf in leaves. B dom leaf`
#[derive(Clone, PartialEq, Eq)]
pub struct Region {
    entry: BasicBlockIndex,
    leaves: BTreeMap<BasicBlockIndex, usize>,
}

impl Region {
    pub fn with_point(point: Point) -> Self {
        // The minimal region containing point just has `point` as a tail.
        Self::new(point.block, Some(point))
    }

    pub fn new<I>(entry: BasicBlockIndex, leaves: I) -> Self
        where I: IntoIterator<Item = Point>
    {
        Region {
            entry: entry,
            leaves: leaves.into_iter()
                          .map(|p| (p.block, p.action))
                          .collect(),
        }
    }

    pub fn leaves(&self) -> Leaves {
        Leaves {
            iter: Box::new(self.leaves
                               .iter()
                               .map(|(&block, &action)| {
                                   Point {
                                       block: block,
                                       action: action,
                                   }
                               })),
        }
    }

    pub fn contains(&self, env: &Environment, point: Point) -> bool {
        log!("contains(self={:?}, point={:?})", self, point);

        // no leaves is the empty region, which contains no points
        if self.leaves.is_empty() {
            return false;
        }

        // point must be dominated by the entry
        if !env.dominators.is_dominated_by(point.block, self.entry) {
            log!("contains: {:?} not dominated by entry {:?}",
                 point.block,
                 self.entry);
            return false;
        }

        // point dominate one of the leaves; if it is in a tail block,
        // compare the actions, otherwise consult the dominator graph
        if let Some(&action) = self.leaves.get(&point.block) {
            log!("contains: {:?} is a tail upto {:?}", point.block, action);
            return point.action <= action;
        }
        log!("contains: comparing leaves");
        self.leaves
            .keys()
            .any(|&tail| env.dominators.is_dominated_by(tail, point.block))
    }

    pub fn add_point(&mut self, env: &Environment, point: Point) -> bool {
        log!("add_point(self={:?}, point={:?})", self, point);
        assert!(point.action < env.end_action(point.block));

        if self.contains(env, point) {
            return false;
        }

        // Update entry point to be the mutual dominator
        self.entry = env.dominators.mutual_dominator_node(self.entry, point.block);

        // Compute the minimal consistent region beginning at
        // `new_entry` that contains all the leaves and the new point.
        let mut rev_dfs = RevDfs::new(env, self.entry);
        for tail in self.leaves() {
            rev_dfs.walk(tail);
        }
        rev_dfs.walk(point);

        // Distill that into just the leaves and update our internal map.
        self.leaves.clear();
        let mut stack = vec![self.entry];
        while let Some(node) = stack.pop() {
            let upto_action = rev_dfs.contained[node];
            let end_action = env.end_action(node);

            log!("node {:?} contained up to action {:?} out of {:?}",
                 node,
                 upto_action,
                 end_action);

            assert!(upto_action <= end_action);
            if upto_action == end_action {
                // Block is fully contained; visit its children,
                // unless this is a leaf of the tree, in which case we
                // have to record is as one of our leaves.
                let children = env.dominator_tree.children(node);
                if children.is_empty() {
                    self.leaves.insert(node, upto_action - 1);
                } else {
                    stack.extend(children);
                }
                continue;
            }

            if upto_action == 0 {
                // Block is not contained at all. Stop walking.
                continue;
            }

            // Block is partially contained. Record as a tail but stop
            // walking.
            self.leaves.insert(node, upto_action - 1);
        }

        log!("add_point: leaves={:?}", self.leaves);
        assert!(self.contains(env, point));
        true
    }
}

struct RevDfs<'env, 'func: 'env, 'arena: 'func> {
    env: &'env Environment<'func, 'arena>,

    // for each block, stores the number of actions that are contained
    // in the set; hence a value of `0` means that a block is not
    // contained at all; a value of `1` means the first action is
    // contained but not any others; and a value of `N` where `N` is
    // the number of actions means the block is fully contained.
    contained: NodeVec<FuncGraph<'arena>, usize>,
    stack: Vec<BasicBlockIndex>,
    entry: BasicBlockIndex,
}

impl<'env, 'func, 'arena> RevDfs<'env, 'func, 'arena> {
    fn new(env: &'env Environment<'func, 'arena>, entry: BasicBlockIndex) -> Self {
        RevDfs {
            env: env,
            contained: NodeVec::from_default(env.graph),
            stack: vec![],
            entry: entry,
        }
    }

    fn walk(&mut self, mut start: Point) {
        log!("walk({:?})", start);
        assert!(self.env.dominators.is_dominated_by(start.block, self.entry));
        assert!(self.stack.is_empty());

        // convert `start` to be exclusive (one past the included point)
        start.action += 1;

        self.push(start);

        // Visit reachable predecessors, unless this is the region entry
        while let Some(p) = self.pop() {
            log!("walk: walking preds of {:?}", p);
            if p == self.entry {
                continue;
            }
            for pred_block in self.env.graph.predecessors(p) {
                self.push(self.env.end_point(pred_block));
            }
        }
    }

    fn push(&mut self, p: Point) {
        log!("push({:?})", p);
        if self.contained[p.block] > 0 {
            log!("push: already visited {:?} up to action {:?} out of {:?}, \
                  adjusting to {:?}",
                 p.block,
                 self.contained[p.block],
                 self.env.end_action(p.block),
                 cmp::max(p.action, self.contained[p.block]));
            self.contained[p.block] = cmp::max(p.action, self.contained[p.block]);
        } else {
            log!("push: enstacking {:?} up to action {:?}", p.block, p.action);
            self.contained[p.block] = p.action;
            self.stack.push(p.block); // still have to visit preds too
        }
    }

    fn pop(&mut self) -> Option<BasicBlockIndex> {
        self.stack.pop()
    }
}

pub struct Leaves<'iter> {
    iter: Box<Iterator<Item = Point> + 'iter>,
}

impl<'iter> Iterator for Leaves<'iter> {
    type Item = Point;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}

impl fmt::Debug for Region {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        try!(write!(fmt, "{{{:?} -> ", self.entry));
        for leaf in self.leaves() {
            try!(write!(fmt, "{:?}", leaf));
        }
        write!(fmt, "}}")
    }
}
