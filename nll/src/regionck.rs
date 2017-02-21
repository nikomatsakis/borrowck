use env::{Environment, Point};
use liveness::Liveness;
use infer::{InferenceContext, RegionVariable};
use nll_repr::repr;
use std::collections::HashMap;
use std::error::Error;
use region::Region;

pub fn region_check(env: &Environment) -> Result<(), Box<Error>> {
    let ck = &mut RegionCheck {
        env,
        infer: InferenceContext::new(),
        var_map: HashMap::new(),
        region_map: HashMap::new()
    };
    ck.check()
}

struct RegionCheck<'env> {
    env: &'env Environment<'env>,
    infer: InferenceContext,
    var_map: HashMap<repr::Variable, RegionVariable>,
    region_map: HashMap<repr::RegionName, RegionVariable>,
}

impl<'env> RegionCheck<'env> {
    fn check(&mut self) -> Result<(), Box<Error>> {
        self.populate_var_map();
        let liveness = &Liveness::new(self.env);
        self.populate_inference(liveness);
        self.check_assertions(liveness)
    }

    fn check_assertions(&mut self, liveness: &Liveness) -> Result<(), Box<Error>> {
        let mut errors = 0;

        for assertion in self.env.graph.assertions() {
            match *assertion {
                repr::Assertion::Eq(region_name, ref region_literal) => {
                    let region_var = self.region_variable(region_name);
                    let region_value = self.to_region(region_literal);
                    if *self.infer.region(region_var) != region_value {
                        errors += 1;
                        println!("error: region variable `{:?}` has wrong value", region_name);
                        println!("  expected: {:?}", region_value);
                        println!("  found   : {:?}", self.infer.region(region_var));
                    }
                }

                repr::Assertion::In(region_name, ref point) => {
                    let region_var = self.region_variable(region_name);
                    let point = self.to_point(point);
                    if !self.infer.region(region_var).contains(point) {
                        errors += 1;
                        println!("error: region variable `{:?}` does not contain `{:?}`",
                                 region_name, point);
                        println!("  found   : {:?}", self.infer.region(region_var));
                    }
                }

                repr::Assertion::NotIn(region_name, ref point) => {
                    let region_var = self.region_variable(region_name);
                    let point = self.to_point(point);
                    if self.infer.region(region_var).contains(point) {
                        errors += 1;
                        println!("error: region variable `{:?}` contains `{:?}`",
                                 region_name, point);
                        println!("  found   : {:?}", self.infer.region(region_var));
                    }
                }

                repr::Assertion::Live(var, block_name) => {
                    let block = self.env.graph.block(block_name);
                    if !liveness.live_on_entry(var, block) {
                        errors += 1;
                        println!("error: variable `{:?}` not live on entry to `{:?}`",
                                 var, block_name);
                    }
                }

                repr::Assertion::NotLive(var, block_name) => {
                    let block = self.env.graph.block(block_name);
                    if liveness.live_on_entry(var, block) {
                        errors += 1;
                        println!("error: variable `{:?}` live on entry to `{:?}`",
                                 var, block_name);
                    }
                }
            }
        }

        if errors > 0 {
            try!(Err(format!("{} errors found", errors)));
        }

        Ok(())
    }

    fn populate_inference(&mut self, liveness: &Liveness) {
        liveness.walk(self.env, |point, action, live_on_entry| {
            // To start, find every variable `x` that is live. All regions
            // in the type of `x` must include `point`.
            let mut live_vars = vec![]; // we keep list of live vars for use as sanity check
            for decl in self.env.graph.decls() {
                let bit = liveness.bit(decl.var);

                if live_on_entry.get(bit) {
                    let region = self.var_map[&decl.var];
                    self.infer.add_live_point(region, point);
                    live_vars.push(decl.var);
                }
            }

            let action = if let Some(action) = action {
                action
            } else {
                return;
            };

            // Next, walk the actions and establish any additional constraints
            // that may arise from subtyping.
            let successor_point = Point { block: point.block, action: point.action + 1 };
            match *action {
                // `p = &'x` -- first, `'x` must include this point @ P,
                // and second `&'x <: typeof(p) @ succ(P)`
                repr::Action::Borrow(var, region_name) => {
                    let var_region = self.var_map[&var];
                    let borrow_region = self.region_variable(region_name);
                    self.infer.add_subregion(var_region, borrow_region, successor_point);
                }

                // a = b
                repr::Action::Assign(a, b) => {
                    let a_region = self.var_map[&a];
                    let b_region = self.var_map[&b];
                    // contravariant regions, so typeof(b) <: typeof(a) implies a <= b
                    self.infer.add_subregion(a_region, b_region, successor_point);
                }

                // a <: b
                repr::Action::Subtype(a, b) => {
                    let a_region = self.var_map[&a];
                    let b_region = self.var_map[&b];
                    // contravariant regions, so a <: b implies b <= a
                    self.infer.add_subregion(b_region, a_region, point);
                }

                // 'X <= 'Y
                repr::Action::Subregion(a, b) => {
                    let a_v = self.region_variable(a);
                    let b_v = self.region_variable(b);
                    self.infer.add_subregion(a_v, b_v, point);
                }

                // use(a); i.e., print(*a)
                repr::Action::Use(var) => {
                    assert!(live_vars.contains(&var));
                }

                // write(a); i.e., *a += 1
                repr::Action::Write(var) => {
                    assert!(live_vars.contains(&var));
                }

                repr::Action::Noop => {
                }
            }
        });

        self.infer.solve(self.env);
    }

    fn region_variable(&mut self, n: repr::RegionName) -> RegionVariable {
        let infer = &mut self.infer;
        *self.region_map.entry(n).or_insert_with(|| {
            let v = infer.add_var();
            log!("{:?} => {:?}", n, v);
            v
        })
    }

    fn populate_var_map(&mut self) {
        let decls = self.env.graph.decls();
        for decl in decls {
            let region = self.region_variable(decl.region);
            self.var_map.insert(decl.var, region);
        }
    }

    fn to_point(&self, point: &repr::Point) -> Point {
        let block = self.env.graph.block(point.block);
        Point { block: block, action: point.action }
    }

    fn to_region(&self, user_region: &repr::Region) -> Region {
        let mut region = Region::new();
        for p in &user_region.points {
            region.add_point(self.to_point(p));
        }
        region
    }
}
