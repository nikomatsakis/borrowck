use graph_algorithms::Graph;
use graph_algorithms::node_vec::NodeVec;
use graph::{BasicBlockIndex, FuncGraph};
use env::{Environment, Point};
use std::collections::HashMap;
use std::cmp;
use std::fmt;

/// A region is fully characterized by a set of exits.
#[derive(Debug)]
pub struct Region {
    exits: HashMap<BasicBlockIndex, usize>,
}

impl Region {
    pub fn with_point(point: Point) -> Self {
        let map = Some((point.block, point.action + 1)).into_iter().collect();
        Region::new(map)
    }

    pub fn new(exits: HashMap<BasicBlockIndex, usize>) -> Self {
        assert!(!exits.is_empty());
        Region { exits: exits }
    }

    pub fn exits(&self) -> &HashMap<BasicBlockIndex, usize> {
        &self.exits
    }

    pub fn entry(&self, env: &Environment) -> BasicBlockIndex {
        env.mutual_interval(self.exits.keys().cloned()).unwrap()
    }

    pub fn add_point(&mut self, env: &Environment, point: Point) {
        // Grow the region in a minimal way so that it contains
        // `block`.
        assert!(point.action > 0);

        println!("add_point: exits={:?} point={:?}", self.exits, point);
        let mut contained_nodes = self.contained_nodes(env);
        let new_head = env.mutual_interval(self.exits
                                               .keys()
                                               .cloned()
                                               .chain(Some(point.block)))
                          .unwrap();

        println!("add_point: new_head={:?}", new_head);

        contained_nodes[point.block] =
            cmp::max(point.action, contained_nodes[point.block]);

        let mut changed = true;
        while changed {
            changed = false;

            println!("propagate");
            for node in env.dominator_tree.iter_children_of(new_head).skip(1) {
                println!("propagate: node={:?}/{:?} end-action={} contained={}",
                         node,
                         env.graph.block_name(node),
                         env.end_action(node),
                         contained_nodes[node]);
                if contained_nodes[node] > 0 {
                    for pred in env.graph.predecessors(node) {
                        let pred_actions = env.end_action(pred);
                        println!("propagate: pred={:?}/{:?} pred_actions={} \
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

        println!("add_point: exits={:?}", self.exits);
    }

    /// Returns a vector such that `v[x] = a` means that all action in
    /// `x` up to (but not including) action a are included.
    pub fn contained_nodes<'func, 'arena>(&self,
                                          env: &Environment<'func, 'arena>)
                                          -> NodeVec<FuncGraph<'arena>, usize> {
        let mut contained = NodeVec::from_default(env.graph);
        let entry = self.entry(env);
        println!("contained_nodes: entry={:?} / {:?}",
                 entry, env.graph.block_name(entry));
        let mut stack = vec![entry];
        while let Some(node) = stack.pop() {
            println!("contained_nodes: node={:?}", node);

            if let Some(&upto_action) = self.exits.get(&node) {
                println!("contained_nodes: exit at {}", upto_action);
                contained[node] = upto_action;
                continue;
            }

            contained[node] = env.end_action(node);
            stack.extend(env.dominator_tree.children(node));
        }
        println!("contained_nodes: contained_nodes={:?}",
                 contained.vec);
        contained
    }
}

