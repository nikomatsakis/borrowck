use graph::BasicBlockIndex;
use env::{Environment, Point};
use std::collections::BTreeMap;
use std::cmp;
use std::fmt;

/// A region is characterized by an entry point and a set of leaves.
///
/// The region contains all blocks B where:
/// - `entry dom B`
/// - and `exists leaf in leaves. B dom leaf`.
///
/// To be valid, two conditions must be met:
/// - entry must dominate all leaves
/// - there must not exist an edge U->V in the graph where:
///   - V is in the region and not the entry
///   - but U is not
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

        let mut changed = false;

        // Ensure that `self.entry` dominates `point.block`
        let new_entry = env.dominators.mutual_dominator_node(self.entry, point.block);
        if self.entry != new_entry {
            log!("add_point: new_entry={:?} self.entry={:?}", new_entry, self.entry);
            changed |= self.entry.update(new_entry);
        }

        // If `point.block` *is* one of the leaves, we may want to update
        // statement, but otherwise we're done.
        if let Some(action) = self.leaves.get_mut(&point.block) {
            log!("add_point: existing tail, action={}, point.action={}",
                 action, point.action);
            let max = cmp::max(*action, point.action);
            changed |= action.update(max);
            return changed;
        }

        // If `point.block` dominates any of the leaves, we're all done.
        let mut dead_leaves = vec![];
        for &leaf_block in self.leaves.keys() {
            if env.dominators.is_dominated_by(leaf_block, point.block) {
                return changed;
            } else if env.dominators.is_dominated_by(point.block, leaf_block) {
                // track the leaves that dominated the new point, see below
                dead_leaves.push(leaf_block);
            }
        }

        // Otherwise, have to add `point.block` as a new leaf.
        // If any of the old leaves dominate `point.block`, that means the
        // old leaf can be removed, because its inclusion is implied by the
        // presence of `point.block` as a leaf.
        for dead_leaf in dead_leaves {
            self.leaves.remove(&dead_leaf);
        }
        self.leaves.insert(point.block, point.action);
        return true;
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
        for (index, leaf) in self.leaves().enumerate() {
            if index > 0 { try!(write!(fmt, ", ")); }
            try!(write!(fmt, "{:?}", leaf));
        }
        write!(fmt, "}}")
    }
}

trait Update {
    fn update(&mut self, value: Self) -> bool;
}

impl<T: PartialEq> Update for T {
    fn update(&mut self, value: T) -> bool {
        if *self != value {
            *self = value;
            true
        } else {
            false
        }
    }
}

