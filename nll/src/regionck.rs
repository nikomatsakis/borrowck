use env::{Environment, Point};
use liveness::Liveness;
use infer::{InferenceContext, RegionVariable};
use nll_repr::repr::{self, Variance};
use std::collections::HashMap;
use std::error::Error;
use region::Region;

pub fn region_check(env: &Environment) -> Result<(), Box<Error>> {
    let ck = &mut RegionCheck {
        env,
        infer: InferenceContext::new(),
        region_map: HashMap::new()
    };
    ck.check()
}

struct RegionCheck<'env> {
    env: &'env Environment<'env>,
    infer: InferenceContext,
    region_map: HashMap<repr::RegionName, RegionVariable>,
}

impl<'env> RegionCheck<'env> {
    fn check(&mut self) -> Result<(), Box<Error>> {
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
                    if !liveness.var_live_on_entry(var, block) {
                        errors += 1;
                        println!("error: variable `{:?}` not live on entry to `{:?}`",
                                 var, block_name);
                    }
                }

                repr::Assertion::NotLive(var, block_name) => {
                    let block = self.env.graph.block(block_name);
                    if liveness.var_live_on_entry(var, block) {
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
            for region_name in liveness.live_regions(live_on_entry) {
                let rv = self.region_variable(region_name);
                self.infer.add_live_point(rv, point);
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
                repr::Action::Borrow(ref path, region_name) => {
                    let path_ty = self.env.path_ty(path);
                    let borrow_region = self.region_variable(region_name);
                    self.infer.add_live_point(borrow_region, point);
                    match *path_ty {
                        repr::Ty::Ref(rn, _) | repr::Ty::RefMut(rn, _) => {
                            let var_region = self.region_variable(rn);
                            self.infer.add_outlives(borrow_region, var_region, successor_point);
                        }
                        _ => {
                            panic!("result must be `&T` or `&mut T` type")
                        }
                    }
                }

                // a = b
                repr::Action::Assign(ref a, ref b) => {
                    let a_ty = self.env.path_ty(a);
                    let b_ty = self.env.path_ty(b);

                    // `b` must be a subtype of `a` to be assignable:
                    self.relate_tys(successor_point, repr::Variance::Co, &b_ty, &a_ty);
                }

                // 'X: 'Y
                repr::Action::Constraint(ref c) => {
                    match **c {
                        repr::Constraint::Outlives(c) => {
                            let sup_v = self.region_variable(c.sup);
                            let sub_v = self.region_variable(c.sub);
                            self.infer.add_outlives(sup_v, sub_v, point);
                        }
                        _ => {
                            panic!("unimplemented rich constraint: {:?}", c);
                        }
                    }
                }

                repr::Action::Init(..) | // a = use(...)
                repr::Action::Use(..) | // use(a)
                repr::Action::Drop(..) | // drop(a)
                repr::Action::Write(..) => { // write(a), e.g. *a += 1
                    // the basic liveness rules suffice here
                }

                repr::Action::Noop => {
                }
            }
        });

        self.infer.solve(self.env);
    }

    fn region_variable(&mut self, n: repr::RegionName) -> RegionVariable {
        let infer = &mut self.infer;
        let r = *self.region_map.entry(n).or_insert_with(|| infer.add_var());
        log!("{:?} => {:?}", n, r);
        r
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

    fn relate_tys(&mut self,
                  successor_point: Point,
                  variance: repr::Variance,
                  a: &repr::Ty,
                  b: &repr::Ty) {
        log!("relate_tys({:?} {:?} {:?} @ {:?})", a, variance, b, successor_point);
        match (a, b) {
            (&repr::Ty::Ref(r_a, ref t_a), &repr::Ty::Ref(r_b, ref t_b)) => {
                self.relate_regions(successor_point, variance.invert(), r_a, r_b);
                self.relate_tys(successor_point, variance, t_a, t_b);
            }
            (&repr::Ty::RefMut(r_a, ref t_a), &repr::Ty::RefMut(r_b, ref t_b)) => {
                self.relate_regions(successor_point, variance.invert(), r_a, r_b);
                self.relate_tys(successor_point, variance.xform(repr::Variance::In), t_a, t_b);
            }
            (&repr::Ty::Unit, &repr::Ty::Unit) => {
            }
            (&repr::Ty::Struct(s_a, ref ps_a), &repr::Ty::Struct(s_b, ref ps_b)) => {
                if s_a != s_b {
                    panic!("cannot compare `{:?}` and `{:?}`", s_a, s_b);
                }
                let s_decl = self.env.struct_map[&s_a];
                if ps_a.len() != s_decl.parameters.len() {
                    panic!("wrong number of parameters for `{:?}`", a);
                }
                if ps_b.len() != s_decl.parameters.len() {
                    panic!("wrong number of parameters for `{:?}`", b);
                }
                for (sp, (p_a, p_b)) in s_decl.parameters.iter().zip(ps_a.iter().zip(ps_b)) {
                    let v = variance.xform(sp.variance);
                    self.relate_parameters(successor_point, v, p_a, p_b);
                }
            }
            _ => panic!("cannot relate types `{:?}` and `{:?}`", a, b)
        }
    }

    fn relate_regions(&mut self,
                      successor_point: Point,
                      variance: repr::Variance,
                      a: repr::RegionName,
                      b: repr::RegionName) {
        log!("relate_regions({:?} {:?} {:?} @ {:?})", a, variance, b, successor_point);
        let r_a = self.region_map[&a];
        let r_b = self.region_map[&b];
        match variance {
            Variance::Co =>
                // "a Co b" == "a <= b"
                self.infer.add_outlives(r_b, r_a, successor_point),
            Variance::Contra =>
                // "a Contra b" == "a >= b"
                self.infer.add_outlives(r_a, r_b, successor_point),
            Variance::In => {
                self.infer.add_outlives(r_a, r_b, successor_point);
                self.infer.add_outlives(r_b, r_a, successor_point);
            }
        }
    }

    fn relate_parameters(&mut self,
                         successor_point: Point,
                         variance: repr::Variance,
                         a: &repr::TyParameter,
                         b: &repr::TyParameter) {
        match (a, b) {
            (&repr::TyParameter::Ty(ref t_a), &repr::TyParameter::Ty(ref t_b)) =>
                self.relate_tys(successor_point, variance, t_a, t_b),
            (&repr::TyParameter::Region(r_a), &repr::TyParameter::Region(r_b)) =>
                self.relate_regions(successor_point, variance, r_a, r_b),
            _ => panic!("cannot relate parameters `{:?}` and `{:?}`", a, b)
        }
    }
}
