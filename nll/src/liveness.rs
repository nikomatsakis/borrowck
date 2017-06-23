use graph::{BasicBlockIndex, FuncGraph};
use env::{Environment, Point};
use graph_algorithms::Graph;
use graph_algorithms::bit_set::{BitBuf, BitSet, BitSlice};
use nll_repr::repr;
use std::collections::HashMap;
use std::iter::once;

/// Compute the set of live variables at each point.
pub struct Liveness {
    var_regions: HashMap<repr::Variable, Vec<repr::RegionName>>,
    var_bits: HashMap<repr::RegionName, usize>,
    liveness: BitSet<FuncGraph>,
}

impl Liveness {
    pub fn new(env: &Environment) -> Liveness {
        let var_regions: HashMap<_, Vec<_>> =
            env.graph.decls()
                     .iter()
                     .map(|d| (
                         d.var,
                         d.ty.walk_regions().collect()
                     ))
                     .collect();
        let mut region_names: Vec<_> = var_regions.iter()
                                                  .flat_map(|(_, ref names)| names.iter())
                                                  .cloned()
                                                  .collect();
        region_names.sort();
        region_names.dedup();
        let var_bits: HashMap<_, _> = region_names.iter()
                                                  .cloned()
                                                  .zip(0..)
                                                  .collect();
        let liveness = BitSet::new(env.graph, var_bits.len());
        let mut this = Liveness { var_regions, var_bits, liveness };
        this.compute(env);
        this
    }

    pub fn var_live_on_entry(&self, var_name: repr::Variable, b: BasicBlockIndex) -> bool {
        self.var_regions[&var_name]
            .iter()
            .map(|rn| self.var_bits[rn])
            .all(|bit| self.liveness.bits(b).get(bit))
    }

    pub fn live_regions<'a>(&'a self, live_bits: BitSlice<'a>)
                            -> impl Iterator<Item = repr::RegionName> + 'a {
        self.var_bits
            .iter()
            .filter(move |&(_, &bit)| live_bits.get(bit))
            .map(move |(&region_name, _)| region_name)
    }

    /// Invokes callback once for each action with (A) the point of
    /// the action; (B) the action itself and (C) the set of live
    /// variables on entry to the action.
    pub fn walk<CB>(&self,
                    env: &Environment,
                    mut callback: CB)
        where CB: FnMut(Point, Option<&repr::Action>, BitSlice)
    {
        let mut bits = self.liveness.empty_buf();
        for &block in &env.reverse_post_order {
            self.simulate_block(env, &mut bits, block, &mut callback);
        }
    }

    fn compute(&mut self, env: &Environment) {
        let mut bits = self.liveness.empty_buf();
        let mut changed = true;
        while changed {
            changed = false;

            for &block in &env.reverse_post_order {
                self.simulate_block(env, &mut bits, block, |_p, _a, _s| ());
                changed |= self.liveness.insert_bits_from_slice(block, bits.as_slice());
            }
        }
    }

    fn simulate_block<CB>(&self,
                          env: &Environment,
                          buf: &mut BitBuf,
                          block: BasicBlockIndex,
                          mut callback: CB)
        where CB: FnMut(Point, Option<&repr::Action>, BitSlice)
    {
        buf.clear();

        // everything live in a successor is live at the exit of the block
        for succ in env.graph.successors(block) {
            buf.set_from(self.liveness.bits(succ));
        }

        // callback for the "goto" point
        callback(env.end_point(block), None, buf.as_slice());

        // walk backwards through the actions
        for (index, action) in env.graph.block_data(block).actions.iter().enumerate().rev() {
            let (def_var, use_var) = action.def_use();

            // anything we write to is no longer live
            for v in def_var {
                for rn in &self.var_regions[&v] {
                    buf.kill(self.var_bits[&rn]);
                }
            }

            // any variables we read from, we make live
            for v in use_var {
                let var_ty = env.var_ty(v);
                self.use_ty(buf, var_ty);
            }

            // drop is special
            if let repr::Action::Drop(ref path) = *action {
                let path_ty = env.path_ty(path);
                self.drop_ty(buf, env, path_ty);
            }

            let point = Point { block, action: index };
            callback(point, Some(action), buf.as_slice());
        }
    }

    fn use_ty(&self, buf: &mut BitBuf, ty: &repr::Ty) {
        for region_name in ty.walk_regions() {
            self.use_region(buf, region_name);
        }
    }

    fn use_region(&self, buf: &mut BitBuf, region_name: repr::RegionName) {
        buf.set(self.var_bits[&region_name]);
    }

    fn drop_ty(&self, buf: &mut BitBuf, env: &Environment, ty: &repr::Ty) {
        match *ty {
            repr::Ty::Ref(..) | repr::Ty::RefMut(..) | repr::Ty::Unit => {
                // Dropping a reference (or `()`) does not require it to be live; it's a no-op.
            }

            repr::Ty::Struct(struct_name, ref params) => {
                let struct_decl = env.struct_map[&struct_name];
                assert_eq!(struct_decl.parameters.len(), params.len());
                for (param_decl, param) in struct_decl.parameters.iter().zip(params.iter()) {
                    match *param {
                        repr::TyParameter::Region(region_name) => {
                            if !param_decl.may_dangle {
                                self.use_region(buf, region_name);
                            }
                        }

                        repr::TyParameter::Ty(ref ty) => {
                            if !param_decl.may_dangle {
                                self.use_ty(buf, ty);
                            } else {
                                self.drop_ty(buf, env, ty);
                            }
                        }
                    }
                }
            }
        }
    }
}

trait UseDefs {
    fn def_use(&self) -> (Vec<repr::Variable>, Vec<repr::Variable>);
}

impl UseDefs for repr::Action {
    fn def_use(&self) -> (Vec<repr::Variable>, Vec<repr::Variable>) {
        match *self {
            repr::Action::Borrow(ref v, _name) => (vec![v.base()], vec![]),
            repr::Action::Init(ref a, ref params) => {
                (a.write_def().into_iter().collect(),
                 params.iter().map(|p| p.base()).chain(a.write_use()).collect())
            }
            repr::Action::Assign(ref a, ref b) => {
                (a.write_def().into_iter().collect(),
                 once(b.base()).chain(a.write_use()).collect())
            }
            repr::Action::Constraint(ref _c) => (vec!(), vec!()),
            repr::Action::Use(ref v) => (vec!(), vec!(v.base())),
            repr::Action::Write(ref v) => (vec!(), vec!(v.base())),

            // drop is special
            repr::Action::Drop(..) => (vec!(), vec!()),

            repr::Action::Noop => (vec!(), vec!()),
        }
    }
}
