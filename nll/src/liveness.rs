use graph::{BasicBlockIndex, FuncGraph};
use env::Environment;
use graph_algorithms::Graph;
use graph_algorithms::bit_set::BitSet;
use nll_repr::repr;
use std::collections::HashMap;

/// Compute the set of live variables at each point.
pub struct Liveness {
    var_bits: HashMap<repr::Variable, usize>,
    liveness: BitSet<FuncGraph>,
}

impl Liveness {
    pub fn new(env: &Environment) -> Liveness {
        let var_bits: HashMap<_, _> = env.graph.decls()
                                               .iter()
                                               .cloned()
                                               .zip(0..)
                                               .collect();
        let liveness = compute(env, &var_bits);
        Liveness { var_bits, liveness }
    }

    pub fn live_on_entry(&self, v: repr::Variable, b: BasicBlockIndex) -> bool {
        let bit = self.var_bits[&v];
        self.liveness.bits(b).get(bit)
    }
}

fn compute(env: &Environment,
           var_bits: &HashMap<repr::Variable, usize>)
           -> BitSet<FuncGraph> {
    let mut liveness = BitSet::new(env.graph, var_bits.len());
    let mut bits = liveness.empty_buf();

    let mut changed = true;
    while changed {
        changed = false;

        for &block in &env.reverse_post_order {
            bits.clear();

            // everything live in a successor is live at the exit of the block
            for succ in env.graph.successors(block) {
                bits.set_from(liveness.bits(succ));
            }

            // walk backwards through the actions
            for action in env.graph.block_data(block).actions.iter().rev() {
                let (def_var, use_var) = action.def_use();

                // anything we write to is no longer live
                for v in def_var {
                    bits.kill(var_bits[&v]);
                }

                // anything we read from, we make live
                for v in use_var {
                    bits.set(var_bits[&v]);
                }
            }

            changed |= liveness.insert_bits_from_slice(block, bits.as_slice());
        }
    }

    liveness
}

trait UseDefs {
    fn def_use(&self) -> (Vec<repr::Variable>, Vec<repr::Variable>);
}

impl UseDefs for repr::Action {
    fn def_use(&self) -> (Vec<repr::Variable>, Vec<repr::Variable>) {
        match *self {
            repr::Action::Borrow(v) => (vec!(v), vec!()),
            repr::Action::Assign(l, r) => (vec!(l), vec![r]),
            repr::Action::Subtype(a, b) => (vec!(), vec![a, b]),
            repr::Action::Use(v) => (vec!(), vec!(v)),
            repr::Action::Write(v) => (vec!(), vec!(v)),
            repr::Action::Noop => (vec!(), vec!()),
        }
    }
}
