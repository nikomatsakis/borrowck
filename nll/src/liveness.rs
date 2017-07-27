use env::{Environment, Point};
use graph::{BasicBlockIndex, FuncGraph};
use graph_algorithms::Graph;
use graph_algorithms::bit_set::{BitBuf, BitSet, BitSlice};
use nll_repr::repr;
use std::collections::{BTreeSet, HashMap};
use std::iter::once;

/// Compute the set of live variables at each point.
pub struct Liveness<'env> {
    env: &'env Environment<'env>,
    bits: Vec<BitKind>,
    bits_map: HashMap<BitKind, usize>,
    liveness: BitSet<FuncGraph>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum BitKind {
    /// If this bit is set, current value of the variable will be **used** later on.
    VariableUsed(repr::Variable),

    /// If this bit is set, current value of the variable will be **dropped** later on.
    VariableDrop(repr::Variable),

    /// If this bit is set, then the given free region will be
    /// **used**.
    FreeRegion(repr::RegionName),
}

impl<'env> Liveness<'env> {
    pub fn new(env: &'env Environment<'env>) -> Liveness {
        let bits: Vec<_> = {
            let used_bits = env.graph
                .decls()
                .iter()
                .map(|d| BitKind::VariableUsed(d.var));
            let drop_bits = env.graph
                .decls()
                .iter()
                .map(|d| BitKind::VariableDrop(d.var));
            let free_region_bits = env.graph
                .free_regions()
                .iter()
                .cloned()
                .map(|rd| BitKind::FreeRegion(rd.name));
            used_bits.chain(drop_bits).chain(free_region_bits).collect()
        };

        let bits_map: HashMap<_, _> = bits.iter()
            .cloned()
            .enumerate()
            .map(|(index, bk)| (bk, index))
            .collect();

        let liveness = BitSet::new(env.graph, bits.len());
        let mut this = Liveness {
            env,
            bits,
            liveness,
            bits_map,
        };
        this.compute();
        this
    }

    pub fn var_live_on_entry(&self, var_name: repr::Variable, b: BasicBlockIndex) -> bool {
        let bit = self.bits_map[&BitKind::VariableUsed(var_name)];
        self.liveness.bits(b).get(bit)
    }

    pub fn region_live_on_entry(&self, region_name: repr::RegionName, b: BasicBlockIndex) -> bool {
        let set = self.regions_set(self.liveness.bits(b));
        set.contains(&region_name)
    }

    pub fn live_regions<'a>(
        &'a self,
        live_bits: BitSlice<'a>,
    ) -> impl Iterator<Item = repr::RegionName> + 'a {
        self.regions_set(live_bits).into_iter()
    }

    fn regions_set(&self, live_bits: BitSlice) -> BTreeSet<repr::RegionName> {
        let mut set = BTreeSet::new();
        for (index, &bk) in self.bits.iter().enumerate() {
            if live_bits.get(index) {
                match bk {
                    BitKind::VariableUsed(v) => {
                        let var_ty = &self.env.var_ty(v);
                        self.use_ty(&mut set, var_ty);
                    }

                    BitKind::VariableDrop(v) => {
                        let var_ty = &self.env.var_ty(v);
                        self.drop_ty(&mut set, var_ty);
                    }

                    BitKind::FreeRegion(rn) => {
                        self.use_region(&mut set, rn);
                    }
                }
            }
        }
        set
    }

    /// Invokes callback once for each action with (A) the point of
    /// the action; (B) the action itself and (C) the set of live
    /// variables on entry to the action.
    pub fn walk<CB>(&self, mut callback: CB)
    where
        CB: FnMut(Point, Option<&repr::Action>, BitSlice),
    {
        let mut bits = self.liveness.empty_buf();
        for &block in &self.env.reverse_post_order {
            self.simulate_block(&mut bits, block, &mut callback);
        }
    }

    fn compute(&mut self) {
        let mut bits = self.liveness.empty_buf();
        let mut changed = true;
        while changed {
            changed = false;

            for &block in &self.env.reverse_post_order {
                self.simulate_block(&mut bits, block, |_p, _a, _s| ());
                changed |= self.liveness.insert_bits_from_slice(block, bits.as_slice());
            }
        }
    }

    fn simulate_block<CB>(&self, buf: &mut BitBuf, block: BasicBlockIndex, mut callback: CB)
    where
        CB: FnMut(Point, Option<&repr::Action>, BitSlice),
    {
        buf.clear();

        // everything live in a successor is live at the exit of the block
        for succ in self.env.graph.successors(block) {
            buf.set_from(self.liveness.bits(succ));
        }

        // callback for the "goto" point
        callback(self.env.end_point(block), None, buf.as_slice());

        // walk backwards through the actions
        for (index, action) in self.env
            .graph
            .block_data(block)
            .actions()
            .iter()
            .enumerate()
            .rev()
        {
            let (def_var, use_var) = action.def_use();

            // anything we write to is no longer live
            for v in def_var {
                buf.kill(self.bits_map[&BitKind::VariableUsed(v)]);
                buf.kill(self.bits_map[&BitKind::VariableDrop(v)]);
            }

            // any variables we read from, we make live
            for v in use_var {
                buf.set(self.bits_map[&BitKind::VariableUsed(v)]);
            }

            // some actions are special
            match action.kind {
                repr::ActionKind::Drop(ref path) => {
                    buf.set(self.bits_map[&BitKind::VariableDrop(path.base())]);
                }
                repr::ActionKind::SkolemizedEnd(name) => {
                    buf.set(self.bits_map[&BitKind::FreeRegion(name)]);
                }
                _ => {}
            }

            let point = Point {
                block,
                action: index,
            };
            callback(point, Some(action), buf.as_slice());
        }
    }

    fn use_ty(&self, buf: &mut BTreeSet<repr::RegionName>, ty: &repr::Ty) {
        for region_name in ty.walk_regions().map(|r| r.assert_free()) {
            self.use_region(buf, region_name);
        }
    }

    fn use_region(&self, buf: &mut BTreeSet<repr::RegionName>, region_name: repr::RegionName) {
        buf.insert(region_name);
    }

    fn drop_ty(&self, buf: &mut BTreeSet<repr::RegionName>, ty: &repr::Ty) {
        match *ty {
            repr::Ty::Ref(..) |
            repr::Ty::Unit => {
                // Dropping a reference (or `()`) does not require it to be live; it's a no-op.
            }

            repr::Ty::Struct(struct_name, ref params) => {
                let struct_decl = self.env.struct_map[&struct_name];
                assert_eq!(struct_decl.parameters.len(), params.len());
                for (param_decl, param) in struct_decl.parameters.iter().zip(params.iter()) {
                    match *param {
                        repr::TyParameter::Region(region) => {
                            if !param_decl.may_dangle {
                                self.use_region(buf, region.assert_free());
                            }
                        }

                        repr::TyParameter::Ty(ref ty) => {
                            if !param_decl.may_dangle {
                                self.use_ty(buf, ty);
                            } else {
                                self.drop_ty(buf, ty);
                            }
                        }
                    }
                }
            }

            repr::Ty::Bound(_) => panic!("drop_ty: unexpected bound type {:?}", ty),
        }
    }
}

pub trait DefUse {
    /// Returns (defs, uses), where `defs` contains variables whose
    /// current value is completely overwritten, and `uses` contains
    /// variables whose current value is used. Note that a variable
    /// may exist in both sets.
    fn def_use(&self) -> (Vec<repr::Variable>, Vec<repr::Variable>);
}

impl DefUse for repr::Action {
    fn def_use(&self) -> (Vec<repr::Variable>, Vec<repr::Variable>) {
        match self.kind {
            repr::ActionKind::Borrow(ref p, _name, _, ref q) => (vec![p.base()], vec![q.base()]),
            repr::ActionKind::Init(ref a, ref params) => {
                (
                    a.write_def().into_iter().collect(),
                    params
                        .iter()
                        .map(|p| p.base())
                        .chain(a.write_use())
                        .collect(),
                )
            }
            repr::ActionKind::Assign(ref a, ref b) => {
                (
                    a.write_def().into_iter().collect(),
                    once(b.base()).chain(a.write_use()).collect(),
                )
            }
            repr::ActionKind::Constraint(ref _c) => (vec![], vec![]),
            repr::ActionKind::Use(ref v) => (vec![], vec![v.base()]),

            // drop is special; it is not considered a "full use" of
            // the variable that is being dropped
            repr::ActionKind::Drop(..) => (vec![], vec![]),

            repr::ActionKind::Noop => (vec![], vec![]),

            repr::ActionKind::StorageDead(_) => (vec![], vec![]),

            repr::ActionKind::SkolemizedEnd(_) => (vec![], vec![]),
        }
    }
}
