use graph_algorithms::Graph;
use graph_algorithms::node_vec::NodeVec;
use graph::{BasicBlockIndex, FuncGraph};
use env::{Environment, Point};
use std::collections::BTreeMap;
use std::cmp;

/// A region is fully characterized by a set of exits.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Region {
    exits: BTreeMap<BasicBlockIndex, usize>,
}

impl Region {
    pub fn with_point(env: &Environment, point: Point) -> Self {
        // The minimal region containing point is to walk up the
        // dominator tree, excluding all "uncles/aunts" from the
        // minimal loop containing point.
        let entry = env.interval_head(point.block);
        let mut exits = BTreeMap::new();
        exits.insert(point.block, point.action + 1);
        let mut p = point.block;
        while p != entry {
            let dom = env.dominators.immediate_dominator(p);
            for node in env.dominator_tree.iter_children_of(dom) {
                if node != p {
                    exits.insert(node, 0);
                }
            }
            p = dom;
        }
        Self::new(exits)
    }

    pub fn with_exits<P>(exits: P) -> Self
        where P: IntoIterator<Item = Point>
    {
        let map = exits.into_iter().map(|p| (p.block, p.action)).collect();
        Region::new(map)
    }

    pub fn new(exits: BTreeMap<BasicBlockIndex, usize>) -> Self {
        assert!(!exits.is_empty());
        Region { exits: exits }
    }

    pub fn exits(&self) -> &BTreeMap<BasicBlockIndex, usize> {
        &self.exits
    }

    pub fn entry(&self, env: &Environment) -> BasicBlockIndex {
        env.mutual_interval(self.exits.keys().cloned()).unwrap()
    }

    pub fn contains(&self, env: &Environment, point: Point) -> bool {
        log!("contains(self={:?}, point={:?})", self, point);

        // Check if point is an exit block. In that case, we have to compare
        // the action index.
        if let Some(&upto_action) = self.exits.get(&point.block) {
            log!("contains: exit {}", upto_action);
            return point.action < upto_action;
        }

        // Otherwise, check whether it is dominated by the entry.  If
        // not, it is certainly not in the region.
        let entry = self.entry(env);
        log!("contains: entry={:?}", entry);
        if !env.dominators.is_dominated_by(point.block, entry) {
            log!("contains: not dominated by entry");
            return false;
        }

        // If it is in the region, it must lie between the exits and the entry.
        // So walk up the dominator tree and see what we find first.
        let mut p = point.block;
        loop {
            log!("contains: p={:?}", p);
            if self.exits.contains_key(&p) {
                return false;
            }
            if p == entry {
                break;
            } else {
                p = env.dominators.immediate_dominator(p);
            }
        }

        log!("contains: done");
        true
    }

    pub fn add_point(&mut self, env: &Environment, point: Point) -> bool {
        assert!(point.action < env.end_action(point.block));

        if self.contains(env, point) {
            return false;
        }

        // Grow the region in a minimal way so that it contains
        // `block`.
        log!("add_point: exits={:?} point={:?}", self.exits, point);
        let mut contained_nodes = self.contained_nodes(env);
        let new_head = env.mutual_interval(self.exits
                                               .keys()
                                               .cloned()
                                               .chain(Some(point.block)))
                          .unwrap();

        log!("add_point: new_head={:?}", new_head);

        contained_nodes[point.block] = cmp::max(point.action + 1, contained_nodes[point.block]);

        let mut changed = true;
        while changed {
            changed = false;

            log!("propagate");
            for node in env.dominator_tree.iter_children_of(new_head).skip(1) {
                log!("propagate: node={:?}/{:?} end-action={} contained={}",
                     node,
                     env.graph.block_name(node),
                     env.end_action(node),
                     contained_nodes[node]);
                if contained_nodes[node] > 0 {
                    for pred in env.graph.predecessors(node) {
                        let pred_actions = env.end_action(pred);
                        log!("propagate: pred={:?}/{:?} pred_actions={} \
                                  contained={}",
                             pred,
                             env.graph.block_name(pred),
                             pred_actions,
                             contained_nodes[pred]);
                        if contained_nodes[pred] != pred_actions {
                            contained_nodes[pred] = pred_actions;
                            changed = true;
                        }
                    }
                }
            }
        }

        assert!(contained_nodes[new_head] == env.end_action(new_head));

        self.exits.clear();
        let mut stack = vec![new_head];
        while let Some(node) = stack.pop() {
            if contained_nodes[node] < env.end_action(node) {
                self.exits.insert(node, contained_nodes[node]);
            } else {
                stack.extend(env.dominator_tree.children(node));
            }
        }

        log!("add_point: exits={:?}", self.exits);
        true
    }

    /// Returns a vector such that `v[x] = a` means that all action in
    /// `x` up to (but not including) action a are included.
    pub fn contained_nodes<'func, 'arena>(&self,
                                          env: &Environment<'func, 'arena>)
                                          -> NodeVec<FuncGraph<'arena>, usize> {
        let mut contained = NodeVec::from_default(env.graph);
        let entry = self.entry(env);
        log!("contained_nodes: entry={:?} / {:?}",
             entry,
             env.graph.block_name(entry));
        let mut stack = vec![entry];
        while let Some(node) = stack.pop() {
            log!("contained_nodes: node={:?}", node);

            if let Some(&upto_action) = self.exits.get(&node) {
                log!("contained_nodes: exit at {}", upto_action);
                contained[node] = upto_action;
                continue;
            }

            contained[node] = env.end_action(node);
            stack.extend(env.dominator_tree.children(node));
        }
        log!("contained_nodes: contained_nodes={:?}", contained.vec);
        contained
    }
}
